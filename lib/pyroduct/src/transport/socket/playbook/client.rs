use std::fmt;
use std::path::Path;

use crate::PyroError;
use crate::captured::Capture;
use crate::format::header::{PyroData, PyroHeader, PyroHeaderMut};
use crate::format::{Bridgeable, PyroFailure, PyroRow, PyroSuccess};
use crate::transport::socket::PyroSocket;

/// A high-level client for communicating with a [`crate::transport::playbook::PlaybookServer`].
pub struct PlaybookClient {
    socket: PyroSocket,
}

impl PlaybookClient {
    /// Connect to a TCP address (e.g. `"127.0.0.1:9000"`).
    pub async fn connect_tcp(
        addr: impl tokio::net::ToSocketAddrs + fmt::Debug,
    ) -> Result<Self, PyroError> {
        tracing::info!(?addr, "Connecting via TCP");
        let socket = PyroSocket::connect_tcp(addr)
            .await
            .capture("Failed to connect via TCP")
            .map_err(PyroError::local_io)?;
        Ok(Self { socket })
    }

    /// Connect to a Unix domain socket path.
    pub async fn connect_unix(path: impl AsRef<Path> + fmt::Debug) -> Result<Self, PyroError> {
        tracing::info!(?path, "Connecting via Unix socket");
        let socket = PyroSocket::connect_unix(path)
            .await
            .capture("Failed to connect via Unix socket")
            .map_err(PyroError::local_io)?;
        Ok(Self { socket })
    }

    /// Call the remote playbook pipeline with a row.
    pub async fn call(&mut self, row: &PyroRow<'_>) -> Result<PyroSuccess, PyroFailure> {
        let req_vec = row.to_static().ship().map_err(|e| PyroFailure {
            row_index: 0,
            result: Err(e.to_string()),
            logs: crate::format::PyroLogs::empty(),
        })?;

        let mut req = req_vec;
        req.set_fn_id(0);

        let resp = self
            .socket
            .request(None, None, Some(0), req.view())
            .await
            .capture("Transport request failed")
            .map_err(|e| PyroFailure {
                row_index: 0,
                result: Err(e.to_string()),
                logs: crate::format::PyroLogs::empty(),
            })?;

        if resp.is_ok() {
            let row_ref = PyroRow::expose_view(resp.py_ref()).map_err(|e| PyroFailure {
                row_index: 0,
                result: Err(e.to_string()),
                logs: crate::format::PyroLogs::empty(),
            })?;
            Ok(PyroSuccess {
                row_index: 0,
                row: PyroRow::from(&*row_ref).to_static(),
                logs: crate::format::PyroLogs::empty(),
            })
        } else {
            let err_msg = format!("Request failed with status: {}", resp.status_u8());
            tracing::error!("{}", err_msg);
            Err(PyroFailure {
                row_index: 0,
                result: Err(err_msg),
                logs: crate::format::PyroLogs::empty(),
            })
        }
    }

    /// Call the remote playbook pipeline with a row inside a session, encoding session_id as mux_id.
    pub async fn call_session(
        &mut self,
        session_id: u32,
        row: &PyroRow<'_>,
    ) -> Result<PyroSuccess, PyroFailure> {
        let req_vec = row.to_static().ship().map_err(|e| PyroFailure {
            row_index: session_id,
            result: Err(e.to_string()),
            logs: crate::format::PyroLogs::empty(),
        })?;

        let mut req = req_vec;
        req.set_fn_id(0);

        let resp = self
            .socket
            .request(Some(session_id), None, Some(0), req.view())
            .await
            .capture("Transport request failed")
            .map_err(|e| PyroFailure {
                row_index: session_id,
                result: Err(e.to_string()),
                logs: crate::format::PyroLogs::empty(),
            })?;

        if resp.is_ok() {
            let row_ref = PyroRow::expose_view(resp.py_ref()).map_err(|e| PyroFailure {
                row_index: session_id,
                result: Err(e.to_string()),
                logs: crate::format::PyroLogs::empty(),
            })?;
            Ok(PyroSuccess {
                row_index: session_id,
                row: PyroRow::from(&*row_ref).to_static(),
                logs: crate::format::PyroLogs::empty(),
            })
        } else {
            let err_msg = format!("Request failed with status: {}", resp.status_u8());
            tracing::error!("{}", err_msg);
            Err(PyroFailure {
                row_index: session_id,
                result: Err(err_msg),
                logs: crate::format::PyroLogs::empty(),
            })
        }
    }

    /// Access the underlying [`PyroSocket`].
    pub fn socket(&self) -> &PyroSocket {
        &self.socket
    }
}
