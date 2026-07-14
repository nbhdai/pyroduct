use crate::module::interconnect::PlaybookInterconnect;
use std::collections::HashMap;
use std::sync::Arc;

use pyro_artifacts::{
    artifacts::PlaybookIdent,
    cache::{CacheManager, LoadedPlaybook, RemoteAddress},
    cargo::CapabilityIdent,
};
use serde::{Deserialize, Serialize};

use crate::{
    CapturedError, PyroError,
    format::log_wal::LogWal,
    module::{PyroFactory, WasmError},
    pipeline::{
        Pipeline,
        session::{SessionPipeline, SessionStatusManager},
        session_diff::SessionDiffPipeline,
    },
};

use super::PipelineError;

// =============================================================================
// Config (deserialized from TOML / JSON)
// =============================================================================

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PipelineConfig {
    pub playbook: PlaybookIdent,
    pub remote: HashMap<CapabilityIdent, RemoteAddress>,
    #[serde(default = "default_wal_capacity")]
    pub wal_capacity: usize,
    #[serde(default = "default_success_retention")]
    pub success_log_retention_secs: u64,
    #[serde(default = "default_error_retention")]
    pub error_log_retention_secs: u64,
    #[serde(default = "default_num_workers")]
    pub num_workers: usize,
    pub log_dir: std::path::PathBuf,
    pub input_dir: std::path::PathBuf,
    pub output_dir: std::path::PathBuf,
}

fn default_wal_capacity() -> usize {
    1000
}
fn default_success_retention() -> u64 {
    3600
}
fn default_error_retention() -> u64 {
    86400 * 7
}
fn default_num_workers() -> usize {
    15
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct LoadedPipelineConfig {
    pub playbook: LoadedPlaybook,
    pub wal_capacity: usize,
    pub success_log_retention_secs: u64,
    pub error_log_retention_secs: u64,
    pub num_workers: usize,
}

impl PipelineConfig {
    pub async fn load(self, cache: &CacheManager) -> Result<LoadedPipelineConfig, WasmError> {
        let loaded_playbook = cache
            .load_playbook(
                self.playbook,
                self.remote,
                self.log_dir,
                self.input_dir,
                self.output_dir,
                self.num_workers,
            )
            .await?;
        Ok(LoadedPipelineConfig {
            playbook: loaded_playbook,
            wal_capacity: self.wal_capacity,
            success_log_retention_secs: self.success_log_retention_secs,
            error_log_retention_secs: self.error_log_retention_secs,
            num_workers: self.num_workers,
        })
    }
}

impl LoadedPipelineConfig {
    pub fn factory(&self) -> Result<PipelineFactory, PipelineError> {
        Ok(PipelineFactory {
            factory: PyroFactory::from_playbook(&self.playbook).map_err(PipelineError::Wasm)?,
            wal_capacity: self.wal_capacity,
            success_log_retention_secs: self.success_log_retention_secs,
            error_log_retention_secs: self.error_log_retention_secs,
            num_workers: self.num_workers,
            log_dir: self.playbook.log_dir.clone(),
            input_dir: self.playbook.input_dir.clone(),
            output_dir: self.playbook.output_dir.clone(),
        })
    }
}

// =============================================================================
// Runtime structures
// =============================================================================

/// A loaded pipeline definition for a single playbook.
pub struct PipelineFactory {
    pub factory: PyroFactory,
    pub log_dir: std::path::PathBuf,
    pub input_dir: std::path::PathBuf,
    pub output_dir: std::path::PathBuf,
    pub wal_capacity: usize,
    pub success_log_retention_secs: u64,
    pub error_log_retention_secs: u64,
    pub num_workers: usize,
}

impl PipelineFactory {
    /// Configure the factory with an interconnect.
    pub fn with_interconnect(mut self, interconnect: Arc<dyn PlaybookInterconnect>) -> Self {
        self.factory.set_interconnect(interconnect);
        self
    }
}

// =============================================================================
// Loading
// =============================================================================

impl PipelineFactory {
    /// Build a pipeline from a fully-loaded `PipelineFactory`.
    ///
    /// Compiles and instantiates the single wasm module, configuring it with WAL.
    pub async fn build(&self) -> Result<Pipeline, PipelineError> {
        tracing::debug!(
            num_workers = self.num_workers,
            "Building wasm module(s) for playbook"
        );
        let mut shards = Vec::with_capacity(self.num_workers);
        for _ in 0..self.num_workers {
            let instance = self.factory.instantiate().await?;
            shards.push(tokio::sync::Mutex::new(instance));
        }
        let input_schema = self.factory.spec().func.input.clone();
        let output_schema = self.factory.spec().func.output.clone();

        Ok(Pipeline {
            shards,
            success_log_retention_secs: self.success_log_retention_secs,
            error_log_retention_secs: self.error_log_retention_secs,
            log_manager: tokio::sync::Mutex::new(
                LogWal::open(self.log_dir.clone(), self.wal_capacity)
                    .await
                    .map_err(|io| {
                        PyroError::local_io(
                            CapturedError::new("Unable to make the log wal").with_source(io),
                        )
                    })?,
            ),
            input_manager: {
                let dm = super::data::DataManager::new(
                    self.input_dir.clone(),
                    input_schema,
                    self.wal_capacity,
                );
                dm.set_metadata_prefix("_input_meta").await;
                dm
            },
            output_manager: {
                let dm = super::data::DataManager::new(
                    self.output_dir.clone(),
                    output_schema,
                    self.wal_capacity,
                );
                dm.set_metadata_prefix("_output_meta").await;
                dm
            },
            callbacks: tokio::sync::Mutex::new(Vec::new()),
        })
    }

    /// Build a session pipeline from a fully-loaded `PipelineFactory`.
    ///
    /// Compiles and instantiates the single wasm module, configuring it with WAL.
    pub async fn build_session(&self) -> Result<SessionPipeline, PipelineError> {
        tracing::debug!(
            num_workers = self.num_workers,
            "Building wasm module(s) for session playbook"
        );
        let mut shards = Vec::with_capacity(self.num_workers);
        for _ in 0..self.num_workers {
            let instance = self.factory.instantiate().await?;
            shards.push(tokio::sync::Mutex::new(instance));
        }
        let input_schema = self.factory.spec().func.input.clone();

        // Session list element type is the raw value type — Str for String,
        // Group({role,content}) for ChatMessage, etc. Not wrapped in field names.
        let element_type = input_schema
            .fields
            .last()
            .map(|f| f.data_type.clone().into_owned())
            .unwrap_or(crate::format::value::PyroType::Null);

        let session_type =
            crate::format::value::PyroType::List(Box::new(element_type), true);
        let output_schema =
            crate::format::value::PyroSchema::new(vec![crate::format::value::PyroField::new(
                "session",
                session_type,
                false,
            )]);

        let session_status_manager = SessionStatusManager::new(&self.output_dir)?;
        let spec = self.factory.spec().clone();

        Ok(SessionPipeline {
            shards,
            spec: std::sync::Arc::new(spec),
            success_log_retention_secs: self.success_log_retention_secs,
            error_log_retention_secs: self.error_log_retention_secs,
            log_manager: tokio::sync::Mutex::new(
                LogWal::open(self.log_dir.clone(), self.wal_capacity)
                    .await
                    .map_err(|io| {
                        PyroError::local_io(
                            CapturedError::new("Unable to make the log wal").with_source(io),
                        )
                    })?,
            ),
            output_manager: super::data::DataManager::new(
                self.output_dir.clone(),
                output_schema,
                self.wal_capacity,
            ),
            log_dir: self.log_dir.clone(),
            output_dir: self.output_dir.clone(),
            wal_capacity: self.wal_capacity,
            max_active_sessions: 10 * self.num_workers,
            active_sessions: tokio::sync::Mutex::new(std::collections::HashMap::new()),
            lru_order: tokio::sync::Mutex::new(std::collections::VecDeque::new()),
            callbacks: tokio::sync::Mutex::new(Vec::new()),
            session_status_manager,
        })
    }

    /// Build a session pipeline from a fully-loaded `PipelineFactory`.
    ///
    /// Compiles and instantiates the single wasm module, configuring it with WAL.
    pub async fn build_session_diff(&self) -> Result<SessionDiffPipeline, PipelineError> {
        tracing::debug!(
            num_workers = self.num_workers,
            "Building wasm module(s) for session_diff playbook"
        );
        let mut shards = Vec::with_capacity(self.num_workers);
        for _ in 0..self.num_workers {
            let instance = self.factory.instantiate().await?;
            shards.push(tokio::sync::Mutex::new(instance));
        }
        let input_schema = self.factory.spec().func.input.clone();
        let output_schema = self.factory.spec().func.output.clone();

        // In Session Diff, the "inputs" list group type only contains the "input" field
        // (which is the last field of input_schema.fields), marked as nullable.
        let mut input_group_fields = Vec::new();
        if let Some(mut last_input_field) = input_schema.fields.last().cloned() {
            last_input_field.nullable = true;
            input_group_fields.push(last_input_field);
        }

        let last_input_type = input_schema
            .fields
            .last()
            .map(|f| f.data_type.clone())
            .unwrap_or(crate::format::value::PyroType::Null);
        let mut output_group_fields = Vec::new();
        for mut out_field in output_schema.fields.iter().cloned() {
            if let crate::format::value::PyroType::Group(ref fields) = out_field.data_type
                && fields.is_empty()
            {
                out_field.data_type = last_input_type.clone();
            }
            output_group_fields.push(out_field);
        }

        let inputs_type = crate::format::value::PyroType::List(
            Box::new(crate::format::value::PyroType::Group(
                std::borrow::Cow::Owned(input_group_fields),
            )),
            false,
        );
        let outputs_type = crate::format::value::PyroType::List(
            Box::new(crate::format::value::PyroType::Group(
                std::borrow::Cow::Owned(output_group_fields),
            )),
            false,
        );
        let overall_output_schema = crate::format::value::PyroSchema::new(vec![
            crate::format::value::PyroField::new("inputs", inputs_type, false),
            crate::format::value::PyroField::new("outputs", outputs_type, false),
        ]);

        let session_status_manager = SessionStatusManager::new(&self.output_dir)?;
        let spec = self.factory.spec().clone();

        Ok(SessionDiffPipeline {
            shards,
            spec: std::sync::Arc::new(spec),
            success_log_retention_secs: self.success_log_retention_secs,
            error_log_retention_secs: self.error_log_retention_secs,
            log_manager: tokio::sync::Mutex::new(
                LogWal::open(self.log_dir.clone(), self.wal_capacity)
                    .await
                    .map_err(|io| {
                        PyroError::local_io(
                            CapturedError::new("Unable to make the log wal").with_source(io),
                        )
                    })?,
            ),
            output_manager: super::data::DataManager::new(
                self.output_dir.clone(),
                overall_output_schema,
                self.wal_capacity,
            ),
            log_dir: self.log_dir.clone(),
            output_dir: self.output_dir.clone(),
            wal_capacity: self.wal_capacity,
            max_active_sessions: 10 * self.num_workers,
            active_sessions: tokio::sync::Mutex::new(std::collections::HashMap::new()),
            lru_order: tokio::sync::Mutex::new(std::collections::VecDeque::new()),
            callbacks: tokio::sync::Mutex::new(Vec::new()),
            session_status_manager,
        })
    }
}
