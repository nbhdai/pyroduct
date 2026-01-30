use crate::module_capability::error;
use rkyv::{
    Archive, Deserialize, Serialize,
    bytecheck::CheckBytes,
    de::Pool,
    rancor::{self, Error as RkyvError, Strategy},
    ser::{Serializer, allocator::ArenaHandle, sharing::Share},
    util::AlignedVec,
    validation::{Validator, archive::ArchiveValidator, shared::SharedValidator},
};

pub type PackedWasmSlicePtr = u64;

pub fn slice_to_wasm_slice<T: 'static + AsRef<[u8]>>(bytes: &T) -> PackedWasmSlicePtr {
    let bytes_ptr = bytes.as_ref().as_ptr();
    let len = bytes.as_ref().len();

    // PACKING: ptr (low 32 bits) | len (high 32 bits)
    let ptr_val = bytes_ptr as u32 as u64;
    let len_val = (len as u32 as u64) << 32;

    let packed = ptr_val | len_val;
    packed
}

pub fn wasm_ptr_to_slice(wasm_slice: PackedWasmSlicePtr) -> Option<(usize, usize)> {
    tracing::info!("WASM execution finished. Decoding result...");
    let output_ptr = (wasm_slice & 0xFFFFFFFF) as usize;
    let len = (wasm_slice >> 32) as usize;

    let start = output_ptr;
    let end = output_ptr + len;

    match start.cmp(&end) {
        std::cmp::Ordering::Less => Some((start, end)),
        std::cmp::Ordering::Equal => {
            tracing::error!(start, end, "Length is 0");
            None
        }
        std::cmp::Ordering::Greater => {
            tracing::error!(start, end, "Start past the length");
            None
        }
    }
}

pub fn call_from_wasm<C, I, O, F>(
    capability: &'static str,
    client_state: Option<&C>,
    input: Option<&I>,
    func: F,
) -> O
where
    C: Archive,
    for<'a> C: Serialize<Strategy<Serializer<AlignedVec, ArenaHandle<'a>, Share>, RkyvError>>,
    I: Archive,
    for<'a> I: Serialize<Strategy<Serializer<AlignedVec, ArenaHandle<'a>, Share>, RkyvError>>,
    O: Archive,
    for<'a> <O as Archive>::Archived:
        CheckBytes<Strategy<Validator<ArchiveValidator<'a>, SharedValidator>, rancor::Error>>,
    for<'a> <O as Archive>::Archived: Deserialize<O, Strategy<Pool, RkyvError>>,
    F: FnOnce(*const u8, usize, *const u8, usize) -> *const u8,
{
    let (c_ptr, c_len, _c_bytes) = if let Some(cs) = client_state {
        let client_bytes = match rkyv::to_bytes::<RkyvError>(cs) {
            Ok(bytes) => bytes,
            Err(err) => {
                error::set_last_error(error::CapabilityIoError::ClientSerialization(
                    err.to_string(),
                ));
                panic!("{} Capability Serialization failed: {}", capability, err);
            }
        };
        (client_bytes.as_ptr(), client_bytes.len(), client_bytes)
    } else {
        (std::ptr::null(), 0, AlignedVec::new())
    };

    let (i_ptr, i_len, _i_bytes) = if let Some(i) = input {
        let i_bytes = match rkyv::to_bytes::<RkyvError>(i) {
            Ok(bytes) => bytes,
            Err(err) => {
                error::set_last_error(error::CapabilityIoError::InputSerialization(
                    err.to_string(),
                ));
                panic!("{} Capability Serialization failed: {}", capability, err);
            }
        };
        (i_bytes.as_ptr(), i_bytes.len(), i_bytes)
    } else {
        (std::ptr::null(), 0, AlignedVec::new())
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
