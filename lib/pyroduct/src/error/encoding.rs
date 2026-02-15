use std::panic::Location;

use crate::{
    CapturedError, PyroError, PyroVec,
    error::ErrorKind,
    header::{DataStatus, PyroHeaderMut},
};

impl PyroError {
    // Users should use the ffi code that does this for them, or should read the codebase to understand.
    #[doc(hidden)]
    #[track_caller]
    pub fn encode(&self) -> PyroVec {
        match self {
            PyroError::IncorrectParse(_) => {
                let mut err_vec=  CapturedError::new("Encoding a PyroError that is an unhandled user deserialization error, not a pyro error").with_location(Location::caller()).encode();
                err_vec.set_status(DataStatus::CodeError);
                err_vec
            }
            PyroError::CodePanic(err) => {
                let mut vec = err.encode();
                vec.set_status(DataStatus::CodeError);
                vec
            }
            PyroError::NotFound(msg) => {
                let mut err_vec = CapturedError::new(msg)
                    .with_location(Location::caller())
                    .encode();
                err_vec.set_status(DataStatus::CodeError);
                err_vec
            }
            PyroError::Header(error) => {
                let mut err_vec = CapturedError::new(error.to_string())
                    .with_location(Location::caller())
                    .encode();
                err_vec.set_status(DataStatus::RemoteInvalidHeader);
                err_vec
            }
            PyroError::Pyro { kind, .. } => {
                let error: Option<&Box<CapturedError>> = match kind {
                    ErrorKind::Serialization(error_payload) => Some(error_payload),
                    ErrorKind::Deserialization(error_payload) => Some(error_payload),
                    ErrorKind::Validation(error_payload) => Some(error_payload),
                    ErrorKind::Transport(error_payload) => Some(error_payload),
                    ErrorKind::Io(io_payload) => Some(io_payload.into()),
                    ErrorKind::Utf8(utf8_payload) => Some(utf8_payload.into()),
                    ErrorKind::InvalidHeader => None,
                    ErrorKind::LayoutError => None,
                    ErrorKind::UnexpectedEof => None,
                };
                let status_code = kind.to_status().to_remote();

                let mut vec = match error {
                    Some(error) => error.encode(),
                    // We overwrite it with our status code immediately
                    None => PyroVec::ok(),
                };

                vec.set_status(status_code);
                vec
            }
            PyroError::HeaderFfi(captured_error) => {
                let mut vec = captured_error.encode();
                vec.set_status(DataStatus::PyroFfiFail);
                vec
            }
        }
    }
}
