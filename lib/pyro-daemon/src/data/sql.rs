use super::DaemonDataManager;
use crate::Result;
use datafusion::prelude::SessionContext;
use pyro_artifacts::cache::CacheManager;
use pyroduct::Capture;
use pyroduct::pipeline::data::DataManager;
use pyroduct::pipeline::factory::PipelineConfig;

impl DaemonDataManager {
    pub async fn query_playbook_data(
        &self,
        playbook_name: &str,
        sql_query: &str,
    ) -> Result<Vec<u8>> {
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
        let factory = loaded
            .factory()
            .map_err(|e| pyroduct::capture!("{:?}", e))?;
        let input_schema = factory.factory.spec().func.input.clone();
        let output_schema = factory.factory.spec().func.output.clone();

        // 4. Create and restore DataManagers for input_dir and output_dir.
        //    Prefixes must be set before restore() so that WAL recovery uses
        //    the same schema that the pipeline wrote the data with.
        let input_dm = DataManager::new(factory.input_dir, input_schema, factory.wal_capacity);
        input_dm.set_metadata_prefix("_input_meta").await;
        input_dm
            .restore()
            .await
            .map_err(|e| pyroduct::capture!("{:?}", e))?;

        let output_dm = DataManager::new(factory.output_dir, output_schema, factory.wal_capacity);
        output_dm.set_metadata_prefix("_output_meta").await;
        output_dm
            .restore()
            .await
            .map_err(|e| pyroduct::capture!("{:?}", e))?;

        // 5. Register both tables and execute query.
        //    - "inputs"  → input WAL   (schema: module input)
        //    - "outputs" → output WAL  (schema: module output)
        //    - "data"    → alias for outputs (backward-compatible)
        //
        //    Join key: inputs._input_meta.index = outputs._output_meta.index
        let input_provider = input_dm
            .sql_provider()
            .await
            .map_err(|e| pyroduct::capture!("{:?}", e))?;
        let output_provider = output_dm
            .sql_provider()
            .await
            .map_err(|e| pyroduct::capture!("{:?}", e))?;
        let output_provider = std::sync::Arc::new(output_provider);

        let ctx = SessionContext::new();
        ctx.register_table("inputs", std::sync::Arc::new(input_provider))
            .capture("Failed to register 'inputs' table in DataFusion")?;
        ctx.register_table("outputs", output_provider.clone())
            .capture("Failed to register 'outputs' table in DataFusion")?;
        // Keep "data" pointing at outputs for backward compatibility
        ctx.register_table("data", output_provider)
            .capture("Failed to register 'data' table in DataFusion")?;

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
}
