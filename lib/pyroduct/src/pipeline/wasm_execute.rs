use std::cmp::min;
use std::sync::Arc;

use arrow::array::RecordBatch;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, mpsc};
use tracing::{error, instrument, warn};

use crate::{format::{
    PyroFailure, PyroLogs, PyroSuccess,
    value::{
        PyroRow, ValueError,
        arrow::{PreBatch, Rowable},
    },
    wal::{WalRecord, WalWriter},
}, pipeline::{PipelineError, PipelineResult}};
use crate::module::{PyroInstance, sessions::SessionResult};

use super::data::DataManager;

// =============================================================================
// Pipeline
// =============================================================================

#[derive(Debug, Clone)]
pub struct PipelineExecution {
    pub row_index: usize,
    pub success: Option<PyroSuccess>,
    pub failure: Option<PyroFailure>,
}

pub struct Pipeline {
    pub step: PyroInstance,
    pub success_log_retention_secs: u64,
    pub error_log_retention_secs: u64,
    pub output_dir: std::path::PathBuf,
    pub data_manager: DataManager,
}

impl Pipeline {
    /// Run the input through the single step.
    #[instrument(skip(self, input))]
    pub async fn process(&mut self, input: &PyroRow<'_>) -> PipelineExecution {
        let mut result: PyroRow<'static> = input.clone().into_owned();
        let mut execution = PipelineExecution {
            success: None,
            failure: None,
            row_index: 0,
        };
        execution.success = Some(PyroSuccess {
            row: result.clone(),
            logs: PyroLogs::empty(),
        });

        match self.step.call(&result).await {
            Ok(output) => {
                result.extend(output.row.clone());
                execution.steps.push(output);
            }
            Err(failure) => {
                match &failure.result {
                    Ok(error) => warn!("Pipeline Step: Module returned error: {}", error),
                    Err(error) => error!("Pipeline Step: Pyroduct Failed: {}", error),
                }
                execution.failure = Some(failure);
            }
        }

        let record = if let Some(failure) = &execution.failure {
            WalRecord::Failure {
                row_index: execution.row_index,
                failure: failure.clone(),
            }
        } else {
            WalRecord::Success {
                row_index: execution.row_index,
                success: execution.steps.last().unwrap().clone(),
            }
        };

        self.data_manager.push_record(&record)?;

        execution
    }

    pub async fn prep_session(
        &mut self,
        session_id: u32,
        inputs: &[PyroRow<'_>],
        outputs: &[PyroRow<'_>],
    ) -> Result<(), PyroFailure> {
        self.step.prep_session(session_id, inputs, outputs).await
    }

    pub async fn call_session(
        &mut self,
        session_id: u32,
        input: &PyroRow<'_>,
    ) -> Result<SessionResult, PyroFailure> {
        self.step.call_session(session_id, input).await
    }

    pub async fn close_session(&mut self, session_id: u32) -> Result<(), PyroFailure> {
        self.step.close_session(session_id).await
    }

    pub async fn session_inputs(&mut self, session_id: u32) -> Result<Vec<PyroRow<'_>>, PyroFailure> {
        self.step.session_inputs(session_id).await
    }

    pub async fn session_outputs(&mut self, session_id: u32) -> Result<Vec<PyroRow<'_>>, PyroFailure> {
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

pub struct PipelinePool {
    pipelines: Arc<Mutex<Vec<Pipeline>>>,
}

impl PipelinePool {
    pub fn new(pipelines: Vec<Pipeline>) -> Self {
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
    ) -> PipelineResult<(Vec<PipelineExecution>, Vec<PipelineExecution>)> {
        let total_rows = batch.num_rows();
        if total_rows == 0 {
            return Ok((Vec::new(), Vec::new()));
        }

        let pipelines = {
            let mut guard = self.pipelines.lock().await;
            if guard.is_empty() {
                return Err(PipelineError::Config(
                    "Pipeline pool is empty or exhausted".to_string(),
                ));
            }
            std::mem::take(&mut *guard)
        };

        let num_pipelines = pipelines.len();
        let (tx, mut rx) = mpsc::channel(100);
        let chunk_size = min((total_rows + num_pipelines - 1) / num_pipelines, 1000);
        let mut handles = Vec::with_capacity(num_pipelines);

        for (i, mut pipeline) in pipelines.into_iter().enumerate() {
            let offset = i * chunk_size;
            if offset >= total_rows {
                handles.push(tokio::spawn(async move { pipeline }));
                continue;
            }

            let length = chunk_size.min(total_rows - offset);
            let batch_slice = batch.slice(offset, length);
            let tx_clone = tx.clone();

            handles.push(tokio::spawn(async move {
                for j in 0..batch_slice.num_rows() {
                    let absolute_index = offset + j;

                    let mut result = match batch_slice.row(j) {
                        Ok(input_row) => pipeline.process(&input_row).await,
                        Err(e) => PipelineExecution {
                            row_index: absolute_index,
                            failure: Some(PyroFailure {
                                result: Ok(crate::CapturedError::new(e)),
                                logs: PyroLogs::empty(),
                            }),
                            steps: Vec::new(),
                        },
                    };
                    result.row_index = absolute_index;

                    if tx_clone.send(result).await.is_err() {
                        break;
                    }
                }

                pipeline
            }));
        }

        drop(tx);
        let mut success_results = Vec::with_capacity(total_rows);
        let mut failures: Vec<PipelineExecution> = Vec::new();

        while let Some(row_result) = rx.recv().await {
            match &row_result.failure {
                None => success_results.push(row_result),
                Some(_) => failures.push(row_result),
            }
        }

        // Reclaim pipelines back into the pool
        let mut reclaimed = Vec::with_capacity(num_pipelines);
        for handle in handles {
            match handle.await {
                Ok(p) => reclaimed.push(p),
                Err(e) => {
                    error!("Worker task panicked, pipeline lost: {}", e);
                }
            }
        }

        {
            let mut guard = self.pipelines.lock().await;
            *guard = reclaimed;
        }

        success_results.sort_by_key(|row| row.row_index);

        Ok((success_results, failures))
    }
}
