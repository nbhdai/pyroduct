use pyro_spec::ModuleFunc;
use std::{collections::HashMap, sync::Mutex};

use crate::CapturedError;
use crate::format::PyroVec;
use crate::format::value::PyroRow;

static INTERCONNECT_SPECS: Mutex<Option<HashMap<String, ModuleFunc<'static>>>> = Mutex::new(None);

#[cfg(target_arch = "wasm32")]
unsafe extern "C" {
    fn call_interconnect(name_ptr: *const u8, name_len: usize, input_ptr: *const u8) -> *mut u8;
}

#[cfg(not(target_arch = "wasm32"))]
unsafe extern "C" fn call_interconnect(
    _name_ptr: *const u8,
    _name_len: usize,
    _input_ptr: *const u8,
) -> *mut u8 {
    std::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn populate_interconnect_specs(input_ptr: *mut u8) -> *const u8 {
    let input_vec = match super::get_input(input_ptr) {
        Some(vec) => vec,
        None => {
            let err = CapturedError::new(
                "populate_interconnect_specs: input pointer not found in registry",
            );
            return super::to_output(err.encode());
        }
    };

    let specs: HashMap<String, ModuleFunc<'static>> = match serde_json::from_slice(&input_vec) {
        Ok(s) => s,
        Err(e) => {
            let err = CapturedError::new(format!(
                "populate_interconnect_specs: failed to deserialize specs: {}",
                e
            ));
            return super::to_output(err.encode());
        }
    };

    let mut lock = INTERCONNECT_SPECS.lock().unwrap();
    if let Some(ref mut map) = *lock {
        map.extend(specs);
    } else {
        *lock = Some(specs);
    }

    super::to_output(PyroVec::ok())
}

#[unsafe(no_mangle)]
pub extern "C" fn clear_interconnect_specs() {
    let mut lock = INTERCONNECT_SPECS.lock().unwrap();
    *lock = None;
}

// ─────────────────────────────────────────────────────────────────────────────
// Extern Interconnect Invocation (WASM/Capability Style)
// ─────────────────────────────────────────────────────────────────────────────

/// Invokes an interconnect function.
/// Ships the input `PyroRow` to the host, runs the FFI call, retrieves and
/// exposes the returned output `PyroRow`. Works similarly to capability calls.
pub fn invoke_interconnect(
    name: &str,
    input: &PyroRow<'_>,
) -> Result<PyroRow<'static>, CapturedError> {
    use crate::format::Bridgeable;
    use crate::format::bridgeable::BridgeableZeroCopy;
    use crate::format::header::PyroData;

    let input_owned = input.clone().into_owned();
    let input_vec = input_owned.ship().map_err(|e| {
        CapturedError::new(format!(
            "invoke_interconnect: failed to ship input row for '{}': {}",
            name, e
        ))
    })?;

    let result_ptr =
        unsafe { call_interconnect(name.as_ptr(), name.len(), super::lend(&input_vec)) };

    if result_ptr.is_null() {
        return Err(CapturedError::new(format!(
            "invoke_interconnect: FFI call to '{}' returned null pointer",
            name
        )));
    }

    let result_vec = match super::get_input(result_ptr) {
        Some(v) => v,
        None => {
            return Err(CapturedError::new(format!(
                "invoke_interconnect: result pointer {:#x} not found in output registry",
                result_ptr as usize
            )));
        }
    };

    if let Err(e) = result_vec.parse_as_error() {
        return Err(e.into());
    }

    let typed = PyroRow::<'static>::expose(result_vec.view()).map_err(|e| {
        CapturedError::new(format!(
            "invoke_interconnect: failed to expose result for '{}': {}",
            name, e
        ))
    })?;

    let mut receiver = <PyroRow<'static> as BridgeableZeroCopy>::receiver();
    let recovered = receiver.receive(&typed).map_err(|e| {
        CapturedError::new(format!(
            "invoke_interconnect: failed to receive result for '{}': {}",
            name, e
        ))
    })?;

    Ok(recovered)
}

// ─────────────────────────────────────────────────────────────────────────────
// Guest/Module-Internal Query Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Query a single interconnect spec by name from within the module, returning a clone if found.
pub fn query_spec(name: &str) -> Option<ModuleFunc<'static>> {
    let lock = INTERCONNECT_SPECS.lock().unwrap();
    lock.as_ref().and_then(|map| map.get(name).cloned())
}

/// Query all interconnect specs from within the module, returning a clone of the hashmap if initialized.
pub fn query_all_specs() -> Option<HashMap<String, ModuleFunc<'static>>> {
    let lock = INTERCONNECT_SPECS.lock().unwrap();
    lock.clone()
}

/// Access a single interconnect spec by name with a closure, avoiding clones.
pub fn with_spec<R, F>(name: &str, f: F) -> Option<R>
where
    F: FnOnce(&ModuleFunc<'static>) -> R,
{
    let lock = INTERCONNECT_SPECS.lock().unwrap();
    lock.as_ref().and_then(|map| map.get(name).map(f))
}

/// Access the entire interconnect specs hashmap with a closure, avoiding clones.
pub fn with_specs<R, F>(f: F) -> Option<R>
where
    F: FnOnce(&HashMap<String, ModuleFunc<'static>>) -> R,
{
    let lock = INTERCONNECT_SPECS.lock().unwrap();
    lock.as_ref().map(f)
}
