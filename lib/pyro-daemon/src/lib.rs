use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::net::{UnixListener, UnixStream};
use tokio::process::{Command, Child};
use tokio::sync::{mpsc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use anyhow::{Context, Result};

use pyro_artifacts::cache::{CacheManager, LoadedPlaybook, RemoteAddress};
use pyroduct::pipeline::factory::PipelineConfig;
use pyroduct::transport::socket::PyroListener;
use pyroduct::transport::socket::playbook::PlaybookServer;

// =============================================================================
// RPC Message Types
// =============================================================================

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonRequest {
    StartPlaybook {
        playbook_config_path: PathBuf,
        playbook_socket: String,
        cap_libraries: HashMap<String, PathBuf>,
        cap_configs: HashMap<String, serde_json::Value>,
    },
    StopPlaybook {
        playbook_id: Uuid,
    },
    ListPlaybooks,
    Status,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlaybookStatus {
    pub id: Uuid,
    pub config_path: PathBuf,
    pub socket_path: String,
    pub active_capabilities: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonResponse {
    Success {
        message: String,
        playbook_id: Option<Uuid>,
    },
    Playbooks {
        playbooks: Vec<PlaybookStatus>,
    },
    StatusInfo {
        active_workers: usize,
        version: String,
    },
    Error {
        message: String,
    },
}

// =============================================================================
// Capability Process Supervisor
// =============================================================================

#[derive(Debug)]
pub struct CapabilityProcess {
    pub cap_name: String,
    pub socket_path: PathBuf,
    pub child: Child,
}

impl CapabilityProcess {
    pub async fn spawn(
        cap_name: &str,
        cap_lib_path: &Path,
        socket_path: &Path,
        cap_config: Option<&serde_json::Value>,
    ) -> Result<Self> {
        let pyroduct_bin = get_pyroduct_bin();
        tracing::info!(
            cap = %cap_name,
            bin = %pyroduct_bin.display(),
            socket = %socket_path.display(),
            "Spawning capability runner process"
        );

        let mut cmd = Command::new(pyroduct_bin);
        cmd.arg("serve")
            .arg("--server-type").arg("capability")
            .arg("--socket").arg(socket_path)
            .arg("--cap-name").arg(cap_name)
            .arg("--cap-path").arg(cap_lib_path);

        if let Some(config) = cap_config {
            let config_json = serde_json::to_string(config)?;
            cmd.arg("--cap-config").arg(config_json);
        }

        // Redirect outputs to avoid cluttering, but capture for tracing
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn().context("Failed to spawn capability runner child process")?;

        // Start tasks to read stdout/stderr and trace them
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let cap_name_clone = cap_name.to_string();

        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::info!(cap = %cap_name_clone, "[STDOUT] {}", line);
            }
        });

        let cap_name_clone = cap_name.to_string();
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::warn!(cap = %cap_name_clone, "[STDERR] {}", line);
            }
        });

        // Wait for UDS socket file to be created
        let mut retries = 0;
        while !socket_path.exists() {
            if retries > 100 {
                let _ = child.kill().await;
                anyhow::bail!("Capability process failed to bind socket at {:?}", socket_path);
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            retries += 1;
        }

        Ok(Self {
            cap_name: cap_name.to_string(),
            socket_path: socket_path.to_path_buf(),
            child,
        })
    }

    pub async fn kill(&mut self) -> Result<()> {
        let _ = self.child.kill().await;
        if self.socket_path.exists() {
            let _ = fs::remove_file(&self.socket_path).await;
        }
        Ok(())
    }
}

impl Drop for CapabilityProcess {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
        if self.socket_path.exists() {
            let _ = std::fs::remove_file(&self.socket_path);
        }
    }
}

// =============================================================================
// Playbook Worker Thread Coordinator
// =============================================================================

pub struct PlaybookWorker {
    pub id: Uuid,
    pub config_path: PathBuf,
    pub socket_path: String,
    pub capability_processes: Vec<CapabilityProcess>,
    pub kill_tx: mpsc::Sender<()>,
}

impl PlaybookWorker {
    pub async fn start(
        id: Uuid,
        playbook_config_path: PathBuf,
        playbook_socket: String,
        cap_libraries: HashMap<String, PathBuf>,
        cap_configs: HashMap<String, serde_json::Value>,
    ) -> Result<Self> {
        // 1. Read and parse pipeline configuration
        let config_str = fs::read_to_string(&playbook_config_path)
            .await
            .context("Failed to read playbook config file")?;
        
        let pipeline_config: PipelineConfig = match playbook_config_path.extension().and_then(|s| s.to_str()) {
            Some("toml") => toml::from_str(&config_str).context("Failed to parse pipeline TOML")?,
            Some("yaml") | Some("yml") => serde_yaml::from_str(&config_str).context("Failed to parse pipeline YAML")?,
            Some("json") => serde_json::from_str(&config_str).context("Failed to parse pipeline JSON")?,
            _ => anyhow::bail!("Unknown playbook config extension; supports toml, yaml and json"),
        };

        // 2. Load the playbook binary via CacheManager
        let cache = CacheManager::from_env().await.context("Failed to initialize CacheManager")?;
        let mut loaded_playbook = cache
            .load_playbook(
                pipeline_config.playbook,
                pipeline_config.log_dir,
                pipeline_config.input_dir,
                pipeline_config.output_dir,
            )
            .await
            .context("Failed to load playbook binary")?;

        // 3. Spawn capability processes behind unique sockets
        let mut capability_processes = Vec::new();
        let mut remote_mappings = HashMap::new();

        for (cap_name, cap_lib_path) in cap_libraries {
            let socket_path = PathBuf::from(format!("/tmp/pyro-cap-{}-{}.sock", id, cap_name));
            let cap_config = cap_configs.get(&cap_name);

            let cap_proc = CapabilityProcess::spawn(&cap_name, &cap_lib_path, &socket_path, cap_config).await?;
            
            // Map the capability to UDS socket path
            remote_mappings.insert(cap_name.clone(), RemoteAddress::Unix(socket_path));
            capability_processes.push(cap_proc);
        }

        // 4. Inject remote mappings into LoadedPlaybook
        loaded_playbook.remote = remote_mappings;

        // 5. Instantiate and run Playbook Server in-process
        tracing::info!(id = %id, "Instantiating playbook server");
        let server = PlaybookServer::new(&loaded_playbook)
            .await
            .context("Failed to construct PlaybookServer")?;

        let listener = if let Ok(addr) = playbook_socket.parse::<std::net::SocketAddr>() {
            PyroListener::bind_tcp(addr).await?
        } else {
            let socket_path = Path::new(&playbook_socket);
            if socket_path.exists() {
                let _ = fs::remove_file(socket_path).await;
            }
            PyroListener::bind_unix(socket_path).await?
        };

        let (kill_tx, mut kill_rx) = mpsc::channel::<()>(1);
        let playbook_socket_clone = playbook_socket.clone();

        tokio::spawn(async move {
            tracing::info!(socket = %playbook_socket_clone, "PlaybookServer running worker loop");
            
            tokio::select! {
                res = server.run(listener) => {
                    if let Err(e) = res {
                        tracing::error!("PlaybookServer run error: {:?}", e);
                    }
                }
                _ = kill_rx.recv() => {
                    tracing::info!("Received kill signal; tearing down playbook worker");
                }
            }

            // Cleanup socket file if Unix UDS
            if playbook_socket_clone.parse::<std::net::SocketAddr>().is_err() {
                let path = Path::new(&playbook_socket_clone);
                if path.exists() {
                    let _ = std::fs::remove_file(path);
                }
            }
        });

        Ok(Self {
            id,
            config_path: playbook_config_path,
            socket_path: playbook_socket,
            capability_processes,
            kill_tx,
        })
    }

    pub async fn shutdown(mut self) -> Result<()> {
        let _ = self.kill_tx.send(()).await;
        for mut cap in self.capability_processes {
            let _ = cap.kill().await;
        }
        Ok(())
    }
}

// =============================================================================
// Helper: Binary candidate lookup
// =============================================================================

fn get_pyroduct_bin() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        let parent = exe.parent().unwrap();
        let candidate = parent.join("pyroduct");
        if candidate.exists() {
            return candidate;
        }
        // Check standard target directory parent for workspace development
        if let Some(grandparent) = parent.parent() {
            let candidate = grandparent.join("pyroduct");
            if candidate.exists() {
                return candidate;
            }
        }
    }
    PathBuf::from("pyroduct")
}

// =============================================================================
// PyroDaemon Central Controller
// =============================================================================

pub struct PyroDaemon {
    control_socket_path: PathBuf,
    workers: Arc<Mutex<HashMap<Uuid, PlaybookWorker>>>,
}

impl PyroDaemon {
    pub fn new(control_socket_path: PathBuf) -> Self {
        Self {
            control_socket_path,
            workers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn run(&self) -> Result<()> {
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

            let workers_clone = self.workers.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_client(socket, workers_clone).await {
                    tracing::error!("Error handling control client: {:?}", e);
                }
            });
        }
    }
}

async fn handle_client(
    mut socket: UnixStream,
    workers: Arc<Mutex<HashMap<Uuid, PlaybookWorker>>>,
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
            DaemonRequest::StartPlaybook {
                playbook_config_path,
                playbook_socket,
                cap_libraries,
                cap_configs,
            } => {
                let id = Uuid::new_v4();
                match PlaybookWorker::start(
                    id,
                    playbook_config_path,
                    playbook_socket,
                    cap_libraries,
                    cap_configs,
                )
                .await
                {
                    Ok(worker) => {
                        let mut guard = workers.lock().await;
                        guard.insert(id, worker);
                        DaemonResponse::Success {
                            message: "Playbook worker and capability servers started successfully".to_string(),
                            playbook_id: Some(id),
                        }
                    }
                    Err(e) => DaemonResponse::Error {
                        message: format!("Failed to start playbook worker: {:?}", e),
                    },
                }
            }
            DaemonRequest::StopPlaybook { playbook_id } => {
                let mut guard = workers.lock().await;
                if let Some(worker) = guard.remove(&playbook_id) {
                    match worker.shutdown().await {
                        Ok(()) => DaemonResponse::Success {
                            message: "Playbook worker and capability processes shut down successfully".to_string(),
                            playbook_id: Some(playbook_id),
                        },
                        Err(e) => DaemonResponse::Error {
                            message: format!("Error during playbook shutdown: {:?}", e),
                        },
                    }
                } else {
                    DaemonResponse::Error {
                        message: format!("No active playbook worker found with ID: {}", playbook_id),
                    }
                }
            }
            DaemonRequest::ListPlaybooks => {
                let guard = workers.lock().await;
                let playbooks = guard
                    .values()
                    .map(|w| PlaybookStatus {
                        id: w.id,
                        config_path: w.config_path.clone(),
                        socket_path: w.socket_path.clone(),
                        active_capabilities: w
                            .capability_processes
                            .iter()
                            .map(|c| c.cap_name.clone())
                            .collect(),
                    })
                    .collect();

                DaemonResponse::Playbooks { playbooks }
            }
            DaemonRequest::Status => {
                let guard = workers.lock().await;
                DaemonResponse::StatusInfo {
                    active_workers: guard.len(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                }
            }
        };

        let resp_str = serde_json::to_string(&response)? + "\n";
        writer.write_all(resp_str.as_bytes()).await?;
    }

    Ok(())
}
