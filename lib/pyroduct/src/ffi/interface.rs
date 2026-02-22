use libloading::Library;
use std::ffi::c_void;
use std::future::Future;
use std::pin::Pin;
use std::ptr::NonNull;
use std::slice;
use std::sync::Arc;
use std::task::{Context, Poll};

use crate::header::PyroData;
use crate::view::{PyroView, PyroViewPtr};
use crate::{CapturedError, PyroError, PyroVec, PyroVecPtr};

pub type LogCallback = unsafe extern "C" fn(i64, i64, *const u8, usize);

// ============================================================================
// Function pointer types
// ============================================================================

#[repr(C)]
pub struct MethodExport {
    pub func: Function,
    pub name: *const u8,
    pub name_len: usize,
}

/// We expect the return to be a future that resolves into a bridge vec.
pub type AsyncFn = unsafe extern "C" fn(
    // itself
    PyroRefObjectPtr,
    // Client (wasm side class state)
    PyroViewPtr,
    // Input
    PyroViewPtr,
) -> FuturePyroVec;

/// We expect the return to be a bridge vec.
pub type SyncFn = unsafe extern "C" fn(PyroRefObjectPtr, PyroViewPtr, PyroViewPtr) -> PyroVecPtr;

#[repr(C, u8)]
#[derive(Clone, Copy)]
pub enum Function {
    Sync(SyncFn),
    Async(AsyncFn),
}

#[repr(C)]
pub enum FuturePyroVec {
    /// The operation succeeded or failed immediately.
    Early(PyroVecPtr),
    /// The operation started successfully, and we have to await the result.
    Future(::async_ffi::BorrowingFfiFuture<'static, PyroVecPtr>),
}

impl From<PyroVecPtr> for FuturePyroVec {
    fn from(value: PyroVecPtr) -> Self {
        FuturePyroVec::Early(value)
    }
}

#[pin_project::pin_project(project = MethodProj)]
pub enum MethodCallFuture {
    /// Wrapping an active async FFI call.
    Async(#[pin] ::async_ffi::BorrowingFfiFuture<'static, PyroVecPtr>),
    /// A synchronous result or an early error.
    Ready(Option<Result<PyroVec, PyroError>>),
}

impl MethodCallFuture {
    pub fn from_async(res: FuturePyroVec) -> Self {
        match res {
            FuturePyroVec::Future(fut) => Self::Async(fut),
            FuturePyroVec::Early(ptr) => {
                let res = unsafe { PyroVec::from_raw(ptr) };
                Self::Ready(Some(res))
            }
        }
    }
}

impl Future for MethodCallFuture {
    type Output = Result<PyroVec, PyroError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            MethodProj::Async(fut) => match fut.poll(cx) {
                Poll::Ready(vec_ptr) => Poll::Ready(unsafe { PyroVec::from_raw(vec_ptr) }),
                Poll::Pending => Poll::Pending,
            },
            MethodProj::Ready(res) => Poll::Ready(
                res.take()
                    .expect("MethodCallFuture polled after completion"),
            ),
        }
    }
}

// ============================================================================
// Object pointer types
// ============================================================================

/// The FFI-safe representation of an OWNED instance.
#[repr(C)]
#[derive(Debug)]
pub struct PyroObjectPtr {
    pub state: *mut c_void,
    pub dropper: ClassDropper,
    span_id: i64,
}

unsafe impl Send for PyroObjectPtr {}

/// The FFI-safe representation of a BORROWED instance.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PyroRefObjectPtr {
    pub state: *mut c_void,
    span_id: i64,
}

unsafe impl Send for PyroRefObjectPtr {}
unsafe impl Sync for PyroRefObjectPtr {}

/// Function pointer to drop the opaque object.
pub type ClassDropper = unsafe extern "C" fn(ptr: *mut c_void);

/// An owned, RAII wrapper around an opaque state pointer.
#[derive(Debug)]
pub struct PyroObject {
    state: NonNull<c_void>,
    dropper: ClassDropper,
    span_id: i64,
}

unsafe impl Send for PyroObject {}
unsafe impl Sync for PyroObject {}



impl PyroObject {
    /// Creates a new PyroObject from raw components.
    pub unsafe fn new(state: *mut c_void, dropper: ClassDropper, span_id: i64) -> Result<Self, CapturedError> {
        let state = NonNull::new(state)
            .ok_or_else(|| CapturedError::new("Cannot construct PyroObject from null pointer"))?;

        Ok(Self { 
            state,
            dropper,
            span_id,
        })
    }

    /// Consumes the wrapper and returns the FFI-safe pointer struct.
    pub fn into_raw(self) -> PyroObjectPtr {
        let ptr = PyroObjectPtr {
            state: self.state.as_ptr(),
            dropper: self.dropper,
            span_id: self.span_id,
        };
        std::mem::forget(self);
        ptr
    }

    /// Reconstructs the wrapper from an FFI-safe pointer struct.
    pub unsafe fn from_raw(raw: PyroObjectPtr) -> Result<Self, CapturedError> {
        let state = NonNull::new(raw.state).ok_or_else(|| {
            CapturedError::new("Cannot construct PyroObject from null PyroObjectPtr")
        })?;

        Ok(Self {
            state,
            dropper: raw.dropper,
            span_id: raw.span_id,
        })
    }

    /// Creates an FFI-safe reference pointer.
    pub fn ref_ptr(&self) -> PyroRefObjectPtr {
        PyroRefObjectPtr {
            state: self.state.as_ptr(),
            span_id: self.span_id,
        }
    }

    /// Creates a lifetime-bound Rust reference wrapper.
    pub fn as_borrowed(&self) -> PyroObjectRef {
        PyroObjectRef {
            state: self.state,
            span_id: self.span_id,
        }
    }

    pub fn as_ptr(&self) -> *mut c_void {
        self.state.as_ptr()
    }

    // Need a guard system to enable mutability
    // /// Safety: Needs to be a pointer to a O within the same binary.
    // pub fn as_mut<O: 'static + Send + Sync>(&self) -> &mut O {
    //     unsafe { &mut *(self.state.as_ptr() as *mut O) }
    // }

    /// Safety: Needs to be a pointer to a O within the same binary.
    pub fn as_ref<O: 'static + Send + Sync>(&self) -> &mut O {
        unsafe { &mut *(self.state.as_ptr() as *mut O) }
    }
}

impl Drop for PyroObject {
    fn drop(&mut self) {
        unsafe {
            (self.dropper)(self.state.as_ptr());
        }
    }
}

/// A borrowed wrapper around an opaque state pointer.
#[derive(Clone, Copy, Debug)]
pub struct PyroObjectRef {
    state: NonNull<c_void>,
    span_id: i64,
}

unsafe impl Send for PyroObjectRef {}
unsafe impl Sync for PyroObjectRef {}

impl PyroObjectRef {
    /// Safety: Needs to be a pointer to a O within the same binary.
    pub unsafe fn from_raw(raw: PyroRefObjectPtr) -> Result<Self, CapturedError> {
        let state = NonNull::new(raw.state).ok_or_else(|| {
            CapturedError::new("Cannot construct PyroObjectRef from null PyroRefObjectPtr")
        })?;

        Ok(Self {
            state,
            span_id: raw.span_id,
        })
    }

    pub fn as_raw(&self) -> PyroRefObjectPtr {
        PyroRefObjectPtr {
            state: self.state.as_ptr(),
            span_id: self.span_id,
        }
    }

    pub fn as_ptr(&self) -> *mut c_void {
        self.state.as_ptr()
    }

    // Need a guard system to enable mutability
    // /// Safety: Needs to be a pointer to a O within the same binary.
    // pub fn as_mut<O: 'static + Send + Sync>(&self) -> &mut O {
    //     unsafe { &mut *(self.state.as_ptr() as *mut O) }
    // }

    /// Safety: Needs to be a pointer to a O within the same binary.
    pub fn as_ref<O: 'static + Send + Sync>(&self) -> &'_ mut O {
        unsafe { &mut *(self.state.as_ptr() as *mut O) }
    }
}

// ============================================================================
// Object management types
// ============================================================================

#[repr(C)]
pub struct InitResult {
    state: PyroObjectPtr,
    error: PyroVecPtr,
}

/// Generate a typed dropper for a given state type `S`.
///
/// # Safety
/// The pointer must have been created by `Box::into_raw(Box::new(s))` where `s: S`.
unsafe extern "C" fn typed_dropper<S>(ptr: *mut std::ffi::c_void) {
    if !ptr.is_null() {
        drop(unsafe { Box::from_raw(ptr as *mut S) });
    }
}

impl InitResult {
    /// Construct a successful `InitResult` from a state value.
    pub fn init_ok<S: 'static>(state: S, span_id: i64) -> InitResult {
        let state_ptr = Box::into_raw(Box::new(state)) as *mut std::ffi::c_void;
        tracing::debug!(?state_ptr, "Object allocated and forgotten");
        InitResult {
            state: PyroObjectPtr {
                state: state_ptr,
                dropper: typed_dropper::<S>,
                span_id,
            },
            error: PyroVec::ok().into_raw(),
        }
    }

    /// Construct an error `InitResult` from a `PyroError`.
    pub fn init_err(err: PyroError, span_id: i64) -> InitResult {
        InitResult {
            state: PyroObjectPtr {
                state: std::ptr::null_mut(),
                dropper: typed_dropper::<()>,
                span_id,
            },
            error: err.encode().into_raw(),
        }
    }
}

#[repr(C)]
pub enum FutureInitResult {
    EarlyError(InitResult),
    Future(::async_ffi::BorrowingFfiFuture<'static, InitResult>),
}

pub type SyncClassInitFn = unsafe extern "C" fn(config: PyroViewPtr) -> InitResult;
pub type AsyncClassInitFn = unsafe extern "C" fn(config: PyroViewPtr) -> FutureInitResult;

#[repr(C, u8)]
#[derive(Clone, Copy)]
pub enum ClassInitFn {
    Sync(SyncClassInitFn),
    Async(AsyncClassInitFn),
}

#[pin_project::pin_project(project = InitProj)]
pub enum ObjectInitFuture {
    Async(#[pin] ::async_ffi::BorrowingFfiFuture<'static, InitResult>),
    Ready(Option<Result<PyroObject, PyroError>>),
}

impl ObjectInitFuture {
    pub fn from_async(res: FutureInitResult) -> Self {
        match res {
            FutureInitResult::Future(fut) => Self::Async(fut),
            FutureInitResult::EarlyError(init_res) => {
                Self::Ready(Some(process_init_result(init_res)))
            }
        }
    }
}

impl Future for ObjectInitFuture {
    type Output = Result<PyroObject, PyroError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            InitProj::Async(fut) => match fut.poll(cx) {
                Poll::Ready(init_res) => Poll::Ready(process_init_result(init_res)),
                Poll::Pending => Poll::Pending,
            },
            InitProj::Ready(res) => Poll::Ready(
                res.take()
                    .expect("ObjectInitFuture polled after completion"),
            ),
        }
    }
}

fn process_init_result(res: InitResult) -> Result<PyroObject, PyroError> {
    let err_vec = unsafe { PyroVec::from_raw(res.error) }?;
    err_vec.parse_as_error()?;

    let state_ptr = NonNull::new(res.state.state).ok_or_else(|| {
        PyroError::CodePanic(
            CapturedError::new("Init returned null state without reporting an error").into(),
        )
    })?;

    Ok(PyroObject {
        state: state_ptr,
        dropper: res.state.dropper,
        span_id: res.state.span_id,
    })
}

// Updated to use PyroRefObjectPtr to allow borrowing state during reset
pub type AsyncClassResetFn = unsafe extern "C" fn(PyroRefObjectPtr) -> FuturePyroVec;
pub type SyncClassResetFn = unsafe extern "C" fn(PyroRefObjectPtr) -> PyroVecPtr;

#[repr(C, u8)]
#[derive(Clone, Copy)]
pub enum ClassResetFn {
    Sync(SyncClassResetFn),
    Async(AsyncClassResetFn),
    Null,
}

#[pin_project::pin_project(project = ResetProj)]
pub enum ObjectResetFuture {
    Async(#[pin] ::async_ffi::BorrowingFfiFuture<'static, PyroVecPtr>),
    Ready(Option<Result<(), PyroError>>),
}

impl ObjectResetFuture {
    pub fn from_async(res: FuturePyroVec) -> Self {
        match res {
            FuturePyroVec::Future(fut) => Self::Async(fut),
            FuturePyroVec::Early(ptr) => {
                let res = unsafe { PyroVec::from_raw(ptr) }.and_then(|v| v.parse_as_error());
                Self::Ready(Some(res))
            }
        }
    }
}

impl Future for ObjectResetFuture {
    type Output = Result<(), PyroError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            ResetProj::Async(fut) => match fut.poll(cx) {
                Poll::Ready(vec_ptr) => {
                    let res =
                        unsafe { PyroVec::from_raw(vec_ptr) }.and_then(|v| v.parse_as_error());
                    Poll::Ready(res)
                }
                Poll::Pending => Poll::Pending,
            },
            ResetProj::Ready(res) => Poll::Ready(
                res.take()
                    .expect("ObjectResetFuture polled after completion"),
            ),
        }
    }
}

// Registration

pub type AsyncClientRegisterFn =
    unsafe extern "C" fn(PyroRefObjectPtr, PyroViewPtr) -> FuturePyroVec;
pub type SyncClientRegisterFn = unsafe extern "C" fn(PyroRefObjectPtr, PyroViewPtr) -> PyroVecPtr;

#[repr(C, u8)]
#[derive(Clone, Copy)]
pub enum ClientRegisterFn {
    Sync(SyncClientRegisterFn),
    Async(AsyncClientRegisterFn),
    Null,
}

#[pin_project::pin_project(project = RegisterProj)]
pub enum ClientRegisterFuture {
    Async(#[pin] ::async_ffi::BorrowingFfiFuture<'static, PyroVecPtr>),
    Ready(Option<Result<PyroVec, PyroError>>),
}

impl ClientRegisterFuture {
    pub fn from_async(res: FuturePyroVec) -> Self {
        match res {
            FuturePyroVec::Future(fut) => Self::Async(fut),
            FuturePyroVec::Early(ptr) => {
                let res = unsafe { PyroVec::from_raw(ptr) };
                Self::Ready(Some(res))
            }
        }
    }
}

impl Future for ClientRegisterFuture {
    type Output = Result<PyroVec, PyroError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            RegisterProj::Async(fut) => match fut.poll(cx) {
                Poll::Ready(vec_ptr) => {
                    let res = unsafe { PyroVec::from_raw(vec_ptr) };
                    Poll::Ready(res)
                }
                Poll::Pending => Poll::Pending,
            },
            RegisterProj::Ready(res) => Poll::Ready(
                res.take()
                    .expect("ObjectResetFuture polled after completion"),
            ),
        }
    }
}

#[repr(C)]
pub struct ClassExport {
    pub name: *const u8,
    pub name_len: usize,
    pub ptr: *const MethodExport,
    pub init: ClassInitFn,
    pub reset: ClassResetFn,
    pub register: ClientRegisterFn,
    pub len: usize,
}

pub type CapabilityRegisterFn =
    unsafe extern "C" fn(class_id: i64, log_callback: LogCallback) -> ClassExport;

pub type ObjectHandle = i64;

pub struct ForeignClass {
    name: String,
    _library: Option<Arc<Library>>,
    methods: Vec<ForeignMethod>,
    init: ClassInitFn,
    reset: ClassResetFn,
    register: ClientRegisterFn,
}

struct ForeignMethod {
    pub name: String,
    pub pointer: Function,
}

impl ForeignMethod {
    fn new(method: &MethodExport) -> Result<Self, CapturedError> {
        if method.name.is_null() {
            return Err(CapturedError::new(
                "Found a method with an empty name (null pointer)",
            ));
        }
        if method.name_len == 0 {
            return Err(CapturedError::new("Found a method with an empty name"));
        }
        let name_bytes = unsafe { slice::from_raw_parts(method.name, method.name_len) };
        let func_name = std::str::from_utf8(name_bytes).map_err(|err| {
            CapturedError::new(format!("Method does not have a valid utf8 name: {err}"))
        })?;
        let pointer = method.func;
        Ok(Self {
            name: func_name.to_string(),
            pointer,
        })
    }
}

impl ForeignClass {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn method_names(&self) -> impl Iterator<Item = &str> {
        self.methods.iter().map(|m| m.name.as_str())
    }

    pub unsafe fn from_export(
        library: Arc<Library>,
        export: ClassExport,
    ) -> Result<Self, CapturedError> {
        unsafe { Self::from_export_inter(Some(library), export) }
    }

    unsafe fn from_export_inter(
        library: Option<Arc<Library>>,
        export: ClassExport,
    ) -> Result<Self, CapturedError> {
        let name = if export.name.is_null() || export.name_len == 0 {
            return Err(CapturedError::new("Empty name, unable to link"));
        } else {
            let name_bytes = unsafe { slice::from_raw_parts(export.name, export.name_len) };
            let name_str = std::str::from_utf8(name_bytes).map_err(|err| {
                CapturedError::new(format!("Class does not have a valid utf8 name: {err}"))
            })?;
            name_str.to_string()
        };

        let methods_slice = if export.ptr.is_null() || export.len == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(export.ptr, export.len) }
        };

        let methods = methods_slice
            .iter()
            .map(ForeignMethod::new)
            .collect::<Result<Vec<_>, CapturedError>>()?;

        Ok(Self {
            name,
            methods,
            _library: library,
            init: export.init,
            reset: export.reset,
            register: export.register,
        })
    }

    /// Creates an instance. Takes &self, uses internal mutability.
    pub async fn create_instance(
        self: &Arc<Self>,
        config: PyroView<'_>,
    ) -> Result<ForeignObject, PyroError> {
        let obj = match self.init {
            ClassInitFn::Sync(f) => process_init_result(unsafe { (f)(config.ptr()) }),
            ClassInitFn::Async(f) => {
                ObjectInitFuture::from_async(unsafe { (f)(config.ptr()) }).await
            }
        }?;

        Ok(ForeignObject {
            obj: Arc::new(obj),
            class: self.clone(),
        })
    }

    fn find_method(&self, name: &str) -> Option<&ForeignMethod> {
        self.methods.iter().find(|m| m.name == name)
    }
}

#[derive(Clone)]
pub struct ForeignObject {
    class: Arc<ForeignClass>,
    obj: Arc<PyroObject>,
}

impl ForeignObject {
    pub fn name(&self) -> &str {
        &self.class.name
    }

    pub fn method_names(&self) -> impl Iterator<Item = &str> {
        self.class.methods.iter().map(|m| m.name.as_str())
    }

    /// Resets the object. Locks the individual object, allowing others to be used.
    pub async fn reset(&self) -> Result<(), PyroError> {
        match self.class.reset {
            ClassResetFn::Sync(f) => {
                let vec_ptr = unsafe { f(self.obj.ref_ptr()) };
                unsafe { PyroVec::from_raw(vec_ptr) }.and_then(|v| v.parse_as_error())
            }
            ClassResetFn::Async(f) => {
                let fut_res = unsafe { f(self.obj.ref_ptr()) };
                ObjectResetFuture::from_async(fut_res).await
            }
            ClassResetFn::Null => Ok(()),
        }
    }
    /// Registers a client
    pub async fn register(&self, client_state: PyroView<'_>) -> Result<PyroVec, PyroError> {
        match self.class.register {
            ClientRegisterFn::Sync(f) => {
                let vec_ptr = unsafe { f(self.obj.ref_ptr(), client_state.ptr()) };
                let vec = unsafe { PyroVec::from_raw(vec_ptr) }?;
                vec.parse_as_error()?;
                Ok(vec)
            }
            ClientRegisterFn::Async(f) => {
                let fut_res = unsafe { f(self.obj.ref_ptr(), client_state.ptr()) };
                ClientRegisterFuture::from_async(fut_res).await
            }
            ClientRegisterFn::Null => Ok(PyroVec::ok()),
        }
    }

    pub async fn call(
        &self,
        method_name: &str,
        client_data: PyroView<'_>,
        input_data: PyroView<'_>,
    ) -> Result<PyroVec, PyroError> {
        let method = self
            .class
            .find_method(method_name)
            .ok_or_else(|| PyroError::NotFound(format!("Object method {method_name} not found")))?;

        match method.pointer {
            Function::Sync(f) => unsafe {
                PyroVec::from_raw((f)(self.obj.ref_ptr(), client_data.ptr(), input_data.ptr()))
            },
            Function::Async(f) => {
                MethodCallFuture::from_async(unsafe {
                    (f)(self.obj.ref_ptr(), client_data.ptr(), input_data.ptr())
                })
                .await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::PyroHeader;
    use std::cell::RefCell;
    use std::ptr;
    use std::sync::atomic::{AtomicBool, Ordering};
    const NAME: &'static str = "TestName";

    /// Holds the actual flags for a specific test instance.
    /// This will be Boxed and passed as the `state` (*mut c_void) of the PyroObject.
    struct TestContext {
        drop_called: Arc<AtomicBool>,
        reset_called: Arc<AtomicBool>,
        register_called: Arc<AtomicBool>,
    }

    // Thread local storage to safely pass the state pointer into `mock_init`
    // without requiring argument threading. This works perfectly with parallel cargo tests.
    thread_local! {
        static NEXT_OBJ_STATE: RefCell<Option<*mut c_void>> = RefCell::new(None);
    }

    unsafe extern "C" fn mock_dropper(ptr: *mut c_void) {
        if ptr.is_null() {
            return;
        }
        // Retake ownership of the box so it drops cleanly when this scope ends
        let ctx = unsafe { Box::from_raw(ptr as *mut TestContext) };
        ctx.drop_called.store(true, Ordering::SeqCst);
    }

    unsafe extern "C" fn mock_init(_: PyroViewPtr) -> InitResult {
        // Retrieve the pointer pre-allocated by the test setup
        let state = NEXT_OBJ_STATE.with(|s| {
            s.borrow_mut()
                .take()
                .expect("mock_init called but no state was prepared in TLS")
        });

        InitResult {
            state: PyroObjectPtr {
                state,
                dropper: mock_dropper,
                span_id: 0,
            },
            error: PyroVec::ok().into_raw(),
        }
    }

    unsafe extern "C" fn mock_reset_sync(ptr: PyroRefObjectPtr) -> PyroVecPtr {
        if !ptr.state.is_null() {
            let ctx = unsafe { &*(ptr.state as *const TestContext) };
            ctx.reset_called.store(true, Ordering::SeqCst);
        }
        PyroVec::ok().into_raw()
    }

    unsafe extern "C" fn mock_register_sync(
        ptr: PyroRefObjectPtr,
        _client: PyroViewPtr,
    ) -> PyroVecPtr {
        if !ptr.state.is_null() {
            let ctx = unsafe { &*(ptr.state as *const TestContext) };
            ctx.register_called.store(true, Ordering::SeqCst);
        }
        PyroVec::ok().into_raw()
    }

    // ============================================================================
    // 3. Test Helper API
    // ============================================================================

    pub struct Mocks {
        pub drop_called: Arc<AtomicBool>,
        pub reset_called: Arc<AtomicBool>,
        pub register_called: Arc<AtomicBool>,

        /// The raw pointer to the TestContext.
        /// For PyroObject::new() tests, pass this directly.
        /// For ForeignClass tests, this is auto-injected via TLS when init is called.
        pub state_ptr: *mut c_void,

        // The function pointers to use in ClassExport
        pub init: ClassInitFn,
        pub dropper: ClassDropper,
        pub reset: ClassResetFn,
        pub register: ClientRegisterFn,
    }

    fn mocks() -> Mocks {
        let drop_called = Arc::new(AtomicBool::new(false));
        let reset_called = Arc::new(AtomicBool::new(false));
        let register_called = Arc::new(AtomicBool::new(false));

        let context = Box::new(TestContext {
            drop_called: drop_called.clone(),
            reset_called: reset_called.clone(),
            register_called: register_called.clone(),
        });

        let state_ptr = Box::into_raw(context) as *mut c_void;

        // Pre-load the TLS so if mock_init is called on this thread, it grabs this state
        NEXT_OBJ_STATE.with(|s| {
            *s.borrow_mut() = Some(state_ptr);
        });

        Mocks {
            drop_called,
            reset_called,
            register_called,
            state_ptr,
            init: ClassInitFn::Sync(mock_init),
            dropper: mock_dropper,
            reset: ClassResetFn::Sync(mock_reset_sync),
            register: ClientRegisterFn::Sync(mock_register_sync),
        }
    }

    // ============================================================================
    // 4. Refactored Tests
    // ============================================================================

    #[test]
    fn test_owned_lifecycle() {
        let m = mocks();

        // Pass m.state_ptr directly. The Mocks struct ensures `NEXT_OBJ_STATE` is set,
        // but for manual `PyroObject::new`, we just use the pointer.
        // Important: clear TLS to avoid leakage if we aren't using init
        NEXT_OBJ_STATE.with(|s| {
            *s.borrow_mut() = None;
        });

        let obj = unsafe { PyroObject::new(m.state_ptr, m.dropper, 0) }.unwrap();
        drop(obj);

        assert!(
            m.drop_called.load(Ordering::SeqCst),
            "Dropper should be called on drop"
        );
    }

    #[test]
    fn test_ref_ptr_does_not_drop() {
        let m = mocks();
        NEXT_OBJ_STATE.with(|s| {
            *s.borrow_mut() = None;
        }); // Manual creation, clear TLS

        let obj = unsafe { PyroObject::new(m.state_ptr, m.dropper, 0) }.unwrap();
        let ref_ptr = obj.ref_ptr();

        assert_eq!(ref_ptr.state, m.state_ptr);

        let rust_ref = unsafe { PyroObjectRef::from_raw(ref_ptr) }.unwrap();

        drop(rust_ref);
        assert!(!m.drop_called.load(Ordering::SeqCst));

        drop(obj);
        assert!(m.drop_called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_foreign_class_reset() {
        let m = mocks();

        let export = ClassExport {
            name: NAME.as_ptr(),
            name_len: NAME.len(),
            ptr: ptr::null(),
            len: 0,
            init: m.init,   // Uses TLS to find m.state_ptr
            reset: m.reset, // Syncs with m.reset_called
            register: ClientRegisterFn::Null,
        };

        let class = Arc::new(unsafe { ForeignClass::from_export_inter(None, export) }.unwrap());

        // Create Instance (calls mock_init, which claims m.state_ptr from TLS)
        let config = PyroVec::ok();
        let handle = class.create_instance(config.view()).await.unwrap();

        // Call Reset
        handle.reset().await.unwrap();
        assert!(
            m.reset_called.load(Ordering::SeqCst),
            "Reset function should have been called"
        );

        // Cleanup happens when `handle` is dropped at end of scope
    }

    #[tokio::test]
    async fn test_foreign_class_register() {
        let m = mocks();

        let export = ClassExport {
            name: NAME.as_ptr(),
            name_len: NAME.len(),
            ptr: ptr::null(),
            len: 0,
            init: m.init,
            reset: ClassResetFn::Null,
            register: m.register,
        };

        let class = Arc::new(unsafe { ForeignClass::from_export_inter(None, export) }.unwrap());

        let config = PyroVec::ok();
        let handle = class.create_instance(config.view()).await.unwrap();

        let client_data = PyroVec::ok();
        let result = handle.register(client_data.view()).await.unwrap();

        assert!(m.register_called.load(Ordering::SeqCst));
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_foreign_class_register_null() {
        let m = mocks();

        let export = ClassExport {
            name: NAME.as_ptr(),
            name_len: NAME.len(),
            ptr: ptr::null(),
            len: 0,
            init: m.init,
            reset: ClassResetFn::Null,
            register: ClientRegisterFn::Null, // Explicitly testing null path
        };

        let class = Arc::new(unsafe { ForeignClass::from_export_inter(None, export) }.unwrap());
        let config = PyroVec::ok();
        let handle = class.create_instance(config.view()).await.unwrap();

        let client_data = PyroVec::ok();
        let result = handle.register(client_data.view()).await;

        assert!(result.is_ok());
    }
}
