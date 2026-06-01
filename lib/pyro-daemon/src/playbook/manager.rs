use crate::playbook::PlaybookWorker;
use anyhow::{Context, Result};
use pyro_artifacts::cargo::CapabilityIdent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlaybookStatus {
    pub name: String,
    pub config_path: PathBuf,
    pub socket_path: String,
    pub active_capabilities: Vec<CapabilityIdent>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CallbackMapping {
    pub uuid: uuid::Uuid,
    pub source: String,
    pub callback_type: String,
    pub target: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum PlaybookRequest {
    Start {
        name: String,
        playbook_config_path: PathBuf,
        #[serde(default)]
        playbook_socket: Option<String>,
        #[serde(default)]
        input_dir: Option<PathBuf>,
        #[serde(default)]
        output_dir: Option<PathBuf>,
    },
    Stop {
        name: String,
    },
    Resume {
        name: String,
    },
    Delete {
        name: String,
    },
    List,
    AddHttpCallback {
        source: String,
        url: String,
    },
    AddSocketCallback {
        source: String,
        socket_path: String,
    },
    AddPlaybookCallback {
        source: String,
        target_playbook: String,
    },
    ListCallbacks {
        source: String,
    },
    DeleteCallback {
        uuid: uuid::Uuid,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PlaybookResponse {
    Success { message: String },
    Playbooks { playbooks: Vec<PlaybookStatus> },
    Callbacks { callbacks: Vec<CallbackMapping> },
    Error { message: String },
}

#[derive(Clone)]
pub struct PlaybooksManager {
    pub working_dir: PathBuf,
    workers: Arc<Mutex<HashMap<String, PlaybookWorker>>>,
    pub db: crate::state::DbStateStore,
}

impl PlaybooksManager {
    pub fn new(working_dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&working_dir);
        let db_path = working_dir.join("state.db");
        let db = crate::state::DbStateStore::open(&db_path)
            .expect("Failed to open playbook state database");
        Self {
            working_dir,
            workers: Arc::new(Mutex::new(HashMap::new())),
            db,
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
                match self
                    .start_playbook(
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
                    },
                    Err(e) => PlaybookResponse::Error {
                        message: format!("Failed to start playbook worker: {:?}", e),
                    },
                }
            }
            PlaybookRequest::Stop { name } => match self.stop_playbook(&name).await {
                Ok(true) => PlaybookResponse::Success {
                    message: "Playbook worker shut down successfully".to_string(),
                },
                Ok(false) => PlaybookResponse::Error {
                    message: format!("No active playbook worker found with ID: {}", name),
                },
                Err(e) => PlaybookResponse::Error {
                    message: format!("Error during playbook shutdown: {:?}", e),
                },
            },
            PlaybookRequest::Resume { name } => match self.resume_playbook(name).await {
                Ok(()) => PlaybookResponse::Success {
                    message: "Playbook worker resumed successfully".to_string(),
                },
                Err(e) => PlaybookResponse::Error {
                    message: format!("Failed to resume playbook: {:?}", e),
                },
            },
            PlaybookRequest::Delete { name } => match self.delete_playbook(name).await {
                Ok(()) => PlaybookResponse::Success {
                    message: "Playbook deleted successfully".to_string(),
                },
                Err(e) => PlaybookResponse::Error {
                    message: format!("Failed to delete playbook: {:?}", e),
                },
            },
            PlaybookRequest::List => {
                let active = self.list_playbooks().await;
                let playbooks = active
                    .into_iter()
                    .map(
                        |(name, config_path, socket_path, active_capabilities)| PlaybookStatus {
                            name,
                            config_path,
                            socket_path,
                            active_capabilities,
                        },
                    )
                    .collect();
                PlaybookResponse::Playbooks { playbooks }
            }
            PlaybookRequest::AddHttpCallback { source, url } => {
                match self.add_http_callback(source, url).await {
                    Ok(uuid) => PlaybookResponse::Success {
                        message: format!("HTTP callback added successfully with UUID: {}", uuid),
                    },
                    Err(e) => PlaybookResponse::Error {
                        message: format!("Failed to add HTTP callback: {:?}", e),
                    },
                }
            }
            PlaybookRequest::AddSocketCallback { source, socket_path } => {
                match self.add_socket_callback(source, socket_path).await {
                    Ok(uuid) => PlaybookResponse::Success {
                        message: format!("Socket callback added successfully with UUID: {}", uuid),
                    },
                    Err(e) => PlaybookResponse::Error {
                        message: format!("Failed to add Socket callback: {:?}", e),
                    },
                }
            }
            PlaybookRequest::AddPlaybookCallback { source, target_playbook } => {
                match self.add_playbook_callback(source, target_playbook).await {
                    Ok(uuid) => PlaybookResponse::Success {
                        message: format!("Playbook callback added successfully with UUID: {}", uuid),
                    },
                    Err(e) => PlaybookResponse::Error {
                        message: format!("Failed to add Playbook callback: {:?}", e),
                    },
                }
            }
            PlaybookRequest::ListCallbacks { source } => {
                match self.list_callbacks(source).await {
                    Ok(callbacks) => PlaybookResponse::Callbacks { callbacks },
                    Err(e) => PlaybookResponse::Error {
                        message: format!("Failed to list callbacks: {:?}", e),
                    },
                }
            }
            PlaybookRequest::DeleteCallback { uuid } => {
                match self.delete_callback(uuid).await {
                    Ok(()) => PlaybookResponse::Success {
                        message: "Callback deleted successfully".to_string(),
                    },
                    Err(e) => PlaybookResponse::Error {
                        message: format!("Failed to delete callback: {:?}", e),
                    },
                }
            }
        }
    }

    pub async fn start_playbook(
        &self,
        name: String,
        playbook_config_path: PathBuf,
        playbook_socket: Option<String>,
        input_dir_override: Option<PathBuf>,
        output_dir_override: Option<PathBuf>,
    ) -> Result<()> {
        // Playbook working directory: ROOT/playbooks/{playbook_name}/
        let playbook_dir = self.working_dir.join("playbooks").join(&name);

        if playbook_dir.exists() {
            anyhow::bail!(
                "Name conflict: playbook with name '{}' already exists",
                name
            );
        }

        let playbook_socket = playbook_socket.unwrap_or_else(|| {
            playbook_dir
                .join("input.sock")
                .to_string_lossy()
                .to_string()
        });

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

        // Store playbook socket path persistently
        tokio::fs::write(
            playbook_dir.join("socket_path"),
            playbook_socket.as_bytes(),
        )
        .await?;

        // Save state and config in SQLite database
        self.db.save_playbook(&name, "running", &pipeline_config, Some(&playbook_socket)).await?;

        let worker = PlaybookWorker::start(name.clone(), pipeline_config, playbook_socket).await?;
        let _ = self.register_callbacks_from_db(&name, &worker).await;
        let mut guard = self.workers.lock().await;
        guard.insert(name, worker);
        Ok(())
    }

    pub async fn stop_playbook(&self, name: &str) -> Result<bool> {
        let mut guard = self.workers.lock().await;
        if let Some(worker) = guard.remove(name) {
            worker.shutdown().await?;
            self.db.update_status(name, "stopped").await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn resume_playbook(&self, name: String) -> Result<()> {
        let mut guard = self.workers.lock().await;
        if guard.contains_key(&name) {
            anyhow::bail!("Playbook '{}' is already running", name);
        }

        let db_entry = self.db.get_playbook(&name).await?;
        let (_status, pipeline_config, socket_path) = match db_entry {
            Some(entry) => entry,
            None => anyhow::bail!("Playbook '{}' does not exist in state store", name),
        };

        let playbook_socket = socket_path.unwrap_or_else(|| {
            self.working_dir
                .join("playbooks")
                .join(&name)
                .join("input.sock")
                .to_string_lossy()
                .to_string()
        });

        let worker = PlaybookWorker::start(name.clone(), pipeline_config, playbook_socket).await?;
        let _ = self.register_callbacks_from_db(&name, &worker).await;
        self.db.update_status(&name, "running").await?;
        guard.insert(name, worker);
        Ok(())
    }

    pub async fn delete_playbook(&self, name: String) -> Result<()> {
        let mut guard = self.workers.lock().await;
        if let Some(worker) = guard.remove(&name) {
            let _ = worker.shutdown().await;
        }

        self.db.delete_playbook(&name).await?;

        let playbook_dir = self.working_dir.join("playbooks").join(&name);
        if playbook_dir.exists() {
            tokio::fs::remove_dir_all(&playbook_dir)
                .await
                .context("Failed to delete playbook directory")?;
        }
        Ok(())
    }

    pub async fn list_playbooks(&self) -> Vec<(String, PathBuf, String, Vec<CapabilityIdent>)> {
        let guard = self.workers.lock().await;
        let db_list = self.db.list_playbooks().await.unwrap_or_default();
        
        db_list
            .into_iter()
            .map(|(name, _status, config, socket_path)| {
                let config_path = config
                    .log_dir
                    .parent()
                    .map(|p| p.join("config.toml"))
                    .unwrap_or_default();
                let socket = socket_path.unwrap_or_default();
                
                // If currently running, get active capabilities
                let active_caps = if let Some(w) = guard.get(&name) {
                    w.capability_processes
                        .iter()
                        .map(|c| c.cap.clone())
                        .collect()
                } else {
                    Vec::new()
                };

                (name, config_path, socket, active_caps)
            })
            .collect()
    }

    pub async fn active_workers_count(&self) -> usize {
        let guard = self.workers.lock().await;
        guard.len()
    }

    pub async fn add_http_callback(&self, source: String, url: String) -> Result<uuid::Uuid> {
        let uuid = uuid::Uuid::new_v4();
        self.db.add_callback_mapping(uuid, &source, "http", &url).await?;
        let workers = self.workers.lock().await;
        if let Some(worker) = workers.get(&source) {
            let cb = pyroduct::pipeline::Callback::http(url);
            worker.add_callback(uuid, cb).await?;
        }
        Ok(uuid)
    }

    pub async fn add_socket_callback(&self, source: String, socket_path: String) -> Result<uuid::Uuid> {
        let uuid = uuid::Uuid::new_v4();
        self.db.add_callback_mapping(uuid, &source, "socket", &socket_path).await?;
        let workers = self.workers.lock().await;
        if let Some(worker) = workers.get(&source) {
            let cb = Self::construct_socket_callback(&socket_path).await?;
            worker.add_callback(uuid, cb).await?;
        }
        Ok(uuid)
    }

    pub async fn add_playbook_callback(&self, source: String, target_playbook: String) -> Result<uuid::Uuid> {
        let uuid = uuid::Uuid::new_v4();
        self.db.add_callback_mapping(uuid, &source, "playbook", &target_playbook).await?;
        let workers = self.workers.lock().await;
        if let Some(worker) = workers.get(&source) {
            let target_socket = match workers.get(&target_playbook) {
                Some(w) => w.socket_path.clone(),
                None => {
                    match self.db.get_playbook(&target_playbook).await? {
                        Some((_status, _config, Some(socket_path))) => socket_path,
                        _ => anyhow::bail!("Target playbook '{}' does not exist or has no socket path configured", target_playbook),
                    }
                }
            };
            let cb = Self::construct_socket_callback(&target_socket).await?;
            worker.add_callback(uuid, cb).await?;
        }
        Ok(uuid)
    }

    pub async fn list_callbacks(&self, source: String) -> Result<Vec<CallbackMapping>> {
        let db_list = self.db.get_callbacks_for_source(&source).await?;
        Ok(db_list
            .into_iter()
            .map(|(uuid, src, cb_type, target)| CallbackMapping {
                uuid,
                source: src,
                callback_type: cb_type,
                target,
            })
            .collect())
    }

    pub async fn delete_callback(&self, uuid: uuid::Uuid) -> Result<()> {
        self.db.delete_callback_mapping(uuid).await?;
        let workers = self.workers.lock().await;
        for worker in workers.values() {
            let _ = worker.delete_callback(uuid).await;
        }
        Ok(())
    }

    async fn construct_socket_callback(target: &str) -> Result<pyroduct::pipeline::Callback> {
        if let Ok(addr) = target.parse::<std::net::SocketAddr>() {
            pyroduct::pipeline::Callback::connect_socket_tcp(addr)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to connect to TCP callback target {}: {:?}", target, e))
        } else {
            let path = std::path::Path::new(target);
            pyroduct::pipeline::Callback::connect_socket_unix(path)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to connect to UDS callback target {}: {:?}", target, e))
        }
    }

    pub async fn register_callbacks_from_db(&self, source: &str, worker: &PlaybookWorker) -> Result<()> {
        let db_list = self.db.get_callbacks_for_source(source).await?;
        for (uuid, _src, cb_type, target) in db_list {
            match cb_type.as_str() {
                "http" => {
                    let cb = pyroduct::pipeline::Callback::http(target);
                    let _ = worker.add_callback(uuid, cb).await;
                }
                "socket" => {
                    if let Ok(cb) = Self::construct_socket_callback(&target).await {
                        let _ = worker.add_callback(uuid, cb).await;
                    }
                }
                "playbook" => {
                    let workers = self.workers.lock().await;
                    let target_socket = match workers.get(&target) {
                        Some(w) => Some(w.socket_path.clone()),
                        None => {
                            match self.db.get_playbook(&target).await {
                                Ok(Some((_status, _config, Some(socket_path)))) => Some(socket_path),
                                _ => None,
                            }
                        }
                    };
                    if let Some(socket_path) = target_socket {
                        if let Ok(cb) = Self::construct_socket_callback(&socket_path).await {
                            let _ = worker.add_callback(uuid, cb).await;
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}
