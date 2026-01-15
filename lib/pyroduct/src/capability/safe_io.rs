use std::{ffi::c_void, mem, panic, slice};

use rkyv::{
    Archive, Deserialize, Serialize,
    bytecheck::CheckBytes,
    de::Pool,
    rancor::{self, Strategy},
    ser::{Serializer, allocator::ArenaHandle, sharing::Share},
    util::AlignedVec,
    validation::{Validator, archive::ArchiveValidator, shared::SharedValidator},
};
use tracing::{debug, error, trace};

use crate::{
    capability_host::ffi::{COutput, FfiResult},
    errors::{FfiError, Phase},
    module_capability::panic::{clear_last_panic, recover_panic_info},
};

/// Updated get_input function that returns Result instead of Option
#[tracing::instrument]
pub unsafe fn get_input<T>(ptr: *const u8, len: usize) -> Result<T, FfiError>
where
    T: Archive,
    // Validate the archived bytes are safe
    for<'b> <T as Archive>::Archived:
        CheckBytes<Strategy<Validator<ArchiveValidator<'b>, SharedValidator>, rancor::Error>>,
    // Deserialize back to the native type
    for<'b> <T as Archive>::Archived: Deserialize<T, Strategy<Pool, rkyv::rancor::Error>>,
{
    trace!("get_input: processing input");
    if ptr.is_null() {
        error!("get_input: capability input pointer is null");
        return Err(FfiError::NullPointer(Phase::Input));
    }
    if len == 0 {
        error!("get_input: capability input length is zero");
        return Err(FfiError::ZeroLength(Phase::Input));
    }
    debug!(len, "get_input: processing bytes");
    let slice = unsafe { slice::from_raw_parts(ptr, len) };

    deserialize(slice, Phase::Input)
}

#[tracing::instrument]
pub unsafe fn get_client_state<T>(ptr: *const u8, len: usize) -> Result<T, FfiError>
where
    T: Archive,
    // Validate the archived bytes are safe
    for<'b> <T as Archive>::Archived:
        CheckBytes<Strategy<Validator<ArchiveValidator<'b>, SharedValidator>, rancor::Error>>,
    // Deserialize back to the native type
    for<'b> <T as Archive>::Archived: Deserialize<T, Strategy<Pool, rkyv::rancor::Error>>,
{
    trace!("get_client_state: processing client state");
    if ptr.is_null() {
        error!("get_client_state: client state pointer is null");
        return Err(FfiError::NullPointer(Phase::Client));
    }
    if len == 0 {
        error!("get_client_state: client state length is zero");
        return Err(FfiError::ZeroLength(Phase::Client));
    }
    debug!(len, "get_client_state: processing bytes");
    let slice = unsafe { slice::from_raw_parts(ptr, len) };

    deserialize(slice, Phase::Client)
}

fn deserialize<T>(slice: &[u8], phase: Phase) -> Result<T, FfiError>
where
    T: Archive,
    // Validate the archived bytes are safe
    for<'b> <T as Archive>::Archived:
        CheckBytes<Strategy<Validator<ArchiveValidator<'b>, SharedValidator>, rancor::Error>>,
    // Deserialize back to the native type
    for<'b> <T as Archive>::Archived: Deserialize<T, Strategy<Pool, rkyv::rancor::Error>>,
{
    clear_last_panic();
    panic::catch_unwind(|| {
        let archived = rkyv::access::<<T as rkyv::Archive>::Archived, rkyv::rancor::Error>(slice)
            .map_err(|e| {
            error!(error = ?e, %phase, "deserialize: validation failed");
            FfiError::ValidationFailed(format!("{e:?}"), phase)
        })?;

        rkyv::deserialize::<_, rancor::Error>(archived).map_err(|e| {
            error!(error = ?e, %phase, "deserialize: deserialization failed");
            FfiError::DeserializationFailed(format!("{e:?}"), phase)
        })
    })
    .map_err(|_| {
        let panic = recover_panic_info();
        error!(panic = ?panic, %phase, "deserialize: panic during deserialization");
        FfiError::DeserializationPanicked(panic, phase)
    })
    .flatten()
}

/// Updated borrow_input function that returns Result instead of Option
#[tracing::instrument]
pub unsafe fn borrow_input<'a, T>(
    ptr: *const u8,
    len: usize,
) -> Result<&'a <T as Archive>::Archived, FfiError>
where
    T: Archive,
    for<'b> <T as Archive>::Archived:
        CheckBytes<Strategy<Validator<ArchiveValidator<'b>, SharedValidator>, rancor::Error>>,
    for<'b> <T as Archive>::Archived: Deserialize<T, Strategy<Pool, rkyv::rancor::Error>>,
{
    trace!("borrow_input: attempting zero-copy access");
    if ptr.is_null() {
        error!("borrow_input: input pointer is null");
        return Err(FfiError::NullPointer(Phase::Input));
    }
    if len == 0 {
        error!("borrow_input: input length is zero");
        return Err(FfiError::ZeroLength(Phase::Input));
    }

    debug!(len, "borrow_input: accessing bytes");
    let slice = unsafe { slice::from_raw_parts(ptr, len) };

    clear_last_panic();
    panic::catch_unwind(|| {
        rkyv::access::<<T as rkyv::Archive>::Archived, rkyv::rancor::Error>(slice).map_err(|e| {
            error!(error = ?e, "borrow_input: validation failed");
            FfiError::ValidationFailed(format!("{e:?}"), Phase::Input)
        })
    })
    .map_err(|_| {
        let panic = recover_panic_info();
        error!(panic = ?panic, "borrow_input: panic during access");
        FfiError::DeserializationPanicked(panic, Phase::Input)
    })
    .flatten()
}

#[tracing::instrument]
pub unsafe fn get_capability_state<'a, T>(
    host_state_ptr: *mut c_void,
) -> Result<&'a mut T, FfiError> {
    if host_state_ptr.is_null() {
        error!("get_capability_state: host_state_ptr is null");
        return Err(FfiError::NullPointer(Phase::State));
    }
    Ok(unsafe { &mut *(host_state_ptr as *mut T) })
}

/// Updated make_output function that returns Result instead of Option
pub unsafe fn make_output<T>(value: &T) -> FfiResult
where
    T: Archive + std::panic::RefUnwindSafe,
    for<'a> T:
        Serialize<Strategy<Serializer<AlignedVec, ArenaHandle<'a>, Share>, rkyv::rancor::Error>>,
{
    trace!("make_output: starting serialization");
    clear_last_panic();

    match panic::catch_unwind(|| {
        rkyv::to_bytes::<rkyv::rancor::Error>(value).map_err(|e| {
            error!(error = ?e, "make_output: serialization failed");
            FfiError::SerializationFailed(format!("{e:?}"), Phase::Output)
        })
    })
    .map_err(|_| -> FfiError {
        let panic = recover_panic_info();
        error!(panic = ?panic, "make_output: panic during serialization");
        FfiError::SerializationPanicked(panic, Phase::Output)
    })
    .flatten()
    {
        Ok(bytes) => {
            let (ptr, len, cap) = (bytes.as_ptr(), bytes.len(), bytes.capacity());
            debug!(?ptr, len, "make_output: serialization successful");
            mem::forget(bytes);
            FfiResult::ok(COutput { ptr, len, cap })
        }
        Err(err) => make_error_output(err),
    }
}

pub fn make_error_output(error: FfiError) -> FfiResult {
    trace!(%error, "make_error_output: constructing error output");
    // If this serialization fails, we're in real trouble
    match rkyv::to_bytes::<rkyv::rancor::Error>(&error) {
        Ok(bytes) => {
            let (ptr, len, cap) = (bytes.as_ptr(), bytes.len(), bytes.capacity());
            debug!(?ptr, len, "make_error_output: serialization successful");
            mem::forget(bytes);
            FfiResult::full_err(COutput { ptr, len, cap })
        }
        Err(e) => {
            // Last resort: return a simple error message
            error!(error = ?e, "make_error_output: failed to serialize error output");
            let msg = format!("{}", error);
            let mut bytes = msg.into_bytes();
            let (ptr, len, cap) = (bytes.as_mut_ptr(), bytes.len(), bytes.capacity());
            mem::forget(bytes);
            FfiResult::partial_error(COutput { ptr, len, cap })
        }
    }
}
