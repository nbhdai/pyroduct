use std::{panic, slice, any::Any};
use tracing::{error, trace};
use rkyv::{
    Archive, Deserialize, Serialize, bytecheck::CheckBytes, rancor::{Error, Strategy}, ser::{Serializer, allocator::{Arena, ArenaHandle}, sharing::Share}, validation::{Validator, archive::ArchiveValidator, shared::SharedValidator}
};

use crate::LenAlignedVec;

// --- Error Definitions ---

#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
pub enum RkyvFfiError {
    NullPointer,
    ZeroLength,
    RemoteSerializationPanic(String),
    RemoteUserErrorSerializationFailed(String),
    ValidationFailed(String),
    SystemErrorDeserializationFailed(String),
    UnknownStatus(u16), // Updated to u16
    RawUtf8(String),
}

impl std::fmt::Display for RkyvFfiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for RkyvFfiError {}

// --- Status Codes ---

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataStatus {
    ValidData = 0,
    UserError = 1,
    RkyvFfiError = 2,
    Utf8Error = 3,
    ValidUtf8 = 4,
}

fn panic_payload_to_string(payload: Box<dyn Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "Box<Any> (Unknown panic payload)".to_string()
    }
}

// --- Implementation ---

impl LenAlignedVec {
    /// Serializes a Result<T, E> for the FFI boundary.
    pub fn serialize_result<T, E>(result: Result<&T, &E>) -> Self
    where
        T: Archive + std::panic::RefUnwindSafe,
        E: Archive + std::panic::RefUnwindSafe,
        for<'a, 'b> T: Serialize<Strategy<Serializer<&'a mut LenAlignedVec, &'b mut Arena, &'b mut Share>, rkyv::rancor::Error>>,
        for<'a, 'b> E: Serialize<Strategy<Serializer<&'a mut LenAlignedVec, &'b mut Arena, &'b mut Share>, rkyv::rancor::Error>>,
    {
        trace!("serialize_result: starting");
        
        let panic_guard = panic::catch_unwind(|| {
            match result {
                Ok(val) => {
                    match Self::serialize_from(val) {
                        Ok(mut vec) => {
                            // Uses lib.rs set_status (u16 offset 14)
                            vec.set_status(DataStatus::ValidData as u16);
                            vec
                        }
                        Err(e) => {
                            let err = RkyvFfiError::RemoteSerializationPanic(format!("Failed to serialize Ok variant: {:?}", e));
                            Self::serialize_system_error(err)
                        }
                    }
                }
                Err(val) => {
                    match Self::serialize_from(val) {
                        Ok(mut vec) => {
                            vec.set_status(DataStatus::UserError as u16);
                            vec
                        }
                        Err(e) => {
                            let err = RkyvFfiError::RemoteUserErrorSerializationFailed(format!("Failed to serialize Err variant: {:?}", e));
                            Self::serialize_system_error(err)
                        }
                    }
                }
            }
        });

        match panic_guard {
            Ok(vec) => vec,
            Err(payload) => {
                let msg = panic_payload_to_string(payload);
                error!(panic = %msg, "serialize_result: panic during serialization");
                let err = RkyvFfiError::RemoteSerializationPanic(msg);
                Self::serialize_system_error(err)
            }
        }
    }

    fn serialize_system_error(err: RkyvFfiError) -> Self 
    where for<'a> RkyvFfiError: Serialize<Strategy<Serializer<&'a mut LenAlignedVec, ArenaHandle<'a>, Share>, Error>>
    {
        match Self::serialize_from(&err) {
            Ok(mut vec) => {
                vec.set_status(DataStatus::RkyvFfiError as u16);
                vec
            }
            Err(e) => {
                let msg = format!("Critical: Failed to serialize RkyvFfiError ({:?}) | Source: {:?}", err, e);
                let mut vec = LenAlignedVec::with_capacity(msg.len());
                vec.extend_from_slice(msg.as_bytes());
                vec.set_status(DataStatus::Utf8Error as u16);
                vec
            }
        }
    }
}

// --- FFI Access ---

/// Reads a result from the FFI boundary.
#[tracing::instrument]
pub unsafe fn access_from_ffi<'a, T, E>(ptr: *const u8) -> Result<Result<&'a T::Archived, &'a E::Archived>, RkyvFfiError>
where
    T: Archive,
    E: Archive,
    for<'b> <T as Archive>::Archived: CheckBytes<Strategy<Validator<ArchiveValidator<'b>, SharedValidator>, rkyv::rancor::Error>>,
    for<'b> <E as Archive>::Archived: CheckBytes<Strategy<Validator<ArchiveValidator<'b>, SharedValidator>, rkyv::rancor::Error>>,
{
    // Reconstruct wrapper to access header safely
    let vec_ref = match unsafe { LenAlignedVec::from_raw(ptr) } {
        Ok(v) => std::mem::ManuallyDrop::new(v), // Don't drop, we don't own it
        Err(_) => return Err(RkyvFfiError::NullPointer),
    };

    let status_val = vec_ref.status();
    let len = vec_ref.len();
    let data_ptr = vec_ref.data_ptr();
    let slice = unsafe { slice::from_raw_parts(data_ptr, len) };

    match status_val {
        0 => { // ValidData
            rkyv::access::<T::Archived, Error>(slice)
                .map(Ok)
                .map_err(|e| RkyvFfiError::ValidationFailed(format!("T validation failed: {:?}", e)))
        }
        1 => { // UserError
            rkyv::access::<E::Archived, Error>(slice)
                .map(Err)
                .map_err(|e| RkyvFfiError::ValidationFailed(format!("E validation failed: {:?}", e)))
        }
        2 => { // RkyvFfiError
            let archived_err = rkyv::access::<ArchivedRkyvFfiError, Error>(slice)
                .map_err(|e| RkyvFfiError::ValidationFailed(format!("System error validation failed: {:?}", e)))?;
            
            let deserialized: RkyvFfiError = rkyv::deserialize(archived_err)
                .map_err(|e: Error| RkyvFfiError::SystemErrorDeserializationFailed(format!("{:?}", e)))?;
                
            Err(deserialized)
        }
        3 => { // Utf8Error
            let msg = std::str::from_utf8(slice).unwrap_or("<Invalid UTF8>");
            Err(RkyvFfiError::RawUtf8(msg.to_string()))
        }
        _ => Err(RkyvFfiError::UnknownStatus(status_val)),
    }
}