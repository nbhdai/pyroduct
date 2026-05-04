use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use dashmap::DashMap;
use tokio::io::{BufReader, BufWriter};
use tokio::net::{TcpListener, TcpStream, UnixListener, UnixStream};
use tokio::sync::{mpsc, oneshot};

use crate::format::PyroVec;
use crate::format::tokio::Request;
use crate::format::{
    PyroView,
    header::PyroHeader,
    tokio::{PyroStreamSettings, read_from_stream, write_to_stream},
};

// ── PyroSocket ───────────────────────────────────────────────────────────────

/// A bidirectional, multiplexed PyroView connection over TCP or Unix domain sockets.
///
/// `PyroSocket` allows multiple tasks to share a single connection concurrently.
/// It automatically manages `mux_id` headers to route responses back to the
/// correct caller when using [`request`](Self::request).
///
/// Constructed via [`PyroSocket::connect_tcp`], [`PyroSocket::connect_unix`],
/// or obtained from a [`PyroListener`].
#[derive(Clone)]
pub struct PyroSocket {
    inner: Arc<SocketInner>,
    unmatched_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<PyroView>>>,
}

struct SocketInner {
    tx: mpsc::UnboundedSender<Request>,
    pending: DashMap<u32, oneshot::Sender<PyroView>>,
    unmatched_tx: mpsc::UnboundedSender<PyroView>,
    next_id: AtomicU32,
    settings: PyroStreamSettings,
}

/// The reading half of a [`PyroSocket`].
pub struct PyroReadHalf {
    socket: PyroSocket,
}

/// The writing half of a [`PyroSocket`].
pub struct PyroWriteHalf {
    socket: PyroSocket,
}

impl PyroSocket {
    // ── Constructors ─────────────────────────────────────────────────────────

    /// Connect to a TCP address (e.g. `"127.0.0.1:9000"`).
    pub async fn connect_tcp(addr: impl tokio::net::ToSocketAddrs) -> std::io::Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        stream.set_nodelay(true)?;
        let settings = PyroStreamSettings::default();
        let (rh, wh) = stream.into_split();
        Ok(Self::new(BufReader::new(rh), BufWriter::new(wh), settings))
    }

    /// Connect to a Unix domain socket path.
    pub async fn connect_unix(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let stream = UnixStream::connect(path).await?;
        let settings = PyroStreamSettings::default();
        let (rh, wh) = stream.into_split();
        Ok(Self::new(BufReader::new(rh), BufWriter::new(wh), settings))
    }

    fn new<R, W>(
        mut reader: BufReader<R>,
        mut writer: BufWriter<W>,
        settings: PyroStreamSettings,
    ) -> Self
    where
        R: tokio::io::AsyncRead + Unpin + Send + 'static,
        W: tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        let (tx, mut rx) = mpsc::unbounded_channel::<Request>();
        let (unmatched_tx, unmatched_rx) = mpsc::unbounded_channel();

        let inner = Arc::new(SocketInner {
            tx,
            pending: DashMap::new(),
            unmatched_tx,
            next_id: AtomicU32::new(1),
            settings,
        });

        // Background Read Task
        let inner_read = inner.clone();
        let settings_read = settings;
        tokio::spawn(async move {
            tracing::debug!("READ TASK: started");
            loop {
                tracing::debug!("READ TASK: waiting for data");
                let mut vec = PyroVec::with_capacity(0);
                match read_from_stream(&mut reader, Some(&settings_read), &mut vec).await {
                    Ok(_) => {
                        let id = vec.mux_id();
                        tracing::debug!("READ TASK: read data, mux_id={}", id);
                        if id != 0 {
                            if let Some((_, sender)) = inner_read.pending.remove(&id) {
                                tracing::debug!("READ TASK: routing to pending mux_id={}", id);
                                let _ = sender.send(vec.view());
                                continue;
                            }
                            tracing::debug!("READ TASK: mux_id={} not in pending, routing to unmatched", id);
                        }
                        // Unmatched or mux_id == 0
                        tracing::debug!("READ TASK: sending to unmatched");
                        if inner_read.unmatched_tx.send(vec.view()).is_err() {
                            tracing::debug!("READ TASK: unmatched send failed");
                            break;
                        }
                    }
                    Err(e) => { tracing::debug!("READ TASK: read error: {:?}", e); break; }
                }
            }
            tracing::debug!("READ TASK: exited");
        });

        // Background Write Task
        let settings_write = settings;
        tokio::spawn(async move {
            while let Some(rec) = rx.recv().await {
                if write_to_stream(&mut writer, &rec, Some(&settings_write))
                    .await
                    .is_err()
                {
                    break;
                }
                if tokio::io::AsyncWriteExt::flush(&mut writer).await.is_err() {
                    break;
                }
            }
        });

        Self {
            inner,
            unmatched_rx: Arc::new(tokio::sync::Mutex::new(unmatched_rx)),
        }
    }

    /// Override the default stream settings (max message size, timeout).
    ///
    /// NOTE: This currently only affects the handle. Background tasks use the settings
    /// provided at creation.
    pub fn with_settings(mut self, settings: PyroStreamSettings) -> Self {
        Arc::get_mut(&mut self.inner).map(|i| i.settings = settings);
        self
    }

    // ── Send / Recv ───────────────────────────────────────────────────────────

    /// Write a [`PyroView`] to the socket.
    ///
    /// This sends the message without waiting for a response.
    pub async fn send(&self, rec: Request) -> std::io::Result<()> {
        self.inner.tx.send(rec).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::BrokenPipe, "socket write task closed")
        })
    }

    /// Read the next unsolicited [`PyroView`] from the socket.
    ///
    /// This returns messages that were not part of a [`request`](Self::request)
    /// flow (e.g. notifications or incoming requests).
    pub async fn recv(&self) -> std::io::Result<PyroView> {
        let mut rx = self.unmatched_rx.lock().await;
        rx.recv().await.ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                "socket read task closed",
            )
        })
    }

    /// Perform a multiplexed RPC request with explicit fields.
    ///
    /// This assigns a unique `mux_id` to the request, sends it, and waits for
    /// a response with the same `mux_id`.
    pub async fn request(
        &self,
        client_id: Option<u32>,
        class_id: Option<u8>,
        fn_id: Option<u8>,
        inner: PyroView,
    ) -> std::io::Result<PyroView> {
        let mux_id = self.inner.next_id.fetch_add(1, Ordering::SeqCst);
        let request = Request {
            client_id,
            class_id,
            fn_id,
            mux_id: Some(mux_id),
            inner,
        };
        let (tx, rx) = oneshot::channel();
        self.inner.pending.insert(mux_id, tx);

        self.inner.tx.send(request).map_err(|_| {
            self.inner.pending.remove(&mux_id);
            std::io::Error::new(std::io::ErrorKind::BrokenPipe, "socket write task closed")
        })?;

        rx.await.map_err(|_| {
            self.inner.pending.remove(&mux_id);
            std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                "socket read task closed or request timed out",
            )
        })
    }

    /// Split the socket into read and write halves.
    ///
    /// Since `PyroSocket` is internally multiplexed and cloneable, these halves
    /// are simply wrappers around the same shared state.
    pub fn split(self) -> (PyroReadHalf, PyroWriteHalf) {
        (
            PyroReadHalf {
                socket: self.clone(),
            },
            PyroWriteHalf {
                socket: self.clone(),
            },
        )
    }
}

impl PyroReadHalf {
    /// Read the next unsolicited [`PyroView`] from the socket.
    pub async fn recv(&mut self) -> std::io::Result<PyroView> {
        self.socket.recv().await
    }
}

impl PyroWriteHalf {
    /// Write a [`PyroView`] to the socket.
    pub async fn send(&mut self, request: Request) -> std::io::Result<()> {
        self.socket.send(request).await
    }
}

// ── PyroListener ─────────────────────────────────────────────────────────────

enum ListenerInner {
    Tcp(TcpListener),
    Unix(UnixListener),
}

/// Accepts incoming [`PyroSocket`] connections on a TCP or Unix socket.
pub struct PyroListener {
    inner: ListenerInner,
    settings: PyroStreamSettings,
}

impl PyroListener {
    /// Bind a TCP listener to `addr`.
    pub async fn bind_tcp(addr: impl tokio::net::ToSocketAddrs) -> std::io::Result<Self> {
        Ok(Self {
            inner: ListenerInner::Tcp(TcpListener::bind(addr).await?),
            settings: PyroStreamSettings::default(),
        })
    }

    /// Bind a Unix domain socket listener to `path`.
    pub async fn bind_unix(path: impl AsRef<Path>) -> std::io::Result<Self> {
        Ok(Self {
            inner: ListenerInner::Unix(UnixListener::bind(path)?),
            settings: PyroStreamSettings::default(),
        })
    }

    /// Override the default stream settings for all accepted connections.
    pub fn with_settings(mut self, settings: PyroStreamSettings) -> Self {
        self.settings = settings;
        self
    }

    /// Accept the next incoming connection.
    ///
    /// Returns a ready-to-use multiplexed [`PyroSocket`].
    pub async fn accept(&self) -> std::io::Result<PyroSocket> {
        match &self.inner {
            ListenerInner::Tcp(l) => {
                let (stream, _addr) = l.accept().await?;
                stream.set_nodelay(true)?;
                let (rh, wh) = stream.into_split();
                Ok(PyroSocket::new(
                    BufReader::new(rh),
                    BufWriter::new(wh),
                    self.settings,
                ))
            }
            ListenerInner::Unix(l) => {
                let (stream, _addr) = l.accept().await?;
                let (rh, wh) = stream.into_split();
                Ok(PyroSocket::new(
                    BufReader::new(rh),
                    BufWriter::new(wh),
                    self.settings,
                ))
            }
        }
    }

    /// Local address the TCP listener is bound to (TCP only).
    pub fn local_addr_tcp(&self) -> std::io::Result<std::net::SocketAddr> {
        match &self.inner {
            ListenerInner::Tcp(l) => l.local_addr(),
            ListenerInner::Unix(_) => Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "not a TCP listener",
            )),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::header::{DataStatus, PyroHeader, PyroHeaderMut};

    fn make_vec(payload: &[u8], status: DataStatus) -> PyroVec {
        let mut v = PyroVec::with_capacity(payload.len());
        v.extend_from_slice(payload);
        v.set_status(status);
        v
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_multiplexing_concurrent_requests() {
        let listener = PyroListener::bind_tcp("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr_tcp().unwrap();
        tracing::debug!("Listener bound to {}", addr);

        // Echo server that preserves mux_id
        tokio::spawn(async move {
            tracing::debug!("Server: waiting for accept");
            let conn = listener.accept().await.unwrap();
            tracing::debug!("Server: accepted connection");
            loop {
                tracing::debug!("Server: reading request");
                let req = match conn.recv().await {
                    Ok(r) => r,
                    Err(e) => { tracing::debug!("Server: recv error: {:?}", e); break; }
                };
                tracing::debug!("Server: read req mux_id={}", req.mux_id());
                let mut resp = PyroVec::with_capacity(req.len());
                resp.extend_from_slice(req.as_slice());
                resp.set_mux_id(req.mux_id());
                resp.set_status(DataStatus::Valid);
                tracing::debug!("Server: sending echo with mux_id={}", resp.mux_id());
                if conn.send(resp.view().into()).await.is_err() {
                    tracing::debug!("Server: send error");
                    break;
                }
                tracing::debug!("Server: echo sent");
            }
            tracing::debug!("Server: loop exited");
        });

        tracing::debug!("Client: connecting");
        let client = PyroSocket::connect_tcp(addr).await.unwrap();
        tracing::debug!("Client: connected");

        let mut handles = Vec::new();
        for i in 0..10 {
            let client = client.clone();
            handles.push(tokio::spawn(async move {
                tracing::debug!("Client task {}: sending request", i);
                let payload = format!("hello {}", i);
                let mut req = PyroVec::with_capacity(payload.len());
                req.extend_from_slice(payload.as_bytes());
                tracing::debug!("Client task {}: waiting for response", i);
                let resp = client.request(None, None, None, req.into()).await.unwrap();
                tracing::debug!("Client task {}: received response", i);
                assert_eq!(resp.as_slice(), payload.as_bytes());
            }));
        }

        tracing::debug!("Client: joining handles");
        for h in handles {
            h.await.unwrap();
        }
        tracing::debug!("Client: all handles joined");
    }

    #[tokio::test]
    async fn test_unsolicited_messages() {
        let listener = PyroListener::bind_tcp("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr_tcp().unwrap();

        tokio::spawn(async move {
            let conn = listener.accept().await.unwrap();
            let msg = make_vec(b"notification", DataStatus::Valid);
            conn.send(msg.into()).await.unwrap();
        });

        let client = PyroSocket::connect_tcp(addr).await.unwrap();
        let received = client.recv().await.unwrap();
        assert_eq!(received.as_slice(), b"notification");
    }
}
