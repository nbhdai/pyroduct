use std::collections::VecDeque;
use std::sync::Arc;

use arrow::array::RecordBatch;
use pyro_artifacts::artifacts::PlaybookSpec;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{debug, error, instrument, warn};

use crate::CapturedError;
use crate::format::log_wal::{LogEntry, LogWal};
use crate::module::PyroInstance;
use crate::{
    format::{PyroFailure, PyroLogs, SessionResult, value::PyroRow},
    pipeline::{PipelineResult, PyroError},
};

use super::data::DataManager;
use super::session::{SessionStatusFilter, SessionStatusManager};

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
    pub shards: Vec<Mutex<PyroInstance>>,
    pub spec: Arc<PlaybookSpec>,
    pub success_log_retention_secs: u64,
    pub error_log_retention_secs: u64,
    pub log_manager: Mutex<LogWal>,
    pub output_manager: DataManager,
    pub log_dir: std::path::PathBuf,
    pub output_dir: std::path::PathBuf,
    pub wal_capacity: usize,
    pub active_sessions: Mutex<std::collections::HashMap<u32, ActiveSession>>,
    pub callbacks: Mutex<Vec<(uuid::Uuid, crate::pipeline::Callback)>>,
    pub session_status_manager: SessionStatusManager,
    pub max_active_sessions: usize,
    pub lru_order: Mutex<VecDeque<u32>>,
}

impl SessionDiffPipeline {
    /// Returns the shard mutex for a given session ID.
    fn shard(&self, session_id: u32) -> &Mutex<PyroInstance> {
        &self.shards[session_id as usize % self.shards.len()]
    }

    pub fn set_session_status(&self, session_id: usize, status: &str) -> Result<(), PyroError> {
        self.session_status_manager.set_status(session_id, status)
    }

    pub fn get_session_status(&self, session_id: usize) -> Result<Option<String>, PyroError> {
        self.session_status_manager.get_status(session_id)
    }

    pub fn list_sessions(
        &self,
        filter: Option<SessionStatusFilter>,
    ) -> Result<Vec<(u32, String)>, PyroError> {
        self.session_status_manager.list_sessions(filter)
    }

    pub fn max_session_id(&self) -> Result<Option<usize>, PyroError> {
        self.session_status_manager.max_session_id()
    }

    pub async fn next_session_id(&self) -> u32 {
        let mut max_id = self.output_manager.len().await as u32;

        // 1. Check in-memory active sessions
        {
            let active = self.active_sessions.lock().await;
            for &id in active.keys() {
                if id >= max_id {
                    max_id = id + 1;
                }
            }
        }

        // 2. Check the SQLite database
        if let Ok(Some(db_max)) = self.max_session_id() {
            let db_max = db_max as u32;
            if db_max >= max_id {
                max_id = db_max + 1;
            }
        }

        // 3. Scan output directory for any existing session files
        if let Ok(entries) = std::fs::read_dir(&self.output_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with("session_val_") {
                    let id_str = name_str
                        .strip_prefix("session_val_")
                        .and_then(|s| s.split('.').next());
                    if let Some(id_str) = id_str {
                        if let Ok(id) = id_str.parse::<u32>() {
                            if id >= max_id {
                                max_id = id + 1;
                            }
                        }
                    }
                }
            }
        }

        // 4. Scan log directory for any existing session directories
        if let Ok(entries) = std::fs::read_dir(&self.log_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with("session_log_") {
                    let id_str = name_str
                        .strip_prefix("session_log_")
                        .and_then(|s| s.split('.').next());
                    if let Some(id_str) = id_str {
                        if let Ok(id) = id_str.parse::<u32>() {
                            if id >= max_id {
                                max_id = id + 1;
                            }
                        }
                    }
                }
            }
        }

        debug!(next_id = max_id, "Determined next session ID");
        max_id
    }

    /// Evict the oldest active session(s) to stay within `max_active_sessions`.
    ///
    /// Eviction drops file handles (LogWal + WalWriter) and calls `free_session`
    /// on the owning shard to release WASM linear memory. The session WAL files
    /// remain on disk and will be re-opened by `get_or_open_session` on next access.
    async fn evict_if_needed(&self) {
        let mut active = self.active_sessions.lock().await;
        let mut lru = self.lru_order.lock().await;

        while active.len() >= self.max_active_sessions {
            if let Some(victim_id) = lru.pop_front() {
                if active.remove(&victim_id).is_some() {
                    debug!(session_id = victim_id, "Evicting session from active cache");
                    // Free WASM memory in the owning shard
                    let mut shard = self.shard(victim_id).lock().await;
                    if let Err(e) = shard.close_session(victim_id).await {
                        warn!(session_id = victim_id, ?e, "Failed to free evicted session in shard");
                    }
                } else {
                    // Stale LRU entry (already removed by rollup), skip it
                    continue;
                }
            } else {
                break;
            }
        }
    }

    #[instrument(skip(self), fields(session_id = session_id))]
    async fn get_or_open_session(&self, session_id: u32) -> Result<(), PyroError> {
        // Fast path: already active — just touch LRU
        {
            let active = self.active_sessions.lock().await;
            if active.contains_key(&session_id) {
                let mut lru = self.lru_order.lock().await;
                if let Some(pos) = lru.iter().position(|&id| id == session_id) {
                    lru.remove(pos);
                }
                lru.push_back(session_id);
                return Ok(());
            }
        }

        debug!("Opening session");
        if let Ok(Some(status)) = self.get_session_status(session_id as usize) {
            if status == "succeeded" || status == "failed" {
                warn!(status, "Cannot resume closed session");
                return Err(PyroError::validation(crate::capture!(
                    "Cannot resume closed session {}",
                    session_id
                )));
            }
        }

        // Evict oldest sessions if at capacity before opening new file handles
        self.evict_if_needed().await;

        let log_dir = self.log_dir.join(format!("session_log_{}", session_id));
        let data_path = self.output_dir.join(format!("session_val_{}", session_id));

        let log_wal = LogWal::open(log_dir, self.wal_capacity)
            .await
            .map_err(|io| {
                PyroError::local_io(
                    CapturedError::new("Unable to open individual log wal").with_source(io),
                )
            })?;
        let input_schema = self.spec.func.input.clone();
        let output_schema = self.spec.func.output.clone();
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
        let data_wal =
            crate::format::value::arrow::wal::WalWriter::open(data_path.clone(), wal_schema)
                .map_err(|io| {
                    PyroError::local_io(
                        CapturedError::new("Unable to open individual data wal")
                            .with_source(io),
                    )
                })?;

        debug!("Reactivating session, preloading history into step");
        let wal_rows =
            crate::format::value::arrow::wal::recover(&data_path).unwrap_or_default();
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        for row in wal_rows {
            if let Some(in_val) = row.get("input")
                && let crate::format::value::PyroValue::Group(g) = in_val
            {
                inputs.push(g.clone());
            }
            if let Some(out_val) = row.get("output")
                && let crate::format::value::PyroValue::Group(g) = out_val
            {
                outputs.push(g.clone());
            }
        }
        {
            let mut shard = self.shard(session_id).lock().await;
            if let Err(e) = shard.prep_session(session_id, &inputs, &outputs).await {
                warn!(?e, "Failed to prep reactivated session_diff");
            }
        }

        debug!("Successfully opened session files, inserting active session");
        let mut active = self.active_sessions.lock().await;
        active.insert(session_id, ActiveSession { log_wal, data_wal });

        let mut lru = self.lru_order.lock().await;
        lru.push_back(session_id);

        Ok(())
    }

    #[instrument(skip(self), fields(session_id = session_id))]
    async fn rollup_and_cleanup_session(&self, session_id: u32) -> Result<(), PyroError> {
        debug!("Rolling up and cleaning up session");

        // Use in-memory prebatch data (which is guaranteed to have all rows
        // including the just-appended output) instead of recovering from disk
        // where the last write may not have been flushed yet.
        let wal_rows = {
            let active_sessions = self.active_sessions.lock().await;
            if let Some(active) = active_sessions.get(&session_id) {
                let mut rows = Vec::with_capacity(active.data_wal.prebatch.len());
                for i in 0..active.data_wal.prebatch.len() {
                    if let Some(row) = active.data_wal.prebatch.get(i) {
                        rows.push(row.clone());
                    }
                }
                rows
            } else {
                // Fallback: session not in memory, recover from disk
                let data_path = self.output_dir.join(format!("session_val_{}", session_id));
                crate::format::value::arrow::wal::recover(&data_path).unwrap_or_default()
            }
        };
        debug!(wal_rows_count = wal_rows.len(), "Recovered data WAL rows");

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
            debug!("Reading individual log WAL to merge with general log manager");
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
                    .lock()
                    .await
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

        let _ = self.log_manager.lock().await.flush().await;

        {
            let mut active = self.active_sessions.lock().await;
            active.remove(&session_id);

            let mut lru = self.lru_order.lock().await;
            if let Some(pos) = lru.iter().position(|&id| id == session_id) {
                lru.remove(pos);
            }
        }

        let data_path = self.output_dir.join(format!("session_val_{}", session_id));
        let data_wal_file = data_path.with_extension("pyrowal");
        if data_wal_file.exists() {
            let _ = std::fs::remove_file(data_wal_file);
        }

        if log_dir.exists() {
            let _ = tokio::fs::remove_dir_all(log_dir).await;
        }

        debug!("Executing callbacks for session rollup");
        {
            let mut callbacks = self.callbacks.lock().await;
            for (_, cb) in callbacks.iter_mut() {
                cb.execute(session_id as usize, &rolled_up_row).await;
            }
        }

        debug!("Rollup and cleanup complete");
        Ok(())
    }

    #[instrument(skip(self, prior_inputs, prior_outputs, input), fields(row_index = row_index))]
    pub async fn process(
        &self,
        row_index: usize,
        prior_inputs: &[PyroRow<'_>],
        prior_outputs: &[PyroRow<'_>],
        input: &PyroRow<'_>,
    ) -> Result<SessionDiffExecutionRecord, PyroError> {
        let session_id = row_index as u32;
        debug!("Processing session diff");

        if let Err(e) = self
            .prep_session(session_id, prior_inputs, prior_outputs)
            .await
        {
            warn!(?e, "Failed to prepare session");
            let logs = e.logs.clone();
            let log_entry = LogEntry {
                row_index,
                module_logs: logs.module_logs.clone(),
                capability_logs: logs.capability_logs.clone(),
                failure: Some(e.result.clone()),
            };
            let _ = self.log_manager.lock().await.append(&log_entry).await;

            return match e.result {
                Ok(captured) => Err(PyroError::CodePanic(captured)),
                Err(msg) => Err(PyroError::local(crate::error::ErrorKind::Transport(
                    CapturedError::new(msg),
                ))),
            };
        }

        match self.call(session_id, input).await {
            Ok(record) => {
                debug!("Session diff call succeeded, closing session");
                let _ = self.close_session(session_id).await;
                Ok(record)
            }
            Err(e) => {
                warn!(?e, "Session diff call failed, closing session");
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

    #[instrument(skip(self, inputs, outputs), fields(session_id = session_id))]
    pub async fn prep_session(
        &self,
        session_id: u32,
        inputs: &[PyroRow<'_>],
        outputs: &[PyroRow<'_>],
    ) -> Result<(), PyroFailure> {
        debug!(
            inputs_len = inputs.len(),
            outputs_len = outputs.len(),
            "Preparing session diff"
        );
        {
            let mut shard = self.shard(session_id).lock().await;
            if let Err(e) = shard.prep_session(session_id, inputs, outputs).await {
                warn!(?e, "prep_session: Step returned error");
                let _ = self.set_session_status(session_id as usize, "failed");
                return Err(e);
            }
        }

        let data_path = self.output_dir.join(format!("session_val_{}", session_id));
        let existing = crate::format::value::arrow::wal::recover(&data_path).unwrap_or_default();
        debug!(existing_len = existing.len(), "Recovered existing data WAL");

        let active_logs = {
            let shard = self.shard(session_id).lock().await;
            shard.unpack_logs()
        };

        if let Err(e) = self.get_or_open_session(session_id).await {
            error!(?e, "Failed to get/open session during prep");
            let _ = self.set_session_status(session_id as usize, "failed");
            return Err(PyroFailure {
                row_index: session_id,
                result: Err(e.to_string()),
                logs: active_logs.clone(),
            });
        }

        let max_len = inputs.len().max(outputs.len());
        debug!(max_len, "Appending inputs/outputs to active session WAL");
        let mut active_sessions = self.active_sessions.lock().await;
        let active = active_sessions.get_mut(&session_id).unwrap();
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
                    error!(
                        index = i,
                        ?e,
                        "Failed to append input/output row to data WAL"
                    );
                    let _ = self.set_session_status(session_id as usize, "failed");
                    return Err(PyroFailure {
                        row_index: session_id,
                        result: Err(e.to_string()),
                        logs: active_logs.clone(),
                    });
                }
            }
        }

        debug!("Session diff prep complete");
        Ok(())
    }

    #[instrument(skip(self, input), fields(session_id = session_id))]
    pub async fn call(
        &self,
        session_id: u32,
        input: &PyroRow<'_>,
    ) -> Result<SessionDiffExecutionRecord, PyroFailure> {
        debug!("Calling step for session diff");
        let res = {
            let mut shard = self.shard(session_id).lock().await;
            shard.call_session(session_id, input).await
        };

        // PERSIST STATUS
        match &res {
            Ok(SessionResult::Continue { .. }) => {
                debug!("Step returned Continue, setting status to active");
                let _ = self.set_session_status(session_id as usize, "active");
            }
            Ok(SessionResult::End { .. }) | Ok(SessionResult::Terminate { .. }) => {
                debug!("Step returned End/Terminate, setting status to succeeded");
                let _ = self.set_session_status(session_id as usize, "succeeded");
            }
            Err(e) => {
                warn!(?e, "Step returned failure, setting status to failed");
                let _ = self.set_session_status(session_id as usize, "failed");
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
            if let Err(e) = self.get_or_open_session(session_id).await {
                error!(?e, "Failed to open session for append");
                let shard = self.shard(session_id).lock().await;
                let logs = shard.unpack_logs();
                return Err(PyroFailure {
                    row_index: session_id,
                    result: Err(e.to_string()),
                    logs,
                });
            }

            let mut active_sessions = self.active_sessions.lock().await;
            let active = active_sessions.get_mut(&session_id).unwrap();

            let record_index = active.data_wal.records_written() as usize;
            let step_row = PyroRow::from([
                (
                    "input",
                    crate::format::value::PyroValue::Group(input.clone().into_owned()),
                ),
                ("output", output_row),
            ]);
            debug!(
                record_index,
                "Appending step (input/output) row to data WAL"
            );
            let _ = active.data_wal.append(record_index, &step_row).await;
        }

        let logs = {
            let shard = self.shard(session_id).lock().await;
            shard.unpack_logs()
        };

        {
            if let Err(e) = self.get_or_open_session(session_id).await {
                error!(?e, "Failed to open session to append logs");
                let shard = self.shard(session_id).lock().await;
                let logs = shard.unpack_logs();
                return Err(PyroFailure {
                    row_index: session_id,
                    result: Err(e.to_string()),
                    logs,
                });
            }

            let mut active_sessions = self.active_sessions.lock().await;
            let active = active_sessions.get_mut(&session_id).unwrap();

            let row_index = active.log_wal.total_entries();
            match &res {
                Ok(_) => {
                    debug!(row_index, "Appending success to log WAL");
                    let log_entry = LogEntry {
                        row_index,
                        module_logs: logs.module_logs.clone(),
                        capability_logs: logs.capability_logs.clone(),
                        failure: None,
                    };
                    let _ = active.log_wal.append(&log_entry).await;
                }
                Err(e) => {
                    debug!(row_index, ?e, "Appending failure to log WAL");
                    let log_entry = LogEntry {
                        row_index,
                        module_logs: e.logs.module_logs.clone(),
                        capability_logs: e.logs.capability_logs.clone(),
                        failure: Some(e.result.clone()),
                    };
                    let _ = active.log_wal.append(&log_entry).await;
                }
            }
        }

        let steps = {
            let active_sessions = self.active_sessions.lock().await;
            let mut steps = Vec::new();
            if let Some(active) = active_sessions.get(&session_id) {
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
            steps
        };

        let is_failed = res.is_err();
        let log_failure = match &res {
            Err(e) => Some(e.result.clone()),
            _ => None,
        };

        let record = Self::into_record(session_id, steps, is_failed, logs.clone(), log_failure);

        match &res {
            Ok(SessionResult::End { .. }) | Ok(SessionResult::Terminate { .. }) => {
                if let Err(e) = self.rollup_and_cleanup_session(session_id).await {
                    error!(
                        "Failed to rollup and cleanup session {}: {:?}",
                        session_id, e
                    );
                }
            }
            Err(_) => {
                if let Err(e) = self.rollup_and_cleanup_session(session_id).await {
                    error!(
                        "Failed to rollup and cleanup failed session {}: {:?}",
                        session_id, e
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

    #[instrument(skip(self), fields(session_id = session_id))]
    pub async fn get_record(
        &self,
        session_id: u32,
    ) -> Result<SessionDiffExecutionRecord, PyroError> {
        debug!("get_record: starting lookup");

        // 1. Determine persistent status
        let status = self
            .get_session_status(session_id as usize)?
            .ok_or_else(|| {
                warn!("Session status not found");
                PyroError::not_found(format!("Session {} status not found", session_id))
            })?;
        debug!(status = ?status, "Found session status");

        // 2. Retrieve all steps (input, output) for the closed session
        let rolled_up_row = self.output_manager.get_record(session_id as usize).await?;
        let steps = Self::unpack_session_diff(rolled_up_row);
        debug!(steps_len = steps.len(), "Unpacked session steps");

        // 3. Retrieve logs
        let mut logs = PyroLogs::empty();
        let mut log_failure = None;
        let log_dir = self.log_dir.join(format!("session_log_{}", session_id));
        if log_dir.exists() {
            debug!("Reading logs from session-specific directory");
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
            debug!("Reading logs from general directory");
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
        debug!(is_failed, "Reconstructed execution record");
        Ok(Self::into_record(
            session_id,
            steps,
            is_failed,
            logs,
            log_failure,
        ))
    }

    #[instrument(skip(self), fields(session_id = session_id))]
    pub async fn close_session(&self, session_id: u32) -> Result<(), PyroFailure> {
        debug!("Closing session");
        let has_active = {
            let active = self.active_sessions.lock().await;
            active.contains_key(&session_id)
        };
        if has_active {
            debug!("Active session found, triggering rollup and cleanup");
            if let Err(e) = self.rollup_and_cleanup_session(session_id).await {
                error!(
                    "Failed to rollup and cleanup active session {} on close: {:?}",
                    session_id, e
                );
            }
        }
        let mut shard = self.shard(session_id).lock().await;
        shard.close_session(session_id).await
    }

    pub async fn session_inputs(
        &self,
        session_id: u32,
    ) -> Result<Vec<PyroRow<'static>>, PyroFailure> {
        let mut shard = self.shard(session_id).lock().await;
        shard
            .session_inputs(session_id)
            .await
            .map(|r| r.into_iter().map(|r| r.to_static()).collect())
    }

    pub async fn session_outputs(
        &self,
        session_id: u32,
    ) -> Result<Vec<PyroRow<'static>>, PyroFailure> {
        let mut shard = self.shard(session_id).lock().await;
        shard
            .session_outputs(session_id)
            .await
            .map(|r| r.into_iter().map(|r| r.to_static()).collect())
    }

    pub fn session_lengths(&self, session_id: u32) -> Option<(u32, u32)> {
        let shard = self.shard(session_id).try_lock().ok()?;
        shard.session_lengths(session_id)
    }

    #[instrument(skip(self), fields(session_id = session_id))]
    pub async fn get_session(
        &self,
        session_id: u32,
    ) -> Result<SessionDiffExecutionRecord, PyroError> {
        debug!("Getting session");
        let active_sessions = self.active_sessions.lock().await;
        if let Some(active) = active_sessions.get(&session_id) {
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
            debug!(steps_len = steps.len(), "Unpacked active session steps");
            drop(active_sessions);

            let mut logs = PyroLogs::empty();
            let log_dir = self.log_dir.join(format!("session_log_{}", session_id));
            if log_dir.exists() {
                debug!("Reading active session logs");
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
            drop(active_sessions);
            warn!("Active session not found");
            Err(PyroError::not_found(format!(
                "Session {} not found",
                session_id
            )))
        }
    }

    #[instrument(skip(self), fields(session_id = session_id))]
    pub async fn get(&self, session_id: u32) -> Result<SessionDiffExecutionRecord, PyroError> {
        debug!("Retrieving session (active or closed)");
        let is_active_in_db = matches!(
            self.get_session_status(session_id as usize),
            Ok(Some(ref status)) if status == "active"
        );

        let has_active = {
            let active = self.active_sessions.lock().await;
            active.contains_key(&session_id)
        };

        if has_active {
            debug!("Session is active in memory");
            self.get_session(session_id).await
        } else if is_active_in_db {
            debug!("Session is active but not loaded in memory, recovering from WAL");
            let data_path = self.output_dir.join(format!("session_val_{}", session_id));
            let wal_rows =
                crate::format::value::arrow::wal::recover(&data_path).unwrap_or_default();

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
            let mut log_failure = None;
            let log_dir = self.log_dir.join(format!("session_log_{}", session_id));
            if log_dir.exists() {
                debug!("Reading active session logs");
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
            }
            Ok(Self::into_record(
                session_id,
                steps,
                false,
                logs,
                log_failure,
            ))
        } else {
            debug!("Session is closed, looking up record");
            self.get_record(session_id).await
        }
    }

    pub async fn active_sessions(&self) -> Vec<u32> {
        let active = self.active_sessions.lock().await;
        let mut keys: Vec<u32> = active.keys().copied().collect();
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
// SessionDiffPipelinePool (deprecated — sharding is now built into SessionDiffPipeline)
// =============================================================================

#[deprecated(
    note = "Sharding is now built into SessionDiffPipeline via the `shards` field. Use SessionDiffPipeline directly."
)]
pub struct SessionDiffPipelinePool {
    _pipelines: Arc<Mutex<Vec<SessionDiffPipeline>>>,
}

#[allow(deprecated)]
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
