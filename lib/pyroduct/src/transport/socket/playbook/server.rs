use std::sync::Arc;
use tokio::sync::Mutex;

use pyro_artifacts::artifacts::PlaybookSpec;
use pyro_artifacts::cache::LoadedPlaybook;

use crate::format::header::{PyroData, PyroHeader};
use crate::format::log_wal::LogWal;
use crate::format::tokio::Request;
use crate::format::{Bridgeable, PyroRow};
use crate::module::PyroFactory;
use crate::module::SessionResult;
use crate::pipeline::{
    ExecutionRecord, Pipeline, session::SessionPipeline, session_diff::SessionDiffPipeline,
};
use crate::transport::socket::{PyroListener, PyroSocket};
use crate::{CapturedError, PyroError};

pub enum PlaybookServerCommand {
    AddCallback(uuid::Uuid, crate::pipeline::Callback),
    DeleteCallback(uuid::Uuid),
}

enum ServerPipeline {
    Normal(Pipeline),
    Session(SessionPipeline),
    SessionDiff(SessionDiffPipeline),
}

/// A server that listens for incoming [`PyroSocket`] connections and routes
/// requests using a single-level [`Pipeline`] instantiated from a playbook.
pub struct PlaybookServer {
    pipeline: Arc<Mutex<ServerPipeline>>,
    spec: Arc<PlaybookSpec>,
}

impl PlaybookServer {
    /// Create a new server from a loaded playbook.
    pub async fn new(playbook: &LoadedPlaybook) -> Result<Self, crate::pipeline::PipelineError> {
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

    /// Run the server, accepting connections on the provided listener.
    pub async fn run(self, listener: PyroListener) -> std::io::Result<()> {
        let server = Arc::new(self);

        loop {
            let socket = listener.accept().await?;
            tracing::info!("Accepted new connection");
            let server_clone = server.clone();

            tokio::spawn(async move {
                if let Err(e) = server_clone.handle_connection(socket).await {
                    tracing::error!("Connection closed with error: {:?}", e);
                }
            });
        }
    }

    /// Run the server, accepting connections on the provided listener, and listening for new commands.
    pub async fn run_with_callbacks(
        self,
        listener: PyroListener,
        mut command_rx: tokio::sync::mpsc::Receiver<PlaybookServerCommand>,
    ) -> std::io::Result<()> {
        let server = Arc::new(self);

        loop {
            tokio::select! {
                accept_res = listener.accept() => {
                    let socket = accept_res?;
                    tracing::info!("Accepted new connection");
                    let server_clone = server.clone();

                    tokio::spawn(async move {
                        if let Err(e) = server_clone.handle_connection(socket).await {
                            tracing::error!("Connection closed with error: {:?}", e);
                        }
                    });
                }
                Some(cmd) = command_rx.recv() => {
                    match cmd {
                        PlaybookServerCommand::AddCallback(uuid, cb) => {
                            server.add_callback(uuid, cb).await;
                        }
                        PlaybookServerCommand::DeleteCallback(uuid) => {
                            server.delete_callback(uuid).await;
                        }
                    }
                }
            }
        }
    }

    async fn handle_connection(&self, socket: PyroSocket) -> std::io::Result<()> {
        loop {
            let view = socket.recv().await?;
            let client_id = view.client_id();

            tracing::debug!(%client_id, "Received playbook request");

            let pipeline_arc = self.pipeline.clone();
            let socket_clone = socket.clone();

            tokio::spawn(async move {
                let response_view = match view.parse_as_error() {
                    Ok(_) => {
                        let py_ref = view.py_ref();
                        match PyroRow::expose_view(py_ref) {
                            Ok(row) => {
                                let mut pipeline = pipeline_arc.lock().await;
                                let native_row = PyroRow::from(&*row).to_static();
                                match &mut *pipeline {
                                    ServerPipeline::Normal(p) => {
                                        match p.process(0, &native_row).await {
                                            Ok(ExecutionRecord::Failure { failure, .. }) => {
                                                let err_msg = match failure {
                                                    Ok(captured) => format!("{:?}", captured),
                                                    Err(s) => s,
                                                };
                                                crate::PyroError::CodePanic(Box::new(
                                                    crate::CapturedError::new(err_msg),
                                                ))
                                                .encode()
                                                .view()
                                            }
                                            Ok(ExecutionRecord::Success { success, .. }) => success
                                                .ship()
                                                .map(|v| v.view())
                                                .unwrap_or_else(|e| e.encode().view()),
                                            Err(e) => e.encode().view(),
                                        }
                                    }
                                    ServerPipeline::Session(p) => {
                                        let mut prepped = true;
                                        let mut prep_error_view = None;
                                        if !p.active_sessions.contains_key(&client_id) {
                                            if let Err(e) = p.prep_session(client_id, &[]).await {
                                                prepped = false;
                                                let err_msg = match e.result {
                                                    Ok(captured) => format!("{:?}", captured),
                                                    Err(s) => s,
                                                };
                                                prep_error_view = Some(
                                                    crate::PyroError::CodePanic(Box::new(
                                                        crate::CapturedError::new(err_msg),
                                                    ))
                                                    .encode()
                                                    .view(),
                                                );
                                            }
                                        }

                                        if prepped {
                                            match p.call(client_id, &native_row).await {
                                                Ok(res) => match res {
                                                    SessionResult::Continue { result, .. }
                                                    | SessionResult::End { result, .. } => result
                                                        .ship()
                                                        .map(|v| v.view())
                                                        .unwrap_or_else(|e| e.encode().view()),
                                                    SessionResult::Terminate { .. } => {
                                                        PyroRow::empty()
                                                            .ship()
                                                            .map(|v| v.view())
                                                            .unwrap_or_else(|e| e.encode().view())
                                                    }
                                                },
                                                Err(e) => {
                                                    let err_msg = match e.result {
                                                        Ok(captured) => format!("{:?}", captured),
                                                        Err(s) => s,
                                                    };
                                                    crate::PyroError::CodePanic(Box::new(
                                                        crate::CapturedError::new(err_msg),
                                                    ))
                                                    .encode()
                                                    .view()
                                                }
                                            }
                                        } else {
                                            prep_error_view.unwrap()
                                        }
                                    }
                                    ServerPipeline::SessionDiff(p) => {
                                        let mut prepped = true;
                                        let mut prep_error_view = None;
                                        if !p.active_sessions.contains_key(&client_id) {
                                            if let Err(e) =
                                                p.prep_session(client_id, &[], &[]).await
                                            {
                                                prepped = false;
                                                let err_msg = match e.result {
                                                    Ok(captured) => format!("{:?}", captured),
                                                    Err(s) => s,
                                                };
                                                prep_error_view = Some(
                                                    crate::PyroError::CodePanic(Box::new(
                                                        crate::CapturedError::new(err_msg),
                                                    ))
                                                    .encode()
                                                    .view(),
                                                );
                                            }
                                        }

                                        if prepped {
                                            match p.call(client_id, &native_row).await {
                                                Ok(res) => match res {
                                                    SessionResult::Continue { result, .. }
                                                    | SessionResult::End { result, .. } => result
                                                        .ship()
                                                        .map(|v| v.view())
                                                        .unwrap_or_else(|e| e.encode().view()),
                                                    SessionResult::Terminate { .. } => {
                                                        PyroRow::empty()
                                                            .ship()
                                                            .map(|v| v.view())
                                                            .unwrap_or_else(|e| e.encode().view())
                                                    }
                                                },
                                                Err(e) => {
                                                    let err_msg = match e.result {
                                                        Ok(captured) => format!("{:?}", captured),
                                                        Err(s) => s,
                                                    };
                                                    crate::PyroError::CodePanic(Box::new(
                                                        crate::CapturedError::new(err_msg),
                                                    ))
                                                    .encode()
                                                    .view()
                                                }
                                            }
                                        } else {
                                            prep_error_view.unwrap()
                                        }
                                    }
                                }
                            }
                            Err(e) => e.encode().view(),
                        }
                    }
                    Err(e) => e.encode().view(),
                };

                let response = Request {
                    client_id: Some(client_id),
                    class_id: Some(view.class_id()),
                    fn_id: Some(view.fn_id()),
                    mux_id: Some(view.mux_id()),
                    inner: response_view,
                };

                if let Err(e) = socket_clone.send(response).await {
                    tracing::error!(
                        "Failed to send response for mux_id {}: {:?}",
                        view.mux_id(),
                        e
                    );
                } else {
                    tracing::debug!(mux_id = view.mux_id(), "Response sent successfully");
                }
            });
        }
    }
}
