pub mod logging;
pub use tracing;

use arrow_scalars::{ArrowRow, DeepRef, FromRow, ToRow};

use std::cell::RefCell;
use std::ops::Deref;
use std::panic::{RefUnwindSafe};
use std::mem;
use thiserror::Error;

use bridge_vec::{BridgeError, BridgeVec, CapturedError};


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

pub unsafe fn parse_call_from_host<'a, T>(data: BridgeVec) -> Result<T::Ref<'a>, BridgeError>
where
    T: DeepRef,
    T::Ref<'a>: FromRow<'a>,
{
    let typed_data = data.parse::<ArrowRow>()?;

    let row: ArrowRow<'a> = typed_data.deref().into();

    <T::Ref<'a>>::from_row(&row).map_err(|error| BridgeError::CodePanic(CapturedError::new(format!("To Row failed {error}")).into()))
}

pub type ReturnToHost<'a> = Result<ArrowRow<'a>, String>;

#[track_caller]
pub fn call<'a, C, O, F>(input_ptr: *mut u8, func: F) -> *const u8
where
    C: DeepRef + 'a,
    <C as DeepRef>::Ref<'a>: FromRow<'a> + RefUnwindSafe,
    O: ToRow + RefUnwindSafe,
    F: FnOnce(&C::Ref<'a>) -> Result<O, String> + std::panic::UnwindSafe,
{
    logging::init_logging();
    #[cfg(target_arch = "wasm32")]
    bridge_vec::ffi::register_ffi_panic_hook();
    clear_last_error();

    let data: BridgeVec = match unsafe { BridgeVec::from_raw(input_ptr) } {
        Ok(data) => data,
        Err(err) => return err.encode().into_raw(),
    };

    let input = match unsafe { parse_call_from_host::<C>(data) } {
        Ok(row) => row,
        Err(err) => return err.encode().into_raw(),
    };
    bridge_vec::value::ffi::execute_value_result_safe(|| (func)(&input))
}
