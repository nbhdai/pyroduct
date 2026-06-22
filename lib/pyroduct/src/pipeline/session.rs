use std::sync::Arc;

use arrow::array::RecordBatch;
use pyro_artifacts::artifacts::PlaybookSpec;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{debug, error, info, instrument, warn};

use crate::CapturedError;
use crate::format::log_wal::{LogEntry, LogWal};
use crate::module::PyroInstance;
use crate::{
    format::{PyroFailure, PyroLogs, SessionResult, value::PyroRow},
    pipeline::{PipelineResult, PyroError},
};

use super::data::DataManager;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatusFilter {
    Active,
    Closed,
    Failed,
}

// =============================================================================
// Pipeline
// =============================================================================

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SessionExecutionRecord {
    Success {
        row_index: usize,
        prior: Vec<PyroRow<'static>>,
        input: PyroRow<'static>,
        success: PyroRow<'static>,
        logs: PyroLogs,
    },
    Failure {
        row_index: usize,
        prior: Vec<PyroRow<'static>>,
        input: PyroRow<'static>,
        failure: Result<CapturedError, String>,
        logs: PyroLogs,
    },
}

impl SessionExecutionRecord {
    pub fn row_index(&self) -> usize {
        match self {
            SessionExecutionRecord::Success { row_index, .. } => *row_index,
            SessionExecutionRecord::Failure { row_index, .. } => *row_index,
        }
    }

    pub fn row(&self) -> Option<&PyroRow<'static>> {
        match self {
            SessionExecutionRecord::Success { success, .. } => Some(success),
            SessionExecutionRecord::Failure { input, .. } => Some(input),
        }
    }
}

pub struct ActiveSession {
    pub log_wal: LogWal,
    pub data_wal: crate::format::value::arrow::wal::WalWriter,
}

pub struct SessionStatusManager {
    pub sqlite_conn: std::sync::Mutex<rusqlite::Connection>,
}

impl SessionStatusManager {
    pub fn new(output_dir: &std::path::Path) -> Result<Self, PyroError> {
        let db_path = output_dir.join("session.db");
        let conn = rusqlite::Connection::open(db_path).map_err(|e| {
            PyroError::validation(
                CapturedError::new("Failed to open SQLite database for sessions").with_source(e),
            )
        })?;
        let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS session_status (
                session_id INTEGER PRIMARY KEY,
                status TEXT NOT NULL
            )",
            [],
        );
        Ok(Self {
            sqlite_conn: std::sync::Mutex::new(conn),
        })
    }

    pub fn set_status(&self, session_id: usize, status: &str) -> Result<(), PyroError> {
        self.sqlite_conn
            .lock()
            .unwrap()
            .execute(
                "INSERT OR REPLACE INTO session_status (session_id, status) VALUES (?1, ?2)",
                rusqlite::params![session_id as i64, status],
            )
            .map_err(|e| {
                PyroError::validation(
                    CapturedError::new("Failed to set session status").with_source(e),
                )
            })?;
        Ok(())
    }

    pub fn get_status(&self, session_id: usize) -> Result<Option<String>, PyroError> {
        let conn = self.sqlite_conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT status FROM session_status WHERE session_id = ?")
            .map_err(|e| {
                PyroError::validation(
                    CapturedError::new("Failed to prepare SELECT statement for session_status")
                        .with_source(e),
                )
            })?;
        let status_opt = stmt.query_row([session_id as i64], |r| r.get::<_, String>(0));
        match status_opt {
            Ok(status) => Ok(Some(status)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(PyroError::validation(
                CapturedError::new("Failed to query session status").with_source(e),
            )),
        }
    }

    pub fn max_session_id(&self) -> Result<Option<usize>, PyroError> {
        let conn = self.sqlite_conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT MAX(session_id) FROM session_status")
            .map_err(|e| {
                PyroError::validation(
                    CapturedError::new("Failed to prepare SELECT statement for max session_id")
                        .with_source(e),
                )
            })?;
        let res = stmt.query_row([], |r| r.get::<_, Option<i64>>(0));
        match res {
            Ok(Some(max_id)) => Ok(Some(max_id as usize)),
            Ok(None) => Ok(None),
            Err(e) => Err(PyroError::validation(
                CapturedError::new("Failed to query max session_id").with_source(e),
            )),
        }
    }

    pub fn list_sessions(
        &self,
        filter: Option<SessionStatusFilter>,
    ) -> Result<Vec<(u32, String)>, PyroError> {
        let conn = self.sqlite_conn.lock().unwrap();
        let sql = match filter {
            Some(SessionStatusFilter::Active) => {
                "SELECT session_id, status FROM session_status WHERE status = 'active' ORDER BY session_id DESC"
            }
            Some(SessionStatusFilter::Closed) => {
                "SELECT session_id, status FROM session_status WHERE status = 'succeeded' ORDER BY session_id DESC"
            }
            Some(SessionStatusFilter::Failed) => {
                "SELECT session_id, status FROM session_status WHERE status = 'failed' ORDER BY session_id DESC"
            }
            None => "SELECT session_id, status FROM session_status ORDER BY session_id DESC",
        };

        let mut stmt = conn.prepare(sql).map_err(|e| {
            PyroError::validation(
                CapturedError::new("Failed to prepare SELECT statement for sessions list")
                    .with_source(e),
            )
        })?;
        let rows = stmt
            .query_map([], |row| {
                let id: i64 = row.get(0)?;
                let status: String = row.get(1)?;
                Ok((id as u32, status))
            })
            .map_err(|e| {
                PyroError::validation(
                    CapturedError::new("Failed to query sessions list").with_source(e),
                )
            })?;

        let mut res = Vec::new();
        for r in rows {
            if let Ok(info) = r {
                res.push(info);
            }
        }
        Ok(res)
    }
}

pub struct SessionPipeline {
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
}

impl SessionPipeline {
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

    #[instrument(skip(self), fields(session_id = session_id))]
    async fn get_or_open_session(&self, session_id: u32) -> Result<(), PyroError> {
        let already_active = {
            let active = self.active_sessions.lock().await;
            active.contains_key(&session_id)
        };

        if !already_active {
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

            let log_dir = self.log_dir.join(format!("session_log_{}", session_id));
            let data_path = self.output_dir.join(format!("session_val_{}", session_id));

            let log_wal = LogWal::open(log_dir, self.wal_capacity)
                .await
                .map_err(|io| {
                    PyroError::local_io(
                        CapturedError::new("Unable to open individual log wal").with_source(io),
                    )
                })?;
            let data_wal = crate::format::value::arrow::wal::WalWriter::open(
                data_path.clone(),
                self.spec.func.input.clone(),
            )
            .map_err(|io| {
                PyroError::local_io(
                    CapturedError::new("Unable to open individual data wal").with_source(io),
                )
            })?;

            debug!("Reactivating session, preloading history into step");
            let existing =
                crate::format::value::arrow::wal::recover(&data_path).unwrap_or_default();
            {
                let mut shard = self.shard(session_id).lock().await;
                if let Err(e) = shard.prep_session(session_id, &existing, &[]).await {
                    warn!(?e, "Failed to prep reactivated session");
                }
            }

            debug!("Successfully opened session files, inserting active session");
            let mut active = self.active_sessions.lock().await;
            active.insert(session_id, ActiveSession { log_wal, data_wal });
        }
        Ok(())
    }

    #[instrument(skip(self), fields(session_id = session_id))]
    async fn rollup_and_cleanup_session(&self, session_id: u32) -> Result<(), PyroError> {
        debug!("Rolling up and cleaning up session");

        // Use in-memory prebatch data (which is guaranteed to have all rows
        // including the just-appended output) instead of recovering from disk
        // where the last write may not have been flushed yet.
        let inputs = {
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
        debug!(inputs_count = inputs.len(), "Recovered data WAL inputs");

        let mut list_vals = Vec::with_capacity(inputs.len());
        for row in inputs {
            let val = if row.0.len() == 1 {
                // Single-field row (scalar session) — extract the inner value
                row.0.into_iter().next().unwrap().value
            } else {
                // Multi-field row (struct session) — the row IS the value
                crate::format::value::PyroValue::Group(row)
            };
            list_vals.push(val);
        }

        let rolled_up_row =
            PyroRow::from([("session", crate::format::value::PyroValue::List(list_vals))]);

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

    #[instrument(skip(self, prior, input), fields(row_index = row_index))]
    pub async fn process(
        &self,
        row_index: usize,
        prior: &[PyroRow<'_>],
        input: &PyroRow<'_>,
    ) -> Result<SessionExecutionRecord, PyroError> {
        let session_id = row_index as u32;
        debug!("Processing session");

        if let Err(e) = self.prep_session(session_id, prior).await {
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
                debug!("Session call succeeded");
                Ok(record)
            }
            Err(e) => {
                warn!(?e, "Session call failed");
                match e.result {
                    Ok(captured) => Err(PyroError::CodePanic(captured)),
                    Err(msg) => Err(PyroError::local(crate::error::ErrorKind::Transport(
                        CapturedError::new(msg),
                    ))),
                }
            }
        }
    }

    #[instrument(skip(self, prior), fields(session_id = session_id))]
    pub async fn prep_session(
        &self,
        session_id: u32,
        prior: &[PyroRow<'_>],
    ) -> Result<(), PyroFailure> {
        debug!(prior_len = prior.len(), "Preparing session");
        {
            let mut shard = self.shard(session_id).lock().await;
            if let Err(e) = shard.prep_session(session_id, prior, &[]).await {
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

        debug!("Appending prior rows to active session WAL");
        let mut active_sessions = self.active_sessions.lock().await;
        let active = active_sessions.get_mut(&session_id).unwrap();
        for (i, row) in prior.iter().enumerate() {
            if i >= existing.len() {
                let unwrapped = Self::unwrap_for_wal(row);
                if let Err(e) = active.data_wal.append(i, &unwrapped).await {
                    error!(index = i, ?e, "Failed to append prior row to data WAL");
                    let _ = self.set_session_status(session_id as usize, "failed");
                    return Err(PyroFailure {
                        row_index: session_id,
                        result: Err(e.to_string()),
                        logs: active_logs.clone(),
                    });
                }
            }
        }

        debug!("Session prep complete");
        Ok(())
    }

    /// Unwrap a row for WAL storage: if the row has a single field whose value
    /// is a Group, return that inner Group row. Otherwise return the row as-is.
    /// This strips field name wrappers like {input: Group({role, content})} → {role, content}.
    fn unwrap_for_wal<'a>(row: &PyroRow<'a>) -> PyroRow<'a> {
        if row.0.len() == 1 {
            if let crate::format::value::PyroValue::Group(ref inner) = row.0[0].value {
                return inner.clone();
            }
        }
        row.clone()
    }

    #[instrument(skip(self, input), fields(session_id = session_id))]
    pub async fn call(
        &self,
        session_id: u32,
        input: &PyroRow<'_>,
    ) -> Result<SessionExecutionRecord, PyroFailure> {
        debug!("Calling step for session");
        {
            if let Err(e) = self.get_or_open_session(session_id).await {
                error!(?e, "Failed to open session for append");
                let _ = self.set_session_status(session_id as usize, "failed");
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
            debug!(record_index, "Appending input row to data WAL");
            let unwrapped_input = Self::unwrap_for_wal(input);
            if let Err(e) = active.data_wal.append(record_index, &unwrapped_input).await {
                error!(?e, "Failed to append input to data WAL");
                let _ = self.set_session_status(session_id as usize, "failed");
                let shard = self.shard(session_id).lock().await;
                let logs = shard.unpack_logs();
                return Err(PyroFailure {
                    row_index: session_id,
                    result: Err(e.to_string()),
                    logs,
                });
            }
        }

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

        {
            if let Err(e) = self.get_or_open_session(session_id).await {
                error!(?e, "Failed to open session to append output");
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

            if let Ok(SessionResult::Continue {
                result: output_row, ..
            })
            | Ok(SessionResult::End {
                result: output_row, ..
            }) = &res
            {
                let record_index = active.data_wal.records_written() as usize;
                debug!(record_index, "Appending output row to data WAL");
                let unwrapped_output = Self::unwrap_for_wal(output_row);
                let _ = active.data_wal.append(record_index, &unwrapped_output).await;
            }
        }

        let logs = {
            let shard = self.shard(session_id).lock().await;
            shard.unpack_logs()
        };

        {
            if let Err(e) = self.get_or_open_session(session_id).await {
                error!(?e, "Failed to open session to append logs");
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

        let wal_rows = {
            let active_sessions = self.active_sessions.lock().await;
            let mut rows = Vec::new();
            if let Some(active) = active_sessions.get(&session_id) {
                for i in 0..active.data_wal.prebatch.len() {
                    if let Some(row) = active.data_wal.prebatch.get(i) {
                        rows.push(row.clone());
                    }
                }
            }
            rows
        };

        let is_failed = res.is_err();
        let log_failure = match &res {
            Err(e) => Some(e.result.clone()),
            _ => None,
        };

        let record = Self::into_record(session_id, wal_rows, is_failed, logs.clone(), log_failure);
        info!(
            "SessionPipeline::call debug: session_id={}, success_row={:?}",
            session_id,
            record.row()
        );

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

    fn unpack_session(row: PyroRow<'static>) -> Vec<PyroRow<'static>> {
        let mut session_rows = Vec::new();
        for item in row.0 {
            if item.key == "session" {
                if let crate::format::value::PyroValue::List(list_vals) = item.value {
                    for val in list_vals {
                        match val {
                            crate::format::value::PyroValue::Group(r) => {
                                session_rows.push(r);
                            }
                            other => {
                                session_rows.push(PyroRow::from([("value", other)]));
                            }
                        }
                    }
                }
                break;
            }
        }
        session_rows
    }

    fn into_record(
        session_id: u32,
        wal_rows: Vec<PyroRow<'static>>,
        is_failed: bool,
        logs: PyroLogs,
        log_failure: Option<Result<CapturedError, String>>,
    ) -> SessionExecutionRecord {
        if is_failed {
            let (prior, input) = if wal_rows.is_empty() {
                (Vec::new(), PyroRow::empty())
            } else if wal_rows.len() % 2 == 1 {
                let mut wal_rows = wal_rows;
                let input = wal_rows.pop().unwrap();
                (wal_rows, input)
            } else {
                (wal_rows, PyroRow::empty())
            };

            let failure = log_failure.unwrap_or_else(|| Err("Session failed".to_string()));

            SessionExecutionRecord::Failure {
                row_index: session_id as usize,
                prior,
                input,
                failure,
                logs,
            }
        } else {
            let (prior, input, success) = if wal_rows.is_empty() {
                (Vec::new(), PyroRow::empty(), PyroRow::empty())
            } else if wal_rows.len() % 2 == 0 {
                let mut wal_rows = wal_rows;
                let success = wal_rows.pop().unwrap();
                let input = wal_rows.pop().unwrap();
                (wal_rows, input, success)
            } else {
                let mut wal_rows = wal_rows;
                let input = wal_rows.pop().unwrap();
                (wal_rows, input, PyroRow::empty())
            };

            SessionExecutionRecord::Success {
                row_index: session_id as usize,
                prior,
                input,
                success,
                logs,
            }
        }
    }

    #[instrument(skip(self), fields(session_id = session_id))]
    pub async fn get_record(&self, session_id: u32) -> Result<SessionExecutionRecord, PyroError> {
        debug!("get_record: starting lookup");

        // 1. Determine persistent status
        let status = self
            .get_session_status(session_id as usize)?
            .ok_or_else(|| {
                warn!("Session status not found");
                PyroError::not_found(format!("Session {} status not found", session_id))
            })?;
        debug!(status = ?status, "Found session status");

        // 2. Retrieve all rows for the closed session from output_manager
        let rolled_up_row = self.output_manager.get_record(session_id as usize).await?;
        let wal_rows = Self::unpack_session(rolled_up_row);

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

        // 4. Reconstruct prior, input, success, failure
        let is_failed = status == "failed";
        debug!(is_failed, "Reconstructed execution record");
        Ok(Self::into_record(
            session_id,
            wal_rows,
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

    pub async fn session(&self, session_id: u32) -> Result<Vec<PyroRow<'static>>, PyroFailure> {
        let mut shard = self.shard(session_id).lock().await;
        shard
            .session_inputs(session_id)
            .await
            .map(|r| r.into_iter().map(|r| r.to_static()).collect())
    }

    pub fn session_lengths(&self, session_id: u32) -> Option<u32> {
        let shard = self.shard(session_id).try_lock().ok()?;
        shard.session_lengths(session_id).map(|o| o.0 + o.1)
    }

    #[instrument(skip(self), fields(session_id = session_id))]
    pub async fn get_session(&self, session_id: u32) -> Result<SessionExecutionRecord, PyroError> {
        debug!("Getting session");
        let active_sessions = self.active_sessions.lock().await;
        if let Some(active) = active_sessions.get(&session_id) {
            let mut wal_rows = Vec::with_capacity(active.data_wal.prebatch.len());
            for i in 0..active.data_wal.prebatch.len() {
                if let Some(row) = active.data_wal.prebatch.get(i) {
                    wal_rows.push(row.clone());
                }
            }
            debug!(
                wal_rows_len = wal_rows.len(),
                "Unpacked active session WAL rows"
            );
            drop(active_sessions);

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
            let is_failed = if let Ok(Some(status)) = self.get_session_status(session_id as usize) {
                status == "failed"
            } else {
                false
            };
            Ok(Self::into_record(
                session_id,
                wal_rows,
                is_failed,
                logs,
                log_failure,
            ))
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
    pub async fn get(&self, session_id: u32) -> Result<SessionExecutionRecord, PyroError> {
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

            let mut logs = PyroLogs::empty();
            let mut log_failure = None;
            let log_dir = self.log_dir.join(format!("session_log_{}", session_id));
            if log_dir.exists() {
                debug!("Reading active session logs from disk");
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
                wal_rows,
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
// SessionPipelinePool (deprecated — sharding is now built into SessionPipeline)
// =============================================================================

#[deprecated(
    note = "Sharding is now built into SessionPipeline via the `shards` field. Use SessionPipeline directly."
)]
pub struct SessionPipelinePool {
    _pipelines: Arc<Mutex<Vec<SessionPipeline>>>,
}

#[allow(deprecated)]
impl SessionPipelinePool {
    pub fn new(pipelines: Vec<SessionPipeline>) -> Self {
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
    ) -> PipelineResult<(Vec<SessionExecutionRecord>, Vec<SessionExecutionRecord>)> {
        todo!()
    }
}
