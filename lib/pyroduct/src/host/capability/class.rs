use std::{ffi::c_void, pin::Pin, ptr, task::{Context, Poll}};

use pin_project::pin_project;
use tracing::{error, info, trace};
use wasmtime::Linker;

use crate::{CapIdentity, PyroductResult, capability_host::ffi::{ClassDropFn, ClassExport, ClassInitFn, ClassResetFn, FfiBorrowedFutureObjectResult, FfiInitResult, Function, FunctionExport}, errors::{FfiError, PyroductError}, host::{capability::WasmArgs, ffi_bridge::{AsyncExecFuture, ExecutionResultBridge, InitResultBridge}, harness::HarnessState, wasm_bridge::WasmMemory}};

use super::CapFunction;

/// Represents a loaded class from a dynamic library
#[derive(Clone)]
pub struct CapClass {
    pub ident: CapIdentity,
    pub imports: Vec<CapFunction>,
    pub init_fn: ClassInitFn<'static>,
    pub reset_fn: ClassResetFn<'static>,
    pub destroy_fn: ClassDropFn,
}

impl CapClass {
    pub fn new(ident: &CapIdentity, class: &ClassExport<'_>) -> Self {
        let exports: &[FunctionExport<'_>] = unsafe { std::slice::from_raw_parts(class.ptr, class.len) };

        let mut imports = Vec::new();

        for export in exports {
            let func = CapFunction::new(ident, export);
            imports.push(func);
        }
        let init_fn =
                unsafe { std::mem::transmute::<ClassInitFn<'_>, ClassInitFn<'static>>(class.init) };
        let reset_fn =
                unsafe { std::mem::transmute::<ClassResetFn<'_>, ClassResetFn<'static>>(class.reset) };
        
        Self {
            ident: ident.clone(),
            imports,
            init_fn,
            reset_fn,
            destroy_fn: class.drop,
        }
    }

    pub fn init(&self, config: Option<&serde_json::Value>) -> PyroductResult<CapabilityInit<'static>> {
        let (config_ptr, config_len, config_bytes) = match config {
            Some(value) => {
                let config_bytes = serde_json::to_vec(value).expect("AARRGG");
                (config_bytes.as_ptr(), config_bytes.len(), config_bytes)
            }
            None => (ptr::null(), 0, Vec::new()),
        };

        let capability_init = match self.init_fn {
            ClassInitFn::Sync(func) => {
                let res = unsafe { func(config_ptr, config_len) };
                let state = unsafe {
                    InitResultBridge::from_ffi(res, &self.ident)?
                };
                CapabilityInit::Sync {
                    reset_fn: self.reset_fn.clone(),
                    state: Some(state),
                    destroy_fn: self.destroy_fn,
                }
            }
            ClassInitFn::Async(func) => {
                let fut_res = unsafe { func(config_ptr, config_len) };
                let future = AsyncInitFuture::new(fut_res, &self.ident);
                let future: AsyncInitFuture<'static> = unsafe { std::mem::transmute(future) };

                CapabilityInit::Async {
                    reset_fn: self.reset_fn.clone(),
                    config_bytes,
                    future,
                    destroy_fn: self.destroy_fn,
                }
            }
            ClassInitFn::Null => CapabilityInit::Null,
        };

        Ok(capability_init)
    }


    /// Executes a Sync capability call
    async fn process_sync_call(
        &self,
        caller: &mut WasmMemory<'_>,
        raw_fn: crate::capability_host::ffi::SyncFn,
        args: WasmArgs,
        cap_index: usize,
        class_index: usize,
    ) -> Option<i32> {
        let (client_ptr, client_len, input_ptr, input_len) = args;
        let host_state_ptr = caller.class_state(cap_index, class_index);
        let input = caller.get_slice(input_ptr, input_len)?;
        let client =  caller.get_slice(client_ptr, client_len)?;

        info!("Entering unsafe plugin function...");
        let result = unsafe {
            raw_fn(
                client.as_ptr(),
                client.len(),
                input.as_ptr(),
                input.len(),
                host_state_ptr,
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
        cap_index: usize,
        class_index: usize,
    ) -> Option<i32> {
        let (client_ptr, client_len, input_ptr, input_len) = args;
        let host_state_ptr = caller.class_state(cap_index, class_index);
        let input = caller.get_slice(input_ptr, input_len)?;
        let client =  caller.get_slice(client_ptr, client_len)?;

        info!("Entering unsafe async plugin function...");
        let fut = unsafe {
            raw_fn(
                client.as_ptr(),
                client.len(),
                input.as_ptr(),
                input.len(),
                host_state_ptr,
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
        for (class_index, func) in self.imports.iter().enumerate() {
            let cap = self.clone();
            let cap_name = func.cap_name.clone();
            let func_name = func.func_name.clone();
            match func.pointer {
                Function::Sync(raw_fn) => {
                    
                    linker.func_wrap_async(
                        &func.cap_name,
                        &func.func_name,
                        move |caller, args: (i32, i32, i32, i32)| {
                            let cap = cap.clone();
                            let cap_name = cap_name.clone();
                            let func_name = func_name.clone();
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
                                match cap.process_sync_call(&mut memory, raw_fn, args, cap_index, class_index).await {
                                    Some(point) => Ok(point),
                                    None => Ok(0),
                                }
                            })
                        },
                    ).expect("Failed to link sync function");
                }
                Function::Async(raw_fn) => {
                    linker.func_wrap_async(
                        &func.cap_name,
                        &func.func_name,
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
                                match cap.process_async_call(&mut memory, raw_fn, args, cap_index, class_index).await {
                                    Some(point) => Ok(point),
                                    None => Ok(0),
                                }
                            })
                        },
                    ).expect("Failed to link async function");
                }
            }
        }
        Ok(())
    }
}

/// Represents the state of a single class instance within a capability
pub struct ClassState {
    pub ptr: *mut c_void,
    pub reset_fn: ClassResetFn<'static>,
    pub destroy_fn: ClassDropFn,
}

impl ClassState {
    pub fn reset(&mut self, ident: &CapIdentity) -> CapabilityReset<'static> {
        match self.reset_fn {
            ClassResetFn::Sync(func) => {
                let res = unsafe { func(self.ptr) };
                CapabilityReset::SyncOrNull(ident.clone(), Some(unsafe {
                    ExecutionResultBridge::expected_null_from_ffi(
                        res,
                        &ident
                    )
                }))
            }
            ClassResetFn::Async(func) => {
                let fut = unsafe { func(self.ptr) };
                let future = AsyncExecFuture::new(fut, &ident);
                let future: AsyncExecFuture<'static> = unsafe { std::mem::transmute(future) };
                CapabilityReset::Async(future)
            }
            ClassResetFn::Null => CapabilityReset::SyncOrNull(ident.clone(), Some(Ok(()))),
        }
    }
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


#[pin_project(project = CapInit)]
pub enum CapabilityInit<'a> {
    Sync {
        reset_fn: ClassResetFn<'static>,
        state: Option<*mut c_void>,
        destroy_fn: ClassDropFn,
    },
    Async {
        reset_fn: ClassResetFn<'static>,
        config_bytes: Vec<u8>,
        #[pin]
        future: AsyncInitFuture<'a>,
        destroy_fn: ClassDropFn,
    },
    Null,
}

impl<'a> Future for CapabilityInit<'a> {
    type Output = PyroductResult<ClassState>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.project() {
            CapInit::Sync { reset_fn, state, destroy_fn } => match state.take() {
                Some(state) => std::task::Poll::Ready(Ok(ClassState {
                    reset_fn: reset_fn.clone(),
                    ptr: state,
                    destroy_fn: *destroy_fn,
                })),
                None => panic!("Double await!"),
            },
            CapInit::Async {
                reset_fn,
                config_bytes: _,
                future,
                destroy_fn,
            } => match future.poll(cx) {
                std::task::Poll::Ready(result) => match result {
                    Ok(pointer) => std::task::Poll::Ready(Ok(ClassState {
                        reset_fn: reset_fn.clone(),
                        ptr: pointer,
                        destroy_fn: *destroy_fn,
                    })),
                    Err(e) => std::task::Poll::Ready(Err(e)),
                },
                std::task::Poll::Pending => std::task::Poll::Pending,
            },
            CapInit::Null => std::task::Poll::Ready(Ok(ClassState {
                reset_fn: ClassResetFn::Null,
                ptr: std::ptr::null_mut(),
                destroy_fn: ClassDropFn::Null,
            })),
        }
    }
}

#[pin_project(project = CapReset)]
pub enum CapabilityReset<'a> {
    Async(#[pin] AsyncExecFuture<'a>),
    SyncOrNull(CapIdentity, Option<PyroductResult<()>>),
}

impl<'a> Future for CapabilityReset<'a> {
    type Output = PyroductResult<()>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.project() {
            CapReset::Async(this) => match this.poll(cx) {
                std::task::Poll::Ready(Ok(_)) => std::task::Poll::Ready(Ok(())),
                std::task::Poll::Ready(Err(e)) => std::task::Poll::Ready(Err(e)),
                std::task::Poll::Pending => std::task::Poll::Pending,
            },
            CapReset::SyncOrNull(ident, result) => {
                match result.take() {
                    Some(result) => std::task::Poll::Ready(result),
                    None => std::task::Poll::Ready(Err(FfiError::FuturePolledAfterCompletion
                        .to_capability_error(ident))),
                }
            }
        }
    }
}

pub enum AsyncInitState<'a> {
    Ffi(async_ffi::BorrowingFfiFuture<'a, FfiInitResult>),
    Ready(Option<Result<*mut c_void, PyroductError>>),
}

/// Wrapper for the async init future that handles both pending futures and early errors.
pub struct AsyncInitFuture<'a> {
    state: AsyncInitState<'a>,
    ident: CapIdentity,
}

impl<'a> AsyncInitFuture<'a> {
    pub fn new(
        res: FfiBorrowedFutureObjectResult<'a>,
        ident: &CapIdentity,
    ) -> Self {
        let state = match res {
            FfiBorrowedFutureObjectResult::Future(fut) => {
                trace!("AsyncInitFuture: created from Future variant");
                AsyncInitState::Ffi(fut)
            }
            FfiBorrowedFutureObjectResult::EarlyError(val) => {
                trace!("AsyncInitFuture: created from EarlyError variant");
                // Convert the early result immediately
                let result = unsafe {
                    InitResultBridge::from_ffi(
                        val,
                        ident,
                    )
                };
                AsyncInitState::Ready(Some(result))
            }
        };
        Self {
            state,
            ident: ident.clone(),
        }
    }
}

impl<'a> Future for AsyncInitFuture<'a> {
    type Output = Result<*mut c_void, PyroductError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // We need to move these out to avoid borrow checker issues when calling from_ffi
        let ident = self.ident.clone();

        match &mut self.state {
            AsyncInitState::Ffi(fut) => match Pin::new(fut).poll(cx) {
                Poll::Ready(res) => {
                    trace!("AsyncInitFuture: underlying future ready");
                    Poll::Ready(unsafe { InitResultBridge::from_ffi(res, &ident) })
                }
                Poll::Pending => Poll::Pending,
            },
            AsyncInitState::Ready(res) => {
                trace!("AsyncInitFuture: returning ready result");
                Poll::Ready(res.take().unwrap_or_else(|| {
                    error!("AsyncInitFuture: polled after completion");
                    Err(FfiError::FuturePolledAfterCompletion.to_capability_error(&ident))
                }))
            }
        }
    }
}
