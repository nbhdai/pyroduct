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

use crate::{
    errors::{FfiError, Phase},
    module_capability::{
        access::slice_to_wasm_slice,
        panic::{clear_last_panic, recover_panic_info, register_ffi_panic_hook},
    },
};

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
pub unsafe extern "C" fn dealloc(ptr: *mut u8, size: usize) {
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

pub fn prepare_return_to_host<T: ToRow>(result: Result<T, String>) -> u64 {
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

            // PACKING: ptr (low 32 bits) | len (high 32 bits)
            let ptr_val = bytes_ptr as u32 as u64;
            let len_val = (len as u32 as u64) << 32;

            let packed = ptr_val | len_val;
            packed
        }
        Err(error) => make_error_output(error),
    }
}

pub fn make_error_output(error: FfiError) -> u64 {
    tracing::trace!(%error, "make_error_output: constructing error output");
    // If this serialization fails, we're in real trouble
    match rkyv::to_bytes::<rkyv::rancor::Error>(&error) {
        Ok(bytes) => {
            let pointer = slice_to_wasm_slice(&bytes);
            std::mem::forget(bytes);

            tracing::debug!(?pointer, "make_error_output: serialization successful");
            pointer
        }
        Err(e) => {
            // Last resort: return a simple error message
            let bytes = format!("{}", error).into_bytes();
            let pointer = slice_to_wasm_slice(&bytes);
            std::mem::forget(bytes);
            tracing::error!(error = ?e, "make_error_output: failed to serialize error output");
            pointer
        }
    }
}

pub fn call<'a, C, O, F>(input_ptr: *mut u8, input_len: usize, func: F) -> u64
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arrow_scalars::{ArrowRow, ArrowValue}; 
    use crate::errors::ArchivedFfiError;
    use crate::{DeepRef, FromRow, ToRow};
    
    struct UserInput {
        name: String,
        age: i32,
    }

    // Manually implementing these as the macro is messed up within pyroduct
    #[derive(Debug, Clone, PartialEq)]
    struct UserInputRef<'a> {
        name: &'a str,
        age: i32,
    }

    impl DeepRef for UserInput {
        type Ref<'a> = UserInputRef<'a>;

        fn as_deep_ref<'a>(&'a self) -> Self::Ref<'a> {
            UserInputRef {
                name: &self.name,
                age: self.age,
            }
        }
    }

    impl<'a> FromRow<'a> for UserInputRef<'a> {
        fn from_row(row: &ArrowRow<'a>) -> Result<Self, String> {
            let name_val = row.get("name").ok_or("Missing field 'name'")?;
            let age_val = row.get("age").ok_or("Missing field 'age'")?;

            let name = match name_val {
                ArrowValue::Str(c) => c.as_ref(),
                _ => return Err("Field 'name' is not a string".into()),
            };

            let age = match age_val {
                ArrowValue::I32(v) => *v,
                _ => return Err("Field 'age' is not an i32".into()),
            };

            // Lifetime reset to 'a
            let name = unsafe { std::mem::transmute(name) };

            Ok(UserInputRef { name, age })
        }
    }

    #[derive(Debug, Clone)]
    struct UserOutput {
        greeting: String,
        is_adult: bool,
    }

    impl ToRow for UserOutput {
        fn to_row(&self) -> ArrowRow<'_> {
            ArrowRow::from([
                ("greeting", ArrowValue::from(self.greeting.clone())),
                ("is_adult", ArrowValue::from(self.is_adult)),
            ])
        }
    }

    fn unpack_ptr(packed: u64) -> (*mut u8, usize) {
        let ptr = (packed & 0xFFFFFFFF) as usize;
        let len = (packed >> 32) as usize;
        (ptr as *mut u8, len)
    }
    
    #[test]
    fn test_ffi_call_happy_path() {
        let input_row = ArrowRow::from([
            ("name", ArrowValue::from("Alice")),
            ("age", ArrowValue::from(30i32)),
        ]);
        let input_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&input_row).unwrap();
        
        // This is memory managed by the wasm module, as we are both host and wasm in this test we hold it.
        let mut input_vec = input_bytes.into_vec();
        let input_ptr = input_vec.as_mut_ptr();
        let input_len = input_vec.len();
        

        let user_logic = |input: &UserInputRef<'_>| -> Result<UserOutput, String> {
            assert_eq!(input.name, "Alice");
            assert_eq!(input.age, 30);
            
            Ok(UserOutput {
                greeting: format!("Hello, {}!", input.name),
                is_adult: input.age >= 18,
            })
        };

        let packed_result = call::<UserInput, UserOutput, _>(
            input_ptr, 
            input_len, 
            user_logic
        );

        let (out_ptr, out_len) = unpack_ptr(packed_result);
        let out_slice = unsafe { std::slice::from_raw_parts(out_ptr, out_len) };
        type ReturnType<'a> = Result<ArrowRow<'a>, String>;
        let archived = rkyv::access::<<ReturnType as rkyv::Archive>::Archived, rkyv::rancor::Error>(out_slice)
            .expect("Failed to access result archive");
            
        let result: Result<ArrowRow, String> = rkyv::deserialize::<_, rkyv::rancor::Error>(archived).unwrap();

        match result {
            Ok(row) => {
                let greeting = row.get("greeting").expect("Missing greeting");
                assert_eq!(greeting, &ArrowValue::from("Hello, Alice!"));
                
                let is_adult = row.get("is_adult").expect("Missing is_adult");
                assert_eq!(is_adult, &ArrowValue::from(true));
            },
            Err(e) => panic!("FFI call returned logic error: {}", e),
        }

        unsafe { dealloc(out_ptr, out_len) }
        // input_vec is dropped here naturally
    }

    #[test]
    fn test_ffi_call_logic_panic() {
        let input_row = ArrowRow::from([
            ("name", ArrowValue::from("Bob")),
            ("age", ArrowValue::from(10i32)),
        ]);
        let input_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&input_row).unwrap();
        
        // Keep the vector alive - don't forget it
        let mut input_vec = input_bytes.into_vec();
        let input_ptr = input_vec.as_mut_ptr();
        let input_len = input_vec.len();
        // Don't forget - let it live through the test

        let user_logic = |_: &UserInputRef<'_>| -> Result<UserOutput, String> {
            panic!("Something went terribly wrong!");
        };

        let packed_result = call::<UserInput, UserOutput, _>(
            input_ptr, 
            input_len, 
            user_logic
        );

        let (out_ptr, out_len) = unpack_ptr(packed_result);
        let out_slice = unsafe { std::slice::from_raw_parts(out_ptr, out_len) };

        let archived = rkyv::access::<ArchivedFfiError, rkyv::rancor::Error>(out_slice)
            .expect("Output should be a valid FfiError archive");
        
        let error: crate::errors::FfiError = rkyv::deserialize::<_, rkyv::rancor::Error>(archived).unwrap();

        match error {
            crate::errors::FfiError::ModuleLogicPanicked(info) => {
                assert!(info.message.contains("Something went terribly wrong"));
            },
            _ => panic!("Expected ModuleLogicPanicked, got {:?}", error),
        }

        unsafe { dealloc(out_ptr, out_len) };
        // input_vec is dropped here naturally
    }
}