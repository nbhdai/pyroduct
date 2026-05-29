use pyro_artifacts::{
    artifacts::CapabilityConfig,
    build::{AnonPlaybook, Builder},
    cache::CacheManager,
};
use pyroduct::{PyroRow, pipeline::PipelineConfig};
use serde_json::json;
use std::collections::{BTreeMap, HashMap};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

const CODE: &str = r#"
//! The behavior of this module changes based on the configuration 
//! of the linked capability

use config::{TransformClient, TransformClientMethods};

#[pyroduct::module(output = (original, transformed, transform_count))]
pub fn call(input: &str) -> Result<(String, String, u64)> {
    let client = TransformClient {
        prefix: "[TEST] ".to_string(),
    }.register()?;

    let original = input.to_string();
    let transformed = client.transform(input.to_string())?;
    let transform_count = client.get_transform_count()? as u64;

    Ok((original, transformed, transform_count))
}
"#;

/// Test that capability configurations (passed via PipelineDef) are correctly respected by the server.
#[tokio::test]
async fn test_capability_configuration_respect() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "trace,cranelift_frontend=off,cranelift_codegen=off,wasmtime=off".into()
    });

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).pretty())
        .with(filter)
        .init();
    let cache = std::sync::Arc::new(CacheManager::from_env().await.unwrap());
    let builder = Builder::from_env(cache.clone()).await.unwrap();
    let source = AnonPlaybook {
        name: "test_integration_config".to_string(),
        dependencies: BTreeMap::new(),
        configurations: vec![pyro_artifacts::cargo::ConfiguredCapability {
            package: "config".to_string(),
            author: "nbhdai".to_string(),
            version: "0.1.0".to_string(),
            configuration: CapabilityConfig {
                classes: HashMap::from([(
                    "transform".to_string(),
                    Some(json!({
                        "uppercase": true,
                        "suffix": "!!!"
                    })),
                )]),
            },
        }],
        source: CODE.to_string(),
    };
    cache.remove_module("anon", "test_integration_config", "0.1.0").await.unwrap();

    let binary = builder
        .compile_anon(&source)
        .await
        .expect("Valid module should compile");

    let tmp_dir = tempfile::tempdir().unwrap();
    let tmp_path = tmp_dir.path().to_path_buf();

    let ident = &binary.spec.ident;

    let config = PipelineConfig {
        playbook_author: ident.author.clone(),
        playbook_name: ident.name.clone(),
        playbook_version: ident.version.clone(),
        remote: HashMap::new(),
        wal_capacity: 2,
        success_log_retention_secs: 3600,
        error_log_retention_secs: 86400 * 7,
        input_dir: tmp_path.clone(),
        output_dir: tmp_path.clone(),
        log_dir: tmp_path.clone(),
    };
    let config = config.load(&cache).await.unwrap();
    let factory = config.factory().unwrap();
    let mut pipeline = factory.build().await.unwrap();

    for i in 0..5 {
        let input = PyroRow::from([("input", format!("hello {}", i).into())]);
        pipeline.process(i, &input).await.unwrap();
    }

    // Verify that multiple log files were created due to low wal_capacity
    let log_files = std::fs::read_dir(&tmp_path)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "pyrolog"))
        .count();
    assert!(
        log_files > 1,
        "Should have rotated logs, but found only {} file(s)",
        log_files
    );

    let input = PyroRow::from([("input", "hello".into())]);
    let result = pipeline.process(10, &input).await.unwrap();

    let logs = match &result {
        pyroduct::pipeline::ExecutionRecord::Success { logs, .. } => logs,
        pyroduct::pipeline::ExecutionRecord::Failure { logs, .. } => logs,
    };

    println!("--- Logs ---");
    for log in &logs.module_logs {
        println!("{}", log);
    }
    for ((cap, ver), logs) in &logs.capability_logs {
        for log in logs {
            println!("[{} v{}]: {}", cap, ver, log);
        }
    }

    if let pyroduct::pipeline::ExecutionRecord::Failure { failure, .. } = &result {
        println!("Failure: {:?}", failure);
    }

    let row = result.row().unwrap();
    let transformed = row.get_str("transformed").unwrap();
    // Result should be (count: 0, incremented: 0) since fetch_add returns previous
    assert_eq!(transformed, "[TEST] HELLO!!!");
}
