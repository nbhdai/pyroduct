use crate::Result;
use pyroduct::Capture;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;

use pyro_artifacts::cache::CacheManager;
use pyroduct::pipeline::factory::PipelineConfig;
use pyroduct::transport::socket::PyroListener;

use crate::capability::CapabilityProcess;

pub struct PlaybookWorker {
    pub name: String,
    pub config: PipelineConfig,
    pub socket_path: Option<PathBuf>,
    pub capability_processes: Vec<CapabilityProcess>,
    pub shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    pub server: pyroduct::pipeline::PipelineServer,
}

use pyroduct::module::interconnect::PlaybookInterconnect;

impl PlaybookWorker {
    pub async fn start(
        name: String,
        pipeline_config: PipelineConfig,
        interconnect: Option<Arc<dyn PlaybookInterconnect>>,
    ) -> Result<Self> {
        // 1. Load the playbook binary via CacheManager
        let cache = CacheManager::from_env()
            .await
            .capture("Failed to initialize CacheManager")?;
        let loaded_pipeline = pipeline_config
            .clone()
            .load(&cache)
            .await
            .capture("Failed to load playbook binary")?;

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
                    .capture("Failed to spawn capability process")?;
                capability_processes.push(proc);
            }
        }

        // 2. Instantiate playbook Server in-process
        tracing::info!(name, "Instantiating playbook server");
        let server = if let Some(ic) = interconnect {
            pyroduct::pipeline::PipelineServer::new_with_interconnect(&loaded_pipeline.playbook, ic)
                .await
                .capture("Failed to construct PipelineServer with interconnect")?
        } else {
            pyroduct::pipeline::PipelineServer::new(&loaded_pipeline.playbook)
                .await
                .capture("Failed to construct PipelineServer")?
        };

        Ok(Self {
            name,
            config: pipeline_config,
            socket_path: None,
            capability_processes,
            shutdown_tx: None,
            server,
        })
    }

    pub async fn listen_socket(&mut self, playbook_socket: impl AsRef<Path>) -> Result<()> {
        if self.shutdown_tx.is_some() {
            pyroduct::bail!("Socket listener is already running");
        }

        let socket_path = playbook_socket.as_ref();
        if socket_path.exists() {
            let _ = fs::remove_file(socket_path).await;
        }
        let listener = PyroListener::bind_unix(socket_path)
            .await
            .capture("Failed to bind PyroListener socket")?;

        let shutdown_tx = pyroduct::transport::socket::playbook::run(self.server.clone(), listener);
        self.socket_path = Some(socket_path.to_path_buf());
        self.shutdown_tx = Some(shutdown_tx);
        Ok(())
    }

    pub async fn call(
        &self,
        row: pyroduct::PyroRow<'_>,
    ) -> Result<(u32, pyroduct::PyroRow<'static>)> {
        self.server
            .call(row)
            .await
            .map_err(|e| pyroduct::capture!("Failed to call playbook: {:?}", e))
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
        if let Some(shutdown_tx) = self.shutdown_tx {
            let _ = shutdown_tx.send(());
        }

        // Cleanup socket file if Unix UDS
        if let Some(socket_path) = &self.socket_path {
            if socket_path.exists() {
                let _ = std::fs::remove_file(socket_path);
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
    use super::super::PlaybooksManager;
    use super::*;
    use pyroduct::PyroRow;
    use pyroduct::transport::socket::playbook::PlaybookClient;
    use std::collections::HashMap;
    use tempfile::tempdir;

    #[tracing_test::traced_test]
    #[tokio::test]
    async fn test_playbook_worker_without_capabilities() {
        let cache = std::sync::Arc::new(CacheManager::from_env().await.unwrap());
        let binary = cache
            .get_named_binary("nbhdai", "integration_error", "0.1.0")
            .await
            .unwrap();

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

        let mut worker = PlaybookWorker::start("test".to_string(), pipeline_config, None)
            .await
            .expect("Failed to start playbook worker");

        worker
            .listen_socket(&socket_path)
            .await
            .expect("Failed to listen on socket");

        // Give a short moment to bind/listen
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let mut client = PlaybookClient::connect_unix(&socket_path)
            .await
            .expect("Failed to connect to playbook worker");

        let res = client
            .call(&PyroRow::from([("input", "World".into())]))
            .await
            .expect("Failed to call playbook client");

        assert_eq!(res.row.get_str("message").unwrap(), "Success: World");

        worker.shutdown().await.expect("Failed to shutdown worker");
    }

    #[tracing_test::traced_test]
    #[tokio::test]
    async fn test_playbook_worker_plain_call() {
        let cache = std::sync::Arc::new(CacheManager::from_env().await.unwrap());
        let binary = cache
            .get_named_binary("nbhdai", "integration_error", "0.1.0")
            .await
            .unwrap();

        let tmp_dir = tempdir().unwrap();
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

        let worker = PlaybookWorker::start("test_plain".to_string(), pipeline_config, None)
            .await
            .expect("Failed to start playbook worker");

        let input_row = PyroRow::from([("input", "World".into())]);
        let (_session_id, res) = worker
            .call(input_row)
            .await
            .expect("Failed to call playbook worker directly");

        assert_eq!(res.get_str("message").unwrap(), "Success: World");

        worker.shutdown().await.expect("Failed to shutdown worker");
    }

    use std::sync::atomic::{AtomicUsize, Ordering};

    static CALLBACK_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

    async fn increment_callback_count(row_index: usize, _row: &PyroRow<'_>) {
        CALLBACK_CALL_COUNT.fetch_add(row_index + 1, Ordering::SeqCst);
    }

    #[tracing_test::traced_test]
    #[tokio::test]
    async fn test_playbook_worker_with_callback() {
        let cache = std::sync::Arc::new(CacheManager::from_env().await.unwrap());
        let binary = cache
            .get_named_binary("nbhdai", "integration_error", "0.1.0")
            .await
            .unwrap();

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

        let mut worker = PlaybookWorker::start("test_cb".to_string(), pipeline_config, None)
            .await
            .expect("Failed to start playbook worker");
        worker
            .listen_socket(socket_path.to_string_lossy().to_string())
            .await
            .unwrap();

        // Register the function callback
        CALLBACK_CALL_COUNT.store(0, Ordering::SeqCst);
        let cb = pyroduct::pipeline::Callback::function(|idx, row| {
            let row_static = row.to_static();
            Box::pin(async move {
                increment_callback_count(idx, &row_static).await;
            })
        });
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

        assert_eq!(res.row.get_str("message").unwrap(), "Success: World");

        // Check if callback was successfully called (row_index = 0, so should add 1)
        assert_eq!(CALLBACK_CALL_COUNT.load(Ordering::SeqCst), 1);

        worker.shutdown().await.expect("Failed to shutdown worker");
    }

    #[tracing_test::traced_test]
    #[tokio::test]
    async fn test_playbook_worker_with_interconnect() {
        tracing::info!("Starting test_playbook_worker_with_interconnect");
        let cache = std::sync::Arc::new(CacheManager::from_env().await.unwrap());
        let target_binary = cache
            .get_named_binary("nbhdai", "interconnect_target", "0.1.0")
            .await
            .unwrap();
        let caller_binary = cache
            .get_named_binary("nbhdai", "interconnect_caller", "0.1.0")
            .await
            .unwrap();

        let tmp_dir = tempdir().unwrap();
        let manager_dir = tmp_dir.path().join("manager_dir");
        let manager = Arc::new(PlaybooksManager::new(manager_dir));

        let config_a_path = tmp_dir.path().join("config_a.toml");
        let pipeline_config_a = PipelineConfig {
            playbook: target_binary.spec.ident.clone(),
            remote: HashMap::new(),
            wal_capacity: 10,
            success_log_retention_secs: 3600,
            error_log_retention_secs: 86400 * 7,
            input_dir: tmp_dir.path().to_path_buf(),
            output_dir: tmp_dir.path().to_path_buf(),
            log_dir: tmp_dir.path().to_path_buf(),
        };
        let toml_a = toml::to_string_pretty(&pipeline_config_a).unwrap();
        std::fs::write(&config_a_path, toml_a).unwrap();

        let config_b_path = tmp_dir.path().join("config_b.toml");
        let pipeline_config_b = PipelineConfig {
            playbook: caller_binary.spec.ident.clone(),
            remote: HashMap::new(),
            wal_capacity: 10,
            success_log_retention_secs: 3600,
            error_log_retention_secs: 86400 * 7,
            input_dir: tmp_dir.path().to_path_buf(),
            output_dir: tmp_dir.path().to_path_buf(),
            log_dir: tmp_dir.path().to_path_buf(),
        };
        let toml_b = toml::to_string_pretty(&pipeline_config_b).unwrap();
        std::fs::write(&config_b_path, toml_b).unwrap();

        // 1. Start target playbook first
        manager
            .start_playbook("target".to_string(), config_a_path, None, None, None)
            .await
            .expect("Failed to start target playbook");

        // 2. Start caller playbook, which requires the target playbook in its interconnect
        manager
            .start_playbook("caller".to_string(), config_b_path, None, None, None)
            .await
            .expect("Failed to start caller playbook");

        // 3. Invoke caller playbook via call_playbook RPC
        let payload = serde_json::json!({
            "input": "World"
        });
        let res = manager
            .call_playbook("caller", payload)
            .await
            .expect("Failed to call caller playbook");

        assert_eq!(
            res.get("message").and_then(|v| v.as_str()).unwrap(),
            "Caller received: Hello: World"
        );

        // 4. Shutdown workers
        manager.stop_playbook("caller").await.unwrap();
        manager.stop_playbook("target").await.unwrap();
    }

    #[tracing_test::traced_test]
    #[tokio::test]
    async fn test_daemon_auto_resume() {
        let test_dir = tempdir().unwrap();
        let working_dir = test_dir.path().to_path_buf();

        let cache = std::sync::Arc::new(CacheManager::from_env().await.unwrap());
        let binary = cache
            .get_named_binary("nbhdai", "integration_error", "0.1.0")
            .await
            .unwrap();

        let config_path = working_dir.join("config.toml");
        let pipeline_config = PipelineConfig {
            playbook: binary.spec.ident.clone(),
            remote: HashMap::new(),
            wal_capacity: 10,
            success_log_retention_secs: 3600,
            error_log_retention_secs: 86400 * 7,
            input_dir: working_dir.join("input"),
            output_dir: working_dir.join("output"),
            log_dir: working_dir.join("log"),
        };
        std::fs::write(
            &config_path,
            toml::to_string_pretty(&pipeline_config).unwrap(),
        )
        .unwrap();

        let pm1 = Arc::new(PlaybooksManager::new(working_dir.clone()));
        pm1.start_playbook(
            "integration_error".to_string(),
            config_path,
            None,
            None,
            None,
        )
        .await
        .unwrap();

        assert_eq!(pm1.active_workers_count().await, 1);

        // Stop pm1 worker to release file and WAL locks
        let worker = pm1
            .workers
            .lock()
            .await
            .remove("integration_error")
            .unwrap();
        worker.shutdown().await.unwrap();

        // Give background tasks a brief moment to completely drop the database connections and locks
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        drop(pm1);

        let pm2 = Arc::new(PlaybooksManager::new(working_dir.clone()));
        assert_eq!(pm2.active_workers_count().await, 0);

        pm2.resume_active_playbooks().await.unwrap();

        assert_eq!(pm2.active_workers_count().await, 1);
        let active = pm2.list_playbooks().await;
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].name, "integration_error");
    }
}
