use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use anyhow::{Context, Result};
use crate::playbooks::PlaybooksManager;
use pyro_artifacts::cache::CacheManager;
use pyroduct::pipeline::factory::PipelineConfig;
use pyroduct::pipeline::data::DataManager;
use datafusion::prelude::SessionContext;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum DataRequest {
    GetBaseDir,
    QueryPlaybook {
        playbook_id: Uuid,
        sql_query: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum DataResponse {
    BaseDir {
        path: String,
    },
    QueryResult {
        ipc_bytes: Vec<u8>,
    },
    Error {
        message: String,
    },
}

/// Manages data access for the PyroDaemon.
#[derive(Clone)]
pub struct DaemonDataManager {
    base_dir: PathBuf,
    playbooks_manager: PlaybooksManager,
}

impl DaemonDataManager {
    pub fn new(base_dir: PathBuf, playbooks_manager: PlaybooksManager) -> Self {
        Self {
            base_dir,
            playbooks_manager,
        }
    }

    pub async fn handle_request(&self, req: DataRequest) -> DataResponse {
        match req {
            DataRequest::GetBaseDir => DataResponse::BaseDir {
                path: self.base_dir().to_string_lossy().to_string(),
            },
            DataRequest::QueryPlaybook { playbook_id, sql_query } => {
                match self.query_playbook_data(playbook_id, &sql_query).await {
                    Ok(ipc_bytes) => DataResponse::QueryResult { ipc_bytes },
                    Err(e) => DataResponse::Error {
                        message: format!("SQL query failed: {:?}", e),
                    },
                }
            }
        }
    }

    async fn query_playbook_data(&self, playbook_id: Uuid, sql_query: &str) -> Result<Vec<u8>> {
        // 1. Find the playbook worker
        let active = self.playbooks_manager.list_playbooks().await;
        let worker_info = active
            .into_iter()
            .find(|(id, _, _, _)| *id == playbook_id)
            .context("Playbook worker not found")?;

        let config_path = worker_info.1;

        // 2. Load the pipeline config
        let config_str = tokio::fs::read_to_string(&config_path)
            .await
            .context("Failed to read playbook pipeline config")?;

        let pipeline_config: PipelineConfig = match config_path
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
            _ => anyhow::bail!("Unknown playbook config extension"),
        };

        // 3. Load factory to get output schema
        let cache = CacheManager::from_env()
            .await
            .context("Failed to initialize CacheManager")?;

        let loaded = pipeline_config.load(&cache).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;
        let factory = loaded.factory().map_err(|e| anyhow::anyhow!("{:?}", e))?;
        let output_schema = factory.factory.spec().func.output.clone();

        // 4. Create and restore DataManager for output_dir
        let mut dm = DataManager::new(factory.output_dir, output_schema);
        dm.restore().map_err(|e| anyhow::anyhow!("{:?}", e))?;

        // 5. Get SQL Provider and execute query
        let provider = dm.sql_provider().map_err(|e| anyhow::anyhow!("{:?}", e))?;
        let ctx = SessionContext::new();
        ctx.register_table("data", std::sync::Arc::new(provider))
            .context("Failed to register table in DataFusion")?;

        let df = ctx.sql(sql_query).await.context("DataFusion SQL execution failed")?;
        let results = df.collect().await.context("Failed to collect query results")?;

        // 6. Serialize RecordBatches to Arrow IPC bytes using FileWriter
        let mut buffer = Vec::new();
        if !results.is_empty() {
            let schema = results[0].schema();
            let mut writer = arrow::ipc::writer::FileWriter::try_new(&mut buffer, &schema)
                .context("Failed to create Arrow IPC FileWriter")?;
            for batch in results {
                writer.write(&batch).context("Failed to write RecordBatch to Arrow IPC")?;
            }
            writer.finish().context("Failed to finish Arrow IPC FileWriter")?;
        }

        Ok(buffer)
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }
}
