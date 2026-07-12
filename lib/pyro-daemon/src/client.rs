use pyroduct::Capture;
use crate::Result;
use std::path::Path;
use pyroduct::transport::socket::PyroSocket;
use pyroduct::format::Bridgeable;

use crate::{DaemonRequest, DaemonResponse};

#[derive(Clone)]
pub struct DaemonClient {
    pub(crate) socket: PyroSocket,
}

impl DaemonClient {
    /// Connect to a running PyroDaemon using the default socket path.
    ///
    /// Resolves the control socket automatically using the same priority chain
    /// as `pyro-daemond` itself:
    ///
    /// 1. `$PYRO_DAEMON_DIR` environment variable
    /// 2. `/var/lib/pyroduct/control` (Linux systemd)
    /// 3. `~/.pyroduct/control` (installed via install.sh / macOS)
    /// 4. `~/Library/Application Support/pyro-daemon/control` (legacy macOS)
    ///
    /// This is the recommended entry point when depending on `pyro-daemon`
    /// from crates.io.
    ///
    /// # Example
    /// ```rust,no_run
    /// use pyro_daemon::client::DaemonClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = DaemonClient::connect_default().await?;
    ///     println!("Connected: {}", client.is_connected());
    ///     Ok(())
    /// }
    /// ```
    pub async fn connect_default() -> Result<Self> {
        let socket_path = crate::PyroDaemon::default_working_dir().join("control");
        Self::connect(socket_path).await
    }

    /// Connect to a running PyroDaemon unix socket
    pub async fn connect(path: impl AsRef<Path>) -> Result<Self> {
        let socket = PyroSocket::connect_unix(path)
            .await
            .capture("Failed to connect to PyroDaemon socket")?;
        Ok(Self { socket })
    }

    /// Returns `true` if the underlying connection is still alive.
    pub fn is_connected(&self) -> bool {
        !self.socket.is_closed()
    }

    /// Connect to a running PyroDaemon TCP socket
    pub async fn connect_tcp(addr: impl tokio::net::ToSocketAddrs) -> Result<Self> {
        let socket = PyroSocket::connect_tcp(addr)
            .await
            .capture("Failed to connect to PyroDaemon TCP socket")?;
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

    /// Receive the next unsolicited response/event from the daemon
    pub async fn recv(&self) -> Result<DaemonResponse> {
        let view = self.socket.recv().await.capture("Failed to receive from daemon socket")?;
        let exposed = DaemonResponse::expose(view).capture("Failed to expose daemon response")?;
        Ok((*exposed).clone())
    }
}
