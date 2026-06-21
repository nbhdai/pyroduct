//! [`CacheServer`] — serve a [`pyro_artifacts::cache::CacheManager`] over a socket.
//!
//! Binds a Unix or TCP listener and dispatches incoming [`CacheRequest`]s
//! to the underlying [`CacheManager`], streaming back [`CacheResponse`]s.

use std::path::Path;
use std::sync::Arc;

use pyroduct::Capture;
use pyroduct::format::Bridgeable;
use pyroduct::format::format::Wrapper;
use pyroduct::format::header::{PyroHeader, PyroHeaderMut};
use pyroduct::transport::socket::{PyroListener, PyroSocket};
use tokio::fs;

use pyro_artifacts::cache::CacheManager;
use super::types::{CacheRequest, CacheResponse};

use crate::Result;

/// Serves a [`CacheManager`] over a socket using the same framed JSON
/// protocol as the daemon control socket.
///
/// # Example
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use pyro_artifacts::cache::CacheManager;
/// use pyro_daemon::cache::CacheServer;
///
/// let cache = CacheManager::from_env().await?;
/// CacheServer::new(cache)
///     .run_unix("/var/run/pyro-cache.sock")
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct CacheServer {
    cache: Arc<CacheManager>,
}

impl CacheServer {
    /// Create a new server wrapping the given [`CacheManager`].
    pub fn new(cache: CacheManager) -> Self {
        Self { cache: Arc::new(cache) }
    }

    /// Bind a Unix domain socket at `path` and start serving.
    ///
    /// Removes an existing socket file at that path first (same as the daemon).
    pub async fn run_unix(self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if path.exists() {
            fs::remove_file(path)
                .await
                .capture("Failed to remove existing cache socket file")?;
        }
        tracing::info!(socket = %path.display(), "CacheServer binding Unix socket");
        let listener = PyroListener::bind_unix(path)
            .await
            .capture("Failed to bind CacheServer Unix listener")?;
        self.serve(listener).await
    }

    /// Bind a TCP socket at `addr` and start serving.
    pub async fn run_tcp(self, addr: &str) -> Result<()> {
        tracing::info!(address = %addr, "CacheServer binding TCP socket");
        let listener = PyroListener::bind_tcp(addr)
            .await
            .capture("Failed to bind CacheServer TCP listener")?;
        self.serve(listener).await
    }

    async fn serve(self, listener: PyroListener) -> Result<()> {
        tracing::info!("CacheServer listening for connections");
        loop {
            let socket = match listener.accept().await {
                Ok(conn) => conn,
                Err(e) => {
                    tracing::error!("Failed to accept cache connection: {:?}", e);
                    continue;
                }
            };
            let cache = self.cache.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_client(socket, cache).await {
                    tracing::error!("Error handling cache client: {:?}", e);
                }
            });
        }
    }
}

// ── Per-connection handler ────────────────────────────────────────────────────

async fn handle_client(socket: PyroSocket, cache: Arc<CacheManager>) -> Result<()> {
    loop {
        let view = match socket.recv().await {
            Ok(v) => v,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::ConnectionAborted
                    || e.kind() == std::io::ErrorKind::UnexpectedEof
                {
                    break;
                }
                return Err(e).capture("Failed to receive from cache socket");
            }
        };

        let cache = cache.clone();
        let socket = socket.clone();

        tokio::spawn(async move {
            let typed = match CacheRequest::expose(view) {
                Ok(t) => t,
                Err(e) => {
                    let err_resp = CacheResponse::Error {
                        message: format!("Invalid CacheRequest: {}", e),
                    };
                    if let Ok(resp_vec) = err_resp.ship() {
                        let _ = socket.send(resp_vec.into()).await;
                    }
                    return;
                }
            };

            let req = (*typed).clone();
            let mux_id = typed.data().mux_id();

            let response = dispatch(&cache, req).await;

            if let Ok(mut resp_vec) = response.ship() {
                resp_vec.set_mux_id(mux_id);
                if let Err(e) = socket.send(resp_vec.into()).await {
                    tracing::error!("Failed to send CacheResponse for mux_id {}: {:?}", mux_id, e);
                }
            }
        });
    }
    Ok(())
}

// ── Request dispatcher ────────────────────────────────────────────────────────

pub(super) async fn dispatch(cache: &CacheManager, req: CacheRequest) -> CacheResponse {
    match req {
        // ── Status ────────────────────────────────────────────────────────────
        CacheRequest::Status => CacheResponse::Status {
            cache_root: cache.root.to_string_lossy().to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },

        // ── Capabilities ──────────────────────────────────────────────────────
        CacheRequest::ListCapabilities { author, package } => {
            match cache.list_available_capabilities().await {
                Ok(all) => {
                    let items = all
                        .into_iter()
                        .filter(|(a, p, _)| {
                            author.as_deref().map_or(true, |f| a == f)
                                && package.as_deref().map_or(true, |f| p == f)
                        })
                        .collect();
                    CacheResponse::ArtifactList { items }
                }
                Err(e) => CacheResponse::Error {
                    message: format!("Failed to list capabilities: {:?}", e),
                },
            }
        }

        CacheRequest::GetCapabilityInterfaceSpec { author, name, version } => {
            match cache.capability_interface_spec(&author, &name, &version).await {
                Ok(content) => CacheResponse::Text { content },
                Err(e) => CacheResponse::Error {
                    message: format!("Failed to read interface spec: {:?}", e),
                },
            }
        }

        CacheRequest::GetCapabilityConfigSpec { author, name, version } => {
            match cache.capability_config_spec(&author, &name, &version).await {
                Ok(Some(content)) => CacheResponse::Text { content },
                Ok(None) => CacheResponse::Json { value: serde_json::Value::Null },
                Err(e) => CacheResponse::Error {
                    message: format!("Failed to read config spec: {:?}", e),
                },
            }
        }

        // ── Modules / Playbooks ───────────────────────────────────────────────
        CacheRequest::ListModules { author, package } => {
            match cache.list_available_modules().await {
                Ok(all) => {
                    let items = all
                        .into_iter()
                        .filter(|(a, p, _)| {
                            author.as_deref().map_or(true, |f| a == f)
                                && package.as_deref().map_or(true, |f| p == f)
                        })
                        .collect();
                    CacheResponse::ArtifactList { items }
                }
                Err(e) => CacheResponse::Error {
                    message: format!("Failed to list modules: {:?}", e),
                },
            }
        }

        CacheRequest::FindLatestVersion { author, package } => {
            match cache.find_latest_version(&author, &package).await {
                Ok(version) => CacheResponse::Version { version },
                Err(e) => CacheResponse::Error {
                    message: format!("Failed to find latest version: {:?}", e),
                },
            }
        }

        CacheRequest::GetPlaybookSpec { author, name, version } => {
            let path = cache.module_dir(&author, &name, &version).join("spec.json");
            match tokio::fs::read_to_string(&path).await {
                Ok(s) => match serde_json::from_str(&s) {
                    Ok(value) => CacheResponse::Json { value },
                    Err(e) => CacheResponse::Error {
                        message: format!("Failed to parse spec.json: {:?}", e),
                    },
                },
                Err(e) => CacheResponse::Error {
                    message: format!("Failed to read spec.json at {}: {:?}", path.display(), e),
                },
            }
        }

        CacheRequest::GetPlaybookSource { author, name, version } => {
            let path = cache.module_dir(&author, &name, &version).join("source.rs");
            match tokio::fs::read_to_string(&path).await {
                Ok(content) => CacheResponse::Text { content },
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    CacheResponse::Text { content: String::new() }
                }
                Err(e) => CacheResponse::Error {
                    message: format!("Failed to read source.rs: {:?}", e),
                },
            }
        }

        // ── Config ────────────────────────────────────────────────────────────
        CacheRequest::GetPyroductConfig => {
            let config_path = cache.root.join("config.toml");
            match tokio::fs::read_to_string(&config_path).await {
                Ok(content) => match toml::from_str::<serde_json::Value>(&content) {
                    Ok(value) => CacheResponse::Json { value },
                    Err(e) => CacheResponse::Error {
                        message: format!("Failed to parse config.toml: {:?}", e),
                    },
                },
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    // Return sensible defaults when no config file exists yet
                    CacheResponse::Json {
                        value: serde_json::json!({ "author": "anon", "build_slots": 4 }),
                    }
                }
                Err(e) => CacheResponse::Error {
                    message: format!("Failed to read config.toml: {:?}", e),
                },
            }
        }

        // ── Playbook configurations ───────────────────────────────────────────
        CacheRequest::GetPlaybookConfigurations { author, name, version } => {
            match cache.get_named_binary(&author, &name, &version).await {
                Ok(binary) => match serde_json::to_value(&binary.configurations) {
                    Ok(value) => CacheResponse::Json { value },
                    Err(e) => CacheResponse::Error {
                        message: format!("Failed to serialize configurations: {:?}", e),
                    },
                },
                Err(e) => CacheResponse::Error {
                    message: format!("Failed to load playbook binary: {:?}", e),
                },
            }
        }

        // ── Purge ─────────────────────────────────────────────────────────────
        CacheRequest::PurgeCache => {
            match cache.purge().await {
                Ok(()) => CacheResponse::Ok,
                Err(e) => CacheResponse::Error {
                    message: format!("Failed to purge cache: {:?}", e),
                },
            }
        }

        CacheRequest::PurgeCapabilities => {
            match cache.purge_capabilities().await {
                Ok(()) => CacheResponse::Ok,
                Err(e) => CacheResponse::Error {
                    message: format!("Failed to purge capabilities: {:?}", e),
                },
            }
        }

        CacheRequest::PurgeModules => {
            match cache.purge_modules().await {
                Ok(()) => CacheResponse::Ok,
                Err(e) => CacheResponse::Error {
                    message: format!("Failed to purge modules: {:?}", e),
                },
            }
        }
    }
}
