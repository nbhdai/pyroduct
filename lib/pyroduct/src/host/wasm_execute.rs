use std::cmp::min;
use std::sync::Arc;

use crate::errors::PyroductError;
use crate::host::capability::Capabilities;
use crate::host::pipeline::PipelineDef;
use crate::host::wasm_bridge::HarnessState;
use crate::module_capability::access::wasm_ptr_to_slice;
use crate::{ModIdentity, PyroductResult};
use arrow::array::RecordBatch;
use arrow_scalars::{ArrowRow, Rowable};

use rkyv::rancor;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, mpsc};
use tracing::{debug, error, info, instrument, warn};
use wasmtime::{Caller, Config, Engine, Extern, Instance, Linker, Memory, Module, Store, TypedFunc};

pub struct Harness {
    pub ident: ModIdentity,
    store: Store<HarnessState>,
    #[allow(dead_code)]
    instance: Instance,
    memory: Memory,
    alloc_func: TypedFunc<i32, i32>,
    call_func: TypedFunc<(i32, i32), u64>,
}

impl Harness {
    pub async fn new(
        engine: &Engine,
        wasm_bytes: &[u8],
        // So we can link against the capabilities we've got.
        capabilities: &Capabilities,
        harness_state: HarnessState,
    ) -> Result<Self, PyroductError> {
        let ident = harness_state.module.clone();
        let module = Module::from_binary(engine, wasm_bytes).map_err(|err| {
            PyroductError::from_module_linking(
                &ident,
                format!("Unable to parse the wasm binary: {err}"),
            )
        })?;

        let mut store = Store::new(engine, harness_state);
        let mut linker = Linker::new(engine);

        let module_span = tracing::span!(tracing::Level::INFO, "MODULE", name = ident.name());

        linker
            .func_wrap(
                "env",
                "host_log",
                move |mut caller: Caller<'_, HarnessState>, ptr: i32, len: i32| {
                    let memory = match caller.get_export("memory") {
                        Some(Extern::Memory(mem)) => mem,
                        _ => return,
                    };
                    let data = match memory.data(&caller).get(ptr as usize..(ptr + len) as usize) {
                        Some(d) => d,
                        None => return,
                    };
                    let log_msg = String::from_utf8_lossy(data);
                    let _entered = module_span.enter();
                    tracing::debug!("{}", log_msg);
                },
            )
            .map_err(|err| {
                PyroductError::from_module_linking(
                    &ident,
                    format!("Module does not have a sutible log function: {err}"),
                )
            })?;

        capabilities.link(store.data().capabilities(), &mut linker)?;

        let instance = linker
            .instantiate_async(&mut store, &module)
            .await
            .map_err(|err| {
                PyroductError::from_module_linking(
                    &ident,
                    format!("Unable to instantiate the module: {err}"),
                )
            })?;
        let memory = instance.get_memory(&mut store, "memory").ok_or_else(|| {
            PyroductError::from_module_linking(&ident, format!("Module does not have a memory"))
        })?;
        let alloc_func = instance
            .get_typed_func::<i32, i32>(&mut store, "alloc")
            .map_err(|err| {
                PyroductError::from_module_linking(
                    &ident,
                    format!("Module does not have a sutible Alloc: {err}"),
                )
            })?;
        let call_func = instance
            .get_typed_func::<(i32, i32), u64>(&mut store, "exter_call")
            .map_err(|err| {
                PyroductError::from_module_linking(
                    &ident,
                    format!("Module does not have a sutible call function: {err}"),
                )
            })?;

        Ok(Self {
            ident: ident.clone(),
            store,
            instance,
            memory,
            alloc_func,
            call_func,
        })
    }

    #[instrument(skip(self, input), fields(module = %self.ident.name()))]
    pub async fn process(
        &mut self,
        input: &ArrowRow<'_>,
    ) -> Result<Result<ArrowRow<'static>, String>, PyroductError> {
        debug!("Resetting capability states...");
        self.store.data_mut().reset().await?;

        debug!("Serializing ArrowRow input...");
        let data = rkyv::to_bytes::<rancor::Error>(input).map_err(|e| {
            PyroductError::from_module_serialization(
                &self.ident,
                format!("Failed to serialize input: {}", e),
            )
        })?;

        debug!("Allocating {} bytes in WASM memory...", data.len());
        let ptr = self
            .alloc_func
            .call_async(&mut self.store, data.len() as i32)
            .await
            .map_err(|e| {
                PyroductError::from_module_memory(
                    &self.ident,
                    format!("Failed to allocate module input: {}", e),
                )
            })?;

        debug!("Writing input to WASM memory at ptr {:#x}", ptr);
        self.memory
            .write(&mut self.store, ptr as usize, data.as_slice())
            .map_err(|e| {
                PyroductError::from_module_memory(
                    &self.ident,
                    format!("Failed to write module input: {}", e),
                )
            })?;

        debug!("Invoking WASM export 'exter_call'...");
        let packed_result = self
            .call_func
            .call_async(&mut self.store, (ptr, data.len() as i32))
            .await
            .map_err(|error| {
                self.store.data_mut().take_error().unwrap_or_else(|| {
                    PyroductError::from_module_unknown(
                        &self.ident,
                        format!("Unknown during call error: {error}"),
                    )
                })
            })?;

        let slice = match wasm_ptr_to_slice(packed_result) {
            Some((start, end)) => {
                let memory = self.memory.data(&self.store);

                if end > memory.len() {
                    let msg = format!(
                        "Result pointer out of bounds! Memory: {}, End: {}",
                        memory.len(),
                        end
                    );
                    error!("{}", msg);
                    return Err(PyroductError::from_module_memory(&self.ident, msg));
                }

                &memory[start..end]
            }
            None => {
                return Err(PyroductError::from_module_memory(
                    &self.ident,
                    "Result pointer weird",
                ));
            }
        };

        // Access the archived Result<ArrowRow, String>
        type ReturnType<'a> = Result<ArrowRow<'a>, String>;
        let archived = rkyv::access::<<ReturnType as rkyv::Archive>::Archived, rancor::Error>(
            slice,
        )
        .map_err(|e| {
            PyroductError::from_module_validation(
                &self.ident,
                format!("Failed to validate call return: {}", e),
            )
        })?;

        match archived {
            rkyv::result::ArchivedResult::Ok(archived_row) => {
                debug!("Result deserialized successfully (Ok).");
                Ok(Ok(ArrowRow::from(archived_row).into_owned()))
            }
            rkyv::result::ArchivedResult::Err(error) => {
                debug!("Result deserialized successfully (Err).");
                Ok(Err(error.to_string()))
            }
        }
    }
}

pub struct Pipeline {
    steps: Vec<Harness>
}

impl Pipeline {
    pub async fn new(def: &PipelineDef, capabilities: &Capabilities) -> PyroductResult<Self> {
        let mut steps = Vec::new();

        // Must enable async support for async host calls
        let mut config = Config::new();
        config.async_support(true);
        
        // --- USING NEW ERROR TYPE HERE ---
        let engine = Engine::new(&config).map_err(|e| {
             PyroductError::from_infrastructure(format!("Failed to create Wasmtime engine: {}", e))
        })?;

        for (index, wasm_def) in def.pipeline.iter().enumerate() {
            debug!(index, wasm=wasm_def.ident.name(), "Initializing");
            // Init capabilities for this step
            let harness_state = capabilities.init(&wasm_def.ident, &wasm_def.capabilities).await?;
            
            // Create the harness
            let harness = Harness::new(
                &engine,
                &wasm_def.binary,
                capabilities,
                harness_state
            ).await?;

            steps.push(harness);
        }

        Ok(Self { steps })
    }

    #[instrument(skip(self, input))]
    pub async fn process(
        &mut self,
        mut input: ArrowRow<'_>,
    ) -> Result<Result<ArrowRow<'static>, Failure>, PyroductError> {
        let pipeline_len = self.steps.len();
        info!("Pipeline Start: Executing {} steps", pipeline_len);
        
        let mut result: ArrowRow<'static> = input.clone().into_owned();

        for (i, step) in self.steps.iter_mut().enumerate() {
            debug!("Pipeline Step {}/{}: Processing module '{}'", i + 1, pipeline_len, step.ident.name());

            match step.process(&input).await? {
                Ok(output) => {
                    // Success case
                    debug!("Pipeline Step {}/{}: Success", i + 1, pipeline_len);
                    result.extend(output.clone());
                    input = output;
                }
                Err(error) => {
                    // Logic failure in the module (returned Err string)
                    warn!(
                        "Pipeline Step {}/{}: Module '{}' returned error: {}", 
                        i + 1, pipeline_len, step.ident.name(), error
                    );
                    return Ok(Err(Failure {
                        row_index: 0,
                        error,
                        partial_data: result,
                    }));
                },
            }
        }

        info!("Pipeline Complete: Successfully finished all {} steps", pipeline_len);
        Ok(Ok(result))
    }
}

/// Represents a failure where the pipeline could not complete successfully,
/// but may have returned partial data before the error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Failure {
    pub row_index: usize,
    pub error: String,
    /// Stores the state of the data at the moment of failure
    pub partial_data: ArrowRow<'static>, 
}

pub struct PipelinePool {
    pipelines: Arc<Mutex<Vec<Pipeline>>>,
}

impl PipelinePool {
    pub fn new(pipelines: Vec<Pipeline>) -> Self {
        Self {
            pipelines: Arc::new(Mutex::new(pipelines)),
        }
    }

    /// Processes a batch by distributing chunks of the RecordBatch to available pipelines.
    /// Results are streamed back via a channel and collected by the main thread.
    ///
    /// Returns:
    /// - `Vec<(usize, ArrowRow<'static>)>`: Unsorted results containing the row index and data.
    /// - `Vec<Failure>`: List of rows that encountered logic errors, with their partial state.
    pub async fn process_batch(
        &self,
        batch: &RecordBatch,
    ) -> Result<(Vec<ArrowRow<'static>>, Vec<Failure>), PyroductError> {
        let total_rows = batch.num_rows();
        if total_rows == 0 {
            return Ok((Vec::new(), Vec::new()));
        }

        let pipelines = {
            let mut guard = self.pipelines.lock().await;
            if guard.is_empty() {
                return Err(PyroductError::from_infrastructure(
                    "Pipeline pool is empty or exhausted".to_string(),
                ));
            }
            std::mem::take(&mut *guard)
        };

        let num_pipelines = pipelines.len();
        
        let (tx, mut rx) = mpsc::channel(100);
        let chunk_size = min((total_rows + num_pipelines - 1) / num_pipelines, 1000);
        let mut handles = Vec::with_capacity(num_pipelines);

        for (i, mut pipeline) in pipelines.into_iter().enumerate() {
            let offset = i * chunk_size;
            if offset >= total_rows {
                handles.push(tokio::spawn(async move { pipeline }));
                continue;
            }

            let length = chunk_size.min(total_rows - offset);
            let batch_slice = batch.slice(offset, length);
            let tx_clone = tx.clone();

            handles.push(tokio::spawn(async move {
                for j in 0..batch_slice.num_rows() {
                    let absolute_index = offset + j;
                    
                    // Extract row (safely handling extraction errors)
                    let row_res = batch_slice.row(j);
                    
                    let result = match row_res {
                        Ok(input_row) => {
                            match pipeline.process(input_row).await {
                                Ok(Ok(mut res)) => {
                                    res.insert("pyroduct_index".to_string(), (absolute_index as u64).into());
                                    Ok(res)
                                },
                                Ok(Err(mut failure)) => {
                                    failure.row_index = absolute_index;
                                    Err(failure)
                                },
                                Err(e) => {
                                    Err(Failure { row_index: absolute_index, error: e.to_string(), partial_data: ArrowRow::empty() })
                                }
                            }
                        }
                        Err(e) => Err(Failure { row_index: absolute_index, error: e.to_string(), partial_data: ArrowRow::empty() })
                    };

                    if let Err(_) = tx_clone.send(result).await {
                        break;
                    }
                }
                
                pipeline
            }));
        }

        drop(tx);
        let mut success_results = Vec::with_capacity(total_rows);
        let mut failures = Vec::new();

        while let Some(row_result) = rx.recv().await {
            match row_result {
                Ok(row) => success_results.push(row),
                Err(failure) => failures.push(failure),
            }
        }

        // Reclaim pipelines and put them back in the pool
        let mut reclaimed_pipelines = Vec::with_capacity(num_pipelines);
        for handle in handles {
            match handle.await {
                Ok(p) => reclaimed_pipelines.push(p),
                Err(e) => {
                    tracing::error!("Worker task panicked, pipeline lost: {}", e);
                }
            }
        }

        {
            let mut guard = self.pipelines.lock().await;
            *guard = reclaimed_pipelines;
        }
        success_results.sort_by_key(|row| row.get_u64("pyroduct_index").unwrap_or_default());


        Ok((success_results, failures))
    }
}