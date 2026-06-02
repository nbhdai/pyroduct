use pyroduct::Capture;
use crate::Result;
use std::path::Path;
use pyroduct::transport::socket::PyroSocket;
use pyroduct::format::Bridgeable;

use crate::{DaemonRequest, DaemonResponse};

pub struct DaemonClient {
    socket: PyroSocket,
}

impl DaemonClient {
    /// Connect to a running PyroDaemon unix socket
    pub async fn connect(path: impl AsRef<Path>) -> Result<Self> {
        let socket = PyroSocket::connect_unix(path)
            .await
            .capture("Failed to connect to PyroDaemon socket")?;
        Ok(Self { socket })
    }

    /// Send a request and wait for a response multiplexed
    pub async fn request(&self, req: DaemonRequest) -> Result<DaemonResponse> {
        let req_vec = req.ship().capture("Failed to ship client request")?;
        
        let resp_view = self.socket
            .request(None, None, None, req_vec.view())
            .await
            .capture("Daemon request failed")?;

        let resp_exposed = DaemonResponse::expose(resp_view)
            .capture("Failed to expose daemon response")?;

        Ok((*resp_exposed).clone())
    }
}
