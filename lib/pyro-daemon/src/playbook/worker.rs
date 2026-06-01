use anyhow::{Context, Result};
use std::path::Path;
use tokio::fs;
use tokio::sync::mpsc;
use uuid::Uuid;

use pyro_artifacts::cache::CacheManager;
use pyroduct::pipeline::factory::PipelineConfig;
use pyroduct::transport::socket::PyroListener;
use pyroduct::transport::socket::playbook::PlaybookServer;

use crate::capability::CapabilityProcess;

pub struct PlaybookWorker {
    pub id: Uuid,
    pub config: PipelineConfig,
    pub socket_path: String,
    pub capability_processes: Vec<CapabilityProcess>,
    pub kill_tx: mpsc::Sender<()>,
}

impl PlaybookWorker {
    pub async fn start(
        id: Uuid,
        pipeline_config: PipelineConfig,
        playbook_socket: String,
    ) -> Result<Self> {
        // 1. Load the playbook binary via CacheManager
        let cache = CacheManager::from_env()
            .await
            .context("Failed to initialize CacheManager")?;
        let loaded_pipeline = pipeline_config
            .clone()
            .load(&cache)
            .await
            .context("Failed to load playbook binary")?;

        let capability_processes = Vec::new();

        // 2. Instantiate and run Playbook Server in-process
        tracing::info!(id = %id, "Instantiating playbook server");
        let server = PlaybookServer::new(&loaded_pipeline.playbook)
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
            config: pipeline_config,
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
    use pyro_artifacts::build::Builder;
    use pyroduct::PyroRow;
    use pyroduct::transport::socket::playbook::PlaybookClient;
    use std::collections::{BTreeMap, HashMap};
    use tempfile::tempdir;

    const TEST_CODE: &str = r#"
use pyroduct;

#[pyroduct::module(output = message)]
pub fn call(input: String) -> Result<String> {
    Ok(format!("Hello: {}", input))
}
"#;

    #[tracing_test::traced_test]
    #[tokio::test]
    async fn test_playbook_worker_without_capabilities() {
        let cache = std::sync::Arc::new(CacheManager::from_env().await.unwrap());
        let builder = Builder::from_env(cache.clone()).await.unwrap();

        let playbook = pyro_artifacts::build::AnonPlaybook {
            package: "test_playbook".to_string(),
            dependencies: BTreeMap::new(),
            configurations: std::vec::Vec::new(),
            source: TEST_CODE.to_string(),
        };

        let binary = builder
            .compile_anon(&playbook)
            .await
            .expect("Valid module should compile");

        let tmp_dir = tempdir().unwrap();
        let socket_path = tmp_dir.path().join("playbook.sock");

        let ident = &binary.spec.ident;

        let pipeline_config = PipelineConfig {
            playbook: ident.clone(),
            remote: HashMap::new(),
            wal_capacity: 10,
            success_log_retention_secs: 3600,
            error_log_retention_secs: 86400 * 7,
            input_dir: tmp_dir.path().to_path_buf(),
            output_dir: tmp_dir.path().to_path_buf(),
            log_dir: tmp_dir.path().to_path_buf(),
        };

        let id = Uuid::new_v4();
        let worker = PlaybookWorker::start(
            id,
            pipeline_config,
            socket_path.to_string_lossy().to_string(),
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
