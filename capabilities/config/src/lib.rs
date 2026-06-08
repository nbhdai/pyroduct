//! Provides async operations for testing async capability lifecycle and method calls.
use std::sync::Mutex;
use pyroduct::CapturedError;

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

    /// Initialize with async setup
    async fn new(config: Option<TransformConfig>) -> Result<Self> {
        pyroduct::tracing::info!("Init");
        let config = config.unwrap_or(TransformConfig {
            uppercase: false,
            suffix: String::new(),
        });
        pyroduct::tracing::info!(uppercase=config.uppercase, suffix=config.suffix, "Init config");
        Ok(Self {
            uppercase: config.uppercase,
            suffix: config.suffix,
            transform_log: Mutex::new(Vec::new()),
        })
    }

    /// Clear transform log between invocations
    async fn reset(&mut self) -> Result<(), CapturedError> {
        if let Ok(mut log) = self.transform_log.lock() {
            log.clear();
        }
        Ok(())
    }

    /// Validate client prefix
    fn register(&self, client: &TransformClient) -> Result<(), CapturedError> {
        pyroduct::tracing::info!("register");
        if client.prefix.len() > 100 {
            pyroduct::bail!("Prefix too long");
        }
        Ok(())
    }

    /// Transform a string with prefix, optional uppercase, and suffix
    async fn transform(&self, client: &TransformClient, input: String) -> Result<String, CapturedError> {
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
    fn get_transform_count(&self, _client: &TransformClient) -> Result<usize, CapturedError> {
        pyroduct::tracing::info!("count");
        self.transform_log
            .lock()
            .map(|log| log.len())
            .map_err(|_| pyroduct::capture!("Lock poisoned"))
    }

    /// Batch transform multiple strings
    async fn batch_transform(
        &self,
        client: &TransformClient,
        inputs: Vec<String>,
    ) -> Result<Vec<String>, CapturedError> {
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
    fn echo(&self, _client: &TransformClient, input: String) -> Result<String, CapturedError> {
        Ok(input)
    }
}