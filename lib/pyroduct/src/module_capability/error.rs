use std::cell::RefCell;
use thiserror::Error;

use crate::errors::{FfiError, Phase};

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Debug, Error)]
pub enum CapabilityIoError {
    #[error("Capability: call client serialization failed {0}")]
    ClientSerialization(String),
    #[error("Capability: call input serialization failed {0}")]
    InputSerialization(String),
    #[error("Capability: return verification failed {0}")]
    Verification(String),
    #[error("Capability: return deserializtion failed {0}")]
    Deserialization(String),
    #[error("Capability: Function on the other side failed")]
    Call,
}

thread_local! {
    static LAST_ERROR: RefCell<Option<FfiError>> = RefCell::new(None);
}

pub fn set_last_error(error: CapabilityIoError) {
    let error = match error {
        CapabilityIoError::ClientSerialization(msg) => {
            FfiError::SerializationFailed(msg, Phase::CapabilityClient)
        }
        CapabilityIoError::InputSerialization(msg) => {
            FfiError::SerializationFailed(msg, Phase::CapabilityInput)
        }
        CapabilityIoError::Verification(msg) => {
            FfiError::ValidationFailed(msg, Phase::CapabilityOutput)
        }
        CapabilityIoError::Deserialization(msg) => {
            FfiError::DeserializationFailed(msg, Phase::CapabilityOutput)
        }
        CapabilityIoError::Call => FfiError::HostSideCapability,
    };
    LAST_ERROR.with(|e| {
        *e.borrow_mut() = Some(error);
    });
}

pub fn take_last_error() -> Option<FfiError> {
    LAST_ERROR.with(|e| e.borrow_mut().take())
}

pub fn clear_last_error() {
    LAST_ERROR.with(|e| {
        *e.borrow_mut() = None;
    });
}
