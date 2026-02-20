use pyroduct::{PyroRow, pipeline::{CapabilityConfig, ModuleConfig, Pipeline, PipelineConfig, PipelineDef}};
use serde_json::json;
use std::{collections::HashMap, path::Path};

/// Test that capability state is preserved across multiple calls to the same module instance.
#[tracing_test::traced_test]
#[tokio::test] 
async fn test_capability_state_preservation() {
    // Use the counter capability from tests/cap_config
    #[cfg(target_os = "linux")]
    let cap_path = Path::new("../../capabilities/state/artifacts/lib.so");
    #[cfg(target_os = "macos")]
    let cap_path = Path::new("../../capabilities/state/artifacts/lib.dylib");
    let config = PipelineConfig {
        capabilities: HashMap::from([
            ("state".to_string(), CapabilityConfig {
                path: cap_path.to_path_buf(),
                classes: HashMap::new(),
            })
        ]),
        modules: HashMap::from([
            ("state_mod".to_string(), ModuleConfig {
                path: Path::new("../../modules/cap_state/artifacts/mod.wasm").to_path_buf(),
                capabilities: vec!["state".to_string()],
            })
        ]),
        pipeline: vec!["state_mod".to_string()],
    };

    let pipeline_def = PipelineDef::load(&config).await.unwrap();
    let mut pipeline = Pipeline::new(pipeline_def).await.unwrap();

    let result1 = pipeline.process(PyroRow::from([("input", "0".into())])).await.unwrap().unwrap();

    // Result should be (count: 0, incremented: 0) since fetch_add returns previous
    assert_eq!(result1.get_u64("incremented").unwrap(), 0);

    // Second call: The call_count in CounterServer should now be 1
    let result2 = pipeline.process(PyroRow::from([("input", "0".into())])).await.unwrap().unwrap();
    assert_eq!(result2.get_u64("incremented").unwrap(), 1);
}


/// Test that capability configurations (passed via PipelineDef) are correctly respected by the server.
#[tracing_test::traced_test]
#[tokio::test]
async fn test_capability_configuration_respect() {
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
