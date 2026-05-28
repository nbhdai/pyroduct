use pyro_artifacts::{
    artifacts::{ModuleDependencies, ModuleSource, Playbook, CapabilityConfig},
    build::Builder,
    cache::CacheManager,
    cargo::ResolvedCapability,
};
use pyroduct::{
    PyroRow,
    pipeline::PipelineConfig,
    transport::socket::{
        PyroListener,
        playbook::{PlaybookClient, PlaybookServer},
    },
};
use std::collections::{BTreeMap, HashMap};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

const CODE: &str = r#"
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

#[tokio::test]
async fn test_playbook_server_client() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "trace,cranelift_frontend=off,cranelift_codegen=off,wasmtime=off".into()
    });

    let _ = tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).pretty())
        .with(filter)
        .try_init();

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
        ident: None,
    };

    let binary = builder
        .compile(&source)
        .await
        .expect("Valid module should compile");

    let config = PipelineConfig {
        playbook: Playbook {
            hash: binary.hash(),
            configurations: HashMap::from([(
                "state".to_string(),
                CapabilityConfig {
                    classes: HashMap::from([("CounterClient".to_string(), None)]),
                },
            )]),
        },
        wal_capacity: 1000,
        success_log_retention_secs: 3600,
        error_log_retention_secs: 86400 * 7,
        input_dir: std::env::current_dir().unwrap(),
        output_dir: std::env::current_dir().unwrap(),
        log_dir: std::env::current_dir().unwrap(),
    };

    let config = config.load(&cache).await.unwrap();
    let loaded_playbook = &config.playbook;

    let server = PlaybookServer::new(loaded_playbook)
        .await
        .expect("Failed to create server");

    let listener = PyroListener::bind_tcp("127.0.0.1:0")
        .await
        .expect("Failed to bind listener");
    let addr = listener.local_addr_tcp().expect("Failed to get local addr");

    // Run server in background
    tokio::spawn(async move {
        if let Err(e) = server.run(listener).await {
            eprintln!("Server error: {:?}", e);
        }
    });

    // Client Connection
    let mut client = PlaybookClient::connect_tcp(addr)
        .await
        .expect("Failed to connect to server");

    // Send first request
    let result1 = client
        .call(&PyroRow::from([("input", "0".into())]))
        .await
        .expect("Failed to call playbook");

    // Result should be (count: 0, incremented: 0)
    assert_eq!(result1.row.get_u64("incremented").unwrap(), 0);

    // Send second request
    let result2 = client
        .call(&PyroRow::from([("input", "0".into())]))
        .await
        .expect("Failed to call playbook");

    // Second call: The call_count in CounterServer should now be 1 (state preserved)
    assert_eq!(result2.row.get_u64("incremented").unwrap(), 1);
}
