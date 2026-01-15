use crate::errors::PyroductError;
use crate::module_capability::access::wasm_ptr_to_slice;
use arrow_scalars::ArrowRow;

use super::wasm_link::{Capabilities, Capability, HarnessState};
use rkyv::rancor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info};
use wasmtime::{Caller, Engine, Extern, Instance, Linker, Memory, Module, Store, TypedFunc};

#[derive(serde::Deserialize, Debug)]
pub struct HarnessConfig {
    pub module_name: String,
    /// Path to the WASM module to run
    pub module: PathBuf,
    /// List of paths to dynamic library capabilities (.so/.dylib/.dll)
    pub capabilities: Vec<CapabilityConfig>,
    /// Input data as JSON - will be deserialized into ArrowRow
    pub inputs: Vec<ArrowRow<'static>>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(untagged)]
pub enum CapabilityConfig {
    NoConfig(PathBuf),
    Config {
        path: PathBuf,
        config: serde_json::Value,
    },
}

pub struct CompiledModule {
    name: String,
    path: PathBuf,
    store: Store<HarnessState>,
    #[allow(dead_code)]
    instance: Instance,
    memory: Memory,
    alloc_func: TypedFunc<i32, i32>,
    call_func: TypedFunc<(i32, i32), u64>,
    capabilities: Capabilities,
}

impl CompiledModule {
    pub async fn new(
        name: &str,
        path: &Path,
        engine: &Engine,
        wasm_bytes: &[u8],
        capabilities: Vec<Arc<dyn Capability>>,
        configs: Vec<Option<&serde_json::Value>>,
    ) -> Result<Self, PyroductError> {
        let module = Module::from_binary(engine, wasm_bytes).map_err(|err| {
            PyroductError::from_module_linking(
                name,
                path,
                format!("Unable to parse the wasm binary: {err}"),
            )
        })?;
        let (harness_state, capabilities) =
            HarnessState::new(name.to_string(), path.to_path_buf(), capabilities, configs).await?;

        let mut store = Store::new(engine, harness_state);
        let mut linker = Linker::new(engine);

        let module_name = name.to_string();
        let module_span = tracing::span!(tracing::Level::INFO, "MODULE", name = module_name);

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
                    name,
                    path,
                    format!("Module does not have a sutible log function: {err}"),
                )
            })?;

        capabilities.attach_imports(&mut linker);

        let instance = linker
            .instantiate_async(&mut store, &module)
            .await
            .map_err(|err| {
                PyroductError::from_module_linking(
                    name,
                    path,
                    format!("Unable to instantiate the module: {err}"),
                )
            })?;
        let memory = instance.get_memory(&mut store, "memory").ok_or_else(|| {
            PyroductError::from_module_linking(name, path, format!("Module does not have a memory"))
        })?;
        let alloc_func = instance
            .get_typed_func::<i32, i32>(&mut store, "alloc")
            .map_err(|err| {
                PyroductError::from_module_linking(
                    name,
                    path,
                    format!("Module does not have a sutible Alloc: {err}"),
                )
            })?;
        let call_func = instance
            .get_typed_func::<(i32, i32), u64>(&mut store, "exter_call")
            .map_err(|err| {
                PyroductError::from_module_linking(
                    name,
                    path,
                    format!("Module does not have a sutible call function: {err}"),
                )
            })?;

        Ok(CompiledModule {
            name: name.to_string(),
            path: path.to_path_buf(),
            store,
            instance,
            memory,
            alloc_func,
            call_func,
            capabilities,
        })
    }

    #[tracing::instrument(skip(self, input))]
    pub async fn process(
        &mut self,
        input: &ArrowRow<'_>,
    ) -> Result<Result<ArrowRow<'static>, String>, PyroductError> {
        debug!("Resetting capability states...");
        self.capabilities
            .reset_states(self.store.data_mut())
            .await?;

        info!("Serializing ArrowRow input...");
        let data = rkyv::to_bytes::<rancor::Error>(input).map_err(|e| {
            PyroductError::from_module_serialization(
                self.name.clone(),
                self.path.clone(),
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
                    self.name.clone(),
                    self.path.clone(),
                    format!("Failed to allocate module input: {}", e),
                )
            })?;

        debug!("Writing input to WASM memory at ptr {:#x}", ptr);
        self.memory
            .write(&mut self.store, ptr as usize, data.as_slice())
            .map_err(|e| {
                PyroductError::from_module_memory(
                    self.name.clone(),
                    self.path.clone(),
                    format!("Failed to write module input: {}", e),
                )
            })?;

        info!("Invoking WASM export 'exter_call'...");
        let packed_result = self
            .call_func
            .call_async(&mut self.store, (ptr, data.len() as i32))
            .await
            .map_err(|error| {
                self.store.data_mut().take_error().unwrap_or_else(|| {
                    PyroductError::from_module_unknown(
                        self.name.clone(),
                        self.path.clone(),
                        format!("Unknown during call error: {error}"),
                    )
                })
            })?;

        let slice = match wasm_ptr_to_slice(packed_result) {
            Some((start, end)) => {
                let memory = self.memory.data(&self.store);

                if end > memory.len() {
                    tracing::error!(
                        "Result pointer out of bounds! Memory: {}, End: {}",
                        memory.len(),
                        end
                    );
                    return Err(PyroductError::from_module_memory(
                        self.name.clone(),
                        self.path.clone(),
                        format!(
                            "Result pointer out of bounds! Memory: {}, End: {}",
                            memory.len(),
                            end
                        ),
                    ));
                }

                &memory[start..end]
            }
            None => {
                return Err(PyroductError::from_module_memory(
                    self.name.clone(),
                    self.path.clone(),
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
                self.name.clone(),
                self.path.clone(),
                format!("Failed to validate call return: {}", e),
            )
        })?;

        match archived {
            rkyv::result::ArchivedResult::Ok(archived_row) => {
                info!("Result deserialized successfully.");
                Ok(Ok(ArrowRow::from(archived_row).into_owned()))
            }
            rkyv::result::ArchivedResult::Err(error) => {
                info!("Result deserialized successfully.");
                Ok(Err(error.to_string()))
            }
        }
    }
}
