use std::sync::Arc;

use arrow::array::RecordBatch;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::CapturedError;
use crate::format::log_wal::{LogEntry, LogWal};
use crate::module::{PyroInstance, sessions::SessionResult};
use crate::{
    format::{
        PyroFailure, PyroLogs,
        value::PyroRow,
    },
    pipeline::{PipelineResult, PyroError},
};

use super::data::DataManager;

// =============================================================================
// Pipeline
// =============================================================================

#[derive(Clone)]
pub enum SessionDiffExecutionRecord {
    Success {
        row_index: usize,
        prior_input: Vec<PyroRow<'static>>,
        prior_output: Vec<PyroRow<'static>>,
        input: PyroRow<'static>,
        success: PyroRow<'static>,
        logs: PyroLogs,
    },
    Failure {
        row_index: usize,
        prior_input: Vec<PyroRow<'static>>,
        prior_output: Vec<PyroRow<'static>>,
        input: PyroRow<'static>,
        failure: Result<CapturedError, String>,
        logs: PyroLogs,
    },
}

pub struct ActiveSession {
    pub log_wal: LogWal,
    pub data_wal: crate::format::value::arrow::wal::WalWriter,
}

pub struct SessionDiffPipeline {
    pub step: PyroInstance,
    pub success_log_retention_secs: u64,
    pub error_log_retention_secs: u64,
    pub log_manager: LogWal,
    pub input_manager: DataManager,
    pub output_manager: DataManager,
    pub log_dir: std::path::PathBuf,
    pub output_dir: std::path::PathBuf,
    pub wal_capacity: usize,
    pub active_sessions: std::collections::HashMap<u32, ActiveSession>,
}

impl SessionDiffPipeline {
    async fn get_or_open_session(&mut self, session_id: u32) -> Result<&mut ActiveSession, PyroError> {
        if !self.active_sessions.contains_key(&session_id) {
            let log_dir = self.log_dir.join(format!("session_log_{}", session_id));
            let data_path = self.output_dir.join(format!("session_val_{}", session_id));

            let log_wal = LogWal::open(log_dir, self.wal_capacity).await
                .map_err(|io| PyroError::local_io(CapturedError::new("Unable to open individual log wal").with_source(io)))?;
            let data_wal = crate::format::value::arrow::wal::WalWriter::open(data_path)
                .map_err(|io| PyroError::local_io(CapturedError::new("Unable to open individual data wal").with_source(io)))?;

            self.active_sessions.insert(session_id, ActiveSession { log_wal, data_wal });
        }
        Ok(self.active_sessions.get_mut(&session_id).unwrap())
    }

    async fn rollup_and_cleanup_session(&mut self, session_id: u32) -> Result<(), PyroError> {
        let data_path = self.output_dir.join(format!("session_val_{}", session_id));
        let wal_rows = crate::format::value::arrow::wal::recover(&data_path).unwrap_or_default();

        let mut in_list = Vec::new();
        let mut out_list = Vec::new();
        for row in wal_rows {
            if let Some(in_val) = row.get("input") {
                if let crate::format::value::PyroValue::Group(g) = in_val {
                    in_list.push(crate::format::value::PyroValue::Group(g.clone()));
                }
            }
            if let Some(out_val) = row.get("output") {
                if let crate::format::value::PyroValue::Group(g) = out_val {
                    out_list.push(crate::format::value::PyroValue::Group(g.clone()));
                }
            }
        }

        let rolled_up_row = PyroRow::from([
            ("inputs", crate::format::value::PyroValue::List(in_list)),
            ("outputs", crate::format::value::PyroValue::List(out_list)),
        ]);

        self.output_manager.push_record(&rolled_up_row)?;

        let log_dir = self.log_dir.join(format!("session_log_{}", session_id));
        if log_dir.exists() {
            let mut reader = crate::format::log_wal::LogWalReader::open(&log_dir).await
                .map_err(|io| PyroError::local_io(CapturedError::new("Unable to open individual log wal reader").with_source(io)))?;

            while let Some(log_entry) = reader.next().await
                .map_err(|io| PyroError::local_io(CapturedError::new("Unable to read from individual log wal").with_source(io)))?
            {
                let overall_row_index = self.log_manager.total_entries();
                let mut entry_to_write = log_entry;
                entry_to_write.row_index = overall_row_index;
                self.log_manager.append(&entry_to_write).await
                    .map_err(|io| PyroError::local_io(CapturedError::new("Unable to write to overall log wal").with_source(io)))?;
            }
        }

        let _ = self.log_manager.flush().await;

        self.active_sessions.remove(&session_id);

        let data_path = self.output_dir.join(format!("session_val_{}", session_id));
        let data_wal_file = data_path.with_extension("pyrowal");
        if data_wal_file.exists() {
            let _ = std::fs::remove_file(data_wal_file);
        }

        if log_dir.exists() {
            let _ = tokio::fs::remove_dir_all(log_dir).await;
        }

        Ok(())
    }

    pub async fn process(
        &mut self,
        row_index: usize,
        prior_inputs: &[PyroRow<'_>],
        prior_outputs: &[PyroRow<'_>],
        input: &PyroRow<'_>,
    ) -> Result<SessionDiffExecutionRecord, PyroError> {
        let session_id = row_index as u32;

        if let Err(e) = self.prep_session(session_id, prior_inputs, prior_outputs).await {
            let logs = e.logs.clone();

            let log_entry = LogEntry {
                row_index,
                module_logs: logs.module_logs.clone(),
                capability_logs: logs.capability_logs.clone(),
                failure: None,
                success_index: None,
            };
            let _ = self.log_manager.append(&log_entry).await;

            return match e.result {
                Ok(captured) => Err(PyroError::CodePanic(Box::new(captured))),
                Err(msg) => Err(PyroError::local(crate::error::ErrorKind::Transport(Box::new(CapturedError::new(msg))))),
            };
        }

        match self.call(session_id, input).await {
            Ok(res) => {
                let row = match res {
                    SessionResult::Continue(r) => r,
                    SessionResult::End(r) => r,
                    SessionResult::Terminate => PyroRow::empty(),
                };

                let logs = self.step.unpack_logs();
                let record = SessionDiffExecutionRecord::Success {
                    row_index,
                    prior_input: prior_inputs.iter().map(|r| r.clone().into_owned()).collect(),
                    prior_output: prior_outputs.iter().map(|r| r.clone().into_owned()).collect(),
                    input: input.clone().into_owned(),
                    success: row,
                    logs: logs.clone(),
                };

                let _ = self.close_session(session_id).await;
                Ok(record)
            }
            Err(e) => {
                let _ = self.close_session(session_id).await;
                match e.result {
                    Ok(captured) => Err(PyroError::CodePanic(Box::new(captured))),
                    Err(msg) => Err(PyroError::local(crate::error::ErrorKind::Transport(Box::new(CapturedError::new(msg))))),
                }
            }
        }
    }

    pub async fn prep_session(
        &mut self,
        session_id: u32,
        inputs: &[PyroRow<'_>],
        outputs: &[PyroRow<'_>],
    ) -> Result<(), PyroFailure> {
        self.step.prep_session(session_id, inputs, outputs).await?;

        let data_path = self.output_dir.join(format!("session_val_{}", session_id));
        let existing = crate::format::value::arrow::wal::recover(&data_path).unwrap_or_default();

        let active_logs = self.step.unpack_logs();
        let active = self.get_or_open_session(session_id).await
            .map_err(|e| PyroFailure {
                result: Err(e.to_string()),
                logs: active_logs.clone(),
            })?;

        let max_len = inputs.len().max(outputs.len());
        for i in 0..max_len {
            if i >= existing.len() {
                let in_val = inputs.get(i).map(|r| crate::format::value::PyroValue::Group(r.clone().into_owned())).unwrap_or(crate::format::value::PyroValue::Null);
                let out_val = outputs.get(i).map(|r| crate::format::value::PyroValue::Group(r.clone().into_owned())).unwrap_or(crate::format::value::PyroValue::Null);
                let row = PyroRow::from([
                    ("input", in_val),
                    ("output", out_val),
                ]);
                active.data_wal.append(i, &row)
                    .map_err(|e| PyroFailure {
                        result: Err(e.to_string()),
                        logs: active_logs.clone(),
                    })?;
            }
        }

        Ok(())
    }

    pub async fn call(
        &mut self,
        session_id: u32,
        input: &PyroRow<'_>,
    ) -> Result<SessionResult, PyroFailure> {
        let active_logs = self.step.unpack_logs();

        let res = self.step.call_session(session_id, input).await;
        let logs = self.step.unpack_logs();

        let output_row = match &res {
            Ok(SessionResult::Continue(r)) => crate::format::value::PyroValue::Group(r.clone().into_owned()),
            Ok(SessionResult::End(r)) => crate::format::value::PyroValue::Group(r.clone().into_owned()),
            _ => crate::format::value::PyroValue::Null,
        };

        let active = self.get_or_open_session(session_id).await
            .map_err(|e| PyroFailure {
                result: Err(e.to_string()),
                logs: active_logs.clone(),
            })?;

        let record_index = active.data_wal.records_written() as usize;
        let step_row = PyroRow::from([
            ("input", crate::format::value::PyroValue::Group(input.clone().into_owned())),
            ("output", output_row),
        ]);
        active.data_wal.append(record_index, &step_row)
            .map_err(|e| PyroFailure {
                result: Err(e.to_string()),
                logs: active_logs.clone(),
            })?;

        let row_index = active.log_wal.total_entries();
        match &res {
            Ok(_) => {
                let log_entry = LogEntry {
                    row_index,
                    module_logs: logs.module_logs,
                    capability_logs: logs.capability_logs,
                    failure: None,
                    success_index: None,
                };
                let _ = active.log_wal.append(&log_entry).await;
            }
            Err(e) => {
                let log_entry = LogEntry {
                    row_index,
                    module_logs: e.logs.module_logs.clone(),
                    capability_logs: e.logs.capability_logs.clone(),
                    failure: e.result.as_ref().ok().cloned(),
                    success_index: None,
                };
                let _ = active.log_wal.append(&log_entry).await;
            }
        }

        match &res {
            Ok(SessionResult::End(_)) | Ok(SessionResult::Terminate) => {
                if let Err(e) = self.rollup_and_cleanup_session(session_id).await {
                    tracing::error!("Failed to rollup and cleanup session {}: {:?}", session_id, e);
                }
            }
            Err(_) => {
                if let Err(e) = self.rollup_and_cleanup_session(session_id).await {
                    tracing::error!("Failed to rollup and cleanup failed session {}: {:?}", session_id, e);
                }
            }
            _ => {}
        }

        res
    }

    pub async fn close_session(&mut self, session_id: u32) -> Result<(), PyroFailure> {
        self.step.close_session(session_id).await
    }

    pub async fn session_inputs(
        &mut self,
        session_id: u32,
    ) -> Result<Vec<PyroRow<'_>>, PyroFailure> {
        self.step.session_inputs(session_id).await
    }

    pub async fn session_outputs(
        &mut self,
        session_id: u32,
    ) -> Result<Vec<PyroRow<'_>>, PyroFailure> {
        self.step.session_outputs(session_id).await
    }

    pub fn session_lengths(&self, session_id: u32) -> Option<(u32, u32)> {
        self.step.session_lengths(session_id)
    }
}

// =============================================================================
// Failure
// =============================================================================

/// A module returned a logic error. The pipeline stopped, but we keep
/// whatever data was accumulated before the failing step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Failure {
    pub row_index: usize,
    pub error: String,
    pub partial_data: PyroRow<'static>,
}

// =============================================================================
// PipelinePool
// =============================================================================

pub struct SessionDiffPipelinePool {
    _pipelines: Arc<Mutex<Vec<SessionDiffPipeline>>>,
}

impl SessionDiffPipelinePool {
    pub fn new(pipelines: Vec<SessionDiffPipeline>) -> Self {
        Self {
            _pipelines: Arc::new(Mutex::new(pipelines)),
        }
    }

    /// Distribute rows across available pipelines and collect results.
    ///
    /// Returns the successful rows (sorted by original index) and any
    /// per-row failures.
    pub async fn process_batch(
        &self,
        _batch: &RecordBatch,
    ) -> PipelineResult<(Vec<SessionDiffExecutionRecord>, Vec<SessionDiffExecutionRecord>)> {
        todo!()
    }
}
