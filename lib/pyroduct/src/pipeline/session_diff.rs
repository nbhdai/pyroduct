use std::sync::Arc;

use arrow::array::RecordBatch;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::CapturedError;
use crate::format::log_wal::{LogEntry, LogWal};
use crate::module::{PyroInstance, sessions::SessionResult};
use crate::{
    format::{PyroFailure, PyroLogs, value::PyroRow},
    pipeline::{PipelineResult, PyroError},
};

use super::data::DataManager;

// =============================================================================
// Pipeline
// =============================================================================

#[derive(Clone, Debug, PartialEq)]
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

impl SessionDiffExecutionRecord {
    pub fn row_index(&self) -> usize {
        match self {
            SessionDiffExecutionRecord::Success { row_index, .. } => *row_index,
            SessionDiffExecutionRecord::Failure { row_index, .. } => *row_index,
        }
    }

    pub fn row(&self) -> Option<&PyroRow<'static>> {
        match self {
            SessionDiffExecutionRecord::Success { success, .. } => Some(success),
            SessionDiffExecutionRecord::Failure { input, .. } => Some(input),
        }
    }
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
    pub output_manager: DataManager,
    pub log_dir: std::path::PathBuf,
    pub output_dir: std::path::PathBuf,
    pub wal_capacity: usize,
    pub active_sessions: std::collections::HashMap<u32, ActiveSession>,
    pub callbacks: Vec<(uuid::Uuid, crate::pipeline::Callback)>,
}

impl SessionDiffPipeline {
    pub fn next_session_id(&self) -> u32 {
        let mut id = self.output_manager.len() as u32;
        while self.active_sessions.contains_key(&id) {
            id += 1;
        }
        id
    }

    async fn get_or_open_session(
        &mut self,
        session_id: u32,
    ) -> Result<&mut ActiveSession, PyroError> {
        if !self.active_sessions.contains_key(&session_id) {
            let log_dir = self.log_dir.join(format!("session_log_{}", session_id));
            let data_path = self.output_dir.join(format!("session_val_{}", session_id));

            let log_wal = LogWal::open(log_dir, self.wal_capacity)
                .await
                .map_err(|io| {
                    PyroError::local_io(
                        CapturedError::new("Unable to open individual log wal").with_source(io),
                    )
                })?;
            let data_wal =
                crate::format::value::arrow::wal::WalWriter::open(data_path).map_err(|io| {
                    PyroError::local_io(
                        CapturedError::new("Unable to open individual data wal").with_source(io),
                    )
                })?;

            self.active_sessions
                .insert(session_id, ActiveSession { log_wal, data_wal });
        }
        Ok(self.active_sessions.get_mut(&session_id).unwrap())
    }

    async fn rollup_and_cleanup_session(&mut self, session_id: u32) -> Result<(), PyroError> {
        let data_path = self.output_dir.join(format!("session_val_{}", session_id));
        let wal_rows = crate::format::value::arrow::wal::recover(&data_path).unwrap_or_default();

        let mut in_list = Vec::new();
        let mut out_list = Vec::new();
        for row in wal_rows {
            if let Some(in_val) = row.get("input")
                && let crate::format::value::PyroValue::Group(g) = in_val
            {
                in_list.push(crate::format::value::PyroValue::Group(g.clone()));
            }
            if let Some(out_val) = row.get("output")
                && let crate::format::value::PyroValue::Group(g) = out_val
            {
                out_list.push(crate::format::value::PyroValue::Group(g.clone()));
            }
        }

        let rolled_up_row = PyroRow::from([
            ("inputs", crate::format::value::PyroValue::List(in_list)),
            ("outputs", crate::format::value::PyroValue::List(out_list)),
        ]);

        self.output_manager
            .push_record(session_id as usize, &rolled_up_row)
            .await?;

        let log_dir = self.log_dir.join(format!("session_log_{}", session_id));
        if log_dir.exists() {
            let mut reader = crate::format::log_wal::LogWalReader::open(&log_dir)
                .await
                .map_err(|io| {
                    PyroError::local_io(
                        CapturedError::new("Unable to open individual log wal reader")
                            .with_source(io),
                    )
                })?;

            while let Some(log_entry) = reader.next().await.map_err(|io| {
                PyroError::local_io(
                    CapturedError::new("Unable to read from individual log wal").with_source(io),
                )
            })? {
                let mut entry_to_write = log_entry;
                entry_to_write.row_index = session_id as usize;
                self.log_manager
                    .append(&entry_to_write)
                    .await
                    .map_err(|io| {
                        PyroError::local_io(
                            CapturedError::new("Unable to write to overall log wal")
                                .with_source(io),
                        )
                    })?;
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

        for (_, cb) in &mut self.callbacks {
            cb.execute(session_id as usize, &rolled_up_row).await;
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

        if let Err(e) = self
            .prep_session(session_id, prior_inputs, prior_outputs)
            .await
        {
            let logs = e.logs.clone();
            let log_entry = LogEntry {
                row_index,
                module_logs: logs.module_logs.clone(),
                capability_logs: logs.capability_logs.clone(),
                failure: Some(e.result.clone()),
            };
            let _ = self.log_manager.append(&log_entry).await;

            return match e.result {
                Ok(captured) => Err(PyroError::CodePanic(Box::new(captured))),
                Err(msg) => Err(PyroError::local(crate::error::ErrorKind::Transport(
                    Box::new(CapturedError::new(msg)),
                ))),
            };
        }

        match self.call(session_id, input).await {
            Ok(res) => {
                let (row, logs) = match res {
                    SessionResult::Continue { result, logs } => (result, logs),
                    SessionResult::End { result, logs } => (result, logs),
                    SessionResult::Terminate { logs } => (PyroRow::empty(), logs),
                };
                let record = SessionDiffExecutionRecord::Success {
                    row_index,
                    prior_input: prior_inputs
                        .iter()
                        .map(|r| r.clone().into_owned())
                        .collect(),
                    prior_output: prior_outputs
                        .iter()
                        .map(|r| r.clone().into_owned())
                        .collect(),
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
                    Err(msg) => Err(PyroError::local(crate::error::ErrorKind::Transport(
                        Box::new(CapturedError::new(msg)),
                    ))),
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
        if let Err(e) = self.step.prep_session(session_id, inputs, outputs).await {
            let _ = self
                .output_manager
                .set_session_status(session_id as usize, "failed");
            return Err(e);
        }

        let data_path = self.output_dir.join(format!("session_val_{}", session_id));
        let existing = crate::format::value::arrow::wal::recover(&data_path).unwrap_or_default();

        let active_logs = self.step.unpack_logs();
        let active = match self.get_or_open_session(session_id).await {
            Ok(act) => act,
            Err(e) => {
                let _ = self
                    .output_manager
                    .set_session_status(session_id as usize, "failed");
                return Err(PyroFailure {
                    result: Err(e.to_string()),
                    logs: active_logs.clone(),
                });
            }
        };

        let max_len = inputs.len().max(outputs.len());
        for i in 0..max_len {
            if i >= existing.len() {
                let in_val = inputs
                    .get(i)
                    .map(|r| crate::format::value::PyroValue::Group(r.clone().into_owned()))
                    .unwrap_or(crate::format::value::PyroValue::Null);
                let out_val = outputs
                    .get(i)
                    .map(|r| crate::format::value::PyroValue::Group(r.clone().into_owned()))
                    .unwrap_or(crate::format::value::PyroValue::Null);
                let row = PyroRow::from([("input", in_val), ("output", out_val)]);
                if let Err(e) = active.data_wal.append(i, &row).await {
                    let _ = self
                        .output_manager
                        .set_session_status(session_id as usize, "failed");
                    return Err(PyroFailure {
                        result: Err(e.to_string()),
                        logs: active_logs.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    pub async fn call(
        &mut self,
        session_id: u32,
        input: &PyroRow<'_>,
    ) -> Result<SessionResult, PyroFailure> {
        let res = self.step.call_session(session_id, input).await;

        // PERSIST STATUS
        match &res {
            Ok(SessionResult::Continue { .. }) => {
                let _ = self
                    .output_manager
                    .set_session_status(session_id as usize, "active");
            }
            Ok(SessionResult::End { .. }) | Ok(SessionResult::Terminate { .. }) => {
                let _ = self
                    .output_manager
                    .set_session_status(session_id as usize, "succeeded");
            }
            Err(_) => {
                let _ = self
                    .output_manager
                    .set_session_status(session_id as usize, "failed");
            }
        }

        let output_row = match &res {
            Ok(SessionResult::Continue { result: r, .. }) => {
                crate::format::value::PyroValue::Group(r.clone().into_owned())
            }
            Ok(SessionResult::End { result: r, .. }) => {
                crate::format::value::PyroValue::Group(r.clone().into_owned())
            }
            _ => crate::format::value::PyroValue::Null,
        };

        {
            let active = match self.get_or_open_session(session_id).await {
                Ok(act) => act,
                Err(e) => {
                    let logs = self.step.unpack_logs();
                    return Err(PyroFailure {
                        result: Err(e.to_string()),
                        logs,
                    });
                }
            };

            let record_index = active.data_wal.records_written() as usize;
            let step_row = PyroRow::from([
                (
                    "input",
                    crate::format::value::PyroValue::Group(input.clone().into_owned()),
                ),
                ("output", output_row),
            ]);
            let _ = active.data_wal.append(record_index, &step_row).await;
        }

        let logs = self.step.unpack_logs();
        let active = match self.get_or_open_session(session_id).await {
            Ok(act) => act,
            Err(e) => {
                let logs = self.step.unpack_logs();
                return Err(PyroFailure {
                    result: Err(e.to_string()),
                    logs,
                });
            }
        };

        let row_index = active.log_wal.total_entries();
        match &res {
            Ok(_) => {
                let log_entry = LogEntry {
                    row_index,
                    module_logs: logs.module_logs,
                    capability_logs: logs.capability_logs,
                    failure: None,
                };
                let _ = active.log_wal.append(&log_entry).await;
            }
            Err(e) => {
                let log_entry = LogEntry {
                    row_index,
                    module_logs: e.logs.module_logs.clone(),
                    capability_logs: e.logs.capability_logs.clone(),
                    failure: Some(e.result.clone()),
                };
                let _ = active.log_wal.append(&log_entry).await;
            }
        }

        match &res {
            Ok(SessionResult::End { .. }) | Ok(SessionResult::Terminate { .. }) => {
                if let Err(e) = self.rollup_and_cleanup_session(session_id).await {
                    tracing::error!(
                        "Failed to rollup and cleanup session {}: {:?}",
                        session_id,
                        e
                    );
                }
            }
            Err(_) => {
                if let Err(e) = self.rollup_and_cleanup_session(session_id).await {
                    tracing::error!(
                        "Failed to rollup and cleanup failed session {}: {:?}",
                        session_id,
                        e
                    );
                }
            }
            _ => {}
        }

        res
    }

    pub async fn get_record(
        &self,
        session_id: u32,
    ) -> Result<SessionDiffExecutionRecord, PyroError> {
        tracing::debug!(session_id, "get_record: starting lookup");

        // 1. Determine persistent status
        let status = self
            .output_manager
            .get_session_status(session_id as usize)?
            .unwrap_or_else(|| "active".to_string());

        // 2. Retrieve all steps (input, output) for the session
        let mut steps = Vec::new();
        let data_path = self.output_dir.join(format!("session_val_{}", session_id));
        let data_wal_file = data_path.with_extension("pyrowal");

        if data_wal_file.exists() {
            let wal_rows =
                crate::format::value::arrow::wal::recover(&data_path).unwrap_or_default();
            for row in wal_rows {
                let input = match row.get("input") {
                    Some(crate::format::value::PyroValue::Group(g)) => g.clone(),
                    _ => PyroRow::empty(),
                };
                let output = match row.get("output") {
                    Some(crate::format::value::PyroValue::Group(g)) => g.clone(),
                    _ => PyroRow::empty(),
                };
                steps.push((input, output));
            }
        } else {
            // Check rolled up rows in output_manager
            if let Ok(rolled_up_row) = self.output_manager.get_record(session_id as usize) {
                let mut inputs = Vec::new();
                let mut outputs = Vec::new();
                if let Some(crate::format::value::PyroValue::List(list_vals)) =
                    rolled_up_row.get("inputs")
                {
                    for val in list_vals {
                        if let crate::format::value::PyroValue::Group(r) = val {
                            inputs.push(r.clone());
                        }
                    }
                }
                if let Some(crate::format::value::PyroValue::List(list_vals)) =
                    rolled_up_row.get("outputs")
                {
                    for val in list_vals {
                        if let crate::format::value::PyroValue::Group(r) = val {
                            outputs.push(r.clone());
                        }
                    }
                }
                let max_len = inputs.len().max(outputs.len());
                for i in 0..max_len {
                    let input = inputs.get(i).cloned().unwrap_or_else(PyroRow::empty);
                    let output = outputs.get(i).cloned().unwrap_or_else(PyroRow::empty);
                    steps.push((input, output));
                }
            }
        }

        // 3. Retrieve logs
        let mut logs = PyroLogs::empty();
        let mut log_failure = None;
        let log_dir = self.log_dir.join(format!("session_log_{}", session_id));
        if log_dir.exists() {
            if let Ok(mut reader) = crate::format::log_wal::LogWalReader::open(&log_dir).await
                && let Ok(entries) = reader.read_all().await
                && let Some(last_entry) = entries.last()
            {
                logs = PyroLogs {
                    module_logs: last_entry.module_logs.clone(),
                    capability_logs: last_entry.capability_logs.clone(),
                };
                log_failure = last_entry.failure.clone();
            }
        } else {
            if let Ok(mut reader) = crate::format::log_wal::LogWalReader::open(&self.log_dir).await
                && let Ok(entries) = reader.read_all().await
                && let Some(entry) = entries.iter().rfind(|e| e.row_index == session_id as usize)
            {
                logs = PyroLogs {
                    module_logs: entry.module_logs.clone(),
                    capability_logs: entry.capability_logs.clone(),
                };
                log_failure = entry.failure.clone();
            }
        }

        // 4. Reconstruct prior_input, prior_output, input, success/failure
        let is_failed = status == "failed";

        let (prior_input, prior_output, input, success) = if steps.is_empty() {
            let input_row = self
                .output_manager
                .get_record(session_id as usize)
                .unwrap_or_else(|_| PyroRow::empty());
            (Vec::new(), Vec::new(), input_row, PyroRow::empty())
        } else {
            let len = steps.len();
            let mut prior_in = Vec::with_capacity(len - 1);
            let mut prior_out = Vec::with_capacity(len - 1);
            for step in steps.iter().take(len - 1) {
                prior_in.push(step.0.clone());
                prior_out.push(step.1.clone());
            }
            let input = steps[len - 1].0.clone();
            let success = steps[len - 1].1.clone();
            (prior_in, prior_out, input, success)
        };

        if is_failed {
            let failure_err = if let Some(err_res) = log_failure {
                err_res
            } else {
                Err("Session failed".to_string())
            };

            Ok(SessionDiffExecutionRecord::Failure {
                row_index: session_id as usize,
                prior_input,
                prior_output,
                input,
                failure: failure_err,
                logs,
            })
        } else {
            Ok(SessionDiffExecutionRecord::Success {
                row_index: session_id as usize,
                prior_input,
                prior_output,
                input,
                success,
                logs,
            })
        }
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
    ) -> PipelineResult<(
        Vec<SessionDiffExecutionRecord>,
        Vec<SessionDiffExecutionRecord>,
    )> {
        todo!()
    }
}
