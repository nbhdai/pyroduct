use crate::CapIdentity;
use crate::capability_host::ffi::{FfiBorrowedFutureResult, FfiInitResult};
use crate::errors::{FfiError, PyroductError};
use std::{
    ffi::c_void,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use bridge_vec::BridgeVec;
use tracing::{debug, error, trace};

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
                Err(unsafe { deserialize_error(res.error) }.to_capability_error(ident))
            }
            _ => {
                error!(tag = res.tag, "InitResultBridge: unknown tag received");
                Err(FfiError::UnknownTag(res.tag).to_capability_error(ident))
            }
        }
    }
}

pub struct ExecutionResultBridge;

impl ExecutionResultBridge {
    pub unsafe fn from_ffi(res: *const u8, ident: &CapIdentity) -> Result<BridgeVec, PyroductError> {
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
                Err(unsafe { deserialize_error(res.output) }.to_capability_error(ident))
            }
            _ => {
                error!(tag = res.tag, "ExecutionResultBridge: unknown tag received");
                Err(FfiError::UnknownTag(res.tag).to_capability_error(ident))
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
            1 | 2 => Err(unsafe { deserialize_error(res.output) }.to_capability_error(ident)),
            _ => {
                error!(tag = res.tag, "ExecutionResultBridge: unknown tag received");
                Err(FfiError::UnknownTag(res.tag).to_capability_error(ident))
            }
        }
    }
}

pub enum AsyncExecState<'a> {
    Ffi(async_ffi::BorrowingFfiFuture<'a, *const u8>),
    Ready(Option<Result<BridgeVec, PyroductError>>),
}

/// Wrapper for the async execution future that handles both pending futures and early errors.
pub struct AsyncExecFuture<'a> {
    state: AsyncExecState<'a>,
    ident: CapIdentity,
}

impl<'a> AsyncExecFuture<'a> {
    pub fn new(res: FfiBorrowedFutureResult<'a>, ident: &CapIdentity) -> Self {
        let state = match res {
            FfiBorrowedFutureResult::Future(fut) => {
                trace!("AsyncExecFuture: created from Future variant");
                AsyncExecState::Ffi(fut)
            }
            FfiBorrowedFutureResult::EarlyError(val) => {
                trace!("AsyncExecFuture: created from EarlyError variant");
                // Convert the early result immediately
                let result = unsafe { ExecutionResultBridge::from_ffi(val, ident) };
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
    type Output = Result<BridgeVec, PyroductError>;

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
