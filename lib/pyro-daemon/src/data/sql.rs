use super::DaemonDataManager;
use crate::Result;
use pyroduct::Capture;
use datafusion::prelude::SessionContext;
use pyro_artifacts::cache::CacheManager;
use pyroduct::pipeline::data::DataManager;
use pyroduct::pipeline::factory::PipelineConfig;

impl DaemonDataManager {
    pub async fn query_playbook_data(&self, playbook_name: &str, sql_query: &str) -> Result<Vec<u8>> {
        // 1. Locate the playbook configuration in ROOT/playbooks/{playbook_name}/config.toml
        let config_path = self
            .playbooks_manager
            .working_dir
            .join("playbooks")
            .join(playbook_name)
            .join("config.toml");

        if !config_path.exists() {
            pyroduct::bail!("Playbook configuration not found for: {}", playbook_name);
        }

        // 2. Load the pipeline config
        let config_str = tokio::fs::read_to_string(&config_path)
            .await
            .capture("Failed to read playbook pipeline config")?;

        let pipeline_config: PipelineConfig = match config_path.extension().and_then(|s| s.to_str())
        {
            Some("toml") => toml::from_str(&config_str).capture("Failed to parse pipeline TOML")?,
            Some("yaml") | Some("yml") => {
                serde_yaml::from_str(&config_str).capture("Failed to parse pipeline YAML")?
            }
            Some("json") => {
                serde_json::from_str(&config_str).capture("Failed to parse pipeline JSON")?
            }
            _ => pyroduct::bail!("Unknown playbook config extension"),
        };

        // 3. Load factory to get output schema
        let cache = CacheManager::from_env()
            .await
            .capture("Failed to initialize CacheManager")?;

        let loaded = pipeline_config
            .load(&cache)
            .await
            .map_err(|e| pyroduct::capture!("{:?}", e))?;
        let factory = loaded.factory().map_err(|e| pyroduct::capture!("{:?}", e))?;
        let output_schema = factory.factory.spec().func.output.clone();

        // 4. Create and restore DataManager for output_dir
        let mut dm = DataManager::new(factory.output_dir, output_schema);
        dm.restore().map_err(|e| pyroduct::capture!("{:?}", e))?;

        // 5. Get SQL Provider and execute query
        let provider = dm.sql_provider().map_err(|e| pyroduct::capture!("{:?}", e))?;
        let ctx = SessionContext::new();
        ctx.register_table("data", std::sync::Arc::new(provider))
            .capture("Failed to register table in DataFusion")?;

        let df = ctx
            .sql(sql_query)
            .await
            .capture("DataFusion SQL execution failed")?;
        let results = df
            .collect()
            .await
            .capture("Failed to collect query results")?;

        // 6. Serialize RecordBatches to Arrow IPC bytes using FileWriter
        let mut buffer = Vec::new();
        if !results.is_empty() {
            let schema = results[0].schema();
            let mut writer = arrow::ipc::writer::FileWriter::try_new(&mut buffer, &schema)
                .capture("Failed to create Arrow IPC FileWriter")?;
            for batch in results {
                writer
                    .write(&batch)
                    .capture("Failed to write RecordBatch to Arrow IPC")?;
            }
            writer
                .finish()
                .capture("Failed to finish Arrow IPC FileWriter")?;
        }

        Ok(buffer)
    }

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
        let db_entry = self.playbooks_manager.db.get_playbook(playbook_name).await?;
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
        let factory = loaded.factory().map_err(|e| pyroduct::capture!("{:?}", e))?;
        let output_schema = factory.factory.spec().func.output.clone();

        // 4. Create and restore DataManager for output_dir
        let mut dm = DataManager::new(factory.output_dir, output_schema);
        dm.restore().map_err(|e| pyroduct::capture!("{:?}", e))?;

        // 5. Get slice of data
        let batch_opt = dm
            .get_batch_slice(offset, limit)
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
}
