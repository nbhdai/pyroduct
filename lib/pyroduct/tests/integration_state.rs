use pyro_artifacts::{
    artifacts::{ModuleDependencies, ModuleSource, Playbook},
    build::Builder,
    cache::CacheManager,
    cargo::ResolvedCapability,
};
use pyroduct::{PyroRow, pipeline::PipelineConfig};
use std::collections::{BTreeMap, HashMap};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

const CODE: &'static str = r#"
//! Test module 1: Uses test_cap1 counter capability
//!
//! Simple module that increments a counter and returns the result.

use state::{CounterClient, CounterClientMethods};

#[pyroduct::module(output = (count, incremented))]
pub fn call(input: &str) -> Result<(u64, u64)> {
    let start: u64 = input.parse().map_err(|e| format!("Parse error: {}", e))?;

    let client = CounterClient { start_value: start }.register()?;

    let count = client.get_count()?;
    let incremented = client.increment()?;

    Ok((count, incremented))
}
"#;

/// Test that capability state is preserved across multiple calls to the same module instance.
#[tokio::test]
async fn test_capability_state_preservation() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "trace,cranelift_frontend=off,cranelift_codegen=off,wasmtime=off".into()
    });

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).pretty())
        .with(filter)
        .init();
    let cache = std::sync::Arc::new(CacheManager::from_env().await.unwrap());
    let builder = Builder::from_env(cache.clone()).await.unwrap();

    let source = ModuleSource {
        dependencies: ModuleDependencies {
            dependencies: BTreeMap::new(),
            capabilities: vec![ResolvedCapability {
                package: "state".to_string(),
                author: "nbhdai".to_string(),
                version: "0.1.0".to_string(),
            }],
        },
        source: CODE.to_string(),
    };

    let binary = builder
        .compile(&source)
        .await
        .expect("Valid module should compile");

    let config = PipelineConfig {
        playbook: Playbook {
            hash: binary.hash(),
            configurations: HashMap::from([("state".to_string(), None)]),
        },
        wal_capacity: 1000,
        success_log_retention_secs: 3600,
        error_log_retention_secs: 86400 * 7,
        output_dir: std::env::current_dir().unwrap(),
    };

    let config = config.load(&cache).await.unwrap();
    let factory = config.factory().unwrap();
    let mut pipeline = factory.build().await.unwrap();

    let result1 = pipeline
        .process(&PyroRow::from([("input", "0".into())]))
        .await;
    let row1 = result1.row().unwrap();
    // Result should be (count: 0, incremented: 0) since fetch_add returns previous
    assert_eq!(row1.get_u64("incremented").unwrap(), 0);

    // Second call: The call_count in CounterServer should now be 1
    let result2 = pipeline
        .process(&PyroRow::from([("input", "0".into())]))
        .await;
    let row2 = result2.row().unwrap();
    assert_eq!(row2.get_u64("incremented").unwrap(), 1);
}
