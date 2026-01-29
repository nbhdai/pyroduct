use std::ffi::c_void;
use std::ptr;

pub type LogCallback = unsafe extern "C" fn(u64, *const u8, usize);

#[repr(C)]
pub struct FfiResult {
    /// Discriminant: 0 = Ok, 1 = Err
    pub tag: u8,
    /// The actual output (success or error)
    pub output: COutput,
}

#[repr(C)]
pub struct COutput {
    pub ptr: *const u8,
    pub len: usize,
    pub cap: usize,
}

// --- Init Result Types ---

#[repr(C)]
pub struct FfiInitResult {
    /// 0 = Ok, 1 = Err
    pub tag: u8,
    /// The opaque state pointer (valid if tag == 0)
    pub state: *mut c_void,
    /// The serialized error (valid if tag == 1)
    pub error: COutput,
}

impl FfiInitResult {
    pub fn ok(state: *mut c_void) -> Self {
        Self {
            tag: 0,
            state,
            error: COutput {
                ptr: ptr::null(),
                len: 0,
                cap: 0,
            },
        }
    }

    pub fn err(error: COutput) -> Self {
        Self {
            tag: 1,
            state: ptr::null_mut(),
            error,
        }
    }
}

// --- Future Result Types ---

#[repr(C)]
pub enum FfiBorrowedFutureResult<'a> {
    /// The operation failed immediately (e.g., input deserialization failed).
    EarlyError(FfiResult),
    /// The operation started successfully.
    Future(::async_ffi::BorrowingFfiFuture<'a, FfiResult>),
}

// NEW: Future result for Object creation (Init)
#[repr(C)]
pub enum FfiBorrowedFutureObjectResult<'a> {
    /// The operation failed immediately.
    EarlyError(FfiInitResult),
    /// The operation started successfully.
    Future(::async_ffi::BorrowingFfiFuture<'a, FfiInitResult>),
}

// --- Function Typedefs ---

pub type AsyncFn<'a> = unsafe extern "C" fn(
    *const u8,
    usize,
    *const u8,
    usize,
    *mut c_void,
) -> FfiBorrowedFutureResult<'a>;

pub type SyncFn =
    unsafe extern "C" fn(*const u8, usize, *const u8, usize, *mut c_void) -> FfiResult;

#[repr(C, u8)]
#[derive(Clone, Copy)]
pub enum Function<'a> {
    Sync(SyncFn),
    Async(AsyncFn<'a>),
}

#[repr(C)]
pub struct FunctionExport<'a> {
    pub module: *const u8,
    pub module_len: usize,
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
pub type SyncClassResetFn = unsafe extern "C" fn(*mut c_void) -> FfiResult;

#[repr(C, u8)]
#[derive(Clone, Copy)]
pub enum ClassResetFn<'a> {
    Sync(SyncClassResetFn),
    Async(AsyncClassResetFn<'a>),
    Null,
}

impl FfiResult {
    pub fn ok_null() -> Self {
        Self {
            tag: 0,
            output: COutput {
                ptr: ptr::null(),
                len: 0,
                cap: 0,
            },
        }
    }

    pub fn ok(output: COutput) -> Self {
        Self { tag: 0, output }
    }

    pub fn full_err(error: COutput) -> Self {
        Self {
            tag: 1,
            output: error,
        }
    }

    pub fn partial_error(error: COutput) -> Self {
        Self {
            tag: 2,
            output: error,
        }
    }
}

pub type CapabilityRegisterFn<'a> =
    unsafe extern "C" fn(class_id: u64, log_callback: LogCallback) -> ClassExport<'a>;
