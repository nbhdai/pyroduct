use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::errors::PyroductError;
use crate::host::capability::Capabilities;
use crate::{ModIdentity, PyroductResult};

// --- Public Structs (Existing) ---

#[derive(Debug)]
pub struct Module {
    pub ident: ModIdentity,
    pub binary: Vec<u8>,
    pub capabilities: Vec<CapabilityDef>,
}

#[derive(serde::Deserialize, Debug)]
pub struct CapabilityDef {
    pub name: String,
    pub config: Option<serde_json::Value>,
}

pub struct PipelineDef {
    pub pipeline: Vec<Module>,
}
/// This is the internal configuration.
///
/// It should point to the saved artifacts in the pyroduct cache directory.
#[derive(Deserialize)]
pub struct PipelineConfig {
    pub capabilities: HashMap<String, CapabilityConfig>,
    pub modules: HashMap<String, ModuleConfig>,
    pub pipeline: Vec<String>,
}

#[derive(Deserialize)]
pub struct CapabilityConfig {
    pub path: PathBuf,
    #[serde(flatten)]
    pub config: serde_json::Value,
}

#[derive(Deserialize)]
pub struct ModuleConfig {
    pub path: PathBuf,
    pub capabilities: Vec<String>,
}

// --- Implementation ---

impl PipelineDef {
    /// Loads a pipeline definition from a configuration
    pub fn load(config: &PipelineConfig, caps_registry: &mut Capabilities) -> PyroductResult<Self> {
        for (name, cap_conf) in &config.capabilities {
            caps_registry.load(name, &cap_conf.path)?;
        }

        // 3. Build the Pipeline Steps
        let mut pipeline_steps = Vec::new();

        for module_name in &config.pipeline {
            let mod_conf = config.modules.get(module_name).ok_or_else(|| {
                PyroductError::from_infrastructure(format!(
                    "Pipeline references module '{}' which is not defined in [modules]",
                    module_name
                ))
            })?;
            let ident = ModIdentity::from(&mod_conf.path);

            // Read Wasm Binary
            let binary = fs::read(&mod_conf.path).map_err(|e| {
                PyroductError::from_infrastructure(format!(
                    "Failed to read WASM binary for module '{}' at '{}': {}",
                    module_name,
                    mod_conf.path.display(),
                    e
                ))
            })?;

            let mut mod_capabilities = Vec::new();
            for cap_name in &mod_conf.capabilities {
                let cap_config = config.capabilities.get(cap_name).ok_or_else(|| {
                    PyroductError::from_infrastructure(format!(
                        "Module '{}' requests capability '{}' which is not defined in [capabilities]",
                        module_name, cap_name
                    ))
                })?;

                mod_capabilities.push(CapabilityDef {
                    name: cap_name.clone(),
                    config: Some(cap_config.config.clone()),
                });
            }

            pipeline_steps.push(Module {
                ident,
                binary,
                capabilities: mod_capabilities,
            });
        }

        Ok(Self {
            pipeline: pipeline_steps,
        })
    }
}
