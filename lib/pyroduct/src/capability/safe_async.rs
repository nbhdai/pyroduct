use std::panic::Location;

use bridge_vec::{Bridgeable, ffi::{deserialize_input, execute_safe_async}};
use rkyv::Archive;

use crate::capability_host::ffi::FfiBorrowedFutureResult;

use super::get_capability_state;

/// Complete call with state, client, and input
#[track_caller]
pub fn sci_call<'a, S, C, I, O, F, Fut>(
    client_state_ptr: *const u8,
    input_ptr: *const u8,
    host_state_ptr: *mut std::ffi::c_void,
    func: F,
) -> FfiBorrowedFutureResult<'a>
where
    S: Send + std::panic::RefUnwindSafe + 'a,
    C: Bridgeable + Send + std::panic::UnwindSafe + 'a,
    <C as Archive>::Archived: 'static,
    I: Bridgeable + Send + std::panic::UnwindSafe + 'a,
    <I as Archive>::Archived: 'static,
    O: Bridgeable + std::panic::RefUnwindSafe + Send + 'static,
    F: FnOnce(&'a mut S, C, I) -> Fut + Send + std::panic::UnwindSafe + 'a,
    Fut: std::future::Future<Output = O> + Send+ std::panic::UnwindSafe + 'a,
{
    let state = match unsafe { get_capability_state::<'a, S>(host_state_ptr) } {
        Ok(state) => state,
        Err(error) => return error.into(),
    };

    let client: C = match deserialize_input(client_state_ptr, Location::caller()) {
        Ok(buf) => buf,
        Err(err) => return err.into(),
    };
    let input: I = match deserialize_input(input_ptr, Location::caller()) {
        Ok(buf) => buf,
        Err(err) => return err.into(),
    };

    execute_safe_async((func)(state, client, input)).into()
}

/// Call with state and client (no input)
#[track_caller]
pub fn sc_call<'a, S, C, O, F, Fut>(
    client_state_ptr: *const u8,
    _input_ptr: *const u8,
    host_state_ptr: *mut std::ffi::c_void,
    func: F,
) -> FfiBorrowedFutureResult<'a>
where
    S: Send + std::panic::RefUnwindSafe + 'a,
    C: Bridgeable + Send + std::panic::UnwindSafe + 'a,
    <C as Archive>::Archived: 'static,
    O: Bridgeable + std::panic::RefUnwindSafe + Send + 'static,
    F: FnOnce(&'a mut S, C) -> Fut + Send + std::panic::UnwindSafe + 'a,
    Fut: std::future::Future<Output = O> + Send + std::panic::UnwindSafe + 'a,
{
    let state = match unsafe { get_capability_state::<'a, S>(host_state_ptr) } {
        Ok(state) => state,
        Err(error) => return error.into(),
    };

    let client: C = match deserialize_input(client_state_ptr, Location::caller()) {
        Ok(buf) => buf,
        Err(err) => return err.into(),
    };
    execute_safe_async((func)(state, client)).into()
}

/// Call with input only
#[track_caller]
pub fn i_call<'a, I, O, F, Fut>(
    _client_state_ptr: *const u8,
    input_ptr: *const u8,
    _host_state_ptr: *mut std::ffi::c_void,
    func: F,
) -> FfiBorrowedFutureResult<'a>
where
    I: Bridgeable + Send + std::panic::UnwindSafe + 'a,
    <I as Archive>::Archived: 'static,
    O: Bridgeable + std::panic::RefUnwindSafe + Send + 'static,
    F: FnOnce(I) -> Fut + Send + std::panic::UnwindSafe + 'a,
    Fut: std::future::Future<Output = O> + Send + std::panic::UnwindSafe + 'a,
{
    let input: I = match deserialize_input(input_ptr, Location::caller()) {
        Ok(buf) => buf,
        Err(err) => return err.into(),
    };
    execute_safe_async((func)(input)).into()
}

pub fn empty_call<'a, O, F, Fut>(
    _client_state_ptr: *const u8,
    _input_ptr: *const u8,
    _host_state_ptr: *mut std::ffi::c_void,
    func: F,
) -> FfiBorrowedFutureResult<'a>
where
    O: Bridgeable + std::panic::RefUnwindSafe + Send + 'static,
    F: FnOnce() -> Fut + Send + std::panic::UnwindSafe + 'a,
    Fut: std::future::Future<Output = O> + Send + std::panic::UnwindSafe + 'a,
{
    execute_safe_async((func)()).into()
}