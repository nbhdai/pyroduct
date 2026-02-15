use std::{
    ffi::c_void,
    pin::Pin,
    ptr,
    task::{Context, Poll},
};

use bridge_vec::{BridgeError, BridgeVec, CapturedError, captured::LibraryInfo};
use pin_project::pin_project;
use tracing::{error, info, trace};
use wasmtime::Linker;

use crate::{
    CapIdentity, PyroductResult,
    capability_host::ffi::{
        ClassDropFn, ClassExport, ClassInitFn, ClassResetFn, FfiBorrowedFutureObjectResult,
        FfiInitResult, Function, FunctionExport,
    },
    errors::PyroductError,
    host::{
        capability::WasmArgs,
        ffi_bridge::{AsyncExecFuture, InitResultBridge},
        wasm_bridge::{HarnessState, WasmMemory},
    },
};

/// Represents a loaded class from a dynamic library
#[derive(Clone)]
pub struct CapClass {
    pub ident: CapIdentity,
    pub imports: Vec<CapFunction>,
    pub init_fn: ClassInitFn<'static>,
    pub reset_fn: ClassResetFn<'static>,
    pub destroy_fn: ClassDropFn,
}

#[derive(Clone)]
pub struct CapFunction {
    pub cap_name: String,
    pub func_name: String,
    pub pointer: Function<'static>,
}

impl CapFunction {
    pub fn new(func: &FunctionExport<'_>, ident: &CapIdentity) -> PyroductResult<Self> {
        if func.capability.is_null() {
            return Err(PyroductError::from_loading(
                ident,
                "FunctionExport capability name pointer is null",
            ));
        }
        if func.name.is_null() {
            return Err(PyroductError::from_loading(
                ident,
                "FunctionExport function name pointer is null",
            ));
        }

        // Safety: We have verified pointers are not null above.
        let cap_name = std::str::from_utf8(unsafe {
            std::slice::from_raw_parts(func.capability, func.capability_len)
        })
        .unwrap_or("unknown_mod")
        .to_string();

        let func_name =
            std::str::from_utf8(unsafe { std::slice::from_raw_parts(func.name, func.name_len) })
                .unwrap_or("unknown_func")
                .to_string();

        let pointer = unsafe { std::mem::transmute::<Function<'_>, Function<'static>>(func.func) };
        let func = CapFunction {
            cap_name,
            func_name,
            pointer,
        };
        Ok(func)
    }
}

impl CapClass {
    pub fn new(ident: &LibraryInfo<'static>, class: &ClassExport<'_>) -> PyroductResult<Self> {
        if class.ptr.is_null() {
            return Err(PyroductError::from_loading(
                ident,
                "ClassExport methods pointer is null",
            ));
        }
        if class.len == 0 {
            return Err(PyroductError::from_loading(
                ident,
                "ClassExport has no methods",
            ));
        }
        let exports: &[FunctionExport<'_>] =
            unsafe { std::slice::from_raw_parts(class.ptr, class.len) };

        let mut imports = Vec::new();

        for export in exports {
            let func = CapFunction::new(export, ident)?;
            imports.push(func);
        }
        let init_fn =
            unsafe { std::mem::transmute::<ClassInitFn<'_>, ClassInitFn<'static>>(class.init) };
        let reset_fn =
            unsafe { std::mem::transmute::<ClassResetFn<'_>, ClassResetFn<'static>>(class.reset) };

        Ok(Self {
            ident: ident.clone(),
            imports,
            init_fn,
            reset_fn,
            destroy_fn: class.drop,
        })
    }

    pub fn init(
        &self,
        config: Option<&serde_json::Value>,
    ) -> PyroductResult<CapabilityInit<'static>> {
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
                let state = unsafe { InitResultBridge::from_ffi(res)? };
                CapabilityInit::Sync {
                    ident: self.ident.clone(),
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
                    ident: self.ident.clone(),
                    reset_fn: self.reset_fn.clone(),
                    config_bytes,
                    future,
                    destroy_fn: self.destroy_fn,
                }
            }
            ClassInitFn::Null => CapabilityInit::Null(self.ident.clone()),
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
    ) -> Option<i32> {
        let (client_ptr, client_len, input_ptr, input_len) = args;
        let host_state_ptr = caller.class_state(cap_index);
        let input = caller.get_slice(input_ptr, input_len)?;
        let client = caller.get_slice(client_ptr, client_len)?;

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

        let output_vec = match unsafe { ExecutionResultBridge::from_ffi(result, &self.ident) } {
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
    ) -> Option<i32> {
        let (client_ptr, client_len, input_ptr, input_len) = args;
        let host_state_ptr = caller.class_state(cap_index);
        let input = caller.get_slice(input_ptr, input_len)?;
        let client = caller.get_slice(client_ptr, client_len)?;

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

    pub fn link(
        &self,
        linker: &mut Linker<HarnessState>,
        _span_index: usize,
        cap_index: usize,
    ) -> PyroductResult<()> {
        for func in self.imports.iter() {
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
                                match cap.process_sync_call(&mut memory, raw_fn, args, cap_index).await {
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
                                match cap.process_async_call(&mut memory, raw_fn, args, cap_index).await {
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
    pub ident: CapIdentity,
    pub ptr: *mut c_void,
    pub reset_fn: ClassResetFn<'static>,
    pub destroy_fn: ClassDropFn,
}

impl ClassState {
    pub fn reset(&mut self) -> CapabilityReset<'static> {
        match self.reset_fn {
            ClassResetFn::Sync(func) => {
                let res = unsafe { func(self.ptr) };
                CapabilityReset::SyncOrNull(
                    self.ident.clone(),
                    Some(unsafe {
                        ExecutionResultBridge::expected_null_from_ffi(res, &self.ident)
                    }),
                )
            }
            ClassResetFn::Async(func) => {
                let fut = unsafe { func(self.ptr) };
                let future = AsyncExecFuture::new(fut, &self.ident);
                let future: AsyncExecFuture<'static> = unsafe { std::mem::transmute(future) };
                CapabilityReset::Async(future)
            }
            ClassResetFn::Null => CapabilityReset::SyncOrNull(self.ident.clone(), Some(Ok(()))),
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
        ident: CapIdentity,
        reset_fn: ClassResetFn<'static>,
        state: Option<*mut c_void>,
        destroy_fn: ClassDropFn,
    },
    Async {
        ident: CapIdentity,
        reset_fn: ClassResetFn<'static>,
        config_bytes: Vec<u8>,
        #[pin]
        future: AsyncInitFuture<'a>,
        destroy_fn: ClassDropFn,
    },
    Null(CapIdentity),
}

impl<'a> Future for CapabilityInit<'a> {
    type Output = Result<ClassState, BridgeError>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.project() {
            CapInit::Sync {
                ident,
                reset_fn,
                state,
                destroy_fn,
            } => match state.take() {
                Some(state) => std::task::Poll::Ready(Ok(ClassState {
                    ident: ident.clone(),
                    reset_fn: reset_fn.clone(),
                    ptr: state,
                    destroy_fn: *destroy_fn,
                })),
                None => panic!("Double await!"),
            },
            CapInit::Async {
                ident,
                reset_fn,
                config_bytes: _,
                future,
                destroy_fn,
            } => match future.poll(cx) {
                std::task::Poll::Ready(result) => match result {
                    Ok(pointer) => std::task::Poll::Ready(Ok(ClassState {
                        ident: ident.clone(),
                        reset_fn: reset_fn.clone(),
                        ptr: pointer,
                        destroy_fn: *destroy_fn,
                    })),
                    Err(e) => std::task::Poll::Ready(Err(e)),
                },
                std::task::Poll::Pending => std::task::Poll::Pending,
            },
            CapInit::Null(ident) => std::task::Poll::Ready(Ok(ClassState {
                ident: ident.clone(),
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
    SyncOrNull(CapIdentity, Option<Result<(), BridgeError>>),
}

impl<'a> Future for CapabilityReset<'a> {
    type Output = Result<(), BridgeError>;

    #[track_caller]
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
            CapReset::SyncOrNull(ident, result) => match result.take() {
                Some(result) => std::task::Poll::Ready(result),
                None => std::task::Poll::Ready(Err(
                    BridgeError::CodePanic(CapturedError::new("AsyncInitFuture: polled after completion").with_location(std::panic::Location::caller()).into())
                )),
            },
        }
    }
}

pub enum AsyncInitState<'a> {
    Ffi(async_ffi::BorrowingFfiFuture<'a, FfiInitResult>),
    Ready(Option<Result<*mut c_void, BridgeError>>),
}

/// Wrapper for the async init future that handles both pending futures and early errors.
pub struct AsyncInitFuture<'a> {
    state: AsyncInitState<'a>,
    ident: CapIdentity,
}

impl<'a> AsyncInitFuture<'a> {
    pub fn new(res: FfiBorrowedFutureObjectResult<'a>, ident: &CapIdentity) -> Self {
        let state = match res {
            FfiBorrowedFutureObjectResult::Future(fut) => {
                trace!("AsyncInitFuture: created from Future variant");
                AsyncInitState::Ffi(fut)
            }
            FfiBorrowedFutureObjectResult::EarlyError(val) => {
                trace!("AsyncInitFuture: created from EarlyError variant");
                match unsafe { BridgeVec::from_raw(val.error) } {
                    Ok(err_vec) => AsyncInitState::Ready(Some(Err(err_vec.parse_as_error()))),
                    Err(err) => AsyncInitState::Ready(Some(Err(err))),
                }
            }
        };
        Self {
            state,
            ident: ident.clone(),
        }
    }
}

impl<'a> Future for AsyncInitFuture<'a> {
    type Output = Result<*mut c_void, BridgeError>;

    #[track_caller]
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
                    Err(BridgeError::CodePanic(CapturedError::new("AsyncInitFuture: polled after completion").with_location(std::panic::Location::caller()).into()))
                }))
            }
        }
    }
}
