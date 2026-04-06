use std::{
    pin::Pin,
    task::{Context, Poll},
};

use crate::{
    PyroError,
    ffi::{FutureInitResult, FuturePyroVec, InitResult, PyroObject},
    format::{PyroVec, PyroVecPtr, header::PyroData},
};

#[pin_project::pin_project(project = ResetProj)]
pub enum ObjectResetFuture {
    Async(#[pin] ::async_ffi::BorrowingFfiFuture<'static, PyroVecPtr>),
    Ready(Option<Result<(), PyroError>>),
}

impl ObjectResetFuture {
    pub fn from_async(res: FuturePyroVec) -> Self {
        match res {
            FuturePyroVec::Future(fut) => Self::Async(fut),
            FuturePyroVec::Early(ptr) => {
                let res = unsafe { PyroVec::from_raw(ptr) }.and_then(|v| v.parse_as_error());
                Self::Ready(Some(res))
            }
        }
    }
}

impl Future for ObjectResetFuture {
    type Output = Result<(), PyroError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            ResetProj::Async(fut) => match fut.poll(cx) {
                Poll::Ready(vec_ptr) => {
                    let res =
                        unsafe { PyroVec::from_raw(vec_ptr) }.and_then(|v| v.parse_as_error());
                    Poll::Ready(res)
                }
                Poll::Pending => Poll::Pending,
            },
            ResetProj::Ready(res) => Poll::Ready(
                res.take()
                    .expect("ObjectResetFuture polled after completion"),
            ),
        }
    }
}

#[pin_project::pin_project(project = InitProj)]
pub enum ObjectInitFuture {
    Async(#[pin] ::async_ffi::BorrowingFfiFuture<'static, InitResult>),
    Ready(Option<Result<PyroObject, PyroError>>),
}

impl ObjectInitFuture {
    pub fn from_async(res: FutureInitResult) -> Self {
        match res {
            FutureInitResult::Future(fut) => Self::Async(fut),
            FutureInitResult::EarlyError(init_res) => Self::Ready(Some(init_res.process())),
        }
    }
}

impl Future for ObjectInitFuture {
    type Output = Result<PyroObject, PyroError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            InitProj::Async(fut) => match fut.poll(cx) {
                Poll::Ready(init_res) => Poll::Ready(init_res.process()),
                Poll::Pending => Poll::Pending,
            },
            InitProj::Ready(res) => Poll::Ready(
                res.take()
                    .expect("ObjectInitFuture polled after completion"),
            ),
        }
    }
}

#[pin_project::pin_project(project = RegisterProj)]
pub enum ClientRegisterFuture {
    Async(#[pin] ::async_ffi::BorrowingFfiFuture<'static, PyroVecPtr>),
    Ready(Option<Result<PyroVec, PyroError>>),
}

impl ClientRegisterFuture {
    pub fn from_async(res: FuturePyroVec) -> Self {
        match res {
            FuturePyroVec::Future(fut) => Self::Async(fut),
            FuturePyroVec::Early(ptr) => {
                let res = unsafe { PyroVec::from_raw(ptr) };
                Self::Ready(Some(res))
            }
        }
    }
}

impl Future for ClientRegisterFuture {
    type Output = Result<PyroVec, PyroError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            RegisterProj::Async(fut) => match fut.poll(cx) {
                Poll::Ready(vec_ptr) => {
                    let res = unsafe { PyroVec::from_raw(vec_ptr) };
                    Poll::Ready(res)
                }
                Poll::Pending => Poll::Pending,
            },
            RegisterProj::Ready(res) => Poll::Ready(
                res.take()
                    .expect("ObjectResetFuture polled after completion"),
            ),
        }
    }
}

#[pin_project::pin_project(project = MethodProj)]
pub enum MethodCallFuture {
    /// Wrapping an active async FFI call.
    Async(#[pin] ::async_ffi::BorrowingFfiFuture<'static, PyroVecPtr>),
    /// A synchronous result or an early error.
    Ready(Option<Result<PyroVec, PyroError>>),
}

impl MethodCallFuture {
    pub fn from_async(res: FuturePyroVec) -> Self {
        match res {
            FuturePyroVec::Future(fut) => Self::Async(fut),
            FuturePyroVec::Early(ptr) => {
                let res = unsafe { PyroVec::from_raw(ptr) };
                Self::Ready(Some(res))
            }
        }
    }
}

impl Future for MethodCallFuture {
    type Output = Result<PyroVec, PyroError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            MethodProj::Async(fut) => match fut.poll(cx) {
                Poll::Ready(vec_ptr) => Poll::Ready(unsafe { PyroVec::from_raw(vec_ptr) }),
                Poll::Pending => Poll::Pending,
            },
            MethodProj::Ready(res) => Poll::Ready(
                res.take()
                    .expect("MethodCallFuture polled after completioS>n"),
            ),
        }
    }
}
