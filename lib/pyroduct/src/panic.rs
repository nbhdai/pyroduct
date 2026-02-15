use crate::CapturedError;
use std::cell::RefCell;
use std::panic::{self, PanicHookInfo};
use std::sync::Once;
use tracing::{debug, error};

thread_local! {
    static LAST_FFI_PANIC: RefCell<Option<Box<CapturedError>>> = RefCell::new(None);
}

static REGISTER_PANIC_HOOK: Once = Once::new();

pub fn register_ffi_panic_hook() {
    REGISTER_PANIC_HOOK.call_once(|| {
        debug!("register_ffi_panic_hook: installing global panic hook for FFI boundary");
        let default_hook = panic::take_hook();

        panic::set_hook(Box::new(move |info: &PanicHookInfo| {
            let mut error = if let Some(s) = info.payload().downcast_ref::<&str>() {
                CapturedError::new(*s)
            } else if let Some(s) = info.payload().downcast_ref::<String>() {
                CapturedError::new(s)
            } else {
                CapturedError::new("Panic occurred (unknown payload type)")
            };

            if let Some(loc) = info.location() {
                error = error.with_location(loc);
            };

            error = error.with_backtrace(std::backtrace::Backtrace::capture());

            error!(?error, "FFI Panic Hook captured a panic");
            LAST_FFI_PANIC.with(|slot| {
                *slot.borrow_mut() = Some(Box::new(error));
            });

            default_hook(info);
        }));
    });
}

pub fn recover_panic_info() -> Box<CapturedError> {
    LAST_FFI_PANIC.with(|slot| {
        slot.borrow_mut().take().unwrap_or_else(|| {
            error!("recover_panic_info: panic detected but no details found in TLS");
            Box::new(CapturedError::new(
                "Panic caught via catch_unwind, but TLS was empty.",
            ))
        })
    })
}

pub fn clear_last_panic() {
    LAST_FFI_PANIC.with(|slot| *slot.borrow_mut() = None);
}
