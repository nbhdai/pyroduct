use std::cmp::min;
use std::sync::Arc;

use arrow::array::RecordBatch;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, mpsc};
use tracing::{error, instrument, warn};

use crate::CapturedError;
use crate::format::log_wal::LogWal;
use crate::module::{PyroInstance, sessions::SessionResult};
use crate::{
    PyroError,
    format::{
        PyroFailure, PyroLogs, PyroSuccess,
        value::{
            PyroRow,
            arrow::{PreBatch, Rowable},
        },
    },
    pipeline::{PipelineError, PipelineResult},
};

use super::data::DataManager;

// =============================================================================
// Pipeline
// =============================================================================

#[derive(Clone)]
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

pub struct SessionPipeline {
    pub step: PyroInstance,
    pub success_log_retention_secs: u64,
    pub error_log_retention_secs: u64,
    pub log_manager: LogWal,
    pub input_manager: DataManager,
    pub output_manager: DataManager,
}

impl SessionPipeline {
    pub async fn prep_session(
        &mut self,
        session_id: u32,
        prior: &[PyroRow<'_>],
    ) -> Result<(), PyroFailure> {
        self.step.prep_session(session_id, prior, &[]).await
    }

    pub async fn call(
        &mut self,
        session_id: u32,
        input: &PyroRow<'_>,
    ) -> Result<SessionResult, PyroFailure> {

        self.step.call_session(session_id, input).await
    }

    pub async fn close_session(&mut self, session_id: u32) -> Result<(), PyroFailure> {
        self.step.close_session(session_id).await
    }

    pub async fn session(
        &mut self,
        session_id: u32,
    ) -> Result<Vec<PyroRow<'_>>, PyroFailure> {
        self.step.session_inputs(session_id).await
    }

    pub fn session_lengths(&self, session_id: u32) -> Option<u32> {
        self.step.session_lengths(session_id).map(|o| o.0)
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

pub struct SessionPipelinePool {
    pipelines: Arc<Mutex<Vec<SessionPipeline>>>,
}

impl SessionPipelinePool {
    pub fn new(pipelines: Vec<SessionPipeline>) -> Self {
        Self {
            pipelines: Arc::new(Mutex::new(pipelines)),
        }
    }

    /// Distribute rows across available pipelines and collect results.
    ///
    /// Returns the successful rows (sorted by original index) and any
    /// per-row failures.
    pub async fn process_batch(
        &self,
        batch: &RecordBatch,
    ) -> PipelineResult<(Vec<SessionExecutionRecord>, Vec<SessionExecutionRecord>)> {
        todo!()
    }
}
