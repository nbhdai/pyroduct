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

use wasmtime::{Engine, Instance, Memory, Store, TypedFunc};

use crate::{
    Bridgeable, CapturedError, ParseError, PyroError,
    format::Receiver,
    header::{DataStatus, PyroData, PyroHeader},
    rkyv_8::RkyvReceiver,
    value::PyroRow,
    view::PyroView,
};

mod call;
mod linker;
mod state;
#[cfg(test)]
mod tests;

pub use linker::PyroFactory;
pub use state::{PyroModule, PyroState};
// Import the IO helper
use call::PyroCallIo;

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
// PyroEngine — thin wrapper around wasmtime::Engine
// ---------------------------------------------------------------------------

pub struct PyroEngine {
    engine: Engine,
}

impl PyroEngine {
    /// Create a new engine with async support enabled.
    pub fn new() -> Result<Self, WasmError> {
        let mut config = wasmtime::Config::new();
        config.async_support(true);
        let engine = Engine::new(&config).map_err(|e| WasmError::EngineError(e.to_string()))?;
        Ok(Self { engine })
    }

    /// Access the inner wasmtime `Engine`.
    pub fn engine(&self) -> &Engine {
        &self.engine
    }
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
        PyroLogs { module_logs: Vec::new(), capability_logs: HashMap::new() }
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

pub struct PyroInstance {
    store: Store<PyroState>,
    linker: PyroFactory,
    instance: Instance,
    memory: Memory,
    receiver: RkyvReceiver<PyroRow<'static>>,
}

impl PyroInstance {
    pub async fn new(
        engine: &PyroEngine,
        linker: PyroFactory,
    ) -> Result<Self, WasmError> {
        let pyro_state = PyroState::new();
        let mut store = Store::new(engine.engine(), pyro_state);

        let instance = linker.instantiate_async(&mut store).await?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| WasmError::MissingExport("Missing 'memory'".to_string()))?;

        // Link the PyroState methods to the instance exports
        PyroState::link(&mut store, &instance)?;

        let receiver = RkyvReceiver::new();
        Ok(Self {
            store,
            memory,
            instance,
            receiver,
            linker,
        })
    }

    pub async fn call(
        &mut self,
        input: &PyroRow<'_>,
    ) -> Result<PyroSuccess, PyroFailure> {
        // Ship the input row via rkyv into a PyroVec
        let input_row_owned = input.to_static();
        let input_vec = input_row_owned.ship().map_err(|err| self.pack_pyro_error(err))?;
        let input_view: PyroView<'_> = input_vec.view();

        // 1. Write Input using PyroCallIo
        let mut io = PyroCallIo::new(&mut self.store, self.memory);
        let input_ptr = io.new_input(&input_view).await.map_err(|err| self.pack_pyro_error(err))?;

        // 2. Call the export
        let entry: TypedFunc<i32, i32> = self
            .instance
            .get_typed_func(&mut self.store, "call_extern")
            .map_err(|e| PyroError::CodePanic(CapturedError::new(format!("Missing main function: {}", e)).into()))
            .map_err(|err| self.pack_pyro_error(err))?;

        let output_ptr = entry
            .call_async(&mut self.store, input_ptr)
            .await
            .map_err(classify_error)
            .map_err(|err| self.pack_pyro_error(err))?;

        // 3. Read Output using PyroCallIo
        let mut io = PyroCallIo::new(&mut self.store, self.memory);
        let output_vec = io.get_output(output_ptr).await.map_err(|err| self.pack_pyro_error(err))?;

        // 4. Parse the result (Zero-copy view of the host-owned vector)
        let result_view = output_vec.view();
        result_view.parse_as_error().map_err(|err| self.pack_pyro_error(err))?;

        match result_view.status() {
            Ok(DataStatus::RkyvValid) => {
                let row = PyroRow::expose_view(result_view).map_err(|err| self.pack_pyro_error(err))?;
                let row = self.receiver.receive(&row).map_err(|err| self.pack_pyro_error(err))?;
                Ok(self.pack_success(row))
            }
            Ok(DataStatus::RkyvError) => match serde_json::from_slice(&result_view) {
                Ok(error) => Err(self.pack_user_error(error)),
                Err(error) => Err(self.pack_pyro_error(PyroError::capture_json(error, &*result_view))),
            },
            _ => Err(self.pack_pyro_error(PyroError::Header(ParseError::UnknownStatus(result_view.status_u8())))),
        }
    }

    pub fn unpack_logs(
        &self,
    ) -> PyroLogs {
        let module_logs = self.store.data().log();
        let capability_logs = self.linker.logs();
        PyroLogs {
            module_logs,
            capability_logs
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
