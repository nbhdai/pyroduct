use std::sync::Arc;

use arrow::array::RecordBatch;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::CapturedError;
use crate::format::log_wal::{LogEntry, LogWal};
use crate::module::PyroInstance;
use crate::{
    format::{PyroFailure, PyroLogs, SessionResult, value::PyroRow},
    pipeline::{PipelineResult, PyroError},
};

use super::data::DataManager;

// =============================================================================
// Pipeline
// =============================================================================

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
            if let Ok(Some(status)) = self.output_manager.get_session_status(session_id as usize) {
                if status == "succeeded" || status == "failed" {
                    return Err(PyroError::validation(crate::capture!(
                        "Cannot resume closed session {}",
                        session_id
                    )));
                }
            }

            let log_dir = self.log_dir.join(format!("session_log_{}", session_id));
            let data_path = self.output_dir.join(format!("session_val_{}", session_id));

            let log_wal = LogWal::open(log_dir, self.wal_capacity)
                .await
                .map_err(|io| {
                    PyroError::local_io(
                        CapturedError::new("Unable to open individual log wal").with_source(io),
                    )
                })?;
            let input_schema = self.step.spec().func.input.clone();
            let output_schema = self.step.spec().func.output.clone();
            let wal_schema = crate::format::value::PyroSchema::new(vec![
                crate::format::value::PyroField::new(
                    "input",
                    crate::format::value::PyroType::Group(std::borrow::Cow::Owned(
                        input_schema.fields.into_owned(),
                    )),
                    true,
                ),
                crate::format::value::PyroField::new(
                    "output",
                    crate::format::value::PyroType::Group(std::borrow::Cow::Owned(
                        output_schema.fields.into_owned(),
                    )),
                    true,
                ),
            ]);
            let data_wal = crate::format::value::arrow::wal::WalWriter::open(data_path, wal_schema)
                .map_err(|io| {
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
                Ok(captured) => Err(PyroError::CodePanic(captured)),
                Err(msg) => Err(PyroError::local(crate::error::ErrorKind::Transport(
                    CapturedError::new(msg),
                ))),
            };
        }

        match self.call(session_id, input).await {
            Ok(record) => {
                let _ = self.close_session(session_id).await;
                Ok(record)
            }
            Err(e) => {
                let _ = self.close_session(session_id).await;
                match e.result {
                    Ok(captured) => Err(PyroError::CodePanic(captured)),
                    Err(msg) => Err(PyroError::local(crate::error::ErrorKind::Transport(
                        CapturedError::new(msg),
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
                    row_index: session_id,
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
                        row_index: session_id,
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
    ) -> Result<SessionDiffExecutionRecord, PyroFailure> {
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
                        row_index: session_id,
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
                    row_index: session_id,
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
                    module_logs: logs.module_logs.clone(),
                    capability_logs: logs.capability_logs.clone(),
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

        let mut steps = Vec::new();
        if let Some(active) = self.active_sessions.get(&session_id) {
            let mut wal_rows = Vec::with_capacity(active.data_wal.prebatch.len());
            for i in 0..active.data_wal.prebatch.len() {
                if let Some(row) = active.data_wal.prebatch.get(i) {
                    wal_rows.push(row.clone());
                }
            }
            for row in wal_rows {
                let in_val = row
                    .get("input")
                    .cloned()
                    .unwrap_or(crate::format::value::PyroValue::Null);
                let out_val = row
                    .get("output")
                    .cloned()
                    .unwrap_or(crate::format::value::PyroValue::Null);

                let in_row = match in_val {
                    crate::format::value::PyroValue::Group(g) => g,
                    _ => PyroRow::empty(),
                };
                let out_row = match out_val {
                    crate::format::value::PyroValue::Group(g) => g,
                    _ => PyroRow::empty(),
                };
                steps.push((in_row, out_row));
            }
        }

        let is_failed = res.is_err();
        let log_failure = match &res {
            Err(e) => Some(e.result.clone()),
            _ => None,
        };

        let record = Self::into_record(session_id, steps, is_failed, logs.clone(), log_failure);

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

        match res {
            Ok(_) => Ok(record),
            Err(e) => Err(e),
        }
    }

    fn unpack_session_diff(row: PyroRow<'static>) -> Vec<(PyroRow<'static>, PyroRow<'static>)> {
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        for item in row.0 {
            if item.key == "inputs" {
                if let crate::format::value::PyroValue::List(list_vals) = item.value {
                    for val in list_vals {
                        if let crate::format::value::PyroValue::Group(r) = val {
                            inputs.push(r);
                        }
                    }
                }
            } else if item.key == "outputs" {
                if let crate::format::value::PyroValue::List(list_vals) = item.value {
                    for val in list_vals {
                        if let crate::format::value::PyroValue::Group(r) = val {
                            outputs.push(r);
                        }
                    }
                }
            }
        }

        let max_len = inputs.len().max(outputs.len());
        let mut steps = Vec::with_capacity(max_len);

        let mut inputs_iter = inputs.into_iter();
        let mut outputs_iter = outputs.into_iter();
        for _ in 0..max_len {
            let input = inputs_iter.next().unwrap_or_else(PyroRow::empty);
            let output = outputs_iter.next().unwrap_or_else(PyroRow::empty);
            steps.push((input, output));
        }
        steps
    }

    fn into_record(
        session_id: u32,
        steps: Vec<(PyroRow<'static>, PyroRow<'static>)>,
        is_failed: bool,
        logs: PyroLogs,
        log_failure: Option<Result<CapturedError, String>>,
    ) -> SessionDiffExecutionRecord {
        let (prior_input, prior_output, input, success) = if steps.is_empty() {
            (Vec::new(), Vec::new(), PyroRow::empty(), PyroRow::empty())
        } else {
            let len = steps.len();
            let mut prior_in = Vec::with_capacity(len - 1);
            let mut prior_out = Vec::with_capacity(len - 1);

            let mut steps = steps;
            for _ in 0..len - 1 {
                let (i, o) = steps.remove(0);
                prior_in.push(i);
                prior_out.push(o);
            }
            let (input, success) = steps.pop().unwrap();
            (prior_in, prior_out, input, success)
        };

        if is_failed {
            let failure = log_failure.unwrap_or_else(|| Err("Session failed".to_string()));

            SessionDiffExecutionRecord::Failure {
                row_index: session_id as usize,
                prior_input,
                prior_output,
                input,
                failure,
                logs,
            }
        } else {
            SessionDiffExecutionRecord::Success {
                row_index: session_id as usize,
                prior_input,
                prior_output,
                input,
                success,
                logs,
            }
        }
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
            .ok_or_else(|| {
                PyroError::not_found(format!("Session {} status not found", session_id))
            })?;

        // 2. Retrieve all steps (input, output) for the closed session
        let rolled_up_row = self.output_manager.get_record(session_id as usize)?;
        let steps = Self::unpack_session_diff(rolled_up_row);

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
        Ok(Self::into_record(
            session_id,
            steps,
            is_failed,
            logs,
            log_failure,
        ))
    }

    pub async fn close_session(&mut self, session_id: u32) -> Result<(), PyroFailure> {
        if self.active_sessions.contains_key(&session_id) {
            if let Err(e) = self.rollup_and_cleanup_session(session_id).await {
                tracing::error!(
                    "Failed to rollup and cleanup active session {} on close: {:?}",
                    session_id,
                    e
                );
            }
        }
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

    pub async fn get_session(
        &self,
        session_id: u32,
    ) -> Result<SessionDiffExecutionRecord, PyroError> {
        if let Some(active) = self.active_sessions.get(&session_id) {
            let mut wal_rows = Vec::with_capacity(active.data_wal.prebatch.len());
            for i in 0..active.data_wal.prebatch.len() {
                if let Some(row) = active.data_wal.prebatch.get(i) {
                    wal_rows.push(row.clone());
                }
            }
            let mut steps = Vec::with_capacity(wal_rows.len());
            for row in wal_rows {
                let in_val = row
                    .get("input")
                    .cloned()
                    .unwrap_or(crate::format::value::PyroValue::Null);
                let out_val = row
                    .get("output")
                    .cloned()
                    .unwrap_or(crate::format::value::PyroValue::Null);

                let in_row = match in_val {
                    crate::format::value::PyroValue::Group(g) => g,
                    _ => PyroRow::empty(),
                };
                let out_row = match out_val {
                    crate::format::value::PyroValue::Group(g) => g,
                    _ => PyroRow::empty(),
                };
                steps.push((in_row, out_row));
            }

            let mut logs = PyroLogs::empty();
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
                }
            }
            Ok(Self::into_record(session_id, steps, false, logs, None))
        } else {
            Err(PyroError::not_found(format!(
                "Session {} not found",
                session_id
            )))
        }
    }

    pub async fn get(&self, session_id: u32) -> Result<SessionDiffExecutionRecord, PyroError> {
        if self.active_sessions.contains_key(&session_id) {
            self.get_session(session_id).await
        } else {
            self.get_record(session_id).await
        }
    }

    pub fn active_sessions(&self) -> Vec<u32> {
        let mut keys: Vec<u32> = self.active_sessions.keys().copied().collect();
        keys.sort();
        keys
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
