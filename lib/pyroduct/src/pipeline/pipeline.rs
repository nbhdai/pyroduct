use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::ffi::host::capability::{Capability, CapabilityLibrary, CapabilityLoading};

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
/// [libraries.geocoder.classes.GeocoderServer]
/// api_key = "abc123"
///
/// [libraries.geocoder.classes.ReverseServer]
/// api_key = "abc123"
///
/// [modules.enrich]
/// path = "target/wasm/enrich.wasm"
/// capabilities = ["GeocoderServer"]
///
/// pipeline = ["enrich"]
/// ```
#[derive(Deserialize, Debug)]
pub struct PipelineConfig {
    /// Named capability libraries, each containing one or more classes.
    pub libraries: HashMap<String, LibraryConfig>,
    /// Named wasm modules.
    pub modules: HashMap<String, ModuleConfig>,
    /// Ordered list of module names to execute.
    pub pipeline: Vec<String>,
}

/// A single shared library on disk that exposes one or more capability classes.
#[derive(Deserialize, Debug)]
pub struct LibraryConfig {
    /// Path to the compiled shared library (.dylib / .so / .dll).
    pub path: PathBuf,
    /// Per-class configuration. Keys are class names as exported by the library.
    #[serde(default)]
    pub classes: HashMap<String, serde_json::Value>,
}

/// A single wasm module in the pipeline.
#[derive(Deserialize, Debug)]
pub struct ModuleConfig {
    /// Path to the compiled .wasm binary.
    pub path: PathBuf,
    /// Capability class names this module imports.
    #[serde(default)]
    pub capabilities: Vec<String>,
}

// =============================================================================
// Runtime structures
// =============================================================================

/// A fully resolved module ready for instantiation.
#[derive(Debug)]
pub struct Module {
    pub binary: Vec<u8>,
    pub capabilities: Vec<String>,
}

/// A loaded pipeline definition: the ordered list of modules and the
/// instantiated capabilities they need.
pub struct PipelineDef {
    pub pipeline: Vec<Module>,
    pub capabilities: Vec<Capability>,
}

// =============================================================================
// Loading
// =============================================================================

impl PipelineDef {
    /// Loads and resolves a full pipeline from its configuration.
    ///
    /// 1. Loads each shared library and instantiates the requested classes.
    /// 2. Reads each wasm binary and validates that its capability references
    ///    resolve to a loaded class.
    /// 3. Returns the ordered pipeline steps alongside the live capabilities.
    pub async fn load(config: &PipelineConfig) -> Result<Self, PipelineError> {
        // --- Phase 1: load capability libraries & instantiate classes ----------
        let mut all_capabilities: Vec<Capability> = Vec::new();
        let mut known_classes: HashMap<String, ()> = HashMap::new();

        for (lib_name, lib_conf) in &config.libraries {
            let library = unsafe { CapabilityLibrary::load(&lib_conf.path) }.map_err(|e| {
                PipelineError::Capability(CapabilityLoading::LibraryOpen {
                    path: lib_conf.path.display().to_string(),
                    reason: format!("library '{}': {}", lib_name, e),
                })
            })?;

            let capability = library
                .instantiate_from_config(&lib_conf.classes)
                .await?;

            for class_name in capability.keys() {
                known_classes.insert(class_name.clone(), ());
            }

            all_capabilities.push(capability);
        }

        // --- Phase 2: resolve modules -----------------------------------------
        let mut pipeline_steps = Vec::new();

        for module_name in &config.pipeline {
            let mod_conf = config.modules.get(module_name).ok_or_else(|| {
                PipelineError::Config(format!(
                    "Pipeline references module '{}' which is not defined in [modules]",
                    module_name
                ))
            })?;

            // Validate that every capability the module wants is actually loaded
            for cap_name in &mod_conf.capabilities {
                if !known_classes.contains_key(cap_name) {
                    return Err(PipelineError::Config(format!(
                        "Module '{}' requires capability class '{}' which was not loaded by any library",
                        module_name, cap_name
                    )));
                }
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
                capabilities: mod_conf.capabilities.clone(),
            });
        }

        Ok(Self {
            pipeline: pipeline_steps,
            capabilities: all_capabilities,
        })
    }
}