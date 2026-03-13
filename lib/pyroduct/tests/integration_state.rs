use indexmap::IndexMap;
use pyroduct::{
    PyroRow,
    module::ModuleConfig,
    pipeline::{PipelineConfig, PipelineFactory},
};
use std::{collections::HashMap, path::Path};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

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
    // Use the counter capability from tests/cap_config
    let cap_path = Path::new("../../capabilities/state/");

    let config = PipelineConfig {
         pipeline: IndexMap::from([(
            "step".to_string(),
            ModuleConfig {
                path: Path::new("../../modules/cap_state/").to_path_buf(),
                libraries: vec![cap_path.to_path_buf()],
                configurations: HashMap::from([("state".to_string(), None)]),
            }
         )]),
    };

    let mut factory = PipelineFactory::load(&config).await.unwrap();
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
