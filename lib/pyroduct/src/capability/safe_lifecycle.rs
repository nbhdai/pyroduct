use std::{
    ffi::c_void,
    panic::{self, AssertUnwindSafe},
};

use async_ffi::BorrowingFfiFuture;
use bridge_vec::{BridgeError, BridgeVec, CapturedError, ffi::{clear_last_panic, get_runtime, recover_panic_info}};
use futures::FutureExt;
use tracing::{debug, error, trace};

use crate::{
    capability_host::ffi::{
        FfiBorrowedFutureObjectResult, FfiBorrowedFutureResult, FfiInitResult,
    },
};

/// For new that doesn't have a config.
#[derive(serde::Deserialize)]
pub struct EmptyConfig {}

// --- Sync Wrappers ---

/// Safe wrapper for Sync Init functions.
/// The closure receives `Option<C>` directly.
#[track_caller]
#[tracing::instrument(skip(init_fn))]
pub unsafe fn execute_safe_init<C, S, F>(
    config_ptr: *const u8,
    config_len: usize,
    init_fn: F,
) -> FfiInitResult
where
    C: serde::de::DeserializeOwned,
    S: 'static,
    F: FnOnce(Option<C>) -> S + panic::UnwindSafe,
{
    trace!("execute_safe_init: entering");

    // Deserialize as Option<C> - the JSON can be null or the actual config
    let config: Option<C> = if config_ptr.is_null() || config_len == 0 {
        None
    } else {
        let config_bytes = unsafe { std::slice::from_raw_parts(config_ptr, config_len) };
        match serde_json::from_slice::<Option<C>>(config_bytes) {
            Ok(config) => config,
            Err(err) => {
                let error = CapturedError::new(err).with_location(location)
                
                return BridgeVec::from_transport_error(&detailed_err).into();
            }
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
            BridgeError::RemoteError(panic_info).into()
        }
    }
}

#[tracing::instrument(skip(reset_fn))]
pub unsafe fn execute_safe_reset<S, F>(state_ptr: *mut c_void, reset_fn: F) -> *const u8
where
    S: 'static,
    F: FnOnce(&mut S) + panic::UnwindSafe,
{
    trace!("execute_safe_reset: entering");

    if state_ptr.is_null() {
        error!("execute_safe_reset: state pointer is null");
        return BridgeError::NullPointer.to_vec().into_raw();
    }

    let state = unsafe { &mut *(state_ptr as *mut S) };

    clear_last_panic();

    trace!("execute_safe_reset: executing reset_fn");
    let result = panic::catch_unwind(AssertUnwindSafe(|| reset_fn(state)));

    match result {
        Ok(_) => {
            debug!("execute_safe_reset: state reset successfully");
            BridgeVec::ok().into_raw()
        }
        Err(_) => {
            let panic_info = recover_panic_info();
            error!(panic = ?panic_info, "execute_safe_reset: panic caught during reset");
            BridgeError::RemoteError(panic_info).to_vec().into_raw()
        }
    }
}

// --- Async Wrappers ---

/// Safe wrapper for Async Init functions.
/// Returns FfiBorrowedFutureObjectResult.
/// The closure receives `Option<C>` directly.
#[tracing::instrument(skip(init_fn))]
pub unsafe fn execute_safe_async_init<'a, C, S, Fut, F>(
    config_ptr: *const u8,
    config_len: usize,
    init_fn: F,
) -> FfiBorrowedFutureObjectResult<'a>
where
    C: serde::de::DeserializeOwned + Send + 'static,
    S: Send + 'static,
    Fut: std::future::Future<Output = S> + Send + 'a,
    F: FnOnce(Option<C>) -> Fut + Send + 'a,
{
    trace!("execute_safe_async_init: entering");

    // Deserialize as Option<C> - the JSON can be null or the actual config
    let config: Option<C> = if config_ptr.is_null() || config_len == 0 {
        None
    } else {
        let config_bytes = unsafe { std::slice::from_raw_parts(config_ptr, config_len) };
        match serde_json::from_slice::<Option<C>>(config_bytes) {
            Ok(config) => config,
            Err(err) => {
                let location = std::panic::Location::caller();
                let error = Some(err.to_string());
                // Capture stack trace to pinpoint where the conversion happened
                let backtrace = std::backtrace::Backtrace::capture();
                let cause = match backtrace.status() {
                    std::backtrace::BacktraceStatus::Captured => Some(backtrace.to_string()),
                    _ => None,
                };

                let detailed_err = CapturedError {
                    message: err.to_string(),
                    file: location.file().to_string(),
                    line: location.line(),
                    column: location.column(),
                    error,
                    cause,
                };
                
                return BridgeVec::from_transport_error(&detailed_err).into();
            }
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
                BridgeError::RemoteError(panic_info).into()
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
    F: FnOnce(&'a mut S) -> Fut + Send + 'a,
{
    trace!("execute_safe_async_reset: entering");

    if state_ptr.is_null() {
        error!("execute_safe_reset: state pointer is null");
        return BridgeError::NullPointer.into();
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
                BridgeVec::ok().into_raw()
            }
            Err(_) => {
                let panic_info = recover_panic_info();
                error!(panic = ?panic_info, "execute_safe_async_reset: panic caught during async reset");
                BridgeError::RemoteError(panic_info).to_vec().into_raw()
            }
        }
    }))
}