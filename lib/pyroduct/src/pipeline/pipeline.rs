use std::io;

use pyro_artifacts::{artifacts::{Artifacts, Module as ArtifactModule, Playbook}, cache::{CacheError, CacheManager}, environment::Environment};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use wasmtime::Engine;

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

impl PipelineConfig {
    pub async fn load_sources(&mut self, cache: &CacheManager) -> Result<(), WasmError> {
        for step in self.pipeline.values_mut() {
            match &step.module {
                Module::Source(_) => {},
                Module::Hash(hash) => {
                    let source = cache.get_source(hash).await?;
                    step.module = Module::Source(source);
                },
                Module::Path(path) => {
                    let env = Environment::new(path.clone()).await?;
                    let package = env.package(true).await?;
                    for a in package.iter() {
                        cache.write_artifacts(a).await?;
                    }
                    let mut source = None;
                    for artifact in package {
                        match artifact {
                            Artifacts::Module(ArtifactModule::Source(b)) => source = Some(b),
                            _ => {}
                        }
                    }
                    let source = source.ok_or(CacheError {
                        context: "Binary was not constructed".to_string(),
                        error: io::Error::new(io::ErrorKind::NotFound, "Not Found"),
                    })?;
                    step.module = Module::Source(source);
                },
            }
        }
        Ok(())
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
    pub async fn load(config: &PipelineConfig) -> Result<Self, PipelineError> {
        let mut wasm_config = wasmtime::Config::new();
        wasm_config.async_support(true);
        let engine = Engine::new(&wasm_config).map_err(|e| WasmError::EngineError(e.to_string()))?;
        let mut pipeline = Vec::new();
        for module in config.pipeline.values() {
            let module_factory = module.load_factory(&engine).await?;
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
