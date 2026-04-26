//! Multiplexer for concurrent RPC requests over a single [`PyroSocket`].
//!
//! [`PyroMux`] allows multiple tasks to share a single connection, using the
//! `mux_id` field in the [`PyroVec`] header to route responses back to the
//! correct caller.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use dashmap::DashMap;
use tokio::sync::{mpsc, oneshot};

use crate::format::PyroVec;
use crate::format::header::{PyroHeader, PyroHeaderMut};
use crate::transport::socket::PyroSocket;

/// A multiplexer that manages concurrent requests over a single connection.
///
/// It spawns background tasks for reading and writing, and provides a
/// [`request`](Self::request) method that pairs outgoing messages with
/// incoming responses based on their `mux_id`.
pub struct PyroMux {
    tx: mpsc::UnboundedSender<PyroVec>,
    pending: Arc<DashMap<u32, oneshot::Sender<PyroVec>>>,
    next_id: AtomicU32,
}

impl PyroMux {
    /// Create a new [`PyroMux`] from a [`PyroSocket`].
    ///
    /// This spawns two background tasks to handle concurrent I/O.
    pub fn new(socket: PyroSocket) -> Arc<Self> {
        let (rh, wh) = socket.split();
        let pending = Arc::new(DashMap::new());
        let (tx, rx) = mpsc::unbounded_channel();

        let mux = Arc::new(Self {
            tx,
            pending: pending.clone(),
            next_id: AtomicU32::new(1),
        });

        // Background Read Task: receives PyroVecs and dispatches to pending oneshots.
        let pending_read = pending.clone();
        tokio::spawn(async move {
            let mut rh = rh;
            loop {
                match rh.recv().await {
                    Ok(vec) => {
                        let id = vec.mux_id();
                        if let Some((_, sender)) = pending_read.remove(&id) {
                            let _ = sender.send(vec);
                        } else {
                            // Message with unknown mux_id; could be a notification or a bug.
                            // For now we just drop it.
                        }
                    }
                    Err(_) => {
                        // Connection closed or error occurred.
                        // Clear all pending requests with error?
                        pending_read.clear();
                        break;
                    }
                }
            }
        });

        // Background Write Task: receives PyroVecs from the channel and sends them.
        tokio::spawn(async move {
            let mut wh = wh;
            let mut rx = rx;
            while let Some(vec) = rx.recv().await {
                if wh.send(&vec).await.is_err() {
                    break;
                }
            }
        });

        mux
    }

    /// Perform a concurrent RPC request.
    ///
    /// This assigns a unique `mux_id` to the request, sends it, and waits for
    /// a response with the same `mux_id`.
    pub async fn request(&self, mut vec: PyroVec) -> std::io::Result<PyroVec> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        vec.set_mux_id(id);

        let (tx, rx) = oneshot::channel();
        self.pending.insert(id, tx);

        if self.tx.send(vec).is_err() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "multiplexer write task closed",
            ));
        }

        rx.await.map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                "multiplexer read task closed or request timed out",
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::socket::PyroListener;
    use crate::format::header::DataStatus;

    #[tokio::test]
    async fn test_mux_concurrent_requests() {
        let listener = PyroListener::bind_tcp("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr_tcp().unwrap();

        // Echo server that preserves mux_id
        tokio::spawn(async move {
            let mut conn = listener.accept().await.unwrap();
            loop {
                let req = match conn.recv().await {
                    Ok(r) => r,
                    Err(_) => break,
                };
                let mut resp = PyroVec::with_capacity(req.len());
                resp.extend_from_slice(req.as_slice());
                resp.set_mux_id(req.mux_id());
                resp.set_status(DataStatus::Valid);
                if conn.send(&resp).await.is_err() {
                    break;
                }
            }
        });

        let client_socket = PyroSocket::connect_tcp(addr).await.unwrap();
        let mux = PyroMux::new(client_socket);

        let mut handles = Vec::new();
        for i in 0..10 {
            let mux = mux.clone();
            handles.push(tokio::spawn(async move {
                let payload = format!("hello {}", i);
                let mut req = PyroVec::with_capacity(payload.len());
                req.extend_from_slice(payload.as_bytes());
                let resp = mux.request(req).await.unwrap();
                assert_eq!(resp.as_slice(), payload.as_bytes());
            }));
        }

        for h in handles {
            h.await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_mux_out_of_order_responses() {
        let listener = PyroListener::bind_tcp("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr_tcp().unwrap();

        // Server that reverses the order of two responses
        tokio::spawn(async move {
            let mut conn = listener.accept().await.unwrap();
            let req1 = conn.recv().await.unwrap();
            let req2 = conn.recv().await.unwrap();

            // Send response to req2 first
            let mut resp2 = PyroVec::with_capacity(req2.len());
            resp2.extend_from_slice(req2.as_slice());
            resp2.set_mux_id(req2.mux_id());
            conn.send(&resp2).await.unwrap();

            // Then response to req1
            let mut resp1 = PyroVec::with_capacity(req1.len());
            resp1.extend_from_slice(req1.as_slice());
            resp1.set_mux_id(req1.mux_id());
            conn.send(&resp1).await.unwrap();
        });

        let client_socket = PyroSocket::connect_tcp(addr).await.unwrap();
        let mux = PyroMux::new(client_socket);

        let req1_payload = b"first";
        let mut req1 = PyroVec::with_capacity(req1_payload.len());
        req1.extend_from_slice(req1_payload);

        let req2_payload = b"second";
        let mut req2 = PyroVec::with_capacity(req2_payload.len());
        req2.extend_from_slice(req2_payload);

        let (resp1, resp2) = tokio::join!(mux.request(req1), mux.request(req2));

        assert_eq!(resp1.unwrap().as_slice(), req1_payload);
        assert_eq!(resp2.unwrap().as_slice(), req2_payload);
    }
}
