use std::panic::{self, AssertUnwindSafe, RefUnwindSafe};

use rkyv::{
    Archive, Deserialize, Serialize,
    bytecheck::CheckBytes,
    de::Pool,
    rancor::{self, Strategy},
    ser::{Serializer, allocator::ArenaHandle, sharing::Share},
    util::AlignedVec,
    validation::{Validator, archive::ArchiveValidator, shared::SharedValidator},
};
use tracing::{debug, trace};

use crate::{
    capability::safe_io,
    errors::FfiError,
    module_capability::panic::{clear_last_panic, recover_panic_info},
};

pub(crate) fn execute_safe<F, O>(func: F) -> FfiResult
where
    O: RefUnwindSafe + Send + 'static,
    O: Archive + std::panic::RefUnwindSafe + Send + 'static,
    for<'c> O:
        Serialize<Strategy<Serializer<AlignedVec, ArenaHandle<'c>, Share>, rkyv::rancor::Error>>,
    F: FnOnce() -> O,
{
    clear_last_panic();
    match panic::catch_unwind(AssertUnwindSafe(|| (func)())) {
        Ok(logic_result) => {
            debug!("execute_safe: logic completed successfully");
            unsafe { safe_io::make_output(&logic_result) }
        }
        Err(_) => {
            let panic = recover_panic_info();
            trace!(panic = ?panic, "execute_safe: panic caught during async execution");
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
) -> FfiResult
where
    S: Send + 'a,
    C: Archive + Send + 'a,
    for<'b> <C as Archive>::Archived:
        CheckBytes<Strategy<Validator<ArchiveValidator<'b>, SharedValidator>, rancor::Error>>,
    for<'b> <C as Archive>::Archived: Deserialize<C, Strategy<Pool, rkyv::rancor::Error>>,
    I: Archive + Send + 'a,
    for<'b> <I as Archive>::Archived:
        CheckBytes<Strategy<Validator<ArchiveValidator<'b>, SharedValidator>, rancor::Error>>,
    for<'b> <I as Archive>::Archived: Deserialize<I, Strategy<Pool, rkyv::rancor::Error>>,
    O: Archive + std::panic::RefUnwindSafe + Send + 'static,
    for<'c> O:
        Serialize<Strategy<Serializer<AlignedVec, ArenaHandle<'c>, Share>, rkyv::rancor::Error>>,
    F: FnOnce(&'a mut S, C, I) -> O + Send + 'a,
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

    execute_safe(|| (func)(state, client, input))
}

/// Complete call with state, client, and input
pub fn sc_call<'a, S, C, O, F>(
    client_state_ptr: *const u8,
    client_state_len: usize,
    _input_ptr: *const u8,
    _input_len: usize,
    host_state_ptr: *mut std::ffi::c_void,
    func: F,
) -> FfiResult
where
    S: Send + 'a,
    C: Archive + Send + 'a,
    for<'b> <C as Archive>::Archived:
        CheckBytes<Strategy<Validator<ArchiveValidator<'b>, SharedValidator>, rancor::Error>>,
    for<'b> <C as Archive>::Archived: Deserialize<C, Strategy<Pool, rkyv::rancor::Error>>,
    O: Archive + std::panic::RefUnwindSafe + Send + 'static,
    for<'c> O:
        Serialize<Strategy<Serializer<AlignedVec, ArenaHandle<'c>, Share>, rkyv::rancor::Error>>,
    F: FnOnce(&'a mut S, C) -> O + Send + 'a,
{
    let state = match unsafe { safe_io::get_capability_state::<'a, S>(host_state_ptr) } {
        Ok(state) => state,
        Err(error) => return error.into(),
    };

    let client: C = match unsafe { safe_io::get_input::<C>(client_state_ptr, client_state_len) } {
        Ok(client) => client,
        Err(error) => return error.into(),
    };
    execute_safe(|| (func)(state, client))
}

/// Complete call with state, client, and input
pub fn i_call<'a, I, O, F>(
    _client_state_ptr: *const u8,
    _client_state_len: usize,
    input_ptr: *const u8,
    input_len: usize,
    _host_state_ptr: *mut std::ffi::c_void,
    func: F,
) -> FfiResult
where
    I: Archive + Send + 'a,
    for<'b> <I as Archive>::Archived:
        CheckBytes<Strategy<Validator<ArchiveValidator<'b>, SharedValidator>, rancor::Error>>,
    for<'b> <I as Archive>::Archived: Deserialize<I, Strategy<Pool, rkyv::rancor::Error>>,
    O: Archive + std::panic::RefUnwindSafe + Send + 'static,
    for<'c> O:
        Serialize<Strategy<Serializer<AlignedVec, ArenaHandle<'c>, Share>, rkyv::rancor::Error>>,
    F: FnOnce(I) -> O + Send + 'a,
{
    let input: I = match unsafe { safe_io::get_input::<I>(input_ptr, input_len) } {
        Ok(input) => input,
        Err(error) => return error.into(),
    };
    execute_safe(|| (func)(input))
}

/// Complete call with state, client, and input
pub fn empty_call<'a, O, F>(
    _client_state_ptr: *const u8,
    _client_state_len: usize,
    _input_ptr: *const u8,
    _input_len: usize,
    _host_state_ptr: *mut std::ffi::c_void,
    func: F,
) -> FfiResult
where
    O: Archive + std::panic::RefUnwindSafe + Send + 'static,
    for<'c> O:
        Serialize<Strategy<Serializer<AlignedVec, ArenaHandle<'c>, Share>, rkyv::rancor::Error>>,
    F: FnOnce() -> O + Send + 'a,
{
    execute_safe(|| (func)())
}
