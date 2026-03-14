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

use std::collections::HashMap;
#[allow(unused_imports)]
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;

use serde::Deserialize;
use wasmtime::{Caller, Engine, FuncType, Instance, Linker, Memory, Store, TypedFunc};

use crate::ffi::ForeignObject;
use crate::format::{
    Bridgeable, ParseError, PyroRow, PyroView, Receiver,
    header::{DataStatus, PyroData, PyroHeader},
    rkyv_8::RkyvReceiver,
};
use crate::module::call::PyroCallIo;
use crate::{CapturedError, PyroError};

mod call;
pub mod capability;
mod state;
#[cfg(all(test, feature = "module"))]
mod tests;

use capability::CapabilityLibrary;
pub use state::{PyroModule, PyroState};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum WasmError {
    #[error("Pyro Error: {0}")]
    Pyro(#[from] PyroError),

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

#[derive(Clone)]
pub struct PyroLogs {
    pub module_logs: Vec<String>,
    pub capability_logs: HashMap<(String, String), Vec<String>>,
}

impl PyroLogs {
    pub fn empty() -> Self {
        PyroLogs {
            module_logs: Vec::new(),
            capability_logs: HashMap::new(),
        }
    }
}

#[derive(Clone)]
pub struct PyroSuccess {
    pub row: PyroRow<'static>,
    pub logs: PyroLogs,
}

#[derive(Clone)]
pub struct PyroFailure {
    pub result: Result<CapturedError, String>,
    pub logs: PyroLogs,
}

/// A linker pre-configured to use `PyroState<T>` as store data.
///
/// Host functions registered through `define_async` / `define_sync` receive
/// a clean `(&T, PyroView)` signature — all wasm memory plumbing is hidden.
pub struct PyroFactory {
    engine: Engine,
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
        let engine = Engine::new(&config).map_err(|e| WasmError::EngineError(e.to_string()))?;

        Ok(Self {
            engine,
            libraries,
            configurations,
            module,
        })
    }

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

    pub async fn instantiate(&mut self) -> Result<PyroInstance, WasmError> {
        let pyro_state = PyroState::new();
        let mut store = Store::new(&self.engine, pyro_state);
        let objects = self.create_capabilities().await?;
        let mut linker = Linker::new(&self.engine);

        Self::link_logger(&mut linker)?;
        Self::link_capabilities(&mut linker, &objects)?;
        let instance = linker
            .instantiate_async(&mut store, self.module.module())
            .await
            .map_err(|e| WasmError::InstantiationFailed(e.to_string()))?;
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| WasmError::MissingExport("Missing 'memory'".to_string()))?;

        // Link the PyroState methods to the instance exports
        PyroState::link(&mut store, &instance)?;
        let receiver = RkyvReceiver::new();
        Ok(PyroInstance {
            store,
            instance,
            memory,
            receiver,
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
                linker.func_wrap_async(
                    &class_name,
                        &method_name,
                        move |caller: Caller<'_, PyroState>,
                              (client_ptr, input_ptr): (i32, i32)| {
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

                                let output_vec =
                                    object.call(&fn_name, client_view, input_view).await?;
                                output_vec.parse_as_error()?;

                                let output_view = PyroView::from(&output_vec);
                                let ptr = io.new_input(&output_view).await?;

                                Ok(ptr)
                            })
                        },
                    )
                    .map_err(|e| {
                        WasmError::LinkFunctionFailed(class_name, method_name, format!("Error: {:#}\nBacktrace: {}", e, e.backtrace()))
                    })?;
            }

            let class_name = class_name.clone();
            let object = object.clone();
            linker
                .func_wrap_async(
                    &class_name,
                    "register",
                    move |caller: Caller<'_, PyroState>, (client_ptr,): (i32,)| {
                        let object = object.clone();
                        Box::new(async move {
                            let mut io = PyroCallIo::from_caller(caller)?;

                            // Read input and get state — both are &self borrows.
                            let client_view = io.borrow_argument(client_ptr).await?;

                            // Call user function — consumes both borrows on return.
                            let output_vec = object.register(client_view).await?;
                            output_vec.parse_as_error()?;

                            // Write output back into wasm memory.
                            let output_view = PyroView::from(&output_vec);
                            let ptr = io.new_input(&output_view).await?;

                            Ok(ptr)
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
    receiver: RkyvReceiver<PyroRow<'static>>,
}

impl PyroInstance {
    pub async fn call(&mut self, input: &PyroRow<'_>) -> Result<PyroSuccess, PyroFailure> {
        // Ship the input row via rkyv into a PyroVec
        let input_row_owned = input.to_static();
        let input_vec = input_row_owned
            .ship()
            .map_err(|err| self.pack_pyro_error(err))?;
        let input_view: PyroView<'_> = input_vec.view();

        // 1. Write Input using PyroCallIo
        let mut io = PyroCallIo::new(&mut self.store, self.memory);
        let input_ptr = io
            .new_input(&input_view)
            .await
            .map_err(|err| self.pack_pyro_error(err))?;

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

        let output_ptr = entry
            .call_async(&mut self.store, input_ptr)
            .await
            .map_err(classify_error)
            .map_err(|err| self.pack_pyro_error(err))?;

        // 3. Read Output using PyroCallIo
        let mut io = PyroCallIo::new(&mut self.store, self.memory);
        let output_vec = io
            .get_output(output_ptr)
            .await
            .map_err(|err| self.pack_pyro_error(err))?;

        // 4. Parse the result (Zero-copy view of the host-owned vector)
        let result_view = output_vec.view();
        result_view
            .parse_as_error()
            .map_err(|err| self.pack_pyro_error(err))?;

        match result_view.status() {
            Ok(DataStatus::RkyvValid) => {
                let row =
                    PyroRow::expose_view(result_view).map_err(|err| self.pack_pyro_error(err))?;
                let row = self
                    .receiver
                    .receive(&row)
                    .map_err(|err| self.pack_pyro_error(err))?;
                Ok(self.pack_success(row))
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

/// A single wasm module in the pipeline.
#[derive(Deserialize, Debug)]
pub struct ModuleConfig {
    /// Path to the module directory.
    pub path: PathBuf,
    pub libraries: Vec<PathBuf>,
    /// Per-class capability configuration. Keys are class names.
    #[serde(default)]
    pub configurations: HashMap<String, Option<serde_json::Value>>,
}

impl ModuleConfig {
    pub async fn load_factory(&self) -> Result<PyroFactory, WasmError> {
        let mut config = wasmtime::Config::new();
        config.async_support(true);
        let engine = Engine::new(&config).map_err(|e| WasmError::EngineError(e.to_string()))?;

        let wasm_path = self.path.join("artifacts").join("mod.wasm");
        let binary = std::fs::read(&wasm_path).map_err(|e| {
            WasmError::InstantiationFailed(format!(
                "Failed to read WASM at {}: {}",
                wasm_path.display(),
                e
            ))
        })?;

        let wasmtime_module = wasmtime::Module::from_binary(&engine, &binary).map_err(|e| {
            WasmError::InstantiationFailed(format!("Failed to compile WASM: {}", e))
        })?;
        let pyro_module = PyroModule::new(wasmtime_module)?;

        let mut libs = Vec::new();
        #[cfg(target_os = "linux")]
        let lib_file = "lib.so";
        #[cfg(target_os = "macos")]
        let lib_file = "lib.dylib";
        #[cfg(target_os = "windows")]
        let lib_file = "lib.dll";
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        let lib_file = "lib.so";

        for lib_path in &self.libraries {
            let artifact_path = lib_path.join("artifacts").join(lib_file);
            let lib_name = lib_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            let library = CapabilityLibrary::load(lib_name, &artifact_path)
                .await
                .map_err(|e| {
                    WasmError::InstantiationFailed(format!(
                        "Failed to load capability library at {}: {}",
                        artifact_path.display(),
                        e
                    ))
                })?;
            libs.push(library);
        }

        Ok(PyroFactory {
            engine,
            libraries: libs,
            configurations: self.configurations.clone(),
            module: pyro_module,
        })
    }
}
