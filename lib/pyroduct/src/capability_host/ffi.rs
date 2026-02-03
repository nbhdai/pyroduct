use std::ffi::c_void;
use len_aligned_vec::{LenAlignedVec, DataStatus};

pub type LogCallback = unsafe extern "C" fn(u64, *const u8, usize);

pub type FfiBufferPtr = *mut u8;

#[repr(C)]
pub struct FfiInitResult {
    /// The opaque state pointer (valid if the buffer status is ValidData)
    pub state: *mut c_void,
    /// Handle to a LenAlignedVec. If state is null, this contains the error.
    /// If state is valid, this might contain initialization metadata or be null.
    pub buffer: FfiBufferPtr,
}

// --- Future Result Types ---

#[repr(C)]
pub enum FfiBorrowedFutureResult<'a> {
    /// Failed immediately (e.g., input deserialization failed).
    /// Buffer status will be TransportError or UserError.
    EarlyError(FfiBufferPtr),
    /// The operation started successfully.
    Future(::async_ffi::BorrowingFfiFuture<'a, FfiBufferPtr>),
}

#[repr(C)]
pub enum FfiBorrowedFutureObjectResult<'a> {
    EarlyError(FfiInitResult),
    Future(::async_ffi::BorrowingFfiFuture<'a, FfiInitResult>),
}

// --- Function Typedefs ---

/// All functions now take and return raw FfiBufferPtr (LenAlignedVec pointers).
pub type AsyncFn<'a> = unsafe extern "C" fn(
    args_ptr: FfiBufferPtr,
    state: *mut c_void,
) -> FfiBorrowedFutureResult<'a>;

pub type SyncFn =
    unsafe extern "C" fn(args_ptr: FfiBufferPtr, state: *mut c_void) -> FfiBufferPtr;

#[repr(C, u8)]
#[derive(Clone, Copy)]
pub enum Function<'a> {
    Sync(SyncFn),
    Async(AsyncFn<'a>),
}

// --- Export Structures ---

#[repr(C)]
pub struct FunctionExport<'a> {
    /// Capability and Name are now passed as LenAlignedVec pointers 
    /// to ensure they carry their own length and status.
    pub capability: FfiBufferPtr,
    pub name: FfiBufferPtr,
    pub func: Function<'a>,
}

#[repr(C)]
pub struct ClassExport<'a> {
    pub funcs: *const FunctionExport<'a>,
    pub init: ClassInitFn<'a>,
    pub drop: ClassDropFn,
    pub reset: ClassResetFn<'a>,
    pub len: usize,
}

pub type AsyncClassInitFn<'a> =
    unsafe extern "C" fn(config_ptr: FfiBufferPtr) -> FfiBorrowedFutureObjectResult<'a>;

pub type SyncClassInitFn =
    unsafe extern "C" fn(config_ptr: FfiBufferPtr) -> FfiInitResult;

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

pub type AsyncClassResetFn<'a> = unsafe extern "C" fn(*mut c_void) -> FfiBorrowedFutureResult<'a>;
pub type SyncClassResetFn = unsafe extern "C" fn(*mut c_void) -> FfiBufferPtr;

#[repr(C, u8)]
#[derive(Clone, Copy)]
pub enum ClassResetFn<'a> {
    Sync(SyncClassResetFn),
    Async(AsyncClassResetFn<'a>),
    Null,
}

pub type CapabilityRegisterFn<'a> =
    unsafe extern "C" fn(class_id: u64, log_callback: LogCallback) -> ClassExport<'a>;