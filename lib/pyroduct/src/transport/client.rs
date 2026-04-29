use std::path::Path;
use pyro_spec::InterfaceSpec;

use crate::format::header::{PyroData, PyroHeader, PyroHeaderMut};
use crate::format::{PyroVec, PyroView, SpecWire};
use crate::transport::PyroSocket;
use crate::PyroError;
use crate::captured::Capture;

/// A high-level client for communicating with a [`crate::transport::PyroServer`].
///
/// The client manages a multiplexed connection and can optionally track a `client_id`
/// for stateful interactions with remote objects.
pub struct PyroClient {
    socket: PyroSocket,
    client_id: Option<u32>,
    interface: Option<pyro_spec::InterfaceSpec<'static>>,
}

impl PyroClient {
    /// Connect to a TCP address (e.g. `"127.0.0.1:9000"`).
    pub async fn connect_tcp(addr: impl tokio::net::ToSocketAddrs) -> Result<Self, PyroError> {
        let socket = PyroSocket::connect_tcp(addr).await.capture("Failed to connect via TCP")
            .map_err(PyroError::local_io)?;
        Ok(Self {
            socket,
            client_id: None,
            interface: None,
        })
    }

    /// Connect to a Unix domain socket path.
    pub async fn connect_unix(path: impl AsRef<Path>) -> Result<Self, PyroError> {
        let socket = PyroSocket::connect_unix(path).await.capture("Failed to connect via Unix socket")
            .map_err(PyroError::local_io)?;
        Ok(Self {
            socket,
            client_id: None,
            interface: None,
        })
    }

    /// Fetch the capability interface.
    ///
    /// Sends a request with `fn_id = 0`.
    pub async fn fetch_interface(&mut self) -> Result<&InterfaceSpec<'static>, PyroError> {
        if let Some(ref interface) = self.interface {
            return Ok(interface);
        }

        let mut req = PyroVec::ok();
        req.set_fn_id(0);

        let resp = self.socket.request(&req.view()).await.capture("Transport request failed")
            .map_err(PyroError::local_io)?;
        
        let interface = InterfaceSpec::parse_wire(resp.view())?;
        self.interface = Some(interface);
        Ok(self.interface.as_ref().unwrap())
    }

    /// Configure a remote class.
    ///
    /// Sends a request with `fn_id = 1` and the provided data.
    pub async fn configure_class(&mut self, class_id: u8, data: PyroView<'_>) -> Result<(), PyroError> {
        let mut req = data.clone_to_vec();
        req.set_class_id(class_id);
        req.set_fn_id(1);

        let resp = self.socket.request(&req.view()).await.capture("Transport request failed")
            .map_err(PyroError::local_io)?;
        resp.parse_as_error()?;

        if resp.is_ok(){
            Ok(())
        } else {
            Err(PyroError::Header(crate::format::ParseError::UnknownStatus(resp.status_u8())))
        }
    }

    /// Register the client with the server to receive a `client_id`.
    ///
    /// Sends a request with `fn_id = 2`. The server responds with the assigned `client_id`.
    pub async fn register(&mut self, data: PyroView<'_>) -> Result<u32, PyroError> {
        let mut req = data.clone_to_vec();
        req.set_fn_id(2);

        let resp = self.socket.request(&req.view()).await.capture("Transport request failed")
            .map_err(PyroError::local_io)?;
        
        // The server returns the client_id using .ship(), which means it's a Bridgeable u32.
        // We use u32::expose to get it back.
        let typed = <u32 as crate::format::Bridgeable>::expose(resp)?;
        let id = *typed.inner();
        self.client_id = Some(id);
        Ok(id)
    }

    /// Reset a remote class.
    ///
    /// Sends a request with `fn_id = 3`.
    pub async fn reset_class(&mut self, class_id: u8) -> Result<(), PyroError> {
        let mut req = PyroVec::ok();
        req.set_class_id(class_id);
        req.set_fn_id(3);

        let resp = self.socket.request(&req.view()).await.capture("Transport request failed")
            .map_err(PyroError::local_io)?;
        resp.parse_as_error()?;

        if resp.is_ok(){
            Ok(())
        } else {
            Err(PyroError::Header(crate::format::ParseError::UnknownStatus(resp.status_u8())))
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

        let resp = self.socket.request(&req.view()).await.capture("Transport request failed")
            .map_err(PyroError::local_io)?;
        resp.parse_as_error()?;
        Ok(resp)
    }

    /// Call a remote method by its index.
    ///
    /// Maps `method_index` to `fn_id` based on the server's routing logic: `fn_id = method_index + 4`.
    pub async fn call_method(&self, class_id: u8, method_index: usize, data: PyroView<'_>) -> Result<PyroVec, PyroError> {
        let fn_id = (method_index + 4) as u8;
        self.call(class_id, fn_id, data).await
    }

    /// Access the underlying [`PyroSocket`].
    pub fn socket(&self) -> &PyroSocket {
        &self.socket
    }
}
