//! Test capability 2: Async string transform capability
//!
//! Provides async operations for testing async capability lifecycle and method calls.

use std::collections::HashMap;
use std::sync::Mutex;

#[pyroduct::interface_item]
pub struct TransformClient {
    pub prefix: String,
}

#[pyroduct::config]
/// Required doc
pub struct TransformConfig {
    pub uppercase: bool,
    pub suffix: String,
}

pub struct TransformServer {
    uppercase: bool,
    suffix: String,
    transform_log: Mutex<Vec<String>>,
}

#[pyroduct::capability]
impl TransformServer {
    type Client = TransformClient;
    type Config = TransformConfig;
    type Error = String;

    /// Initialize with async setup
    async fn new(config: Option<TransformConfig>) -> Self {
        let config = config.unwrap_or(TransformConfig {
            uppercase: false,
            suffix: String::new(),
        });
        Self {
            uppercase: config.uppercase,
            suffix: config.suffix,
            transform_log: Mutex::new(Vec::new()),
        }
    }

    /// Clear transform log between invocations
    async fn reset(&mut self) {
        if let Ok(mut log) = self.transform_log.lock() {
            log.clear();
        }
    }

    /// Validate client prefix
    fn new_client(&self, client: &TransformClient) -> Result<(), String> {
        if client.prefix.len() > 100 {
            return Err("Prefix too long".to_string());
        }
        Ok(())
    }

    /// Transform a string with prefix, optional uppercase, and suffix
    async fn transform(&self, client: &TransformClient, input: String) -> Result<String, String> {
        let mut result = format!("{}{}", client.prefix, input);
        if self.uppercase {
            result = result.to_uppercase();
        }
        result.push_str(&self.suffix);

        if let Ok(mut log) = self.transform_log.lock() {
            log.push(result.clone());
        }

        Ok(result)
    }

    /// Get the number of transforms performed
    fn get_transform_count(&self, _client: &TransformClient) -> Result<usize, String> {
        self.transform_log
            .lock()
            .map(|log| log.len())
            .map_err(|_| "Lock poisoned".to_string())
    }

    /// Batch transform multiple strings
    async fn batch_transform(
        &self,
        client: &TransformClient,
        inputs: Vec<String>,
    ) -> Result<Vec<String>, String> {
        let mut results = Vec::with_capacity(inputs.len());
        for input in inputs {
            let mut result = format!("{}{}", client.prefix, input);
            if self.uppercase {
                result = result.to_uppercase();
            }
            result.push_str(&self.suffix);
            results.push(result);
        }

        if let Ok(mut log) = self.transform_log.lock() {
            log.extend(results.clone());
        }

        Ok(results)
    }

    /// Echo back input (for testing passthrough)
    fn echo(&self, _client: &TransformClient, input: String) -> Result<String, String> {
        Ok(input)
    }
}