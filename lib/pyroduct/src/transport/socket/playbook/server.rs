use crate::format::header::{PyroData, PyroHeader};
use crate::format::tokio::Request;
use crate::format::{Bridgeable, PyroRow};
use crate::pipeline::PipelineServer;
use crate::transport::socket::{PyroListener, PyroSocket};

/// A server that listens for incoming [`PyroSocket`] connections and routes
/// requests using a single-level [`PipelineServer`] instantiated from a playbook.

/// Run the server in a background task, accepting connections on the provided listener.
/// Returns a `tokio::sync::oneshot::Sender<()>` which can be used to interrupt and stop the server.
pub fn run(
    server: PipelineServer,
    listener: PyroListener,
) -> tokio::sync::oneshot::Sender<()> {
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                accept_res = listener.accept() => {
                    let socket = match accept_res {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::error!("Failed to accept connection: {:?}", e);
                            break;
                        }
                    };
                    tracing::info!("Accepted new connection");
                    let server_clone = server.clone();

                    tokio::spawn(async move {
                        if let Err(e) = server_clone.handle_connection(socket).await {
                            tracing::error!("Connection closed with error: {:?}", e);
                        }
                    });
                }
                _ = &mut shutdown_rx => {
                    tracing::info!("Shutdown signal received, stopping playbook server");
                    break;
                }
            }
        }
    });
    shutdown_tx
}



impl PipelineServer {
    async fn handle_connection(&self, socket: PyroSocket) -> std::io::Result<()> {
        loop {
            let view = socket.recv().await?;
            let client_id = view.client_id();

            tracing::debug!(%client_id, "Received playbook request");

            let pipeline_server = self.clone();
            let socket_clone = socket.clone();

            tokio::spawn(async move {
                let response_view = match view.parse_as_error() {
                    Ok(_) => {
                        let py_ref = view.py_ref();
                        match PyroRow::expose_view(py_ref) {
                            Ok(row) => {
                                match pipeline_server
                                    .call_session(client_id, PyroRow::from(&*row))
                                    .await
                                {
                                    Ok(success_row) => success_row
                                        .ship()
                                        .map(|v| v.view())
                                        .unwrap_or_else(|e| e.encode().view()),
                                    Err(e) => e.encode().view(),
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
