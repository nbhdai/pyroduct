use std::panic::{self, AssertUnwindSafe};

use bridge_vec::{BridgeVec, Bridgeable};
use tracing::{debug, trace};

use crate::{
    capability::safe_io,
    errors::FfiError,
    module_capability::panic::{clear_last_panic, recover_panic_info},
};

pub(crate) fn execute_safe<F, O>(func: F) -> BridgeVec
where
    O: Bridgeable + std::panic::RefUnwindSafe + Send + 'static,
    F: FnOnce() -> O,
{
    clear_last_panic();
    match panic::catch_unwind(AssertUnwindSafe(|| (func)())) {
        Ok(logic_result) => {
            debug!("execute_safe: logic completed successfully");
            safe_io::make_output(&logic_result)
        }
        Err(_) => {
            let panic = recover_panic_info();
            trace!(panic = ?panic, "execute_safe: panic caught during execution");
            safe_io::make_error_output(FfiError::CapabilityLogicPanicked(panic))
        }
    }
}

/// Complete call with state, client, and input
pub fn sci_call<'a, S, C, I, O, F>(
    client_state_ptr: *const u8,
    client_state_len: usize,
    input_ptr: *const u8,
    input_len: usize,
    host_state_ptr: *mut std::ffi::c_void,
    func: F,
) -> BridgeVec
where
    S: Send + 'a,
    C: Bridgeable + Send + 'a,
    I: Bridgeable + Send + 'a,
    O: Bridgeable + std::panic::RefUnwindSafe + Send + 'static,
    F: FnOnce(&'a mut S, C, I) -> O + Send + 'a,
{
    let state = match unsafe { safe_io::get_capability_state::<'a, S>(host_state_ptr) } {
        Ok(state) => state,
        Err(error) => return safe_io::make_error_output(error),
    };

    let client: C = match unsafe { safe_io::get_input::<C>(client_state_ptr, client_state_len) } {
        Ok(client) => client,
        Err(error) => return safe_io::make_error_output(error),
    };

    let input: I = match unsafe { safe_io::get_input::<I>(input_ptr, input_len) } {
        Ok(input) => input,
        Err(error) => return safe_io::make_error_output(error),
    };

    execute_safe(|| (func)(state, client, input))
}

/// Call with state and client (no input)
pub fn sc_call<'a, S, C, O, F>(
    client_state_ptr: *const u8,
    client_state_len: usize,
    _input_ptr: *const u8,
    _input_len: usize,
    host_state_ptr: *mut std::ffi::c_void,
    func: F,
) -> BridgeVec
where
    S: Send + 'a,
    C: Bridgeable + Send + 'a,
    O: Bridgeable + std::panic::RefUnwindSafe + Send + 'static,
    F: FnOnce(&'a mut S, C) -> O + Send + 'a,
{
    let state = match unsafe { safe_io::get_capability_state::<'a, S>(host_state_ptr) } {
        Ok(state) => state,
        Err(error) => return safe_io::make_error_output(error),
    };

    let client: C = match unsafe { safe_io::get_input::<C>(client_state_ptr, client_state_len) } {
        Ok(client) => client,
        Err(error) => return safe_io::make_error_output(error),
    };
    execute_safe(|| (func)(state, client))
}

/// Call with input only (no state or client)
pub fn i_call<'a, I, O, F>(
    _client_state_ptr: *const u8,
    _client_state_len: usize,
    input_ptr: *const u8,
    input_len: usize,
    _host_state_ptr: *mut std::ffi::c_void,
    func: F,
) -> BridgeVec
where
    I: Bridgeable + Send + 'a,
    O: Bridgeable + std::panic::RefUnwindSafe + Send + 'static,
    F: FnOnce(I) -> O + Send + 'a,
{
    let input: I = match unsafe { safe_io::get_input::<I>(input_ptr, input_len) } {
        Ok(input) => input,
        Err(error) => return safe_io::make_error_output(error),
    };
    execute_safe(|| (func)(input))
}

/// Call with no arguments
pub fn empty_call<'a, O, F>(
    _client_state_ptr: *const u8,
    _client_state_len: usize,
    _input_ptr: *const u8,
    _input_len: usize,
    _host_state_ptr: *mut std::ffi::c_void,
    func: F,
) -> BridgeVec
where
    O: Bridgeable + std::panic::RefUnwindSafe + Send + 'static,
    F: FnOnce() -> O + Send + 'a,
{
    execute_safe(|| (func)())
}