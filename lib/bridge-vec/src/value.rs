use arrow_scalars::{ArrowRow, ArrowValue};

use crate::{BridgeError, BridgeVec, Bridgeable, DataStatus, TypedBuf, header::BridgeHeaderMut};

impl<'a> Bridgeable for ArrowValue<'a> {
    fn serialize(&self) -> Result<BridgeVec, BridgeError> {
        // Safe as we have the same
        let mut vec = BridgeVec::serialize_from(self)?;
        vec.set_status(DataStatus::ValidData);
        Ok(vec)
    }

    fn deserialize(buf: &TypedBuf<Self>) -> Result<Self, BridgeError> {
        buf.deserialize()
    }

    fn unchecked_parse(vec: BridgeVec) -> Result<TypedBuf<Self>, BridgeError> {
        vec.unchecked_parse::<Self>()
    }
}

impl<'a> Bridgeable for ArrowRow<'a> {
    fn serialize(&self) -> Result<BridgeVec, BridgeError> {
        let mut vec = BridgeVec::serialize_from(self)?;
        vec.set_status(DataStatus::ValidData);
        Ok(vec)
    }

    fn deserialize(buf: &TypedBuf<Self>) -> Result<Self, BridgeError> {
        buf.deserialize()
    }

    fn unchecked_parse(vec: BridgeVec) -> Result<TypedBuf<Self>, BridgeError> {
        vec.unchecked_parse::<Self>()
    }
}


pub mod ffi {
    use std::panic::{self, AssertUnwindSafe};

    use arrow_scalars::ToRow;
    use tracing::trace;

    use crate::{BridgeError, Bridgeable, DataStatus, ffi::{clear_last_panic, recover_panic_info, register_ffi_panic_hook}, header::BridgeHeaderMut};

/// Executes a closure that returns a `ToRow` type.
/// The result is converted to an ArrowRow and serialized immediately,
/// allowing the ArrowRow to borrow from the stack-allocated result of `func`.
#[track_caller]
pub fn execute_value_safe<F, O>(func: F) -> *const u8
where
    O: ToRow + std::panic::RefUnwindSafe,
    F: FnOnce() -> O + std::panic::UnwindSafe,
{
    register_ffi_panic_hook();
    clear_last_panic();

    // We must execute logic, conversion, and serialization inside the same catch_unwind
    // scope. This is because `val` lives on the stack here, `row` borrows `val`,
    // and we must finish serialization before `val` drops.
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        let val = func();
        let row = val.to_row();
        
        // ArrowRow is Bridgeable, so we serialize it directly.
        match row.serialize() {
            Ok(mut vec) => {
                vec.set_status(DataStatus::ValidData);
                vec
            }
            Err(e) => e.encode(),
        }
    }));

    match result {
        Ok(bridge_vec) => bridge_vec.into_raw(),
        Err(_) => {
            let panic_info = recover_panic_info();
            trace!(panic = ?panic_info, "execute_value_safe: panic caught");
            BridgeError::CodePanic(panic_info).encode().into_raw()
        }
    }
}

/// Executes a closure that returns a `Result<ToRow, Bridgeable>`.
/// - On `Ok(val)`: `val` is converted to `ArrowRow` (borrowing from stack) and serialized.
/// - On `Err(e)`: `e` is serialized directly as a Bridgeable error.
#[track_caller]
pub fn execute_value_result_safe<F, O, E>(func: F) -> *const u8
where
    O: ToRow + std::panic::RefUnwindSafe,
    E: Bridgeable + std::panic::RefUnwindSafe,
    F: FnOnce() -> Result<O, E> + std::panic::UnwindSafe,
{
    register_ffi_panic_hook();
    clear_last_panic();

    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        match func() {
            Ok(val) => {
                let row = val.to_row();
                
                // Success path: Serialize the borrowed row
                match row.serialize() {
                    Ok(mut vec) => {
                        vec.set_status(DataStatus::ValidData);
                        vec
                    }
                    Err(e) => e.encode(),
                }
            }
            Err(e) => {
                // Error path: Serialize the error directly
                // Note: Bridgeable::serialize returns Result<BridgeVec, BridgeError>
                match e.serialize() {
                    Ok(vec) => vec, 
                    Err(ser_err) => ser_err.encode(),
                }
            }
        }
    }));

    match result {
        Ok(bridge_vec) => bridge_vec.into_raw(),
        Err(_) => {
            let panic_info = recover_panic_info();
            trace!(panic = ?panic_info, "execute_value_result_safe: panic caught");
            BridgeError::CodePanic(panic_info).encode().into_raw()
        }
    }
}
}