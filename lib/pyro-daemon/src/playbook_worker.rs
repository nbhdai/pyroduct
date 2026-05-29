use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::sync::mpsc;
use uuid::Uuid;

use pyro_artifacts::cache::CacheManager;
use pyroduct::pipeline::factory::PipelineConfig;
use pyroduct::transport::socket::PyroListener;
use pyroduct::transport::socket::playbook::PlaybookServer;

use crate::CapabilityProcess;

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
        _cap_configs: HashMap<String, serde_json::Value>,
    ) -> Result<Self> {
        // 1. Read and parse pipeline configuration
        let config_str = fs::read_to_string(&playbook_config_path)
            .await
            .context("Failed to read playbook config file")?;

        let pipeline_config: PipelineConfig = match playbook_config_path
            .extension()
            .and_then(|s| s.to_str())
        {
            Some("toml") => toml::from_str(&config_str).context("Failed to parse pipeline TOML")?,
            Some("yaml") | Some("yml") => {
                serde_yaml::from_str(&config_str).context("Failed to parse pipeline YAML")?
            }
            Some("json") => {
                serde_json::from_str(&config_str).context("Failed to parse pipeline JSON")?
            }
            _ => anyhow::bail!("Unknown playbook config extension; supports toml, yaml and json"),
        };

        // 2. Load the playbook binary via CacheManager
        let cache = CacheManager::from_env()
            .await
            .context("Failed to initialize CacheManager")?;
        let mut loaded_playbook = cache
            .load_playbook(
                pipeline_config.playbook_hash,
                HashMap::new(),
                pipeline_config.log_dir,
                pipeline_config.input_dir,
                pipeline_config.output_dir,
            )
            .await
            .context("Failed to load playbook binary")?;

        // 3. Map all capabilities locally directly to their libraries
        let mut local_paths = HashMap::new();
        for (cap_name, cap_lib_path) in cap_libraries {
            local_paths.insert(cap_name, cap_lib_path);
        }

        // 4. Inject local paths into LoadedPlaybook and ensure remote maps are empty
        loaded_playbook.paths = local_paths;
        loaded_playbook.remote.clear();

        let capability_processes = Vec::new();

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
            if playbook_socket_clone
                .parse::<std::net::SocketAddr>()
                .is_err()
            {
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

    pub async fn shutdown(self) -> Result<()> {
        let _ = self.kill_tx.send(()).await;
        for mut cap in self.capability_processes {
            let _ = cap.kill().await;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyro_artifacts::artifacts::{ModuleDependencies, PlaybookSource};
    use pyro_artifacts::build::Builder;
    use pyroduct::PyroRow;
    use pyroduct::transport::socket::playbook::PlaybookClient;
    use std::collections::BTreeMap;
    use tempfile::tempdir;

    const TEST_CODE: &str = r#"
use pyroduct;

#[pyroduct::module(output = message)]
pub fn call(input: String) -> Result<String> {
    Ok(format!("Hello: {}", input))
}
"#;

    #[tokio::test]
    async fn test_playbook_worker_without_capabilities() {
        let cache = std::sync::Arc::new(CacheManager::from_env().await.unwrap());
        let builder = Builder::from_env(cache.clone()).await.unwrap();

        let source = PlaybookSource {
            dependencies: ModuleDependencies {
                dependencies: BTreeMap::new(),
                capabilities: vec![],
            },
            source: TEST_CODE.to_string(),
            ident: None,
            configurations: std::vec::Vec::new(),
        };

        let binary = builder
            .compile(&source)
            .await
            .expect("Valid module should compile");

        let tmp_dir = tempdir().unwrap();
        let config_path = tmp_dir.path().join("pipeline.json");
        let socket_path = tmp_dir.path().join("playbook.sock");

        // Write the pipeline config json
        let config_json = serde_json::json!({
            "playbook_hash": binary.hash(),
            "remote": {},
            "wal_capacity": 10,
            "success_log_retention_secs": 3600,
            "error_log_retention_secs": 86400 * 7,
            "input_dir": tmp_dir.path(),
            "output_dir": tmp_dir.path(),
            "log_dir": tmp_dir.path(),
        });

        tokio::fs::write(&config_path, serde_json::to_string(&config_json).unwrap())
            .await
            .unwrap();

        let id = Uuid::new_v4();
        let worker = PlaybookWorker::start(
            id,
            config_path,
            socket_path.to_string_lossy().to_string(),
            HashMap::new(),
            HashMap::new(),
        )
        .await
        .expect("Failed to start playbook worker");

        // Give a short moment to bind/listen
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let mut client = PlaybookClient::connect_unix(&socket_path)
            .await
            .expect("Failed to connect to playbook worker");

        let res = client
            .call(&PyroRow::from([("input", "World".into())]))
            .await
            .expect("Failed to call playbook client");

        assert_eq!(res.row.get_str("message").unwrap(), "Hello: World");

        worker.shutdown().await.expect("Failed to shutdown worker");
    }
}
