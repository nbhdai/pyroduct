use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::{ffi::host::{Capability, CapabilityLibrary, CapabilityLoading}, pipeline::wasm::PyroFactory};

use super::PipelineError;

// =============================================================================
// Config (deserialized from TOML / JSON)
// =============================================================================

/// Top-level pipeline configuration.
///
/// ```toml
/// [libraries.geocoder]
/// path = "target/release/libgeocoder.dylib"
///
/// [modules.enrich]
/// path = "target/wasm/enrich.wasm"
///
/// [modules.enrich.capabilities.GeocoderServer]
/// library = "geocoder"
/// api_key = "abc123"
///
/// [modules.enrich2]
/// path = "target/wasm/enrich2.wasm"
///
/// [modules.enrich2.capabilities.GeocoderServer]
/// library = "geocoder"
/// api_key = "different_key"
///
/// pipeline = ["enrich", "enrich2"]
/// ```
#[derive(Deserialize, Debug)]
pub struct PipelineConfig {
    /// Named shared libraries on disk. Each is loaded once.
    pub libraries: HashMap<String, PathBuf>,
    /// Named wasm modules, each with its own capability instances.
    pub modules: HashMap<String, ModuleConfig>,
    /// Ordered list of module names to execute.
    pub pipeline: Vec<String>,
}

/// A single wasm module in the pipeline.
#[derive(Deserialize, Debug)]
pub struct ModuleConfig {
    /// Path to the directory.
    pub path: PathBuf,
    /// Per-class capability configuration. Keys are class names.
    #[serde(default)]
    pub capabilities: HashMap<String, serde_json::Value>,
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
        // --- Phase 1: load shared libraries ------------------------------------
        let mut libraries: HashMap<String, CapabilityLibrary> = HashMap::new();

        #[cfg(target_os = "linux")]
        let lib_file = "lib.so";
        #[cfg(target_os = "macos")]
        let lib_file = "lib.dylib";
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        return Err(PipelineError::Config("Only macho and elf binaries are supported".to_string()));

        for (lib_name, path) in &config.libraries {
            let library = CapabilityLibrary::load(lib_name.clone(), &path.join("artifacts").join(lib_file)).map_err(|e| {
                PipelineError::Capability(CapabilityLoading::LibraryOpen {
                    path: path.display().to_string(),
                    reason: format!("library '{}': {}", lib_name, e),
                })
            })?;
            libraries.insert(lib_name.clone(), library);
        }

        // --- Phase 2: resolve modules & instantiate per-module capabilities ----
        let mut pipeline_steps = Vec::new();

        for module_name in &config.pipeline {
            let binary = fs::read(&mod_conf.path.join("artifacts").join("mod.wasm")).map_err(|e| {
                PipelineError::Config(format!(
                    "Failed to read WASM binary for module '{}' at '{}': {}",
                    module_name,
                    mod_conf.path.display(),
                    e
                ))
            })?;
            

            let mod_conf = config.modules.get(module_name).ok_or_else(|| {
                PipelineError::Config(format!(
                    "Pipeline references module '{}' which is not defined in [modules]",
                    module_name
                ))
            })?;

            let mut module_capabilities = Vec::new();

            for library in libraries.values() {
                let mut class_configs = HashMap::new();
                for (class_name, cap_conf) in &mod_conf.capabilities {
                    if library.capabilities.contains_key(class_name) {
                        class_configs.insert(class_name.clone(),  cap_conf.clone());
                    }
                }
                let capability = library.instantiate_from_config(&class_configs).await?;
                module_capabilities.push(capability);
            }



            pipeline_steps.push(Module {
                binary,
                capabilities: module_capabilities,
            });
        }

        Ok(Self {
            pipeline: pipeline_steps,
        })
    }
}