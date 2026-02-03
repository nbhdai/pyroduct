use std::sync::OnceLock;

use async_ffi::BorrowingFfiFuture;
use bridge_vec::{BridgeVec, ser_de::Bridgable};
use futures::FutureExt;
use tokio::runtime::Runtime;
use tracing::{debug, trace};

use crate::{
    errors::FfiError,
    module_capability::panic::{clear_last_panic, recover_panic_info},
};

use super::safe_io;

// There is async functions in this plugin.
static RUNTIME: OnceLock<Runtime> = OnceLock::new();

pub fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        trace!("get_runtime: initializing tokio runtime");
        Runtime::new().expect("Failed to create Tokio runtime in plugin")
    })
}

/// Result type for async FFI calls - either an early error or a future
pub enum FfiBorrowedFutureResult<'a> {
    EarlyError(BridgeVec),
    Future(BorrowingFfiFuture<'a, BridgeVec>),
}

impl<'a> From<FfiError> for FfiBorrowedFutureResult<'a> {
    fn from(error: FfiError) -> Self {
        FfiBorrowedFutureResult::EarlyError(safe_io::make_error_output(error))
    }
}

pub fn execute_safe_async<'a, Fut, O>(fut: Fut) -> FfiBorrowedFutureResult<'a>
where
    Fut: std::future::Future<Output = O> + Send + 'a,
    O: Bridgable + std::panic::RefUnwindSafe + Send + 'static,
{
    trace!("execute_safe_async: preparing future");
    let _guard = get_runtime().enter();

    FfiBorrowedFutureResult::Future(BorrowingFfiFuture::<'a>::new(async move {
        trace!("execute_safe_async: future polling started");
        clear_last_panic();
        let result = std::panic::AssertUnwindSafe(fut).catch_unwind().await;

        match result {
            Ok(logic_result) => {
                debug!("execute_safe_async: logic completed successfully");
                safe_io::make_output(&logic_result)
            }
            Err(_) => {
                let panic = recover_panic_info();
                trace!(panic = ?panic, "execute_safe_async: panic caught during async execution");
                safe_io::make_error_output(FfiError::CapabilityLogicPanicked(panic))
            }
        }
    }))
}

/// Complete call with state, client, and input
pub fn sci_call<'a, S, C, I, O, F, Fut>(
    client_state_ptr: *const u8,
    client_state_len: usize,
    input_ptr: *const u8,
    input_len: usize,
    host_state_ptr: *mut std::ffi::c_void,
    func: F,
) -> FfiBorrowedFutureResult<'a>
where
    S: Send + 'a,
    C: Bridgable + Send + 'a,
    I: Bridgable + Send + 'a,
    O: Bridgable + std::panic::RefUnwindSafe + Send + 'static,
    F: FnOnce(&'a mut S, C, I) -> Fut + Send + 'a,
    Fut: std::future::Future<Output = O> + Send + 'a,
{
    let state = match unsafe { safe_io::get_capability_state::<'a, S>(host_state_ptr) } {
        Ok(state) => state,
        Err(error) => return error.into(),
    };

    let client: C = match unsafe { safe_io::get_input::<C>(client_state_ptr, client_state_len) } {
        Ok(client) => client,
        Err(error) => return error.into(),
    };

    let input: I = match unsafe { safe_io::get_input::<I>(input_ptr, input_len) } {
        Ok(input) => input,
        Err(error) => return error.into(),
    };

    execute_safe_async((func)(state, client, input))
}

/// Call with state and client (no input)
pub fn sc_call<'a, S, C, O, F, Fut>(
    client_state_ptr: *const u8,
    client_state_len: usize,
    _input_ptr: *const u8,
    _input_len: usize,
    host_state_ptr: *mut std::ffi::c_void,
    func: F,
) -> FfiBorrowedFutureResult<'a>
where
    S: Send + 'a,
    C: Bridgable + Send + 'a,
    O: Bridgable + std::panic::RefUnwindSafe + Send + 'static,
    F: FnOnce(&'a mut S, C) -> Fut + Send + 'a,
    Fut: std::future::Future<Output = O> + Send + 'a,
{
    let state = match unsafe { safe_io::get_capability_state::<'a, S>(host_state_ptr) } {
        Ok(state) => state,
        Err(error) => return error.into(),
    };

    let client: C = match unsafe { safe_io::get_input::<C>(client_state_ptr, client_state_len) } {
        Ok(client) => client,
        Err(error) => return error.into(),
    };
    execute_safe_async((func)(state, client))
}

/// Call with input only (no state or client)
pub fn i_call<'a, I, O, F, Fut>(
    _client_state_ptr: *const u8,
    _client_state_len: usize,
    input_ptr: *const u8,
    input_len: usize,
    _host_state_ptr: *mut std::ffi::c_void,
    func: F,
) -> FfiBorrowedFutureResult<'a>
where
    I: Bridgable + Send + 'a,
    O: Bridgable + std::panic::RefUnwindSafe + Send + 'static,
    F: FnOnce(I) -> Fut + Send + 'a,
    Fut: std::future::Future<Output = O> + Send + 'a,
{
    let input: I = match unsafe { safe_io::get_input::<I>(input_ptr, input_len) } {
        Ok(input) => input,
        Err(error) => return error.into(),
    };
    execute_safe_async((func)(input))
}

/// Call with no arguments
pub fn empty_call<'a, O, F, Fut>(
    _client_state_ptr: *const u8,
    _client_state_len: usize,
    _input_ptr: *const u8,
    _input_len: usize,
    _host_state_ptr: *mut std::ffi::c_void,
    func: F,
) -> FfiBorrowedFutureResult<'a>
where
    O: Bridgable + std::panic::RefUnwindSafe + Send + 'static,
    F: FnOnce() -> Fut + Send + 'a,
    Fut: std::future::Future<Output = O> + Send + 'a,
{
    execute_safe_async((func)())
}