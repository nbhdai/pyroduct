use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe, Location, PanicHookInfo};
use std::sync::Once;
use rkyv::Archive;
use tracing::{debug, error, trace};

// Assuming these exist in your crate based on the snippet
use crate::{BridgeError, BridgeVec, Bridgeable, CapturedError, DataStatus, TypedBuf};

// ============================================================================
// 1. Error Definitions & Data Structures
// ============================================================================

/// Errors occurring on the remote side of the FFI boundary.
/// This is what is serialized into the BridgeVec when Status is TransportError.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum FfiError {
    /// The remote side panicked while processing the request.
    Panic(Box<CapturedError>),
    /// The remote side completed logic, but failed to serialize the result.
    SerializationFailed(String),
    /// A raw error string (used for fallbacks or raw UTF-8 errors).
    Generic(String),
}

impl std::fmt::Display for FfiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FfiError::Panic(p) => write!(f, "Remote Panic at {}:{}: {}", p.file, p.line, p.message),
            FfiError::SerializationFailed(msg) => write!(f, "Serialization Failed: {}", msg),
            FfiError::Generic(msg) => write!(f, "Generic FFI Error: {}", msg),
        }
    }
}

impl std::error::Error for FfiError {}

// ============================================================================
// 2. Panic Infrastructure (Thread Local Storage & Hooks)
// ============================================================================

thread_local! {
    /// Temporarily holds panic info for the current thread so it can be retrieved
    /// after `catch_unwind` returns.
    static LAST_FFI_PANIC: RefCell<Option<Box<CapturedError>>> = RefCell::new(None);
}

static REGISTER_PANIC_HOOK: Once = Once::new();

pub fn register_ffi_panic_hook() {
    REGISTER_PANIC_HOOK.call_once(|| {
        debug!("register_ffi_panic_hook: installing global panic hook for FFI boundary");
        let default_hook = panic::take_hook();

        panic::set_hook(Box::new(move |info: &PanicHookInfo| {
            // ... (Existing message extraction logic) ...
            let mut error = if let Some(s) = info.payload().downcast_ref::<&str>() {
                CapturedError::new(*s)
            } else if let Some(s) = info.payload().downcast_ref::<String>() {
                CapturedError::new(s)
            } else {
                CapturedError::new("Panic occurred (unknown payload type)")
            };

            // ... (Existing location extraction logic) ...
            if let Some(loc) = info.location() {
                error = error.with_location(loc);
            };
            
            // --- NEW: Capture Stack Trace ---
            error = error.with_backtrace(std::backtrace::Backtrace::capture());

            error!(?error,"FFI Panic Hook captured a panic");
            LAST_FFI_PANIC.with(|slot| {
                *slot.borrow_mut() = Some(Box::new(error));
            });

            default_hook(info);
        }));
    });
}

/// Retrieves and clears the last panic from TLS.
pub fn recover_panic_info() -> Box<CapturedError> {
    LAST_FFI_PANIC.with(|slot| {
        slot.borrow_mut().take().unwrap_or_else(|| {
            error!("recover_panic_info: panic detected but no details found in TLS");
            Box::new(CapturedError::new("Panic caught via catch_unwind, but TLS was empty."))
        })
    })
}

/// Clears any stale panic state before starting a new FFI call.
pub fn clear_last_panic() {
    LAST_FFI_PANIC.with(|slot| *slot.borrow_mut() = None);
}

// ============================================================================
// 3. Execution & Serialization Logic
// ============================================================================

/// Safe entry point for FFI operations.
/// 1. Clears old state.
/// 2. Catches Unwind.
/// 3. Serializes Output or Error.
#[track_caller]
pub fn execute_safe<F, O>(func: F) -> *const u8
where
    O: Bridgeable + std::panic::RefUnwindSafe,
    F: FnOnce() -> O + std::panic::UnwindSafe,
{
    // Ensure the hook is registered (idempotent due to Once)
    register_ffi_panic_hook();
    clear_last_panic();

    let result = panic::catch_unwind(func);

    match result {
        Ok(output_obj) => {
            debug!("execute_safe: logic completed, serializing output");
            let location = Location::caller();
            serialize_output(&output_obj, location).into_raw()
        }
        Err(_) => {
            let panic_info = recover_panic_info();
            trace!(panic = ?panic_info, "execute_safe: panic caught, returning error");
            BridgeError::RemotePanic(panic_info).encode().into_raw()
        }
    }
}

/// Safe entry point for FFI operations.
/// 1. Clears old state.
/// 2. Catches Unwind.
/// 3. Serializes Output or Error.
#[track_caller]
pub fn execute_safe_result<F, O, E>(func: F) -> *const u8
where
    O: Bridgeable + std::panic::RefUnwindSafe,
    E: Bridgeable + std::panic::RefUnwindSafe,
    F: FnOnce() -> Result<O, E> + std::panic::UnwindSafe,
{
    // Ensure the hook is registered (idempotent due to Once)
    register_ffi_panic_hook();
    clear_last_panic();

    let result = panic::catch_unwind(func);

    match result {
        Ok(output_obj) => {
            debug!("execute_safe: logic completed, serializing output");
            let location = Location::caller();
            serialize_result(&output_obj, location).into_raw()
        }
        Err(_) => {
            // catch_unwind gives us a Box<dyn Any>, but we want the rich info
            // we captured in our custom hook via TLS.
            let panic_info = recover_panic_info();
            trace!(panic = ?panic_info, "execute_safe: panic caught, returning error");
            BridgeError::RemotePanic(panic_info).encode().into_raw()
        }
    }
}

/// Helper to serialize a successful result.
pub fn deserialize_input<I>(data: *const u8, location: &Location) -> Result<I, BridgeError>
where
    I: Bridgeable + Send + std::panic::UnwindSafe,
    <I as Archive>::Archived: 'static,
{
    // We construct a nested catch_unwind here in case *serialization itself* panics.
    let guard = panic::catch_unwind(AssertUnwindSafe(|| {
        unsafe { TypedBuf::from_raw(data) }.and_then(|v| I::deserialize(&v).map_err(BridgeError::from))
    }));

    match guard {
        Ok(input) => input,
        Err(_) => {
            // Serialization panicked
            let panic_info = recover_panic_info();
            let panic_info = panic_info.with_location(location);
            Err(BridgeError::DeserializationPanic(panic_info.into()))
        }
    }
}

/// Helper to serialize a successful result.
pub fn serialize_output<T>(val: &T, location: &Location) -> BridgeVec
where
    T: Bridgeable,
{
    // We construct a nested catch_unwind here in case *serialization itself* panics.
    let guard = panic::catch_unwind(AssertUnwindSafe(|| val.serialize()));

    match guard {
        Ok(Ok(mut vec)) => {
            // Success path
            vec.set_status(DataStatus::ValidData as u8);
            vec
        }
        Ok(Err(e)) => {
            e.encode()
        }
        Err(_) => {
            // Serialization panicked
            let panic_info = recover_panic_info();
            let panic_info = panic_info.with_location(location);
            BridgeError::SerializationPanic(panic_info.into()).encode()
        }
    }
}

/// Helper to serialize a successful result.
pub fn serialize_result<T, E>(result: &Result<T, E>, location: &Location) -> BridgeVec
where
    T: Bridgeable,
    E: Bridgeable,
{
    // We construct a nested catch_unwind here in case *serialization itself* panics.
    let guard = panic::catch_unwind(AssertUnwindSafe(|| BridgeVec::serialize_result(result)));

    match guard {
        Ok(Ok(mut vec)) => {
            // Success path
            vec.set_status(DataStatus::ValidData as u8);
            vec
        }
        Ok(Err(e)) => {
            // Logic worked, but serialization failed
            e.encode()
        }
        Err(_) => {
            // Serialization panicked
            let panic_info = recover_panic_info();
            let panic_info = panic_info.with_location(location);
            BridgeError::SerializationPanic(panic_info.into()).encode()
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use async_ffi::*;

#[cfg(not(target_arch = "wasm32"))]
mod async_ffi {
    use std::sync::OnceLock;

    use ::async_ffi::BorrowingFfiFuture;
    use futures::FutureExt;
    use tokio::runtime::Runtime;

    use super::*;
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();

    pub fn get_runtime() -> &'static Runtime {
        RUNTIME.get_or_init(|| {
            trace!("get_runtime: initializing tokio runtime");
            Runtime::new().expect("Failed to create Tokio runtime in plugin")
        })
    }

    #[track_caller]
    pub fn execute_safe_async<'a, Fut, O>(fut: Fut) -> BorrowingFfiFuture<'a, *const u8>
    where
        Fut: std::future::Future<Output = O> + Send + std::panic::UnwindSafe + 'a,
        O: Bridgeable + std::panic::RefUnwindSafe + Send + 'static,
    {
        trace!("execute_safe_async: preparing future");
        let _guard = get_runtime().enter();

        BorrowingFfiFuture::<'a>::new(async move {
            trace!("execute_safe_async: future polling started");
            clear_last_panic();
            let result = std::panic::AssertUnwindSafe(fut).catch_unwind().await;

            match result {
                Ok(val) => {
                    debug!("execute_safe_async: logic completed successfully");
                    let location = Location::caller();
                    serialize_output(&val, location).into_raw()
                }

                Err(_) => {
                    let panic_info = recover_panic_info();
                    trace!(panic = ?panic_info, "execute_safe_async: panic caught, returning error");
                    BridgeError::RemotePanic(panic_info).encode().into_raw()
                }
            }
        })
    }

    #[track_caller]
    pub fn execute_safe_result_async<'a, Fut, O, E>(fut: Fut) -> BorrowingFfiFuture<'a, *const u8>
    where
        Fut: std::future::Future<Output = Result<O, E>> + Send + std::panic::UnwindSafe + 'a,
        O: Bridgeable + std::panic::RefUnwindSafe + Send + 'static,
        E: Bridgeable + std::panic::RefUnwindSafe + Send + 'static,
    {
        trace!("execute_safe_async: preparing future");
        let _guard = get_runtime().enter();

        BorrowingFfiFuture::<'a>::new(async move {
            trace!("execute_safe_async: future polling started");
            clear_last_panic();
            let result = std::panic::AssertUnwindSafe(fut).catch_unwind().await;

            match result {
                Ok(result) => {
                    debug!("execute_safe_async: logic completed successfully");
                    let location = Location::caller();
                    serialize_result(&result, location).into_raw()
                }

                Err(_) => {
                    let panic_info = recover_panic_info();
                    trace!(panic = ?panic_info, "execute_safe_async: panic caught, returning error");
                    BridgeError::RemotePanic(panic_info).encode().into_raw()
                }
            }
        })
    }
}
