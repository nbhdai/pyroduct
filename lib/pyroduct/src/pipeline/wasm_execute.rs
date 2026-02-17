use std::cmp::min;
use std::sync::Arc;

use arrow::array::RecordBatch;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, mpsc};
use tracing::{debug, error, info, instrument, warn};
use wasmtime::Module as WasmtimeModule;

use crate::value::PyroRow;
use crate::value::arrow::Rowable;
use crate::wasm::host::{PyroEngine, PyroInstance, PyroLinker, PyroModule};

use super::pipeline::PipelineDef;
use super::{PipelineError, PipelineResult};

// =============================================================================
// Pipeline
// =============================================================================

pub struct Pipeline {
    steps: Vec<PyroInstance>,
}

impl Pipeline {
    /// Build a pipeline from a fully-loaded `PipelineDef`.
    ///
    /// Creates one `PyroEngine` and one `PyroLinker` (with all capabilities
    /// linked), then compiles and instantiates each wasm module in order.
    pub async fn new(def: PipelineDef) -> PipelineResult<Self> {
        let engine = PyroEngine::new()?;
        let linker = PyroLinker::new(engine.engine(), def.capabilities)?;

        let mut steps = Vec::with_capacity(def.pipeline.len());

        for (index, module_def) in def.pipeline.iter().enumerate() {
            debug!(index, "Compiling wasm module");

            let wasm_module = WasmtimeModule::from_binary(engine.engine(), &module_def.binary)
                .map_err(|e| {
                    PipelineError::Config(format!(
                        "Failed to compile WASM for module: {}",
                        e
                    ))
                })?;

            let pyro_module = PyroModule::new(wasm_module)?;
            let instance = PyroInstance::new(&engine, &pyro_module, &linker).await?;
            steps.push(instance);
        }

        Ok(Self { steps })
    }

    /// Run the input through every step in sequence.
    ///
    /// Returns `Ok(Ok(row))` on success, `Ok(Err(failure))` if a module
    /// returned a logic error (with partial data), or `Err` on infrastructure
    /// failure.
    #[instrument(skip(self, input))]
    pub async fn process(
        &mut self,
        input: PyroRow<'_>,
    ) -> PipelineResult<Result<PyroRow<'static>, Failure>> {
        let pipeline_len = self.steps.len();
        info!("Pipeline Start: Executing {} steps", pipeline_len);

        let mut result: PyroRow<'static> = input.clone().into_owned();
        let mut current_input = input;

        for (i, step) in self.steps.iter_mut().enumerate() {
            debug!("Pipeline Step {}/{}: Processing", i + 1, pipeline_len);

            match step.call(&current_input).await? {
                Ok(output) => {
                    debug!("Pipeline Step {}/{}: Success", i + 1, pipeline_len);
                    result.extend(output.clone());
                    current_input = output;
                }
                Err(error) => {
                    warn!(
                        "Pipeline Step {}/{}: Module returned error: {}",
                        i + 1,
                        pipeline_len,
                        error
                    );
                    return Ok(Err(Failure {
                        row_index: 0,
                        error: error.to_string(),
                        partial_data: result,
                    }));
                }
            }
        }

        info!(
            "Pipeline Complete: Successfully finished all {} steps",
            pipeline_len
        );
        Ok(Ok(result))
    }
}

// =============================================================================
// Failure
// =============================================================================

/// A module returned a logic error. The pipeline stopped, but we keep
/// whatever data was accumulated before the failing step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Failure {
    pub row_index: usize,
    pub error: String,
    pub partial_data: PyroRow<'static>,
}

// =============================================================================
// PipelinePool
// =============================================================================

pub struct PipelinePool {
    pipelines: Arc<Mutex<Vec<Pipeline>>>,
}

impl PipelinePool {
    pub fn new(pipelines: Vec<Pipeline>) -> Self {
        Self {
            pipelines: Arc::new(Mutex::new(pipelines)),
        }
    }

    /// Distribute rows across available pipelines and collect results.
    ///
    /// Returns the successful rows (sorted by original index) and any
    /// per-row failures.
    pub async fn process_batch(
        &self,
        batch: &RecordBatch,
    ) -> PipelineResult<(Vec<PyroRow<'static>>, Vec<Failure>)> {
        let total_rows = batch.num_rows();
        if total_rows == 0 {
            return Ok((Vec::new(), Vec::new()));
        }

        let pipelines = {
            let mut guard = self.pipelines.lock().await;
            if guard.is_empty() {
                return Err(PipelineError::Config(
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

                    let result = match batch_slice.row(j) {
                        Ok(input_row) => match pipeline.process(input_row).await {
                            Ok(Ok(mut res)) => {
                                res.insert(
                                    "pyroduct_index".to_string(),
                                    (absolute_index as u64).into(),
                                );
                                Ok(res)
                            }
                            Ok(Err(mut failure)) => {
                                failure.row_index = absolute_index;
                                Err(failure)
                            }
                            Err(e) => Err(Failure {
                                row_index: absolute_index,
                                error: e.to_string(),
                                partial_data: PyroRow::empty(),
                            }),
                        },
                        Err(e) => Err(Failure {
                            row_index: absolute_index,
                            error: e.to_string(),
                            partial_data: PyroRow::empty(),
                        }),
                    };

                    if tx_clone.send(result).await.is_err() {
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

        // Reclaim pipelines back into the pool
        let mut reclaimed = Vec::with_capacity(num_pipelines);
        for handle in handles {
            match handle.await {
                Ok(p) => reclaimed.push(p),
                Err(e) => {
                    error!("Worker task panicked, pipeline lost: {}", e);
                }
            }
        }

        {
            let mut guard = self.pipelines.lock().await;
            *guard = reclaimed;
        }

        success_results.sort_by_key(|row| row.get_u64("pyroduct_index").unwrap_or_default());

        Ok((success_results, failures))
    }
}