use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::ffi::host::{Capability, CapabilityLibrary, CapabilityLoading};

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

/// Per-module reference to a capability class, with its own configuration.
#[derive(Deserialize, Debug)]
pub struct ModuleCapabilityConfig {
    /// Which library (key in `[libraries]`) provides this class.
    pub library: String,
    /// Remaining fields are the class-specific configuration.
    #[serde(flatten)]
    pub config: serde_json::Value,
}

/// A single wasm module in the pipeline.
#[derive(Deserialize, Debug)]
pub struct ModuleConfig {
    /// Path to the compiled .wasm binary.
    pub path: PathBuf,
    /// Per-class capability configuration. Keys are class names.
    #[serde(default)]
    pub capabilities: HashMap<String, ModuleCapabilityConfig>,
}

// =============================================================================
// Runtime structures
// =============================================================================

/// A fully resolved module ready for instantiation.
pub struct Module {
    pub binary: Vec<u8>,
    pub capabilities: Vec<Capability>,
}

/// A loaded pipeline definition: the ordered list of modules with their
/// individually instantiated capabilities.
pub struct PipelineDef {
    pub pipeline: Vec<Module>,
}

// =============================================================================
// Loading
// =============================================================================

impl PipelineDef {
    /// Loads and resolves a full pipeline from its configuration.
    ///
    /// 1. Loads each shared library once.
    /// 2. For each module, instantiates its own capability instances from the
    ///    referenced libraries with the module-specific configuration.
    /// 3. Returns the ordered pipeline steps with capabilities attached.
    pub async fn load(config: &PipelineConfig) -> Result<Self, PipelineError> {
        // --- Phase 1: load shared libraries ------------------------------------
        let mut libraries: HashMap<String, CapabilityLibrary> = HashMap::new();

        for (lib_name, path) in &config.libraries {
            let library = CapabilityLibrary::load(lib_name.clone(), &path).map_err(|e| {
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
            let mod_conf = config.modules.get(module_name).ok_or_else(|| {
                PipelineError::Config(format!(
                    "Pipeline references module '{}' which is not defined in [modules]",
                    module_name
                ))
            })?;

            // Group capability requests by library so we can call
            // instantiate_from_config once per library per module.
            let mut by_library: HashMap<String, HashMap<String, serde_json::Value>> =
                HashMap::new();

            for (class_name, cap_conf) in &mod_conf.capabilities {
                if !libraries.contains_key(&cap_conf.library) {
                    return Err(PipelineError::Config(format!(
                        "Module '{}' capability '{}' references library '{}' \
                         which is not defined in [libraries]",
                        module_name, class_name, cap_conf.library
                    )));
                }
                by_library
                    .entry(cap_conf.library.clone())
                    .or_default()
                    .insert(class_name.clone(), cap_conf.config.clone());
            }

            // Instantiate capabilities for this module
            let mut module_capabilities = Vec::new();
            for (lib_name, class_configs) in &by_library {
                let library = libraries.get(lib_name).unwrap();
                let capability = library.instantiate_from_config(class_configs).await?;
                module_capabilities.push(capability);
            }

            let binary = fs::read(&mod_conf.path).map_err(|e| {
                PipelineError::Config(format!(
                    "Failed to read WASM binary for module '{}' at '{}': {}",
                    module_name,
                    mod_conf.path.display(),
                    e
                ))
            })?;

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