use std::cmp::min;
use std::sync::Arc;

use arrow::array::RecordBatch;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, mpsc};
use tracing::{error, instrument, warn};

use crate::CapturedError;
use crate::format::log_wal::{LogEntry, LogWal};
use crate::module::PyroInstance;
use crate::{
    PyroError,
    format::{
        PyroLogs, PyroSuccess,
        value::{
            PyroRow,
            arrow::{PreBatch, Rowable},
        },
    },
    pipeline::{PipelineError, PipelineResult},
};

use super::data::DataManager;

#[derive(Clone)]
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
    pub step: PyroInstance,
    pub success_log_retention_secs: u64,
    pub error_log_retention_secs: u64,
    pub log_manager: LogWal,
    pub input_manager: DataManager,
    pub output_manager: DataManager,
}

impl Pipeline {
    /// Run the input through the single step.
    #[instrument(skip(self, input))]
    pub async fn process(
        &mut self,
        row_index: usize,
        input: &PyroRow<'_>,
    ) -> Result<ExecutionRecord, PyroError> {
        let mut result: PyroRow<'static> = input.clone().into_owned();

        let mut success = Some(PyroSuccess {
            row: result.clone(),
            logs: PyroLogs::empty(),
        });
        let mut failure = None;

        match self.step.call(&result).await {
            Ok(output) => {
                result.extend(output.row.clone());
                success = Some(output);
            }
            Err(f) => {
                match &f.result {
                    Ok(error) => warn!("Pipeline Step: Module returned error: {}", error),
                    Err(error) => error!("Pipeline Step: Pyroduct Failed: {}", error),
                }
                failure = Some(f);
            }
        }

        let record = if let Some(f) = failure {
            ExecutionRecord::Failure {
                row_index,
                input: input.clone().into_owned(),
                failure: f.result,
                logs: f.logs,
            }
        } else {
            let s = success.unwrap();
            ExecutionRecord::Success {
                row_index,
                input: input.clone().into_owned(),
                success: s.row,
                logs: s.logs,
            }
        };

        if let ExecutionRecord::Success { .. } = &record {
            self.output_manager.push_record(&result)?;
        }

        let log_entry = match &record {
            ExecutionRecord::Success {
                row_index,
                logs,
                ..
            } => LogEntry {
                row_index: *row_index,
                module_logs: logs.module_logs.clone(),
                capability_logs: logs.capability_logs.clone(),
                failure: None,
                success_index: Some(self.output_manager.len() - 1),
            },
            ExecutionRecord::Failure {
                row_index,
                logs,
                failure,
                ..
            } => LogEntry {
                row_index: *row_index,
                module_logs: logs.module_logs.clone(),
                capability_logs: logs.capability_logs.clone(),
                failure: failure.as_ref().ok().cloned(),
                success_index: None,
            },
        };
        self.log_manager.append(&log_entry).await?;

        Ok(record)
    }

    /// Retrieve a single record by its global index.
    pub async fn get_record(&self, index: usize) -> Result<ExecutionRecord, PyroError> {
        let input_row = self.input_manager.get_record(index)?;

        // Try to read from LogWal in O(1)
        let log_entry = self.log_manager.get(index).await
            .map_err(|e| PyroError::local_io(CapturedError::new(e)))?;

        if let Some(entry) = log_entry {
            let logs = PyroLogs {
                module_logs: entry.module_logs,
                capability_logs: entry.capability_logs,
            };

            if let Some(err) = entry.failure {
                Ok(ExecutionRecord::Failure {
                    row_index: index,
                    input: input_row,
                    failure: Ok(err),
                    logs,
                })
            } else {
                let success_index = entry.success_index.unwrap_or(0);
                let success_row = self.output_manager.get_record(success_index)?;
                Ok(ExecutionRecord::Success {
                    row_index: index,
                    input: input_row,
                    success: success_row,
                    logs,
                })
            }
        } else {
            // Logs cleaned/missing, return blank logs
            let output_len = self.output_manager.len();
            let mut matching_row = None;
            for k in 0..output_len {
                if let Ok(row) = self.output_manager.get_record(k) {
                    let mut matches = true;
                    for (field_name, val) in input_row.iter() {
                        if row.get(field_name) != Some(val) {
                            matches = false;
                            break;
                        }
                    }
                    if matches {
                        matching_row = Some(row);
                        break;
                    }
                }
            }

            if let Some(success_row) = matching_row {
                Ok(ExecutionRecord::Success {
                    row_index: index,
                    input: input_row,
                    success: success_row,
                    logs: PyroLogs::empty(),
                })
            } else {
                Ok(ExecutionRecord::Failure {
                    row_index: index,
                    input: input_row,
                    failure: Err("Log cleaned and no success record found".to_string()),
                    logs: PyroLogs::empty(),
                })
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
    ) -> PipelineResult<(Vec<ExecutionRecord>, Vec<ExecutionRecord>)> {
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

                    let result = match batch_slice.row(j) {
                        Ok(input_row) => pipeline.process(absolute_index, &input_row).await,
                        Err(e) => {
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
