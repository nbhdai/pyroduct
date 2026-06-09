use std::sync::Arc;
use tokio::sync::Mutex;

use pyro_artifacts::artifacts::PlaybookSpec;
use pyro_artifacts::cache::LoadedPlaybook;

use crate::format::{PyroRow, log_wal::LogWal};
use crate::module::PyroFactory;
use crate::module::interconnect::PlaybookInterconnect;
use crate::pipeline::{
    Pipeline, PipelineError,
    session::{SessionPipeline, SessionStatusFilter, SessionStatusManager},
    session_diff::SessionDiffPipeline,
};
use crate::{CapturedError, PyroError};

pub enum ServerPipeline {
    Normal(Pipeline),
    Session(SessionPipeline),
    SessionDiff(SessionDiffPipeline),
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ServerExecutionRecord {
    Normal(crate::pipeline::ExecutionRecord),
    Session(crate::pipeline::session::SessionExecutionRecord),
    SessionDiff(crate::pipeline::session_diff::SessionDiffExecutionRecord),
}

impl ServerExecutionRecord {
    pub fn into_result(self) -> Result<(u32, PyroRow<'static>), CapturedError> {
        match self {
            ServerExecutionRecord::Normal(rec) => match rec {
                crate::pipeline::ExecutionRecord::Success {
                    row_index, success, ..
                } => Ok((row_index as u32, success)),
                crate::pipeline::ExecutionRecord::Failure { failure, .. } => {
                    Err(failure.unwrap_or_else(|s| CapturedError::new(s)))
                }
            },
            ServerExecutionRecord::Session(rec) => match rec {
                crate::pipeline::session::SessionExecutionRecord::Success {
                    row_index,
                    success,
                    ..
                } => Ok((row_index as u32, success)),
                crate::pipeline::session::SessionExecutionRecord::Failure { failure, .. } => {
                    Err(failure.unwrap_or_else(|s| CapturedError::new(s)))
                }
            },
            ServerExecutionRecord::SessionDiff(rec) => match rec {
                crate::pipeline::session_diff::SessionDiffExecutionRecord::Success {
                    row_index,
                    success,
                    ..
                } => Ok((row_index as u32, success)),
                crate::pipeline::session_diff::SessionDiffExecutionRecord::Failure {
                    failure,
                    ..
                } => Err(failure.unwrap_or_else(|s| CapturedError::new(s))),
            },
        }
    }
}

/// A reusable server pipeline that routes incoming rows to the correct underlying
/// execution engine based on the playbook type (Normal, Session, SessionDiff).
///
/// For Session and SessionDiff pipelines, per-shard locking enables concurrent
/// session execution without holding the top-level lock during WASM calls.
#[derive(Clone)]
pub struct PipelineServer {
    pipeline: Arc<Mutex<ServerPipeline>>,
    spec: Arc<PlaybookSpec>,
}

impl PipelineServer {
    /// Create a new server pipeline from a loaded playbook.
    pub async fn new(playbook: &LoadedPlaybook) -> Result<Self, PipelineError> {
        Self::new_internal(playbook, None).await
    }

    /// Create a new server pipeline from a loaded playbook with an interconnect.
    pub async fn new_with_interconnect(
        playbook: &LoadedPlaybook,
        interconnect: Arc<dyn PlaybookInterconnect>,
    ) -> Result<Self, PipelineError> {
        Self::new_internal(playbook, Some(interconnect)).await
    }

    async fn new_internal(
        playbook: &LoadedPlaybook,
        interconnect: Option<Arc<dyn PlaybookInterconnect>>,
    ) -> Result<Self, PipelineError> {
        let mut factory = PyroFactory::from_playbook(playbook)?;
        if let Some(ic) = interconnect {
            factory.set_interconnect(ic);
        }
        let spec = Arc::new(factory.spec().clone());
        let input_schema = factory.spec().func.input.clone();
        let output_schema = factory.spec().func.output.clone();
        let kind = factory.spec().func.kind;

        let num_workers = playbook.num_workers;
        let mut shards = Vec::with_capacity(num_workers);
        for _ in 0..num_workers {
            let instance = factory.instantiate().await?;
            shards.push(tokio::sync::Mutex::new(instance));
        }

        let server_pipeline = match kind {
            pyro_spec::ModuleKind::Normal => {
                let pipeline = Pipeline {
                    shards,
                    success_log_retention_secs: 3600,
                    error_log_retention_secs: 86400 * 7,
                    log_manager: tokio::sync::Mutex::new(
                        LogWal::open(playbook.log_dir.clone(), 1000)
                            .await
                            .map_err(|io| {
                                PyroError::local_io(
                                    CapturedError::new("Unable to make the log wal")
                                        .with_source(io),
                                )
                            })?,
                    ),
                    input_manager: crate::pipeline::data::DataManager::new(
                        playbook.input_dir.clone(),
                        input_schema,
                    ),
                    output_manager: crate::pipeline::data::DataManager::new(
                        playbook.output_dir.clone(),
                        output_schema,
                    ),
                    callbacks: tokio::sync::Mutex::new(Vec::new()),
                };
                ServerPipeline::Normal(pipeline)
            }
            pyro_spec::ModuleKind::Session => {
                let session_status_manager = SessionStatusManager::new(&playbook.output_dir)?;
                let pipeline = SessionPipeline {
                    shards,
                    spec: spec.clone(),
                    success_log_retention_secs: 3600,
                    error_log_retention_secs: 86400 * 7,
                    log_manager: tokio::sync::Mutex::new(
                        LogWal::open(playbook.log_dir.clone(), 1000)
                            .await
                            .map_err(|io| {
                                PyroError::local_io(
                                    CapturedError::new("Unable to make the log wal")
                                        .with_source(io),
                                )
                            })?,
                    ),
                    output_manager: crate::pipeline::data::DataManager::new(
                        playbook.output_dir.clone(),
                        output_schema,
                    ),
                    log_dir: playbook.log_dir.clone(),
                    output_dir: playbook.output_dir.clone(),
                    wal_capacity: 1000,
                    active_sessions: tokio::sync::Mutex::new(std::collections::HashMap::new()),
                    callbacks: tokio::sync::Mutex::new(Vec::new()),
                    session_status_manager,
                };
                ServerPipeline::Session(pipeline)
            }
            pyro_spec::ModuleKind::SessionDiff => {
                let session_status_manager = SessionStatusManager::new(&playbook.output_dir)?;
                let pipeline = SessionDiffPipeline {
                    shards,
                    spec: spec.clone(),
                    success_log_retention_secs: 3600,
                    error_log_retention_secs: 86400 * 7,
                    log_manager: tokio::sync::Mutex::new(
                        LogWal::open(playbook.log_dir.clone(), 1000)
                            .await
                            .map_err(|io| {
                                PyroError::local_io(
                                    CapturedError::new("Unable to make the log wal")
                                        .with_source(io),
                                )
                            })?,
                    ),
                    output_manager: crate::pipeline::data::DataManager::new(
                        playbook.output_dir.clone(),
                        output_schema,
                    ),
                    log_dir: playbook.log_dir.clone(),
                    output_dir: playbook.output_dir.clone(),
                    wal_capacity: 1000,
                    active_sessions: tokio::sync::Mutex::new(std::collections::HashMap::new()),
                    callbacks: tokio::sync::Mutex::new(Vec::new()),
                    session_status_manager,
                };
                ServerPipeline::SessionDiff(pipeline)
            }
        };

        Ok(Self {
            pipeline: Arc::new(Mutex::new(server_pipeline)),
            spec,
        })
    }

    /// Add a callback dynamically to the running pipeline.
    pub async fn add_callback(&self, uuid: uuid::Uuid, callback: crate::pipeline::Callback) {
        let pipeline = self.pipeline.lock().await;
        match &*pipeline {
            ServerPipeline::Normal(p) => {
                let mut cbs = p.callbacks.lock().await;
                cbs.push((uuid, callback));
            }
            ServerPipeline::Session(p) => {
                let mut cbs = p.callbacks.lock().await;
                cbs.push((uuid, callback));
            }
            ServerPipeline::SessionDiff(p) => {
                let mut cbs = p.callbacks.lock().await;
                cbs.push((uuid, callback));
            }
        }
    }

    /// Delete a callback dynamically from the running pipeline by UUID.
    pub async fn delete_callback(&self, uuid: uuid::Uuid) {
        let pipeline = self.pipeline.lock().await;
        match &*pipeline {
            ServerPipeline::Normal(p) => {
                let mut cbs = p.callbacks.lock().await;
                cbs.retain(|(u, _)| *u != uuid);
            }
            ServerPipeline::Session(p) => {
                let mut cbs = p.callbacks.lock().await;
                cbs.retain(|(u, _)| *u != uuid);
            }
            ServerPipeline::SessionDiff(p) => {
                let mut cbs = p.callbacks.lock().await;
                cbs.retain(|(u, _)| *u != uuid);
            }
        }
    }

    /// Get the `ModuleSpec` for the playbook.
    pub fn spec(&self) -> Arc<PlaybookSpec> {
        self.spec.clone()
    }

    /// Get the current number of samples.
    pub async fn len(&self) -> usize {
        let pipeline = self.pipeline.lock().await;
        match &*pipeline {
            ServerPipeline::Normal(p) => p.input_manager.len().await,
            ServerPipeline::Session(p) => p.output_manager.len().await,
            ServerPipeline::SessionDiff(p) => p.output_manager.len().await,
        }
    }

    /// Get a chunk of input data with pagination, returning up to limit elements.
    pub async fn get_input_batch(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<Option<arrow::array::RecordBatch>, PyroError> {
        let pipeline = self.pipeline.lock().await;
        match &*pipeline {
            ServerPipeline::Normal(p) => p.input_manager.get_batch_slice(offset, limit).await,
            ServerPipeline::Session(_) => Ok(None),
            ServerPipeline::SessionDiff(_) => Ok(None),
        }
    }

    /// Get a chunk of output data with pagination, returning up to limit elements.
    pub async fn get_output_batch(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<Option<arrow::array::RecordBatch>, PyroError> {
        let pipeline = self.pipeline.lock().await;
        match &*pipeline {
            ServerPipeline::Normal(p) => p.output_manager.get_batch_slice(offset, limit).await,
            ServerPipeline::Session(p) => p.output_manager.get_batch_slice(offset, limit).await,
            ServerPipeline::SessionDiff(p) => p.output_manager.get_batch_slice(offset, limit).await,
        }
    }

    /// List all sessions for this pipeline (only for Session and SessionDiff pipelines).
    pub async fn list_sessions(
        &self,
        filter: Option<SessionStatusFilter>,
    ) -> Result<Vec<(u32, String)>, PyroError> {
        let pipeline = self.pipeline.lock().await;
        match &*pipeline {
            ServerPipeline::Normal(_) => Ok(Vec::new()),
            ServerPipeline::Session(p) => p.list_sessions(filter),
            ServerPipeline::SessionDiff(p) => p.list_sessions(filter),
        }
    }

    /// Retrieve a single execution record by its global ID.
    pub async fn get(&self, id: u32) -> Result<ServerExecutionRecord, PyroError> {
        let pipeline = self.pipeline.lock().await;
        match &*pipeline {
            ServerPipeline::Normal(p) => {
                let rec = p.get_record(id as usize).await?;
                Ok(ServerExecutionRecord::Normal(rec))
            }
            ServerPipeline::Session(p) => {
                let rec = p.get(id).await?;
                Ok(ServerExecutionRecord::Session(rec))
            }
            ServerPipeline::SessionDiff(p) => {
                let rec = p.get(id).await?;
                Ok(ServerExecutionRecord::SessionDiff(rec))
            }
        }
    }

    /// Call the pipeline without a specific session ID.
    /// For session-based pipelines, this will generate a new session ID.
    /// Returns `(session_id, success_row)`.
    pub async fn call(&self, row: PyroRow<'_>) -> Result<ServerExecutionRecord, CapturedError> {
        let mut pipeline = self.pipeline.lock().await;
        let native_row = row.to_static();
        match &mut *pipeline {
            ServerPipeline::Normal(p) => match p.call(&native_row).await {
                Ok(r) => Ok(ServerExecutionRecord::Normal(r)),
                Err(e) => Err(CapturedError::new(e.to_string())),
            },
            ServerPipeline::Session(p) => {
                let session_id = p.next_session_id().await;
                if let Err(e) = p.prep_session(session_id, &[]).await {
                    let captured = match e.result {
                        Ok(captured) => captured,
                        Err(s) => CapturedError::new(s),
                    };
                    return Err(captured);
                }

                match p.call(session_id, &native_row).await {
                    Ok(r) => Ok(ServerExecutionRecord::Session(r)),

                    Err(e) => {
                        let captured = match e.result {
                            Ok(captured) => captured,
                            Err(s) => CapturedError::new(s),
                        };
                        Err(captured)
                    }
                }
            }
            ServerPipeline::SessionDiff(p) => {
                let session_id = p.next_session_id().await;
                if let Err(e) = p.prep_session(session_id, &[], &[]).await {
                    let captured = match e.result {
                        Ok(captured) => captured,
                        Err(s) => CapturedError::new(s),
                    };
                    return Err(captured);
                }

                match p.call(session_id, &native_row).await {
                    Ok(r) => Ok(ServerExecutionRecord::SessionDiff(r)),
                    Err(e) => {
                        let captured = match e.result {
                            Ok(captured) => captured,
                            Err(s) => CapturedError::new(s),
                        };
                        Err(captured)
                    }
                }
            }
        }
    }

    /// Call the pipeline with a specific client ID (useful for session pipelines).
    pub async fn call_session(
        &self,
        client_id: u32,
        row: PyroRow<'_>,
    ) -> Result<ServerExecutionRecord, CapturedError> {
        let mut pipeline = self.pipeline.lock().await;
        match &mut *pipeline {
            ServerPipeline::Normal(p) => match p.process(client_id as usize, &row).await {
                Ok(r) => Ok(ServerExecutionRecord::Normal(r)),
                Err(e) => Err(CapturedError::new(e.to_string())),
            },
            ServerPipeline::Session(p) => {
                let has_active = {
                    let active = p.active_sessions.lock().await;
                    active.contains_key(&client_id)
                };
                if !has_active {
                    if let Err(e) = p.prep_session(client_id, &[]).await {
                        let captured = match e.result {
                            Ok(captured) => captured,
                            Err(s) => CapturedError::new(s),
                        };
                        return Err(captured);
                    }
                }

                match p.call(client_id, &row).await {
                    Ok(r) => Ok(ServerExecutionRecord::Session(r)),
                    Err(e) => {
                        let captured = match e.result {
                            Ok(captured) => captured,
                            Err(s) => CapturedError::new(s),
                        };
                        Err(captured)
                    }
                }
            }
            ServerPipeline::SessionDiff(p) => {
                let has_active = {
                    let active = p.active_sessions.lock().await;
                    active.contains_key(&client_id)
                };
                if !has_active {
                    if let Err(e) = p.prep_session(client_id, &[], &[]).await {
                        let captured = match e.result {
                            Ok(captured) => captured,
                            Err(s) => CapturedError::new(s),
                        };
                        return Err(captured);
                    }
                }

                match p.call(client_id, &row).await {
                    Ok(r) => Ok(ServerExecutionRecord::SessionDiff(r)),
                    Err(e) => {
                        let captured = match e.result {
                            Ok(captured) => captured,
                            Err(s) => CapturedError::new(s),
                        };
                        Err(captured)
                    }
                }
            }
        }
    }
}
