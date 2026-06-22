use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

use pyro_artifacts::cache::CacheManager;

// =============================================================================
// RPC Message Types
// =============================================================================

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonRequest {
    Playbook(playbook::PlaybookRequest),
    Capability(capability::CapabilityRequest),
    Cache(cache::CacheRequest),
    Data(data::DataRequest),
    Status,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonResponse {
    Playbook(playbook::PlaybookResponse),
    Capability(capability::CapabilityResponse),
    Cache(cache::CacheResponse),
    Data(data::DataResponse),
    StatusInfo {
        active_workers: usize,
        version: String,
        running_playbooks: Vec<playbook::PlaybookStatus>,
    },
    Error {
        message: String,
    },
}

pub mod capability;
pub mod cache;
pub mod client;
pub mod data;
pub mod playbook;
pub mod server;
pub mod state;

pub use capability::CapabilityManager;
pub use cache::{CacheRequest, CacheResponse};
pub use data::DaemonDataManager;
pub use state::DbStateStore;

use crate::playbook::PlaybooksManager;

impl pyroduct::format::Bridgeable for DaemonRequest {
    type Format = pyroduct::format::json::Json<DaemonRequest>;
}

impl pyroduct::format::Bridgeable for DaemonResponse {
    type Format = pyroduct::format::json::Json<DaemonResponse>;
}

// =============================================================================
// PyroDaemon Central Controller
// =============================================================================

pub struct PyroDaemon {
    pub working_dir: PathBuf,
    pub control_socket_path: PathBuf,
    pub playbooks_manager: std::sync::Arc<PlaybooksManager>,
    pub capability_manager: CapabilityManager,
    pub data_manager: DaemonDataManager,
    pub cache_manager: Arc<CacheManager>,
    pub bind_tcp: Option<String>,
}

impl PyroDaemon {
    pub fn default_working_dir() -> PathBuf {
        // 1. Explicit env var always wins
        if let Ok(dir) = std::env::var("PYRO_DAEMON_DIR") {
            let path = PathBuf::from(&dir);
            // Canonicalize to resolve relative paths (e.g. "../test" from
            // process-compose) against the current working directory.
            return path.canonicalize().unwrap_or(path);
        }

        // 2. Check the standard systemd service location (Linux)
        let system_dir = PathBuf::from("/var/lib/pyroduct");
        if system_dir.join("control").exists() {
            return system_dir;
        }

        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));

        // 3. Check the user-local ~/.pyroduct location (set up by install.sh)
        let user_dir = home.join(".pyroduct");
        if user_dir.join("control").exists() {
            return user_dir;
        }

        // 4. Check the legacy macOS Application Support location
        let macos_dir = home.join("Library/Application Support/pyro-daemon");
        if macos_dir.join("control").exists() {
            return macos_dir;
        }

        // 5. Fallback to the user-local ~/.pyroduct directory (matches install.sh)
        home.join(".pyroduct")
    }

    pub async fn new(working_dir: PathBuf) -> Self {
        let control_socket_path = working_dir.join("control");
        let playbooks_manager = std::sync::Arc::new(PlaybooksManager::new(working_dir.clone()));
        let capability_manager = CapabilityManager::new();
        let data_manager =
            DaemonDataManager::new(working_dir.join("data"), playbooks_manager.clone());

        // Initialise the cache manager from the daemon's working directory.
        // The working dir doubles as the cache root (it contains config.toml,
        // capabilities/, modules/, etc.).  We read the local config.toml for
        // `author` and `pyroduct` settings rather than calling
        // CacheManager::from_env(), because environment variables like
        // PYRODUCT may not survive the full process chain
        // (process-compose → bacon → cargo → daemon binary).
        let config_path = working_dir.join("config.toml");
        let author = match tokio::fs::read_to_string(&config_path).await {
            Ok(content) => {
                match toml::from_str::<pyro_artifacts::cache::PyroductConfig>(&content) {
                    Ok(cfg) => {
                        tracing::info!(
                            config_path = %config_path.display(),
                            author = %cfg.author,
                            "Loaded pyroduct config from working directory"
                        );
                        cfg.author
                    }
                    Err(e) => {
                        tracing::warn!(
                            config_path = %config_path.display(),
                            error = ?e,
                            "Failed to parse config.toml, using defaults"
                        );
                        "anon".to_string()
                    }
                }
            }
            Err(_) => {
                tracing::warn!(
                    config_path = %config_path.display(),
                    "No config.toml found in working directory, using defaults"
                );
                "anon".to_string()
            }
        };

        let cache_manager = Arc::new(
            CacheManager::new(&working_dir, author)
                .await
                .expect("Failed to create CacheManager from working directory"),
        );
        tracing::info!(cache_root = %cache_manager.root.display(), "CacheManager initialized");

        Self {
            working_dir,
            control_socket_path,
            playbooks_manager,
            capability_manager,
            data_manager,
            cache_manager,
            bind_tcp: None,
        }
    }

    pub fn with_bind_tcp(mut self, bind_tcp: Option<String>) -> Self {
        self.bind_tcp = bind_tcp;
        self
    }
}

pub type Result<T, E = pyroduct::CapturedError> = std::result::Result<T, E>;
