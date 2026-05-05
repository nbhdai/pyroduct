//! # FFI Boundary Safety & Execution
//!
//! This module provides the "safety firewall" required when exposing Rust functions to foreign code.
//! It guarantees that panics and errors strictly adhere to the `PyroView` protocol and never
//! unwind across the FFI boundary.
//!
//! ## Core Responsibilities
//!
//! 1.  **Panic Isolation**: Wraps user logic in `catch_unwind`.
//! 2.  **Rich Error Reporting**: Captures diagnostic info into TLS.
//! 3.  **Serialization Safety**: Guards the serialization step itself.
//! 4.  **Boundary Types**: Exclusively uses `PyroViewPtr` for stable ABI passing.

use ::async_ffi::BorrowingFfiFuture;
use futures::FutureExt;
use std::panic::{self, AssertUnwindSafe};
use std::sync::OnceLock;
use tokio::runtime::Runtime;
use tracing::Instrument;

use crate::ffi::FuturePyroView;
use crate::ffi::guest::logger::object_span;
use crate::format::{Bridgeable, Decoder, PyroRefPtr, PyroView, PyroViewPtr};
use crate::panic::{clear_last_panic, recover_panic_info, register_ffi_panic_hook};
use crate::{CapturedError, PyroError};

// ============================================================================
// Execution & Serialization Logic
// ============================================================================

/// Safe entry point for FFI operations returning a PyroViewPtr.
#[track_caller]
pub fn execute_safe<F>(func: F, object_id: u64) -> PyroViewPtr
where
    F: FnOnce() -> PyroView,
{
    let span = object_span(object_id);
    let _guard = span.enter();
    register_ffi_panic_hook();
    clear_last_panic();

    let result = panic::catch_unwind(AssertUnwindSafe(func));

    match result {
        Ok(output_obj) => output_obj.into_ptr(),
        Err(_) => {
            let panic_info = recover_panic_info();
            PyroError::CodePanic(panic_info).encode().into_ptr()
        }
    }
}

/// Helper to deserialize input from a raw PyroViewPtr.
pub fn deserialize_input<I>(data: PyroRefPtr) -> Result<I, PyroError>
where
    for<'a> I: Bridgeable + From<I::Ref<'a>>,
{
    tracing::trace!("Deserializing view");
    let guard = panic::catch_unwind(AssertUnwindSafe(|| {
        let view = unsafe { data.try_ref() }?;
        let mut decoder = <I as Bridgeable>::Decoder::default();
        let typed = decoder.decode(&view)?;
        Ok(typed.into())
    }));

    match guard {
        Ok(input) => input,
        Err(_) => {
            let panic_info = recover_panic_info().with_source("Panic");
            Err(PyroError::deserialization(panic_info))
        }
    }
}

/// Helper to serialize a successful output object.
pub fn serialize_output<T>(val: T) -> PyroView
where
    T: Bridgeable,
{
    tracing::trace!("Serializing return");
    let guard = panic::catch_unwind(AssertUnwindSafe(|| val.ship()));

    match guard {
        Ok(Ok(vec)) => vec,
        Ok(Err(e)) => e.encode(),
        Err(_) => {
            let panic_info = recover_panic_info().with_source("Panic");
            PyroError::serialization(panic_info).encode()
        }
    }
}

/// Helper to serialize a Result into a PyroVec.
pub fn serialize_result<T, E>(result: Result<T, E>) -> PyroView
where
    T: Bridgeable,
    E: Bridgeable,
{
    tracing::trace!("Serializing return result");
    let guard = panic::catch_unwind(AssertUnwindSafe(|| result.ship()));

    match guard {
        Ok(Ok(vec)) => vec,
        Ok(Err(e)) => e.encode(),
        Err(_) => {
            let panic_info = recover_panic_info().with_source("Panic");
            PyroError::serialization(panic_info).encode()
        }
    }
}

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

pub fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| Runtime::new().expect("Failed to create Tokio runtime in plugin"))
}

#[track_caller]
pub fn execute_safe_async<F, Fut>(f: F, object_id: u64) -> FuturePyroView
where
    F: FnOnce() -> Fut + Send,
    Fut: std::future::Future<Output = PyroView> + Send + 'static,
{
    let fut = f();
    FuturePyroView::Future(BorrowingFfiFuture::<'static>::new(SafeMethodHandle::new(
        get_runtime().spawn(async move {
            let span = object_span(object_id);
            {
                let _guard = span.enter();
                register_ffi_panic_hook();
                clear_last_panic();
            }

            let result = std::panic::AssertUnwindSafe(fut)
                .catch_unwind()
                .instrument(span)
                .await;

            match result {
                Ok(val) => val.into_ptr(),
                Err(_) => {
                    let panic_info = recover_panic_info();
                    PyroError::CodePanic(panic_info).encode().into_ptr()
                }
            }
        }),
    )))
}

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::task::JoinHandle;

pub struct SafeMethodHandle {
    handle: JoinHandle<PyroViewPtr>,
}

impl SafeMethodHandle {
    pub fn new(handle: JoinHandle<PyroViewPtr>) -> Self {
        Self { handle }
    }
}

impl Future for SafeMethodHandle {
    type Output = PyroViewPtr;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Poll the underlying tokio JoinHandle
        match Pin::new(&mut self.handle).poll(cx) {
            Poll::Ready(Ok(val)) => Poll::Ready(val),
            Poll::Ready(Err(join_error)) => Poll::Ready(
                PyroError::CodePanic(CapturedError::new(join_error).into())
                    .encode()
                    .into_ptr(),
            ),
            Poll::Pending => Poll::Pending,
        }
    }
}
