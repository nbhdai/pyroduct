use std::cmp::min;
use std::sync::Arc;

use arrow::array::RecordBatch;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, mpsc};
use tracing::{debug, error, info, instrument, warn};

use crate::format::{
    PyroFailure, PyroLogs, PyroSuccess,
    value::{
        PyroRow, ValueError,
        arrow::{PreBatch, Rowable},
    },
    wal::{WalRecord, WalWriter, recover},
};
use crate::module::PyroInstance;

use super::{PipelineError, PipelineResult};

// =============================================================================
// Pipeline
// =============================================================================

#[derive(Debug, Clone)]
pub struct PipelineExecution {
    pub row_index: usize,
    pub steps: Vec<PyroSuccess>,
    pub failure: Option<PyroFailure>,
}

impl PipelineExecution {
    pub fn row(&self) -> Option<PyroRow<'_>> {
        let mut steps = self.steps.iter();
        let mut row = steps.next().map(|r| r.row.clone())?;
        for step in steps {
            row.extend(step.row.clone());
        }
        Some(row)
    }

    /// Row from steps [0, step_index] merged together
    pub fn row_up_to(&self, step_index: usize) -> Option<PyroRow<'_>> {
        if self.steps.len() < step_index {
            return None;
        }
        let mut steps = self.steps[..=step_index.min(self.steps.len() - 1)].iter();
        let mut row = steps.next().map(|r| r.row.clone())?;
        for step in steps {
            row.extend(step.row.clone());
        }
        Some(row)
    }

    /// Row from only the given step index
    pub fn row_at(&self, step_index: usize) -> Option<PyroRow<'_>> {
        self.steps.get(step_index).map(|s| s.row.clone())
    }
}

pub fn extract_upto_batch(
    executions: &[PipelineExecution],
    step_index: usize,
) -> Result<Option<RecordBatch>, ValueError> {
    let batch = PreBatch::from_iter(executions.iter().filter_map(|s| s.row_up_to(step_index)));
    match batch {
        Some(mut b) => b.flush(),
        None => Ok(None),
    }
}

pub fn extract_at_batch(
    executions: &[PipelineExecution],
    step_index: usize,
) -> Result<Option<RecordBatch>, ValueError> {
    let batch = PreBatch::from_iter(executions.iter().filter_map(|s| s.row_at(step_index)));
    match batch {
        Some(mut b) => b.flush(),
        None => Ok(None),
    }
}

pub struct Pipeline {
    pub step: PyroInstance,
    pub wal_capacity: usize,
    pub success_log_retention_secs: u64,
    pub error_log_retention_secs: u64,
    pub output_dir: std::path::PathBuf,
    pub wal_writer: Option<WalWriter<std::fs::File, std::fs::File>>,
    pub current_wal_id: usize,
}

impl Pipeline {
    /// Run the input through the single step.
    #[instrument(skip(self, input))]
    pub async fn process(&mut self, input: &PyroRow<'_>) -> PipelineExecution {
        let mut result: PyroRow<'static> = input.clone().into_owned();
        let mut execution = PipelineExecution {
            steps: Vec::new(),
            failure: None,
            row_index: 0,
        };
        execution.steps.push(PyroSuccess {
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

        if let Some(wal) = &mut self.wal_writer {
            if let Err(e) = wal.append(&record) {
                error!("Failed to append to WAL: {}", e);
            }
            if wal.records_written() >= self.wal_capacity as u64 {
                self.flush_wal().await;
            }
        }

        execution
    }

    pub async fn flush_wal(&mut self) {
        if let Some(wal) = self.wal_writer.take() {
            let path = wal.wal_path().map(|p| p.to_path_buf());
            let base_path = self.output_dir.join(format!("wal_{}", self.current_wal_id));
            
            // Recover and build arrow batch
            if let Ok(records) = recover(&base_path) {
                let success_rows = records.into_iter().filter_map(|r| {
                    if let WalRecord::Success { success, .. } = r {
                        Some(success.row)
                    } else { None }
                });
                
                if let Some(mut prebatch) = PreBatch::from_iter(success_rows) {
                    if let Ok(Some(batch)) = prebatch.flush() {
                        let arrow_path = self.output_dir.join(format!("batch_{}.arrow", self.current_wal_id));
                        #[cfg(feature = "cli")]
                        {
                            if let Ok(bytes) = pyro_arrow_file::record_batch_to_bytes(&batch) {
                                let _ = std::fs::write(&arrow_path, bytes);
                            }
                        }
                        #[cfg(not(feature = "cli"))]
                        {
                            // If arrow_file feature is not enabled, we just warn or handle differently
                            // Assuming it's available or we can just ignore.
                        }
                    }
                }
            }

            // Prune old logs before advancing
            self.prune_logs().await;

            if let Some(p) = path {
                let _ = std::fs::remove_file(p);
            }

            self.current_wal_id += 1;
            let next_base = self.output_dir.join(format!("wal_{}", self.current_wal_id));
            self.wal_writer = WalWriter::open(next_base).ok();
        }
    }

    async fn prune_logs(&self) {
        let now = std::time::SystemTime::now();
        if let Ok(entries) = std::fs::read_dir(&self.output_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("pyrolog") {
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            if let Ok(elapsed) = now.duration_since(modified) {
                                // Simple heuristic: Since logs are mixed in .pyrolog,
                                // we'll use the error log retention as the upper bound for the file's lifetime.
                                if elapsed.as_secs() > self.error_log_retention_secs {
                                    let _ = std::fs::remove_file(path);
                                }
                            }
                        }
                    }
                }
            }
        }
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
