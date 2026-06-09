use super::DaemonDataManager;
use crate::Result;
use pyro_artifacts::cache::CacheManager;
use pyroduct::Capture;
use pyroduct::pipeline::data::DataManager;

impl DaemonDataManager {
    pub async fn get_playbook_data(
        &self,
        playbook_name: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<u8>> {
        // 1. Check if the playbook is currently running and query the server directly
        let workers = self.playbooks_manager.workers.lock().await;
        if let Some(worker) = workers.get(playbook_name) {
            let batch_opt = worker
                .server
                .get_output_batch(offset, limit)
                .await
                .map_err(|e| pyroduct::capture!("{:?}", e))?;

            let mut buffer = Vec::new();
            if let Some(batch) = batch_opt {
                let schema = batch.schema();
                let mut writer = arrow::ipc::writer::FileWriter::try_new(&mut buffer, &schema)
                    .capture("Failed to create Arrow IPC FileWriter")?;
                writer
                    .write(&batch)
                    .capture("Failed to write RecordBatch to Arrow IPC")?;
                writer
                    .finish()
                    .capture("Failed to finish Arrow IPC FileWriter")?;
            }
            return Ok(buffer);
        }
        drop(workers);

        // 2. If not running, load the config from playbooks_manager's database
        let db_entry = self
            .playbooks_manager
            .db
            .get_playbook(playbook_name)
            .await?;
        let (_status, pipeline_config, _socket_path) = match db_entry {
            Some(entry) => entry,
            None => pyroduct::bail!("Playbook '{}' does not exist in state store", playbook_name),
        };

        // 3. Load factory to get output schema
        let cache = CacheManager::from_env()
            .await
            .capture("Failed to initialize CacheManager")?;

        let loaded = pipeline_config
            .load(&cache)
            .await
            .map_err(|e| pyroduct::capture!("{:?}", e))?;
        let factory = loaded
            .factory()
            .map_err(|e| pyroduct::capture!("{:?}", e))?;
        let output_schema = factory.factory.spec().func.output.clone();

        // 4. Create and restore DataManager for output_dir
        let dm = DataManager::new(factory.output_dir, output_schema);
        dm.restore()
            .await
            .map_err(|e| pyroduct::capture!("{:?}", e))?;

        // 5. Get slice of data
        let batch_opt = dm
            .get_batch_slice(offset, limit)
            .await
            .map_err(|e| pyroduct::capture!("{:?}", e))?;

        // 6. Serialize to Arrow IPC bytes using FileWriter
        let mut buffer = Vec::new();
        if let Some(batch) = batch_opt {
            let schema = batch.schema();
            let mut writer = arrow::ipc::writer::FileWriter::try_new(&mut buffer, &schema)
                .capture("Failed to create Arrow IPC FileWriter")?;
            writer
                .write(&batch)
                .capture("Failed to write RecordBatch to Arrow IPC")?;
            writer
                .finish()
                .capture("Failed to finish Arrow IPC FileWriter")?;
        }

        Ok(buffer)
    }

    pub async fn get_playbook_execution_record(
        &self,
        playbook_name: &str,
        id: u32,
    ) -> Result<pyroduct::pipeline::ServerExecutionRecord> {
        let workers = self.playbooks_manager.workers.lock().await;
        let worker = workers
            .get(playbook_name)
            .ok_or_else(|| pyroduct::capture!("Playbook '{}' is not running", playbook_name))?;
        let record = worker
            .server
            .get(id)
            .await
            .map_err(|e| pyroduct::capture!("{:?}", e))?;
        Ok(record)
    }

    pub async fn get_playbook_failures(
        &self,
        playbook_name: &str,
    ) -> Result<Vec<pyroduct::pipeline::ServerExecutionRecord>> {
        let workers = self.playbooks_manager.workers.lock().await;
        let worker = workers
            .get(playbook_name)
            .ok_or_else(|| pyroduct::capture!("Playbook '{}' is not running", playbook_name))?;

        let len = worker.server.len().await;
        let mut failures = Vec::new();
        for i in 0..len {
            if let Ok(record) = worker.server.get(i as u32).await {
                let is_fail = match &record {
                    pyroduct::pipeline::ServerExecutionRecord::Normal(
                        pyroduct::pipeline::ExecutionRecord::Failure { .. },
                    ) => true,
                    pyroduct::pipeline::ServerExecutionRecord::Session(
                        pyroduct::pipeline::session::SessionExecutionRecord::Failure { .. },
                    ) => true,
                    pyroduct::pipeline::ServerExecutionRecord::SessionDiff(
                        pyroduct::pipeline::session_diff::SessionDiffExecutionRecord::Failure {
                            ..
                        },
                    ) => true,
                    _ => false,
                };
                if is_fail {
                    failures.push(record);
                }
            }
        }
        Ok(failures)
    }
}
