use std::{
    ffi::c_void,
    panic::{self, AssertUnwindSafe},
};

use async_ffi::BorrowingFfiFuture;
use futures::FutureExt;
use tracing::{debug, error, trace};

use super::{safe_async::get_runtime, safe_io::make_error_output};
use crate::module_capability::panic::{clear_last_panic, recover_panic_info};
use crate::{
    capability_host::ffi::{
        FfiBorrowedFutureObjectResult, FfiBorrowedFutureResult, FfiInitResult, FfiResult,
    },
    errors::{FfiError, Phase},
};

// --- Sync Wrappers ---

#[tracing::instrument(skip(init_fn))]
pub unsafe fn execute_safe_init<C, S, F>(
    config_ptr: *const u8,
    config_len: usize,
    init_fn: F,
) -> FfiInitResult
where
    C: serde::Deserialize<'static>,
    S: 'static,
    F: FnOnce(C) -> S + panic::UnwindSafe,
{
    trace!("execute_safe_init: entering");
    if config_ptr.is_null() {
        return FfiInitResult::err(make_error_output(FfiError::NullPointer(Phase::Init)).output);
    }
    if config_len == 0 {
        return FfiInitResult::err(make_error_output(FfiError::ZeroLength(Phase::Init)).output);
    }
    let config_bytes = unsafe { std::slice::from_raw_parts(config_ptr, config_len) };

    let config = match serde_json::from_slice::<C>(config_bytes) {
        Ok(config) => config,
        Err(error) => {
            let ffi_error = FfiError::DeserializationFailed(error.to_string(), Phase::Init);
            return FfiInitResult::err(make_error_output(ffi_error).output);
        }
    };

    clear_last_panic();

    trace!("execute_safe_init: executing init_fn");
    let result = panic::catch_unwind(AssertUnwindSafe(|| init_fn(config)));

    match result {
        Ok(state) => {
            debug!("execute_safe_init: state created successfully");
            FfiInitResult::ok(Box::into_raw(Box::new(state)) as *mut c_void)
        }
        Err(_) => {
            let panic_info = recover_panic_info();
            error!(panic = ?panic_info, "execute_safe_init: panic caught during initialization");
            FfiInitResult::err(
                make_error_output(FfiError::CapabilityLogicPanicked(panic_info)).output,
            )
        }
    }
}

#[tracing::instrument(skip(reset_fn))]
pub unsafe fn execute_safe_reset<S, F>(state_ptr: *mut c_void, reset_fn: F) -> FfiResult
where
    S: 'static,
    F: FnOnce(&mut S) + panic::UnwindSafe,
{
    trace!("execute_safe_reset: entering");

    if state_ptr.is_null() {
        error!("execute_safe_reset: state pointer is null");
        return FfiError::NullPointer(Phase::State).into();
    }

    let state = unsafe { &mut *(state_ptr as *mut S) };

    clear_last_panic();

    trace!("execute_safe_reset: executing reset_fn");
    let result = panic::catch_unwind(AssertUnwindSafe(|| reset_fn(state)));

    match result {
        Ok(_) => {
            debug!("execute_safe_reset: state reset successfully");
            FfiResult::ok_null()
        }
        Err(_) => {
            let panic_info = recover_panic_info();
            error!(panic = ?panic_info, "execute_safe_reset: panic caught during reset");
            make_error_output(FfiError::CapabilityLogicPanicked(panic_info))
        }
    }
}

// --- Async Wrappers ---

/// Safe wrapper for Async Init functions.
/// Returns FfiBorrowedFutureObjectResult.
#[tracing::instrument(skip(init_fn))]
pub unsafe fn execute_safe_async_init<'a, C, S, Fut, F>(
    config_ptr: *const u8,
    config_len: usize,
    init_fn: F,
) -> FfiBorrowedFutureObjectResult<'a>
where
    C: serde::Deserialize<'static> + Send + 'static,
    S: Send + 'static,
    Fut: std::future::Future<Output = S> + Send + 'a,
    F: FnOnce(C) -> Fut + Send + 'a,
{
    trace!("execute_safe_async_init: entering");

    if config_ptr.is_null() {
        return FfiBorrowedFutureObjectResult::EarlyError(FfiInitResult::err(
            make_error_output(FfiError::NullPointer(Phase::Init)).output,
        ));
    }
    if config_len == 0 {
        return FfiBorrowedFutureObjectResult::EarlyError(FfiInitResult::err(
            make_error_output(FfiError::ZeroLength(Phase::Init)).output,
        ));
    }
    let config_bytes = unsafe { std::slice::from_raw_parts(config_ptr, config_len) };

    let config = match serde_json::from_slice::<C>(config_bytes) {
        Ok(config) => config,
        Err(error) => {
            let ffi_error = FfiError::DeserializationFailed(error.to_string(), Phase::Init);
            return FfiBorrowedFutureObjectResult::EarlyError(FfiInitResult::err(
                make_error_output(ffi_error).output,
            ));
        }
    };

    // 2. Return Borrowing Future
    let _guard = get_runtime().enter();
    FfiBorrowedFutureObjectResult::Future(BorrowingFfiFuture::<'a>::new(async move {
        trace!("execute_safe_async_init: future polling started");
        clear_last_panic();

        // Note: We move the deserialized config into the future here
        let result = AssertUnwindSafe(init_fn(config)).catch_unwind().await;

        match result {
            Ok(state) => {
                debug!("execute_safe_async_init: async init completed successfully");
                FfiInitResult::ok(Box::into_raw(Box::new(state)) as *mut c_void)
            }
            Err(_) => {
                let panic_info = recover_panic_info();
                error!(panic = ?panic_info, "execute_safe_async_init: panic caught during async init");
                FfiInitResult::err(
                    make_error_output(FfiError::CapabilityLogicPanicked(panic_info)).output,
                )
            }
        }
    }))
}

/// Safe wrapper for Async Reset functions.
/// Returns FfiBorrowedFutureResult (which wraps FfiResult).
#[tracing::instrument(skip(reset_fn))]
pub unsafe fn execute_safe_async_reset<'a, S, Fut, F>(
    state_ptr: *mut c_void,
    reset_fn: F,
) -> FfiBorrowedFutureResult<'a>
where
    S: Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'a,
    F: FnOnce(&mut S) -> Fut + Send + 'a,
{
    trace!("execute_safe_async_reset: entering");

    if state_ptr.is_null() {
        error!("execute_safe_async_reset: state pointer is null");
        return FfiBorrowedFutureResult::EarlyError(FfiError::NullPointer(Phase::State).into());
    }

    let state = unsafe { &mut *(state_ptr as *mut S) };

    let _guard = get_runtime().enter();
    FfiBorrowedFutureResult::Future(BorrowingFfiFuture::<'a>::new(async move {
        trace!("execute_safe_async_reset: future polling started");
        clear_last_panic();

        let result = AssertUnwindSafe(reset_fn(state)).catch_unwind().await;

        match result {
            Ok(_) => {
                debug!("execute_safe_async_reset: async reset completed successfully");
                FfiResult::ok_null()
            }
            Err(_) => {
                let panic_info = recover_panic_info();
                error!(panic = ?panic_info, "execute_safe_async_reset: panic caught during async reset");
                make_error_output(FfiError::CapabilityLogicPanicked(panic_info))
            }
        }
    }))
}
