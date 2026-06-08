//! Host-side harness for calling the wasm module's session entry functions.
//!
//! Exposes session-related functions directly on `PyroInstance`.
//!
//! All data crosses the boundary as rkyv-serialized PyroVecs. The host uses
//! `Bridgeable::ship()` to produce them and `Bridgeable::expose_view()` to
//! get zero-copy access to the archived data in wasm linear memory.

use wasmtime::TypedFunc;

use crate::format::SessionResult;
use crate::format::bridgeable::Bridgeable;
use crate::format::header::{PyroData, PyroHeader};
use crate::format::{ParseError, PyroFailure, PyroRow, header::DataStatus};
use crate::{CapturedError, PyroError};
use pyro_spec::ModuleKind;

use super::call::PyroCallIo;
use super::{PyroInstance, classify_error};

impl PyroInstance {
    /// Push input and call a session module for one step.
    ///
    /// This encapsulates the full session call lifecycle: pushing the input row
    /// into wasm linear memory, calling `call_session_extern`, reading the result
    /// from the session's output slot, and updating session state.
    pub async fn prep_session(
        &mut self,
        session_id: u32,
        inputs: &[PyroRow<'_>],
        outputs: &[PyroRow<'_>],
    ) -> Result<(), PyroFailure> {
        tracing::debug!(
            session_id,
            inputs_len = inputs.len(),
            outputs_len = outputs.len(),
            "Preparing session"
        );
        let mut io = PyroCallIo::new(&mut self.store, self.memory);

        for input in inputs {
            let input_row_owned = input.to_static();
            let input_vec = input_row_owned
                .ship()
                .map_err(|e| Self::pack_setup_pyro_error(session_id, e))?;
            let input_view = input_vec.view();
            io.new_session_input(session_id, input_view)
                .await
                .map_err(|e| Self::pack_setup_pyro_error(session_id, e))?;
        }

        for output in outputs {
            let output_row_owned = output.to_static();
            let output_vec = output_row_owned
                .ship()
                .map_err(|e| Self::pack_setup_pyro_error(session_id, e))?;
            let output_view = output_vec.view();
            io.new_session_output(session_id, output_view)
                .await
                .map_err(|e| Self::pack_setup_pyro_error(session_id, e))?;
        }

        let state = self.session_states.entry(session_id).or_default();
        state.input_len = inputs.len() as u32;
        state.output_len = outputs.len() as u32;

        Ok(())
    }

    /// Push input and call a session module for one step.
    ///
    /// This encapsulates the full session call lifecycle: pushing the input row
    /// into wasm linear memory, calling `call_session_extern`, reading the result
    /// from the session's output slot, and updating session state.
    pub async fn call_session(
        &mut self,
        session_id: u32,
        input: &PyroRow<'_>,
    ) -> Result<SessionResult, PyroFailure> {
        tracing::debug!(session_id, "Calling session");
        // 1. Ship input into session history
        let input_row_owned = input.to_static();
        let input_vec = input_row_owned
            .ship()
            .map_err(|err| self.pack_pyro_error(session_id, err))?;
        let input_view = input_vec.view();

        let mut io = PyroCallIo::new(&mut self.store, self.memory);
        io.new_session_input(session_id, input_view)
            .await
            .map_err(|err| self.pack_pyro_error(session_id, err))?;

        // 2. Call the session export
        let entry: TypedFunc<i32, i32> = self
            .instance
            .get_typed_func(&mut self.store, "call_session_extern")
            .map_err(|e| {
                PyroError::CodePanic(
                    CapturedError::new(format!("Missing call_session_extern: {}", e)).into(),
                )
            })
            .map_err(|err| self.pack_pyro_error(session_id, err))?;

        let output_ptr = match entry.call_async(&mut self.store, session_id as i32).await {
            Ok(ptr) => ptr,
            Err(e) => {
                let mut io = PyroCallIo::new(&mut self.store, self.memory);
                if let Ok(Some(err_vec)) = io.get_panic_error().await {
                    match err_vec.status() {
                        Ok(DataStatus::RkyvError) => match serde_json::from_slice(&err_vec) {
                            Ok(error) => return Err(self.pack_user_error(session_id, error)),
                            Err(error) => {
                                return Err(self.pack_pyro_error(
                                    session_id,
                                    PyroError::capture_json(error, &err_vec),
                                ));
                            }
                        },
                        _ => match err_vec.parse_as_error() {
                            Ok(_) => {
                                return Err(self.pack_pyro_error(session_id, classify_error(e)));
                            }
                            Err(err) => return Err(self.pack_pyro_error(session_id, err)),
                        },
                    }
                }
                return Err(self.pack_pyro_error(session_id, classify_error(e)));
            }
        };

        tracing::debug!(session_id, ?output_ptr, "Session call returned");

        // 3. Read Output
        let mut io = PyroCallIo::new(&mut self.store, self.memory);
        let output_vec = io
            .get_output(output_ptr)
            .await
            .map_err(|err| self.pack_pyro_error(session_id, err))?;

        // 4. Parse Result
        let result_view = output_vec.view();
        result_view
            .parse_as_error()
            .map_err(|err| self.pack_pyro_error(session_id, err))?;

        let pyref = result_view.py_ref();
        let fn_id = result_view.fn_id();

        let logs = self.unpack_logs();

        let res = match result_view.status() {
            Ok(DataStatus::RkyvValid) => {
                let row = PyroRow::expose_view(pyref)
                    .map_err(|err| self.pack_pyro_error(session_id, err))?;
                let row_static = PyroRow::from(&*row).to_static();

                let result = match fn_id {
                    0 => SessionResult::Continue {
                        result: row_static,
                        session_id,
                        logs,
                    },
                    1 => SessionResult::End {
                        result: row_static,
                        session_id,
                        logs,
                    },
                    _ => SessionResult::Terminate { session_id, logs },
                };
                tracing::debug!(session_id, fn_id, "Session result: Valid");
                Ok(result)
            }
            Ok(DataStatus::Empty) => {
                let result = match fn_id {
                    0 => {
                        return Err(self.pack_user_error(
                            session_id,
                            CapturedError::new("Session returned 'continue', but provided no data"),
                        ));
                    }
                    1 => {
                        return Err(self.pack_user_error(
                            session_id,
                            CapturedError::new("Session returned 'end', but provided no data"),
                        ));
                    }
                    _ => SessionResult::Terminate { session_id, logs },
                };
                tracing::debug!(session_id, fn_id, "Session result: Valid");
                Ok(result)
            }
            Ok(DataStatus::RkyvError) => {
                tracing::debug!(session_id, "Session result: RkyvError");
                match serde_json::from_slice(&result_view) {
                    Ok(error) => Err(self.pack_user_error(session_id, error)),
                    Err(error) => Err(self
                        .pack_pyro_error(session_id, PyroError::capture_json(error, &result_view))),
                }
            }
            Ok(DataStatus::CodeError) => {
                tracing::debug!("Session status: CodeError");
                match serde_json::from_slice(&result_view) {
                    Ok(error) => Err(self.pack_user_error(session_id, error)),
                    Err(error) => Err(self
                        .pack_pyro_error(session_id, PyroError::capture_json(error, &result_view))),
                }
            }
            _ => {
                tracing::debug!(
                    session_id,
                    status = result_view.status_u8(),
                    "Session result: Unknown"
                );
                Err(self.pack_pyro_error(
                    session_id,
                    PyroError::Header(ParseError::UnknownStatus(result_view.status_u8())),
                ))
            }
        };

        // 5. Update state
        if res.is_ok() {
            let state = self.session_states.entry(session_id).or_default();
            state.input_len += 1;
            state.output_len += 1;
        }

        res
    }

    /// Get all serialized inputs recorded in the wasm session's history.
    pub async fn session_inputs(
        &mut self,
        session_id: u32,
    ) -> Result<Vec<PyroRow<'_>>, PyroFailure> {
        tracing::debug!(session_id, "Getting session inputs");
        let state = self.session_states.get(&session_id).ok_or_else(|| {
            self.pack_pyro_error(
                session_id,
                PyroError::not_found(format!("Session {} not found", session_id)),
            )
        })?;
        let len = state.input_len;

        let mut io = PyroCallIo::new(&mut self.store, self.memory);
        let actual_len = io
            .session_input_length(session_id)
            .await
            .map_err(|e| Self::pack_setup_pyro_error(session_id, e))?;

        let mut inputs = Vec::with_capacity(actual_len as usize);

        let is_session_diff = self.spec.func.kind == ModuleKind::SessionDiff;

        if is_session_diff {
            debug_assert_eq!(actual_len, len);
        } else {
            debug_assert!(actual_len == 2 * len || actual_len == 2 * len - 1);
        }

        for i in 0..actual_len {
            let view = io
                .borrow_session_input(session_id, i)
                .await
                .map_err(|e| Self::pack_setup_pyro_error(session_id, e))?;
            let row = if view.status() == Ok(DataStatus::Empty) {
                PyroRow::empty()
            } else {
                let exposed = PyroRow::expose_view(view)
                    .map_err(|e| Self::pack_setup_pyro_error(session_id, e))?;
                PyroRow::from(&*exposed)
            };
            inputs.push(row);
        }

        tracing::debug!(session_id, count = inputs.len(), "Retrieved session inputs");
        Ok(inputs)
    }

    /// Get all serialized outputs recorded in the wasm session's history.
    pub async fn session_outputs(
        &mut self,
        session_id: u32,
    ) -> Result<Vec<PyroRow<'_>>, PyroFailure> {
        tracing::debug!(session_id, "Getting session outputs");
        let state = self.session_states.get(&session_id).ok_or_else(|| {
            self.pack_pyro_error(
                session_id,
                PyroError::not_found(format!("Session {} not found", session_id)),
            )
        })?;
        let len = state.output_len;

        let mut io = PyroCallIo::new(&mut self.store, self.memory);
        let mut outputs = Vec::with_capacity(len as usize);
        let actual_len = io
            .session_output_length(session_id)
            .await
            .map_err(|e| Self::pack_setup_pyro_error(session_id, e))?;
        debug_assert_eq!(actual_len, len);

        for i in 0..actual_len {
            let view = io
                .borrow_session_output(session_id, i)
                .await
                .map_err(|e| Self::pack_setup_pyro_error(session_id, e))?;
            let row = PyroRow::expose_view(view)
                .map_err(|e| Self::pack_setup_pyro_error(session_id, e))?;
            outputs.push(PyroRow::from(&*row));
        }

        tracing::debug!(
            session_id,
            count = outputs.len(),
            "Retrieved session outputs"
        );
        Ok(outputs)
    }

    /// Close the wasm session and free its resources.
    pub async fn close_session(&mut self, session_id: u32) -> Result<(), PyroFailure> {
        tracing::debug!(session_id, "Closing session");
        let mut io = PyroCallIo::new(&mut self.store, self.memory);
        io.free_session(session_id)
            .await
            .map_err(|err| self.pack_pyro_error(session_id, err))?;
        self.session_states.remove(&session_id);
        tracing::debug!(session_id, "Session closed");
        Ok(())
    }

    /// Get current input/output counts for the session.
    pub fn session_lengths(&self, session_id: u32) -> Option<(u32, u32)> {
        self.session_states
            .get(&session_id)
            .map(|s| (s.input_len, s.output_len))
    }
}
