//! # FFI Boundary Safety & Execution
//!
//! This module provides the "safety firewall" required when exposing Rust functions to foreign code.
//! It guarantees that panics and errors strictly adhere to the `PyroVec` protocol and never
//! unwind across the FFI boundary.
//!
//! ## Core Responsibilities
//!
//! 1.  **Panic Isolation**: Wraps user logic in `catch_unwind`.
//! 2.  **Rich Error Reporting**: Captures diagnostic info into TLS.
//! 3.  **Serialization Safety**: Guards the serialization step itself.
//! 4.  **Boundary Types**: Exclusively uses `PyroVecPtr` for stable ABI passing.

use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe, Location, PanicHookInfo};
use std::sync::Once;
use tracing::{debug, error};

use crate::bridgeable::BridgeableZeroCopy;
use crate::format::{PyroZeroCopyFormat, Receiver};
use crate::{PyroError, PyroVec, PyroVecPtr, Bridgeable, BridgeableResult, CapturedError};

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

// ============================================================================
// Execution & Serialization Logic
// ============================================================================

/// Safe entry point for FFI operations returning a PyroVecPtr.
#[track_caller]
pub fn execute_safe<F, O>(func: F) -> PyroVecPtr
where
    O: Bridgeable + std::panic::RefUnwindSafe,
    F: FnOnce() -> O + std::panic::UnwindSafe,
{
    register_ffi_panic_hook();
    clear_last_panic();

    let result = panic::catch_unwind(func);

    match result {
        Ok(output_obj) => {
            let location = Location::caller();
            serialize_output(&output_obj, location).into_raw()
        }
        Err(_) => {
            let panic_info = recover_panic_info();
            PyroError::CodePanic(panic_info).encode().into_raw()
        }
    }
}

/// Safe entry point for FFI operations returning a Result, mapped to PyroVecPtr.
#[track_caller]
pub fn execute_safe_result<F, O, E>(func: F) -> PyroVecPtr
where
    O: Bridgeable + std::panic::RefUnwindSafe,
    E: Bridgeable + std::panic::RefUnwindSafe,
    F: FnOnce() -> Result<O, E> + std::panic::UnwindSafe,
{
    register_ffi_panic_hook();
    clear_last_panic();

    let result = panic::catch_unwind(func);

    match result {
        Ok(output_obj) => {
            let location = Location::caller();
            serialize_result(&output_obj, location).into_raw()
        }
        Err(_) => {
            let panic_info = recover_panic_info();
            PyroError::CodePanic(panic_info).encode().into_raw()
        }
    }
}

/// Helper to deserialize input from a raw PyroVecPtr.
pub fn deserialize_input<I: Bridgeable + BridgeableZeroCopy>(
    data: PyroVecPtr,
    location: &Location,
) -> Result<I, PyroError>
where
    <I as Bridgeable>::Format: PyroZeroCopyFormat<I>,
{
    let guard = panic::catch_unwind(AssertUnwindSafe(|| {
        let vec = unsafe { PyroVec::from_raw(data) }?;
        let typed = I::expose(vec)?;
        let mut receiver = <I as BridgeableZeroCopy>::receiver();
        receiver.receive(&typed)
    }));

    match guard {
        Ok(input) => input,
        Err(_) => {
            let panic_info = recover_panic_info()
                .with_location(location)
                .with_source("Panic");
            Err(PyroError::deserialization(panic_info))
        }
    }
}

/// Helper to serialize a successful output object.
pub fn serialize_output<T>(val: &T, location: &Location) -> PyroVec
where
    T: Bridgeable,
{
    let guard = panic::catch_unwind(AssertUnwindSafe(|| val.ship()));

    match guard {
        Ok(Ok(vec)) => vec,
        Ok(Err(e)) => e.encode(),
        Err(_) => {
            let panic_info = recover_panic_info()
                .with_location(location)
                .with_source("Panic");
            PyroError::serialization(panic_info).encode()
        }
    }
}

/// Helper to serialize a Result into a PyroVec.
pub fn serialize_result<T, E>(result: &Result<T, E>, location: &Location) -> PyroVec
where
    T: Bridgeable,
    E: Bridgeable,
{
    let guard = panic::catch_unwind(AssertUnwindSafe(|| result.ship()));

    match guard {
        Ok(Ok(vec)) => vec,
        Ok(Err(e)) => e.encode(),
        Err(_) => {
            let panic_info = recover_panic_info()
                .with_location(location)
                .with_source("Panic");
            PyroError::serialization(panic_info).encode()
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use async_ffi_mod::*;

#[cfg(not(target_arch = "wasm32"))]
mod async_ffi_mod {
    use super::*;
    use ::async_ffi::BorrowingFfiFuture;
    use futures::FutureExt;
    use std::sync::OnceLock;
    use tokio::runtime::Runtime;

    static RUNTIME: OnceLock<Runtime> = OnceLock::new();

    pub fn get_runtime() -> &'static Runtime {
        RUNTIME.get_or_init(|| Runtime::new().expect("Failed to create Tokio runtime in plugin"))
    }

    #[track_caller]
    pub fn execute_safe_async<'a, Fut, O>(fut: Fut) -> BorrowingFfiFuture<'a, PyroVecPtr>
    where
        Fut: std::future::Future<Output = O> + Send + std::panic::UnwindSafe + 'a,
        O: Bridgeable + std::panic::RefUnwindSafe + Send + 'static,
    {
        let _guard = get_runtime().enter();

        BorrowingFfiFuture::<'a>::new(async move {
            clear_last_panic();
            let result = std::panic::AssertUnwindSafe(fut).catch_unwind().await;

            match result {
                Ok(val) => {
                    let location = Location::caller();
                    serialize_output(&val, location).into_raw()
                }
                Err(_) => {
                    let panic_info = recover_panic_info();
                    PyroError::CodePanic(panic_info).encode().into_raw()
                }
            }
        })
    }

    #[track_caller]
    pub fn execute_safe_result_async<'a, Fut, O, E>(
        fut: Fut,
    ) -> BorrowingFfiFuture<'a, PyroVecPtr>
    where
        Fut: std::future::Future<Output = Result<O, E>> + Send + std::panic::UnwindSafe + 'a,
        O: Bridgeable + std::panic::RefUnwindSafe + Send + 'static,
        E: Bridgeable + std::panic::RefUnwindSafe + Send + 'static,
    {
        let _guard = get_runtime().enter();

        BorrowingFfiFuture::<'a>::new(async move {
            clear_last_panic();
            let result = std::panic::AssertUnwindSafe(fut).catch_unwind().await;

            match result {
                Ok(result) => {
                    let location = Location::caller();
                    serialize_result(&result, location).into_raw()
                }
                Err(_) => {
                    let panic_info = recover_panic_info();
                    PyroError::CodePanic(panic_info).encode().into_raw()
                }
            }
        })
    }
}
