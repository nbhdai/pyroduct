//! [`CacheClient`] — connect to a remote [`super::CacheServer`] over a socket.

use std::path::Path;
use std::sync::OnceLock;

use pyroduct::Capture;
use pyroduct::format::Bridgeable;
use pyroduct::transport::socket::PyroSocket;

use super::types::{CacheRequest, CacheResponse};

use crate::Result;

/// A cloneable, multiplexed client connection to a remote [`super::CacheServer`].
///
/// # Example
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use pyro_daemon::cache::CacheClient;
///
/// let client = CacheClient::connect("/var/run/pyro-cache.sock").await?;
/// let response = client.request(pyro_daemon::cache::CacheRequest::Status).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct CacheClient {
    pub(crate) socket: PyroSocket,
}

impl CacheClient {
    // ── Constructors ─────────────────────────────────────────────────────────

    /// Connect to a cache server listening on a Unix domain socket.
    pub async fn connect(path: impl AsRef<Path>) -> Result<Self> {
        let socket = PyroSocket::connect_unix(path)
            .await
            .capture("Failed to connect to CacheServer Unix socket")?;
        Ok(Self { socket })
    }

    /// Connect to a cache server listening on a TCP address (e.g. `"127.0.0.1:9100"`).
    pub async fn connect_tcp(addr: impl tokio::net::ToSocketAddrs) -> Result<Self> {
        let socket = PyroSocket::connect_tcp(addr)
            .await
            .capture("Failed to connect to CacheServer TCP socket")?;
        Ok(Self { socket })
    }

    /// Returns `true` if the underlying connection is still alive.
    pub fn is_connected(&self) -> bool {
        !self.socket.is_closed()
    }

    // ── RPC ──────────────────────────────────────────────────────────────────

    /// Send a [`CacheRequest`] and wait for the corresponding [`CacheResponse`].
    pub async fn request(&self, req: CacheRequest) -> Result<CacheResponse> {
        let req_vec = req.ship().capture("Failed to serialise CacheRequest")?;

        let resp_view = self
            .socket
            .request(None, None, None, req_vec.view())
            .await
            .capture("CacheServer request failed")?;

        let resp =
            CacheResponse::expose(resp_view).capture("Failed to deserialise CacheResponse")?;

        Ok((*resp).clone())
    }

    // ── High-level helpers ────────────────────────────────────────────────────

    /// List capabilities, optionally filtering by author and/or package name.
    pub async fn list_capabilities(
        &self,
        author: Option<String>,
        package: Option<String>,
    ) -> Result<Vec<(String, String, String)>> {
        match self
            .request(CacheRequest::ListCapabilities { author, package })
            .await?
        {
            CacheResponse::ArtifactList { items } => Ok(items),
            CacheResponse::Error { message } => {
                Err(std::io::Error::new(std::io::ErrorKind::Other, message))
                    .capture("Cache server error")
            }
            other => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("unexpected response: {:?}", other),
            ))
            .capture("Cache server error"),
        }
    }

    /// List playbook modules, optionally filtering by author and/or package name.
    pub async fn list_modules(
        &self,
        author: Option<String>,
        package: Option<String>,
    ) -> Result<Vec<(String, String, String)>> {
        match self
            .request(CacheRequest::ListModules { author, package })
            .await?
        {
            CacheResponse::ArtifactList { items } => Ok(items),
            CacheResponse::Error { message } => {
                Err(std::io::Error::new(std::io::ErrorKind::Other, message))
                    .capture("Cache server error")
            }
            other => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("unexpected response: {:?}", other),
            ))
            .capture("Cache server error"),
        }
    }

    /// Get the raw interface spec JSON string for a capability.
    pub async fn get_capability_interface_spec(
        &self,
        author: String,
        name: String,
        version: String,
    ) -> Result<String> {
        match self
            .request(CacheRequest::GetCapabilityInterfaceSpec {
                author,
                name,
                version,
            })
            .await?
        {
            CacheResponse::Text { content } => Ok(content),
            CacheResponse::Error { message } => {
                Err(std::io::Error::new(std::io::ErrorKind::Other, message))
                    .capture("Cache server error")
            }
            other => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("unexpected response: {:?}", other),
            ))
            .capture("Cache server error"),
        }
    }

    /// Get the optional capability config spec JSON string.
    pub async fn get_capability_config_spec(
        &self,
        author: String,
        name: String,
        version: String,
    ) -> Result<Option<String>> {
        match self
            .request(CacheRequest::GetCapabilityConfigSpec {
                author,
                name,
                version,
            })
            .await?
        {
            CacheResponse::Text { content } => Ok(Some(content)),
            CacheResponse::Json { value } if value.is_null() => Ok(None),
            CacheResponse::Error { message } => {
                Err(std::io::Error::new(std::io::ErrorKind::Other, message))
                    .capture("Cache server error")
            }
            other => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("unexpected response: {:?}", other),
            ))
            .capture("Cache server error"),
        }
    }

    /// Find the latest semver version of a module.
    pub async fn find_latest_version(
        &self,
        author: String,
        package: String,
    ) -> Result<Option<String>> {
        match self
            .request(CacheRequest::FindLatestVersion { author, package })
            .await?
        {
            CacheResponse::Version { version } => Ok(version),
            CacheResponse::Error { message } => {
                Err(std::io::Error::new(std::io::ErrorKind::Other, message))
                    .capture("Cache server error")
            }
            other => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("unexpected response: {:?}", other),
            ))
            .capture("Cache server error"),
        }
    }

    /// Get the spec.json for a playbook module as a JSON value.
    pub async fn get_playbook_spec(
        &self,
        author: String,
        name: String,
        version: String,
    ) -> Result<serde_json::Value> {
        match self
            .request(CacheRequest::GetPlaybookSpec {
                author,
                name,
                version,
            })
            .await?
        {
            CacheResponse::Json { value } => Ok(value),
            CacheResponse::Error { message } => {
                Err(std::io::Error::new(std::io::ErrorKind::Other, message))
                    .capture("Cache server error")
            }
            other => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("unexpected response: {:?}", other),
            ))
            .capture("Cache server error"),
        }
    }

    /// Get the Rust source of a playbook module.
    pub async fn get_playbook_source(
        &self,
        author: String,
        name: String,
        version: String,
    ) -> Result<String> {
        match self
            .request(CacheRequest::GetPlaybookSource {
                author,
                name,
                version,
            })
            .await?
        {
            CacheResponse::Text { content } => Ok(content),
            CacheResponse::Error { message } => {
                Err(std::io::Error::new(std::io::ErrorKind::Other, message))
                    .capture("Cache server error")
            }
            other => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("unexpected response: {:?}", other),
            ))
            .capture("Cache server error"),
        }
    }

    /// Get the pyroduct config.toml content as a JSON value.
    pub async fn get_pyroduct_config(&self) -> Result<serde_json::Value> {
        match self.request(CacheRequest::GetPyroductConfig).await? {
            CacheResponse::Json { value } => Ok(value),
            CacheResponse::Error { message } => {
                Err(std::io::Error::new(std::io::ErrorKind::Other, message))
                    .capture("Cache server error")
            }
            other => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("unexpected response: {:?}", other),
            ))
            .capture("Cache server error"),
        }
    }
}

// ── Cached connection helper (for use from pyro-gui commands) ─────────────────

static CACHED_CACHE_CLIENT: OnceLock<tokio::sync::Mutex<Option<CacheClient>>> = OnceLock::new();

fn client_cache() -> &'static tokio::sync::Mutex<Option<CacheClient>> {
    CACHED_CACHE_CLIENT.get_or_init(|| tokio::sync::Mutex::new(None))
}

/// Connect (or return the cached connection) to the active cache server.
///
/// Falls back to env-based `PYRODUCT` / `~/.pyroduct` paths if no explicit
/// socket path is provided.
pub async fn connect_to_active_cache_server(socket_path: Option<&Path>) -> Result<CacheClient> {
    let mut guard = client_cache().lock().await;

    if let Some(ref client) = *guard {
        if client.is_connected() {
            return Ok(client.clone());
        }
        tracing::debug!("Cached cache-server connection is stale, reconnecting");
    }

    let client = if let Some(path) = socket_path {
        CacheClient::connect(path).await?
    } else {
        let default_path = default_cache_socket_path();
        CacheClient::connect(&default_path).await?
    };

    *guard = Some(client.clone());
    Ok(client)
}

/// Invalidate the cached cache-server connection (e.g. after settings change).
pub async fn invalidate_cached_cache_connection() {
    *client_cache().lock().await = None;
}

fn default_cache_socket_path() -> std::path::PathBuf {
    // Mirrors the fallback logic in CacheManager::from_env
    if let Ok(dir) = std::env::var("PYRODUCT") {
        return std::path::PathBuf::from(dir).join("cache.sock");
    }
    let system_cache = std::path::PathBuf::from("/var/lib/pyro-daemon/cache.sock");
    if system_cache.exists() {
        return system_cache;
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    home.join(".pyroduct").join("cache.sock")
}
