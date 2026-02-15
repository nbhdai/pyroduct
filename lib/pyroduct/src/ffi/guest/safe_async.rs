use std::panic::Location;
// TODO: make this borrow C

use crate::{
    Bridgeable, PyroError, PyroViewPtr,
    ffi::{
        FuturePyroVec, PyroObjectRef, PyroRefObjectPtr,
        guest::panic_wrap::{deserialize_input, execute_safe_async},
    },
    format::PyroZeroCopyFormat,
};

/// Complete call with state, client, and input
#[track_caller]
pub fn sci_call<'a, S, C, I, O, F, Fut>(
    host_state_ptr: PyroRefObjectPtr,
    client_state_ptr: PyroViewPtr,
    input_ptr: PyroViewPtr,
    func: F,
) -> FuturePyroVec<'a>
where
    S: Send + Sync + std::panic::UnwindSafe + 'static,
    &'a S: std::panic::UnwindSafe,
    C: Bridgeable,
    <C as Bridgeable>::Format: PyroZeroCopyFormat<C>,
    I: Bridgeable,
    <I as Bridgeable>::Format: PyroZeroCopyFormat<I>,
    O: Bridgeable + std::panic::RefUnwindSafe + Send + 'static,
    F: FnOnce(&'a S, C, I) -> Fut + Send + std::panic::UnwindSafe + 'a,
    Fut: std::future::Future<Output = O> + Send + std::panic::UnwindSafe + 'a,
{
    let state = match unsafe { PyroObjectRef::<'a>::from_raw(host_state_ptr) } {
        Ok(state) => state,
        Err(error) => return PyroError::CodePanic(error.into()).encode().into(),
    };

    let client: C = match deserialize_input(client_state_ptr, Location::caller()) {
        Ok(buf) => buf,
        Err(err) => return err.encode().into(),
    };
    let input: I = match deserialize_input(input_ptr, Location::caller()) {
        Ok(buf) => buf,
        Err(err) => return err.encode().into(),
    };

    execute_safe_async((func)(state.as_ref(), client, input))
}

/// Call with state and client (no input)
#[track_caller]
pub fn sc_call<'a, S, C, O, F, Fut>(
    host_state_ptr: PyroRefObjectPtr,
    client_state_ptr: PyroViewPtr,
    _input_ptr: PyroViewPtr,
    func: F,
) -> FuturePyroVec<'a>
where
    S: Send + Sync + std::panic::UnwindSafe + 'static,
    &'a S: std::panic::UnwindSafe,
    C: Bridgeable,
    <C as Bridgeable>::Format: PyroZeroCopyFormat<C>,
    O: Bridgeable + std::panic::RefUnwindSafe + Send + 'static,
    F: FnOnce(&'a S, C) -> Fut + Send + std::panic::UnwindSafe + 'a,
    Fut: std::future::Future<Output = O> + Send + std::panic::UnwindSafe + 'a,
{
    let state = match unsafe { PyroObjectRef::<'a>::from_raw(host_state_ptr) } {
        Ok(state) => state,
        Err(error) => return PyroError::CodePanic(error.into()).encode().into(),
    };

    let client: C = match deserialize_input(client_state_ptr, Location::caller()) {
        Ok(buf) => buf,
        Err(err) => return err.encode().into(),
    };
    execute_safe_async((func)(state.as_ref(), client))
}

/// Call with input only
#[track_caller]
pub fn i_call<'a, I, O, F, Fut>(
    _host_state_ptr: PyroRefObjectPtr,
    _client_state_ptr: PyroViewPtr,
    input_ptr: PyroViewPtr,
    func: F,
) -> FuturePyroVec<'a>
where
    I: Bridgeable,
    <I as Bridgeable>::Format: PyroZeroCopyFormat<I>,
    O: Bridgeable + std::panic::RefUnwindSafe + Send + 'static,
    F: FnOnce(I) -> Fut + Send + std::panic::UnwindSafe + 'a,
    Fut: std::future::Future<Output = O> + Send + std::panic::UnwindSafe + 'a,
{
    let input: I = match deserialize_input(input_ptr, Location::caller()) {
        Ok(buf) => buf,
        Err(err) => return err.encode().into(),
    };
    execute_safe_async((func)(input))
}

pub fn empty_call<'a, O, F, Fut>(
    _host_state_ptr: PyroRefObjectPtr,
    _client_state_ptr: PyroViewPtr,
    _input_ptr: PyroViewPtr,
    func: F,
) -> FuturePyroVec<'a>
where
    O: Bridgeable + std::panic::RefUnwindSafe + Send + 'static,
    F: FnOnce() -> Fut + Send + std::panic::UnwindSafe + 'a,
    Fut: std::future::Future<Output = O> + Send + std::panic::UnwindSafe + 'a,
{
    execute_safe_async((func)())
}
