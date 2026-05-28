//! Provides async operations for testing async capability lifecycle and method calls.
use std::sync::Mutex;

#[pyroduct::magma]
pub struct TransformClient {
    pub prefix: String,
}

#[pyroduct::config]
/// Required doc
pub struct TransformConfig {
    pub uppercase: bool,
    pub suffix: String,
}

pub struct Transform {
    uppercase: bool,
    suffix: String,
    transform_log: Mutex<Vec<String>>,
}

#[pyroduct::capability]
impl Transform {
    type Client = TransformClient;
    type Config = TransformConfig;
    type Error = String;

    /// Initialize with async setup
    async fn new(config: Option<TransformConfig>) -> Self {
        pyroduct::tracing::info!("Init");
        let config = config.unwrap_or(TransformConfig {
            uppercase: false,
            suffix: String::new(),
        });
        pyroduct::tracing::info!(uppercase=config.uppercase, suffix=config.suffix, "Init config");
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
    fn register(&self, client: &TransformClient) -> Result<(), String> {
        pyroduct::tracing::info!("register");
        if client.prefix.len() > 100 {
            return Err("Prefix too long".to_string());
        }
        Ok(())
    }

    /// Transform a string with prefix, optional uppercase, and suffix
    async fn transform(&self, client: &TransformClient, input: String) -> Result<String, String> {
        pyroduct::tracing::info!(input, prefix=client.prefix, "transform prefix");
        let mut result = format!("{}{}", client.prefix, input);
        if self.uppercase {
            result = result.to_uppercase();
        }
        pyroduct::tracing::info!(suffix=self.suffix, "transform suffix");
        result.push_str(&self.suffix);

        if let Ok(mut log) = self.transform_log.lock() {
            log.push(result.clone());
        }

        pyroduct::tracing::info!(result, "transform RETURN");
        Ok(result)
    }

    /// Get the number of transforms performed
    fn get_transform_count(&self, _client: &TransformClient) -> Result<usize, String> {
        pyroduct::tracing::info!("count");
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
        pyroduct::tracing::info!("batch_transform");
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