use std::collections::HashMap;
#[allow(unused_imports)]
use std::future::Future;

use wasmtime::{Caller, Engine, Instance, Linker, Memory, Store, TypedFunc};

use super::{PyroState, WasmError};
use crate::format::Receiver;
use crate::header::{DataStatus, PyroHeader};
use crate::{Bridgeable, CapturedError, ParseError, PyroError, PyroRow};
use crate::ffi::ForeignObject;
use crate::ffi::host::CapabilityLibrary;
use crate::pipeline::wasm::{PyroFailure, PyroLogs, PyroModule, PyroSuccess};
use crate::pipeline::wasm::call::PyroCallIo;
use crate::rkyv_8::RkyvReceiver;
use crate::{
    header::PyroData,
    view::PyroView,
};

/// A linker pre-configured to use `PyroState<T>` as store data.
///
/// Host functions registered through `define_async` / `define_sync` receive
/// a clean `(&T, PyroView)` signature — all wasm memory plumbing is hidden.
pub struct PyroFactory {
    engine: Engine,
    configurations: HashMap<String, serde_json::Value>,
    libraries: Vec<CapabilityLibrary>,
    module: PyroModule,
}

impl PyroFactory {
    /// Create a new linker for the given engine.
    pub fn new(
        libraries: Vec<CapabilityLibrary>,
        configurations: HashMap<String, serde_json::Value>,
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

    async fn create_capabilities(
        &self,
    ) -> Result<HashMap<String, ForeignObject>, WasmError> {
        let mut objects = HashMap::new();
        for (class, config) in &self.configurations {
            for library in self.libraries.iter() {
                if let Ok(object) = library.instantiate_class(class, config).await {
                    objects.insert(class.clone(), object);
                }
            }
        }

        Ok(objects)
    }

    pub async fn instantiate(
        &mut self,
    ) -> Result<PyroInstance, WasmError> {
        let pyro_state = PyroState::new();
        let mut store = Store::new(&self.engine, pyro_state);
        let objects = self.create_capabilities().await?;
        let mut linker = Linker::new(&self.engine);

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

    fn link_logger(linker: &mut Linker<PyroState>, ) -> Result<(), WasmError> {
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
    fn link_capabilities(linker: &mut Linker<PyroState>, objects: &HashMap<String, ForeignObject>) -> Result<(), WasmError> {
        for (class_name, object) in objects.iter() {
            // Capture lib for the closures (Arc clone is cheap)

            for method_name in object.method_names() {
                let method_name = method_name.to_string();

                let class_name = class_name.clone();
                let fn_name = method_name.clone();
                let object = object.clone();

                linker
                    .func_wrap_async(
                        &class_name,
                        &method_name,
                        move |caller: Caller<'_, PyroState>,
                                (client_ptr, input_ptr): (i32, i32)| {
                            let object = object.clone();
                            let fn_name = fn_name.clone();
                            Box::new(async move {
                                tracing::debug!(class_name = object.name(), fn_name, "Calling function");
                                let mut io = PyroCallIo::from_caller(caller)?;

                                // Read input and get state — both are &self borrows.
                                let client_view = io.borrow_argument(client_ptr).await?;
                                let input_view = io.borrow_argument(input_ptr).await?;

                                // Call user function — consumes both borrows on return.
                                let output_vec =
                                    object.call(&fn_name, client_view, input_view).await?;
                                output_vec.parse_as_error()?;

                                // Write output back into wasm memory.
                                let output_view = PyroView::from(&output_vec);
                                let ptr = io.new_input(&output_view).await?;

                                Ok((ptr,))
                            })
                        },
                    )
                    .map_err(|e| {
                        WasmError::LinkFunctionFailed(class_name, method_name, e.to_string())
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

                            Ok((ptr,))
                        })
                    },
                )
                .map_err(|e| {
                    WasmError::LinkFunctionFailed(
                        class_name,
                        "register".to_string(),
                        e.to_string(),
                    )
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
        let capability_logs = self.logs();
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

    pub fn logs(&self) -> HashMap<(String, String), Vec<String>> {
        let mut logs = HashMap::new();
        for object in self.objects.values() {
            let object_logs = object.take_logs();
            logs.insert((object.lib_name().to_string(), object.name().to_string()), object_logs);
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
