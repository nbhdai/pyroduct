use crate::Result;
use crate::playbook::PlaybookWorker;
use pyro_artifacts::cache::CacheManager;
use pyro_artifacts::cargo::CapabilityIdent;
use pyroduct::Capture;
use pyroduct::PyroRow;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type ModuleSpec = std::sync::Arc<pyro_artifacts::artifacts::PlaybookSpec>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlaybookStatus {
    pub name: String,
    pub config_path: PathBuf,
    pub socket_path: String,
    pub active_capabilities: Vec<CapabilityIdent>,
    pub spec: ModuleSpec,
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
    Call {
        name: String,
        payload: serde_json::Value,
    },
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
    CallResult { result: serde_json::Value },
    Error { message: String },
}

#[derive(Clone)]
pub struct PlaybooksManager {
    pub working_dir: PathBuf,
    pub workers: Arc<Mutex<HashMap<String, PlaybookWorker>>>,
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

    pub async fn handle_request(self: &Arc<Self>, req: PlaybookRequest) -> PlaybookResponse {
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
                let playbooks = self.list_playbooks().await;
                PlaybookResponse::Playbooks { playbooks }
            }
            PlaybookRequest::Call { name, payload } => {
                match self.call_playbook(&name, payload).await {
                    Ok(result) => PlaybookResponse::CallResult { result },
                    Err(e) => PlaybookResponse::Error {
                        message: format!("Failed to call playbook: {:?}", e),
                    },
                }
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
            PlaybookRequest::AddSocketCallback {
                source,
                socket_path,
            } => match self.add_socket_callback(source, socket_path).await {
                Ok(uuid) => PlaybookResponse::Success {
                    message: format!("Socket callback added successfully with UUID: {}", uuid),
                },
                Err(e) => PlaybookResponse::Error {
                    message: format!("Failed to add Socket callback: {:?}", e),
                },
            },
            PlaybookRequest::AddPlaybookCallback {
                source,
                target_playbook,
            } => match self.add_playbook_callback(source, target_playbook).await {
                Ok(uuid) => PlaybookResponse::Success {
                    message: format!("Playbook callback added successfully with UUID: {}", uuid),
                },
                Err(e) => PlaybookResponse::Error {
                    message: format!("Failed to add Playbook callback: {:?}", e),
                },
            },
            PlaybookRequest::ListCallbacks { source } => match self.list_callbacks(source).await {
                Ok(callbacks) => PlaybookResponse::Callbacks { callbacks },
                Err(e) => PlaybookResponse::Error {
                    message: format!("Failed to list callbacks: {:?}", e),
                },
            },
            PlaybookRequest::DeleteCallback { uuid } => match self.delete_callback(uuid).await {
                Ok(()) => PlaybookResponse::Success {
                    message: "Callback deleted successfully".to_string(),
                },
                Err(e) => PlaybookResponse::Error {
                    message: format!("Failed to delete callback: {:?}", e),
                },
            },
        }
    }

    pub async fn start_playbook(
        self: &Arc<Self>,
        name: String,
        playbook_config_path: PathBuf,
        playbook_socket: Option<String>,
        input_dir_override: Option<PathBuf>,
        output_dir_override: Option<PathBuf>,
    ) -> Result<()> {
        // Playbook working directory: ROOT/playbooks/{playbook_name}/
        let playbook_dir = self.working_dir.join("playbooks").join(&name);

        let db_entry = self.db.get_playbook(&name).await?;
        if db_entry.is_some() || self.workers.lock().await.contains_key(&name) {
            pyroduct::bail!(
                "Name conflict: playbook with name '{}' already exists",
                name
            );
        }

        let config_str = tokio::fs::read_to_string(&playbook_config_path)
            .await
            .capture("Failed to read playbook config file")?;

        let mut pipeline_config: pyroduct::pipeline::factory::PipelineConfig =
            match playbook_config_path.extension().and_then(|s| s.to_str()) {
                Some("toml") => {
                    toml::from_str(&config_str).capture("Failed to parse pipeline TOML")?
                }
                Some("yaml") | Some("yml") => {
                    serde_yaml::from_str(&config_str).capture("Failed to parse pipeline YAML")?
                }
                Some("json") => {
                    serde_json::from_str(&config_str).capture("Failed to parse pipeline JSON")?
                }
                _ => {
                    pyroduct::bail!(
                        "Unknown playbook config extension; supports toml, yaml and json"
                    )
                }
            };

        // Update dirs (supporting custom paths elsewhere on the system if overridden)
        let input_dir = input_dir_override.unwrap_or_else(|| playbook_dir.join("input"));
        let output_dir = output_dir_override.unwrap_or_else(|| playbook_dir.join("output"));
        let log_dir = playbook_dir.join("log");

        // Create these directories
        tokio::fs::create_dir_all(&input_dir)
            .await
            .capture("Failed to create input directory")?;
        tokio::fs::create_dir_all(&output_dir)
            .await
            .capture("Failed to create output directory")?;
        tokio::fs::create_dir_all(&log_dir)
            .await
            .capture("Failed to create log directory")?;

        pipeline_config.input_dir = input_dir;
        pipeline_config.output_dir = output_dir;
        pipeline_config.log_dir = log_dir;

        // Store references to input and output directories if they are not self-contained
        if pipeline_config.input_dir != playbook_dir.join("input") {
            tokio::fs::write(
                playbook_dir.join("input_dir"),
                pipeline_config.input_dir.to_string_lossy().as_bytes(),
            )
            .await
            .capture("Failed to write custom input directory reference")?;
        }
        if pipeline_config.output_dir != playbook_dir.join("output") {
            tokio::fs::write(
                playbook_dir.join("output_dir"),
                pipeline_config.output_dir.to_string_lossy().as_bytes(),
            )
            .await
            .capture("Failed to write custom output directory reference")?;
        }

        // Store PipelineConfig in ROOT/playbooks/{playbook_name}/config.toml
        let new_config_path = playbook_dir.join("config.toml");
        let toml_string = toml::to_string_pretty(&pipeline_config)
            .capture("Failed to serialize modified PipelineConfig to TOML")?;
        tokio::fs::write(&new_config_path, toml_string)
            .await
            .capture("Failed to write config.toml")?;

        // Store playbook socket path persistently if provided
        if let Some(ref socket) = playbook_socket {
            tokio::fs::write(playbook_dir.join("socket_path"), socket.as_bytes())
                .await
                .capture("Failed to write socket_path reference file")?;
        }

        // Save state and config in SQLite database
        self.db
            .save_playbook(
                &name,
                "running",
                &pipeline_config,
                playbook_socket.as_deref(),
            )
            .await?;

        let cache = CacheManager::from_env()
            .await
            .capture("Failed to initialize CacheManager")?;
        let loaded_pipeline = pipeline_config
            .clone()
            .load(&cache)
            .await
            .capture("Failed to load playbook binary")?;

        tracing::info!(spec = ?loaded_pipeline.playbook.binary.spec, "start_playbook: loaded playbook spec");
        let interconnect = self
            .build_interconnect(&loaded_pipeline.playbook.binary.spec)
            .await?;

        let mut worker =
            PlaybookWorker::start(name.clone(), pipeline_config, Some(interconnect)).await?;
        if let Some(ref socket) = playbook_socket {
            worker.listen_socket(socket).await?;
        }
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

    pub async fn resume_playbook(self: &Arc<Self>, name: String) -> Result<()> {
        {
            let guard = self.workers.lock().await;
            if guard.contains_key(&name) {
                pyroduct::bail!("Playbook '{}' is already running", name);
            }
        }

        let db_entry = self.db.get_playbook(&name).await?;
        let (_status, pipeline_config, socket_path) = match db_entry {
            Some(entry) => entry,
            None => pyroduct::bail!("Playbook '{}' does not exist in state store", name),
        };

        let cache = CacheManager::from_env()
            .await
            .capture("Failed to initialize CacheManager")?;
        let loaded_pipeline = pipeline_config
            .clone()
            .load(&cache)
            .await
            .capture("Failed to load playbook binary")?;

        let interconnect = self
            .build_interconnect(&loaded_pipeline.playbook.binary.spec)
            .await?;

        let mut worker =
            PlaybookWorker::start(name.clone(), pipeline_config, Some(interconnect)).await?;
        if let Some(ref socket) = socket_path {
            worker.listen_socket(socket).await?;
        }
        let _ = self.register_callbacks_from_db(&name, &worker).await;
        self.db.update_status(&name, "running").await?;
        
        let mut guard = self.workers.lock().await;
        if guard.contains_key(&name) {
            let _ = worker.shutdown().await;
            pyroduct::bail!("Playbook '{}' was started by another task", name);
        }
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
                .capture("Failed to delete playbook directory")?;
        }
        Ok(())
    }

    pub async fn list_playbooks(&self) -> Vec<PlaybookStatus> {
        let guard = self.workers.lock().await;
        let mut results = Vec::new();
        for worker in guard.values() {
            let config_path = worker
                .config
                .log_dir
                .parent()
                .map(|p| p.join("config.toml"))
                .unwrap_or_default();

            let active_capabilities = worker
                .capability_processes
                .iter()
                .map(|c| c.cap.clone())
                .collect();

            results.push(PlaybookStatus {
                name: worker.name.clone(),
                config_path,
                socket_path: worker
                    .socket_path
                    .clone()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                active_capabilities,
                spec: worker.server.spec(),
            });
        }
        results
    }

    pub async fn active_workers_count(&self) -> usize {
        let guard = self.workers.lock().await;
        guard.len()
    }

    pub async fn resume_active_playbooks(self: &Arc<Self>) -> Result<()> {
        let playbooks = self.db.list_playbooks().await?;
        let mut to_resume: Vec<String> = playbooks
            .into_iter()
            .filter(|(_name, status, _config, _socket)| status == "running")
            .map(|(name, _, _, _)| name)
            .collect();

        let mut attempts = 0;
        let max_attempts = to_resume.len() + 1;

        while !to_resume.is_empty() && attempts < max_attempts {
            let mut failed = Vec::new();
            let mut succeeded_any = false;

            for name in to_resume {
                match self.resume_playbook(name.clone()).await {
                    Ok(()) => {
                        tracing::info!(name, "Successfully resumed running playbook on daemon startup");
                        succeeded_any = true;
                    }
                    Err(e) => {
                        tracing::warn!(
                            name,
                            error = ?e,
                            "Failed to resume playbook, will retry if dependencies are resolved"
                        );
                        failed.push(name);
                    }
                }
            }

            to_resume = failed;
            attempts += 1;

            if !succeeded_any && !to_resume.is_empty() {
                tracing::error!(
                    remaining = ?to_resume,
                    "Circular or unresolved dependencies preventing resumption of remaining playbooks"
                );
                break;
            }
        }

        Ok(())
    }

    pub async fn add_http_callback(&self, source: String, url: String) -> Result<uuid::Uuid> {
        let uuid = uuid::Uuid::new_v4();
        self.db
            .add_callback_mapping(uuid, &source, "http", &url)
            .await?;
        let workers = self.workers.lock().await;
        if let Some(worker) = workers.get(&source) {
            let cb = pyroduct::pipeline::Callback::http(url);
            worker.add_callback(uuid, cb).await?;
        }
        Ok(uuid)
    }

    pub async fn add_socket_callback(
        &self,
        source: String,
        socket_path: String,
    ) -> Result<uuid::Uuid> {
        let uuid = uuid::Uuid::new_v4();
        self.db
            .add_callback_mapping(uuid, &source, "socket", &socket_path)
            .await?;
        let workers = self.workers.lock().await;
        if let Some(worker) = workers.get(&source) {
            let cb = Self::construct_socket_callback(&socket_path).await?;
            worker.add_callback(uuid, cb).await?;
        }
        Ok(uuid)
    }

    pub async fn add_playbook_callback(
        self: &Arc<Self>,
        source: String,
        target_playbook: String,
    ) -> Result<uuid::Uuid> {
        let uuid = uuid::Uuid::new_v4();
        self.db
            .add_callback_mapping(uuid, &source, "playbook", &target_playbook)
            .await?;
        let workers = self.workers.lock().await;
        if let Some(worker) = workers.get(&source) {
            let manager = self.clone();
            let target = target_playbook.clone();
            let cb = pyroduct::pipeline::Callback::function(move |row_index, row| {
                let manager = manager.clone();
                let target = target.clone();
                let row_static = row.to_static();
                Box::pin(async move {
                    let _ = manager.call(&target, row_index, row_static).await;
                })
            });
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
                .map_err(|e| {
                    pyroduct::capture!(
                        "Failed to connect to TCP callback target {}: {:?}",
                        target,
                        e
                    )
                })
        } else {
            let path = std::path::Path::new(target);
            pyroduct::pipeline::Callback::connect_socket_unix(path)
                .await
                .map_err(|e| {
                    pyroduct::capture!(
                        "Failed to connect to UDS callback target {}: {:?}",
                        target,
                        e
                    )
                })
        }
    }

    pub async fn register_callbacks_from_db(
        self: &Arc<Self>,
        source: &str,
        worker: &PlaybookWorker,
    ) -> Result<()> {
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
                    let manager = self.clone();
                    let target_playbook = target.clone();
                    let cb = pyroduct::pipeline::Callback::function(move |row_index, row| {
                        let manager = manager.clone();
                        let target = target_playbook.clone();
                        let row_static = row.to_static();
                        Box::pin(async move {
                            let _ = manager.call(&target, row_index, row_static).await;
                        })
                    });
                    let _ = worker.add_callback(uuid, cb).await;
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub async fn call(
        &self,
        playbook: &str,
        _row_index: usize,
        row: pyroduct::PyroRow<'static>,
    ) -> Result<()> {
        let workers = self.workers.lock().await;
        if let Some(worker) = workers.get(playbook) {
            let (_session_id, _res) = worker.call(row).await?;
        }
        Ok(())
    }

    pub async fn call_playbook(
        &self,
        name: &str,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let (worker, spec) = {
            let workers = self.workers.lock().await;
            let worker = workers
                .get(name)
                .ok_or_else(|| pyroduct::capture!("Playbook '{}' is not running", name))?;
            (worker.server.clone(), worker.server.spec())
        };

        let input_row: PyroRow<'static> = serde_json::from_value(payload)
            .capture("Invalid JSON payload: failed to deserialize into PyroRow")?;

        let repaired_row = input_row
            .project_repair(spec.func.input.fields())
            .capture("Failed to repair input JSON according to module spec")?;

        let (_session_id, res) = worker
            .call(repaired_row)
            .await
            .map_err(|e| pyroduct::capture!("Failed to call playbook: {:?}", e))?;

        let result_val =
            serde_json::to_value(&res).capture("Failed to serialize returned row to JSON")?;
        Ok(result_val)
    }
}
