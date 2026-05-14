//! Host-side harness for calling the wasm module's session entry function.
//!
//! `Session` owns the session input/output history in wasm memory and provides
//! methods to:
//!   1. Push new inputs via `new_session_input` / `grow_session_input`.
//!   2. Call the generated `call_session_extern(session_id)` export.
//!   3. Read the output back as a `PyroSuccess` or `PyroFailure`.
//!   4. Access prior inputs/outputs via `borrow_session_input` / `borrow_session_output`.
//!   5. Free the session and its history via `free_session`.
//!
//! All data crosses the boundary as rkyv-serialized PyroVecs. The host uses
//! `Bridgeable::ship()` to produce them and `Bridgeable::expose_view()` to
//! get zero-copy access to the archived data in wasm linear memory.
//!
//! ### Session lifecycle
//!
//! ```rust,ignore
//! let mut session = Session::new(session_id)?;
//! session.push_input(&mut instance, &input_row).await?;
//! let result = session.call(&mut instance).await?;
//! match result {
//!     SessionResult::Continue(row) | SessionResult::End(row) => {
//!         // process the row
//!         // push more input if needed
//!     }
//!     SessionResult::Terminate => {
//!         // session ended
//!     }
//! }
//! session.free(&mut instance)?;
//! ```

use wasmtime::{AsContext, Instance, Memory, Store};

use crate::format::bridgeable::Bridgeable;
use crate::format::header::{PyroData, PyroHeader};
use crate::format::{
    PyroFailure, PyroLogs, PyroRow, get_ref,
    header::{DataStatus, PyroParser},
};
use crate::format::ParseError;
use crate::{CapturedError, PyroError};

use super::PyroState;
use super::WasmError;

/// The type of session response returned by `Session::call()`.
#[derive(Debug)]
pub enum SessionResult {
    /// The session should continue. Contains the output row.
    Continue(PyroRow<'static>),
    /// The session is ending normally. Contains the final output row.
    End(PyroRow<'static>),
    /// The session has been terminated. No output row.
    Terminate,
}

/// An error from calling a session module.
#[derive(Debug)]
pub struct SessionCallError {
    pub error: PyroFailure,
}

impl std::fmt::Display for SessionCallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Session call error")
    }
}

impl std::error::Error for SessionCallError {}

impl From<WasmError> for SessionCallError {
    fn from(err: WasmError) -> Self {
        SessionCallError {
            error: PyroFailure {
                result: Err(err.to_string()),
                logs: PyroLogs::empty(),
            },
        }
    }
}

impl From<anyhow::Error> for SessionCallError {
    fn from(err: anyhow::Error) -> Self {
        SessionCallError {
            error: PyroFailure {
                result: Err(err.to_string()),
                logs: PyroLogs::empty(),
            },
        }
    }
}

// Wasm export function names for session management
const SESSION_INPUT: &str = "new_session_input";
const SESSION_BORROW_OUTPUT: &str = "borrow_session_output";
const SESSION_FREE: &str = "free_session";
const SESSION_CALL: &str = "call_session_extern";

// ---------------------------------------------------------------------------
// Session — owns session input/output history, drives host↔wasm IO
// ---------------------------------------------------------------------------

/// A handle to a single wasm session.
///
/// Manages the session's input/output history in wasm linear memory by
/// calling the wasm-exported session management functions.
pub struct Session {
    session_id: u32,
    input_count: u32,
    output_count: u32,
}

impl Session {
    /// Create a new session handle.
    ///
    /// The session is not yet initialized in wasm — call `push_input` to
    /// begin the session with an initial input.
    pub fn new(session_id: u32) -> Self {
        Self {
            session_id,
            input_count: 0,
            output_count: 0,
        }
    }

    /// Push a new input row onto the session's input history.
    ///
    /// This calls the wasm-exported `new_session_input` to allocate space in
    /// linear memory, then copies the serialized row into that space.
    pub async fn push_input(
        &mut self,
        store: &mut Store<PyroState>,
        instance: &Instance,
        memory: Memory,
        input: &PyroRow<'_>,
    ) -> Result<(), WasmError> {
        // Ship the input row via rkyv into a PyroVec
        let input_row_owned = input.to_static();
        let input_vec = input_row_owned
            .ship()
            .map_err(|err| WasmError::InputMemory(wasmtime::Error::msg(err.to_string())))?;

        // Capture the slice data before consuming input_vec via view()
        let total_len = PyroParser::HEADER_SIZE + input_vec.len();
        let data_slice = input_vec.as_raw_slice().to_vec();

        // Allocate new session input buffer
        let new_input_fn = instance
            .get_typed_func::<(u32, u32), i32>(&mut *store, SESSION_INPUT)
            .map_err(|_| WasmError::MissingExport(SESSION_INPUT.to_string()))?;
        let ptr = new_input_fn
            .call_async(&mut *store, (self.session_id, total_len as u32))
            .await
            .map_err(WasmError::InputMemory)?;

        // Copy data into wasm memory
        let wasm_memory = memory.data_mut(&mut *store);
        let memory_len = wasm_memory.len();
        if (ptr as usize + total_len) > memory_len {
            return Err(WasmError::OutputMemory(wasmtime::Error::msg(format!(
                "wasm pointer {:#x} + {} out of bounds (memory size: {})",
                ptr, total_len, memory_len
            ))));
        }
        let dest = &mut wasm_memory[ptr as usize..ptr as usize + total_len];
        dest.copy_from_slice(&data_slice);

        self.input_count += 1;
        Ok(())
    }

    /// Call the session module's `call_session_extern` export.
    ///
    /// This advances the session by one step, processing the pending input
    /// and returning the session response type and output row (if any).
    ///
    /// The session's input/output invariant is maintained automatically:
    /// inputs.len() == outputs.len() + 1 at call time.
    #[tracing::instrument(skip_all)]
    pub async fn call(
        &mut self,
        store: &mut Store<PyroState>,
        instance: &Instance,
        memory: Memory,
    ) -> Result<SessionResult, SessionCallError> {
        // Call call_session_extern(session_id)
        tracing::debug!("calling call_session_extern(session_id={})", self.session_id);
        let call_fn = instance
            .get_typed_func::<u32, i32>(&mut *store, SESSION_CALL)?;

        let call_output_ptr = call_fn
            .call_async(&mut *store, self.session_id)
            .await?;

        tracing::debug!("call_session_extern returned ptr={:#x}", call_output_ptr);

        // Read the return value to check if the session function succeeded
        if call_output_ptr != 0 {
            let wasm_memory = memory.data(&*store);
            if let Ok(row) = get_ref(wasm_memory, call_output_ptr as usize) {
                tracing::debug!("call_session_extern output: status={}, fn_id={}, row_len={}",
                    row.status_u8(), row.fn_id(), row.len());
            }
        }

        // The return pointer points to a PyroVec containing sessions.len() as the output.
        // The actual result is stored in SESSION_OUTPUT for this session at index output_count.
        // We read the result via borrow_session_output rather than the output registry.

        let borrow_output_fn = instance
            .get_typed_func::<(u32, u32), i32>(&mut *store, SESSION_BORROW_OUTPUT)?;

        tracing::debug!("calling borrow_session_output(session_id={}, index={})", self.session_id, self.output_count);

        let result_ptr = borrow_output_fn
            .call_async(&mut *store, (self.session_id, self.output_count))
            .await?;

        tracing::debug!("borrow_session_output returned ptr={:#x}", result_ptr);

        if result_ptr == 0 {
            return Err(SessionCallError {
                error: PyroFailure {
                    result: Err(
                        "borrow_session_output returned null — check session output history"
                            .to_string(),
                    ),
                    logs: PyroLogs::empty(),
                },
            });
        }

        // Read the result from wasm memory (zero-copy)
        let wasm_memory = memory.data(store.as_context());
        let result_view = get_ref(wasm_memory, result_ptr as usize).map_err(|e| {
            SessionCallError {
                error: PyroFailure {
                    result: Err(format!(
                        "Failed to read result from wasm memory at {:#x}: {}",
                        result_ptr, e
                    )),
                    logs: PyroLogs::empty(),
                },
            }
        })?;

        // Check for pyro-level errors forwarded from the host.
        if let Err(e) = result_view.parse_as_error() {
            return Err(SessionCallError {
                error: PyroFailure {
                    result: Err(e.to_string()),
                    logs: PyroLogs::empty(),
                },
            });
        }

        let pyref = result_view.py_ref();
        let fn_id = result_view.fn_id();

        match result_view.status() {
            Ok(DataStatus::RkyvValid) => {
                let row = PyroRow::expose_view(pyref).map_err(|e| SessionCallError {
                    error: PyroFailure {
                        result: Err(e.to_string()),
                        logs: PyroLogs::empty(),
                    },
                })?;
                let row = PyroRow::from(&*row).to_static();

                let result = match fn_id {
                    0 => SessionResult::Continue(row), // Continue
                    1 => SessionResult::End(row),       // End
                    2 => SessionResult::Terminate,      // Terminate
                    _ => {
                        // Unknown fn_id — treat as Continue
                        tracing::warn!(
                            "Session {} returned unknown fn_id: {}, treating as Continue",
                            self.session_id,
                            fn_id
                        );
                        SessionResult::Continue(row)
                    }
                };

                self.output_count += 1;
                Ok(result)
            }
            Ok(DataStatus::RkyvError) => {
                match serde_json::from_slice::<CapturedError>(result_view.as_ref()) {
                    Ok(error) => Err(SessionCallError {
                        error: PyroFailure {
                            result: Ok(error),
                            logs: PyroLogs::empty(),
                        },
                    }),
                    Err(_) => Err(SessionCallError {
                        error: PyroFailure {
                            result: Err(
                                ParseError::UnknownStatus(result_view.status_u8()).to_string(),
                            ),
                            logs: PyroLogs::empty(),
                        },
                    }),
                }
            }
            _ => Err(SessionCallError {
                error: PyroFailure {
                    result: Err(ParseError::UnknownStatus(result_view.status_u8()).to_string()),
                    logs: PyroLogs::empty(),
                },
            }),
        }
    }

    /// Get the number of inputs in this session's history.
    pub fn input_count(&self) -> u32 {
        self.input_count
    }

    /// Get the number of outputs in this session's history.
    pub fn output_count(&self) -> u32 {
        self.output_count
    }

    /// Get the session ID.
    pub fn session_id(&self) -> u32 {
        self.session_id
    }

    /// Free the session and all its input/output history.
    ///
    /// After calling this, the session is invalid and must not be used.
    pub async fn free(
        &self,
        store: &mut Store<PyroState>,
        instance: &Instance,
    ) -> Result<(), WasmError> {
        let free_fn = instance
            .get_typed_func::<u32, ()>(&mut *store, SESSION_FREE)
            .map_err(|_| WasmError::MissingExport(SESSION_FREE.to_string()))?;
        free_fn
            .call_async(&mut *store, self.session_id)
            .await
            .map_err(classify_error)?;
        Ok(())
    }
}

/// Attempts to downcast an anyhow::Error into specific pyro error types.
fn classify_error(error: anyhow::Error) -> WasmError {
    if error.is::<WasmError>() {
        return error.downcast().unwrap();
    }
    if error.is::<PyroError>() {
        return WasmError::Pyro(error.downcast().unwrap());
    }
    WasmError::Unknown(error)
}
