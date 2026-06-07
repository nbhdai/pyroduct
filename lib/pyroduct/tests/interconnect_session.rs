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

const TARGET_SESSION_MODULE: &str = r#"
use pyroduct::{session::SessionResponse, tracing};

#[pyroduct::module(session, output = message)]
fn counter(
    prior: Vec<String>,
    input: String,
) -> Result<SessionResponse<String>> {
    let turn = (prior.len() as u32 + 1) / 2;
    match turn {
        0 => Ok(SessionResponse::Continue(format!("Hello! Turn {}", turn + 1))),
        1 => Ok(SessionResponse::End(format!("Goodbye! Turn {}", turn + 1))),
        _ => Ok(SessionResponse::Terminate),
    }
}
"#;

const CALLER_SESSION_MODULE: &str = r#"
use pyroduct;
use pyroduct::call_session;
use pyroduct::format::PyroRow;
use pyroduct::session::SessionResponse;

#[pyroduct::module(session, output = message)]
fn caller(
    prior: Vec<String>,
    input: String,
) -> Result<SessionResponse<String>> {
    let turn = (prior.len() as u32 + 1) / 2;
    let target_input = PyroRow::from([("input", input.into())]);
    let target_output = call_session("target", 42, &target_input);
    let msg = target_output.get_str("message").unwrap();
    match turn {
        0 => Ok(SessionResponse::Continue(format!("Caller turn 1: {}", msg))),
        1 => Ok(SessionResponse::End(format!("Caller turn 2: {}", msg))),
        _ => Ok(SessionResponse::Terminate),
    }
}
"#;

#[tokio::test]
async fn test_interconnect_session_lifecycle() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "trace,cranelift_frontend=off,cranelift_codegen=off,wasmtime=off".into()
    });

    let _ = tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).pretty())
        .with(filter)
        .try_init();

    let cache = std::sync::Arc::new(CacheManager::from_env().await.unwrap());
    let builder = Builder::from_env(cache.clone()).await.unwrap();

    // 1. Compile target session playbook
    let target_source = AnonPlaybook {
        package: "test_interconnect_session_target".to_string(),
        dependencies: std::collections::BTreeMap::new(),
        configurations: Vec::new(),
        source: TARGET_SESSION_MODULE.to_string(),
        interconnect: std::collections::BTreeMap::new(),
    };
    cache
        .remove_module("anon", "test_interconnect_session_target", "0.1.0")
        .await
        .unwrap();
    let target_binary = builder
        .compile_anon(&target_source)
        .await
        .expect("Target session module should compile");

    // 2. Compile caller session playbook
    let mut interconnect_map = std::collections::BTreeMap::new();
    interconnect_map.insert("target".to_string(), target_binary.spec.ident.clone());
    let caller_source = AnonPlaybook {
        package: "test_interconnect_session_caller".to_string(),
        dependencies: std::collections::BTreeMap::new(),
        configurations: Vec::new(),
        source: CALLER_SESSION_MODULE.to_string(),
        interconnect: interconnect_map,
    };
    cache
        .remove_module("anon", "test_interconnect_session_caller", "0.1.0")
        .await
        .unwrap();
    let caller_binary = builder
        .compile_anon(&caller_source)
        .await
        .expect("Caller session module should compile");

    let tmp_dir = tempfile::tempdir().unwrap();
    let tmp_path = tmp_dir.path().to_path_buf();

    let target_path = tmp_path.join("target");
    let caller_path = tmp_path.join("caller");
    std::fs::create_dir_all(&target_path).unwrap();
    std::fs::create_dir_all(&caller_path).unwrap();

    // Load target pipeline config
    let target_config = PipelineConfig {
        playbook: target_binary.spec.ident.clone(),
        remote: HashMap::new(),
        wal_capacity: 5,
        success_log_retention_secs: 3600,
        error_log_retention_secs: 86400 * 7,
        input_dir: target_path.clone(),
        output_dir: target_path.clone(),
        log_dir: target_path.clone(),
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
        input_dir: caller_path.clone(),
        output_dir: caller_path.clone(),
        log_dir: caller_path.clone(),
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
    let mut caller_pipeline = caller_factory.build_session().await.unwrap();

    let session_id = 42;
    caller_pipeline
        .prep_session(session_id, &[])
        .await
        .expect("Should prep session");

    // Call caller server with session ID 42
    let turn1_input = PyroRow::from([("input", "Hello!".into())]);
    let result1 = caller_pipeline
        .call(session_id, &turn1_input)
        .await
        .unwrap();
    match result1 {
        pyroduct::pipeline::session::SessionExecutionRecord::Success {
            success: turn1_output,
            ..
        } => {
            assert_eq!(
                turn1_output.get_str("message").unwrap(),
                "Caller turn 1: Hello! Turn 1"
            );
        }
        other => panic!("Expected Success, got {:?}", other),
    }

    let turn2_input = PyroRow::from([("input", "Turn 2".into())]);
    let result2 = caller_pipeline
        .call(session_id, &turn2_input)
        .await
        .unwrap();
    match result2 {
        pyroduct::pipeline::session::SessionExecutionRecord::Success {
            success: turn2_output,
            ..
        } => {
            assert_eq!(
                turn2_output.get_str("message").unwrap(),
                "Caller turn 2: Goodbye! Turn 2"
            );
        }
        other => panic!("Expected Success, got {:?}", other),
    }
}
