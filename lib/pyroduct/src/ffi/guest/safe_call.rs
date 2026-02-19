use std::panic::Location;

use crate::{
    Bridgeable, PyroError, PyroVecPtr, PyroViewPtr,
    ffi::{
        PyroObjectRef, PyroRefObjectPtr,
        guest::panic_wrap::{deserialize_input, execute_safe, execute_safe_result},
    },
    format::PyroZeroCopyFormat,
};

#[track_caller]
pub fn sci_call_result<'a, S, C, I, O, E, F>(
    host_state_ptr: PyroRefObjectPtr,
    client_state_ptr: PyroViewPtr,
    input_ptr: PyroViewPtr,
    func: F,
) -> PyroVecPtr
where
    S: Send + Sync + 'static,
    C: Bridgeable,
    <C as Bridgeable>::Format: PyroZeroCopyFormat<C>,
    I: Bridgeable,
    <I as Bridgeable>::Format: PyroZeroCopyFormat<I>,
    O: Bridgeable + Send + 'static,
    E: Bridgeable + Send + 'static,
    F: FnOnce(&'a S, C, I) -> Result<O, E> + Send + 'a,
{
    let state = match unsafe { PyroObjectRef::<'a>::from_raw(host_state_ptr) } {
        Ok(state) => state,
        Err(error) => return PyroError::CodePanic(error.into()).encode().into_raw(),
    };

    let client: C = match deserialize_input(client_state_ptr, Location::caller()) {
        Ok(buf) => buf,
        Err(err) => return err.encode().into_raw(),
    };
    let input: I = match deserialize_input(input_ptr, Location::caller()) {
        Ok(buf) => buf,
        Err(err) => return err.encode().into_raw(),
    };

    let result = execute_safe_result(|| (func)(state.as_ref(), client, input));
    result
}

#[track_caller]
pub fn sci_call<'a, S, C, I, O, F>(
    host_state_ptr: PyroRefObjectPtr,
    client_state_ptr: PyroViewPtr,
    input_ptr: PyroViewPtr,
    func: F,
) -> PyroVecPtr
where
    S: Send + Sync + 'static,
    C: Bridgeable,
    <C as Bridgeable>::Format: PyroZeroCopyFormat<C>,
    I: Bridgeable,
    <I as Bridgeable>::Format: PyroZeroCopyFormat<I>,
    O: Bridgeable + Send + 'static,
    F: FnOnce(&'a S, C, I) -> O + Send + 'a,
{
    let state = match unsafe { PyroObjectRef::<'a>::from_raw(host_state_ptr) } {
        Ok(state) => state,
        Err(error) => return PyroError::CodePanic(error.into()).encode().into_raw(),
    };

    let client: C = match deserialize_input(client_state_ptr, Location::caller()) {
        Ok(buf) => buf,
        Err(err) => return err.encode().into_raw(),
    };
    let input: I = match deserialize_input(input_ptr, Location::caller()) {
        Ok(buf) => buf,
        Err(err) => return err.encode().into_raw(),
    };

    let result = execute_safe(|| (func)(state.as_ref(), client, input));
    result
}

#[track_caller]
pub fn sc_call_result<'a, S, C, O, E, F>(
    host_state_ptr: PyroRefObjectPtr,
    client_state_ptr: PyroViewPtr,
    _input_ptr: PyroViewPtr,
    func: F,
) -> PyroVecPtr
where
    S: Send + Sync + 'static,
    &'a S: std::panic::UnwindSafe,
    C: Bridgeable + Send,
    <C as Bridgeable>::Format: PyroZeroCopyFormat<C>,
    O: Bridgeable + Send + 'static,
    E: Bridgeable + Send + 'static,
    F: FnOnce(&'a S, C) -> Result<O, E> + Send + 'a,
{
    let state = match unsafe { PyroObjectRef::<'a>::from_raw(host_state_ptr) } {
        Ok(state) => state,
        Err(error) => return PyroError::CodePanic(error.into()).encode().into_raw(),
    };

    let client: C = match deserialize_input(client_state_ptr, Location::caller()) {
        Ok(buf) => buf,
        Err(err) => return err.encode().into_raw(),
    };
    execute_safe_result(|| (func)(state.as_ref(), client))
}

#[track_caller]
pub fn sc_call<'a, S, C, O, F>(
    host_state_ptr: PyroRefObjectPtr,
    client_state_ptr: PyroViewPtr,
    _input_ptr: PyroViewPtr,
    func: F,
) -> PyroVecPtr
where
    S: Send + Sync + 'static,
    &'a S: std::panic::UnwindSafe,
    C: Bridgeable + Send,
    <C as Bridgeable>::Format: PyroZeroCopyFormat<C>,
    O: Bridgeable + Send + 'static,
    F: FnOnce(&'a S, C) -> O + Send + 'a,
{
    let state = match unsafe { PyroObjectRef::<'a>::from_raw(host_state_ptr) } {
        Ok(state) => state,
        Err(error) => return PyroError::CodePanic(error.into()).encode().into_raw(),
    };

    let client: C = match deserialize_input(client_state_ptr, Location::caller()) {
        Ok(buf) => buf,
        Err(err) => return err.encode().into_raw(),
    };
    execute_safe(|| (func)(state.as_ref(), client))
}

#[track_caller]
pub fn i_call_result<'a, I, O, E, F>(
    _host_state_ptr: PyroRefObjectPtr,
    _client_state_ptr: PyroViewPtr,
    input_ptr: PyroViewPtr,
    func: F,
) -> PyroVecPtr
where
    I: Bridgeable + Send,
    <I as Bridgeable>::Format: PyroZeroCopyFormat<I>,
    O: Bridgeable + Send + 'static,
    E: Bridgeable + Send + 'static,
    F: FnOnce(I) -> Result<O,E> + Send + 'a,
{
    let input: I = match deserialize_input(input_ptr, Location::caller()) {
        Ok(buf) => buf,
        Err(err) => return err.encode().into_raw(),
    };
    execute_safe_result(|| (func)(input))
}

#[track_caller]
pub fn i_call<'a, I, O, F>(
    _host_state_ptr: PyroRefObjectPtr,
    _client_state_ptr: PyroViewPtr,
    input_ptr: PyroViewPtr,
    func: F,
) -> PyroVecPtr
where
    I: Bridgeable + Send,
    <I as Bridgeable>::Format: PyroZeroCopyFormat<I>,
    O: Bridgeable + Send + 'static,
    F: FnOnce(I) -> O + Send + 'a,
{
    let input: I = match deserialize_input(input_ptr, Location::caller()) {
        Ok(buf) => buf,
        Err(err) => return err.encode().into_raw(),
    };
    execute_safe(|| (func)(input))
}

pub fn empty_call_result<'a, O, E, F>(
    _host_state_ptr: PyroRefObjectPtr,
    _client_state_ptr: PyroViewPtr,
    _input_ptr: PyroViewPtr,
    func: F,
) -> PyroVecPtr
where
    O: Bridgeable + Send + 'static,
    E: Bridgeable + Send + 'static,
    F: FnOnce() -> Result<O, E> + Send + 'a,
{
    execute_safe_result(|| (func)())
}


pub fn empty_call<'a, O, F>(
    _host_state_ptr: PyroRefObjectPtr,
    _client_state_ptr: PyroViewPtr,
    _input_ptr: PyroViewPtr,
    func: F,
) -> PyroVecPtr
where
    O: Bridgeable + Send + 'static,
    F: FnOnce() -> O + Send + 'a,
{
    execute_safe(|| (func)())
}
