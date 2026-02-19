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

use ::async_ffi::BorrowingFfiFuture;
use futures::FutureExt;
use std::panic::{self, AssertUnwindSafe, Location};
use std::sync::OnceLock;
use tokio::runtime::Runtime;

use crate::bridgeable::BridgeableZeroCopy;
use crate::ffi::FuturePyroVec;
use crate::format::{PyroZeroCopyFormat, Receiver};
use crate::panic::{clear_last_panic, recover_panic_info, register_ffi_panic_hook};
use crate::{Bridgeable, BridgeableResult, PyroError, PyroVec, PyroVecPtr, PyroView, PyroViewPtr};

// ============================================================================
// Execution & Serialization Logic
// ============================================================================

/// Safe entry point for FFI operations returning a PyroVecPtr.
#[track_caller]
pub fn execute_safe<F, O>(func: F) -> PyroVecPtr
where
    O: Bridgeable,
    F: FnOnce() -> O,
{
    register_ffi_panic_hook();
    clear_last_panic();

    let result = panic::catch_unwind(AssertUnwindSafe(func));

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
    O: Bridgeable,
    E: Bridgeable,
    F: FnOnce() -> Result<O, E>,
{
    register_ffi_panic_hook();
    clear_last_panic();

    let result = panic::catch_unwind(AssertUnwindSafe(func));

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
    data: PyroViewPtr,
    location: &Location,
) -> Result<I, PyroError>
where
    <I as Bridgeable>::Format: PyroZeroCopyFormat<I>,
{
    let guard = panic::catch_unwind(AssertUnwindSafe(|| {
        let vec = unsafe { PyroView::from_ptr(data) }?;
        let typed = I::expose_view(vec)?;
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

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

pub fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| Runtime::new().expect("Failed to create Tokio runtime in plugin"))
}

#[track_caller]
pub fn execute_safe_async<'a, Fut, O>(fut: Fut) -> FuturePyroVec<'a>
where
    Fut: std::future::Future<Output = O> + Send + 'a,
    O: Bridgeable + Send + 'static,
{
    let _guard = get_runtime().enter();

    FuturePyroVec::Future(BorrowingFfiFuture::<'a>::new(async move {
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
    }))
}

#[track_caller]
pub fn execute_safe_result_async<'a, Fut, O, E>(fut: Fut) -> FuturePyroVec<'a>
where
    Fut: std::future::Future<Output = Result<O, E>> + Send + 'a,
    O: Bridgeable + Send + 'static,
    E: Bridgeable + Send + 'static,
{
    let _guard = get_runtime().enter();

    FuturePyroVec::Future(BorrowingFfiFuture::<'a>::new(async move {
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
    }))
}
