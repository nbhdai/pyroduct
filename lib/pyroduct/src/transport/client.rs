use std::path::Path;
use crate::format::{PyroVec, PyroView};
use crate::transport::PyroSocket;
use crate::PyroError;

/// A high-level client for communicating with a [`crate::transport::PyroServer`].
///
/// The client manages a multiplexed connection and can optionally track a `client_id`
/// for stateful interactions with remote objects.
pub struct PyroClient {
    socket: PyroSocket,
    client_id: Option<u32>,
}

impl PyroClient {
    /// Connect to a TCP address (e.g. `"127.0.0.1:9000"`).
    pub async fn connect_tcp(addr: impl tokio::net::ToSocketAddrs) -> std::io::Result<Self> {
        let socket = PyroSocket::connect_tcp(addr).await?;
        Ok(Self {
            socket,
            client_id: None,
        })
    }

    /// Connect to a Unix domain socket path.
    pub async fn connect_unix(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let socket = PyroSocket::connect_unix(path).await?;
        Ok(Self {
            socket,
            client_id: None,
        })
    }

    /// Configure a remote class.
    ///
    /// Sends a request with `fn_id = 0` and the provided data.
    pub async fn configure_class(&mut self, class_id: u8, data: PyroView<'_>) -> Result<(), PyroError> {
        let mut req = data.clone_to_vec();
        req.set_class_id(class_id);
        req.set_fn_id(0);

        let resp = self.socket.request(&req.view()).await.map_err(PyroError::Transport)?;
        if resp.status().is_ok() {
            Ok(())
        } else {
            resp.parse_as_error()
        }
    }

    /// Register the client with the server to receive a `client_id`.
    ///
    /// Sends a request with `fn_id = 1`. The server responds with the assigned `client_id`.
    pub async fn register(&mut self, data: PyroView<'_>) -> Result<u32, PyroError> {
        let mut req = data.clone_to_vec();
        req.set_fn_id(1);

        let resp = self.socket.request(&req.view()).await.map_err(PyroError::Transport)?;
        
        // The server returns the client_id using .ship(), which means it's a Bridgeable u32.
        // We use u32::expose to get it back.
        let typed = <u32 as crate::format::Bridgeable>::expose(resp)?;
        let id = *typed.inner();
        self.client_id = Some(id);
        Ok(id)
    }

    /// Reset a remote class.
    ///
    /// Sends a request with `fn_id = 2`.
    pub async fn reset_class(&mut self, class_id: u8) -> Result<(), PyroError> {
        let mut req = PyroVec::ok();
        req.set_class_id(class_id);
        req.set_fn_id(2);

        let resp = self.socket.request(&req.view()).await.map_err(PyroError::Transport)?;
        if resp.status().is_ok() {
            Ok(())
        } else {
            resp.parse_as_error()
        }
    }

    /// Call a remote method.
    ///
    /// `fn_id` should be the identifier for the method (usually > 2).
    pub async fn call(&self, class_id: u8, fn_id: u8, data: PyroView<'_>) -> Result<PyroVec, PyroError> {
        let mut req = data.clone_to_vec();
        req.set_class_id(class_id);
        req.set_fn_id(fn_id);
        if let Some(cid) = self.client_id {
            req.set_client_id(cid);
        }

        let resp = self.socket.request(&req.view()).await.map_err(PyroError::Transport)?;
        if resp.status().is_ok() {
            Ok(resp)
        } else {
            resp.parse_as_error()
        }
    }

    /// Call a remote method by its index.
    ///
    /// Maps `method_index` to `fn_id` based on the server's routing logic: `fn_id = method_index + 2`.
    /// Note: Since `fn_id = 2` is reserved for reset, `method_index` 0 corresponds to `fn_id` 3
    /// if the server logic is `method_index = fn_id - 3`? 
    /// But server logic is `method_index = fn_id - 2`. 
    /// If `fn_id = 3`, `method_index = 1`. 
    /// If `fn_id = 2`, it's reset.
    /// So `method_index` 0 is unreachable?
    ///
    /// For now, we follow `fn_id = method_index + 2` as hinted by the server's `other - 2`, 
    /// while keeping in mind the potential bug in the server.
    pub async fn call_method(&self, class_id: u8, method_index: usize, data: PyroView<'_>) -> Result<PyroVec, PyroError> {
        let fn_id = (method_index + 2) as u8;
        self.call(class_id, fn_id, data).await
    }

    /// Access the underlying [`PyroSocket`].
    pub fn socket(&self) -> &PyroSocket {
        &self.socket
    }
}
