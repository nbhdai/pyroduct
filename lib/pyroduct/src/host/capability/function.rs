use tracing::info;
use wasmtime::Linker;

use crate::{CapIdentity, PyroductResult, capability_host::ffi::{Function, FunctionExport}, host::{capability::WasmArgs, ffi_bridge::{AsyncExecFuture, ExecutionResultBridge}, harness::HarnessState, wasm_bridge::WasmMemory}};

#[derive(Clone)]
pub struct CapFunction {
    pub ident: CapIdentity,
    pub cap_name: String,
    pub func_name: String,
    pub pointer: Function<'static>,
}

impl CapFunction {
    pub fn new(ident: &CapIdentity, func: &FunctionExport<'_>) -> Self {
        let cap_name = std::str::from_utf8(unsafe { std::slice::from_raw_parts(
            func.module,
            func.module_len,
        ) })
        .unwrap_or("unknown_mod")
        .to_string();

        let func_name =
            std::str::from_utf8(unsafe { std::slice::from_raw_parts(func.name, func.name_len) })
                .unwrap_or("unknown_func")
                .to_string();

        let pointer =
            unsafe { std::mem::transmute::<Function<'_>, Function<'static>>(func.func) };
        let func = CapFunction {
            ident: ident.clone(),
            cap_name,
            func_name,
            pointer,
        };
        func
    }

    /// Executes a Sync capability call
    async fn process_sync_call(
        &self,
        caller: &mut WasmMemory<'_>,
        raw_fn: crate::capability_host::ffi::SyncFn,
        args: WasmArgs,
    ) -> Option<i32> {
        let (_, _, input_ptr, input_len) = args;
        let input = caller.get_slice(input_ptr, input_len)?;

        info!("Entering unsafe plugin function...");
        let result = unsafe {
            raw_fn(
                std::ptr::null_mut(),
                0,
                input.as_ptr(),
                input.len(),
                std::ptr::null_mut(),
            )
        };
        info!("Exited unsafe plugin function.");

        let output_vec = match unsafe {
            ExecutionResultBridge::from_ffi(result, &self.ident)
        } {
            Ok(v) => v,
            Err(e) => {
                caller.write_error(e);
                return None;
            }
        };

        caller.write(&output_vec).await
    }

    /// Executes an Async capability call
    async fn process_async_call(
        &self,
        caller: &mut WasmMemory<'_>,
        raw_fn: crate::capability_host::ffi::AsyncFn<'static>,
        args: WasmArgs,
    ) -> Option<i32> {
        let (_, _, input_ptr, input_len) = args;
        let input = caller.get_slice(input_ptr, input_len)?;

        info!("Entering unsafe async plugin function...");
        let fut = unsafe {
            raw_fn(
                std::ptr::null_mut(),
                0,
                input.as_ptr(),
                input.len(),
                std::ptr::null_mut(),
            )
        };

        let exec_fut = AsyncExecFuture::new(fut, &self.ident);
        let output_vec = match exec_fut.await {
            Ok(v) => v,
            Err(e) => {
                info!("Exited unsafe async plugin function (Error).");
                caller.write_error(e);
                return None;
            }
        };
        info!("Exited unsafe async plugin function (Success).");

       caller.write(&output_vec).await
    }

    pub fn link(&self, linker: &mut Linker<HarnessState>, cap_index: usize) -> PyroductResult<()> {
        let cap_name = self.cap_name.clone();
        let func_name = self.func_name.clone();
        let cap = self.clone();
        match self.pointer {
            Function::Sync(raw_fn) => {
                linker.func_wrap_async(
                    &self.cap_name,
                    &self.func_name,
                    move |caller, args: (i32, i32, i32, i32)| {
                        let cap_name = cap_name.clone();
                        let func_name = func_name.clone();
                        let cap = cap.clone();
                        Box::new(async move {
                            let mut memory = match WasmMemory::from_caller(caller) {
                            Ok(memory) => memory,
                                Err((mut caller, error)) => {
                                    return Err(caller.data_mut().set_error(error));
                                },
                            };
                            info!(
                                "[Plugin -> Capability] Sync Call: {}::{} (CapIdx: {}) | Ptr: {:#x}, Len: {}", 
                                cap_name, func_name, cap_index, args.2, args.3
                            );
                            // DELEGATE TO CAPABILITY EXTENSION
                            match cap.process_sync_call(&mut memory, raw_fn, args).await {
                                Some(point) => Ok(point),
                                None => Ok(0),
                            }
                        })
                    },
                ).expect("Failed to link sync function");
                Ok(())
            }
            Function::Async(raw_fn) => {
                linker.func_wrap_async(
                    &self.cap_name,
                    &self.func_name,
                    move |caller, args: (i32, i32, i32, i32)| {
                        let cap_name = cap_name.clone();
                        let func_name = func_name.clone();
                        let cap = cap.clone();
                        Box::new(async move {
                            let mut memory = match WasmMemory::from_caller(caller) {
                            Ok(memory) => memory,
                                Err((mut caller, error)) => {
                                    return Err(caller.data_mut().set_error(error));
                                },
                            };
                            info!(
                                "[Plugin -> Capability] Sync Call: {}::{} (CapIdx: {}) | Ptr: {:#x}, Len: {}", 
                                cap_name, func_name, cap_index, args.2, args.3
                            );
                            // DELEGATE TO CAPABILITY EXTENSION
                            match cap.process_async_call(&mut memory, raw_fn, args).await {
                                Some(point) => Ok(point),
                                None => Ok(0),
                            }
                        })
                    },
                ).expect("Failed to link sync function");
                Ok(())
            }
        }
    }
}