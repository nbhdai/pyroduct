use std::panic::Location;

use bridge_vec::{Bridgeable, ffi::{deserialize_input, execute_safe}};
use rkyv::Archive;
use super::get_capability_state;

#[track_caller]
pub fn sci_call<'a, S, C, I, O, F>(
    client_state_ptr: *const u8,
    input_ptr: *const u8,
    host_state_ptr: *mut std::ffi::c_void,
    func: F,
) -> *const u8
where
    S: Send + std::panic::UnwindSafe + 'a,
    &'a mut S: std::panic::UnwindSafe,
    C: Bridgeable + Send + std::panic::UnwindSafe + 'a,
    <C as Archive>::Archived: 'static,
    I: Bridgeable + Send + std::panic::UnwindSafe + 'a,
    <I as Archive>::Archived: 'static,
    O: Bridgeable + std::panic::RefUnwindSafe + Send + 'static,
    F: FnOnce(&'a mut S, C, I) -> O + Send + std::panic::UnwindSafe + 'a,
{
    let state = match unsafe { get_capability_state::<'a, S>(host_state_ptr) } {
        Ok(state) => state,
        Err(error) => return error.encode().into_raw(),
    };

    let client: C = match deserialize_input(client_state_ptr, Location::caller()) {
        Ok(buf) => buf,
        Err(err) => return err.encode().into_raw(),
    };
    let input: I = match deserialize_input(input_ptr, Location::caller()) {
        Ok(buf) => buf,
        Err(err) => return err.encode().into_raw(),
    };

    execute_safe(|| (func)(state, client, input))
}

#[track_caller]
pub fn sc_call<'a, S, C, O, F>(
    client_state_ptr: *const u8,
    _input_ptr: *const u8,
    host_state_ptr: *mut std::ffi::c_void,
    func: F,
) -> *const u8
where
    S: Send + std::panic::UnwindSafe + 'a,
    &'a mut S: std::panic::UnwindSafe,
    C: Bridgeable + Send + std::panic::UnwindSafe + 'a,
    <C as Archive>::Archived: 'static,
    O: Bridgeable + std::panic::RefUnwindSafe + Send + 'static,
    F: FnOnce(&'a mut S, C) -> O + Send + std::panic::UnwindSafe + 'a,
{
    let state = match unsafe { get_capability_state::<'a, S>(host_state_ptr) } {
        Ok(state) => state,
        Err(error) => return error.encode().into_raw(),
    };

    let client: C = match deserialize_input(client_state_ptr, Location::caller()) {
        Ok(buf) => buf,
        Err(err) => return err.encode().into_raw(),
    };
    execute_safe(|| (func)(state, client))
}

#[track_caller]
pub fn i_call<'a, I, O, F>(
    _client_state_ptr: *const u8,
    input_ptr: *const u8,
    _host_state_ptr: *mut std::ffi::c_void,
    func: F,
) -> *const u8
where
    I: Bridgeable + Send + std::panic::UnwindSafe + 'a,
    <I as Archive>::Archived: 'static,
    O: Bridgeable + std::panic::RefUnwindSafe + Send + 'static,
    F: FnOnce(I) -> O + Send + std::panic::UnwindSafe + 'a,
{
    let input: I = match deserialize_input(input_ptr, Location::caller()) {
        Ok(buf) => buf,
        Err(err) => return err.encode().into_raw(),
    };
    execute_safe(|| (func)(input))
}

pub fn empty_call<'a, O, F>(
    _client_state_ptr: *const u8,
    _client_state_len: usize,
    _input_ptr: *const u8,
    _input_len: usize,
    _host_state_ptr: *mut std::ffi::c_void,
    func: F,
) -> *const u8
where
    O: Bridgeable + std::panic::RefUnwindSafe + Send + 'static,
    F: FnOnce() -> O + Send + std::panic::UnwindSafe + 'a,
{
    execute_safe(|| (func)())
}