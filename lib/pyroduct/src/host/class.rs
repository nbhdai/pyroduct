use std::{
    ffi::c_void,
    pin::Pin,
    ptr,
    task::{Context, Poll},
};

use pin_project::pin_project;
use tracing::{error, info, trace};
use wasmtime::Linker;

use crate::{
    CapIdentity, PyroductResult,
    capability_host::ffi::{
        ClassDropFn, ClassExport, ClassInitFn, ClassResetFn, FfiBorrowedFutureObjectResult,
        FfiInitResult, Function, FunctionExport,
    },
    errors::{FfiError, PyroductError},
    host::{
        capability::WasmArgs,
        ffi_bridge::{AsyncExecFuture, ExecutionResultBridge, InitResultBridge},
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
            return Err(PyroductError::from_capability_loading(
                ident,
                "FunctionExport capability name pointer is null",
            ));
        }
        if func.name.is_null() {
            return Err(PyroductError::from_capability_loading(
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
    pub fn new(ident: &CapIdentity, class: &ClassExport<'_>) -> PyroductResult<Self> {
        if class.ptr.is_null() {
            return Err(PyroductError::from_capability_loading(
                ident,
                "ClassExport methods pointer is null",
            ));
        }
        if class.len == 0 {
            return Err(PyroductError::from_capability_loading(
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
                let state = unsafe { InitResultBridge::from_ffi(res, &self.ident)? };
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

    pub fn link(&self, linker: &mut Linker<HarnessState>, _span_index: usize, cap_index: usize) -> PyroductResult<()> {
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
    type Output = PyroductResult<ClassState>;

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
            CapReset::SyncOrNull(ident, result) => match result.take() {
                Some(result) => std::task::Poll::Ready(result),
                None => std::task::Poll::Ready(Err(
                    FfiError::FuturePolledAfterCompletion.to_capability_error(ident)
                )),
            },
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
    pub fn new(res: FfiBorrowedFutureObjectResult<'a>, ident: &CapIdentity) -> Self {
        let state = match res {
            FfiBorrowedFutureObjectResult::Future(fut) => {
                trace!("AsyncInitFuture: created from Future variant");
                AsyncInitState::Ffi(fut)
            }
            FfiBorrowedFutureObjectResult::EarlyError(val) => {
                trace!("AsyncInitFuture: created from EarlyError variant");
                // Convert the early result immediately
                let result = unsafe { InitResultBridge::from_ffi(val, ident) };
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability_host::ffi::*;
    use std::cell::RefCell;
    use std::ffi::c_void;
    use std::ptr;
    use crate::errors::FfiError;

    // ============================================================================
    // Mock FFI Functions & Thread-Local State
    // ============================================================================

    // We use thread_local variables instead of global Atomics to ensure that
    // tests running in parallel do not interfere with each other's state.
    #[derive(Default, Clone, Copy, Debug)]
    struct MockCounters {
        init: u32,
        drop: u32,
        reset: u32,
    }

    thread_local! {
        static COUNTERS: RefCell<MockCounters> = RefCell::new(MockCounters::default());
    }

    fn reset_mock_counters() {
        COUNTERS.with(|c| *c.borrow_mut() = MockCounters::default());
    }

    fn get_mock_counters() -> MockCounters {
        COUNTERS.with(|c| *c.borrow())
    }

    // Mock state struct
    #[repr(C)]
    struct MockState {
        value: u32,
    }

    // Sync init that returns a valid state pointer
    unsafe extern "C" fn mock_sync_init(_config_ptr: *const u8, _config_len: usize) -> FfiInitResult {
        COUNTERS.with(|c| c.borrow_mut().init += 1);
        let state = Box::new(MockState { value: 42 });
        FfiInitResult::ok(Box::into_raw(state) as *mut c_void)
    }

    // Sync init that returns an error
    unsafe extern "C" fn mock_sync_init_error(_config_ptr: *const u8, _config_len: usize) -> FfiInitResult {
        let ffi_error = FfiError::DeserializationFailed("Init failed (Mock)".to_string(), crate::errors::Phase::Init);
        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&ffi_error).unwrap().into_vec();
        
        let mut bytes = bytes; 
        let (ptr, len, cap) = (bytes.as_mut_ptr(), bytes.len(), bytes.capacity());
        std::mem::forget(bytes); 
        FfiInitResult::err(COutput { ptr, len, cap })
    }

    // Sync drop
    unsafe extern "C" fn mock_sync_drop(state: *mut c_void) {
        COUNTERS.with(|c| c.borrow_mut().drop += 1);
        if !state.is_null() {
            let _ = unsafe { Box::from_raw(state as *mut MockState) };
        }
    }

    // Sync reset
    unsafe extern "C" fn mock_sync_reset(state: *mut c_void) -> FfiResult {
        COUNTERS.with(|c| c.borrow_mut().reset += 1);
        if state.is_null() {
            return FfiResult::full_err(COutput {
                ptr: ptr::null(),
                len: 0,
                cap: 0,
            });
        }
        let state = unsafe { &mut *(state as *mut MockState) };
        state.value = 0;
        FfiResult::ok_null()
    }

    // Sync reset that returns error
    unsafe extern "C" fn mock_sync_reset_error(_state: *mut c_void) -> FfiResult {
        let ffi_error = FfiError::DeserializationFailed("Reset failed (Mock)".to_string(), crate::errors::Phase::Reset);
        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&ffi_error).unwrap().into_vec();

        let mut bytes = bytes;
        let (ptr, len, cap) = (bytes.as_mut_ptr(), bytes.len(), bytes.capacity());
        std::mem::forget(bytes);
        FfiResult::full_err(COutput { ptr, len, cap })
    }

    // Mock capability functions
    unsafe extern "C" fn mock_sync_function(
        _client_ptr: *const u8,
        _client_len: usize,
        _input_ptr: *const u8,
        _input_len: usize,
        state_ptr: *mut c_void,
    ) -> FfiResult {
        if state_ptr.is_null() {
            return FfiResult::full_err(COutput {
                ptr: ptr::null(),
                len: 0,
                cap: 0,
            });
        }
        let state = unsafe { &*(state_ptr as *const MockState) };
        let result = state.value.to_string().into_bytes();
        let (ptr, len, cap) = (result.as_ptr(), result.len(), result.capacity());
        std::mem::forget(result);
        FfiResult::ok(COutput { ptr, len, cap })
    }

    // ============================================================================
    // Test Helpers
    // ============================================================================

    fn create_mock_function_export(cap_name: &str, func_name: &str) -> FunctionExport<'static> {
        FunctionExport {
            capability: cap_name.as_ptr(),
            capability_len: cap_name.len(),
            name: func_name.as_ptr(),
            name_len: func_name.len(),
            func: Function::Sync(mock_sync_function),
        }
    }

    fn create_mock_class_export(
        exports: &[FunctionExport<'static>],
        init_fn: ClassInitFn<'static>,
        drop_fn: ClassDropFn,
        reset_fn: ClassResetFn<'static>,
    ) -> ClassExport<'static> {
        ClassExport {
            ptr: exports.as_ptr(),
            init: init_fn,
            drop: drop_fn,
            reset: reset_fn,
            len: exports.len(),
        }
    }

    fn create_test_identity() -> CapIdentity {
        CapIdentity::from(std::path::Path::new("/test/capability.so"))
    }

    // ============================================================================
    // CapFunction Tests
    // ============================================================================

    #[test]
    fn test_cap_function_creation() {
        reset_mock_counters();
        let cap_name = "test_cap";
        let func_name = "test_func";
        let export = create_mock_function_export(cap_name, func_name);
        let ident = create_test_identity();

        let func = CapFunction::new(&export, &ident).expect("Should succeed");

        assert_eq!(func.cap_name, cap_name);
        assert_eq!(func.func_name, func_name);
    }

    #[test]
    fn test_cap_function_with_null_pointers_returns_error() {
        reset_mock_counters();
        let export = FunctionExport {
            capability: ptr::null(),
            capability_len: 0,
            name: ptr::null(),
            name_len: 0,
            func: Function::Sync(mock_sync_function),
        };
        let ident = create_test_identity();

        let result = CapFunction::new(&export, &ident);
        assert!(result.is_err());
    }

    // ============================================================================
    // CapClass Tests
    // ============================================================================

    #[test]
    fn test_cap_class_creation() {
        reset_mock_counters();
        let cap_name = "test_cap";
        let func_name = "test_func";
        let export = create_mock_function_export(cap_name, func_name);
        let exports = vec![export];

        let class_export = create_mock_class_export(
            &exports,
            ClassInitFn::Sync(mock_sync_init),
            ClassDropFn::Sync(mock_sync_drop),
            ClassResetFn::Sync(mock_sync_reset),
        );

        let ident = create_test_identity();
        let class = CapClass::new(&ident, &class_export).expect("Should create class");

        assert_eq!(class.ident, ident);
        assert_eq!(class.imports.len(), 1);
        assert_eq!(class.imports[0].cap_name, cap_name);
        assert_eq!(class.imports[0].func_name, func_name);
    }

    #[test]
    fn test_cap_class_with_multiple_functions() {
        reset_mock_counters();
        let exports = vec![
            create_mock_function_export("cap1", "func1"),
            create_mock_function_export("cap2", "func2"),
            create_mock_function_export("cap3", "func3"),
        ];

        let class_export = create_mock_class_export(
            &exports,
            ClassInitFn::Sync(mock_sync_init),
            ClassDropFn::Sync(mock_sync_drop),
            ClassResetFn::Sync(mock_sync_reset),
        );

        let ident = create_test_identity();
        let class = CapClass::new(&ident, &class_export).expect("Should create class");

        assert_eq!(class.imports.len(), 3);
        assert_eq!(class.imports[0].func_name, "func1");
        assert_eq!(class.imports[1].func_name, "func2");
        assert_eq!(class.imports[2].func_name, "func3");
    }

    // ============================================================================
    // CapClass::init Tests
    // ============================================================================

    #[tokio::test]
    async fn test_cap_class_sync_init_success() {
        reset_mock_counters();
        let exports = vec![create_mock_function_export("test", "func")];
        let class_export = create_mock_class_export(
            &exports,
            ClassInitFn::Sync(mock_sync_init),
            ClassDropFn::Sync(mock_sync_drop),
            ClassResetFn::Sync(mock_sync_reset),
        );

        let ident = create_test_identity();
        let class = CapClass::new(&ident, &class_export).unwrap();

        let init_result = class.init(None).unwrap();
        let state = init_result.await.unwrap();

        assert_eq!(get_mock_counters().init, 1);
        assert!(!state.ptr.is_null());
        assert_eq!(unsafe { (*(state.ptr as *const MockState)).value }, 42);
    }

    #[tokio::test]
    async fn test_cap_class_sync_init_with_config() {
        reset_mock_counters();
        let exports = vec![create_mock_function_export("test", "func")];
        let class_export = create_mock_class_export(
            &exports,
            ClassInitFn::Sync(mock_sync_init),
            ClassDropFn::Sync(mock_sync_drop),
            ClassResetFn::Sync(mock_sync_reset),
        );

        let ident = create_test_identity();
        let class = CapClass::new(&ident, &class_export).unwrap();

        let config = serde_json::json!({"key": "value"});
        let init_result = class.init(Some(&config)).unwrap();
        let state = init_result.await.unwrap();

        assert_eq!(get_mock_counters().init, 1);
        assert!(!state.ptr.is_null());
    }

    #[tokio::test]
    async fn test_cap_class_sync_init_error() {
        reset_mock_counters();
        let exports = vec![create_mock_function_export("test", "func")];
        let class_export = create_mock_class_export(
            &exports,
            ClassInitFn::Sync(mock_sync_init_error),
            ClassDropFn::Sync(mock_sync_drop),
            ClassResetFn::Sync(mock_sync_reset),
        );

        let ident = create_test_identity();
        let class = CapClass::new(&ident, &class_export).unwrap();

        let init_result = class.init(None);

        assert!(init_result.is_err());
    }

    #[tokio::test]
    async fn test_cap_class_null_init() {
        reset_mock_counters();
        let exports = vec![create_mock_function_export("test", "func")];
        let class_export = create_mock_class_export(
            &exports,
            ClassInitFn::Null,
            ClassDropFn::Null,
            ClassResetFn::Null,
        );

        let ident = create_test_identity();
        let class = CapClass::new(&ident, &class_export).unwrap();

        let init_result = class.init(None).unwrap();
        let state = init_result.await.unwrap();

        assert_eq!(get_mock_counters().init, 0);
        assert!(state.ptr.is_null());
    }

    // ============================================================================
    // ClassState Tests
    // ============================================================================

    #[tokio::test]
    async fn test_class_state_reset_sync_success() {
        reset_mock_counters();
        let state_ptr = Box::into_raw(Box::new(MockState { value: 42 })) as *mut c_void;
        let mut state = ClassState {
            ident: create_test_identity(),
            ptr: state_ptr,
            reset_fn: ClassResetFn::Sync(mock_sync_reset),
            destroy_fn: ClassDropFn::Sync(mock_sync_drop),
        };

        let reset_result = state.reset();
        let result = reset_result.await;

        assert!(result.is_ok());
        assert_eq!(get_mock_counters().reset, 1);
        assert_eq!(unsafe { (*(state.ptr as *const MockState)).value }, 0);
    }

    #[tokio::test]
    async fn test_class_state_reset_sync_error() {
        reset_mock_counters();
        let state_ptr = Box::into_raw(Box::new(MockState { value: 42 })) as *mut c_void;
        let mut state = ClassState {
            ident: create_test_identity(),
            ptr: state_ptr,
            reset_fn: ClassResetFn::Sync(mock_sync_reset_error),
            destroy_fn: ClassDropFn::Sync(mock_sync_drop),
        };

        let reset_result = state.reset();
        let result = reset_result.await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_class_state_reset_null() {
        reset_mock_counters();
        let state_ptr = Box::into_raw(Box::new(MockState { value: 42 })) as *mut c_void;
        let mut state = ClassState {
            ident: create_test_identity(),
            ptr: state_ptr,
            reset_fn: ClassResetFn::Null,
            destroy_fn: ClassDropFn::Sync(mock_sync_drop),
        };

        let reset_result = state.reset();
        let result = reset_result.await;

        assert!(result.is_ok());
        assert_eq!(get_mock_counters().reset, 0);
    }

    #[test]
    fn test_class_state_drop() {
        reset_mock_counters();
        let state_ptr = Box::into_raw(Box::new(MockState { value: 42 })) as *mut c_void;
        {
            let _state = ClassState {
                ident: create_test_identity(),
                ptr: state_ptr,
                reset_fn: ClassResetFn::Sync(mock_sync_reset),
                destroy_fn: ClassDropFn::Sync(mock_sync_drop),
            };
            // State dropped here
        }

        assert_eq!(get_mock_counters().drop, 1);
    }

    #[test]
    fn test_class_state_drop_with_null_ptr() {
        reset_mock_counters();
        {
            let _state = ClassState {
                ident: create_test_identity(),
                ptr: ptr::null_mut(),
                reset_fn: ClassResetFn::Null,
                destroy_fn: ClassDropFn::Sync(mock_sync_drop),
            };
            // State dropped here
        }

        assert_eq!(get_mock_counters().drop, 0);
    }

    #[test]
    fn test_class_state_drop_with_null_fn() {
        reset_mock_counters();
        let state_ptr = Box::into_raw(Box::new(MockState { value: 42 })) as *mut c_void;
        {
            let _state = ClassState {
                ident: create_test_identity(),
                ptr: state_ptr,
                reset_fn: ClassResetFn::Null,
                destroy_fn: ClassDropFn::Null,
            };
            // State dropped here
        }

        assert_eq!(get_mock_counters().drop, 0);
        // Manual cleanup to prevent leak in test
        unsafe { drop(Box::from_raw(state_ptr as *mut MockState)) };
    }

    // ============================================================================
    // CapabilityInit Future Tests
    // ============================================================================

    #[tokio::test]
    async fn test_capability_init_sync_polling() {
        reset_mock_counters();
        let state_ptr = Box::into_raw(Box::new(MockState { value: 99 })) as *mut c_void;
        let init = CapabilityInit::Sync {
            ident: create_test_identity(),
            reset_fn: ClassResetFn::Sync(mock_sync_reset),
            state: Some(state_ptr),
            destroy_fn: ClassDropFn::Sync(mock_sync_drop),
        };

        let state = init.await.unwrap();

        assert_eq!(state.ptr, state_ptr);
        assert_eq!(unsafe { (*(state.ptr as *const MockState)).value }, 99);
    }

    #[tokio::test]
    #[should_panic(expected = "Double await!")]
    async fn test_capability_init_sync_double_poll_panics() {
        reset_mock_counters();
        let state_ptr = Box::into_raw(Box::new(MockState { value: 99 })) as *mut c_void;
        let mut init = CapabilityInit::Sync {
            ident: create_test_identity(),
            reset_fn: ClassResetFn::Sync(mock_sync_reset),
            state: Some(state_ptr),
            destroy_fn: ClassDropFn::Sync(mock_sync_drop),
        };

        // First poll should succeed
        use std::future::Future;
        use std::pin::Pin;
        use std::task::{Context, Poll, Waker};
        
        let waker: Waker = futures::task::noop_waker();
        let mut cx = Context::from_waker(&waker);

        let mut pinned = Pin::new(&mut init);
        let poll1 = pinned.as_mut().poll(&mut cx);
        assert!(matches!(poll1, Poll::Ready(Ok(_))));

        // Second poll should panic
        let _poll2 = pinned.poll(&mut cx);
    }

    #[tokio::test]
    async fn test_capability_init_null() {
        reset_mock_counters();
        let init = CapabilityInit::Null(create_test_identity());
        let state = init.await.unwrap();

        assert!(state.ptr.is_null());
    }

    // ============================================================================
    // CapabilityReset Future Tests
    // ============================================================================

    #[tokio::test]
    async fn test_capability_reset_sync_or_null_success() {
        reset_mock_counters();
        let reset = CapabilityReset::SyncOrNull(create_test_identity(), Some(Ok(())));
        let result = reset.await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_capability_reset_sync_or_null_error() {
        reset_mock_counters();
        let error = PyroductError::from_infrastructure("Test error");
        let reset = CapabilityReset::SyncOrNull(create_test_identity(), Some(Err(error)));
        let result = reset.await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_capability_reset_sync_or_null_double_poll() {
        reset_mock_counters();
        let mut reset = CapabilityReset::SyncOrNull(create_test_identity(), Some(Ok(())));

        use std::future::Future;
        use std::pin::Pin;
        use std::task::{Context, Poll};
        
        let waker = futures::task::noop_waker();
        let mut cx = Context::from_waker(&waker);

        // First poll
        let mut pinned = Pin::new(&mut reset);
        let poll1 = pinned.as_mut().poll(&mut cx);
        assert!(matches!(poll1, Poll::Ready(Ok(_))));

        // Second poll should return error
        let poll2 = pinned.poll(&mut cx);
        assert!(matches!(poll2, Poll::Ready(Err(_))));
    }

    // ============================================================================
    // Edge Cases and Safety Tests
    // ============================================================================

    #[test]
    fn test_class_state_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<ClassState>();
    }

    #[tokio::test]
    async fn test_multiple_states_independent_lifecycle() {
        reset_mock_counters();
        
        let state1_ptr = Box::into_raw(Box::new(MockState { value: 1 })) as *mut c_void;
        let state2_ptr = Box::into_raw(Box::new(MockState { value: 2 })) as *mut c_void;

        let state1 = ClassState {
            ident: create_test_identity(),
            ptr: state1_ptr,
            reset_fn: ClassResetFn::Sync(mock_sync_reset),
            destroy_fn: ClassDropFn::Sync(mock_sync_drop),
        };

        let mut state2 = ClassState {
            ident: create_test_identity(),
            ptr: state2_ptr,
            reset_fn: ClassResetFn::Sync(mock_sync_reset),
            destroy_fn: ClassDropFn::Sync(mock_sync_drop),
        };

        // Reset state2
        let reset_result = state2.reset();
        reset_result.await.unwrap();

        // State1 should be unaffected
        assert_eq!(unsafe { (*(state1.ptr as *const MockState)).value }, 1);
        // State2 should be reset
        assert_eq!(unsafe { (*(state2.ptr as *const MockState)).value }, 0);
        assert_eq!(get_mock_counters().reset, 1);

        drop(state1);
        assert_eq!(get_mock_counters().drop, 1);

        drop(state2);
        assert_eq!(get_mock_counters().drop, 2);
    }

    #[tokio::test]
    async fn test_cap_class_init_called_multiple_times() {
        reset_mock_counters();
        
        let exports = vec![create_mock_function_export("test", "func")];
        let class_export = create_mock_class_export(
            &exports,
            ClassInitFn::Sync(mock_sync_init),
            ClassDropFn::Sync(mock_sync_drop),
            ClassResetFn::Sync(mock_sync_reset),
        );

        let ident = create_test_identity();
        let class = CapClass::new(&ident, &class_export).unwrap();

        // Call init multiple times
        let init1 = class.init(None).unwrap();
        let state1 = init1.await.unwrap();

        let init2 = class.init(None).unwrap();
        let state2 = init2.await.unwrap();

        assert_eq!(get_mock_counters().init, 2);
        assert_ne!(state1.ptr, state2.ptr); // Different instances
    }
}