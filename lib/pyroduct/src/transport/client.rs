use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;

use pyro_spec::InterfaceSpec;

use crate::PyroError;
use crate::captured::Capture;
use crate::format::header::{PyroData, PyroHeader, PyroHeaderMut};
use crate::format::{PyroVec, PyroView, SpecWire};
use crate::transport::PyroSocket;

/// A high-level client for communicating with a [`crate::transport::PyroServer`].
///
/// The client manages a multiplexed connection and uses SHA-256 hashing of
/// `client_data` to assign and cache `client_id` values for stateful interactions
/// with remote objects.
pub struct PyroClient {
    socket: PyroSocket,
    /// Maps SHA-256 hash of client data -> assigned client_id.
    client_hashes: HashMap<[u8; 32], u32>,
    interface: pyro_spec::InterfaceSpec<'static>,
}

impl PyroClient {
    /// Connect to a TCP address (e.g. `"127.0.0.1:9000"`).
    pub async fn connect_tcp(addr: impl tokio::net::ToSocketAddrs) -> Result<Self, PyroError> {
        let socket = PyroSocket::connect_tcp(addr)
            .await
            .capture("Failed to connect via TCP")
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
        let socket = PyroSocket::connect_unix(path)
            .await
            .capture("Failed to connect via Unix socket")
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

        let resp = socket
            .request(None, None, Some(0), req.view().into())
            .await
            .capture("Transport request failed")
            .map_err(PyroError::local_io)?;

        InterfaceSpec::parse_wire(resp.py_ref())
    }

    /// Configure a remote class.
    ///
    /// Sends a request with `fn_id = 1` and the provided data.
    pub async fn configure_class(&mut self, class_id: u8, data: PyroView) -> Result<(), PyroError> {
        let mut req = data.clone_to_vec();
        req.set_fn_id(1);

        let resp = self
            .socket
            .request(None, Some(class_id), Some(1), req.view().into())
            .await
            .capture("Transport request failed")
            .map_err(PyroError::local_io)?;
        resp.parse_as_error()?;

        if resp.is_ok() {
            Ok(())
        } else {
            Err(PyroError::Header(crate::format::ParseError::UnknownStatus(
                resp.status_u8(),
            )))
        }
    }

    /// Get the client_id for the given `client_data`, registering with the server
    /// if this is a new hash that hasn't been seen before.
    ///
    /// The data is hashed with SHA-256. If the hash is already cached, the stored
    /// client_id is returned. Otherwise, the server is contacted (fn_id = 2) to
    /// receive a new client_id, which is then cached.
    async fn get_or_register_client_id(&mut self, client_data: PyroView) -> Result<u32, PyroError> {
        let hash = Self::hash_data(&client_data);

        // Check cache first
        if let Some(&id) = self.client_hashes.get(&hash) {
            return Ok(id);
        }

        // Register with server to get a new client_id
        let mut req = PyroVec::ok();
        req.set_fn_id(2);
        // Send the raw client_data in the payload
        req.extend_from_slice(client_data.as_slice());

        let resp = self
            .socket
            .request(None, None, Some(2), req.view().into())
            .await
            .capture("Transport request failed")
            .map_err(PyroError::local_io)?;

        // The server returns the client_id using .ship(), which means it's a Bridgeable u32.
        let typed = <u32 as crate::format::Bridgeable>::expose(resp)?;
        let id = typed.to_native();

        self.client_hashes.insert(hash, id);
        Ok(id)
    }

    /// Hash `client_data` using SHA-256.
    fn hash_data(data: &PyroView) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(data.as_slice());
        hasher.finalize().into()
    }

    /// Reset a remote class.
    ///
    /// Sends a request with `fn_id = 3`.
    pub async fn reset_class(&mut self, class_id: u8) -> Result<(), PyroError> {
        let mut req = PyroVec::ok();
        req.set_fn_id(3);

        let resp = self
            .socket
            .request(None, Some(class_id), Some(3), req.view().into())
            .await
            .capture("Transport request failed")
            .map_err(PyroError::local_io)?;
        resp.parse_as_error()?;

        if resp.is_ok() {
            Ok(())
        } else {
            Err(PyroError::Header(crate::format::ParseError::UnknownStatus(
                resp.status_u8(),
            )))
        }
    }

    /// Call a remote method by its index.
    ///
    /// Maps `method_index` to `fn_id` based on the server's routing logic: `fn_id = method_index + 4`.
    async fn call_method(
        &self,
        class_id: u8,
        method_index: usize,
        client_id: u32,
        data: PyroView,
    ) -> Result<PyroView, PyroError> {
        let fn_id = (method_index + 4) as u8;
        let mut req = data.clone_to_vec();
        req.set_fn_id(fn_id);

        let resp = self
            .socket
            .request(
                Some(client_id),
                Some(class_id),
                Some(fn_id),
                req.view().into(),
            )
            .await
            .capture("Transport request failed")
            .map_err(PyroError::local_io)?;
        resp.parse_as_error()?;
        Ok(resp)
    }

    /// Call a remote method by the class and method names.
    ///
    /// The `client_data` is hashed with SHA-256 to determine which client_id is used.
    /// This must be called with `&mut self` since it may register a new client_id
    /// if the data hasn't been seen before.
    pub async fn call(
        &mut self,
        class_name: &str,
        method_name: &str,
        client_data: PyroView,
        data: PyroView,
    ) -> Result<PyroView, PyroError> {
        let class_id = self
            .interface
            .classes
            .iter()
            .position(|c| c.name == class_name)
            .ok_or_else(|| PyroError::NotFound(format!("Class '{}' not found", class_name)))?;

        let class_spec = &self.interface.classes[class_id];
        let method_index = class_spec
            .methods
            .iter()
            .position(|m| m.name == method_name)
            .ok_or_else(|| {
                PyroError::NotFound(format!(
                    "Method '{}' not found in class '{}'",
                    method_name, class_name
                ))
            })?;

        // Get or register client_id based on client_data hash
        let client_id = self.get_or_register_client_id(client_data).await?;

        self.call_method(class_id as u8, method_index, client_id, data)
            .await
    }

    /// Access the underlying [`PyroSocket`].
    pub fn socket(&self) -> &PyroSocket {
        &self.socket
    }
}
