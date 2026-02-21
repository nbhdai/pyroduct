use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use pyroduct::{PyroRow, pipeline::{CapabilityConfig, ModuleConfig, Pipeline, PipelineConfig, PipelineDef}};
use serde_json::json;
use std::{collections::HashMap, path::Path};

/// Test that capability configurations (passed via PipelineDef) are correctly respected by the server.
#[tokio::test]
async fn test_capability_configuration_respect() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "trace,cranelift_frontend=off,cranelift_codegen=off,wasmtime=off".into());

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).pretty())
        .with(filter)
        .init();
    
    // Use the counter capability from tests/cap_config
    #[cfg(target_os = "linux")]
    let cap_path = Path::new("../../capabilities/config/artifacts/lib.so");
    #[cfg(target_os = "macos")]
    let cap_path = Path::new("../../capabilities/config/artifacts/lib.dylib");
    let config = PipelineConfig {
        capabilities: HashMap::from([
            ("config".to_string(), CapabilityConfig {
                path: cap_path.to_path_buf(),
                classes: HashMap::from([(
                    "config".to_string(),
                    json!({
                        "uppercase": true,
                        "suffix": "!!!"
                    }),
                )]),
            })
        ]),
        modules: HashMap::from([
            ("config_mod".to_string(), ModuleConfig {
                path: Path::new("../../modules/cap_config/artifacts/mod.wasm").to_path_buf(),
                capabilities: vec!["config".to_string()],
            })
        ]),
        pipeline: vec!["config_mod".to_string()],
    };

    let pipeline_def = PipelineDef::load(&config).await.unwrap();
    let mut pipeline = Pipeline::new(pipeline_def).await.unwrap();

    let input = PyroRow::from([("input", "hello".into())]);
    let result = pipeline.process(input).await.unwrap().unwrap();
    let transformed = result.get_str("transformed").unwrap();
    // Result should be (count: 0, incremented: 0) since fetch_add returns previous
    assert_eq!(transformed, "[TEST] HELLO!!!");
}
