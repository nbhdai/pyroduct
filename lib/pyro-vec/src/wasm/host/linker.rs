//! Thin wrapper around `wasmtime::Linker` for defining host functions.
//!
//! The user-facing API hides all wasm pointer IO. A host function receives
//! `(&T, PyroView)` and returns a `PyroVec`. The linker handles:
//!
//!   1. Reading the input pointer from wasm memory → `PyroView`
//!   2. Calling the user function with `(&T, PyroView) → Result<PyroVec>`
//!   3. Writing the returned `PyroVec` back into wasm memory via `new_return`
//!   4. Returning the wasm pointer to the guest
//!
//! Errors are automatically encoded as `CapturedError` PyroVecs so the
//! guest always receives a valid pyro packet.

use std::future::Future;

use wasmtime::{Caller, Linker};

use super::{PyroState, WasmError};
use crate::{
    PyroVec,
    view::{PyroView},
    wasm::host::call::PyroCallIo,
};

/// A linker pre-configured to use `PyroState<T>` as store data.
///
/// Host functions registered through `define_async` / `define_sync` receive
/// a clean `(&T, PyroView)` signature — all wasm memory plumbing is hidden.
pub struct PyroLinker<T: 'static> {
    linker: Linker<PyroState<T>>,
}

impl<T: Send + Sync + 'static> PyroLinker<T> {
    /// Create a new linker for the given engine.
    pub fn new(engine: &wasmtime::Engine) -> Self {
        Self {
            linker: Linker::new(engine),
        }
    }

    /// Define an **async** host function in the given module namespace.
    ///
    /// The wasm guest calls this as `fn(i32) -> i32` (input ptr → output ptr).
    /// The linker reads the input `PyroView` from wasm memory, invokes
    /// your callback with `(&T, PyroView)`, and writes the resulting
    /// `PyroVec` back. Errors are encoded as `CapturedError` packets.
    pub fn define_async<F, Fut>(
        &mut self,
        module: &str,
        name: &str,
        func: F,
    ) -> Result<(), WasmError>
    where
        F: for<'a> FnOnce(PyroView<'a>) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = PyroVec> + Send,
    {
        let mod_name = module.to_string();
        let fn_name = name.to_string();

        self.linker
            .func_wrap_async(
                module,
                name,
                move |caller: Caller<'_, PyroState<T>>, (input_ptr,): (i32,)| {
                    let f = func.clone();
                    Box::new(async move {
                        let mut io = PyroCallIo::from_caller(caller)?;

                        // Read input and get state — both are &self borrows.
                        let input_view = io.get_output(input_ptr).await?;

                        // Call user function — consumes both borrows on return.
                        let output_vec = f(input_view.view()).await;

                        // Write output back into wasm memory.
                        let output_view = PyroView::from(&output_vec);
                        let ptr = io.new_input(&output_view).await?;

                        Ok((ptr,))
                    })
                },
            )
            .map_err(|e| {
                WasmError::LinkFunctionFailed(mod_name, fn_name, e.to_string())
            })?;

        Ok(())
    }

    /// Define a **synchronous** host function in the given module namespace.
    ///
    /// Same contract as `define_async` but the callback is not async.
    pub fn define_sync<F>(
        &mut self,
        module: &str,
        name: &str,
        func: F,
    ) -> Result<(), WasmError>
    where
        F: for<'a> Fn(PyroView<'a>) -> PyroVec + Clone
            + Send
            + Sync
            + 'static,
    {
        let mod_name = module.to_string();
        let fn_name = name.to_string();

        self.linker
            .func_wrap_async(
                module,
                name,
                move |caller: Caller<'_, PyroState<T>>, (input_ptr,): (i32,)| {
                    let f = func.clone();
                    Box::new(async move {
                        let mut io = PyroCallIo::from_caller(caller)?;

                        // Read input and call user function.
                        let input_view = io.get_output(input_ptr).await?;
                        let output_vec = f(input_view.view());

                        // Write output back into wasm memory.
                        let output_view = PyroView::from(&output_vec);
                        let ptr = io.new_input(&output_view).await?;

                        Ok((ptr,))
                    })
                },
            )
            .map_err(|e| {
                WasmError::LinkFunctionFailed(mod_name, fn_name, e.to_string())
            })?;

        Ok(())
    }

    /// Raw access to the inner wasmtime `Linker` for advanced use cases.
    pub fn inner(&self) -> &Linker<PyroState<T>> {
        &self.linker
    }

    /// Mutable raw access to the inner wasmtime `Linker`.
    pub fn inner_mut(&mut self) -> &mut Linker<PyroState<T>> {
        &mut self.linker
    }
}