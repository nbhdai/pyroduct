use std::path::Path;

use serde::Deserialize;

use crate::{
    module::{ModuleConfig, PyroFactory},
    pipeline::Pipeline,
};

use super::PipelineError;

// =============================================================================
// Config (deserialized from TOML / JSON)
// =============================================================================

#[derive(Deserialize, Debug)]
pub struct PipelineConfig {
    pub pipeline: Vec<ModuleConfig>,
}

impl PipelineConfig {
    pub fn repair_relative(&mut self, config_dir: &Path) {
        // Resolve relative paths
        for module in self.pipeline.iter_mut() {
            for path in module.libraries.iter_mut() {
                if path.is_relative() {
                    *path = config_dir.join(&path);
                }
            }
            if module.path.is_relative() {
                module.path = config_dir.join(&module.path);
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
    /// Loads and resolves a full pipeline from its configuration.
    ///
    /// 1. Loads each shared library once.
    /// 2. For each module, instantiates its own capability instances from the
    ///    referenced libraries with the module-specific configuration.
    /// 3. Returns the ordered pipeline steps with capabilities attached.
    pub async fn load(config: &PipelineConfig) -> Result<Self, PipelineError> {
        let mut pipeline = Vec::new();
        for module in &config.pipeline {
            let module_factory = module.load_factory().await?;
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
