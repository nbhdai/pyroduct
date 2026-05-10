use std::ffi::c_void;
use std::ptr::NonNull;

use crate::format::PyroView;
use crate::format::vec_buf::PyroRefPtr;
use crate::format::{PyroVec, PyroViewPtr, header::PyroData};
use crate::{CapturedError, PyroError};

pub type LogCallback = unsafe extern "C" fn(i64, u64, u32, *const u8, usize);

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
    PyroRefPtr,
    // Input
    PyroRefPtr,
) -> FuturePyroView;

/// We expect the return to be a bridge vec.
pub type SyncFn = unsafe extern "C" fn(PyroRefObjectPtr, PyroRefPtr, PyroRefPtr) -> PyroViewPtr;

#[repr(C, u8)]
#[derive(Clone, Copy)]
pub enum Function {
    Sync(SyncFn),
    Async(AsyncFn),
}

#[repr(C)]
pub enum FuturePyroView {
    /// The operation succeeded or failed immediately.
    Early(PyroViewPtr),
    /// The operation started successfully, and we have to await the result.
    Future(::async_ffi::BorrowingFfiFuture<'static, PyroViewPtr>),
}

impl From<PyroViewPtr> for FuturePyroView {
    fn from(value: PyroViewPtr) -> Self {
        FuturePyroView::Early(value)
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
    pub object_id: u64,
}

unsafe impl Send for PyroObjectPtr {}

/// The FFI-safe representation of a BORROWED instance.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PyroRefObjectPtr {
    pub state: *mut c_void,
    pub object_id: u64,
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
    pub object_id: u64,
}

unsafe impl Send for PyroObject {}
unsafe impl Sync for PyroObject {}

impl PyroObject {
    /// Creates a new PyroObject from raw components.
    pub unsafe fn new(
        state: *mut c_void,
        dropper: ClassDropper,
        object_id: u64,
    ) -> Result<Self, CapturedError> {
        let state = NonNull::new(state)
            .ok_or_else(|| CapturedError::new("Cannot construct PyroObject from null pointer"))?;

        Ok(Self {
            state,
            dropper,
            object_id,
        })
    }

    /// Consumes the wrapper and returns the FFI-safe pointer struct.
    pub fn ptr(self) -> PyroObjectPtr {
        let ptr = PyroObjectPtr {
            state: self.state.as_ptr(),
            dropper: self.dropper,
            object_id: self.object_id,
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
            object_id: raw.object_id,
        })
    }

    /// Creates an FFI-safe reference pointer.
    pub fn ref_ptr(&self) -> PyroRefObjectPtr {
        PyroRefObjectPtr {
            state: self.state.as_ptr(),
            object_id: self.object_id,
        }
    }

    /// Creates a lifetime-bound Rust reference wrapper.
    pub fn as_borrowed(&self) -> PyroObjectRef {
        PyroObjectRef {
            state: self.state,
            object_id: self.object_id,
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
    pub object_id: u64,
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
            object_id: raw.object_id,
        })
    }

    pub fn as_raw(&self) -> PyroRefObjectPtr {
        PyroRefObjectPtr {
            state: self.state.as_ptr(),
            object_id: self.object_id,
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
    pub state: PyroObjectPtr,
    pub error: PyroViewPtr,
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
    pub fn init_ok<S: 'static>(state: S, object_id: u64) -> InitResult {
        let state_ptr = Box::into_raw(Box::new(state)) as *mut std::ffi::c_void;
        tracing::debug!(?state_ptr, "Object allocated and forgotten");
        InitResult {
            state: PyroObjectPtr {
                state: state_ptr,
                dropper: typed_dropper::<S>,
                object_id: object_id,
            },
            error: PyroVec::ok().view().into_ptr(),
        }
    }

    /// Construct an error `InitResult` from a `PyroError`.
    pub fn init_err(err: PyroError, object_id: u64) -> InitResult {
        InitResult {
            state: PyroObjectPtr {
                state: std::ptr::null_mut(),
                dropper: typed_dropper::<()>,
                object_id: object_id,
            },
            error: err.encode().view().into_ptr(),
        }
    }

    pub fn process(self) -> Result<PyroObject, PyroError> {
        let err_vec = unsafe { PyroView::from_ptr(self.error) }?;
        err_vec.parse_as_error()?;

        let state_ptr = NonNull::new(self.state.state).ok_or_else(|| {
            PyroError::CodePanic(
                CapturedError::new("Init returned null state without reporting an error").into(),
            )
        })?;

        Ok(PyroObject {
            state: state_ptr,
            dropper: self.state.dropper,
            object_id: self.state.object_id,
        })
    }
}

#[repr(C)]
pub enum FutureInitResult {
    EarlyError(InitResult),
    Future(::async_ffi::BorrowingFfiFuture<'static, InitResult>),
}

pub type SyncClassInitFn = unsafe extern "C" fn(config: PyroRefPtr, object_id: u64) -> InitResult;
pub type AsyncClassInitFn =
    unsafe extern "C" fn(config: PyroRefPtr, object_id: u64) -> FutureInitResult;

#[repr(C, u8)]
#[derive(Clone, Copy)]
pub enum ClassInitFn {
    Sync(SyncClassInitFn),
    Async(AsyncClassInitFn),
}

// Updated to use PyroRefObjectPtr to allow borrowing state during reset
pub type AsyncClassResetFn = unsafe extern "C" fn(PyroRefObjectPtr) -> FuturePyroView;
pub type SyncClassResetFn = unsafe extern "C" fn(PyroRefObjectPtr) -> PyroViewPtr;

#[repr(C, u8)]
#[derive(Clone, Copy)]
pub enum ClassResetFn {
    Sync(SyncClassResetFn),
    Async(AsyncClassResetFn),
    Null,
}

// Registration

pub type AsyncClientRegisterFn =
    unsafe extern "C" fn(PyroRefObjectPtr, PyroRefPtr) -> FuturePyroView;
pub type SyncClientRegisterFn = unsafe extern "C" fn(PyroRefObjectPtr, PyroRefPtr) -> PyroViewPtr;

#[repr(C, u8)]
#[derive(Clone, Copy)]
pub enum ClientRegisterFn {
    Sync(SyncClientRegisterFn),
    Async(AsyncClientRegisterFn),
    Null,
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
