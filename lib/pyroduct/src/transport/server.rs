use std::sync::Arc;

use crate::format::header::{PyroHeader, PyroHeaderMut};
use crate::transport::{PyroListener, PyroRouter, PyroSocket};

/// A server that listens for incoming [`PyroSocket`] connections and routes
/// requests using a [`PyroRouter`].
pub struct PyroServer {
    router: Arc<PyroRouter>,
}

impl PyroServer {
    /// Create a new server with the given router.
    pub fn new(router: PyroRouter) -> Self {
        Self {
            router: Arc::new(router),
        }
    }

    /// Run the server, accepting connections on the provided listener.
    ///
    /// This method runs forever until the listener is closed or an unrecoverable
    /// I/O error occurs.
    pub async fn run(self, listener: PyroListener) -> std::io::Result<()> {
        let server = Arc::new(self);

        loop {
            let socket = listener.accept().await?;
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
            // Receive the next request/notification
            let req = socket.recv().await?;
            let mux_id = req.mux_id();

            let router = self.router.clone();
            let socket_clone = socket.clone();

            // Handle each request concurrently to leverage multiplexing
            tokio::spawn(async move {
                let mut response = match router.handle(req.view()).await {
                    Ok(vec) => vec,
                    Err(e) => e.encode(),
                };

                // Preserve the mux_id so the client can match the response to the request
                response.set_mux_id(mux_id);

                if let Err(e) = socket_clone.send(response.into()).await {
                    tracing::error!("Failed to send response for mux_id {}: {:?}", mux_id, e);
                }
            });
        }
    }
}
