#[allow(unused_imports)]
use std::future::Future;

use wasmtime::{Caller, Instance, Linker, Store};

use super::{PyroState, WasmError};
use crate::ffi::host::capability::Capability;
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
pub struct PyroLinker {
    linker: Linker<PyroState>,
}

impl PyroLinker {
    /// Create a new linker for the given engine.
    pub fn new(
        engine: &wasmtime::Engine,
        capabilities: Vec<Capability>,
    ) -> Result<Self, WasmError> {
        let mut linker = Self {
            linker: Linker::new(engine),
        };
        linker.link_capabilities(capabilities)?;
        Ok(linker)
    }

    pub async fn instantiate_async(
        &self,
        store: &mut Store<PyroState>,
        module: &PyroModule,
    ) -> Result<Instance, WasmError> {
        self.linker
            .instantiate_async(store, module.module())
            .await
            .map_err(|e| WasmError::InstantiationFailed(e.to_string()))
    }

    /// Links all capabilities from the provided libraries into the linker.
    fn link_capabilities(&mut self, capabilities: Vec<Capability>) -> Result<(), WasmError> {
        for lib in capabilities.iter() {
            for (class_name, object) in lib.iter() {
                // Capture lib for the closures (Arc clone is cheap)

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
}
