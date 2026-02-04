use std::ffi::c_void;
use std::ptr;

use bridge_vec::{BridgeError, BridgeVec};

pub type LogCallback = unsafe extern "C" fn(u64, *const u8, usize);



// --- Init Result Types ---

#[repr(C)]
pub struct FfiInitResult {
    /// The opaque state pointer (valid if tag == 0)
    pub state: *mut c_void,
    /// The serialized Result<(),BridgeError> as a BridgeVec. 
    /// If not Ok<()> this contains the error.
    pub error: *const u8,
}

impl FfiInitResult {
    pub fn ok(state: *mut c_void) -> Self {
        Self {
            state,
            error: BridgeVec::ok().into_raw(),
        }
    }

    pub fn err(error: *const u8) -> Self {
        Self {
            state: ptr::null_mut(),
            error,
        }
    }
}

impl<'a> From<BridgeError> for FfiInitResult {
    fn from(value: BridgeError) -> Self {
        Self {
            state: ptr::null_mut(),
            error: value.encode().into_raw(),
        }
    }
}

// --- Future Result Types ---

#[repr(C)]
pub enum FfiBorrowedFutureResult<'a> {
    /// The operation failed immediately (e.g., input deserialization failed).
    /// The error is encoded in a BridgeVec
    EarlyError(*const u8),
    /// The operation started successfully.
    /// 
    /// The result is encoded in a BridgeVec
    Future(::async_ffi::BorrowingFfiFuture<'a, *const u8>),
}

impl<'a> From<BridgeError> for FfiBorrowedFutureResult<'a> {
    fn from(value: BridgeError) -> Self {
        FfiBorrowedFutureResult::EarlyError(value.encode().into_raw())
    }
}

impl<'a> From<BridgeVec> for FfiBorrowedFutureResult<'a> {
    fn from(value: BridgeVec) -> Self {
        FfiBorrowedFutureResult::EarlyError(value.into_raw())
    }
}

impl<'a> From<::async_ffi::BorrowingFfiFuture<'a, *const u8>> for FfiBorrowedFutureResult<'a> {
    fn from(value: ::async_ffi::BorrowingFfiFuture<'a, *const u8>) -> Self {
        FfiBorrowedFutureResult::Future(value)
    }
}

// NEW: Future result for Object creation (Init)
#[repr(C)]
pub enum FfiBorrowedFutureObjectResult<'a> {
    /// The operation failed immediately.
    EarlyError(FfiInitResult),
    /// The operation started successfully.
    Future(::async_ffi::BorrowingFfiFuture<'a, FfiInitResult>),
}

impl<'a> From<BridgeError> for FfiBorrowedFutureObjectResult<'a> {
    fn from(value: BridgeError) -> Self {
        FfiBorrowedFutureObjectResult::EarlyError(value.into())
    }
}

// --- Function Typedefs ---

/// We expect the return to be a future that resolves into a bridge vec.
pub type AsyncFn<'a> = unsafe extern "C" fn(
    *const u8,
    usize,
    *const u8,
    usize,
    *mut c_void,
) -> FfiBorrowedFutureResult<'a>;

/// We expect the return to be a bridge vec.
pub type SyncFn =
    unsafe extern "C" fn(*const u8, usize, *const u8, usize, *mut c_void) -> *const u8;

#[repr(C, u8)]
#[derive(Clone, Copy)]
pub enum Function<'a> {
    Sync(SyncFn),
    Async(AsyncFn<'a>),
}

#[repr(C)]
pub struct FunctionExport<'a> {
    pub capability: *const u8,
    pub capability_len: usize,
    pub name: *const u8,
    pub name_len: usize,
    pub func: Function<'a>,
}

#[repr(C)]
pub struct ClassExport<'a> {
    pub ptr: *const FunctionExport<'a>,
    pub init: ClassInitFn<'a>,
    pub drop: ClassDropFn,
    pub reset: ClassResetFn<'a>,
    pub len: usize,
}

// --- Init Functions ---

// UPDATED: Returns FfiBorrowedFutureObjectResult
pub type AsyncClassInitFn<'a> =
    unsafe extern "C" fn(config: *const u8, config_len: usize) -> FfiBorrowedFutureObjectResult<'a>;

pub type SyncClassInitFn =
    unsafe extern "C" fn(config: *const u8, config_len: usize) -> FfiInitResult;

#[repr(C, u8)]
#[derive(Clone, Copy)]
pub enum ClassInitFn<'a> {
    Sync(SyncClassInitFn),
    Async(AsyncClassInitFn<'a>),
    Null,
}

#[repr(C, u8)]
#[derive(Clone, Copy)]
pub enum ClassDropFn {
    Sync(unsafe extern "C" fn(*mut c_void)),
    Null,
}

// --- Reset Functions ---

// UPDATED: Returns FfiBorrowedFutureResult (yields FfiResult)
pub type AsyncClassResetFn<'a> = unsafe extern "C" fn(*mut c_void) -> FfiBorrowedFutureResult<'a>;
pub type SyncClassResetFn = unsafe extern "C" fn(*mut c_void) -> *const u8;

#[repr(C, u8)]
#[derive(Clone, Copy)]
pub enum ClassResetFn<'a> {
    Sync(SyncClassResetFn),
    Async(AsyncClassResetFn<'a>),
    Null,
}

pub type CapabilityRegisterFn<'a> =
    unsafe extern "C" fn(class_id: u64, log_callback: LogCallback) -> ClassExport<'a>;
