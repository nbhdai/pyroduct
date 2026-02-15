use crate::capability_host::ffi::{FfiBorrowedFutureResult, FfiInitResult};
use std::{
    ffi::c_void,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use bridge_vec::{BridgeError, BridgeVec, CapturedError};
use pin_project::pin_project;
use tracing::{error, trace};

pub struct InitResultBridge;

impl InitResultBridge {
    pub unsafe fn from_ffi(
        res: FfiInitResult,
    ) -> Result<*mut c_void, BridgeError> {
        let potential_error = unsafe { BridgeVec::from_raw(res.error) }?;
        if potential_error.is_ok() {
            Ok(res.state)
        } else {
            Err(potential_error.parse_as_error())
        }
    }
}

#[pin_project]
pub enum AsyncExecState<'a> {
    Ffi(async_ffi::BorrowingFfiFuture<'a, *const u8>),
    Ready(Option<Result<BridgeVec, BridgeError>>),
}

/// Wrapper for the async execution future that handles both pending futures and early errors.
pub struct AsyncExecFuture<'a> {
    state: AsyncExecState<'a>,
}

impl<'a> AsyncExecFuture<'a> {
    pub fn new(res: FfiBorrowedFutureResult<'a>) -> Self {
        let state = match res {
            FfiBorrowedFutureResult::Future(fut) => {
                trace!("AsyncExecFuture: created from Future variant");
                AsyncExecState::Ffi(fut)
            }
            FfiBorrowedFutureResult::EarlyError(val) => {
                trace!("AsyncExecFuture: created from EarlyError variant");
                // Convert the early result immediately
                AsyncExecState::Ready(Some(unsafe { BridgeVec::from_raw(val) }))
            }
        };
        Self {
            state,
        }
    }
}

impl<'a> Future for AsyncExecFuture<'a> {
    type Output = Result<BridgeVec, BridgeError>;

    #[track_caller]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        match &mut this.state {
            AsyncExecState::Ffi(fut) => match Pin::new(fut).poll(cx) {
                Poll::Ready(res) => {
                    trace!("AsyncExecFuture: underlying future ready");
                    Poll::Ready(unsafe { BridgeVec::from_raw(res) })
                }
                Poll::Pending => Poll::Pending,
            },
            AsyncExecState::Ready(res) => {
                trace!("AsyncExecFuture: returning ready result");
                Poll::Ready(res.take().unwrap_or_else(|| {
                    error!("AsyncExecFuture: polled after completion");
                    Err(
                    BridgeError::CodePanic(CapturedError::new("AsyncInitFuture: polled after completion").with_location(std::panic::Location::caller()).with_backtrace(std::backtrace::Backtrace::capture()).into())
                    )
                }))
            }
        }
    }
}
