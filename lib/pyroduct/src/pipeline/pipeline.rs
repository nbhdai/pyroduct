use pyro_artifacts::{
    artifacts::Playbook,
    cache::{CacheManager, LoadedPlaybook},
};
use serde::{Deserialize, Serialize};

use crate::{
    module::{PyroFactory, WasmError},
    pipeline::Pipeline,
};

use super::PipelineError;

// =============================================================================
// Config (deserialized from TOML / JSON)
// =============================================================================

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PipelineConfig {
    pub playbook: Playbook,
    #[serde(default = "default_wal_capacity")]
    pub wal_capacity: usize,
    #[serde(default = "default_success_retention")]
    pub success_log_retention_secs: u64,
    #[serde(default = "default_error_retention")]
    pub error_log_retention_secs: u64,
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct LoadedPipelineConfig {
    pub playbook: LoadedPlaybook,
    pub wal_capacity: usize,
    pub success_log_retention_secs: u64,
    pub error_log_retention_secs: u64,
    pub output_dir: std::path::PathBuf,
}

impl PipelineConfig {
    pub async fn load(self, cache: &CacheManager) -> Result<LoadedPipelineConfig, WasmError> {
        let loaded_playbook = cache.load_playbook(self.playbook).await?;
        Ok(LoadedPipelineConfig {
            playbook: loaded_playbook,
            wal_capacity: self.wal_capacity,
            success_log_retention_secs: self.success_log_retention_secs,
            error_log_retention_secs: self.error_log_retention_secs,
            output_dir: self.output_dir,
        })
    }
}

impl LoadedPipelineConfig {
    pub fn factory(&self) -> Result<PipelineFactory, PipelineError> {
        Ok(PipelineFactory {
            factory: PyroFactory::from_playbook(&self.playbook).map_err(PipelineError::Wasm)?,
            success_log_retention_secs: self.success_log_retention_secs,
            error_log_retention_secs: self.error_log_retention_secs,
            output_dir: self.output_dir.clone(),
        })
    }
}

// =============================================================================
// Runtime structures
// =============================================================================

/// A loaded pipeline definition for a single playbook.
pub struct PipelineFactory {
    pub factory: PyroFactory,
    pub output_dir: std::path::PathBuf,
    pub success_log_retention_secs: u64,
    pub error_log_retention_secs: u64,
}

// =============================================================================
// Loading
// =============================================================================

impl PipelineFactory {
    /// Build a pipeline from a fully-loaded `PipelineFactory`.
    ///
    /// Compiles and instantiates the single wasm module, configuring it with WAL.
    pub async fn build(&self) -> Result<Pipeline, PipelineError> {
        tracing::debug!("Building wasm module for playbook");
        let instance = self.factory.instantiate().await?;
        let input_schema = self.factory.spec().func.input.clone();
        let output_schema = self.factory.spec().func.output.clone();

        Ok(Pipeline {
            step: instance,
            success_log_retention_secs: self.success_log_retention_secs,
            error_log_retention_secs: self.error_log_retention_secs,
            output_dir: self.output_dir.clone(),
            input_manager: super::data::DataManager::new(self.output_dir.clone(), input_schema),
            output_manager: super::data::DataManager::new(self.output_dir.clone(), output_schema),
        })
    }
}
