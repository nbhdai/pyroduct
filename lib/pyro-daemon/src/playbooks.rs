use crate::playbook_worker::PlaybookWorker;
use anyhow::{Context, Result};
use pyro_artifacts::cargo::CapabilityIdent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlaybookStatus {
    pub id: Uuid,
    pub config_path: PathBuf,
    pub socket_path: String,
    pub active_capabilities: Vec<CapabilityIdent>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum PlaybookRequest {
    Start {
        playbook_config_path: PathBuf,
        playbook_socket: String,
    },
    Stop {
        playbook_id: Uuid,
    },
    List,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PlaybookResponse {
    Success {
        message: String,
        playbook_id: Option<Uuid>,
    },
    Playbooks {
        playbooks: Vec<PlaybookStatus>,
    },
    Error {
        message: String,
    },
}

#[derive(Clone)]
pub struct PlaybooksManager {
    workers: Arc<Mutex<HashMap<Uuid, PlaybookWorker>>>,
}

impl PlaybooksManager {
    pub fn new() -> Self {
        Self {
            workers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn handle_request(&self, req: PlaybookRequest) -> PlaybookResponse {
        match req {
            PlaybookRequest::Start {
                playbook_config_path,
                playbook_socket,
            } => {
                let id = Uuid::new_v4();
                match self
                    .start_playbook(id, playbook_config_path, playbook_socket)
                    .await
                {
                    Ok(()) => PlaybookResponse::Success {
                        message: "Playbook worker and capability servers started successfully"
                            .to_string(),
                        playbook_id: Some(id),
                    },
                    Err(e) => PlaybookResponse::Error {
                        message: format!("Failed to start playbook worker: {:?}", e),
                    },
                }
            }
            PlaybookRequest::Stop { playbook_id } => match self.stop_playbook(&playbook_id).await {
                Ok(true) => PlaybookResponse::Success {
                    message: "Playbook worker shut down successfully".to_string(),
                    playbook_id: Some(playbook_id),
                },
                Ok(false) => PlaybookResponse::Error {
                    message: format!("No active playbook worker found with ID: {}", playbook_id),
                },
                Err(e) => PlaybookResponse::Error {
                    message: format!("Error during playbook shutdown: {:?}", e),
                },
            },
            PlaybookRequest::List => {
                let active = self.list_playbooks().await;
                let playbooks = active
                    .into_iter()
                    .map(
                        |(id, config_path, socket_path, active_capabilities)| PlaybookStatus {
                            id,
                            config_path,
                            socket_path,
                            active_capabilities,
                        },
                    )
                    .collect();
                PlaybookResponse::Playbooks { playbooks }
            }
        }
    }

    pub async fn start_playbook(
        &self,
        id: Uuid,
        playbook_config_path: PathBuf,
        playbook_socket: String,
    ) -> Result<()> {
        let config_str = tokio::fs::read_to_string(&playbook_config_path)
            .await
            .context("Failed to read playbook config file")?;

        let pipeline_config: pyroduct::pipeline::factory::PipelineConfig =
            match playbook_config_path.extension().and_then(|s| s.to_str()) {
                Some("toml") => {
                    toml::from_str(&config_str).context("Failed to parse pipeline TOML")?
                }
                Some("yaml") | Some("yml") => {
                    serde_yaml::from_str(&config_str).context("Failed to parse pipeline YAML")?
                }
                Some("json") => {
                    serde_json::from_str(&config_str).context("Failed to parse pipeline JSON")?
                }
                _ => {
                    anyhow::bail!("Unknown playbook config extension; supports toml, yaml and json")
                }
            };

        let worker = PlaybookWorker::start(id, pipeline_config, playbook_socket).await?;
        let mut guard = self.workers.lock().await;
        guard.insert(id, worker);
        Ok(())
    }

    pub async fn stop_playbook(&self, id: &Uuid) -> Result<bool> {
        let mut guard = self.workers.lock().await;
        if let Some(worker) = guard.remove(id) {
            worker.shutdown().await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn list_playbooks(&self) -> Vec<(Uuid, PathBuf, String, Vec<CapabilityIdent>)> {
        let guard = self.workers.lock().await;
        guard
            .values()
            .map(|w| {
                (
                    w.id,
                    PathBuf::new(),
                    w.socket_path.clone(),
                    w.capability_processes
                        .iter()
                        .map(|c| c.cap.clone())
                        .collect(),
                )
            })
            .collect()
    }

    pub async fn active_workers_count(&self) -> usize {
        let guard = self.workers.lock().await;
        guard.len()
    }
}
