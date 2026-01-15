// Capability functions
pub mod logger;
pub mod safe_async;
pub mod safe_call;
pub mod safe_io;
pub mod safe_lifecycle;
pub use logger::init_logging;

pub use serde;

use crate::{
    capability::safe_io::make_error_output,
    capability_host::ffi::{COutput, FfiBorrowedFutureResult, FfiResult},
    errors::FfiError,
};

#[cfg(test)]
mod safety_tests;

impl From<Result<COutput, FfiError>> for FfiResult {
    fn from(result: Result<COutput, FfiError>) -> Self {
        match result {
            Ok(output) => FfiResult::ok(output),
            Err(error) => make_error_output(error),
        }
    }
}

impl From<FfiError> for FfiResult {
    fn from(error: FfiError) -> Self {
        make_error_output(error)
    }
}

impl<'a> From<FfiError> for FfiBorrowedFutureResult<'a> {
    fn from(error: FfiError) -> Self {
        FfiBorrowedFutureResult::EarlyError(make_error_output(error))
    }
}
