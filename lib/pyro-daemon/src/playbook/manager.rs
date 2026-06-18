use crate::Result;
use crate::playbook::PlaybookWorker;
use pyro_artifacts::artifacts::PlaybookIdent;
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
    pub http_address: Option<String>,
    pub active_capabilities: Vec<CapabilityIdent>,
    pub local_capabilities: Vec<CapabilityIdent>,
    pub remote_capabilities: Vec<CapabilityIdent>,
    pub spec: ModuleSpec,
    pub processed_rows: usize,
    #[serde(default)]
    pub pinned_version: Option<String>,
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
        pipeline_config: PlaybookIdent,
        #[serde(default)]
        playbook_socket: Option<String>,
        #[serde(default)]
        http_address: Option<String>,
        #[serde(default)]
        input_dir: Option<PathBuf>,
        #[serde(default)]
        output_dir: Option<PathBuf>,
        #[serde(default)]
        pinned_version: Option<String>,
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
        #[serde(default)]
        session_id: Option<u32>,
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
    ListSessions {
        name: String,
        status: Option<pyroduct::pipeline::session::SessionStatusFilter>,
    },
    BulkCall {
        name: String,
        file_name: String,
        file_content: Vec<u8>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SessionInfo {
    pub session_id: u32,
    pub status: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PlaybookResponse {
    Success {
        message: String,
    },
    Playbooks {
        playbooks: Vec<PlaybookStatus>,
    },
    Callbacks {
        callbacks: Vec<CallbackMapping>,
    },
    CallResult {
        result: pyroduct::pipeline::ServerExecutionRecord,
    },
    BulkCallResult {
        results: Vec<pyroduct::pipeline::ServerExecutionRecord>,
    },
    Sessions {
        sessions: Vec<SessionInfo>,
    },
    Error {
        message: String,
    },
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
                pipeline_config,
                playbook_socket,
                http_address,
                input_dir,
                output_dir,
                pinned_version,
            } => {
                tracing::info!(playbook = %name, playbook = ?pipeline_config, "Received Start request for playbook");
                match self
                    .start_playbook(
                        name.clone(),
                        pipeline_config,
                        playbook_socket,
                        http_address,
                        input_dir,
                        output_dir,
                        pinned_version,
                    )
                    .await
                {
                    Ok(()) => {
                        tracing::info!(playbook = %name, "Playbook started successfully");
                        PlaybookResponse::Success {
                            message: "Playbook worker and capability servers started successfully"
                                .to_string(),
                        }
                    }
                    Err(e) => {
                        tracing::error!(playbook = %name, error = ?e, "Failed to start playbook");
                        PlaybookResponse::Error {
                            message: format!("Failed to start playbook worker: {:?}", e),
                        }
                    }
                }
            }
            PlaybookRequest::Stop { name } => {
                tracing::info!(playbook = %name, "Received Stop request for playbook");
                match self.stop_playbook(&name).await {
                    Ok(true) => {
                        tracing::info!(playbook = %name, "Playbook worker shut down successfully");
                        PlaybookResponse::Success {
                            message: "Playbook worker shut down successfully".to_string(),
                        }
                    }
                    Ok(false) => {
                        tracing::warn!(playbook = %name, "No active playbook worker found to stop");
                        PlaybookResponse::Error {
                            message: format!("No active playbook worker found with ID: {}", name),
                        }
                    }
                    Err(e) => {
                        tracing::error!(playbook = %name, error = ?e, "Error during playbook shutdown");
                        PlaybookResponse::Error {
                            message: format!("Error during playbook shutdown: {:?}", e),
                        }
                    }
                }
            }
            PlaybookRequest::Resume { name } => {
                tracing::info!(playbook = %name, "Received Resume request for playbook");
                match self.resume_playbook(name.clone()).await {
                    Ok(()) => {
                        tracing::info!(playbook = %name, "Playbook worker resumed successfully");
                        PlaybookResponse::Success {
                            message: "Playbook worker resumed successfully".to_string(),
                        }
                    }
                    Err(e) => {
                        tracing::error!(playbook = %name, error = ?e, "Failed to resume playbook");
                        PlaybookResponse::Error {
                            message: format!("Failed to resume playbook: {:?}", e),
                        }
                    }
                }
            }
            PlaybookRequest::Delete { name } => {
                tracing::info!(playbook = %name, "Received Delete request for playbook");
                match self.delete_playbook(name.clone()).await {
                    Ok(()) => {
                        tracing::info!(playbook = %name, "Playbook deleted successfully");
                        PlaybookResponse::Success {
                            message: "Playbook deleted successfully".to_string(),
                        }
                    }
                    Err(e) => {
                        tracing::error!(playbook = %name, error = ?e, "Failed to delete playbook");
                        PlaybookResponse::Error {
                            message: format!("Failed to delete playbook: {:?}", e),
                        }
                    }
                }
            }
            PlaybookRequest::List => {
                tracing::info!("Received List playbooks request");
                let playbooks = self.list_playbooks().await;
                tracing::debug!(count = playbooks.len(), "Retrieved active playbooks list");
                PlaybookResponse::Playbooks { playbooks }
            }
            PlaybookRequest::Call {
                name,
                payload,
                session_id,
            } => {
                tracing::info!(playbook = %name, "Received Call request for playbook");
                match self.call_playbook_record(&name, payload, session_id).await {
                    Ok(result) => {
                        tracing::info!(playbook = %name, "Playbook call completed successfully");
                        PlaybookResponse::CallResult { result }
                    }
                    Err(e) => {
                        tracing::error!(playbook = %name, error = ?e, "Failed to call playbook");
                        PlaybookResponse::Error {
                            message: format!("Failed to call playbook: {:?}", e),
                        }
                    }
                }
            }
            PlaybookRequest::BulkCall {
                name,
                file_name,
                file_content,
            } => {
                tracing::info!(playbook = %name, file = %file_name, "Received BulkCall request for playbook");
                match self.call_playbook_bulk(&name, &file_name, file_content).await {
                    Ok(results) => {
                        tracing::info!(playbook = %name, "Playbook bulk call completed successfully");
                        PlaybookResponse::BulkCallResult { results }
                    }
                    Err(e) => {
                        tracing::error!(playbook = %name, error = ?e, "Failed to execute bulk playbook call");
                        PlaybookResponse::Error {
                            message: format!("Failed to run bulk call: {:?}", e),
                        }
                    }
                }
            }
            PlaybookRequest::AddHttpCallback { source, url } => {
                tracing::info!(playbook = %source, url = %url, "Received AddHttpCallback request");
                match self.add_http_callback(source.clone(), url).await {
                    Ok(uuid) => {
                        tracing::info!(playbook = %source, uuid = %uuid, "HTTP callback added successfully");
                        PlaybookResponse::Success {
                            message: format!(
                                "HTTP callback added successfully with UUID: {}",
                                uuid
                            ),
                        }
                    }
                    Err(e) => {
                        tracing::error!(playbook = %source, error = ?e, "Failed to add HTTP callback");
                        PlaybookResponse::Error {
                            message: format!("Failed to add HTTP callback: {:?}", e),
                        }
                    }
                }
            }
            PlaybookRequest::AddSocketCallback {
                source,
                socket_path,
            } => {
                tracing::info!(playbook = %source, socket_path = %socket_path, "Received AddSocketCallback request");
                match self.add_socket_callback(source.clone(), socket_path).await {
                    Ok(uuid) => {
                        tracing::info!(playbook = %source, uuid = %uuid, "Socket callback added successfully");
                        PlaybookResponse::Success {
                            message: format!(
                                "Socket callback added successfully with UUID: {}",
                                uuid
                            ),
                        }
                    }
                    Err(e) => {
                        tracing::error!(playbook = %source, error = ?e, "Failed to add Socket callback");
                        PlaybookResponse::Error {
                            message: format!("Failed to add Socket callback: {:?}", e),
                        }
                    }
                }
            }
            PlaybookRequest::AddPlaybookCallback {
                source,
                target_playbook,
            } => {
                tracing::info!(playbook = %source, target = %target_playbook, "Received AddPlaybookCallback request");
                match self
                    .add_playbook_callback(source.clone(), target_playbook)
                    .await
                {
                    Ok(uuid) => {
                        tracing::info!(playbook = %source, uuid = %uuid, "Playbook callback added successfully");
                        PlaybookResponse::Success {
                            message: format!(
                                "Playbook callback added successfully with UUID: {}",
                                uuid
                            ),
                        }
                    }
                    Err(e) => {
                        tracing::error!(playbook = %source, error = ?e, "Failed to add Playbook callback");
                        PlaybookResponse::Error {
                            message: format!("Failed to add Playbook callback: {:?}", e),
                        }
                    }
                }
            }
            PlaybookRequest::ListCallbacks { source } => {
                tracing::info!(playbook = %source, "Received ListCallbacks request");
                match self.list_callbacks(source.clone()).await {
                    Ok(callbacks) => {
                        tracing::debug!(playbook = %source, count = callbacks.len(), "Retrieved callbacks list");
                        PlaybookResponse::Callbacks { callbacks }
                    }
                    Err(e) => {
                        tracing::error!(playbook = %source, error = ?e, "Failed to list callbacks");
                        PlaybookResponse::Error {
                            message: format!("Failed to list callbacks: {:?}", e),
                        }
                    }
                }
            }
            PlaybookRequest::DeleteCallback { uuid } => {
                tracing::info!(uuid = %uuid, "Received DeleteCallback request");
                match self.delete_callback(uuid).await {
                    Ok(()) => {
                        tracing::info!(uuid = %uuid, "Callback deleted successfully");
                        PlaybookResponse::Success {
                            message: "Callback deleted successfully".to_string(),
                        }
                    }
                    Err(e) => {
                        tracing::error!(uuid = %uuid, error = ?e, "Failed to delete callback");
                        PlaybookResponse::Error {
                            message: format!("Failed to delete callback: {:?}", e),
                        }
                    }
                }
            }
            PlaybookRequest::ListSessions { name, status } => {
                tracing::info!(playbook = %name, status = ?status, "Received ListSessions request");
                match self.list_sessions_for_playbook(&name, status).await {
                    Ok(sessions) => PlaybookResponse::Sessions { sessions },
                    Err(e) => PlaybookResponse::Error {
                        message: format!("Failed to list sessions: {:?}", e),
                    },
                }
            }
        }
    }

    pub async fn start_playbook(
        self: &Arc<Self>,
        name: String,
        pipeline_config: PlaybookIdent,
        playbook_socket: Option<String>,
        http_address: Option<String>,
        input_dir_override: Option<PathBuf>,
        output_dir_override: Option<PathBuf>,
        pinned_version: Option<String>,
    ) -> Result<()> {
        if self.workers.lock().await.contains_key(&name) {
            tracing::warn!(playbook = %name, "Name conflict detected: playbook is already running");
            pyroduct::bail!(
                "Name conflict: playbook with name '{}' is already running",
                name
            );
        }

        let db_entry = self.db.get_playbook(&name).await?;
        if db_entry.is_some() {
            tracing::info!(playbook = %name, "Playbook exists in DB but is not running. Cleaning up before restart.");
            self.delete_playbook(name.clone()).await?;
        }
        let playbook_dir = self.working_dir.join("playbooks").join(&name);
        tracing::debug!(playbook = %name, playbook_dir = ?playbook_dir, "Starting playbook workflow - checking name conflict");

        // Update dirs (supporting custom paths elsewhere on the system if overridden)
        let input_dir = input_dir_override.unwrap_or_else(|| playbook_dir.join("input"));
        let output_dir = output_dir_override.unwrap_or_else(|| playbook_dir.join("output"));
        let log_dir = playbook_dir.join("log");

        tracing::debug!(playbook = %name, ?input_dir, ?output_dir, ?log_dir, "Creating playbook directories");
        tokio::fs::create_dir_all(&input_dir)
            .await
            .capture("Failed to create input directory")?;
        tokio::fs::create_dir_all(&output_dir)
            .await
            .capture("Failed to create output directory")?;
        tokio::fs::create_dir_all(&log_dir)
            .await
            .capture("Failed to create log directory")?;

        let pipeline_config = pyroduct::pipeline::factory::PipelineConfig {
            playbook: pipeline_config,
            remote: HashMap::new(),
            wal_capacity: 1000,
            success_log_retention_secs: 3600,
            error_log_retention_secs: 86400 * 7,
            num_workers: 4,
            log_dir,
            input_dir,
            output_dir,
        };

        // Store references to input and output directories if they are not self-contained
        if pipeline_config.input_dir != playbook_dir.join("input") {
            tracing::debug!(playbook = %name, custom_input_dir = ?pipeline_config.input_dir, "Writing custom input directory reference");
            tokio::fs::write(
                playbook_dir.join("input_dir"),
                pipeline_config.input_dir.to_string_lossy().as_bytes(),
            )
            .await
            .capture("Failed to write custom input directory reference")?;
        }
        if pipeline_config.output_dir != playbook_dir.join("output") {
            tracing::debug!(playbook = %name, custom_output_dir = ?pipeline_config.output_dir, "Writing custom output directory reference");
            tokio::fs::write(
                playbook_dir.join("output_dir"),
                pipeline_config.output_dir.to_string_lossy().as_bytes(),
            )
            .await
            .capture("Failed to write custom output directory reference")?;
        }

        // Store PipelineConfig in ROOT/playbooks/{playbook_name}/config.toml
        let new_config_path = playbook_dir.join("config.toml");
        tracing::debug!(playbook = %name, config_path = ?new_config_path, "Writing PipelineConfig");
        let toml_string = toml::to_string_pretty(&pipeline_config)
            .capture("Failed to serialize modified PipelineConfig to TOML")?;
        tokio::fs::write(&new_config_path, toml_string)
            .await
            .capture("Failed to write config.toml")?;

        // Store playbook socket path persistently if provided
        if let Some(ref socket) = playbook_socket {
            tracing::debug!(playbook = %name, socket_path = %socket, "Writing socket_path reference");
            tokio::fs::write(playbook_dir.join("socket_path"), socket.as_bytes())
                .await
                .capture("Failed to write socket_path reference file")?;
        }

        tracing::debug!(playbook = %name, "Loading cache manager and playbook binary");
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
            .build_interconnect(&name, &loaded_pipeline.playbook.binary.spec)
            .await?;

        tracing::info!(playbook = %name, "Starting PlaybookWorker process");
        let mut worker =
            PlaybookWorker::start(name.clone(), pipeline_config.clone(), Some(interconnect))
                .await?;
        if let Some(ref socket) = playbook_socket {
            tracing::debug!(playbook = %name, socket = %socket, "Worker listening to custom socket");
            worker.listen_socket(socket).await?;
        }
        if let Some(ref addr) = http_address {
            tracing::debug!(playbook = %name, http_address = %addr, "Worker starting HTTP server");
            worker.listen_http(addr).await?;
        }
        let _ = self.register_callbacks_from_db(&name, &worker).await;

        // Save state and config in SQLite database
        tracing::debug!(playbook = %name, "Saving state to database");
        self.db
            .save_playbook(
                &name,
                "running",
                &pipeline_config,
                playbook_socket.as_deref(),
                pinned_version.as_deref(),
                http_address.as_deref(),
            )
            .await?;

        let mut guard = self.workers.lock().await;
        guard.insert(name, worker);
        Ok(())
    }

    pub async fn stop_playbook(&self, name: &str) -> Result<bool> {
        tracing::debug!(playbook = %name, "Stopping playbook");
        let mut guard = self.workers.lock().await;
        if let Some(worker) = guard.remove(name) {
            tracing::debug!(playbook = %name, "Shutting down worker");
            worker.shutdown().await?;
            tracing::debug!(playbook = %name, "Updating status to stopped in DB");
            self.db.update_status(name, "stopped").await?;
            Ok(true)
        } else {
            tracing::warn!(playbook = %name, "No active worker found to stop");
            Ok(false)
        }
    }

    pub async fn resume_playbook(self: &Arc<Self>, name: String) -> Result<()> {
        tracing::debug!(playbook = %name, "Resuming playbook");
        {
            let guard = self.workers.lock().await;
            if guard.contains_key(&name) {
                tracing::warn!(playbook = %name, "Playbook already running");
                pyroduct::bail!("Playbook '{}' is already running", name);
            }
        }

        let db_entry = self.db.get_playbook(&name).await?;
        let (_status, pipeline_config, socket_path, _pinned_version, http_address) = match db_entry {
            Some(entry) => entry,
            None => {
                tracing::error!(playbook = %name, "Playbook does not exist in state store");
                pyroduct::bail!("Playbook '{}' does not exist in state store", name);
            }
        };

        tracing::debug!(playbook = %name, "Loading cache manager and playbook binary for resume");
        let cache = CacheManager::from_env()
            .await
            .capture("Failed to initialize CacheManager")?;
        let loaded_pipeline = pipeline_config
            .clone()
            .load(&cache)
            .await
            .capture("Failed to load playbook binary")?;

        let interconnect = self
            .build_interconnect(&name, &loaded_pipeline.playbook.binary.spec)
            .await?;

        tracing::info!(playbook = %name, "Starting worker for resumed playbook");
        let mut worker =
            PlaybookWorker::start(name.clone(), pipeline_config, Some(interconnect)).await?;
        if let Some(ref socket) = socket_path {
            tracing::debug!(playbook = %name, socket = %socket, "Resumed worker listening on socket");
            worker.listen_socket(socket).await?;
        }
        if let Some(ref addr) = http_address {
            tracing::debug!(playbook = %name, http_address = %addr, "Resumed worker starting HTTP server");
            worker.listen_http(addr).await?;
        }
        let _ = self.register_callbacks_from_db(&name, &worker).await;
        tracing::debug!(playbook = %name, "Updating status to running in DB");
        self.db.update_status(&name, "running").await?;

        let mut guard = self.workers.lock().await;
        if guard.contains_key(&name) {
            let _ = worker.shutdown().await;
            tracing::warn!(playbook = %name, "Conflict: playbook was started by another task");
            pyroduct::bail!("Playbook '{}' was started by another task", name);
        }
        guard.insert(name, worker);
        Ok(())
    }

    pub async fn delete_playbook(&self, name: String) -> Result<()> {
        tracing::debug!(playbook = %name, "Deleting playbook");
        let mut guard = self.workers.lock().await;
        if let Some(worker) = guard.remove(&name) {
            tracing::debug!(playbook = %name, "Shutting down worker before delete");
            let _ = worker.shutdown().await;
        }

        tracing::debug!(playbook = %name, "Deleting playbook from database");
        self.db.delete_playbook(&name).await?;

        let playbook_dir = self.working_dir.join("playbooks").join(&name);
        if playbook_dir.exists() {
            tracing::debug!(playbook = %name, playbook_dir = ?playbook_dir, "Removing playbook directory");
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

            let active_capabilities = worker.server.spec().capabilities.clone();

            let remote_capabilities: Vec<CapabilityIdent> = worker
                .capability_processes
                .iter()
                .map(|proc| proc.cap.clone())
                .collect();

            let local_capabilities: Vec<CapabilityIdent> = worker
                .server
                .spec()
                .capabilities
                .iter()
                .filter(|cap| !remote_capabilities.contains(cap))
                .cloned()
                .collect();

            let processed_rows = worker.server.len().await;

            results.push(PlaybookStatus {
                name: worker.name.clone(),
                config_path,
                socket_path: worker
                    .socket_path
                    .clone()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                http_address: worker.http_address.clone(),
                active_capabilities,
                local_capabilities,
                remote_capabilities,
                spec: worker.server.spec(),
                processed_rows,
                pinned_version: None, // Populated by caller if needed
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
            .filter(|(_name, status, _config, _socket, _pinned, _http)| status == "running")
            .map(|(name, _, _, _, _, _)| name)
            .collect();

        let mut attempts = 0;
        let max_attempts = to_resume.len() + 1;

        while !to_resume.is_empty() && attempts < max_attempts {
            let mut failed = Vec::new();
            let mut succeeded_any = false;

            for name in to_resume {
                match self.resume_playbook(name.clone()).await {
                    Ok(()) => {
                        tracing::info!(
                            name,
                            "Successfully resumed running playbook on daemon startup"
                        );
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
        tracing::debug!(playbook = %source, url = %url, uuid = %uuid, "Saving HTTP callback to database");
        self.db
            .add_callback_mapping(uuid, &source, "http", &url)
            .await?;
        let workers = self.workers.lock().await;
        if let Some(worker) = workers.get(&source) {
            tracing::debug!(playbook = %source, uuid = %uuid, "Adding HTTP callback to active worker");
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
        tracing::debug!(playbook = %source, socket_path = %socket_path, uuid = %uuid, "Saving socket callback to database");
        self.db
            .add_callback_mapping(uuid, &source, "socket", &socket_path)
            .await?;
        let workers = self.workers.lock().await;
        if let Some(worker) = workers.get(&source) {
            tracing::debug!(playbook = %source, uuid = %uuid, "Adding socket callback to active worker");
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
        tracing::debug!(playbook = %source, target = %target_playbook, uuid = %uuid, "Saving playbook callback to database");
        self.db
            .add_callback_mapping(uuid, &source, "playbook", &target_playbook)
            .await?;
        let workers = self.workers.lock().await;
        if let Some(worker) = workers.get(&source) {
            tracing::debug!(playbook = %source, uuid = %uuid, "Adding playbook callback to active worker");
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
        tracing::debug!(playbook = %source, "Retrieving callback mappings from database");
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
        tracing::debug!(uuid = %uuid, "Deleting callback mapping from database");
        self.db.delete_callback_mapping(uuid).await?;
        let workers = self.workers.lock().await;
        tracing::debug!(uuid = %uuid, "Removing callback from active workers");
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
        tracing::debug!(playbook = %source, "Loading registered callbacks from database");
        let db_list = self.db.get_callbacks_for_source(source).await?;
        for (uuid, _src, cb_type, target) in db_list {
            tracing::debug!(playbook = %source, uuid = %uuid, cb_type = %cb_type, "Registering loaded callback to worker");
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

    pub async fn list_sessions_for_playbook(
        &self,
        name: &str,
        filter: Option<pyroduct::pipeline::session::SessionStatusFilter>,
    ) -> Result<Vec<SessionInfo>> {
        let worker = {
            let workers = self.workers.lock().await;
            workers
                .get(name)
                .ok_or_else(|| {
                    tracing::error!(playbook = %name, "Playbook is not running");
                    pyroduct::capture!("Playbook '{}' is not running", name)
                })?
                .server
                .clone()
        };

        let raw_sessions = worker
            .list_sessions(filter)
            .await
            .map_err(|e| pyroduct::capture!("Failed to list sessions: {:?}", e))?;

        let sessions = raw_sessions
            .into_iter()
            .map(|(session_id, status)| SessionInfo { session_id, status })
            .collect();

        Ok(sessions)
    }

    pub async fn call(
        &self,
        playbook: &str,
        _row_index: usize,
        row: pyroduct::PyroRow<'static>,
    ) -> Result<()> {
        tracing::debug!(playbook = %playbook, "Invoking playbook callback with input row");
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
        let rec = self.call_playbook_record(name, payload, None).await?;
        match rec {
            pyroduct::pipeline::ServerExecutionRecord::Normal(
                pyroduct::pipeline::ExecutionRecord::Success { success, .. },
            ) => {
                let val = serde_json::to_value(&success).capture("Failed to serialize row")?;
                Ok(val)
            }
            pyroduct::pipeline::ServerExecutionRecord::Session(
                pyroduct::pipeline::session::SessionExecutionRecord::Success { success, .. },
            ) => {
                let val = serde_json::to_value(&success).capture("Failed to serialize row")?;
                Ok(val)
            }
            pyroduct::pipeline::ServerExecutionRecord::SessionDiff(
                pyroduct::pipeline::session_diff::SessionDiffExecutionRecord::Success {
                    success,
                    ..
                },
            ) => {
                let val = serde_json::to_value(&success).capture("Failed to serialize row")?;
                Ok(val)
            }
            pyroduct::pipeline::ServerExecutionRecord::Normal(
                pyroduct::pipeline::ExecutionRecord::Failure { failure, .. },
            ) => {
                let msg = match failure {
                    Ok(captured) => captured.to_string(),
                    Err(s) => s,
                };
                Err(pyroduct::capture!("{}", msg))
            }
            pyroduct::pipeline::ServerExecutionRecord::Session(
                pyroduct::pipeline::session::SessionExecutionRecord::Failure { failure, .. },
            ) => {
                let msg = match failure {
                    Ok(captured) => captured.to_string(),
                    Err(s) => s,
                };
                Err(pyroduct::capture!("{}", msg))
            }
            pyroduct::pipeline::ServerExecutionRecord::SessionDiff(
                pyroduct::pipeline::session_diff::SessionDiffExecutionRecord::Failure {
                    failure,
                    ..
                },
            ) => {
                let msg = match failure {
                    Ok(captured) => captured.to_string(),
                    Err(s) => s,
                };
                Err(pyroduct::capture!("{}", msg))
            }
        }
    }

    pub async fn call_playbook_record(
        &self,
        name: &str,
        payload: serde_json::Value,
        session_id: Option<u32>,
    ) -> Result<pyroduct::pipeline::ServerExecutionRecord> {
        tracing::debug!(playbook = %name, "Executing call_playbook_record");
        let worker = {
            let workers = self.workers.lock().await;
            workers
                .get(name)
                .ok_or_else(|| {
                    tracing::error!(playbook = %name, "Playbook is not running");
                    pyroduct::capture!("Playbook '{}' is not running", name)
                })?
                .server
                .clone()
        };
        let spec = worker.spec();

        let is_session = spec.func.kind == pyro_spec::ModuleKind::Session
            || spec.func.kind == pyro_spec::ModuleKind::SessionDiff;

        let repaired_row = if is_session {
            let input_field = spec.func.input.field_with_name("input").ok_or_else(|| {
                pyroduct::capture!("Session input field 'input' not found in playbook spec")
            })?;

            if let pyro_spec::PyroType::Group(fields) = &input_field.data_type {
                tracing::debug!(playbook = %name, "Deserializing session struct payload to PyroRow");
                let input_row: PyroRow<'static> = serde_json::from_value(payload).capture(
                    "Invalid JSON payload: failed to deserialize session struct input into PyroRow",
                )?;

                tracing::debug!(playbook = %name, "Repairing session input struct row matching schema");
                input_row
                    .project_repair(fields)
                    .capture("Failed to repair session input JSON according to module spec")?
            } else {
                let wrapped_payload = if payload.get("input").is_some() {
                    payload
                } else {
                    serde_json::json!({
                        "input": payload
                    })
                };

                tracing::debug!(playbook = %name, "Deserializing session payload to PyroRow");
                let input_row: PyroRow<'static> = serde_json::from_value(wrapped_payload).capture(
                    "Invalid JSON payload: failed to deserialize session input into PyroRow",
                )?;

                tracing::debug!(playbook = %name, "Repairing session input row matching schema");
                input_row
                    .project_repair(&[input_field.clone()])
                    .capture("Failed to repair session input JSON according to module spec")?
            }
        } else {
            tracing::debug!(playbook = %name, "Deserializing payload to PyroRow");
            let input_row: PyroRow<'static> = serde_json::from_value(payload)
                .capture("Invalid JSON payload: failed to deserialize into PyroRow")?;

            tracing::debug!(playbook = %name, "Repairing row matching schema");
            input_row
                .project_repair(spec.func.input.fields())
                .capture("Failed to repair input JSON according to module spec")?
        };

        tracing::debug!(playbook = %name, "Sending call to worker");
        if let Some(session_id) = session_id {
            worker.call_session(session_id, repaired_row).await
        } else {
            worker.call(repaired_row).await
        }
    }

    pub async fn call_playbook_bulk(
        &self,
        name: &str,
        file_name: &str,
        file_content: Vec<u8>,
    ) -> Result<Vec<pyroduct::pipeline::ServerExecutionRecord>> {
        tracing::debug!(playbook = %name, file = %file_name, "Executing call_playbook_bulk");
        let worker = {
            let workers = self.workers.lock().await;
            workers
                .get(name)
                .ok_or_else(|| pyroduct::capture!("Playbook '{}' is not running", name))?
                .server
                .clone()
        };
        let spec = worker.spec();

        if spec.func.kind == pyro_spec::ModuleKind::Session
            || spec.func.kind == pyro_spec::ModuleKind::SessionDiff
        {
            return Err(pyroduct::capture!("Bulk call is not allowed for session playbooks"));
        }

        let batches = pyro_file::parse_data_to_batch(file_content, file_name)
            .await
            .map_err(|e| pyroduct::capture!("Failed to parse file payload: {:?}", e))?;

        if batches.is_empty() {
            return Err(pyroduct::capture!("No batches found in file"));
        }

        let mut results = Vec::new();

        use pyroduct::format::value::arrow::Rowable;

        for batch_ipc in batches {
            let batch = batch_ipc.to_batch();
            for i in 0..batch.num_rows() {
                let pyro_row = batch.row(i).map_err(|e| {
                    pyroduct::capture!("Row extraction failed at index {}: {:?}", i, e)
                })?;

                let repaired_row = pyro_row
                    .project_repair(spec.func.input.fields())
                    .capture("Failed to repair input according to module spec")?;

                match worker.call(repaired_row).await {
                    Ok(rec) => results.push(rec),
                    Err(e) => {
                        return Err(pyroduct::capture!("Error executing bulk row at index {}: {:?}", i, e));
                    }
                }
            }
        }

        Ok(results)
    }

    /// Check all running (non-pinned) playbooks for newer versions in the cache.
    /// If a newer version is found, stop the old worker and start a new one.
    pub async fn check_for_updates(self: &Arc<Self>) {
        let db_playbooks = match self.db.list_playbooks().await {
            Ok(p) => p,
            Err(e) => {
                tracing::error!(error = ?e, "Auto-update: failed to list playbooks from DB");
                return;
            }
        };

        let cache = match pyro_artifacts::cache::CacheManager::from_env().await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(error = ?e, "Auto-update: failed to initialize CacheManager");
                return;
            }
        };

        for (name, status, config, socket_path, pinned_version, http_address) in db_playbooks {
            // Only check running, non-pinned playbooks
            if status != "running" || pinned_version.is_some() {
                continue;
            }

            // Check if there's a newer version in the cache
            let current = &config.playbook;
            let latest = match cache
                .find_latest_version(&current.author, &current.package)
                .await
            {
                Ok(Some(v)) => v,
                Ok(None) => continue,
                Err(e) => {
                    tracing::warn!(
                        playbook = %name,
                        error = ?e,
                        "Auto-update: failed to query latest version"
                    );
                    continue;
                }
            };

            // Compare using semver
            let current_ver = match semver::Version::parse(&current.version) {
                Ok(v) => v,
                Err(_) => continue, // Can't compare non-semver versions
            };
            let latest_ver = match semver::Version::parse(&latest) {
                Ok(v) => v,
                Err(_) => continue,
            };

            if latest_ver <= current_ver {
                continue;
            }

            tracing::info!(
                playbook = %name,
                current_version = %current.version,
                new_version = %latest,
                "Auto-update: newer version detected, restarting playbook"
            );

            // Stop the old worker
            if let Err(e) = self.stop_playbook(&name).await {
                tracing::error!(
                    playbook = %name,
                    error = ?e,
                    "Auto-update: failed to stop old playbook version"
                );
                continue;
            }

            // Start with the updated version
            let updated_ident = PlaybookIdent {
                author: current.author.clone(),
                package: current.package.clone(),
                version: latest.clone(),
            };

            match self
                .start_playbook(
                    name.clone(),
                    updated_ident,
                    socket_path.clone(),
                    http_address.clone(),
                    None, // Keep existing dirs (they're in the config already)
                    None,
                    None, // Still not pinned
                )
                .await
            {
                Ok(()) => {
                    tracing::info!(
                        playbook = %name,
                        version = %latest,
                        "Auto-update: playbook restarted with new version"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        playbook = %name,
                        new_version = %latest,
                        error = ?e,
                        "Auto-update: failed to start new version, attempting rollback"
                    );
                    // Rollback: try to restart the old version
                    if let Err(rollback_err) = self
                        .start_playbook(
                            name.clone(),
                            current.clone(),
                            socket_path,
                            http_address,
                            None,
                            None,
                            None,
                        )
                        .await
                    {
                        tracing::error!(
                            playbook = %name,
                            error = ?rollback_err,
                            "Auto-update: rollback also failed, playbook is stopped"
                        );
                    }
                }
            }
        }
    }

    /// Run the update check loop on a fixed interval.
    pub async fn run_update_loop(self: Arc<Self>, interval: std::time::Duration) {
        let mut ticker = tokio::time::interval(interval);
        // Skip the first immediate tick
        ticker.tick().await;

        loop {
            ticker.tick().await;
            tracing::debug!("Auto-update: checking for playbook updates");
            self.check_for_updates().await;
        }
    }
}
