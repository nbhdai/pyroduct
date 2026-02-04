use crate::module_capability::error;
use rkyv::{
    Archive, Deserialize, Serialize,
    bytecheck::CheckBytes,
    de::Pool,
    rancor::{self, Error as RkyvError, Strategy},
    ser::{Serializer, allocator::ArenaHandle, sharing::Share},
    validation::{Validator, archive::ArchiveValidator, shared::SharedValidator},
};

use bridge_vec::{BridgeVec, Bridgeable};

pub type PackedWasmSlicePtr = u64;

pub fn call_from_wasm<I, O, F>(
    capability: &'static str,
    client_state: Option<&BridgeVec>,
    input: Option<&I>,
    func: F,
) -> O
where
    I: Bridgeable,
    for<'a> <O as Archive>::Archived:
        CheckBytes<Strategy<Validator<ArchiveValidator<'a>, SharedValidator>, rancor::Error>>,
    for<'a> <O as Archive>::Archived: Deserialize<O, Strategy<Pool, RkyvError>>,
    F: FnOnce(*const u8, usize, *const u8, usize) -> *const u8,
{
    let (c_ptr, c_len) = if let Some(cs) = client_state {
        (cs.as_ptr(), cs.len())
    } else {
        (std::ptr::null(), 0)
    };
    let (i_ptr, i_len) = if let Some(i) = input {
        input.serialize()
    } else {
        BridgeVec::ok()
    };

    let result_ptr = (func)(c_ptr, c_len, i_ptr, i_len);
    if result_ptr.is_null() {
        panic!("Capability Error {capability}: Linked function failed");
    }
    let len = unsafe { *(result_ptr as *const u32) as usize };
    let data = unsafe { std::slice::from_raw_parts(result_ptr.add(4), len) };
    let archived = match rkyv::access::<O::Archived, RkyvError>(data) {
        Ok(archived) => archived,
        Err(err) => {
            error::set_last_error(error::CapabilityIoError::Verification(err.to_string()));
            panic!(
                "{} Capability Error: Verification failed: {}",
                capability, err
            );
        }
    };
    let result: O = match rkyv::deserialize::<_, RkyvError>(archived) {
        Ok(result) => result,
        Err(err) => {
            error::set_last_error(error::CapabilityIoError::Verification(err.to_string()));
            panic!(
                "{} Capability Error: Deserialization failed: {}",
                capability, err
            );
        }
    };
    result
}
