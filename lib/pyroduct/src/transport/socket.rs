//! Socket transport for PyroVec — Unix and TCP.
//!
//! Provides [`PyroSocket`], a unified connection abstraction that wraps either a
//! Unix domain socket or a TCP stream. Both transports use the same framing
//! protocol as [`read_from_stream`] / [`write_to_stream`].
//!
//! # Quick start
//!
//! ```rust,ignore
//! // Connect over TCP
//! let mut sock = PyroSocket::connect_tcp("127.0.0.1:9000").await?;
//!
//! // Connect over a Unix domain socket
//! let mut sock = PyroSocket::connect_unix("/tmp/pyro.sock").await?;
//!
//! // Send a PyroVec
//! sock.send(&vec).await?;
//!
//! // Receive a PyroVec
//! let response = sock.recv().await?;
//!
//! // Listen for incoming connections (TCP)
//! let mut listener = PyroListener::bind_tcp("127.0.0.1:9000").await?;
//! while let Some(conn) = listener.accept().await? {
//!     tokio::spawn(async move { handle(conn).await });
//! }
//! ```

use std::path::Path;

use tokio::io::{AsyncWriteExt, BufReader, BufWriter};
use tokio::net::{TcpListener, TcpStream, UnixListener, UnixStream};

use crate::format::{
    PyroVec,
    tokio::{PyroStreamSettings, read_from_stream, write_to_stream},
};

// ── Inner stream ─────────────────────────────────────────────────────────────

enum Inner {
    Tcp(BufReader<TcpStream>),
    Unix(BufReader<UnixStream>),
}

impl Inner {}

// ── PyroSocket ───────────────────────────────────────────────────────────────

/// A bidirectional PyroVec connection over TCP or Unix domain sockets.
///
/// Constructed via [`PyroSocket::connect_tcp`], [`PyroSocket::connect_unix`],
/// or obtained from a [`PyroListener`].
pub struct PyroSocket {
    inner: Inner,
    settings: PyroStreamSettings,
}

/// The reading half of a [`PyroSocket`].
pub struct PyroReadHalf {
    inner: ReadInner,
    settings: PyroStreamSettings,
}

/// The writing half of a [`PyroSocket`].
pub struct PyroWriteHalf {
    inner: WriteInner,
    settings: PyroStreamSettings,
}

enum ReadInner {
    Tcp(BufReader<tokio::net::tcp::OwnedReadHalf>),
    Unix(BufReader<tokio::net::unix::OwnedReadHalf>),
}

enum WriteInner {
    Tcp(BufWriter<tokio::net::tcp::OwnedWriteHalf>),
    Unix(BufWriter<tokio::net::unix::OwnedWriteHalf>),
}

impl PyroSocket {
    // ── Constructors ─────────────────────────────────────────────────────────

    /// Connect to a TCP address (e.g. `"127.0.0.1:9000"`).
    pub async fn connect_tcp(addr: impl tokio::net::ToSocketAddrs) -> std::io::Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        stream.set_nodelay(true)?;
        Ok(Self {
            inner: Inner::Tcp(BufReader::new(stream)),
            settings: PyroStreamSettings::default(),
        })
    }

    /// Connect to a Unix domain socket path.
    pub async fn connect_unix(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let stream = UnixStream::connect(path).await?;
        Ok(Self {
            inner: Inner::Unix(BufReader::new(stream)),
            settings: PyroStreamSettings::default(),
        })
    }

    /// Override the default stream settings (max message size, timeout).
    pub fn with_settings(mut self, settings: PyroStreamSettings) -> Self {
        self.settings = settings;
        self
    }

    // ── Send / Recv ───────────────────────────────────────────────────────────

    /// Write a [`PyroVec`] to the socket.
    pub async fn send(&mut self, vec: &PyroVec) -> std::io::Result<()> {
        match &mut self.inner {
            Inner::Tcp(r) => {
                // get_mut gives the underlying TcpStream through BufReader
                let stream = r.get_mut();
                let mut w = BufWriter::new(stream);
                write_to_stream(&mut w, vec, Some(&self.settings)).await?;
                use tokio::io::AsyncWriteExt;
                w.flush().await
            }
            Inner::Unix(r) => {
                let stream = r.get_mut();
                let mut w = BufWriter::new(stream);
                write_to_stream(&mut w, vec, Some(&self.settings)).await?;
                use tokio::io::AsyncWriteExt;
                w.flush().await
            }
        }
    }

    /// Read the next [`PyroVec`] from the socket.
    pub async fn recv(&mut self) -> std::io::Result<PyroVec> {
        match &mut self.inner {
            Inner::Tcp(r) => read_from_stream(r, Some(&self.settings)).await,
            Inner::Unix(r) => read_from_stream(r, Some(&self.settings)).await,
        }
    }

    /// Perform a single send + recv round-trip.
    pub async fn request(&mut self, vec: &PyroVec) -> std::io::Result<PyroVec> {
        self.send(vec).await?;
        self.recv().await
    }

    /// Split the socket into owned read and write halves.
    pub fn split(self) -> (PyroReadHalf, PyroWriteHalf) {
        match self.inner {
            Inner::Tcp(r) => {
                let stream = r.into_inner();
                let (rh, wh) = stream.into_split();
                (
                    PyroReadHalf {
                        inner: ReadInner::Tcp(BufReader::new(rh)),
                        settings: self.settings,
                    },
                    PyroWriteHalf {
                        inner: WriteInner::Tcp(BufWriter::new(wh)),
                        settings: self.settings,
                    },
                )
            }
            Inner::Unix(r) => {
                let stream = r.into_inner();
                let (rh, wh) = stream.into_split();
                (
                    PyroReadHalf {
                        inner: ReadInner::Unix(BufReader::new(rh)),
                        settings: self.settings,
                    },
                    PyroWriteHalf {
                        inner: WriteInner::Unix(BufWriter::new(wh)),
                        settings: self.settings,
                    },
                )
            }
        }
    }
}

impl PyroReadHalf {
    /// Read the next [`PyroVec`] from the socket.
    pub async fn recv(&mut self) -> std::io::Result<PyroVec> {
        match &mut self.inner {
            ReadInner::Tcp(r) => read_from_stream(r, Some(&self.settings)).await,
            ReadInner::Unix(r) => read_from_stream(r, Some(&self.settings)).await,
        }
    }
}

impl PyroWriteHalf {
    /// Write a [`PyroVec`] to the socket.
    pub async fn send(&mut self, vec: &PyroVec) -> std::io::Result<()> {
        match &mut self.inner {
            WriteInner::Tcp(w) => {
                write_to_stream(w, vec, Some(&self.settings)).await?;
                w.flush().await
            }
            WriteInner::Unix(w) => {
                write_to_stream(w, vec, Some(&self.settings)).await?;
                w.flush().await
            }
        }
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
    ///
    /// The socket file is **not** removed automatically when the listener is
    /// dropped — callers should delete it beforehand if it may already exist.
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
    /// Returns a ready-to-use [`PyroSocket`] inheriting this listener's
    /// settings.
    pub async fn accept(&self) -> std::io::Result<PyroSocket> {
        match &self.inner {
            ListenerInner::Tcp(l) => {
                let (stream, _addr) = l.accept().await?;
                stream.set_nodelay(true)?;
                Ok(PyroSocket {
                    inner: Inner::Tcp(BufReader::new(stream)),
                    settings: self.settings,
                })
            }
            ListenerInner::Unix(l) => {
                let (stream, _addr) = l.accept().await?;
                Ok(PyroSocket {
                    inner: Inner::Unix(BufReader::new(stream)),
                    settings: self.settings,
                })
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

    // ── TCP ──────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_tcp_send_recv() {
        let listener = PyroListener::bind_tcp("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr_tcp().unwrap();

        let server = tokio::spawn(async move {
            let mut conn = listener.accept().await.unwrap();
            conn.recv().await.unwrap()
        });

        let mut client = PyroSocket::connect_tcp(addr).await.unwrap();
        let sent = make_vec(b"hello tcp", DataStatus::Valid);
        client.send(&sent).await.unwrap();

        let received = server.await.unwrap();
        assert_eq!(received.as_slice(), b"hello tcp");
        assert_eq!(received.status(), Ok(DataStatus::Valid));
    }

    #[tokio::test]
    async fn test_tcp_request_response() {
        let listener = PyroListener::bind_tcp("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr_tcp().unwrap();

        tokio::spawn(async move {
            let mut conn = listener.accept().await.unwrap();
            let req = conn.recv().await.unwrap();
            // Echo back with class_id bumped
            let mut resp = PyroVec::with_capacity(req.len());
            resp.extend_from_slice(req.as_slice());
            resp.set_class_id(req.class_id() + 1);
            conn.send(&resp).await.unwrap();
        });

        let mut client = PyroSocket::connect_tcp(addr).await.unwrap();
        let req = make_vec(b"ping", DataStatus::Valid);
        let resp = client.request(&req).await.unwrap();

        assert_eq!(resp.as_slice(), b"ping");
        assert_eq!(resp.class_id(), req.class_id() + 1);
    }

    #[tokio::test]
    async fn test_tcp_multi_message() {
        let listener = PyroListener::bind_tcp("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr_tcp().unwrap();

        let server = tokio::spawn(async move {
            let mut conn = listener.accept().await.unwrap();
            let a = conn.recv().await.unwrap();
            let b = conn.recv().await.unwrap();
            (a, b)
        });

        let mut client = PyroSocket::connect_tcp(addr).await.unwrap();
        client
            .send(&make_vec(b"first", DataStatus::Valid))
            .await
            .unwrap();
        client
            .send(&make_vec(b"second", DataStatus::Error))
            .await
            .unwrap();

        let (a, b) = server.await.unwrap();
        assert_eq!(a.as_slice(), b"first");
        assert_eq!(b.as_slice(), b"second");
        assert_eq!(b.status(), Ok(DataStatus::Error));
    }

    // ── Unix ─────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_unix_send_recv() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pyro_test.sock");

        let listener = PyroListener::bind_unix(&path).await.unwrap();

        let server = tokio::spawn(async move {
            let mut conn = listener.accept().await.unwrap();
            conn.recv().await.unwrap()
        });

        let mut client = PyroSocket::connect_unix(&path).await.unwrap();
        let sent = make_vec(b"hello unix", DataStatus::Valid);
        client.send(&sent).await.unwrap();

        let received = server.await.unwrap();
        assert_eq!(received.as_slice(), b"hello unix");
    }

    #[tokio::test]
    async fn test_unix_roundtrip_preserves_header() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pyro_hdr.sock");

        let listener = PyroListener::bind_unix(&path).await.unwrap();

        tokio::spawn(async move {
            let mut conn = listener.accept().await.unwrap();
            let msg = conn.recv().await.unwrap();
            conn.send(&msg).await.unwrap(); // echo
        });

        let mut client = PyroSocket::connect_unix(&path).await.unwrap();
        let mut original = make_vec(b"header check", DataStatus::Valid);
        original.set_class_id(7);
        original.set_fn_id(3);

        let response = client.request(&original).await.unwrap();
        assert_eq!(response.as_slice(), b"header check");
        assert_eq!(response.class_id(), 7);
        assert_eq!(response.fn_id(), 3);
        assert_eq!(response.status(), Ok(DataStatus::Valid));
    }
}
