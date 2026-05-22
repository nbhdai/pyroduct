use std::sync::Arc;
use tokio::sync::Mutex;

use pyro_artifacts::cache::LoadedPlaybook;

use crate::format::header::{PyroData, PyroHeader};
use crate::format::log_wal::LogWal;
use crate::format::tokio::Request;
use crate::format::{Bridgeable, PyroRow};
use crate::module::PyroFactory;
use crate::pipeline::{ExecutionRecord, Pipeline};
use crate::transport::{PyroListener, PyroSocket};
use crate::{CapturedError, PyroError};

/// A server that listens for incoming [`PyroSocket`] connections and routes
/// requests using a single-level [`Pipeline`] instantiated from a playbook.
pub struct PlaybookServer {
    pipeline: Arc<Mutex<Pipeline>>,
}

impl PlaybookServer {
    /// Create a new server from a loaded playbook.
    pub async fn new(playbook: &LoadedPlaybook) -> Result<Self, crate::pipeline::PipelineError> {
        let factory = PyroFactory::from_playbook(playbook)?;
        let instance = factory.instantiate().await?;
        let input_schema = factory.spec().func.input.clone();
        let output_schema = factory.spec().func.output.clone();

        let pipeline = Pipeline {
            step: instance,
            success_log_retention_secs: 3600,
            error_log_retention_secs: 86400 * 7,
            log_manager: LogWal::open(playbook.log_dir.clone(), 1000)
                .await
                .map_err(|io| {
                    PyroError::local_io(
                        CapturedError::new("Unable to make the log wal").with_source(io),
                    )
                })?,
            input_manager: crate::pipeline::data::DataManager::new(
                playbook.input_dir.clone(),
                input_schema,
            ),
            output_manager: crate::pipeline::data::DataManager::new(
                playbook.output_dir.clone(),
                output_schema,
            ),
        };
        Ok(Self {
            pipeline: Arc::new(Mutex::new(pipeline)),
        })
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

    async fn handle_connection(&self, socket: PyroSocket) -> std::io::Result<()> {
        loop {
            let view = socket.recv().await?;
            let mux_id = view.mux_id();

            tracing::debug!(%mux_id, "Received playbook request");

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
                                match pipeline.process(0, &native_row).await {
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
                                    Ok(ExecutionRecord::Success { success, .. }) => {
                                        success.ship().map(|v| v.view()).unwrap_or_else(|e| {
                                            crate::PyroError::from(e).encode().view()
                                        })
                                    }
                                    Err(e) => crate::PyroError::from(e).encode().view(),
                                }
                            }
                            Err(e) => crate::PyroError::from(e).encode().view(),
                        }
                    }
                    Err(e) => crate::PyroError::from(e).encode().view(),
                };

                let response = Request {
                    client_id: Some(view.client_id()),
                    class_id: Some(view.class_id()),
                    fn_id: Some(view.fn_id()),
                    mux_id: Some(mux_id),
                    inner: response_view,
                };

                if let Err(e) = socket_clone.send(response).await {
                    tracing::error!("Failed to send response for mux_id {}: {:?}", mux_id, e);
                } else {
                    tracing::debug!(%mux_id, "Response sent successfully");
                }
            });
        }
    }
}
