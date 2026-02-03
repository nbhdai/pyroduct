use std::{ffi::c_void, panic, slice};

use bridge_vec::{BridgeVec, DataStatus, ffi::RkyvFfiError, ser_de::Bridgable};
use tracing::{debug, error, trace};

use crate::{
    errors::{FfiError, Phase},
    module_capability::panic::{clear_last_panic, recover_panic_info},
};

/// Updated get_input function that returns Result instead of Option
#[tracing::instrument]
pub unsafe fn get_input<T: Bridgable>(ptr: *const u8, len: usize) -> Result<T, FfiError> {
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
pub unsafe fn get_client_state<T: Bridgable>(ptr: *const u8, len: usize) -> Result<T, FfiError> {
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

fn deserialize<T: Bridgable>(slice: &[u8], phase: Phase) -> Result<T, FfiError> {
    clear_last_panic();
    panic::catch_unwind(|| {
        let archived = rkyv::access::<<T as rkyv::Archive>::Archived, rkyv::rancor::Error>(slice)
            .map_err(|e| {
                error!(error = ?e, %phase, "deserialize: validation failed");
                FfiError::ValidationFailed(format!("{e:?}"), phase)
            })?;

        rkyv::deserialize::<_, rkyv::rancor::Error>(archived).map_err(|e| {
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
pub unsafe fn borrow_input<'a, T: Bridgable>(
    ptr: *const u8,
    len: usize,
) -> Result<&'a <T as rkyv::Archive>::Archived, FfiError> {
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

/// Serialize a successful output value into a BridgeVec
pub fn make_output<T: Bridgable + std::panic::RefUnwindSafe>(value: &T) -> BridgeVec {
    trace!("make_output: starting serialization");
    clear_last_panic();

    match panic::catch_unwind(|| {
        BridgeVec::serialize_from(value).map_err(|e| {
            error!(error = ?e, "make_output: serialization failed");
            FfiError::SerializationFailed(format!("{e:?}"), Phase::Output)
        })
    }) {
        Ok(Ok(mut vec)) => {
            debug!(len = vec.len(), "make_output: serialization successful");
            vec.set_status(DataStatus::ValidData as u16);
            vec
        }
        Ok(Err(err)) => {
            error!(%err, "make_output: serialization error");
            make_error_output(err)
        }
        Err(_) => {
            let panic = recover_panic_info();
            error!(panic = ?panic, "make_output: panic during serialization");
            make_error_output(FfiError::SerializationPanicked(panic, Phase::Output))
        }
    }
}

/// Serialize an FfiError into a BridgeVec with TransportError status
pub fn make_error_output(error: FfiError) -> BridgeVec {
    trace!(%error, "make_error_output: constructing error output");
    
    let rkyv_error = RkyvFfiError::from(error);
    
    match BridgeVec::serialize_from(&rkyv_error) {
        Ok(mut vec) => {
            debug!(len = vec.len(), "make_error_output: serialization successful");
            vec.set_status(DataStatus::TransportError as u16);
            vec
        }
        Err(e) => {
            error!(error = ?e, "make_error_output: failed to serialize error output");
            let msg = format!("{:?}", rkyv_error);
            let mut vec = BridgeVec::with_capacity(msg.len());
            vec.extend_from_slice(msg.as_bytes());
            vec.set_status(DataStatus::Utf8Error as u16);
            vec
        }
    }
}