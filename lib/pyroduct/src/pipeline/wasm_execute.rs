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
}};
use crate::module::PyroInstance;

use super::{PipelineError, PipelineResult};

// =============================================================================
// Pipeline
// =============================================================================

#[derive(Clone)]
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
    pub steps: Vec<PyroInstance>,
}

impl Pipeline {
    /// Run the input through every step in sequence.
    ///
    /// Returns `Ok(Ok(row))` on success, `Ok(Err(failure))` if a module
    /// returned a logic error (with partial data), or `Err` on infrastructure
    /// failure.
    #[instrument(skip(self, input))]
    pub async fn process(&mut self, input: &PyroRow<'_>) -> PipelineExecution {
        let pipeline_len = self.steps.len();
        info!("Pipeline Start: Executing {} steps", pipeline_len);

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
        for (i, step) in self.steps.iter_mut().enumerate() {
            debug!("Pipeline Step {}/{}: Processing", i + 1, pipeline_len);

            match step.call(&result).await {
                Ok(output) => {
                    debug!("Pipeline Step {}/{}: Success", i + 1, pipeline_len);
                    result.extend(output.row.clone());
                    execution.steps.push(output);
                }
                Err(failure) => {
                    match &failure.result {
                        Ok(error) => warn!(
                            "Pipeline Step {}/{}: Module returned error: {}",
                            i + 1,
                            pipeline_len,
                            error
                        ),
                        Err(error) => error!(
                            "Pipeline Step {}/{}: Pyroduct Failed: {}",
                            i + 1,
                            pipeline_len,
                            error
                        ),
                    }
                    execution.failure = Some(failure);
                    return execution;
                }
            }
        }

        info!(
            "Pipeline Complete: Successfully finished all {} steps",
            pipeline_len
        );
        execution
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
