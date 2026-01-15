//! Reporter Capability
//! 
//! Pattern: Host state only (server maintains state, client is stateless)

use capability_derive::*;

// ============================================================================
// SHARED TYPES - Always compiled for both targets
// ============================================================================

/// Configuration for the reporter server
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReporterConfig {
    pub max_history: usize,
}

impl Default for ReporterConfig {
    fn default() -> Self {
        Self { max_history: 10 }
    }
}

// ============================================================================
// CAPABILITY DEFINITION
// ============================================================================

/// The Reporter capability trait.
/// 
/// `#[capability]` generates:
/// - The trait itself
/// - For WASM: client module with `report()` function that calls host
/// - FFI type signatures
#[capability]
pub trait Reporter {
    /// Report a message and get back a processed result
    fn report(&mut self, message: String) -> String;
}

// ============================================================================
// SERVER IMPLEMENTATION - Only compiled for native targets
// ============================================================================

#[cfg(not(target_arch = "wasm32"))]
mod server {
    use super::*;
    use std::collections::VecDeque;

    /// The server-side implementation of Reporter.
    /// 
    /// `#[capability_server(service = Reporter)]` generates:
    /// - `__host_report()` FFI function that delegates to trait impl
    /// - `__plugin_init()`, `__plugin_drop()`, `__plugin_reset()`
    /// - `__capability_exports()` returning Vec<PluginExport>
    /// 
    /// The config is defined here so we can have different servers configured differently.
    #[capability_server(service = Reporter, config = ReporterConfig)]
    pub struct ReporterServer {
        logs: VecDeque<String>,
        max_history: usize,
    }

    /// Generated trait by the "capability_server" macro
    impl ReporterServerInit for ReporterServer {
        /// Called when config is None
        fn new() -> Self {
            tracing::info!("ReporterServer initialized with defaults");
            Self {
                logs: VecDeque::new(),
                max_history: 10,
            }
        }

        /// Called when config is Some
        fn with_config(config: ReporterConfig) -> Self {
            tracing::info!("ReporterServer initialized with config: {:?}", config);
            Self {
                logs: VecDeque::new(),
                max_history: config.max_history,
            }
        }

        /// Called between WASM invocations to reset state
        fn reset(&mut self) {
            self.logs.clear();
            tracing::debug!("ReporterServer state reset");
        }
    }

    impl Reporter for ReporterServer {
        fn report(&mut self, message: String) -> String {
            tracing::debug!("Processing message: '{}'", message);
            
            self.logs.push_back(message.clone());
            if self.logs.len() > self.max_history {
                self.logs.pop_front();
            }
            
            format!("Processed: '{}' | History: {:?}", message, self.logs)
        }
    }

    // Generate the plugin manifest entry point
    capability_export!(env = "reporter", ReporterServer);
}
