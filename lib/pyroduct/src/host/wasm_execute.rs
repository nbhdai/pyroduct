use crate::ModIdentity;
use crate::errors::PyroductError;
use crate::host::capability::Capabilities;
use crate::host::CapabilityConfig;
use crate::host::harness::HarnessState;
use crate::module_capability::access::wasm_ptr_to_slice;
use arrow_scalars::ArrowRow;

use rkyv::rancor;
use std::path::PathBuf;
use tracing::{debug, info};
use wasmtime::{Caller, Engine, Extern, Instance, Linker, Memory, Module, Store, TypedFunc};

#[derive(serde::Deserialize, Debug)]
pub struct HarnessConfig {
    pub module_name: String,
    /// Path to the WASM module to run
    pub module: PathBuf,
    /// List of paths to dynamic library capabilities (.so/.dylib/.dll)
    pub capabilities: Vec<CapabilityDefinition>,
    /// Input data as JSON - will be deserialized into ArrowRow
    pub inputs: Vec<ArrowRow<'static>>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(untagged)]
pub enum CapabilityDefinition {
    NoConfig(PathBuf),
    Config {
        path: PathBuf,
        config: CapabilityConfig,
    },
}

impl CapabilityDefinition {
    pub fn config(&self) -> Option<&CapabilityConfig> {
        match self {
            CapabilityDefinition::NoConfig(_) => None,
            CapabilityDefinition::Config { config, .. } => Some(config),
        }
    }
}

pub struct CompiledModule {
    ident: ModIdentity,
    store: Store<HarnessState>,
    #[allow(dead_code)]
    instance: Instance,
    memory: Memory,
    alloc_func: TypedFunc<i32, i32>,
    call_func: TypedFunc<(i32, i32), u64>,
}

impl CompiledModule {
    pub async fn new(
        ident: &ModIdentity,
        engine: &Engine,
        wasm_bytes: &[u8],
        capabilities: &Capabilities,
        config: &HarnessConfig,
    ) -> Result<Self, PyroductError> {
        let module = Module::from_binary(engine, wasm_bytes).map_err(|err| {
            PyroductError::from_module_linking(
                &ident,
                format!("Unable to parse the wasm binary: {err}"),
            )
        })?;
        let harness_state = capabilities.init(ident, config.capabilities.iter().map(|c| c.config())).await?;

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
        
        capabilities.link(&mut linker)?;

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

        Ok(CompiledModule {
            ident: ident.clone(),
            store,
            instance,
            memory,
            alloc_func,
            call_func,
        })
    }

    #[tracing::instrument(skip(self, input))]
    pub async fn process(
        &mut self,
        input: &ArrowRow<'_>,
    ) -> Result<Result<ArrowRow<'static>, String>, PyroductError> {
        debug!("Resetting capability states...");
        self.store.data_mut().reset().await?;

        info!("Serializing ArrowRow input...");
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

        info!("Invoking WASM export 'exter_call'...");
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
                    tracing::error!(
                        "Result pointer out of bounds! Memory: {}, End: {}",
                        memory.len(),
                        end
                    );
                    return Err(PyroductError::from_module_memory(
                        &self.ident,
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
