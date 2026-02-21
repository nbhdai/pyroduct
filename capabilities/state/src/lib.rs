
//! Provides basic stateful operations for demonstrating capability lifecycle and method calls.

use std::sync::atomic::{AtomicU64, Ordering};

#[pyroduct::magma]
pub struct CounterClient {
    pub start_value: u64,
}

#[pyroduct::config]
/// Required doc
pub struct CounterConfig {
    pub max_value: u64,
}

pub struct CounterServer {
    max_value: u64,
    call_count: AtomicU64,
}

#[pyroduct::capability]
impl CounterServer {
    type Client = CounterClient;
    type Config = CounterConfig;
    type Error = String;

    /// Initialize the counter server with optional max value config
    fn new(config: Option<CounterConfig>) -> Self {
        pyroduct::tracing::info!("Init");
        let max_value = config.map(|c| c.max_value).unwrap_or(u64::MAX);
        Self {
            max_value,
            call_count: AtomicU64::new(0),
        }
    }

    /// Reset call count between pipeline invocations
    fn reset(&mut self) {
        self.call_count.store(0, Ordering::SeqCst);
    }

    /// Validate client start value
    fn register(&self, client: &CounterClient) -> Result<(), String> {
        if client.start_value > self.max_value {
            return Err(format!(
                "start_value {} exceeds max_value {}",
                client.start_value, self.max_value
            ));
        }
        Ok(())
    }

    /// Increment and return the current count
    fn increment(&self, client: &CounterClient) -> Result<u64, String> {
        let count = self.call_count.fetch_add(1, Ordering::SeqCst);
        let value = client.start_value.saturating_add(count);
        if value > self.max_value {
            Err(format!("Counter exceeded max_value {}", self.max_value))
        } else {
            Ok(value)
        }
    }

    /// Get current count without incrementing
    fn get_count(&self, _client: &CounterClient) -> Result<u64, String> {
        Ok(self.call_count.load(Ordering::SeqCst))
    }

    /// Add a value to the counter
    fn add(&self, client: &CounterClient, value: u64) -> Result<u64, String> {
        let current = client.start_value.saturating_add(self.call_count.load(Ordering::SeqCst));
        let result = current.saturating_add(value);
        if result > self.max_value {
            Err(format!("Result {} exceeds max_value {}", result, self.max_value))
        } else {
            Ok(result)
        }
    }
}