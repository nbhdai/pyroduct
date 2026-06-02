use std::sync::Arc;
use tokio::sync::Mutex;

use pyro_artifacts::artifacts::PlaybookSpec;
use pyro_artifacts::cache::LoadedPlaybook;

use crate::format::PyroFailure;
use crate::format::PyroRow;
use crate::format::PyroSuccess;
use crate::format::SessionResult;
use crate::format::log_wal::LogWal;
use crate::module::PyroFactory;
use crate::pipeline::{
    ExecutionRecord, Pipeline, PipelineError, session::SessionPipeline,
    session_diff::SessionDiffPipeline,
};
use crate::{CapturedError, PyroError};

pub enum ServerPipeline {
    Normal(Pipeline),
    Session(SessionPipeline),
    SessionDiff(SessionDiffPipeline),
}

/// A reusable server pipeline that routes incoming rows to the correct underlying
/// execution engine based on the playbook type (Normal, Session, SessionDiff).
#[derive(Clone)]
pub struct PipelineServer {
    pipeline: Arc<Mutex<ServerPipeline>>,
    spec: Arc<PlaybookSpec>,
}

impl PipelineServer {
    /// Create a new server pipeline from a loaded playbook.
    pub async fn new(playbook: &LoadedPlaybook) -> Result<Self, PipelineError> {
        let factory = PyroFactory::from_playbook(playbook)?;
        let instance = factory.instantiate().await?;
        let spec = instance.spec.clone();
        let input_schema = factory.spec().func.input.clone();
        let output_schema = factory.spec().func.output.clone();
        let kind = factory.spec().func.kind;

        let server_pipeline = match kind {
            pyro_spec::ModuleKind::Normal => {
                let pipeline = Pipeline {
                    step: instance,
                    success_log_retention_secs: 3600,
                    error_log_retention_secs: 86400 * 7,
                    log_manager: LogWal::open(playbook.log_dir.clone(), 1000).await.map_err(
                        |io| {
                            PyroError::local_io(
                                CapturedError::new("Unable to make the log wal").with_source(io),
                            )
                        },
                    )?,
                    input_manager: crate::pipeline::data::DataManager::new(
                        playbook.input_dir.clone(),
                        input_schema,
                    ),
                    output_manager: crate::pipeline::data::DataManager::new(
                        playbook.output_dir.clone(),
                        output_schema,
                    ),
                    callbacks: Vec::new(),
                };
                ServerPipeline::Normal(pipeline)
            }
            pyro_spec::ModuleKind::Session => {
                let pipeline = SessionPipeline {
                    step: instance,
                    success_log_retention_secs: 3600,
                    error_log_retention_secs: 86400 * 7,
                    log_manager: LogWal::open(playbook.log_dir.clone(), 1000).await.map_err(
                        |io| {
                            PyroError::local_io(
                                CapturedError::new("Unable to make the log wal").with_source(io),
                            )
                        },
                    )?,
                    output_manager: crate::pipeline::data::DataManager::new(
                        playbook.output_dir.clone(),
                        output_schema,
                    ),
                    log_dir: playbook.log_dir.clone(),
                    output_dir: playbook.output_dir.clone(),
                    wal_capacity: 1000,
                    active_sessions: std::collections::HashMap::new(),
                    callbacks: Vec::new(),
                };
                ServerPipeline::Session(pipeline)
            }
            pyro_spec::ModuleKind::SessionDiff => {
                let pipeline = SessionDiffPipeline {
                    step: instance,
                    success_log_retention_secs: 3600,
                    error_log_retention_secs: 86400 * 7,
                    log_manager: LogWal::open(playbook.log_dir.clone(), 1000).await.map_err(
                        |io| {
                            PyroError::local_io(
                                CapturedError::new("Unable to make the log wal").with_source(io),
                            )
                        },
                    )?,
                    output_manager: crate::pipeline::data::DataManager::new(
                        playbook.output_dir.clone(),
                        output_schema,
                    ),
                    log_dir: playbook.log_dir.clone(),
                    output_dir: playbook.output_dir.clone(),
                    wal_capacity: 1000,
                    active_sessions: std::collections::HashMap::new(),
                    callbacks: Vec::new(),
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
        let mut pipeline = self.pipeline.lock().await;
        match &mut *pipeline {
            ServerPipeline::Normal(p) => p.callbacks.push((uuid, callback)),
            ServerPipeline::Session(p) => p.callbacks.push((uuid, callback)),
            ServerPipeline::SessionDiff(p) => p.callbacks.push((uuid, callback)),
        }
    }

    /// Delete a callback dynamically from the running pipeline by UUID.
    pub async fn delete_callback(&self, uuid: uuid::Uuid) {
        let mut pipeline = self.pipeline.lock().await;
        match &mut *pipeline {
            ServerPipeline::Normal(p) => p.callbacks.retain(|(u, _)| *u != uuid),
            ServerPipeline::Session(p) => p.callbacks.retain(|(u, _)| *u != uuid),
            ServerPipeline::SessionDiff(p) => p.callbacks.retain(|(u, _)| *u != uuid),
        }
    }

    /// Get the `ModuleSpec` for the playbook.
    pub fn spec(&self) -> Arc<PlaybookSpec> {
        self.spec.clone()
    }

    /// Call the pipeline without a specific session ID.
    /// For session-based pipelines, this will generate a new session ID.
    /// Returns `(session_id, success_row)`.
    pub async fn call(&self, row: PyroRow<'_>) -> Result<(u32, PyroRow<'static>), CapturedError> {
        let mut pipeline = self.pipeline.lock().await;
        let native_row = row.to_static();
        match &mut *pipeline {
            ServerPipeline::Normal(p) => match p.call(&native_row).await {
                Ok(PyroSuccess { row, .. }) => Ok((0, row)),
                Err(PyroFailure { result, .. }) => {
                    let captured = match result {
                        Ok(captured) => captured,
                        Err(s) => CapturedError::new(s),
                    };
                    Err(captured)
                }
            },
            ServerPipeline::Session(p) => {
                let session_id = p.next_session_id();
                if let Err(e) = p.prep_session(session_id, &[]).await {
                    let captured = match e.result {
                        Ok(captured) => captured,
                        Err(s) => CapturedError::new(s),
                    };
                    return Err(captured);
                }

                match p.call(session_id, &native_row).await {
                    Ok(res) => match res {
                        SessionResult::Continue { result, .. }
                        | SessionResult::End { result, .. } => Ok((session_id, result)),
                        SessionResult::Terminate { .. } => Ok((session_id, PyroRow::empty())),
                    },
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
                let session_id = p.next_session_id();
                if let Err(e) = p.prep_session(session_id, &[], &[]).await {
                    let captured = match e.result {
                        Ok(captured) => captured,
                        Err(s) => CapturedError::new(s),
                    };
                    return Err(captured);
                }

                match p.call(session_id, &native_row).await {
                    Ok(res) => match res {
                        SessionResult::Continue { result, .. }
                        | SessionResult::End { result, .. } => Ok((session_id, result)),
                        SessionResult::Terminate { .. } => Ok((session_id, PyroRow::empty())),
                    },
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
    ) -> Result<PyroRow<'static>, CapturedError> {
        let mut pipeline = self.pipeline.lock().await;
        match &mut *pipeline {
            ServerPipeline::Normal(p) => match p.process(client_id as usize, &row).await {
                Ok(ExecutionRecord::Success { success, .. }) => Ok(success),
                Ok(ExecutionRecord::Failure { failure, .. }) => {
                    let captured = match failure {
                        Ok(captured) => captured,
                        Err(s) => CapturedError::new(s),
                    };
                    Err(captured)
                }
                Err(e) => Err(CapturedError::new(e.to_string())),
            },
            ServerPipeline::Session(p) => {
                if !p.active_sessions.contains_key(&client_id) {
                    if let Err(e) = p.prep_session(client_id, &[]).await {
                        let captured = match e.result {
                            Ok(captured) => captured,
                            Err(s) => CapturedError::new(s),
                        };
                        return Err(captured);
                    }
                }

                match p.call(client_id, &row).await {
                    Ok(res) => match res {
                        SessionResult::Continue { result, .. }
                        | SessionResult::End { result, .. } => Ok(result),
                        SessionResult::Terminate { .. } => Ok(PyroRow::empty()),
                    },
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
                if !p.active_sessions.contains_key(&client_id) {
                    if let Err(e) = p.prep_session(client_id, &[], &[]).await {
                        let captured = match e.result {
                            Ok(captured) => captured,
                            Err(s) => CapturedError::new(s),
                        };
                        return Err(captured);
                    }
                }

                match p.call(client_id, &row).await {
                    Ok(res) => match res {
                        SessionResult::Continue { result, .. }
                        | SessionResult::End { result, .. } => Ok(result),
                        SessionResult::Terminate { .. } => Ok(PyroRow::empty()),
                    },
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
