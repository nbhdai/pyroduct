use std::ffi::c_void;

use tracing::error;
use wasmtime::{Caller, Extern, Memory, TypedFunc};

use crate::{ModIdentity, errors::PyroductError, host::harness::HarnessState};

pub struct WasmMemory<'a> {
    ident: ModIdentity,
    caller: Caller<'a, HarnessState>,
    memory: Memory,
    alloc: TypedFunc<i32, i32>,
    _dealloc: TypedFunc<(i32, i32), ()>,
}

impl<'a> WasmMemory<'a> {
    pub fn from_caller(
        mut caller: Caller<'a, HarnessState>,
    ) -> Result<Self, (Caller<'a, HarnessState>, PyroductError)> {
        let ident = caller.data().module.clone();

        // --- Memory ---
        let memory = match caller.get_export("memory") {
            Some(Extern::Memory(memory)) => memory,
            Some(_) => {
                return Err((
                    caller,
                    PyroductError::from_module_linking(
                        &ident,
                        "The 'memory' in the module isn't actually memory",
                    ),
                ))
            }
            None => {
                return Err((
                    caller,
                    PyroductError::from_module_linking(&ident, "Missing 'memory'"),
                ))
            }
        };

        // --- Alloc ---
        let alloc_func = match caller.get_export("alloc") {
            Some(Extern::Func(func)) => func,
            Some(_) => {
                return Err((
                    caller,
                    PyroductError::from_module_linking(&ident, "Alloc is not a function"),
                ))
            }
            None => {
                return Err((
                    caller,
                    PyroductError::from_module_linking(&ident, "Missing alloc"),
                ))
            }
        };

        let alloc = match alloc_func.typed::<i32, i32>(&mut caller) {
            Ok(typed_func) => typed_func,
            Err(err) => {
                return Err((
                    caller,
                    PyroductError::from_module_linking(
                        &ident,
                        format!("Alloc has incorrect function signature: {err}"),
                    ),
                ))
            }
        };

        // --- Dealloc ---
        let dealloc_func = match caller.get_export("dealloc") {
            Some(Extern::Func(func)) => func,
            Some(_) => {
                return Err((
                    caller,
                    PyroductError::from_module_linking(&ident, "Dealloc is not a function"),
                ))
            }
            None => {
                return Err((
                    caller,
                    PyroductError::from_module_linking(&ident, "Missing dealloc"),
                ))
            }
        };

        let _dealloc = match dealloc_func.typed::<(i32, i32), ()>(&mut caller) {
            Ok(typed_func) => typed_func,
            Err(err) => {
                return Err((
                    caller,
                    PyroductError::from_module_linking(
                        &ident,
                        format!("Dealloc has incorrect function signature: {err}"),
                    ),
                ))
            }
        };

        Ok(Self {
            ident,
            memory,
            alloc,
            _dealloc,
            caller,
        })
    }

    pub fn get_slice(&self, pointer: i32, len: i32) -> Option<&[u8]> {
        let mem_slice = self.memory.data(&self.caller);
        let start = pointer as usize;
        let end = start + len as usize;
        // Validate bounds
        if start > mem_slice.len() || end > mem_slice.len()  {
            error!("Segfault Risk: Input pointer out of WASM memory bounds!");
            self.write_error(PyroductError::from_module_linking(
                        &self.ident,
                
                "Returned memory pointer out of bounds",
            ));
            return None;
        }
        Some(&mem_slice[start..end])
    }

    pub async fn write(&mut self, data: &[u8]) -> Option<i32> {
        let result_ptr = self
            .alloc
            .call_async(&mut self.caller, data.len() as i32)
            .await
            .map_err(|err| {
                self.write_error(
                    PyroductError::from_module_linking(
                        &self.ident,
                    format!("Allocation failed: {err}"),
                ))
            })
            .ok()?;
        let mem_slice = self.memory.data_mut(&mut self.caller);
        
        let start = result_ptr as usize;
        let end = start + data.len();
        // Write result
        if start > mem_slice.len() || end > mem_slice.len()  {
            error!("Segfault Risk: Input pointer out of WASM memory bounds!");
            self.write_error(PyroductError::from_module_linking(
                        &self.ident,

                "Written pointer out of bounds",
            ));
            return None;
        }
        let dest_slice = &mut mem_slice[start..end];
        dest_slice.copy_from_slice(data);

        Some(result_ptr)
    }

    pub fn class_state(&mut self, 
        state_index: usize,
        class_index: usize,
    ) -> *mut c_void {
        self.caller
            .data()
            .cap_states
            .get(state_index)
            .map(|s| s.get_class_ptr(class_index))
            .unwrap_or(std::ptr::null_mut())
    }

    pub fn write_error(&self, error: PyroductError) {
        self.caller.data().set_error(error);
    }
}