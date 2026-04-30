//! Helpers for getting data in/out of wasm during a host-linked function call.
//!
//! When wasm calls an extern function the host has linked, `PyroCallIo`
//! provides access to pyro methods to read call arguments (`get_input`)
//! and write return data back (`new_return`).

use wasmtime::{AsContextMut, Caller, Extern, Memory};

use crate::{
    format::{
        PyroVec, PyroView, get_view,
        header::{PyroHeader, PyroParser},
    },
    module::{PyroState, WasmError},
};

/// IO helper for reading/writing PyroViews from/to wasm memory.
///
/// Works with any wasmtime context — `Caller`, `&mut Store`, etc.
pub struct PyroCallIo<S: AsContextMut> {
    ctx: S,
    memory: Memory,
}

impl<S: AsContextMut> PyroCallIo<S> {
    /// Construct from any wasmtime context and a pre-resolved `Memory`.
    ///
    /// Use `from_caller` when inside a host callback (resolves memory
    /// automatically). Use this when you already have the `Memory` handle
    /// (e.g. from an `Instance`).
    pub fn new(ctx: S, memory: Memory) -> Self {
        Self { ctx, memory }
    }
}

impl<'a> PyroCallIo<Caller<'a, PyroState>> {
    /// Construct from a wasmtime `Caller` inside a host callback.
    ///
    /// Resolves `"memory"` from the caller's exports automatically.
    pub fn from_caller(mut caller: Caller<'a, PyroState>) -> Result<Self, WasmError> {
        if !caller.data().linked() {
            return Err(WasmError::MissingExport("PyroState not linked".to_string()));
        }

        let memory = match caller.get_export("memory") {
            Some(Extern::Memory(mem)) => mem,
            Some(_) => {
                return Err(WasmError::MissingExport(
                    "export 'memory' exists but is not a Memory".to_string(),
                ));
            }
            None => {
                return Err(WasmError::MissingExport("memory".to_string()));
            }
        };

        Ok(Self {
            ctx: caller,
            memory,
        })
    }
}

// Methods that require PyroState as the store data.
impl<S: AsContextMut<Data = PyroState>> PyroCallIo<S> {
    /// Read a `PyroView` from wasm memory at the given pointer.
    pub async fn get_output(&mut self, ptr: i32) -> Result<PyroVec, WasmError> {
        let wasm_memory = self.memory.data(self.ctx.as_context());
        let view = get_view(wasm_memory, ptr as usize)
            .map_err(|e| WasmError::InputMemory(wasmtime::Error::msg(e.to_string())))?;
        let vec = PyroVec::clone_from_pyro(&view);
        let free_output = self
            .ctx
            .as_context()
            .data()
            .methods()
            .ok_or_else(|| WasmError::MissingExport("PyroState not linked".to_string()))?
            .free_output();
        free_output
            .call_async(&mut self.ctx, ptr)
            .await
            .map_err(|err| WasmError::OutputMemory(err.context("Unable to free an output")))?;
        Ok(vec)
    }

    /// Read a `PyroView` from wasm memory at the given pointer.
    pub async fn borrow_argument(&self, ptr: i32) -> Result<PyroView, WasmError> {
        let wasm_memory = self.memory.data(self.ctx.as_context());
        let view = get_view(wasm_memory, ptr as usize)
            .map_err(|e| WasmError::InputMemory(wasmtime::Error::msg(e.to_string())))?;
        Ok(view)
    }

    /// Allocate a new buffer in wasm memory via `new_input` and copy the
    /// full PyroView (header + payload) into it. Returns the wasm pointer.
    pub async fn new_input(&mut self, data: &PyroView) -> Result<i32, WasmError> {
        let data_len = data.len(); // payload only
        let total_len = PyroParser::HEADER_SIZE + crate::format::vec_buf::INNER_HEADER;

        let new_input = self
            .ctx
            .as_context()
            .data()
            .methods()
            .ok_or_else(|| WasmError::MissingExport("PyroState not linked".to_string()))?
            .new_input();

        let ptr = new_input
            .call_async(&mut self.ctx, data_len as i32)
            .await
            .map_err(WasmError::OutputMemory)?;

        let wasm_memory = self.memory.data_mut(&mut self.ctx);
        let memory_len = wasm_memory.len();
        let dest = wasm_memory
            .get_mut(ptr as usize..ptr as usize + total_len)
            .ok_or_else(|| {
                WasmError::OutputMemory(wasmtime::Error::msg(format!(
                    "wasm pointer {:#x} + {} out of bounds (memory size {})",
                    ptr, total_len, memory_len
                )))
            })?;
        dest.copy_from_slice(data.as_raw_slice());

        Ok(ptr)
    }

    /// Allocate a new buffer in wasm memory via `new_input` and copy the
    /// full PyroView (header + payload) into it. Returns the wasm pointer.
    pub fn log(&self, ptr: i32, len: i32) -> Result<i32, WasmError> {
        let wasm_memory = self.memory.data(self.ctx.as_context());
        let start = ptr as usize;
        let end = start + len as usize;
        let slice = &wasm_memory[start..end];
        let data = self.ctx.as_context().data();
        data.module_log(slice);

        Ok(len)
    }
}
