use std::panic;
use std::{cell::RefCell, panic::PanicHookInfo, sync::Once};
use tracing::{debug, error, trace};

use crate::errors::FfiPanic;

// Thread Local Storage to temporarily hold the panic info for the current thread
thread_local! {
    static LAST_FFI_PANIC: RefCell<Option<FfiPanic>> = RefCell::new(None);
}

pub static REGISTER_PANIC_HOOK: Once = Once::new();

pub fn register_ffi_panic_hook() {
    REGISTER_PANIC_HOOK.call_once(|| {
        debug!("register_ffi_panic_hook: installing global panic hook for FFI boundary");
        let default_hook = panic::take_hook();

        panic::set_hook(Box::new(move |info: &PanicHookInfo| {
            let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = info.payload().downcast_ref::<String>() {
                s.clone()
            } else {
                "Panic occurred (unknown payload type)".to_string()
            };

            let (file, line, col) = if let Some(loc) = info.location() {
                (loc.file().to_string(), loc.line(), loc.column())
            } else {
                ("unknown".to_string(), 0, 0)
            };

            // Log the panic immediately so it appears in logs even if FFI recovery fails
            error!(
                panic.message = %msg,
                panic.file = %file,
                panic.line = line,
                panic.col = col,
                "FFI Panic Hook captured a panic"
            );

            let ffi_panic = FfiPanic {
                message: msg,
                file,
                line,
                column: col,
            };

            LAST_FFI_PANIC.with(|slot| {
                *slot.borrow_mut() = Some(ffi_panic);
            });

            default_hook(info);
        }));
    });
}

pub fn recover_panic_info() -> FfiPanic {
    trace!("recover_panic_info: attempting to retrieve panic details from TLS");
    LAST_FFI_PANIC.with(|slot| {
        slot.borrow_mut().take().unwrap_or_else(|| {
            let p = FfiPanic {
                message: "Panic caught, but hook failed to capture details".to_string(),
                file: "unknown".to_string(),
                line: 0,
                column: 0,
            };
            error!("recover_panic_info: failed to find panic details in TLS (hook might not have run or TLS was cleared)");
            p
        })
    })
}

pub fn clear_last_panic() {
    LAST_FFI_PANIC.with(|slot| *slot.borrow_mut() = None);
}
