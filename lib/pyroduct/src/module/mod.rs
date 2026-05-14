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

use pyro_artifacts::cache::{CacheError, LoadedPlaybook};
use pyro_artifacts::build::BuildError;
use pyro_artifacts::environment::EnvironmentError;
use wasmtime::{
    Caller, Engine, FuncType, Instance, Linker, Memory, Store, TypedFunc, Val, ValType,
};

use crate::ffi::host::ForeignObject;
use crate::format::Bridgeable;
use crate::format::{
    ParseError, PyroFailure, PyroLogs, PyroRow, PyroSuccess, PyroView,
    header::{DataStatus, PyroData, PyroHeader},
};
use crate::module::call::PyroCallIo;
use crate::{CapturedError, PyroError};

mod call;
pub mod capability;
pub mod sessions;
mod state;
// #[cfg(all(test, feature = "module"))]
// mod tests;

use capability::CapabilityLibrary;
pub use sessions::{Session, SessionCallError, SessionResult};
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

/// A linker pre-configured to use `PyroState<T>` as store data.
///
/// Host functions registered through `define_async` / `define_sync` receive
/// a clean `(&T, PyroView)` signature — all wasm memory plumbing is hidden.
pub struct PyroFactory {
    configurations: HashMap<String, Option<serde_json::Value>>,
    libraries: Vec<Arc<CapabilityLibrary>>,
    module: PyroModule,
}

impl PyroFactory {
    /// Create a new linker for the given engine.
    pub fn new(
        libraries: Vec<Arc<CapabilityLibrary>>,
        configurations: HashMap<String, Option<serde_json::Value>>,
        module: PyroModule,
    ) -> Result<Self, WasmError> {
        let mut config = wasmtime::Config::new();
        config.async_support(true);

        Ok(Self {
            libraries,
            configurations,
            module,
        })
    }

    pub fn from_playbook(playbook: &LoadedPlaybook) -> Result<Self, WasmError> {
        tracing::debug!("Loading module from hash: {}", playbook.binary.hash());
        let wasmtime_module = wasmtime::Module::from_binary(&DEFAULT_ENGINE, &playbook.binary.wasm)
            .map_err(|e| {
                WasmError::InstantiationFailed(format!("Failed to compile WASM: {}", e))
            })?;
        let pyro_module = PyroModule::new(wasmtime_module)?;

        let mut libs = Vec::new();

        for (name, path) in playbook.paths.iter() {
            let library = CapabilityLibrary::load(name.clone(), path).map_err(|e| {
                WasmError::InstantiationFailed(format!(
                    "Failed to load capability library at {}: {}",
                    path.display(),
                    e
                ))
            })?;
            libs.push(library);
        }

        Ok(PyroFactory {
            libraries: libs,
            configurations: playbook.configurations.clone(),
            module: pyro_module,
        })
    }

    // Todo: make this more robust.
    async fn create_capabilities(&self) -> Result<HashMap<String, ForeignObject>, WasmError> {
        let mut objects = HashMap::new();
        for (class, config) in &self.configurations {
            for library in self.libraries.iter() {
                if let Ok(object) = library.instantiate_class(class, config.as_ref()).await {
                    objects.insert(class.clone(), object);
                }
            }
        }

        Ok(objects)
    }

    pub async fn instantiate(&self) -> Result<PyroInstance, WasmError> {
        let pyro_state = PyroState::new();
        let mut store = Store::new(&DEFAULT_ENGINE, pyro_state);
        let objects = self.create_capabilities().await?;
        let mut linker = Linker::new(&DEFAULT_ENGINE);

        Self::link_logger(&mut linker)?;
        Self::link_capabilities(&mut linker, &objects)?;

        for (class_name, object) in objects.iter() {
            for method_name in object.method_names() {
                // Check what the linker actually holds for this class & method
                if let Some(ext) = linker.get(&mut store, class_name, &method_name) {
                    if let Some(func) = ext.into_func() {
                        let ty = func.ty(&store);

                        // Print the actual signature Wasmtime is about to use
                        tracing::debug!(
                            "LINKER CHECK: {}::{} -> {:?}",
                            class_name,
                            method_name,
                            ty
                        );

                        // Catch the ghost 4-parameter signature
                        if ty.params().len() != 2 {
                            tracing::error!(
                                class_name,
                                method_name,
                                ?ty,
                                "Incorrectly registered in the linker. Need 2 parameters right before instantiation! ",
                            );
                        }
                        if ty.results().len() != 1 {
                            tracing::error!(
                                class_name,
                                method_name,
                                ?ty,
                                "Incorrectly registered in the linker. Need 1 result right before instantiation! ",
                            );
                        }
                    } else {
                        tracing::error!(
                            "LINKER CHECK: {}::{} is registered, but it's NOT a function!",
                            class_name,
                            method_name
                        );
                    }
                } else {
                    tracing::error!(
                        "LINKER CHECK: {}::{} is completely MISSING from the linker!",
                        class_name,
                        method_name
                    );
                }
                tracing::debug!("LINKER CHECK: {}::{} PASSED", class_name, method_name);
            }
        }

        let instance = linker
            .instantiate_async(&mut store, self.module.module())
            .await
            .map_err(|e| WasmError::InstantiationFailed(format!("{:#}", e)))?;
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| WasmError::MissingExport("Missing 'memory'".to_string()))?;

        // Link the PyroState methods to the instance exports
        PyroState::link(&mut store, &instance)?;
        Ok(PyroInstance {
            store,
            instance,
            memory,
            objects,
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
        objects: &HashMap<String, ForeignObject>,
    ) -> Result<(), WasmError> {
        for (class_name, object) in objects.iter() {
            // Capture lib for the closures (Arc clone is cheap)

            for method_name in object.method_names() {
                let method_name = method_name.to_string();
                let class_name = class_name.clone();
                let fn_name = method_name.clone();
                let object = object.clone();

                let ty = FuncType::new(
                    linker.engine(),
                    [ValType::I32, ValType::I32],
                    [ValType::I32],
                );

                linker
                    .func_new_async(
                        &class_name,
                        &method_name,
                        ty,
                        move |caller, params, results| {
                            let client_ptr = params[0].unwrap_i32();
                            let input_ptr = params[1].unwrap_i32();

                            let object = object.clone();
                            let fn_name = fn_name.clone();

                            Box::new(async move {
                                tracing::debug!(
                                    class_name = object.name(),
                                    fn_name,
                                    "Calling function"
                                );

                                let mut io = PyroCallIo::from_caller(caller)?;
                                let client_view = io.borrow_argument(client_ptr).await?;
                                let input_view = io.borrow_argument(input_ptr).await?;

                                let output_view =
                                    object.call(&fn_name, client_view, input_view).await?;
                                output_view.parse_as_error()?;

                                let ptr = io.new_input(&output_view).await?;

                                results[0] = Val::I32(ptr);

                                Ok(())
                            })
                        },
                    )
                    .map_err(|e| {
                        WasmError::LinkFunctionFailed(
                            class_name,
                            method_name,
                            format!("Error: {:#}\nBacktrace: {}", e, e.backtrace()),
                        )
                    })?;
            }

            let class_name = class_name.clone();
            let object = object.clone();
            let ty = FuncType::new(linker.engine(), [ValType::I32], [ValType::I32]);
            linker
                .func_new_async(
                    &class_name,
                    "register",
                    ty,
                    move |caller, params, results| {
                        let object = object.clone();
                        Box::new(async move {
                            let mut io = PyroCallIo::from_caller(caller)?;
                            let client_ptr = params[0].unwrap_i32();

                            // Read input and get state — both are &self borrows.
                            let client_view = io.borrow_argument(client_ptr).await?;

                            // Call user function — consumes both borrows on return.
                            let output_view = object.register(client_view).await?;
                            output_view.parse_as_error()?;

                            // Write output back into wasm memory.
                            let ptr = io.new_input(&output_view).await?;
                            results[0] = Val::I32(ptr);
                            Ok(())
                        })
                    },
                )
                .map_err(|e| {
                    WasmError::LinkFunctionFailed(class_name, "register".to_string(), e.to_string())
                })?;
        }
        Ok(())
    }
}

pub struct PyroInstance {
    store: Store<PyroState>,
    instance: Instance,
    memory: Memory,
    objects: HashMap<String, ForeignObject>,
}

impl PyroInstance {
    pub async fn call(&mut self, input: &PyroRow<'_>) -> Result<PyroSuccess, PyroFailure> {
        // Ship the input row via rkyv into a PyroVec
        let input_row_owned = input.to_static();
        let input_vec = input_row_owned
            .ship()
            .map_err(|err| self.pack_pyro_error(err))?;
        let input_view: PyroView = input_vec.view();

        // 1. Write Input using PyroCallIo
        let mut io = PyroCallIo::new(&mut self.store, self.memory);
        let input_ptr = io
            .new_input(&input_view)
            .await
            .map_err(|err| self.pack_pyro_error(err))?;

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
            .map_err(|err| self.pack_pyro_error(err))?;

        tracing::debug!("calling call_extern...");
        let output_ptr = entry
            .call_async(&mut self.store, input_ptr)
            .await
            .map_err(classify_error)
            .map_err(|err| self.pack_pyro_error(err))?;
        tracing::debug!("output_ptr = {:#x}", output_ptr);

        // 3. Read Output using PyroCallIo
        let mut io = PyroCallIo::new(&mut self.store, self.memory);
        tracing::debug!("getting output...");
        let output_vec = io
            .get_output(output_ptr)
            .await
            .map_err(|err| self.pack_pyro_error(err))?;
        tracing::debug!("output_vec received");

        // 4. Parse the result (Zero-copy view of the host-owned vector)
        let result_view = output_vec.view();
        result_view
            .parse_as_error()
            .map_err(|err| self.pack_pyro_error(err))?;

        let pyref = result_view.py_ref();
        match result_view.status() {
            Ok(DataStatus::RkyvValid) => {
                let row = PyroRow::expose_view(pyref).map_err(|err| self.pack_pyro_error(err))?;
                Ok(self.pack_success(PyroRow::from(&*row).to_static()))
            }
            Ok(DataStatus::RkyvError) => match serde_json::from_slice(&result_view) {
                Ok(error) => Err(self.pack_user_error(error)),
                Err(error) => {
                    Err(self.pack_pyro_error(PyroError::capture_json(error, &*result_view)))
                }
            },
            _ => Err(
                self.pack_pyro_error(PyroError::Header(ParseError::UnknownStatus(
                    result_view.status_u8(),
                ))),
            ),
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

    /// Access the underlying wasmtime Store.
    pub fn store(&self) -> &Store<PyroState> {
        &self.store
    }

    /// Access the underlying wasmtime Store mutably.
    pub fn store_mut(&mut self) -> &mut Store<PyroState> {
        &mut self.store
    }

    /// Access the underlying wasmtime Instance.
    pub fn instance(&self) -> &Instance {
        &self.instance
    }

    /// Access the underlying wasmtime Memory.
    pub fn memory(&self) -> Memory {
        self.memory
    }

    pub fn pack_pyro_error(&self, error: impl std::error::Error) -> PyroFailure {
        PyroFailure {
            result: Err(error.to_string()),
            logs: self.unpack_logs(),
        }
    }

    pub fn pack_user_error(&self, error: CapturedError) -> PyroFailure {
        PyroFailure {
            result: Ok(error),
            logs: self.unpack_logs(),
        }
    }

    pub fn pack_success(&self, row: PyroRow<'static>) -> PyroSuccess {
        PyroSuccess {
            row,
            logs: self.unpack_logs(),
        }
    }

    pub fn logs(&self) -> HashMap<(String, String), Vec<String>> {
        let mut logs = HashMap::new();
        for object in self.objects.values() {
            let object_logs = object.take_logs();
            logs.insert(
                (object.lib_name().to_string(), object.name().to_string()),
                object_logs,
            );
        }

        logs
    }
}

/// Attempts to downcast an anyhow::Error into specific pyro error types.
fn classify_error(error: anyhow::Error) -> WasmError {
    if error.is::<WasmError>() {
        return error.downcast().unwrap();
    }
    if error.is::<PyroError>() {
        return WasmError::Pyro(error.downcast().unwrap());
    }
    WasmError::Unknown(error)
}
