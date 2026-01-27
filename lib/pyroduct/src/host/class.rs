use std::{ffi::c_void, path::{Path, PathBuf}};

use tracing::{error, info};
use wasmtime::Caller;

use crate::{PyroductResult, capability_host::ffi::{ClassDropFn, ClassExport, ClassInitFn, ClassResetFn, Function, FunctionExport, SyncFn}, errors::PyroductError, host::{ffi_bridge::{AsyncExecFuture, ExecutionResultBridge}, function::CapFunction, linker::WasmArgs, wasm_link::HarnessState}};

/// Represents a loaded class from a dynamic library
pub struct CapClass {
    pub imports: Vec<CapFunction>,
    pub init_fn: ClassInitFn<'static>,
    pub reset_fn: ClassResetFn<'static>,
    pub destroy_fn: ClassDropFn,
}

impl CapClass {
    pub fn new(class: ClassExport<'static>) -> Self {
        let exports: &[FunctionExport<'static>] = unsafe { std::slice::from_raw_parts(class.ptr, class.len) };
        let mut imports = Vec::new();

        for export in exports {
            let cap_name = std::str::from_utf8(unsafe { std::slice::from_raw_parts(
                export.module,
                export.module_len,
            ) })
            .unwrap_or("unknown_mod")
            .to_string();

            let func_name =
                std::str::from_utf8(unsafe { std::slice::from_raw_parts(export.name, export.name_len) })
                    .unwrap_or("unknown_func")
                    .to_string();

            let pointer =
                unsafe { std::mem::transmute::<Function<'_>, Function<'static>>(export.func) };
            let func = CapFunction {
                cap_name,
                func_name,
                pointer,
            };
            imports.push(func);
        }
        let init_fn =
                unsafe { std::mem::transmute::<ClassInitFn<'_>, ClassInitFn<'static>>(class.init) };
            let reset_fn =
                unsafe { std::mem::transmute::<ClassResetFn<'_>, ClassResetFn<'static>>(class.reset) };
        
        Self {
                imports,
                init_fn,
                reset_fn,
                destroy_fn: class.drop,
            }
    }
}

/// Represents the state of a single class instance within a capability
pub struct ClassState {
    pub ptr: *mut c_void,
    pub destroy_fn: ClassDropFn,
}

// Safety: The pointer is opaque and managed by the plugin
unsafe impl Send for ClassState {}

impl Drop for ClassState {
    fn drop(&mut self) {
        match self.destroy_fn {
            ClassDropFn::Sync(destroy_fn) => {
                if !self.ptr.is_null() {
                    unsafe { (destroy_fn)(self.ptr) }
                }
            }
            ClassDropFn::Null => {}
        }
    }
}


impl CapClass {
    /// Handles the "Preparation" phase: getting host state, validating memory bounds,
    /// and calculating raw pointers.
    fn prepare_io(
        &self,
        caller: &mut Caller<'_, HarnessState>,
        cap_index: usize,
        class_index: Option<usize>,
        args: WasmArgs,
        wasm_name: &str,
        wasm_path: &Path,
    ) -> Result<(*mut c_void, *mut u8, *const u8), PyroductError> {
        let (wasm_state_ptr, wasm_state_len, ptr, len) = args;

        let host_state_ptr = if let Some(c_idx) = class_index {
            caller
                .data()
                .cap_states
                .get(cap_index)
                .map(|s| s.get_class_ptr(c_idx))
                .unwrap_or(std::ptr::null_mut())
        } else {
            std::ptr::null_mut()
        };

        // Calculate pointers
        let input_ptr = mem_slice.as_ptr().wrapping_add(ptr as usize);
        let w_state_ptr = mem_slice.as_mut_ptr().wrapping_add(wasm_state_ptr as usize);

        Ok((host_state_ptr, w_state_ptr, input_ptr))
    }

    /// Handles the "Finalization" phase: allocating memory in WASM and writing the result.
    async fn finalize_io(
        &self,
        mut caller: &mut Caller<'_, HarnessState>,
        output_vec: Vec<u8>,
        wasm_name: &str,
        wasm_path: &Path,
    ) -> Result<i32, PyroductError> {
        let total_len = 4 + output_vec.len();

        // Get alloc function
        let alloc = caller
            .get_export("alloc")
            .ok_or_else(|| {
                PyroductError::from_module_linking(
                    wasm_name.to_string(),
                    wasm_path.to_path_buf(),
                    "Missing alloc",
                )
            })?
            .into_func()
            .ok_or_else(|| {
                PyroductError::from_module_linking(
                    wasm_name.to_string(),
                    wasm_path.to_path_buf(),
                    "Alloc is not a function",
                )
            })?;

        let alloc_typed = alloc.typed::<i32, i32>(&mut caller).map_err(|err| {
            PyroductError::from_module_linking(
                wasm_name.to_string(),
                wasm_path.to_path_buf(),
                format!("Alloc has incorrect function signature: {err}"),
            )
        })?;

        // Call alloc (async)
        let result_ptr = alloc_typed
            .call_async(&mut caller, total_len as i32)
            .await
            .map_err(|err| {
                PyroductError::from_module_linking(
                    wasm_name.to_string(),
                    wasm_path.to_path_buf(),
                    format!("Allocation failed: {err}"),
                )
            })?;

        // Re-acquire memory
        let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
        let (mem_slice, _) = memory.data_and_store_mut(&mut caller);

        // Write result
        unsafe {
            let dest_ptr = mem_slice.as_mut_ptr().add(result_ptr as usize);
            *(dest_ptr as *mut u32) = output_vec.len() as u32;
            std::ptr::copy_nonoverlapping(output_vec.as_ptr(), dest_ptr.add(4), output_vec.len());
        }

        Ok(result_ptr)
    }

    /// Executes a Sync capability call
    async fn process_sync_call(
        &self,
        mut caller: Caller<'_, HarnessState>,
        raw_fn: crate::capability_host::ffi::SyncFn,
        args: WasmArgs,
        cap_index: usize,
        class_index: Option<usize>,
        wasm_name: String,
        wasm_path: PathBuf,
    ) -> i32 {
        let (_, wasm_state_len, _, input_len) = args;

        let (host_state_ptr, w_state_ptr, input_ptr) = match self.prepare_io(
            &mut caller,
            cap_index,
            class_index,
            args,
            &wasm_name,
            &wasm_path,
        ) {
            Ok(ptrs) => ptrs,
            Err(e) => {
                caller.data_mut().error_slot = Some(e);
                return 0;
            }
        };

        info!("Entering unsafe plugin function...");
        let result = unsafe {
            raw_fn(
                w_state_ptr,
                wasm_state_len as usize,
                input_ptr,
                input_len as usize,
                host_state_ptr,
            )
        };
        info!("Exited unsafe plugin function.");

        let output_vec = match unsafe {
            ExecutionResultBridge::from_ffi(result, self.name(), self.path().unwrap().to_path_buf())
        } {
            Ok(v) => v,
            Err(e) => {
                caller.data_mut().error_slot = Some(e);
                return 0;
            }
        };

        match self
            .finalize_io(&mut caller, output_vec, &wasm_name, &wasm_path)
            .await
        {
            Ok(ptr) => ptr,
            Err(e) => {
                caller.data_mut().error_slot = Some(e);
                0
            }
        }
    }

    /// Executes an Async capability call
    async fn process_async_call(
        &self,
        mut caller: Caller<'_, HarnessState>,
        raw_fn: crate::capability_host::ffi::AsyncFn<'static>,
        args: WasmArgs,
        cap_index: usize,
        class_index: Option<usize>,
        wasm_name: String,
        wasm_path: PathBuf,
    ) -> i32 {
        let (_, wasm_state_len, _, input_len) = args;

        let (host_state_ptr, w_state_ptr, input_ptr) = match self.prepare_io(
            &mut caller,
            cap_index,
            class_index,
            args,
            &wasm_name,
            &wasm_path,
        ) {
            Ok(ptrs) => ptrs,
            Err(e) => {
                caller.data_mut().error_slot = Some(e);
                return 0;
            }
        };

        info!("Entering unsafe async plugin function...");
        let fut = unsafe {
            raw_fn(
                w_state_ptr,
                wasm_state_len as usize,
                input_ptr,
                input_len as usize,
                host_state_ptr,
            )
        };

        let exec_fut = AsyncExecFuture::new(fut, self.name(), self.path().unwrap().to_path_buf());
        let output_vec = match exec_fut.await {
            Ok(v) => v,
            Err(e) => {
                info!("Exited unsafe async plugin function (Error).");
                caller.data_mut().error_slot = Some(e);
                return 0;
            }
        };
        info!("Exited unsafe async plugin function (Success).");

        match self
            .finalize_io(&mut caller, output_vec, &wasm_name, &wasm_path)
            .await
        {
            Ok(ptr) => ptr,
            Err(e) => {
                caller.data_mut().error_slot = Some(e);
                0
            }
        }
    }
}
