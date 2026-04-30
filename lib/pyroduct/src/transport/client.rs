use std::collections::HashMap;
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
    client_hashes: HashMap<[u8;16], u32>,
    interface: pyro_spec::InterfaceSpec<'static>,
}

impl PyroClient {
    /// Connect to a TCP address (e.g. `"127.0.0.1:9000"`).
    pub async fn connect_tcp(addr: impl tokio::net::ToSocketAddrs) -> Result<Self, PyroError> {
        let socket = PyroSocket::connect_tcp(addr).await.capture("Failed to connect via TCP")
            .map_err(PyroError::local_io)?;
        let interface = PyroClient::fetch_interface(&socket).await?;
        let client = Self {
            socket,
            client_hashes: HashMap::new(),
            interface,
        };
        Ok(client)
    }

    /// Connect to a Unix domain socket path.
    pub async fn connect_unix(path: impl AsRef<Path>) -> Result<Self, PyroError> {
        let socket =  PyroSocket::connect_unix(path).await.capture("Failed to connect via Unix socket")
            .map_err(PyroError::local_io)?;
        let interface = PyroClient::fetch_interface(&socket).await?;
        let client = Self {
            socket,
            client_hashes: HashMap::new(),
            interface,
        };
        Ok(client)
    }

    /// Fetch the capability interface.
    ///
    /// Sends a request with `fn_id = 0`.
    async fn fetch_interface(socket: &PyroSocket) -> Result<InterfaceSpec<'static>, PyroError> {

        let mut req = PyroVec::ok();
        req.set_fn_id(0);

        let resp = socket.request(&req.view()).await.capture("Transport request failed")
            .map_err(PyroError::local_io)?;
        
        InterfaceSpec::parse_wire(resp.view())
    }

    /// Configure a remote class.
    ///
    /// Sends a request with `fn_id = 1` and the provided data.
    pub async fn configure_class(&mut self, class_id: u8, data: PyroView) -> Result<(), PyroError> {
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
    pub async fn register(&mut self, client_data: PyroView) -> Result<u32, PyroError> {
        let mut req = data.clone_to_vec();
        req.set_fn_id(2);

        let resp = self.socket.request(&req.view()).await.capture("Transport request failed")
            .map_err(PyroError::local_io)?;
        
        // The server returns the client_id using .ship(), which means it's a Bridgeable u32.
        // We use u32::expose to get it back.
        let typed = <u32 as crate::format::Bridgeable>::expose(resp)?;
        let id = *typed.inner();
        // Take the hash of the data
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

    /// Call a remote method by its index.
    ///
    /// Maps `method_index` to `fn_id` based on the server's routing logic: `fn_id = method_index + 4`.
    async fn call_method(&self, class_id: u8, method_index: usize, client_data: PyroView, data: PyroView) -> Result<PyroVec, PyroError> {
        let fn_id = (method_index + 4) as u8;
        req.set_class_id(class_id);
        req.set_fn_id(fn_id);
        if let Some(cid) = self.client_id {
            req.set_client_id(cid);
        }

        let resp = self.socket.request(&req).await.capture("Transport request failed")
            .map_err(PyroError::local_io)?;
        resp.parse_as_error()?;
        Ok(resp)
    }

    /// Call a remote method by the class and method names.
    pub async fn call(&self, class_name: &str, method_name: &str, client_data: PyroView, data: PyroView) -> Result<PyroVec, PyroError> {
        let interface = self.interface.as_ref().ok_or_else(|| {
            PyroError::NotFound("Interface not fetched".to_string())
        })?;

        let class_id = interface.classes.iter().position(|c| c.name == class_name)
            .ok_or_else(|| PyroError::NotFound(format!("Class '{}' not found", class_name)))?;

        let class_spec = &interface.classes[class_id];
        let method_index = class_spec.methods.iter().position(|m| m.name == method_name)
            .ok_or_else(|| PyroError::NotFound(format!("Method '{}' not found in class '{}'", method_name, class_name)))?;

        self.call_method(class_id as u8, method_index, data).await
    }

    /// Access the underlying [`PyroSocket`].
    pub fn socket(&self) -> &PyroSocket {
        &self.socket
    }
}
