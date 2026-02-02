pub mod logging;

pub use tracing;

use arrow_scalars::{ArrowRow, DeepRef, FromRow, ToRow};
use core::slice;

use std::cell::RefCell;
use std::{
    mem,
    panic::{self, AssertUnwindSafe},
};
use thiserror::Error;

use crate::{
    errors::{FfiError, Phase},
    module_capability::{
        panic::{clear_last_panic, recover_panic_info, register_ffi_panic_hook},
    },
};


#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Debug, Error)]
pub enum CapabilityIoError {
    #[error("Capability {0}: call serialization failed {1}")]
    Serialization(String, String),
    #[error("Capability {0}: return verification failed {1}")]
    Verification(String, String),
    #[error("Capability {0}: return deserializtion failed {1}")]
    Deserialization(String, String),
}

thread_local! {
    static LAST_ERROR: RefCell<Option<CapabilityIoError>> = RefCell::new(None);
}

pub fn set_last_error(error: CapabilityIoError) {
    LAST_ERROR.with(|e| {
        *e.borrow_mut() = Some(error);
    });
}

pub fn take_last_error() -> Option<CapabilityIoError> {
    LAST_ERROR.with(|e| e.borrow_mut().take())
}

pub fn clear_last_error() {
    LAST_ERROR.with(|e| {
        *e.borrow_mut() = None;
    });
}


#[unsafe(no_mangle)]
pub unsafe extern "C" fn alloc(size: usize) -> *mut u8 {
    // Calculate capacity in u128 units (16 bytes)
    let count = (size + 15) / 16;
    let mut buf: Vec<u128> = Vec::with_capacity(count);
    let ptr = buf.as_mut_ptr() as *mut u8;
    mem::forget(buf);
    ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dealloc(ptr: *const u8, size: usize) {
    let count = (size + 15) / 16;
    unsafe {
        let _ = Vec::from_raw_parts(ptr as *mut u128, 0, count);
    }
}

pub unsafe fn parse_call_from_host<'a, T>(
    input_ptr: *mut u8,
    input_len: usize,
) -> Result<T::Ref<'a>, FfiError>
where
    T: DeepRef,
    T::Ref<'a>: FromRow<'a>,
{
    let input_slice = unsafe { slice::from_raw_parts(input_ptr, input_len) };

    let archived = panic::catch_unwind(AssertUnwindSafe(|| {
        rkyv::access::<arrow_scalars::ArchivedArrowRow, rkyv::rancor::Error>(input_slice)
            .map_err(|err| FfiError::ValidationFailed(err.to_string(), Phase::Call))
    }))
    .map_err(|_| {
        let panic_info = recover_panic_info();
        tracing::error!(panic = ?panic_info, "exter_call: panic during deserialization");
        FfiError::DeserializationPanicked(panic_info, Phase::Call)
    })
    .flatten()?;

    let row: ArrowRow<'a> = archived.into();

    <T::Ref<'a>>::from_row(&row).map_err(FfiError::ToRowFailed)
}

pub type ReturnToHost<'a> = Result<ArrowRow<'a>, String>;

pub fn prepare_return_to_host<T: ToRow>(result: Result<T, String>) -> (*const u8, u32) {
    let result = panic::catch_unwind(AssertUnwindSafe(|| {match result {
            Ok(result_row) => rkyv::to_bytes::<rkyv::rancor::Error>(&Result::<ArrowRow, String>::Ok(result_row.to_row())),
            Err(error) => rkyv::to_bytes::<rkyv::rancor::Error>(&Result::<ArrowRow, String>::Err(error)),
        }}.map_err(|e| {
            tracing::error!(error = ?e, "prepare_return_to_host: serialization failed");
            FfiError::SerializationFailed(format!("{e:?}"), Phase::Call)
        })
        )).map_err(|_| {
            let panic_info = recover_panic_info();
            tracing::error!(panic = ?panic_info, "prepare_return_to_host: panic during deserialization");
            FfiError::SerializationPanicked(panic_info, Phase::Call)
        })
        .flatten();

    match result {
        Ok(bytes) => {
            let bytes_ptr = bytes.as_ptr();
            let len = bytes.len();

            mem::forget(bytes);
            (bytes_ptr, len as u32)
        }
        Err(error) => make_error_output(error),
    }
}

pub fn make_error_output(error: FfiError) -> (*const u8, u32) {
    tracing::trace!(%error, "make_error_output: constructing error output");
    // If this serialization fails, we're in real trouble
    match rkyv::to_bytes::<rkyv::rancor::Error>(&error) {
        Ok(bytes) => {
            let bytes_ptr = bytes.as_ptr();
            let len = bytes.len();

            std::mem::forget(bytes);

            tracing::debug!(len, ?bytes_ptr, "make_error_output: serialization successful");
            (bytes_ptr, len as u32)
        }
        Err(e) => {
            // Last resort: return a simple error message
            let bytes = format!("{}", error).into_bytes();

            let bytes_ptr = bytes.as_ptr();
            let len = bytes.len();
            tracing::error!(error = ?e, "make_error_output: failed to serialize error output");
            (bytes_ptr, len as u32)
        }
    }
}

pub fn call<'a, C, O, F>(input_ptr: *mut u8, input_len: usize, func: F) -> (*const u8, u32)
where
    C: DeepRef + 'a,
    C::Ref<'a>: FromRow<'a>,
    O: ToRow,
    F: FnOnce(&C::Ref<'a>) -> Result<O, String>,
{
    logging::init_logging();
    register_ffi_panic_hook();
    clear_last_panic();
    clear_last_error();

    let input = match unsafe { parse_call_from_host::<C>(input_ptr, input_len) } {
        Ok(row) => row,
        Err(error) => return make_error_output(error),
    };

    let logic_result = match panic::catch_unwind(AssertUnwindSafe(|| (func)(&input))) {
        Ok(logic_result) => logic_result,
        Err(_) => {
            let panic = recover_panic_info();
            if let Some(error) = crate::module_capability::error::take_last_error() {
                return make_error_output(error);
            } else {
                return make_error_output(FfiError::ModuleLogicPanicked(panic));
            }
        }
    };

    prepare_return_to_host(logic_result)
}
