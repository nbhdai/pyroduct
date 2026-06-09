use std::cmp::min;
use std::sync::Arc;

use arrow::array::RecordBatch;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, mpsc};
use tracing::{debug, error, info, instrument, warn};

use crate::CapturedError;
use crate::format::log_wal::{LogEntry, LogWal};
use crate::module::PyroInstance;
use crate::{
    PyroError,
    format::{
        PyroLogs,
        value::{
            PyroRow,
            arrow::{PreBatch, Rowable},
        },
    },
    pipeline::{PipelineError, PipelineResult},
};

use super::data::DataManager;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ExecutionRecord {
    Success {
        row_index: usize,
        input: PyroRow<'static>,
        success: PyroRow<'static>,
        logs: PyroLogs,
    },
    Failure {
        row_index: usize,
        input: PyroRow<'static>,
        failure: Result<CapturedError, String>,
        logs: PyroLogs,
    },
}

impl ExecutionRecord {
    pub fn row_index(&self) -> usize {
        match self {
            ExecutionRecord::Success { row_index, .. } => *row_index,
            ExecutionRecord::Failure { row_index, .. } => *row_index,
        }
    }

    pub fn row(&self) -> Option<&PyroRow<'static>> {
        match self {
            ExecutionRecord::Success { success, .. } => Some(success),
            ExecutionRecord::Failure { input, .. } => Some(input),
        }
    }
}

// =============================================================================
// Pipeline
// =============================================================================

pub struct Pipeline {
    pub shards: Vec<Mutex<PyroInstance>>,
    pub success_log_retention_secs: u64,
    pub error_log_retention_secs: u64,
    pub log_manager: Mutex<LogWal>,
    pub input_manager: DataManager,
    pub output_manager: DataManager,
    pub callbacks: Mutex<Vec<(uuid::Uuid, crate::pipeline::Callback)>>,
}

impl Pipeline {
    /// Returns the shard mutex for a given row index.
    fn shard(&self, row_index: usize) -> &Mutex<PyroInstance> {
        &self.shards[row_index % self.shards.len()]
    }

    pub async fn call(&self, input: &PyroRow<'_>) -> Result<ExecutionRecord, PyroError> {
        let index = self.input_manager.len().await;
        self.process(index, input).await
    }

    /// Run the input through the single step.
    #[instrument(skip(self, input), fields(row_index = row_index))]
    pub async fn process(
        &self,
        row_index: usize,
        input: &PyroRow<'_>,
    ) -> Result<ExecutionRecord, PyroError> {
        debug!(row_index, "Processing row");
        self.input_manager.push_record(row_index, input).await?;

        let mut shard = self.shard(row_index).lock().await;
        match shard.call(row_index as u32, input).await {
            Ok(output) => {
                debug!(row_index, "Step succeeded");
                drop(shard);
                // Execute callbacks
                {
                    let mut cbs = self.callbacks.lock().await;
                    for (_, cb) in cbs.iter_mut() {
                        cb.execute(row_index, &output.row).await;
                    }
                }

                self.output_manager
                    .push_record(row_index, &output.row)
                    .await?;

                let log_entry = LogEntry {
                    row_index,
                    module_logs: output.logs.module_logs.clone(),
                    capability_logs: output.logs.capability_logs.clone(),
                    failure: None,
                };
                self.log_manager.lock().await.append(&log_entry).await?;
                let record = ExecutionRecord::Success {
                    row_index,
                    input: input.clone().into_owned(),
                    success: output.row,
                    logs: output.logs,
                };
                Ok(record)
            }
            Err(f) => {
                drop(shard);
                match &f.result {
                    Ok(error) => {
                        warn!(row_index, "Pipeline Step: Module returned error: {}", error)
                    }
                    Err(error) => error!(row_index, "Pipeline Step: Pyroduct Failed: {}", error),
                }
                let log_entry = LogEntry {
                    row_index,
                    module_logs: f.logs.module_logs.clone(),
                    capability_logs: f.logs.capability_logs.clone(),
                    failure: Some(f.result.clone()),
                };
                self.log_manager.lock().await.append(&log_entry).await?;
                let record = ExecutionRecord::Failure {
                    row_index,
                    input: input.clone().into_owned(),
                    failure: f.result,
                    logs: f.logs,
                };
                Ok(record)
            }
        }
    }

    /// Retrieve a single record by its global index.
    pub async fn get_record(&self, index: usize) -> Result<ExecutionRecord, PyroError> {
        debug!(index, "Retrieving record");
        let input_row = self.input_manager.get_record(index).await?;

        // Try to read from LogWal in O(1)
        let log_entry = self
            .log_manager
            .lock()
            .await
            .get(index)
            .await
            .map_err(|e| PyroError::local_io(CapturedError::new(e)))?;

        if let Some(entry) = log_entry {
            debug!(index, "Found record in LogWal");
            let logs = PyroLogs {
                module_logs: entry.module_logs,
                capability_logs: entry.capability_logs,
            };

            if let Some(err) = entry.failure {
                debug!(index, "Record is a failure");
                Ok(ExecutionRecord::Failure {
                    row_index: index,
                    input: input_row,
                    failure: err,
                    logs,
                })
            } else {
                debug!(index, "Record is a success");
                let success_row = self.output_manager.get_record(index).await?;
                Ok(ExecutionRecord::Success {
                    row_index: index,
                    input: input_row,
                    success: success_row,
                    logs,
                })
            }
        } else {
            debug!(
                index,
                "Logs cleaned/missing, attempting fallback search in output manager"
            );
            // Logs cleaned/missing, return blank logs
            if let Ok(success_row) = self.output_manager.get_record(index).await {
                debug!(index, "Fallback search found matching record");
                Ok(ExecutionRecord::Success {
                    row_index: index,
                    input: input_row,
                    success: success_row,
                    logs: PyroLogs::empty(),
                })
            } else {
                debug!(index, "Fallback search failed to find matching record");
                Err(PyroError::not_found(format!(
                    "Unable to find record {}",
                    index.to_string()
                )))
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
// PipelinePool (deprecated — sharding is now built into Pipeline)
// =============================================================================

#[deprecated(note = "Sharding is now built into Pipeline via the `shards` field. Use Pipeline directly.")]
pub struct PipelinePool {
    pipelines: Arc<Mutex<Vec<Pipeline>>>,
}

#[allow(deprecated)]
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
    ) -> PipelineResult<(Vec<ExecutionRecord>, Vec<ExecutionRecord>)> {
        let total_rows = batch.num_rows();
        info!(total_rows, "Processing batch");
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
        debug!(num_pipelines, "Distributing batch across pipelines");
        let (tx, mut rx) = mpsc::channel(100);
        let chunk_size = min(total_rows.div_ceil(num_pipelines), 1000);
        let mut handles = Vec::with_capacity(num_pipelines);

        for (i, pipeline) in pipelines.into_iter().enumerate() {
            let offset = i * chunk_size;
            if offset >= total_rows {
                handles.push(tokio::spawn(async move { pipeline }));
                continue;
            }

            let length = chunk_size.min(total_rows - offset);
            let batch_slice = batch.slice(offset, length);
            let tx_clone = tx.clone();

            handles.push(tokio::spawn(async move {
                debug!(offset, length, "Worker starting chunk processing");
                for j in 0..batch_slice.num_rows() {
                    let absolute_index = offset + j;

                    let result = match batch_slice.row(j) {
                        Ok(input_row) => pipeline.process(absolute_index, &input_row).await,
                        Err(e) => {
                            error!(absolute_index, "Failed to read row from batch: {}", e);
                            let record = ExecutionRecord::Failure {
                                row_index: absolute_index,
                                input: PyroRow::empty(),
                                failure: Ok(crate::CapturedError::new(e)),
                                logs: PyroLogs::empty(),
                            };
                            if tx_clone.send(record).await.is_err() {
                                break;
                            }
                            continue;
                        }
                    };

                    match result {
                        Ok(execution) => {
                            if tx_clone.send(execution).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            error!(absolute_index, "Pipeline process returned error: {}", e);
                            let record = ExecutionRecord::Failure {
                                row_index: absolute_index,
                                input: PyroRow::empty(),
                                failure: Err(e.to_string()),
                                logs: PyroLogs::empty(),
                            };
                            if tx_clone.send(record).await.is_err() {
                                break;
                            }
                        }
                    }
                }
                debug!(offset, "Worker finished chunk processing");

                pipeline
            }));
        }

        drop(tx);
        let mut success_results = Vec::with_capacity(total_rows);
        let mut failures: Vec<ExecutionRecord> = Vec::new();

        while let Some(row_result) = rx.recv().await {
            match &row_result {
                ExecutionRecord::Success { .. } => success_results.push(row_result),
                ExecutionRecord::Failure { .. } => failures.push(row_result),
            }
        }

        info!(
            successes = success_results.len(),
            failures = failures.len(),
            "Batch processing complete"
        );

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

        success_results.sort_by_key(|row| row.row_index());

        Ok((success_results, failures))
    }
}

pub type PipelineExecution = ExecutionRecord;

pub fn extract_upto_batch(
    executions: &[ExecutionRecord],
    _step_index: usize,
) -> anyhow::Result<Option<RecordBatch>> {
    if executions.is_empty() {
        return Ok(None);
    }

    // Find the first successful row to get its schema
    let first_success = executions.iter().find_map(|e| match e {
        ExecutionRecord::Success { success, .. } => Some(success),
        _ => None,
    });

    let Some(first_row) = first_success else {
        return Ok(None);
    };

    let schema = first_row.schema().map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let mut prebatch = PreBatch::new(schema);

    for e in executions {
        if let ExecutionRecord::Success { success, .. } = e {
            prebatch
                .push(success.clone())
                .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        }
    }

    prebatch.flush().map_err(|e| anyhow::anyhow!("{:?}", e))
}
