// lib/pyroduct/src/pipeline/wasm/mod.rs
//! Host-side harness for calling the wasm module's main/entry function.
//!
//! `PyroInstance` owns the `Store` and `Instance` and provides methods to:
//!   1. Write input data into wasm memory (via the wasm-exported `new_input`).
//!   2. Call an exported wasm function that returns an output pointer.
//!   3. Read the output back as a `PyroView` (zero-copy into wasm memory).
//!   4. Free output pointers via the wasm-exported `free_output`.
//!
//! All data crosses the boundary as rkyv-serialized PyroVecs. The host uses
//! `Bridgeable::ship()` to produce them and `Bridgeable::expose_view()` to
//! get zero-copy access to the archived data in wasm linear memory.
//!

use std::{collections::HashMap, sync::Arc};

use pyro_artifacts::artifacts::{CapabilityConfig, PlaybookSpec};
use pyro_artifacts::build::BuildError;
use pyro_artifacts::cache::{CacheError, LoadedPlaybook, RemoteAddress};
use pyro_artifacts::cargo::CapabilityIdent;
use pyro_artifacts::environment::EnvironmentError;
use wasmtime::{Caller, Engine, Instance, Linker, Memory, Store, TypedFunc};

use crate::format::Bridgeable;
use crate::format::{
    ParseError, PyroFailure, PyroLogs, PyroRow, PyroSuccess, PyroView,
    header::{DataStatus, PyroData, PyroHeader},
};
use crate::module::call::PyroCallIo;
use crate::module::capability::ForeignCapability;
use crate::transport::socket::capability::RemoteCapability;
use crate::{CapturedError, PyroError};

pub(crate) mod call;
pub mod capability;
pub mod sessions;
mod state;
// #[cfg(all(test, feature = "module"))]
// mod tests;

use capability::{CapabilityError, CapabilityLibrary};
pub use state::{PyroModule, PyroState};

use thiserror::Error;

use lazy_static::lazy_static;

lazy_static! {
    static ref DEFAULT_ENGINE: Engine = {
        let mut config = wasmtime::Config::new();
        config.async_support(true);
        Engine::new(&config).unwrap()
    };
}

#[derive(Error, Debug)]
pub enum WasmError {
    #[error("Pyro Error: {0}")]
    Pyro(#[from] PyroError),

    #[error("Cache Error: {0}")]
    Cache(#[from] CacheError),

    #[error("Build Error: {0}")]
    Build(#[from] BuildError),

    #[error("Build Error: {0}")]
    Environment(#[from] EnvironmentError),

    #[error("Capability Error: {0}")]
    Capability(#[from] CapabilityError),

    #[error("Wasm module is missing required export: '{0}'")]
    MissingExport(String),

    #[error("Wasm module is missing required import: '{0}'")]
    MissingImport(String),

    #[error("Export '{0}' has incorrect signature.")]
    SignatureMismatch(String),

    #[error("Failed to link host function '{1}' in module '{0}': {2}")]
    LinkFunctionFailed(String, String, String),

    #[error("Wasmtime instantiation failed: {0}")]
    InstantiationFailed(String),

    #[error("Failed to configure Wasmtime engine: {0}")]
    EngineError(String),

    #[error("Input allocation failed: {0}")]
    InputMemory(wasmtime::Error),

    #[error("Output allocation failed: {0}")]
    OutputMemory(wasmtime::Error),

    #[error("Unknown: {0}")]
    Unknown(wasmtime::Error),
}

// ---------------------------------------------------------------------------
// PyroInstance — owns Store + Instance, drives host↔wasm IO
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub(crate) struct SessionState {
    pub(crate) input_len: u32,
    pub(crate) output_len: u32,
}

/// A linker pre-configured to use `PyroState<T>` as store data.
///
/// Host functions registered through `define_async` / `define_sync` receive
/// a clean `(&T, PyroView)` signature — all wasm memory plumbing is hidden.
pub struct PyroFactory {
    spec: Arc<PlaybookSpec>,
    configurations: HashMap<CapabilityIdent, CapabilityConfig>,
    libraries: Vec<Arc<CapabilityLibrary>>,
    module: PyroModule,
    remote: HashMap<CapabilityIdent, RemoteAddress>,
}

impl PyroFactory {
    pub fn spec(&self) -> &PlaybookSpec {
        &self.spec
    }

    pub fn from_playbook(playbook: &LoadedPlaybook) -> Result<Self, WasmError> {
        tracing::info!(hash = %playbook.binary.hash(), "Loading module from playbook");
        let wasmtime_module = wasmtime::Module::from_binary(&DEFAULT_ENGINE, &playbook.binary.wasm)
            .map_err(|e| {
                WasmError::InstantiationFailed(format!("Failed to compile WASM: {}", e))
            })?;
        let pyro_module = PyroModule::new(wasmtime_module)?;

        let mut libs = Vec::new();

        for (cap, path) in playbook.paths.iter() {
            tracing::debug!(name = %cap.package, path = %path.display(), "Loading capability library");
            let library = CapabilityLibrary::load(cap.clone(), path).map_err(|e| {
                WasmError::InstantiationFailed(format!(
                    "Failed to load capability library at {}: {}",
                    path.display(),
                    e
                ))
            })?;
            libs.push(library);
        }

        let spec = Arc::new(playbook.binary.spec.clone());
        let mut configurations = HashMap::new();
        for cap in &playbook.binary.configurations {
            configurations.insert(cap.ident(), cap.configuration.clone());
        }

        Ok(PyroFactory {
            spec,
            libraries: libs,
            configurations,
            module: pyro_module,
            remote: playbook.remote.clone(),
        })
    }

    // Todo: make this more robust.
    async fn create_capabilities(
        &self,
    ) -> Result<HashMap<CapabilityIdent, Box<dyn ForeignCapability>>, WasmError> {
        tracing::debug!("Creating capabilities");
        let mut caps: HashMap<CapabilityIdent, Box<dyn ForeignCapability>> = HashMap::new();

        // 1. Validate that all configured libraries are loaded (unless remote)
        for lib_ident in self.configurations.keys() {
            if !self.remote.contains_key(lib_ident)
                && !self.libraries.iter().any(|l| l.ident == *lib_ident)
            {
                return Err(WasmError::InstantiationFailed(format!(
                    "Capability library '{}' not found",
                    lib_ident
                )));
            }
        }

        // 2. Validate that all configured classes exist in their respective loaded libraries
        for library in &self.libraries {
            let lib_ident = &library.ident;
            if let Some(cap_config) = self.configurations.get(lib_ident) {
                for class_name in cap_config.classes.keys() {
                    if !library.capabilities.contains_key(class_name) {
                        return Err(WasmError::InstantiationFailed(format!(
                            "Capability library '{}' does not contain class '{}'",
                            lib_ident, class_name
                        )));
                    }
                }
            }
        }

        // 3. Handle remote capabilities
        for (lib_ident, remote_addr) in self.remote.iter() {
            tracing::info!(
                lib = ?lib_ident,
                addr = ?remote_addr,
                "Connecting to remote capability library"
            );
            let remote_cap = match remote_addr {
                RemoteAddress::Unix(path) => {
                    RemoteCapability::connect_unix(lib_ident.clone(), path).await
                }
                RemoteAddress::Tcp(addr) => {
                    RemoteCapability::connect_tcp(lib_ident.clone(), addr).await
                }
            }
            .map_err(|e| {
                WasmError::InstantiationFailed(format!(
                    "Failed to connect to remote capability library '{}': {}",
                    lib_ident, e
                ))
            })?;

            tracing::info!(
                lib = %lib_ident,
                "Successfully connected to remote capability library"
            );

            caps.insert(lib_ident.clone(), Box::new(remote_cap));
        }

        // 4. Handle local capability libraries
        for library in &self.libraries {
            let lib_ident = &library.ident;
            if let Some(cap_config) = self.configurations.get(lib_ident) {
                tracing::debug!(
                    lib = %library.ident,
                    classes = ?library.capabilities.keys(),
                    configs = ?cap_config.classes.keys(),
                    "Linking local capability library"
                );
                // If it maps to a remote address, it was already handled above.
                if self.remote.contains_key(lib_ident) {
                    tracing::debug!("Capability library is remote, skipping instantiation");
                    continue;
                }

                let cap = library.instantiate_from_config(&cap_config).await?;
                caps.insert(lib_ident.clone(), Box::new(cap));
            } else {
                tracing::debug!(lib = %lib_ident, "No configuration found for local capability library");
            }
        }

        Ok(caps)
    }

    pub async fn instantiate(&self) -> Result<PyroInstance, WasmError> {
        tracing::info!("Instantiating PyroInstance");
        let pyro_state = PyroState::new();
        let mut store = Store::new(&DEFAULT_ENGINE, pyro_state);
        let objects = self.create_capabilities().await?;
        let mut linker = Linker::new(&DEFAULT_ENGINE);

        Self::link_logger(&mut linker)?;
        Self::link_capabilities(&mut linker, &objects)?;

        let instance = linker
            .instantiate_async(&mut store, self.module.module())
            .await
            .map_err(|e| WasmError::InstantiationFailed(format!("{:#}", e)))?;
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| WasmError::MissingExport("Missing 'memory'".to_string()))?;

        // Link the PyroState methods to the instance exports
        PyroState::link(&mut store, &instance)?;
        tracing::info!("PyroInstance instantiated successfully");
        Ok(PyroInstance {
            spec: self.spec.clone(),
            store,
            instance,
            memory,
            objects,
            session_states: HashMap::new(),
        })
    }

    fn link_logger(linker: &mut Linker<PyroState>) -> Result<(), WasmError> {
        linker
            .func_wrap(
                "env",
                "host_log",
                move |caller: Caller<'_, PyroState>, ptr: i32, len: i32| {
                    let io = PyroCallIo::from_caller(caller)?;
                    let _ = io.log(ptr, len)?;
                    Ok(())
                },
            )
            .map_err(|e| {
                WasmError::LinkFunctionFailed(
                    "env".to_string(),
                    "host_log".to_string(),
                    e.to_string(),
                )
            })?;
        Ok(())
    }

    /// Links all capabilities from the provided libraries into the linker.
    fn link_capabilities(
        linker: &mut Linker<PyroState>,
        capabilities: &HashMap<CapabilityIdent, Box<dyn ForeignCapability>>,
    ) -> Result<(), WasmError> {
        for (ident, cap) in capabilities.iter() {
            tracing::debug!(lib = %ident, "Linking capability");
            cap.link(linker)?;
        }
        Ok(())
    }
}

pub struct PyroInstance {
    pub(crate) spec: Arc<PlaybookSpec>,
    pub(crate) store: Store<PyroState>,
    pub(crate) instance: Instance,
    pub(crate) memory: Memory,
    pub(crate) objects: HashMap<CapabilityIdent, Box<dyn ForeignCapability>>,
    pub(crate) session_states: HashMap<u32, SessionState>,
}

impl PyroInstance {
    pub fn spec(&self) -> &PlaybookSpec {
        &self.spec
    }

    pub async fn call(
        &mut self,
        row_index: u32,
        input: &PyroRow<'_>,
    ) -> Result<PyroSuccess, PyroFailure> {
        tracing::debug!("Calling wasm module");
        // Ship the input row via rkyv into a PyroVec
        let input_row_owned = input.to_static();
        let input_vec = input_row_owned
            .ship()
            .map_err(|err| self.pack_pyro_error(row_index, err))?;
        let input_view: PyroView = input_vec.view();

        // 1. Write Input using PyroCallIo
        let mut io = PyroCallIo::new(&mut self.store, self.memory);
        let input_ptr = io
            .new_input(&input_view)
            .await
            .map_err(|err| self.pack_pyro_error(row_index, err))?;

        tracing::debug!("input_ptr = {:#x}", input_ptr);

        // 2. Call the export
        let entry: TypedFunc<i32, i32> = self
            .instance
            .get_typed_func(&mut self.store, "call_extern")
            .map_err(|e| {
                PyroError::CodePanic(
                    CapturedError::new(format!("Missing main function: {}", e)).into(),
                )
            })
            .map_err(|err| self.pack_pyro_error(row_index, err))?;

        tracing::debug!("calling call_extern...");
        let output_ptr = match entry.call_async(&mut self.store, input_ptr).await {
            Ok(ptr) => ptr,
            Err(e) => {
                let mut io = PyroCallIo::new(&mut self.store, self.memory);
                if let Ok(Some(err_vec)) = io.get_panic_error().await {
                    match err_vec.status() {
                        Ok(DataStatus::RkyvError) => match serde_json::from_slice(&err_vec) {
                            Ok(error) => return Err(self.pack_user_error(row_index, error)),
                            Err(error) => {
                                return Err(self.pack_pyro_error(
                                    row_index,
                                    PyroError::capture_json(error, &err_vec),
                                ));
                            }
                        },
                        _ => match err_vec.parse_as_error() {
                            Ok(_) => return Err(self.pack_pyro_error(row_index, classify_error(e))),
                            Err(err) => return Err(self.pack_pyro_error(row_index, err)),
                        },
                    }
                }
                return Err(self.pack_pyro_error(row_index, classify_error(e)));
            }
        };
        tracing::debug!("output_ptr = {:#x}", output_ptr);

        // 3. Read Output using PyroCallIo
        let mut io = PyroCallIo::new(&mut self.store, self.memory);
        tracing::debug!("getting output...");
        let output_vec = io
            .get_output(output_ptr)
            .await
            .map_err(|err| self.pack_pyro_error(row_index, err))?;
        tracing::debug!("output_vec received");

        // 4. Parse the result (Zero-copy view of the host-owned vector)
        let result_view = output_vec.view();
        result_view
            .parse_as_error()
            .map_err(|err| self.pack_pyro_error(row_index, err))?;

        let pyref = result_view.py_ref();
        match result_view.status() {
            Ok(DataStatus::RkyvValid) => {
                tracing::debug!("Result status: RkyvValid");
                let row = PyroRow::expose_view(pyref)
                    .map_err(|err| self.pack_pyro_error(row_index, err))?;
                Ok(self.pack_success(row_index, PyroRow::from(&*row).to_static()))
            }
            Ok(DataStatus::RkyvError) => {
                tracing::debug!("Result status: RkyvError");
                match serde_json::from_slice(&result_view) {
                    Ok(error) => Err(self.pack_user_error(row_index, error)),
                    Err(error) => Err(self
                        .pack_pyro_error(row_index, PyroError::capture_json(error, &result_view))),
                }
            }
            Ok(DataStatus::CodeError) => {
                tracing::debug!("Result status: CodeError");
                match serde_json::from_slice(&result_view) {
                    Ok(error) => Err(self.pack_user_error(row_index, error)),
                    Err(error) => Err(self
                        .pack_pyro_error(row_index, PyroError::capture_json(error, &result_view))),
                }
            }
            _ => {
                tracing::debug!(status = result_view.status_u8(), "Result status: Unknown");
                Err(self.pack_pyro_error(
                    row_index,
                    PyroError::Header(ParseError::UnknownStatus(result_view.status_u8())),
                ))
            }
        }
    }

    pub fn unpack_logs(&self) -> PyroLogs {
        let module_logs = self.store.data().log();
        let capability_logs = self.logs();
        PyroLogs {
            module_logs,
            capability_logs,
        }
    }

    pub fn pack_pyro_error(&self, row_index: u32, error: impl std::error::Error) -> PyroFailure {
        PyroFailure {
            row_index,
            result: Err(error.to_string()),
            logs: self.unpack_logs(),
        }
    }

    pub fn pack_setup_pyro_error(row_index: u32, error: impl std::error::Error) -> PyroFailure {
        PyroFailure {
            row_index,
            result: Err(error.to_string()),
            logs: PyroLogs::empty(),
        }
    }

    pub fn pack_user_error(&self, row_index: u32, error: CapturedError) -> PyroFailure {
        PyroFailure {
            row_index,
            result: Ok(error),
            logs: self.unpack_logs(),
        }
    }

    pub fn pack_success(&self, row_index: u32, row: PyroRow<'static>) -> PyroSuccess {
        PyroSuccess {
            row_index,
            row,
            logs: self.unpack_logs(),
        }
    }

    pub fn logs(&self) -> HashMap<(String, String), Vec<String>> {
        let mut logs = HashMap::new();
        for object in self.objects.values() {
            let object_logs = object.take_logs();
            let lib_ident_str = object.lib_ident().to_string();
            for (class_name, class_logs) in object_logs {
                logs.insert((lib_ident_str.clone(), class_name), class_logs);
            }
        }

        logs
    }
}

/// Attempts to downcast an anyhow::Error into specific pyro error types.
pub(crate) fn classify_error(error: anyhow::Error) -> WasmError {
    if error.is::<WasmError>() {
        return error.downcast().unwrap();
    }
    if error.is::<PyroError>() {
        return WasmError::Pyro(error.downcast().unwrap());
    }
    WasmError::Unknown(error)
}
