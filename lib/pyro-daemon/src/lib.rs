use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// =============================================================================
// RPC Message Types
// =============================================================================

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonRequest {
    Playbook(playbook::PlaybookRequest),
    Capability(capability::CapabilityRequest),
    Data(data::DataRequest),
    Status,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonResponse {
    Playbook(playbook::PlaybookResponse),
    Capability(capability::CapabilityResponse),
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
pub mod client;
pub mod data;
pub mod playbook;
pub mod server;
pub mod state;

pub use capability::CapabilityManager;
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
}

impl PyroDaemon {
    pub fn default_working_dir() -> PathBuf {
        // 1. Explicit env var always wins
        if let Ok(dir) = std::env::var("PYRO_DAEMON_DIR") {
            return PathBuf::from(dir);
        }

        // 2. Check the standard systemd service location (Linux)
        let system_dir = PathBuf::from("/var/lib/pyro-daemon");
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

    pub fn new(working_dir: PathBuf) -> Self {
        let control_socket_path = working_dir.join("control");
        let playbooks_manager = std::sync::Arc::new(PlaybooksManager::new(working_dir.clone()));
        let capability_manager = CapabilityManager::new();
        let data_manager =
            DaemonDataManager::new(working_dir.join("data"), playbooks_manager.clone());
        Self {
            working_dir,
            control_socket_path,
            playbooks_manager,
            capability_manager,
            data_manager,
        }
    }
}

pub type Result<T, E = pyroduct::CapturedError> = std::result::Result<T, E>;
