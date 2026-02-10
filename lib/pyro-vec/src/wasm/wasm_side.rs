use std::collections::HashMap;
use std::sync::Mutex;

use crate::{PyroVec, CapturedError, header::PyroData};

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

static ERROR_REGISTRY: Mutex<Option<HashMap<StoredPtr, PyroVec>>> = Mutex::new(None);

fn get_input_registry() -> std::sync::MutexGuard<'static, Option<HashMap<StoredMutPtr, PyroVec>>>
{
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
pub fn store_error(error: &CapturedError) -> *const u8 {
    let vec = error.encode();
    let raw = vec.as_packet_slice();
    let ptr = raw.as_ptr();

    get_output_registry()
        .as_mut()
        .unwrap()
        .insert(StoredPtr(ptr), vec);
    ptr
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
extern "C" fn free_output(ptr: *const u8) {
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
