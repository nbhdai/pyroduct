use crate::playbook::PlaybookWorker;
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
        #[serde(default)]
        name: Option<String>,
        playbook_config_path: PathBuf,
        playbook_socket: String,
        #[serde(default)]
        input_dir: Option<PathBuf>,
        #[serde(default)]
        output_dir: Option<PathBuf>,
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
    pub working_dir: PathBuf,
    workers: Arc<Mutex<HashMap<Uuid, PlaybookWorker>>>,
}

impl PlaybooksManager {
    pub fn new(working_dir: PathBuf) -> Self {
        Self {
            working_dir,
            workers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn handle_request(&self, req: PlaybookRequest) -> PlaybookResponse {
        match req {
            PlaybookRequest::Start {
                name,
                playbook_config_path,
                playbook_socket,
                input_dir,
                output_dir,
            } => {
                let id = Uuid::new_v4();
                match self
                    .start_playbook(
                        id,
                        name,
                        playbook_config_path,
                        playbook_socket,
                        input_dir,
                        output_dir,
                    )
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
        name: Option<String>,
        playbook_config_path: PathBuf,
        playbook_socket: String,
        input_dir_override: Option<PathBuf>,
        output_dir_override: Option<PathBuf>,
    ) -> Result<()> {
        let config_str = tokio::fs::read_to_string(&playbook_config_path)
            .await
            .context("Failed to read playbook config file")?;

        let mut pipeline_config: pyroduct::pipeline::factory::PipelineConfig =
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

        // Determine playbook name: use provided name, or fallback to playbook package name
        let playbook_name = name.unwrap_or_else(|| pipeline_config.playbook.package.clone());

        // Playbook working directory: ROOT/playbooks/{playbook_name}/
        let playbook_dir = self.working_dir.join("playbooks").join(&playbook_name);

        if playbook_dir.exists() {
            anyhow::bail!(
                "Name conflict: playbook with name '{}' already exists",
                playbook_name
            );
        }

        // Update dirs (supporting custom paths elsewhere on the system if overridden)
        let input_dir = input_dir_override.unwrap_or_else(|| playbook_dir.join("input"));
        let output_dir = output_dir_override.unwrap_or_else(|| playbook_dir.join("output"));
        let log_dir = playbook_dir.join("log");

        // Create these directories
        tokio::fs::create_dir_all(&input_dir).await?;
        tokio::fs::create_dir_all(&output_dir).await?;
        tokio::fs::create_dir_all(&log_dir).await?;

        pipeline_config.input_dir = input_dir;
        pipeline_config.output_dir = output_dir;
        pipeline_config.log_dir = log_dir;

        // Store references to input and output directories if they are not self-contained
        if pipeline_config.input_dir != playbook_dir.join("input") {
            tokio::fs::write(
                playbook_dir.join("input_dir"),
                pipeline_config.input_dir.to_string_lossy().as_bytes(),
            )
            .await?;
        }
        if pipeline_config.output_dir != playbook_dir.join("output") {
            tokio::fs::write(
                playbook_dir.join("output_dir"),
                pipeline_config.output_dir.to_string_lossy().as_bytes(),
            )
            .await?;
        }

        // Store PipelineConfig in ROOT/playbooks/{playbook_name}/config.toml
        let new_config_path = playbook_dir.join("config.toml");
        let toml_string = toml::to_string_pretty(&pipeline_config)
            .context("Failed to serialize modified PipelineConfig to TOML")?;
        tokio::fs::write(&new_config_path, toml_string).await?;

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
                let config_path = w
                    .config
                    .log_dir
                    .parent()
                    .map(|p| p.join("config.toml"))
                    .unwrap_or_default();
                (
                    w.id,
                    config_path,
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
