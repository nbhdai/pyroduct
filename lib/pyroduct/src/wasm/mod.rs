use std::panic::{self, PanicHookInfo};
use std::sync::{Mutex, Once};
use std::{collections::HashMap, ops::Deref};
use tracing::{debug, error, trace};

use crate::format::header::PyroHeader;
use crate::format::{
    Bridgeable, BridgeableResult, PyroVec, ToRow,
    bridgeable::BridgeableZeroCopy,
    format::{PyroZeroCopyFormat, Receiver},
    header::PyroData,
    header::{DataStatus, PyroHeaderMut},
    value::PyroRow,
};

use crate::CapturedError;
use crate::session::SessionResponse;

pub mod interconnect;
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

static SESSION_INPUT: Mutex<Option<HashMap<u32, Vec<PyroVec>>>> = Mutex::new(None);
static SESSION_OUTPUT: Mutex<Option<HashMap<u32, Vec<PyroVec>>>> = Mutex::new(None);

static ERROR_REGISTRY: Mutex<Option<PyroVec>> = Mutex::new(None);

fn store_error(mut vec: PyroVec) {
    vec.set_status(DataStatus::RkyvError);
    ERROR_REGISTRY.clear_poison();
    *ERROR_REGISTRY.lock().unwrap() = Some(vec);
}

static REGISTER_PANIC_HOOK: Once = Once::new();

pub fn register_ffi_panic_hook() {
    REGISTER_PANIC_HOOK.call_once(|| {
        debug!("register_ffi_panic_hook: installing global panic hook for FFI boundary");
        let default_hook = panic::take_hook();

        panic::set_hook(Box::new(move |info: &PanicHookInfo| {
            let mut error = if let Some(s) = info.payload().downcast_ref::<&str>() {
                CapturedError::new(*s)
            } else if let Some(s) = info.payload().downcast_ref::<String>() {
                CapturedError::new(s)
            } else {
                CapturedError::new("Panic occurred (unknown payload type)")
            };

            if let Some(loc) = info.location() {
                error = error.with_location(loc);
            };

            error = error.with_backtrace(std::backtrace::Backtrace::capture());

            let mut vec = error.encode();
            vec.set_status(DataStatus::RkyvError);

            error!(?error, "FFI Panic Hook captured a panic");
            ERROR_REGISTRY.clear_poison();
            let mut registry = ERROR_REGISTRY.lock().unwrap();
            *registry = Some(vec);

            default_hook(info);
        }));
    });
}

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

fn get_input_session_registry() -> std::sync::MutexGuard<'static, Option<HashMap<u32, Vec<PyroVec>>>>
{
    let mut lock = SESSION_INPUT.lock().unwrap();
    if lock.is_none() {
        *lock = Some(HashMap::new());
    }
    lock
}

fn get_output_session_registry()
-> std::sync::MutexGuard<'static, Option<HashMap<u32, Vec<PyroVec>>>> {
    let mut lock = SESSION_OUTPUT.lock().unwrap();
    if lock.is_none() {
        *lock = Some(HashMap::new());
    }
    lock
}

/// Allocates an input and provides the pointer
#[unsafe(no_mangle)]
pub extern "C" fn new_input(capacity: u32) -> *mut u8 {
    trace!(capacity, "new_input");
    let mut vec = PyroVec::with_capacity(capacity as usize);
    let raw = vec.as_raw_slice_mut();
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
        let raw = vec.as_raw_slice_mut();
        let new_ptr = raw.as_mut_ptr();

        // Re-insert under the (potentially new) pointer — same lock guard.
        map.insert(StoredMutPtr(new_ptr), vec);
        trace!(?ptr, ?new_ptr, new_capacity, "grow_input");
        new_ptr
    } else {
        error!(?ptr, "grow_input: pointer not found in registry");
        std::ptr::null_mut()
    }
}

#[unsafe(no_mangle)]
pub fn get_input(ptr: *mut u8) -> Option<PyroVec> {
    trace!(?ptr, "get_input");
    get_input_registry()
        .as_mut()
        .unwrap()
        .remove(&StoredMutPtr(ptr))
}

/// Lend a read-only pointer to data owned inside wasm, for the host to read.
pub fn lend(vec: &PyroVec) -> *const u8 {
    let raw = vec.as_raw_slice();
    raw.as_ptr()
}

/// Consume a PyroVec and register it for the host to pick up.
pub fn to_output(vec: PyroVec) -> *const u8 {
    let raw = vec.as_raw_slice();
    let ptr = raw.as_ptr();

    get_output_registry()
        .as_mut()
        .unwrap()
        .insert(StoredPtr(ptr), vec);
    trace!(?ptr, "to_output");
    ptr
}

/// Host calls this to free the result after reading it.
#[unsafe(no_mangle)]
pub extern "C" fn free_output(ptr: *const u8) {
    trace!(?ptr, "free_output");
    get_output_registry()
        .as_mut()
        .unwrap()
        .remove(&StoredPtr(ptr));
}

/// Allocate a new input vector on a session's input history.
#[unsafe(no_mangle)]
pub extern "C" fn new_session_input(session_id: u32, capacity: u32) -> *mut u8 {
    trace!(session_id, capacity, "new_session_input");
    let mut registry = SESSION_INPUT.lock().unwrap();
    if registry.is_none() {
        *registry = Some(HashMap::new());
    }
    let map = registry.as_mut().unwrap();
    let mut vec = PyroVec::with_capacity(capacity as usize);
    let raw = vec.as_raw_slice_mut();
    let ptr = raw.as_mut_ptr();
    let vecs = map.entry(session_id).or_default();
    vecs.push(vec);
    ptr
}

/// Grow the last input vector for a session.
#[unsafe(no_mangle)]
pub extern "C" fn grow_session_input(session_id: u32, new_capacity: u32) -> *mut u8 {
    trace!(session_id, new_capacity, "grow_session_input");
    let mut registry = SESSION_INPUT.lock().unwrap();
    if registry.is_none() {
        *registry = Some(HashMap::new());
    }
    let map = registry.as_mut().unwrap();
    let vecs = map.entry(session_id).or_default();

    if let Some(last) = vecs.last_mut() {
        let current_cap = last.capacity();
        if (new_capacity as usize) > current_cap {
            last.grow(new_capacity as usize);
        }
        let raw = last.as_raw_slice_mut();
        raw.as_mut_ptr()
    } else {
        let mut vec = PyroVec::with_capacity(new_capacity as usize);
        let raw = vec.as_raw_slice_mut();
        let ptr = raw.as_mut_ptr();
        vecs.push(vec);
        ptr
    }
}

/// Allocate a new output vector on a session's output history.
#[unsafe(no_mangle)]
pub extern "C" fn new_session_output(session_id: u32, capacity: u32) -> *mut u8 {
    trace!(session_id, capacity, "new_session_output");
    let mut registry = SESSION_OUTPUT.lock().unwrap();
    if registry.is_none() {
        *registry = Some(HashMap::new());
    }
    let map = registry.as_mut().unwrap();
    let mut vec = PyroVec::with_capacity(capacity as usize);
    let raw = vec.as_raw_slice_mut();
    let ptr = raw.as_mut_ptr();
    let vecs = map.entry(session_id).or_default();
    vecs.push(vec);
    ptr
}

/// Grow the last output vector for a session.
#[unsafe(no_mangle)]
pub extern "C" fn grow_session_output(session_id: u32, new_capacity: u32) -> *mut u8 {
    trace!(session_id, new_capacity, "grow_session_output");
    let mut registry = SESSION_OUTPUT.lock().unwrap();
    if registry.is_none() {
        *registry = Some(HashMap::new());
    }
    let map = registry.as_mut().unwrap();
    let vecs = map.entry(session_id).or_default();

    if let Some(last) = vecs.last_mut() {
        let current_cap = last.capacity();
        if (new_capacity as usize) > current_cap {
            last.grow(new_capacity as usize);
        }
        let raw = last.as_raw_slice_mut();
        raw.as_mut_ptr()
    } else {
        let mut vec = PyroVec::with_capacity(new_capacity as usize);
        let raw = vec.as_raw_slice_mut();
        let ptr = raw.as_mut_ptr();
        vecs.push(vec);
        ptr
    }
}

/// Borrow a pointer to a session's input vector at the given index.
#[unsafe(no_mangle)]
pub extern "C" fn borrow_session_input(session_id: u32, index: u32) -> *mut u8 {
    let registry = SESSION_INPUT.lock().unwrap();
    if registry.is_none() {
        error!(session_id, index, "borrow_session_input: registry is none");
        return std::ptr::null_mut();
    }
    let map = registry.as_ref().unwrap();
    if let Some(vecs) = map.get(&session_id)
        && let Some(vec) = vecs.get(index as usize)
    {
        let ptr = vec.as_raw_slice().as_ptr() as *mut u8;
        trace!(session_id, index, ?ptr, "borrow_session_input");
        return ptr;
    }
    error!(
        session_id,
        index, "borrow_session_input: session or index not found"
    );
    std::ptr::null_mut()
}

/// Get the number of input vectors for a session.
#[unsafe(no_mangle)]
pub extern "C" fn session_input_length(session_id: u32) -> u32 {
    let registry = SESSION_INPUT.lock().unwrap();
    if let Some(map) = registry.as_ref()
        && let Some(vecs) = map.get(&session_id)
    {
        let len = vecs.len() as u32;
        trace!(session_id, len, "session_input_length");
        return len;
    }
    trace!(session_id, "session_input_length: 0");
    0
}

/// Borrow a pointer to a session's output vector at the given index.
#[unsafe(no_mangle)]
pub extern "C" fn borrow_session_output(session_id: u32, index: u32) -> *mut u8 {
    let registry = SESSION_OUTPUT.lock().unwrap();
    if registry.is_none() {
        error!(session_id, index, "borrow_session_output: registry is none");
        return std::ptr::null_mut();
    }
    let map = registry.as_ref().unwrap();
    if let Some(vecs) = map.get(&session_id)
        && let Some(vec) = vecs.get(index as usize)
    {
        let ptr = vec.as_raw_slice().as_ptr() as *mut u8;
        trace!(session_id, index, ?ptr, "borrow_session_output");
        return ptr;
    }
    error!(
        session_id,
        index, "borrow_session_output: session or index not found"
    );
    std::ptr::null_mut()
}

/// Get the number of output vectors for a session.
#[unsafe(no_mangle)]
pub extern "C" fn session_output_length(session_id: u32) -> u32 {
    let registry = SESSION_OUTPUT.lock().unwrap();
    if let Some(map) = registry.as_ref()
        && let Some(vecs) = map.get(&session_id)
    {
        let len = vecs.len() as u32;
        trace!(session_id, len, "session_output_length");
        return len;
    }
    trace!(session_id, "session_output_length: 0");
    0
}

/// Free all vectors associated with a session and remove the session.
#[unsafe(no_mangle)]
pub extern "C" fn free_session(session_id: u32) {
    trace!(session_id, "free_session");
    if let Some(map) = SESSION_INPUT.lock().unwrap().as_mut() {
        map.remove(&session_id);
    }
    if let Some(map) = SESSION_OUTPUT.lock().unwrap().as_mut() {
        map.remove(&session_id);
    }
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
            error!(?input_ptr, "Unable to locate input");
            let result = Err(CapturedError::new(format!(
                "Unable to locate input with offset {}",
                input_ptr as usize
            )));
            return to_output(match encode_result(result) {
                Ok(r) => r,
                Err(r) => r,
            });
        }
    };

    // 2. Check for pyro-level errors forwarded from the host.
    if let Err(err) = input_vec.parse_as_error() {
        error!(?err, "pyro-level error forwarded from host");
        return to_output(err.encode());
    };
    let input_row = match PyroRow::expose(input_vec.view()) {
        Ok(vec) => vec,
        Err(err) => {
            error!(?err, "Unable to expose PyroRow");
            return to_output(err.encode());
        }
    };
    let input = PyroRow::from(&*input_row);

    register_ffi_panic_hook();

    to_output(match func(input) {
        Ok(o) => match encode_result(Ok(o.to_row())) {
            Ok(r) => r,
            Err(r) => r,
        },
        Err(err) => {
            error!(?err, "Function execution failed");

            match encode_result(Err(err)) {
                Ok(r) => r,
                Err(r) => r,
            }
        }
    })
}

pub fn wasm_row_main_session<'a, O, F>(session_id: u32, func: F) -> *const u8
where
    O: ToRow,
    F: Fn(&[PyroRow<'a>], PyroRow<'a>) -> Result<SessionResponse<O>, CapturedError>,
{
    logger::init_logging();
    let mut sessions_guard = get_input_session_registry();
    let sessions = sessions_guard.as_mut().unwrap();

    let inputs = sessions.entry(session_id).or_default();

    if inputs.is_empty() {
        error!(session_id, "The input is missing for session");
        let result = Err(CapturedError::new("The input is missing"));
        return to_output(match encode_result(result) {
            Ok(r) => r,
            Err(r) => r,
        });
    }

    let current_input = &inputs[inputs.len() - 1];
    if current_input.status() == Ok(DataStatus::Empty) {
        error!(session_id, "Session terminated");
        return to_output(CapturedError::new(format!("Session {session_id} Terminated")).encode());
    }
    let input_row = match PyroRow::expose_view(current_input.py_ref()) {
        Ok(vec) => vec,
        Err(err) => {
            error!(session_id, ?err, "Unable to expose view for current input");
            return to_output(err.encode());
        }
    };
    let input = PyroRow::from(&*input_row);

    let mut prior = Vec::with_capacity(inputs.len());
    for (input, ir) in inputs[0..inputs.len() - 1].iter().enumerate() {
        if ir.status() == Ok(DataStatus::Empty) {
            error!(session_id, input, "Session terminated");
            return to_output(
                CapturedError::new(format!("Session {session_id} Terminated")).encode(),
            );
        }
        let input_row = match PyroRow::expose_view(ir.py_ref()) {
            Ok(vec) => vec,
            Err(err) => {
                error!(session_id, ?err, "Unable to expose view for prior input");
                return to_output(err.encode());
            }
        };
        let input = PyroRow::from(&*input_row);
        prior.push(input);
    }

    trace!(priors = prior.len(), "Retrieve priors");

    register_ffi_panic_hook();

    let result = match func(&prior, input) {
        Ok(result) => result,
        Err(err) => {
            error!(session_id, ?err, "Session function execution failed");
            let mut vec = err.encode();
            vec.set_status(DataStatus::RkyvError);
            return to_output(vec);
        }
    };

    trace!("Processed function");

    let result = match result {
        SessionResponse::Continue(o) => {
            let mut result = match encode_result(Ok(o.to_row())) {
                Ok(r) => r,
                Err(e) => return to_output(e),
            };
            result.set_fn_id(0);
            result
        }
        SessionResponse::End(o) => {
            let mut result = match encode_result(Ok(o.to_row())) {
                Ok(r) => r,
                Err(e) => return to_output(e),
            };
            result.set_fn_id(1);
            result
        }
        SessionResponse::Terminate => {
            let mut result = PyroVec::ok();
            result.set_fn_id(2);
            result
        }
    };
    let sessions = sessions.entry(session_id).or_default();
    sessions.push(result);
    let last = sessions.last().unwrap();
    tracing::debug!(header = ?*last.header(), "wasm_row_main_session: before return");
    // Return pointer to PyroInner (the allocation base that host get_ref expects)
    last.raw_ptr()
}

pub fn wasm_row_main_session_diff<'a, O, F>(session_id: u32, func: F) -> *const u8
where
    O: ToRow,
    F: Fn(&[PyroRow<'a>], &[PyroRow<'a>], PyroRow<'a>) -> Result<SessionResponse<O>, CapturedError>,
{
    logger::init_logging();
    let mut input_sessions_guard = get_input_session_registry();
    let mut output_sessions_guard = get_output_session_registry();
    let input_sessions = input_sessions_guard.as_mut().unwrap();
    let output_sessions = output_sessions_guard.as_mut().unwrap();

    let inputs = input_sessions.entry(session_id).or_default();
    let outputs = output_sessions.entry(session_id).or_default();

    if outputs.len() + 1 != inputs.len() {
        error!(
            session_id,
            inputs_len = inputs.len(),
            outputs_len = outputs.len(),
            "The input is missing or session state is inconsistent"
        );
        let result = Err(CapturedError::new("The input is missing"));
        return to_output(match encode_result(result) {
            Ok(r) => r,
            Err(r) => r,
        });
    }

    let current_input = &inputs[outputs.len()];
    if current_input.status() == Ok(DataStatus::Empty) {
        error!(session_id, "Session terminated");
        return to_output(CapturedError::new(format!("Session {session_id} Terminated")).encode());
    }
    let input_row = match PyroRow::expose_view(current_input.py_ref()) {
        Ok(vec) => vec,
        Err(err) => {
            error!(
                session_id,
                ?err,
                "Unable to expose view for current input in diff session"
            );
            return to_output(err.encode());
        }
    };
    let input = PyroRow::from(&*input_row);

    let mut prior_inputs = Vec::with_capacity(inputs.len());
    for (input, ir) in inputs[0..inputs.len() - 1].iter().enumerate() {
        if ir.status() == Ok(DataStatus::Empty) {
            error!(session_id, input, "Session terminated");
            return to_output(
                CapturedError::new(format!("Session {session_id} Terminated")).encode(),
            );
        }
        let input_row = match PyroRow::expose_view(ir.py_ref()) {
            Ok(vec) => vec,
            Err(err) => {
                error!(
                    session_id,
                    ?err,
                    "Unable to expose view for prior input in diff session"
                );
                return to_output(err.encode());
            }
        };
        let input = PyroRow::from(&*input_row);
        prior_inputs.push(input);
    }

    let mut prior_outputs = Vec::with_capacity(outputs.len());
    for (output, or) in outputs.iter().enumerate() {
        if or.status() == Ok(DataStatus::Empty) {
            error!(session_id, output, "Session terminated");
            return to_output(
                CapturedError::new(format!("Session {session_id} Terminated")).encode(),
            );
        }
        let output_row = match PyroRow::expose_view(or.py_ref()) {
            Ok(vec) => vec,
            Err(err) => {
                error!(
                    session_id,
                    ?err,
                    "Unable to expose view for prior output in diff session"
                );
                return to_output(err.encode());
            }
        };
        let output = PyroRow::from(&*output_row);
        prior_outputs.push(output);
    }

    register_ffi_panic_hook();

    let result = match func(&prior_inputs, &prior_outputs, input) {
        Ok(result) => result,
        Err(err) => {
            error!(session_id, ?err, "Session function execution failed");
            let mut vec = err.encode();
            vec.set_status(DataStatus::RkyvError);
            return to_output(vec);
        }
    };

    let result = match result {
        SessionResponse::Continue(o) => match encode_result(Ok(o.to_row())) {
            Ok(mut r) => {
                r.set_fn_id(0);
                r
            }
            Err(e) => return to_output(e),
        },
        SessionResponse::End(o) => match encode_result(Ok(o.to_row())) {
            Ok(mut r) => {
                r.set_fn_id(1);
                r
            }
            Err(e) => return to_output(e),
        },
        SessionResponse::Terminate => {
            let mut result = PyroVec::ok();
            result.set_fn_id(2);
            result
        }
    };
    let sessions = output_sessions.entry(session_id).or_default();
    sessions.push(result);
    // Return pointer to PyroInner (the allocation base that host get_ref expects)
    sessions.last().unwrap().raw_ptr()
}

fn encode_result<'a>(result: Result<PyroRow<'a>, CapturedError>) -> Result<PyroVec, PyroVec> {
    let encoding = match result {
        Ok(success) => {
            let static_success = success.into_owned();
            static_success.ship()
        }
        Err(err) => {
            error!(?err, "encode_result: result is Err");
            let mut vec = err.encode();
            vec.set_status(DataStatus::RkyvError);
            return Err(vec);
        }
    };
    match encoding {
        Ok(mut v) => {
            v.set_status(DataStatus::RkyvValid);
            Ok(v)
        }
        Err(e) => {
            error!(?e, "encode_result: encoding failed");
            Err(e.encode())
        }
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
                error!(?err, "__register: was unable to serialize the client state");
                panic!("Was unable to serialize the client state");
            }
        };

        // Execute the registration via the provided callback (Host Import)
        let view_ptr = lend(&config_buf);
        let result_raw = register_func(view_ptr);
        let result_vec = match get_input(result_raw) {
            Some(result_vec) => result_vec,
            None => {
                error!(
                    ?result_raw,
                    "__register: Host registration failed with no returned"
                );
                panic!("Host registration failed with no returned");
            }
        };

        // Check for transport/host errors
        if let Err(e) = result_vec.parse_as_error() {
            error!(?e, "__register: Host registration failed with a pyro error");
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
                error!(
                    ?err,
                    "__register_result: was unable to serialize the client state"
                );
                panic!("Was unable to serialize the client state");
            }
        };

        // Execute the registration via the provided callback (Host Import)
        let view_ptr = lend(&config_buf);
        let result_raw = register_func(view_ptr);
        let result_vec = match get_input(result_raw) {
            Some(result_vec) => result_vec,
            None => {
                error!(
                    ?result_raw,
                    "__register_result: Host registration failed with no returned"
                );
                panic!("Host registration failed with no returned");
            }
        };

        // Check for transport/host errors
        if let Err(e) = result_vec.parse_as_error() {
            error!(
                ?e,
                "__register_result: Host registration failed with a pyro error"
            );
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
                Err(err) => {
                    error!(?err, "__call_from_wasm: failed to ship input");
                    err.encode()
                }
            },
            None => PyroVec::ok(),
        };

        let result_ptr = (func)(lend(&self.config_buf), lend(&input));
        let result_vec = match get_input(result_ptr) {
            Some(result_vec) => result_vec,
            None => {
                error!(
                    ?result_ptr,
                    "__call_from_wasm: Host registration failed with no returned"
                );
                panic!("Host registration failed with no returned");
            }
        };
        let result = O::expose(result_vec.view()).and_then(|r| O::receiver().receive(&r));
        match result {
            Ok(result) => result,
            Err(err) => {
                error!(
                    ?err,
                    "__call_from_wasm: Received an unhandled error from host"
                );
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
                Err(err) => {
                    error!(?err, "__call_result_from_wasm: failed to ship input");
                    err.encode()
                }
            },
            None => PyroVec::ok(),
        };

        let result_ptr = (func)(lend(&self.config_buf), lend(&input));
        let result_vec = match get_input(result_ptr) {
            Some(result_vec) => result_vec,
            None => {
                error!(
                    ?result_ptr,
                    "__call_result_from_wasm: Host registration failed with no returned"
                );
                panic!("Host registration failed with no returned");
            }
        };
        let result = Result::<O, E>::expose(result_vec.view()).and_then(|r| {
            let res = match r {
                Ok(o) => Ok(O::receiver().receive(&o)?),
                Err(e) => Err(E::receiver().receive(&e)?),
            };
            Ok(res)
        });
        match result {
            Ok(result) => result,
            Err(err) => {
                error!(
                    ?err,
                    "__call_result_from_wasm: Received an unhandled error from host"
                );
                panic!("Received an unhandled error from host")
            }
        }
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn lend_error() -> *const u8 {
    trace!("lend_error");
    ERROR_REGISTRY.clear_poison();
    let registry = ERROR_REGISTRY.lock().unwrap();
    if let Some(ref vec) = *registry {
        let raw = vec.as_raw_slice();
        raw.as_ptr()
    } else {
        std::ptr::null()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn free_error() {
    trace!("free_error");
    ERROR_REGISTRY.clear_poison();
    let mut registry = ERROR_REGISTRY.lock().unwrap();
    *registry = None;
}
