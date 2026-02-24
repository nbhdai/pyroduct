use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use pyroduct::{PyroRow, pipeline::{ModuleConfig, Pipeline, PipelineConfig, PipelineDef}};
use std::{collections::HashMap, path::Path};

/// Test that capability state is preserved across multiple calls to the same module instance.
#[tokio::test] 
async fn test_capability_state_preservation() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "trace,cranelift_frontend=off,cranelift_codegen=off,wasmtime=off".into());

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).pretty())
        .with(filter)
        .init();
    // Use the counter capability from tests/cap_config
    #[cfg(target_os = "linux")]
    let cap_path = Path::new("../../capabilities/state/artifacts/lib.so");
    #[cfg(target_os = "macos")]
    let cap_path = Path::new("../../capabilities/state/artifacts/lib.dylib");
    let config = PipelineConfig {
        libraries: HashMap::from([("state".to_string(), cap_path.to_path_buf())]),
        modules: HashMap::from([
            ("state_mod".to_string(), ModuleConfig {
                path: Path::new("../../modules/cap_state/artifacts/mod.wasm").to_path_buf(),
                capabilities: HashMap::new(),
            })
        ]),
        pipeline: vec!["state_mod".to_string()],
    };

    let pipeline_def = PipelineDef::load(&config).await.unwrap();
    let mut pipeline = Pipeline::new(pipeline_def).await.unwrap();

    let result1 = pipeline.process(&PyroRow::from([("input", "0".into())])).await;
    let row1 = result1.row().unwrap();
    // Result should be (count: 0, incremented: 0) since fetch_add returns previous
    assert_eq!(row1.get_u64("incremented").unwrap(), 0);

    // Second call: The call_count in CounterServer should now be 1
    let result2 = pipeline.process(&PyroRow::from([("input", "0".into())])).await;
    let row2 = result2.row().unwrap();
    assert_eq!(row2.get_u64("incremented").unwrap(), 1);
}