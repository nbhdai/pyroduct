use std::panic::{self, AssertUnwindSafe};

use async_ffi::BorrowingFfiFuture;
use futures::FutureExt;
use tracing::{debug, error, trace};

use crate::ffi::{FutureInitResult, InitResult};
use crate::ffi::guest::panic_wrap::get_runtime;
use crate::header::{DataStatus, PyroHeader};
use crate::panic::{clear_last_panic, recover_panic_info, register_ffi_panic_hook};
use crate::view::{PyroView, PyroViewPtr};
use crate::{CapturedError, PyroError};

/// For `new` that doesn't have a config.
#[derive(serde::Deserialize)]
pub struct EmptyConfig {}


/// Deserialize config from a `PyroViewPtr` as `Option<C>`.
///
/// Returns `None` if the view is empty/null, otherwise deserializes the JSON payload.
pub fn deserialize_config<C: serde::de::DeserializeOwned>(
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

/// Safe wrapper for Sync Init functions.
#[track_caller]
#[tracing::instrument(skip(init_fn))]
pub fn execute_safe_init<F>(
    init_fn: F,
) -> InitResult
where
    F: FnOnce() -> InitResult,
{
    trace!("execute_safe_init: entering");
    register_ffi_panic_hook();
    clear_last_panic();

    trace!("execute_safe_init: executing init_fn");
    let result = panic::catch_unwind(AssertUnwindSafe(|| init_fn()));

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

// --- Async Wrappers ---
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::task::JoinHandle;

pub struct SafeInitHandle {
    handle: JoinHandle<InitResult>,
}

impl SafeInitHandle {
    pub fn new(handle: JoinHandle<InitResult>) -> Self {
        Self {
            handle,
        }
    }
}

impl Future for SafeInitHandle
{
    type Output = InitResult;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Poll the underlying tokio JoinHandle
        match Pin::new(&mut self.handle).poll(cx) {
            Poll::Ready(Ok(val)) => Poll::Ready(val),
            Poll::Ready(Err(join_error)) => {
                Poll::Ready(InitResult::init_err(PyroError::CodePanic(CapturedError::new(join_error).into())))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}


/// Safe wrapper for Async Init functions.
/// Returns `FutureInitResult`.
pub fn execute_safe_async_init<Fut, F>(
    init_fn: F,
) -> FutureInitResult
where
    Fut: std::future::Future<Output = InitResult> + Send + 'static,
    F: FnOnce() -> Fut + Send,
{
    let fut = init_fn();
    FutureInitResult::Future(BorrowingFfiFuture::<'static>::new(SafeInitHandle::new(get_runtime().spawn(async move {
        trace!("execute_safe_async_init: future polling started");
        clear_last_panic();

        let result = AssertUnwindSafe(fut).catch_unwind().await;

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
    }))))
}