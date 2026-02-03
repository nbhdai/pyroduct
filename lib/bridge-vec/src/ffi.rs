use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe, PanicHookInfo};
use std::sync::Once;
use tracing::{debug, error, trace};

// Assuming these exist in your crate based on the snippet
use crate::{BridgeVec, Bridgeable, DataStatus}; 

// ============================================================================
// 1. Error Definitions & Data Structures
// ============================================================================

/// Detailed information about a panic captured via the global hook.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FfiPanic {
    pub message: String,
    pub file: String,
    pub line: u32,
    pub column: u32,
}

/// Errors occurring on the remote side of the FFI boundary.
/// This is what is serialized into the BridgeVec when Status is TransportError.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum FfiError {
    /// The remote side panicked while processing the request.
    Panic(FfiPanic),
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
    static LAST_FFI_PANIC: RefCell<Option<FfiPanic>> = RefCell::new(None);
}

static REGISTER_PANIC_HOOK: Once = Once::new();

/// Registers a global panic hook that captures panic details into TLS.
/// This must be called once (usually at library init).
pub fn register_ffi_panic_hook() {
    REGISTER_PANIC_HOOK.call_once(|| {
        debug!("register_ffi_panic_hook: installing global panic hook for FFI boundary");
        let default_hook = panic::take_hook();

        panic::set_hook(Box::new(move |info: &PanicHookInfo| {
            // 1. Extract Message
            let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = info.payload().downcast_ref::<String>() {
                s.clone()
            } else {
                "Panic occurred (unknown payload type)".to_string()
            };

            // 2. Extract Location
            let (file, line, col) = if let Some(loc) = info.location() {
                (loc.file().to_string(), loc.line(), loc.column())
            } else {
                ("unknown".to_string(), 0, 0)
            };

            // 3. Log immediately (crucial for debugging if FFI return fails)
            error!(
                panic.message = %msg,
                panic.file = %file,
                panic.line = line,
                "FFI Panic Hook captured a panic"
            );

            // 4. Store in TLS
            let ffi_panic = FfiPanic {
                message: msg,
                file,
                line,
                column: col,
            };

            LAST_FFI_PANIC.with(|slot| {
                *slot.borrow_mut() = Some(ffi_panic);
            });

            // 5. Call the previous hook (so we don't silence terminal output)
            default_hook(info);
        }));
    });
}

/// Retrieves and clears the last panic from TLS.
fn recover_panic_info() -> FfiPanic {
    trace!("recover_panic_info: attempting to retrieve panic details from TLS");
    LAST_FFI_PANIC.with(|slot| {
        slot.borrow_mut().take().unwrap_or_else(|| {
            error!("recover_panic_info: panic detected but no details found in TLS");
            FfiPanic {
                message: "Panic caught via catch_unwind, but TLS was empty.".to_string(),
                file: "unknown".to_string(),
                line: 0,
                column: 0,
            }
        })
    })
}

/// Clears any stale panic state before starting a new FFI call.
fn clear_last_panic() {
    LAST_FFI_PANIC.with(|slot| *slot.borrow_mut() = None);
}

// ============================================================================
// 3. Execution & Serialization Logic
// ============================================================================

/// Safe entry point for FFI operations.
/// 1. Clears old state.
/// 2. Catches Unwind.
/// 3. Serializes Output or Error.
pub fn execute_safe<F, O>(func: F) -> BridgeVec
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
            serialize_output(&output_obj)
        }
        Err(_) => {
            // catch_unwind gives us a Box<dyn Any>, but we want the rich info
            // we captured in our custom hook via TLS.
            let panic_info = recover_panic_info();
            trace!(panic = ?panic_info, "execute_safe: panic caught, returning error");
            BridgeVec::from_transport_error(&FfiError::Panic(panic_info))
        }
    }
}

/// Helper to serialize a successful result.
fn serialize_output<T>(val: &T) -> BridgeVec
where
    T: Bridgeable,
{
    // We construct a nested catch_unwind here in case *serialization itself* panics.
    let guard = panic::catch_unwind(AssertUnwindSafe(|| {
        val.serialize()
    }));

    match guard {
        Ok(Ok(mut vec)) => {
            // Success path
            vec.set_status(DataStatus::ValidData as u8);
            vec
        }
        Ok(Err(e)) => {
            // Logic worked, but serialization failed
            let err_msg = format!("Failed to serialize result: {:?}", e);
            BridgeVec::from_transport_error(&FfiError::SerializationFailed(err_msg))
        }
        Err(_) => {
            // Serialization panicked
            let panic_info = recover_panic_info();
            BridgeVec::from_transport_error(&FfiError::Panic(panic_info))
        }
    }
}