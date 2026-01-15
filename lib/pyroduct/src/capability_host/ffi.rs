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

pub type PluginRegisterFn<'a> =
    unsafe extern "C" fn(plugin_id: u64, log_callback: LogCallback) -> PluginExports<'a>;

pub type AsyncPluginProcessFn<'a> = unsafe extern "C" fn(
    *const u8,
    usize,
    *const u8,
    usize,
    *mut c_void,
) -> FfiBorrowedFutureResult<'a>;

pub type SyncPluginProcessFn =
    unsafe extern "C" fn(*const u8, usize, *const u8, usize, *mut c_void) -> FfiResult;

#[repr(C, u8)]
#[derive(Clone, Copy)]
pub enum PluginFunction<'a> {
    Sync(SyncPluginProcessFn),
    Async(AsyncPluginProcessFn<'a>),
}

#[repr(C)]
pub struct PluginExport<'a> {
    pub module: *const u8,
    pub module_len: usize,
    pub name: *const u8,
    pub name_len: usize,
    pub func: PluginFunction<'a>,
}

#[repr(C)]
pub struct PluginExports<'a> {
    pub ptr: *mut PluginExport<'a>,
    pub init: PluginInitFn<'a>,
    pub drop: PluginDropFn,
    pub reset: PluginResetFn<'a>,
    pub len: usize,
    pub cap: usize,
}

// --- Init Functions ---

// UPDATED: Returns FfiBorrowedFutureObjectResult
pub type AsyncPluginInitFn<'a> =
    unsafe extern "C" fn(config: *const u8, config_len: usize) -> FfiBorrowedFutureObjectResult<'a>;

pub type SyncPluginInitFn =
    unsafe extern "C" fn(config: *const u8, config_len: usize) -> FfiInitResult;

#[repr(C, u8)]
#[derive(Clone, Copy)]
pub enum PluginInitFn<'a> {
    Sync(SyncPluginInitFn),
    Async(AsyncPluginInitFn<'a>),
    Null,
}

#[repr(C, u8)]
#[derive(Clone, Copy)]
pub enum PluginDropFn {
    Sync(unsafe extern "C" fn(*mut c_void)),
    Null,
}

// --- Reset Functions ---

// UPDATED: Returns FfiBorrowedFutureResult (yields FfiResult)
pub type AsyncPluginResetFn<'a> = unsafe extern "C" fn(*mut c_void) -> FfiBorrowedFutureResult<'a>;
pub type SyncPluginResetFn = unsafe extern "C" fn(*mut c_void) -> FfiResult;

#[repr(C, u8)]
#[derive(Clone, Copy)]
pub enum PluginResetFn<'a> {
    Sync(SyncPluginResetFn),
    Async(AsyncPluginResetFn<'a>),
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
