use indexmap::IndexMap;
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
    pub pipeline: IndexMap<String, Playbook>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct LoadedPipelineConfig {
    pub pipeline: IndexMap<String, LoadedPlaybook>,
}

impl PipelineConfig {
    pub async fn load(self, cache: &CacheManager) -> Result<LoadedPipelineConfig, WasmError> {
        let mut loaded_pipeline = LoadedPipelineConfig {
            pipeline: IndexMap::new(),
        };
        for (key, playbook) in self.pipeline {
            let loaded_playbook = cache.load_playbook(playbook).await?;
            loaded_pipeline.pipeline.insert(key, loaded_playbook);
        }
        Ok(loaded_pipeline)
    }
}

impl LoadedPipelineConfig {
    pub fn factory(&self) -> Result<PipelineFactory, PipelineError> {
        Ok(PipelineFactory {
            pipeline: self
                .pipeline
                .values()
                .map(|p| PyroFactory::from_playbook(p).map_err(PipelineError::Wasm))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

// =============================================================================
// Runtime structures
// =============================================================================

/// A loaded pipeline definition: the ordered list of modules with their
/// individually instantiated capabilities.
pub struct PipelineFactory {
    pub pipeline: Vec<PyroFactory>,
}

// =============================================================================
// Loading
// =============================================================================

impl PipelineFactory {
    /// Build a pipeline from a fully-loaded `PipelineDef`.
    ///
    /// Creates one `PyroEngine` and one `PyroLinker` (with all capabilities
    /// linked), then compiles and instantiates each wasm module in order.
    pub async fn build(&self) -> Result<Pipeline, PipelineError> {
        let mut steps = Vec::with_capacity(self.pipeline.len());

        for (index, mod_factory) in self.pipeline.iter().enumerate() {
            tracing::debug!(index, "Building wasm module");
            let instance = mod_factory.instantiate().await?;
            steps.push(instance);
        }

        Ok(Pipeline { steps })
    }
}
