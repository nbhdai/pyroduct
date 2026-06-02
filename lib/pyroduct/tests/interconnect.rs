use pyro_artifacts::{
    build::{AnonPlaybook, Builder},
    cache::CacheManager,
};
use pyroduct::{
    PyroRow,
    pipeline::{PipelineConfig, PipelineServer, PlaybookCollection},
};
use std::collections::HashMap;
use std::sync::Arc;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

const TARGET_MODULE: &str = r#"
use pyroduct;

#[pyroduct::module(output = message)]
pub fn call(input: String) -> Result<String> {
    Ok(format!("Hello: {}", input))
}
"#;

const CALLER_MODULE: &str = r#"
use pyroduct;
use pyroduct::call_playbook;
use pyroduct::format::PyroRow;

#[pyroduct::module(output = message)]
pub fn call(input: String) -> Result<String> {
    let target_input = PyroRow::from([("input", input.into())]);
    let (_session_id, target_output) = call_playbook("target", &target_input);
    let msg = target_output.get_str("message").unwrap();
    Ok(format!("Caller received: {}", msg))
}
"#;

#[tokio::test]
async fn test_interconnect_lifecycle() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "trace,cranelift_frontend=off,cranelift_codegen=off,wasmtime=off".into()
    });

    let _ = tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).pretty())
        .with(filter)
        .try_init();

    let cache = std::sync::Arc::new(CacheManager::from_env().await.unwrap());
    let builder = Builder::from_env(cache.clone()).await.unwrap();

    // 1. Compile target playbook
    let target_source = AnonPlaybook {
        package: "test_interconnect_target".to_string(),
        dependencies: std::collections::BTreeMap::new(),
        configurations: Vec::new(),
        source: TARGET_MODULE.to_string(),
    };
    cache
        .remove_module("anon", "test_interconnect_target", "0.1.0")
        .await
        .unwrap();
    let target_binary = builder
        .compile_anon(&target_source)
        .await
        .expect("Target module should compile");

    // 2. Compile caller playbook
    let caller_source = AnonPlaybook {
        package: "test_interconnect_caller".to_string(),
        dependencies: std::collections::BTreeMap::new(),
        configurations: Vec::new(),
        source: CALLER_MODULE.to_string(),
    };
    cache
        .remove_module("anon", "test_interconnect_caller", "0.1.0")
        .await
        .unwrap();
    let caller_binary = builder
        .compile_anon(&caller_source)
        .await
        .expect("Caller module should compile");

    let tmp_dir = tempfile::tempdir().unwrap();
    let tmp_path = tmp_dir.path().to_path_buf();

    // Load target pipeline config
    let target_config = PipelineConfig {
        playbook: target_binary.spec.ident.clone(),
        remote: HashMap::new(),
        wal_capacity: 5,
        success_log_retention_secs: 3600,
        error_log_retention_secs: 86400 * 7,
        input_dir: tmp_path.clone(),
        output_dir: tmp_path.clone(),
        log_dir: tmp_path.clone(),
    };
    let target_loaded = target_config.load(&cache).await.unwrap();
    let target_server = PipelineServer::new(&target_loaded.playbook).await.unwrap();

    // Load caller pipeline config
    let caller_config = PipelineConfig {
        playbook: caller_binary.spec.ident.clone(),
        remote: HashMap::new(),
        wal_capacity: 5,
        success_log_retention_secs: 3600,
        error_log_retention_secs: 86400 * 7,
        input_dir: tmp_path.clone(),
        output_dir: tmp_path.clone(),
        log_dir: tmp_path.clone(),
    };
    let caller_loaded = caller_config.load(&cache).await.unwrap();

    // Setup interconnect collection
    let mut playbooks = HashMap::new();
    playbooks.insert("target".to_string(), target_binary.spec.func.clone());

    let mut servers = HashMap::new();
    servers.insert("target".to_string(), target_server);

    let interconnect = Arc::new(PlaybookCollection { playbooks, servers });

    // Instantiate caller pipeline with interconnect via PipelineFactory
    let caller_factory = caller_loaded
        .factory()
        .unwrap()
        .with_interconnect(interconnect);
    let mut caller_pipeline = caller_factory.build().await.unwrap();

    // Call caller server
    let input = PyroRow::from([("input", "World".into())]);
    let result = caller_pipeline.call(&input).await.unwrap();

    assert_eq!(
        result.row.get_str("message").unwrap(),
        "Caller received: Hello: World"
    );
}
