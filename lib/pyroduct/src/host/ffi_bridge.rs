use crate::CapIdentity;
use crate::capability_host::ffi::{
    COutput, FfiBorrowedFutureResult, FfiInitResult, FfiResult
};
use crate::errors::{FfiError, PyroductError};
use std::{ffi::c_void, future::Future, pin::Pin, task::{Context, Poll}};
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
