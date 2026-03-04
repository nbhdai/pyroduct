use std::collections::HashMap;
#[allow(unused_imports)]
use std::future::Future;

use wasmtime::{Caller, Instance, Linker, Store};

use super::{PyroState, WasmError};
use crate::ffi::host::{Capability, CapabilityLibrary};
use crate::pipeline::wasm::PyroModule;
use crate::pipeline::wasm::call::PyroCallIo;
use crate::{
    header::PyroData,
    view::PyroView,
};

/// A linker pre-configured to use `PyroState<T>` as store data.
///
/// Host functions registered through `define_async` / `define_sync` receive
/// a clean `(&T, PyroView)` signature — all wasm memory plumbing is hidden.
pub struct PyroFactory {
    linker: Linker<PyroState>,
    configurations: HashMap<String, serde_json::Value>,
    libraries: Vec<CapabilityLibrary>,
    module: PyroModule,
}

impl PyroFactory {
    /// Create a new linker for the given engine.
    pub fn new(
        engine: &wasmtime::Engine,
        libraries: Vec<CapabilityLibrary>,
        configurations: HashMap<String, serde_json::Value>,
        module: PyroModule,
    ) -> Result<Self, WasmError> {
        let mut linker = Self {
            linker: Linker::new(engine),
            libraries,
            configurations,
            module,
        };
        linker.link_logger()?;
        linker.link_capabilities()?;
        Ok(linker)
    }

    pub async fn create_capabilities(
        &self,
    ) -> Result<Vec<Capability>, WasmError> {
        self.linker
            .instantiate_async(store, self.module.module())
            .await
            .map_err(|e| WasmError::InstantiationFailed(e.to_string()))
    }

    pub async fn instantiate_async(
        &self,
        store: &mut Store<PyroState>,
    ) -> Result<Instance, WasmError> {
        self.linker
            .instantiate_async(store, self.module.module())
            .await
            .map_err(|e| WasmError::InstantiationFailed(e.to_string()))
    }

    fn link_logger(&mut self) -> Result<(), WasmError> {
        self.linker
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
    fn link_capabilities(&mut self) -> Result<(), WasmError> {
        for lib in self.capabilities.iter() {
            for (class_name, object) in lib.iter() {
                // Capture lib for the closures (Arc clone is cheap)
                if !self.module.has_class(class_name) {
                    continue;
                }

                for method_name in object.method_names() {
                    let method_name = method_name.to_string();

                    let class_name = class_name.clone();
                    let fn_name = method_name.clone();
                    let object = object.clone();

                    self.linker
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
                self.linker
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
        }
        Ok(())
    }

    pub fn logs(&self) -> HashMap<(String, String), Vec<String>> {
        let mut logs = HashMap::new();
        for cap in &self.capabilities {
            let lib_name = cap.name();
            for (name, object) in cap.iter() {
                let object_logs = object.take_logs();
                logs.insert((lib_name.to_string(), name.clone()), object_logs);
            }
        }

        logs
    }
}
