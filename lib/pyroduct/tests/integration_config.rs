use indexmap::IndexMap;
use pyro_artifacts::{
    artifacts::{ModuleDependencies, ModuleSource, Playbook},
    cache::CacheManager,
    cargo::ResolvedCapability,
};
use pyroduct::{PyroRow, pipeline::PipelineConfig};
use serde_json::json;
use std::collections::{BTreeMap, HashMap};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

const CODE: &'static str = r#"
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
    let cache = CacheManager::from_env().await.unwrap();
    let source = ModuleSource {
        dependencies: ModuleDependencies {
            dependencies: BTreeMap::new(),
            capabilities: vec![ResolvedCapability {
                package: "config".to_string(),
                author: "nbhdai".to_string(),
                version: "0.1.0".to_string(),
            }],
        },
        source: CODE.to_string(),
    };

    let binary = cache
        .compile(&source)
        .await
        .expect("Valid module should compile");

    let config = PipelineConfig {
        pipeline: IndexMap::from([(
            "config".to_string(),
            Playbook {
                hash: binary.hash,
                configurations: HashMap::from([(
                    "config".to_string(),
                    Some(json!({
                        "uppercase": true,
                        "suffix": "!!!"
                    })),
                )]),
            },
        )]),
    };
    let config = config.load(&cache).await.unwrap();
    let factory = config.factory().unwrap();
    let mut pipeline = factory.build().await.unwrap();

    let input = PyroRow::from([("input", "hello".into())]);
    let result = pipeline.process(&input).await;

    for (i, step) in result.steps.iter().enumerate() {
        println!("--- Step {} logs ---", i);
        for log in &step.logs.module_logs {
            println!("{}", log);
        }
        for ((cap, ver), logs) in &step.logs.capability_logs {
            for log in logs {
                println!("[{} v{}]: {}", cap, ver, log);
            }
        }
    }

    if let Some(failure) = &result.failure {
        println!("--- Failure logs ---");
        for log in &failure.logs.module_logs {
            println!("{}", log);
        }
        for ((cap, ver), logs) in &failure.logs.capability_logs {
            for log in logs {
                println!("[{} v{}]: {}", cap, ver, log);
            }
        }
        println!("Failure: {:?}", failure.result);
    }

    let row = result.row().unwrap();
    let transformed = row.get_str("transformed").unwrap();
    // Result should be (count: 0, incremented: 0) since fetch_add returns previous
    assert_eq!(transformed, "[TEST] HELLO!!!");
}
