use indexmap::IndexMap;
use pyro_artifacts::{artifacts::Artifacts, cache::CacheManager, environment::Environment};
use pyroduct::{
    PyroRow,
    module::ModuleConfig,
    pipeline::{PipelineConfig, PipelineFactory},
};
use std::collections::HashMap;
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
    let cache = CacheManager::from_env().await.unwrap();

    let env = Environment::new("../../modules/cap_state/".into())
        .await
        .unwrap();
    let artifacts = env.package(false).await.unwrap();
    for artifact in &artifacts {
        cache.write_artifacts(artifact).await.unwrap();
    }
    let hash = match &artifacts[0] {
        Artifacts::Module(module) => module.hash(),
        _ => panic!("Not a module!"),
    };

    let config = PipelineConfig {
        pipeline: IndexMap::from([(
            "step".to_string(),
            ModuleConfig {
                module: pyroduct::module::Module::Hash(hash),
                configurations: HashMap::from([("state".to_string(), None)]),
            },
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
