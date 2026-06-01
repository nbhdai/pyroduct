use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

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
    },
    Error {
        message: String,
    },
}

pub mod capability;
pub mod data;
pub mod playbook;

pub use capability::CapabilityManager;
pub use data::DaemonDataManager;

use crate::playbook::PlaybooksManager;

// =============================================================================
// PyroDaemon Central Controller
// =============================================================================

pub struct PyroDaemon {
    pub working_dir: PathBuf,
    pub control_socket_path: PathBuf,
    pub playbooks_manager: PlaybooksManager,
    pub capability_manager: CapabilityManager,
    pub data_manager: DaemonDataManager,
}

impl PyroDaemon {
    pub fn default_working_dir() -> PathBuf {
        std::env::var("PYRO_DAEMON_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME")
                    .or_else(|_| std::env::var("USERPROFILE"))
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| PathBuf::from("."));
                home.join(".pyro-daemon")
            })
    }

    pub fn new(working_dir: PathBuf) -> Self {
        let control_socket_path = working_dir.join("control");
        let playbooks_manager = PlaybooksManager::new(working_dir.clone());
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

    pub async fn run(&self) -> Result<()> {
        fs::create_dir_all(&self.working_dir)
            .await
            .context("Failed to create working directory")?;
        fs::create_dir_all(self.working_dir.join("data"))
            .await
            .context("Failed to create data directory")?;

        if self.control_socket_path.exists() {
            fs::remove_file(&self.control_socket_path)
                .await
                .context("Failed to clean up existing control socket file")?;
        }

        let listener = UnixListener::bind(&self.control_socket_path)
            .context("Failed to bind Unix control listener")?;

        tracing::info!(socket = %self.control_socket_path.display(), "PyroDaemon listing for control commands");

        loop {
            let (socket, _) = match listener.accept().await {
                Ok(conn) => conn,
                Err(e) => {
                    tracing::error!("Failed to accept incoming control connection: {:?}", e);
                    continue;
                }
            };

            let playbooks_clone = self.playbooks_manager.clone();
            let capability_clone = self.capability_manager.clone();
            let data_clone = self.data_manager.clone();
            tokio::spawn(async move {
                if let Err(e) =
                    handle_client(socket, playbooks_clone, capability_clone, data_clone).await
                {
                    tracing::error!("Error handling control client: {:?}", e);
                }
            });
        }
    }
}

async fn handle_client(
    mut socket: UnixStream,
    playbooks_manager: PlaybooksManager,
    capability_manager: CapabilityManager,
    data_manager: DaemonDataManager,
) -> Result<()> {
    let (reader, mut writer) = socket.split();
    let reader = BufReader::new(reader);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        let req: DaemonRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let err_resp = DaemonResponse::Error {
                    message: format!("Invalid JSON request: {}", e),
                };
                let resp_str = serde_json::to_string(&err_resp)? + "\n";
                writer.write_all(resp_str.as_bytes()).await?;
                continue;
            }
        };

        let response = match req {
            DaemonRequest::Playbook(playbook_req) => {
                DaemonResponse::Playbook(playbooks_manager.handle_request(playbook_req).await)
            }
            DaemonRequest::Capability(capability_req) => {
                DaemonResponse::Capability(capability_manager.handle_request(capability_req).await)
            }
            DaemonRequest::Data(data_req) => {
                DaemonResponse::Data(data_manager.handle_request(data_req).await)
            }
            DaemonRequest::Status => {
                let count = playbooks_manager.active_workers_count().await;
                DaemonResponse::StatusInfo {
                    active_workers: count,
                    version: env!("CARGO_PKG_VERSION").to_string(),
                }
            }
        };

        let resp_str = serde_json::to_string(&response)? + "\n";
        writer.write_all(resp_str.as_bytes()).await?;
    }

    Ok(())
}
