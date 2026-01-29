use futures::future::try_join_all;
use std::{path::PathBuf, sync::Mutex};

use crate::{
    ModIdentity, PyroductResult,
    errors::PyroductError,
    host::{ClassState, capability::Capabilities},
};

#[derive(serde::Deserialize, Debug)]
pub struct HarnessConfig {
    pub module_name: String,
    /// Path to the WASM module to run
    pub module: PathBuf,
    /// List of paths to dynamic library capabilities (.so/.dylib/.dll)
    pub capabilities: Vec<CapabilityDefinition>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(untagged)]
pub enum CapabilityDefinition {
    NoConfig(PathBuf),
    Config {
        path: PathBuf,
        config: serde_json::Value,
    },
}

impl CapabilityDefinition {
    pub fn config(&self) -> Option<&serde_json::Value> {
        match self {
            CapabilityDefinition::NoConfig(_) => None,
            CapabilityDefinition::Config { config, .. } => Some(&config),
        }
    }
}

pub struct HarnessState {
    pub module: ModIdentity,
    // Map capability index -> CapabilityState
    pub cap_states: Vec<ClassState>,

    /// Shared slot for an error that occurred during a host function call
    pub error_slot: Mutex<Option<PyroductError>>,

    pub capabilities: Capabilities,
}

impl HarnessState {
    pub fn take_error(&self) -> Option<PyroductError> {
        let mut guard = match self.error_slot.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                self.error_slot.clear_poison();
                poisoned.into_inner()
            }
        };

        guard.take()
    }

    pub fn set_error(&self, error: PyroductError) -> anyhow::Error {
        let ret_error = anyhow::anyhow!("Error: {error}");

        let mut guard = match self.error_slot.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                self.error_slot.clear_poison();
                poisoned.into_inner()
            }
        };

        *guard = Some(error);
        ret_error
    }

    pub async fn reset(&mut self) -> PyroductResult<()> {
        let resets: Vec<_> = self
            .cap_states
            .iter_mut()
            .map(|state| state.reset())
            .collect();

        try_join_all(resets).await?;
        Ok(())
    }
}
