use std::panic::Location;
// TODO: make this borrow C

use crate::{
    Bridgeable, PyroError, PyroViewPtr,
    ffi::{
        FuturePyroVec, PyroObjectRef, PyroRefObjectPtr,
        guest::panic_wrap::{deserialize_input, execute_safe_async, execute_safe_result_async},
    },
    format::PyroZeroCopyFormat,
};

/// Complete call with state, client, and input
#[track_caller]
pub fn sci_call_result<'a, S, C, I, O, E, F, Fut>(
    host_state_ptr: PyroRefObjectPtr,
    client_state_ptr: PyroViewPtr,
    input_ptr: PyroViewPtr,
    func: F,
) -> FuturePyroVec<'a>
where
    S: Send + Sync + 'static,
    C: Bridgeable,
    <C as Bridgeable>::Format: PyroZeroCopyFormat<C>,
    I: Bridgeable,
    <I as Bridgeable>::Format: PyroZeroCopyFormat<I>,
    O: Bridgeable + Send + 'static,
    E: Bridgeable + Send + 'static,
    F: FnOnce(&'a S, C, I) -> Fut + Send + 'a,
    Fut: std::future::Future<Output = Result<O, E>> + Send + 'a,
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

    execute_safe_result_async((func)(state.as_ref(), client, input))
}

/// Complete call with state, client, and input
#[track_caller]
pub fn sci_call<'a, S, C, I, O, F, Fut>(
    host_state_ptr: PyroRefObjectPtr,
    client_state_ptr: PyroViewPtr,
    input_ptr: PyroViewPtr,
    func: F,
) -> FuturePyroVec<'a>
where
    S: Send + Sync + 'static,
    C: Bridgeable,
    <C as Bridgeable>::Format: PyroZeroCopyFormat<C>,
    I: Bridgeable,
    <I as Bridgeable>::Format: PyroZeroCopyFormat<I>,
    O: Bridgeable + Send + 'static,
    F: FnOnce(&'a S, C, I) -> Fut + Send + 'a,
    Fut: std::future::Future<Output = O> + Send + 'a,
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
pub fn sc_call_result<'a, S, C, O, E, F, Fut>(
    host_state_ptr: PyroRefObjectPtr,
    client_state_ptr: PyroViewPtr,
    _input_ptr: PyroViewPtr,
    func: F,
) -> FuturePyroVec<'a>
where
    S: Send + Sync + 'static,
    C: Bridgeable,
    <C as Bridgeable>::Format: PyroZeroCopyFormat<C>,
    O: Bridgeable + Send + 'static,
    E: Bridgeable + Send + 'static,
    F: FnOnce(&'a S, C) -> Fut + Send + 'a,
    Fut: std::future::Future<Output = Result<O, E>> + Send + 'a,
{
    let state = match unsafe { PyroObjectRef::<'a>::from_raw(host_state_ptr) } {
        Ok(state) => state,
        Err(error) => return PyroError::CodePanic(error.into()).encode().into(),
    };

    let client: C = match deserialize_input(client_state_ptr, Location::caller()) {
        Ok(buf) => buf,
        Err(err) => return err.encode().into(),
    };
    execute_safe_result_async((func)(state.as_ref(), client))
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
    S: Send + Sync + 'static,
    C: Bridgeable,
    <C as Bridgeable>::Format: PyroZeroCopyFormat<C>,
    O: Bridgeable + Send + 'static,
    F: FnOnce(&'a S, C) -> Fut + Send + 'a,
    Fut: std::future::Future<Output = O> + Send + 'a,
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
pub fn i_call_result<'a, I, O, E, F, Fut>(
    _host_state_ptr: PyroRefObjectPtr,
    _client_state_ptr: PyroViewPtr,
    input_ptr: PyroViewPtr,
    func: F,
) -> FuturePyroVec<'a>
where
    I: Bridgeable,
    <I as Bridgeable>::Format: PyroZeroCopyFormat<I>,
    O: Bridgeable + Send + 'static,
    E: Bridgeable + Send + 'static,
    F: FnOnce(I) -> Fut + Send + 'a,
    Fut: std::future::Future<Output = Result<O, E>> + Send + 'a,
{
    let input: I = match deserialize_input(input_ptr, Location::caller()) {
        Ok(buf) => buf,
        Err(err) => return err.encode().into(),
    };
    execute_safe_result_async((func)(input))
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
    O: Bridgeable + Send + 'static,
    F: FnOnce(I) -> Fut + Send + 'a,
    Fut: std::future::Future<Output = O> + Send + 'a,
{
    let input: I = match deserialize_input(input_ptr, Location::caller()) {
        Ok(buf) => buf,
        Err(err) => return err.encode().into(),
    };
    execute_safe_async((func)(input))
}

pub fn empty_call_result<'a, O, E, F, Fut>(
    _host_state_ptr: PyroRefObjectPtr,
    _client_state_ptr: PyroViewPtr,
    _input_ptr: PyroViewPtr,
    func: F,
) -> FuturePyroVec<'a>
where
    O: Bridgeable + Send + 'static,
    E: Bridgeable + Send + 'static,
    F: FnOnce() -> Fut + Send + 'a,
    Fut: std::future::Future<Output = Result<O, E>> + Send + 'a,
{
    execute_safe_result_async((func)())
}


pub fn empty_call<'a, O, F, Fut>(
    _host_state_ptr: PyroRefObjectPtr,
    _client_state_ptr: PyroViewPtr,
    _input_ptr: PyroViewPtr,
    func: F,
) -> FuturePyroVec<'a>
where
    O: Bridgeable + Send + 'static,
    F: FnOnce() -> Fut + Send + 'a,
    Fut: std::future::Future<Output = O> + Send + 'a,
{
    execute_safe_async((func)())
}
