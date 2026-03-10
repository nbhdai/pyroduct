use std::sync::Mutex;
use std::{collections::HashMap, ops::Deref};

use crate::bridgeable::BridgeableZeroCopy;
use crate::format::{PyroZeroCopyFormat, Receiver};
use crate::header::{DataStatus, PyroHeaderMut};
use crate::value::PyroRow;
use crate::{Bridgeable, BridgeableResult, CapturedError, PyroError, ToRow};
use crate::{PyroVec, header::PyroData};

mod logger;

pub type ModuleResult<T> = Result<T, CapturedError>;

#[derive(Debug, Hash, PartialEq, Eq)]
struct StoredPtr(*const u8);
#[derive(Debug, Hash, PartialEq, Eq)]
struct StoredMutPtr(*const u8);

unsafe impl Send for StoredPtr {}
unsafe impl Send for StoredMutPtr {}

// Registry for vectors created by the Host (inputs)
static INPUT_REGISTRY: Mutex<Option<HashMap<StoredMutPtr, PyroVec>>> = Mutex::new(None);
// Registry for vectors created by WASM (outputs)
static OUTPUT_REGISTRY: Mutex<Option<HashMap<StoredPtr, PyroVec>>> = Mutex::new(None);

static ERROR_REGISTRY: Mutex<Vec<PyroVec>> = Mutex::new(Vec::new());

fn get_input_registry() -> std::sync::MutexGuard<'static, Option<HashMap<StoredMutPtr, PyroVec>>> {
    let mut lock = INPUT_REGISTRY.lock().unwrap();
    if lock.is_none() {
        *lock = Some(HashMap::new());
    }
    lock
}

fn get_output_registry() -> std::sync::MutexGuard<'static, Option<HashMap<StoredPtr, PyroVec>>> {
    let mut lock = OUTPUT_REGISTRY.lock().unwrap();
    if lock.is_none() {
        *lock = Some(HashMap::new());
    }
    lock
}

/// 1. Input Management (Host -> WASM)
/// -------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn new_input(capacity: u32) -> *mut u8 {
    let mut vec = PyroVec::with_capacity(capacity as usize);
    let raw = vec.as_packet_slice_mut();
    let ptr = raw.as_mut_ptr();

    get_input_registry()
        .as_mut()
        .unwrap()
        .insert(StoredMutPtr(ptr), vec);
    ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn grow_input(ptr: *mut u8, new_capacity: u32) -> *mut u8 {
    // Single lock scope — avoids the deadlock from re-acquiring the mutex.
    let mut registry = get_input_registry();
    let map = registry.as_mut().unwrap();

    if let Some(mut vec) = map.remove(&StoredMutPtr(ptr)) {
        let current_cap = vec.capacity();
        if (new_capacity as usize) > current_cap {
            vec.grow(new_capacity as usize);
        }
        let raw = vec.as_packet_slice_mut();
        let new_ptr = raw.as_mut_ptr();

        // Re-insert under the (potentially new) pointer — same lock guard.
        map.insert(StoredMutPtr(new_ptr), vec);
        new_ptr
    } else {
        std::ptr::null_mut()
    }
}

#[unsafe(no_mangle)]
pub fn get_input(ptr: *mut u8) -> Option<PyroVec> {
    get_input_registry()
        .as_mut()
        .unwrap()
        .remove(&StoredMutPtr(ptr))
}

/// Lend a read-only pointer to data owned inside wasm, for the host to read.
pub unsafe fn lend(vec: &PyroVec) -> *const u8 {
    let raw = vec.as_packet_slice();
    raw.as_ptr()
}

/// Consume a PyroVec and register it for the host to pick up.
pub fn store_error(error: PyroError) {
    let vec = error.encode();

    ERROR_REGISTRY.lock().as_mut().unwrap().push(vec);
}

/// Consume a PyroVec and register it for the host to pick up.
pub fn to_output(vec: PyroVec) -> *const u8 {
    let raw = vec.as_packet_slice();
    let ptr = raw.as_ptr();

    get_output_registry()
        .as_mut()
        .unwrap()
        .insert(StoredPtr(ptr), vec);
    ptr
}

/// Host calls this to free the result after reading it.
#[unsafe(no_mangle)]
pub extern "C" fn free_output(ptr: *const u8) {
    get_output_registry()
        .as_mut()
        .unwrap()
        .remove(&StoredPtr(ptr));
}

/// Test-only: re-insert a PyroVec into the input registry under a given pointer.
/// This simulates the host writing data into wasm linear memory.
#[cfg(test)]
pub fn _test_reinsert_input(ptr: *mut u8, vec: PyroVec) {
    get_input_registry()
        .as_mut()
        .unwrap()
        .insert(StoredMutPtr(ptr), vec);
}

/// Wasm-side entry-point wrapper for the standard `call_extern` convention.
///
/// The host calls a wasm export with signature `fn(i32) -> i32` where the i32
/// values are pointers into linear memory pointing at PyroVec buffers
/// containing rkyv-serialized `PyroRowOwned` data.
///
/// This module provides:
/// - `wasm_call_extern`: the generic trampoline that the `#[no_mangle]`
///   export delegates to.
/// - `WasmMain`: a trait the plugin author implements with their business logic.
///
/// The entire path is zero-copy on input: the host writes an rkyv PyroVec
/// into wasm linear memory via `new_input`, the wasm side retrieves ownership
/// via `get_input`, and `expose` / `expose_view` gives direct archived access
/// without deserialization.
///
/// ```rust,ignore
/// #[unsafe(no_mangle)]
/// pub extern "C" fn call_extern(ptr: *mut u8) -> *const u8 {
///     wasm_row_main(ptr, main_fn)
/// }
/// ```
pub fn wasm_row_main<'a, O, F>(input_ptr: *mut u8, func: F) -> *const u8
where
    O: ToRow,
    F: Fn(PyroRow<'a>) -> Result<O, CapturedError>,
{
    logger::init_logging();
    let input_vec = match get_input(input_ptr) {
        Some(vec) => vec,
        None => {
            let result = Err(CapturedError::new(format!(
                "Unable to locate input with offset {}",
                input_ptr as usize
            )));
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
    let input_row = PyroRow::from(&*input_row);
    let input = match input_row.try_into() {
        Ok(input) => input,
        Err(_) => {
            let result = Err(CapturedError::new("Unable to extract input"));
            return to_output(encode_result(result));
        }
    };

    let output = (func)(input);

    to_output(match output {
        Ok(o) => encode_result(Ok(o.to_row())),
        Err(err) => encode_result(Err(err)),
    })
}

fn encode_result<'a>(result: Result<PyroRow<'a>, CapturedError>) -> PyroVec {
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

pub struct Client<T> {
    data: T,
    config_buf: PyroVec,
}

impl<T> Deref for Client<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> Client<T> {
    pub fn __register<F>(data: T, register_func: F) -> Self
    where
        T: Bridgeable,
        F: FnOnce(*const u8) -> *mut u8,
    {
        // Serialize the data (client state) to get the config buffer
        let config_buf = match data.ship() {
            Ok(config_buf) => config_buf,
            Err(err) => {
                store_error(err);
                panic!("Was unable to serialize the client state");
            }
        };

        // Execute the registration via the provided callback (Host Import)
        let view_ptr = unsafe { lend(&config_buf) };
        let result_raw = register_func(view_ptr);
        let result_vec = match get_input(result_raw) {
            Some(result_vec) => result_vec,
            None => panic!("Host registration failed with no returned"),
        };

        // Check for transport/host errors
        if let Err(e) = result_vec.parse_as_error() {
            store_error(e);
            panic!("Host registration failed with a pyro error");
        }

        Self { data, config_buf }
    }

    pub fn __register_result<E, F>(data: T, register_func: F) -> Result<Self, E>
    where
        T: Bridgeable,
        E: Bridgeable,
        F: FnOnce(*const u8) -> *mut u8,
    {
        // Serialize the data (client state) to get the config buffer
        let config_buf = match data.ship() {
            Ok(config_buf) => config_buf,
            Err(err) => {
                store_error(err);
                panic!("Was unable to serialize the client state");
            }
        };

        // Execute the registration via the provided callback (Host Import)
        let view_ptr = unsafe { lend(&config_buf) };
        let result_raw = register_func(view_ptr);
        let result_vec = match get_input(result_raw) {
            Some(result_vec) => result_vec,
            None => panic!("Host registration failed with no returned"),
        };

        // Check for transport/host errors
        if let Err(e) = result_vec.parse_as_error() {
            store_error(e);
            panic!("Host registration failed with a pyro error");
        }

        Ok(Self { data, config_buf })
    }

    pub fn __call_from_wasm<I, O, F>(&self, input: Option<&I>, func: F) -> O
    where
        I: Bridgeable,
        O: Bridgeable + BridgeableZeroCopy,
        <O as Bridgeable>::Format: PyroZeroCopyFormat<O>,
        F: FnOnce(*const u8, *const u8) -> *mut u8,
    {
        let input = match input {
            Some(i) => match i.ship() {
                Ok(vec) => vec,
                Err(err) => err.encode(),
            },
            None => PyroVec::ok(),
        };

        let result_ptr = (func)(unsafe { lend(&self.config_buf) }, unsafe { lend(&input) });
        let result_vec = match get_input(result_ptr) {
            Some(result_vec) => result_vec,
            None => panic!("Host registration failed with no returned"),
        };
        let result = O::expose(result_vec).and_then(|r| O::receiver().receive(&r));
        match result {
            Ok(result) => result,
            Err(err) => {
                store_error(err);
                panic!("Received an unhandled error from host")
            }
        }
    }

    pub fn __call_result_from_wasm<I, O, E, F>(&self, input: Option<&I>, func: F) -> Result<O, E>
    where
        I: Bridgeable,
        O: Bridgeable + BridgeableZeroCopy,
        <O as Bridgeable>::Format: PyroZeroCopyFormat<O>,
        E: Bridgeable + BridgeableZeroCopy,
        <E as Bridgeable>::Format: PyroZeroCopyFormat<E>,
        F: FnOnce(*const u8, *const u8) -> *mut u8,
    {
        let input = match input {
            Some(i) => match i.ship() {
                Ok(vec) => vec,
                Err(err) => err.encode(),
            },
            None => PyroVec::ok(),
        };

        let result_ptr = (func)(unsafe { lend(&self.config_buf) }, unsafe { lend(&input) });
        let result_vec = match get_input(result_ptr) {
            Some(result_vec) => result_vec,
            None => panic!("Host registration failed with no returned"),
        };
        let result = Result::<O, E>::expose(result_vec).and_then(|r| {
            let res = match r {
                Ok(o) => Ok(O::receiver().receive(&o)?),
                Err(e) => Err(E::receiver().receive(&e)?),
            };
            Ok(res)
        });
        match result {
            Ok(result) => result,
            Err(err) => {
                store_error(err);
                panic!("Received an unhandled error from host")
            }
        }
    }
}

impl From<String> for CapturedError {
    fn from(err: String) -> Self {
        CapturedError::new(err)
    }
}
