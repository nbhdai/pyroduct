use crate::{CapIdentity, PyroductResult};
use crate::capability_host::ffi::{
    COutput, ClassDropFn, ClassResetFn, FfiBorrowedFutureObjectResult, FfiBorrowedFutureResult, FfiInitResult, FfiResult
};
use crate::errors::{FfiError, PyroductError};
use crate::host::class::ClassState;
use std::ffi::c_void;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use pin_project::pin_project;
use tracing::{debug, error, trace};

/// Helper to take ownership of the FFI output vector
unsafe fn consume_output(output: COutput) -> Vec<u8> {
    trace!("consume_output: processing raw output pointer");
    if output.ptr.is_null() {
        trace!("consume_output: pointer is null, returning empty vec");
        return Vec::new();
    }
    let vec = unsafe { Vec::from_raw_parts(output.ptr as *mut u8, output.len, output.cap) };
    debug!(
        len = vec.len(),
        "consume_output: took ownership of output vector"
    );
    vec
}

/// Helper to deserialize an FfiError from COutput using rkyv
unsafe fn deserialize_error(output: COutput) -> FfiError {
    trace!("deserialize_error: attempting to deserialize error from plugin");
    let bytes = unsafe { consume_output(output) };

    if bytes.is_empty() {
        error!("deserialize_error: received empty error output");
        return FfiError::DeserializationFailed(
            "Empty error output".into(),
            crate::errors::Phase::Output,
        );
    }

    match rkyv::from_bytes::<FfiError, rkyv::rancor::Error>(&bytes) {
        Ok(ffi_error) => {
            debug!(error = ?ffi_error, "deserialize_error: successfully deserialized FfiError");
            ffi_error
        }
        Err(e) => {
            error!(error = ?e, "deserialize_error: rkyv deserialization failed");
            FfiError::DeserializationFailed(e.to_string(), crate::errors::Phase::Output)
        }
    }
}

// ============================================================================
// Initialization Bridge
// ============================================================================

pub struct InitResultBridge;

impl InitResultBridge {
    pub unsafe fn from_ffi(
        res: FfiInitResult,
        ident: &CapIdentity,
    ) -> Result<*mut c_void, PyroductError> {
        trace!(tag = res.tag, "InitResultBridge: processing FFI result");
        match res.tag {
            0 => {
                debug!("InitResultBridge: initialization successful");
                Ok(res.state)
            }
            1 => {
                debug!("InitResultBridge: initialization failed, deserializing error");
                Err(unsafe { deserialize_error(res.error) }
                    .to_capability_error(ident))
            }
            _ => {
                error!(tag = res.tag, "InitResultBridge: unknown tag received");
                Err(FfiError::UnknownTag(res.tag)
                    .to_capability_error(ident))
            }
        }
    }
}

pub enum AsyncInitState<'a> {
    Ffi(async_ffi::BorrowingFfiFuture<'a, FfiInitResult>),
    Ready(Option<Result<*mut c_void, PyroductError>>),
}

/// Wrapper for the async init future that handles both pending futures and early errors.
pub struct AsyncInitFuture<'a> {
    state: AsyncInitState<'a>,
    ident: CapIdentity,
}

impl<'a> AsyncInitFuture<'a> {
    pub fn new(
        res: FfiBorrowedFutureObjectResult<'a>,
        ident: &CapIdentity,
    ) -> Self {
        let state = match res {
            FfiBorrowedFutureObjectResult::Future(fut) => {
                trace!("AsyncInitFuture: created from Future variant");
                AsyncInitState::Ffi(fut)
            }
            FfiBorrowedFutureObjectResult::EarlyError(val) => {
                trace!("AsyncInitFuture: created from EarlyError variant");
                // Convert the early result immediately
                let result = unsafe {
                    InitResultBridge::from_ffi(
                        val,
                        ident,
                    )
                };
                AsyncInitState::Ready(Some(result))
            }
        };
        Self {
            state,
            ident: ident.clone(),
        }
    }
}

impl<'a> Future for AsyncInitFuture<'a> {
    type Output = Result<*mut c_void, PyroductError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // We need to move these out to avoid borrow checker issues when calling from_ffi
        let ident = self.ident.clone();

        match &mut self.state {
            AsyncInitState::Ffi(fut) => match Pin::new(fut).poll(cx) {
                Poll::Ready(res) => {
                    trace!("AsyncInitFuture: underlying future ready");
                    Poll::Ready(unsafe { InitResultBridge::from_ffi(res, &ident) })
                }
                Poll::Pending => Poll::Pending,
            },
            AsyncInitState::Ready(res) => {
                trace!("AsyncInitFuture: returning ready result");
                Poll::Ready(res.take().unwrap_or_else(|| {
                    error!("AsyncInitFuture: polled after completion");
                    Err(FfiError::FuturePolledAfterCompletion.to_capability_error(&ident))
                }))
            }
        }
    }
}

// ============================================================================
// Function Call Bridge (Execution)
// ============================================================================

pub struct ExecutionResultBridge;

impl ExecutionResultBridge {
    pub unsafe fn from_ffi(
        res: FfiResult,
        ident: &CapIdentity,
    ) -> Result<Vec<u8>, PyroductError> {
        trace!(
            tag = res.tag,
            "ExecutionResultBridge: processing FFI result"
        );
        match res.tag {
            0 => {
                debug!("ExecutionResultBridge: execution successful");
                Ok(unsafe { consume_output(res.output) })
            }
            1 | 2 => {
                debug!("ExecutionResultBridge: execution failed (error)");
                Err(unsafe { deserialize_error(res.output) }
                    .to_capability_error(ident))
            }
            _ => {
                error!(tag = res.tag, "ExecutionResultBridge: unknown tag received");
                Err(FfiError::UnknownTag(res.tag)
                    .to_capability_error(ident))
            }
        }
    }

    pub unsafe fn expected_null_from_ffi(
        res: FfiResult,
        ident: &CapIdentity,
    ) -> Result<(), PyroductError> {
        trace!(
            tag = res.tag,
            "ExecutionResultBridge: processing void FFI result"
        );
        match res.tag {
            0 => {
                if !res.output.ptr.is_null() {
                    error!(
                        "ExecutionResultBridge: Reset not returning a null pointer for the Ok, Ignoring"
                    );
                }
                Ok(())
            }
            1 | 2 => Err(unsafe { deserialize_error(res.output) }
                .to_capability_error(ident)),
            _ => {
                error!(tag = res.tag, "ExecutionResultBridge: unknown tag received");
                Err(FfiError::UnknownTag(res.tag)
                    .to_capability_error(ident))
            }
        }
    }
}

pub enum AsyncExecState<'a> {
    Ffi(async_ffi::BorrowingFfiFuture<'a, FfiResult>),
    Ready(Option<Result<Vec<u8>, PyroductError>>),
}

/// Wrapper for the async execution future that handles both pending futures and early errors.
pub struct AsyncExecFuture<'a> {
    state: AsyncExecState<'a>,
    ident: CapIdentity,
}

impl<'a> AsyncExecFuture<'a> {
    pub fn new(
        res: FfiBorrowedFutureResult<'a>,
        ident: &CapIdentity,
    ) -> Self {
        let state = match res {
            FfiBorrowedFutureResult::Future(fut) => {
                trace!("AsyncExecFuture: created from Future variant");
                AsyncExecState::Ffi(fut)
            }
            FfiBorrowedFutureResult::EarlyError(val) => {
                trace!("AsyncExecFuture: created from EarlyError variant");
                // Convert the early result immediately
                let result = unsafe {
                    ExecutionResultBridge::from_ffi(
                        val,
                        ident,
                    )
                };
                AsyncExecState::Ready(Some(result))
            }
        };
        Self {
            state,
            ident: ident.clone(),
        }
    }
}

impl<'a> Future for AsyncExecFuture<'a> {
    type Output = Result<Vec<u8>, PyroductError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let ident = self.ident.clone();

        match &mut self.state {
            AsyncExecState::Ffi(fut) => match Pin::new(fut).poll(cx) {
                Poll::Ready(res) => {
                    trace!("AsyncExecFuture: underlying future ready");
                    Poll::Ready(unsafe { ExecutionResultBridge::from_ffi(res, &ident) })
                }
                Poll::Pending => Poll::Pending,
            },
            AsyncExecState::Ready(res) => {
                trace!("AsyncExecFuture: returning ready result");
                Poll::Ready(res.take().unwrap_or_else(|| {
                    error!("AsyncExecFuture: polled after completion");
                    Err(FfiError::FuturePolledAfterCompletion.to_capability_error(&ident))
                }))
            }
        }
    }
}



#[pin_project(project = CapInit)]
pub enum CapabilityInit<'a> {
    Sync {
        reset_fn: ClassResetFn<'static>,
        state: Option<*mut c_void>,
        destroy_fn: ClassDropFn,
    },
    Async {
        reset_fn: ClassResetFn<'static>,
        config_bytes: Vec<u8>,
        #[pin]
        future: AsyncInitFuture<'a>,
        destroy_fn: ClassDropFn,
    },
    Null,
}

impl<'a> Future for CapabilityInit<'a> {
    type Output = PyroductResult<ClassState>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.project() {
            CapInit::Sync { reset_fn, state, destroy_fn } => match state.take() {
                Some(state) => std::task::Poll::Ready(Ok(ClassState {
                    reset_fn: reset_fn.clone(),
                    ptr: state,
                    destroy_fn: *destroy_fn,
                })),
                None => panic!("Double await!"),
            },
            CapInit::Async {
                reset_fn,
                config_bytes: _,
                future,
                destroy_fn,
            } => match future.poll(cx) {
                std::task::Poll::Ready(result) => match result {
                    Ok(pointer) => std::task::Poll::Ready(Ok(ClassState {
                        reset_fn: reset_fn.clone(),
                        ptr: pointer,
                        destroy_fn: *destroy_fn,
                    })),
                    Err(e) => std::task::Poll::Ready(Err(e)),
                },
                std::task::Poll::Pending => std::task::Poll::Pending,
            },
            CapInit::Null => std::task::Poll::Ready(Ok(ClassState {
                reset_fn: ClassResetFn::Null,
                ptr: std::ptr::null_mut(),
                destroy_fn: ClassDropFn::Null,
            })),
        }
    }
}

#[pin_project(project = CapReset)]
pub enum CapabilityReset<'a> {
    Async(#[pin] AsyncExecFuture<'a>),
    SyncOrNull(CapIdentity, Option<PyroductResult<()>>),
}

impl<'a> Future for CapabilityReset<'a> {
    type Output = PyroductResult<()>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.project() {
            CapReset::Async(this) => match this.poll(cx) {
                std::task::Poll::Ready(Ok(_)) => std::task::Poll::Ready(Ok(())),
                std::task::Poll::Ready(Err(e)) => std::task::Poll::Ready(Err(e)),
                std::task::Poll::Pending => std::task::Poll::Pending,
            },
            CapReset::SyncOrNull(ident, result) => {
                match result.take() {
                    Some(result) => std::task::Poll::Ready(result),
                    None => std::task::Poll::Ready(Err(FfiError::FuturePolledAfterCompletion
                        .to_capability_error(ident))),
                }
            }
        }
    }
}