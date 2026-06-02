use anyhow::{Context, Result};
use std::path::Path;
use tokio::fs;

use pyro_artifacts::cache::CacheManager;
use pyroduct::pipeline::factory::PipelineConfig;
use pyroduct::transport::socket::PyroListener;

use crate::capability::CapabilityProcess;

pub struct PlaybookWorker {
    pub name: String,
    pub config: PipelineConfig,
    pub socket_path: String,
    pub capability_processes: Vec<CapabilityProcess>,
    pub shutdown_tx: tokio::sync::oneshot::Sender<()>,
    pub server: pyroduct::pipeline::PipelineServer,
}

impl PlaybookWorker {
    pub async fn start(
        name: String,
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

        let mut capability_processes = Vec::new();
        for (cap, addr) in &loaded_pipeline.playbook.remote {
            if let pyro_artifacts::cache::RemoteAddress::Unix(socket_path) = addr {
                tracing::info!(
                    name,
                    %cap,
                    socket = %socket_path.display(),
                    "Spawning capability process for playbook"
                );
                let cap_config = loaded_pipeline
                    .playbook
                    .binary
                    .configurations
                    .iter()
                    .find(|c| c.package == cap.package)
                    .map(|c| serde_json::to_value(&c.configuration).unwrap());

                let proc = CapabilityProcess::spawn(cap, socket_path, cap_config.as_ref())
                    .await
                    .context("Failed to spawn capability process")?;
                capability_processes.push(proc);
            }
        }

        // 2. Instantiate and run Playbook Server in-process
        tracing::info!(name, "Instantiating playbook server");
        let server = pyroduct::pipeline::PipelineServer::new(&loaded_pipeline.playbook)
            .await
            .context("Failed to construct PipelineServer")?;
        let listener = if let Ok(addr) = playbook_socket.parse::<std::net::SocketAddr>() {
            PyroListener::bind_tcp(addr).await?
        } else {
            let socket_path = Path::new(&playbook_socket);
            if socket_path.exists() {
                let _ = fs::remove_file(socket_path).await;
            }
            PyroListener::bind_unix(socket_path).await?
        };

        let shutdown_tx = pyroduct::transport::socket::playbook::run(server.clone(), listener);

        Ok(Self {
            name,
            config: pipeline_config,
            socket_path: playbook_socket,
            capability_processes,
            shutdown_tx,
            server,
        })
    }

    pub async fn add_callback(
        &self,
        uuid: uuid::Uuid,
        callback: pyroduct::pipeline::Callback,
    ) -> Result<()> {
        self.server.add_callback(uuid, callback).await;
        Ok(())
    }

    pub async fn delete_callback(&self, uuid: uuid::Uuid) -> Result<()> {
        self.server.delete_callback(uuid).await;
        Ok(())
    }

    pub async fn shutdown(self) -> Result<()> {
        let _ = self.shutdown_tx.send(());

        // Cleanup socket file if Unix UDS
        if self.socket_path.parse::<std::net::SocketAddr>().is_err() {
            let path = Path::new(&self.socket_path);
            if path.exists() {
                let _ = std::fs::remove_file(path);
            }
        }

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

        let worker = PlaybookWorker::start(
            "test".to_string(),
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

    use std::sync::atomic::{AtomicUsize, Ordering};

    static CALLBACK_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

    fn increment_callback_count(row_index: usize, _row: &PyroRow<'_>) {
        CALLBACK_CALL_COUNT.fetch_add(row_index + 1, Ordering::SeqCst);
    }

    #[tracing_test::traced_test]
    #[tokio::test]
    async fn test_playbook_worker_with_callback() {
        let cache = std::sync::Arc::new(CacheManager::from_env().await.unwrap());
        let builder = Builder::from_env(cache.clone()).await.unwrap();

        let playbook = pyro_artifacts::build::AnonPlaybook {
            package: "test_playbook_cb".to_string(),
            dependencies: BTreeMap::new(),
            configurations: std::vec::Vec::new(),
            source: TEST_CODE.to_string(),
        };

        let binary = builder
            .compile_anon(&playbook)
            .await
            .expect("Valid module should compile");

        let tmp_dir = tempdir().unwrap();
        let socket_path = tmp_dir.path().join("playbook_cb.sock");

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

        let worker = PlaybookWorker::start(
            "test_cb".to_string(),
            pipeline_config,
            socket_path.to_string_lossy().to_string(),
        )
        .await
        .expect("Failed to start playbook worker");

        // Register the function callback
        CALLBACK_CALL_COUNT.store(0, Ordering::SeqCst);
        let cb = pyroduct::pipeline::Callback::function(increment_callback_count);
        worker
            .add_callback(uuid::Uuid::new_v4(), cb)
            .await
            .expect("Failed to add callback");

        // Give a short moment to bind/listen and process message
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        let mut client = PlaybookClient::connect_unix(&socket_path)
            .await
            .expect("Failed to connect to playbook worker");

        let res = client
            .call(&PyroRow::from([("input", "World".into())]))
            .await
            .expect("Failed to call playbook client");

        assert_eq!(res.row.get_str("message").unwrap(), "Hello: World");

        // Check if callback was successfully called (row_index = 0, so should add 1)
        assert_eq!(CALLBACK_CALL_COUNT.load(Ordering::SeqCst), 1);

        worker.shutdown().await.expect("Failed to shutdown worker");
    }
}
