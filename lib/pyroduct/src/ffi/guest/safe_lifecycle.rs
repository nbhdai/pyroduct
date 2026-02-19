use std::panic::{self, AssertUnwindSafe};

use async_ffi::BorrowingFfiFuture;
use futures::FutureExt;
use tracing::{debug, error, trace};

use crate::ffi::{FutureInitResult, FuturePyroVec, InitResult, PyroRefObjectPtr};
use crate::ffi::guest::panic_wrap::get_runtime;
use crate::header::{DataStatus, PyroHeader};
use crate::panic::{clear_last_panic, recover_panic_info, register_ffi_panic_hook};
use crate::view::{PyroView, PyroViewPtr};
use crate::{CapturedError, PyroError, PyroVec, PyroVecPtr};

/// For `new` that doesn't have a config.
#[derive(serde::Deserialize)]
pub struct EmptyConfig {}



/// Deserialize config from a `PyroViewPtr` as `Option<C>`.
///
/// Returns `None` if the view is empty/null, otherwise deserializes the JSON payload.
fn deserialize_config<C: serde::de::DeserializeOwned>(
    config: PyroViewPtr,
) -> Result<Option<C>, PyroError> {
    if config.ptr.is_null() || config.len == 0 {
        return Ok(None);
    }


    let view = unsafe { PyroView::from_ptr(config) }?;
    if let Ok(DataStatus::Empty) = view.status() {
        return Ok(None);
    }
    let slice: &[u8] = &*view;

    serde_json::from_slice::<Option<C>>(slice).map_err(|err| {
        let error = CapturedError::new(format!("Json Serialization: {err}"))
            .with_location(std::panic::Location::caller())
            .with_backtrace(std::backtrace::Backtrace::capture());
        PyroError::serialization(error)
    })
}

// --- Sync Wrappers ---

/// Safe wrapper for Sync Init functions.
/// The closure receives `Option<C>` directly.
#[track_caller]
#[tracing::instrument(skip(init_fn))]
pub unsafe fn execute_safe_init<C, S, F>(
    config: PyroViewPtr,
    init_fn: F,
) -> InitResult
where
    C: serde::de::DeserializeOwned,
    S: 'static,
    F: FnOnce(Option<C>) -> S,
{
    trace!("execute_safe_init: entering");
    register_ffi_panic_hook();

    let config: Option<C> = match deserialize_config(config) {
        Ok(c) => c,
        Err(err) => return InitResult::init_err(err),
    };

    clear_last_panic();

    trace!("execute_safe_init: executing init_fn");
    let result = panic::catch_unwind(AssertUnwindSafe(|| init_fn(config)));

    match result {
        Ok(state) => {
            debug!("execute_safe_init: state created successfully");
            InitResult::init_ok(state)
        }
        Err(_) => {
            let panic_info = recover_panic_info();
            error!(panic = ?panic_info, "execute_safe_init: panic caught during initialization");
            InitResult::init_err(PyroError::CodePanic(panic_info))
        }
    }
}

/// Safe wrapper for Sync Reset functions.
pub unsafe fn execute_safe_reset<S, F>(
    state: PyroRefObjectPtr,
    reset_fn: F,
) -> PyroVecPtr
where
    S: 'static,
    F: FnOnce(&mut S),
{
    trace!("execute_safe_reset: entering");
    register_ffi_panic_hook();

    if state.state.is_null() {
        error!("execute_safe_reset: state pointer is null");
        return PyroError::null_pointer().encode().into_raw();
    }

    let s = unsafe { &mut *(state.state as *mut S) };

    clear_last_panic();

    trace!("execute_safe_reset: executing reset_fn");
    let result = panic::catch_unwind(AssertUnwindSafe(|| reset_fn(s)));

    match result {
        Ok(_) => {
            debug!("execute_safe_reset: state reset successfully");
            PyroVec::ok().into_raw()
        }
        Err(_) => {
            let panic_info = recover_panic_info();
            error!(panic = ?panic_info, "execute_safe_reset: panic caught during reset");
            PyroError::CodePanic(panic_info).encode().into_raw()
        }
    }
}

// --- Async Wrappers ---

/// Safe wrapper for Async Init functions.
/// Returns `FutureInitResult`.
/// The closure receives `Option<C>` directly.
#[tracing::instrument(skip(init_fn))]
pub unsafe fn execute_safe_async_init<'a, C, S, Fut, F>(
    config: PyroViewPtr,
    init_fn: F,
) -> FutureInitResult<'a>
where
    C: serde::de::DeserializeOwned + Send + 'static,
    S: Send + 'static,
    Fut: std::future::Future<Output = S> + Send + 'a,
    F: FnOnce(Option<C>) -> Fut + Send + 'a,
{
    trace!("execute_safe_async_init: entering");
    register_ffi_panic_hook();

    let config: Option<C> = match deserialize_config(config) {
        Ok(c) => c,
        Err(err) => return FutureInitResult::EarlyError(InitResult::init_err(err)),
    };

    let _guard = get_runtime().enter();
    FutureInitResult::Future(BorrowingFfiFuture::<'a>::new(async move {
        trace!("execute_safe_async_init: future polling started");
        clear_last_panic();

        let result = AssertUnwindSafe(init_fn(config)).catch_unwind().await;

        match result {
            Ok(state) => {
                debug!("execute_safe_async_init: async init completed successfully");
                InitResult::init_ok(state)
            }
            Err(_) => {
                let panic_info = recover_panic_info();
                error!(panic = ?panic_info, "execute_safe_async_init: panic caught during async init");
                InitResult::init_err(PyroError::CodePanic(panic_info))
            }
        }
    }))
}

/// Safe wrapper for Async Reset functions.
/// Returns `FuturePyroVec`.
pub unsafe fn execute_safe_async_reset<'a, S, Fut, F>(
    state: PyroRefObjectPtr,
    reset_fn: F,
) -> FuturePyroVec<'a>
where
    S: Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'a,
    F: FnOnce(&'a mut S) -> Fut + Send + 'a,
{
    trace!("execute_safe_async_reset: entering");
    register_ffi_panic_hook();

    if state.state.is_null() {
        error!("execute_safe_async_reset: state pointer is null");
        return FuturePyroVec::from(PyroError::null_pointer().encode());
    }

    let s = unsafe { &mut *(state.state as *mut S) };

    let _guard = get_runtime().enter();
    FuturePyroVec::Future(BorrowingFfiFuture::<'a>::new(async move {
        trace!("execute_safe_async_reset: future polling started");
        clear_last_panic();

        let result = AssertUnwindSafe(reset_fn(s)).catch_unwind().await;

        match result {
            Ok(_) => {
                debug!("execute_safe_async_reset: async reset completed successfully");
                PyroVec::ok().into_raw()
            }
            Err(_) => {
                let panic_info = recover_panic_info();
                error!(panic = ?panic_info, "execute_safe_async_reset: panic caught during async reset");
                PyroError::CodePanic(panic_info).encode().into_raw()
            }
        }
    }))
}