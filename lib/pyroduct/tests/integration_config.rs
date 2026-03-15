use indexmap::IndexMap;
use pyroduct::{
    PyroRow,
    module::ModuleConfig,
    pipeline::{PipelineConfig, PipelineFactory},
};
use serde_json::json;
use std::{collections::HashMap, path::Path};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

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

    let cap_path = Path::new("../../capabilities/config/artifacts/");

    let config = PipelineConfig {
        pipeline: IndexMap::from([(
            "config".to_string(),
            ModuleConfig {
                path: Path::new("../../modules/cap_config/artifacts/").to_path_buf(),
                libraries: vec![cap_path.to_path_buf()],
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

    let mut factory = PipelineFactory::load(&config).await.unwrap();
    let mut pipeline = factory.build().await.unwrap();

    let input = PyroRow::from([("input", "hello".into())]);
    let result = pipeline.process(&input).await;
    let row = result.row().unwrap();
    let transformed = row.get_str("transformed").unwrap();
    // Result should be (count: 0, incremented: 0) since fetch_add returns previous
    assert_eq!(transformed, "[TEST] HELLO!!!");
}
