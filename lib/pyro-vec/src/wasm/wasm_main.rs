//! Wasm-side entry-point wrapper for the standard `call_extern` convention.
//!
//! The host calls a wasm export with signature `fn(i32) -> i32` where the i32
//! values are pointers into linear memory pointing at PyroVec buffers
//! containing rkyv-serialized `PyroRowOwned` data.
//!
//! This module provides:
//! - `wasm_call_extern`: the generic trampoline that the `#[no_mangle]`
//!   export delegates to.
//! - `WasmMain`: a trait the plugin author implements with their business logic.
//!
//! The entire path is zero-copy on input: the host writes an rkyv PyroVec
//! into wasm linear memory via `new_input`, the wasm side retrieves ownership
//! via `get_input`, and `expose` / `expose_view` gives direct archived access
//! without deserialization.

use crate::{
    PyroVec, Bridgeable, CapturedError,
    header::{PyroData, PyroHeaderMut, DataStatus},
    value::PyroRow,
    wasm::wasm_side::{get_input, to_output},
};

/// The raw wasm export trampoline. Plugin authors write:
///
/// ```rust,ignore
/// #[unsafe(no_mangle)]
/// pub extern "C" fn call_extern(ptr: *mut u8) -> *const u8 {
///     wasm_row_main(ptr, main_fn)
/// }
/// ```
pub fn wasm_row_main<'a, 'b, F: Fn(PyroRow<'a>) -> Result<PyroRow<'b>, anyhow::Error>>(
    input_ptr: *mut u8,
    func: F,
) -> *const u8 {
    let input_vec = match get_input(input_ptr) {
        Some(vec) => vec,
        None => {
            let result = Err(anyhow::anyhow!(
                "Unable to locate input with offset {}",
                input_ptr as usize
            ));
            return to_output(encode_result(result));
        }
    };

    // 2. Check for pyro-level errors forwarded from the host.
    if let Err(err) = input_vec.parse_as_error() {
        return to_output(err.encode());
    };
    let input_row = match PyroRow::expose(input_vec) {
        Ok(vec) => vec,
        Err(err) => return to_output(err.encode()),
    };
    let input = PyroRow::from(&*input_row);

    let result_vec = encode_result((func)(input));
    to_output(result_vec)
}

fn encode_result<'a>(result: anyhow::Result<PyroRow<'a>>) -> PyroVec {
    let encoding = match result {
        Ok(success) => {
            let static_success = success.into_owned();
            static_success.ship()
        }
        Err(err) => {
            let captured: CapturedError = err.into();
            let mut vec = captured.encode();
            vec.set_status(DataStatus::RkyvError);
            return vec;
        }
    };
    match encoding {
        Ok(v) => v,
        Err(e) => e.encode(),
    }
}
