use std::path::Path;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{
    module::{ModuleConfig, PyroFactory},
    pipeline::Pipeline,
};

use super::PipelineError;

// =============================================================================
// Config (deserialized from TOML / JSON)
// =============================================================================

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PipelineConfig {
    pub pipeline: IndexMap<String, ModuleConfig>,
}

impl PipelineConfig {
    pub fn repair_relative(&mut self, config_dir: &Path) {
        // Resolve relative paths
        for module in self.pipeline.values_mut() {
            for path in module.libraries.iter_mut() {
                if path.is_relative() {
                    *path = config_dir.join(&path);
                }
            }
        }
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
    /// Loads and resolves a full pipeline from its configuration and corresponding WASm binaries.
    pub async fn load(
        config: &PipelineConfig,
        wasm_binaries: &std::collections::HashMap<String, Vec<u8>>,
    ) -> Result<Self, PipelineError> {
        let mut pipeline = Vec::new();
        for (name, module) in &config.pipeline {
            let wasm = wasm_binaries.get(name).ok_or_else(|| {
                PipelineError::Config(format!("Missing WASM binary for module '{}'", name))
            })?;
            let module_factory = module.load_factory(wasm).await?;
            pipeline.push(module_factory);
        }

        Ok(Self { pipeline })
    }

    /// Build a pipeline from a fully-loaded `PipelineDef`.
    ///
    /// Creates one `PyroEngine` and one `PyroLinker` (with all capabilities
    /// linked), then compiles and instantiates each wasm module in order.
    pub async fn build(&mut self) -> Result<Pipeline, PipelineError> {
        let mut steps = Vec::with_capacity(self.pipeline.len());

        for (index, mod_factory) in self.pipeline.iter_mut().enumerate() {
            tracing::debug!(index, "Building wasm module");
            let instance = mod_factory.instantiate().await?;
            steps.push(instance);
        }

        Ok(Pipeline { steps })
    }
}
