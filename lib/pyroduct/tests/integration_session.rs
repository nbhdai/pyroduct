use pyro_artifacts::{
    artifacts::{ModuleDependencies, ModuleSource, Playbook},
    build::Builder,
    cache::CacheManager,
};
use pyroduct::module::sessions::SessionResult;
use pyroduct::{PyroRow, pipeline::PipelineConfig};
use std::collections::HashMap;
use tracing_subscriber::EnvFilter;

/// A session module that returns a single string field.
const SIMPLE_SESSION_MODULE: &str = r#"
// Session module v2
use pyroduct::session::SessionResponse;

#[pyroduct::module(session, output = message)]
fn counter<'a>(
    _prior_input: Vec<String>,
    prior_output: Vec<String>,
    input: String,
) -> Result<SessionResponse<String>> {
    let turn = prior_output.len() as u32 + 1;

    match turn {
        1 => Ok(SessionResponse::Continue(format!("Hello! Turn {}", turn))),
        2 => Ok(SessionResponse::End(format!("Goodbye! Turn {}", turn))),
        _ => Ok(SessionResponse::Terminate),
    }
}
"#;

/// Test that a session module can be compiled, instantiated, and called via the Pipeline.
#[tokio::test]
async fn test_session_lifecycle() {
    init_tracing();

    let cache = std::sync::Arc::new(CacheManager::from_env().await.unwrap());
    let builder = Builder::from_env(cache.clone()).await.unwrap();

    let source = ModuleSource {
        dependencies: ModuleDependencies {
            dependencies: std::collections::BTreeMap::new(),
            capabilities: vec![],
        },
        source: SIMPLE_SESSION_MODULE.to_string(),
        ident: None,
    };

    let binary = builder
        .compile(&source)
        .await
        .expect("Valid session module should compile");

    let config = PipelineConfig {
        playbook: Playbook {
            hash: binary.hash(),
            configurations: HashMap::new(),
        },
        wal_capacity: 1000,
        success_log_retention_secs: 3600,
        error_log_retention_secs: 86400 * 7,
        input_dir: std::env::current_dir().unwrap(),
        output_dir: std::env::current_dir().unwrap(),
        log_dir: std::env::current_dir().unwrap(),
    };

    let loaded = config.load(&cache).await.unwrap();
    let pipeline_factory = loaded.factory().unwrap();
    let mut pipeline = pipeline_factory.build().await.unwrap();

    let session_id = 42;
    pipeline
        .prep_session(session_id, &[], &[])
        .await
        .expect("Should prep session");

    // --- Turn 1 ---
    let turn1_input = PyroRow::from([("input", "Hello!".into())]);
    let result1 = pipeline
        .call_session(session_id, &turn1_input)
        .await
        .expect("Session call turn 1 should succeed");

    match result1 {
        SessionResult::Continue(row) => {
            assert_eq!(row.get_str("message").unwrap(), "Hello! Turn 1");
        }
        other => panic!("Expected Continue, got {:?}", other),
    }

    // --- Turn 2 ---
    let turn2_input = PyroRow::from([("input", "How are you?".into())]);
    let result2 = pipeline
        .call_session(session_id, &turn2_input)
        .await
        .expect("Session call turn 2 should succeed");

    match result2 {
        SessionResult::End(row) => {
            assert_eq!(row.get_str("message").unwrap(), "Goodbye! Turn 2");
        }
        other => panic!("Expected End, got {:?}", other),
    }

    // --- Turn 3 ---
    let turn3_input = PyroRow::from([("input", "Wait, don't go!".into())]);
    let result3 = pipeline
        .call_session(session_id, &turn3_input)
        .await
        .expect("Session call turn 3 should succeed");

    match result3 {
        SessionResult::Terminate => {}
        other => panic!("Expected Terminate, got {:?}", other),
    }

    let (in_len, out_len) = pipeline
        .session_lengths(session_id)
        .expect("Session should exist");
    assert_eq!(in_len, 3);
    assert_eq!(out_len, 3);

    pipeline
        .close_session(session_id)
        .await
        .expect("Should close session");
}

#[tokio::test]
async fn test_multiple_sessions_isolation() {
    init_tracing();

    let cache = std::sync::Arc::new(CacheManager::from_env().await.unwrap());
    let builder = Builder::from_env(cache.clone()).await.unwrap();

    let source = ModuleSource {
        dependencies: ModuleDependencies {
            dependencies: std::collections::BTreeMap::new(),
            capabilities: vec![],
        },
        source: SIMPLE_SESSION_MODULE.to_string(),
        ident: None,
    };

    let binary = builder
        .compile(&source)
        .await
        .expect("Module should compile");

    let config = PipelineConfig {
        playbook: Playbook {
            hash: binary.hash(),
            configurations: HashMap::new(),
        },
        wal_capacity: 1000,
        success_log_retention_secs: 3600,
        error_log_retention_secs: 86400 * 7,
        input_dir: std::env::current_dir().unwrap(),
        output_dir: std::env::current_dir().unwrap(),
        log_dir: std::env::current_dir().unwrap(),
    };

    let loaded = config.load(&cache).await.unwrap();
    let pipeline_factory = loaded.factory().unwrap();
    let mut pipeline = pipeline_factory.build().await.unwrap();

    let id_a = 100;
    let id_b = 200;
    pipeline.prep_session(id_a, &[], &[]).await.unwrap();
    pipeline.prep_session(id_b, &[], &[]).await.unwrap();

    let input_a = PyroRow::from([("input", "Hi A".into())]);
    let input_b = PyroRow::from([("input", "Hi B".into())]);

    let res_a = pipeline.call_session(id_a, &input_a).await.unwrap();
    let res_b = pipeline.call_session(id_b, &input_b).await.unwrap();

    if let SessionResult::Continue(row) = res_a {
        assert_eq!(row.get_str("message").unwrap(), "Hello! Turn 1");
    } else {
        panic!("A should continue")
    }

    if let SessionResult::Continue(row) = res_b {
        assert_eq!(row.get_str("message").unwrap(), "Hello! Turn 1");
    } else {
        panic!("B should continue")
    }

    let input_a2 = PyroRow::from([("input", "A2".into())]);
    let res_a2 = pipeline.call_session(id_a, &input_a2).await.unwrap();
    if let SessionResult::End(row) = res_a2 {
        assert_eq!(row.get_str("message").unwrap(), "Goodbye! Turn 2");
    } else {
        panic!("A should end")
    }

    let input_b2 = PyroRow::from([("input", "B2".into())]);
    let res_b2 = pipeline.call_session(id_b, &input_b2).await.unwrap();
    if let SessionResult::End(row) = res_b2 {
        assert_eq!(row.get_str("message").unwrap(), "Goodbye! Turn 2");
    } else {
        panic!("B should end")
    }

    pipeline.close_session(id_a).await.unwrap();
    pipeline.close_session(id_b).await.unwrap();
}

#[tokio::test]
async fn test_session_error_handling() {
    init_tracing();

    let cache = std::sync::Arc::new(CacheManager::from_env().await.unwrap());
    let builder = Builder::from_env(cache.clone()).await.unwrap();

    const ERROR_MODULE: &str = r#"
use pyroduct::session::SessionResponse;
#[pyroduct::module(session, output = message)]
fn error_fn<'a>(_prior_in: Vec<String>, _prior_out: Vec<String>, input: String) -> Result<SessionResponse<String>> {
    Err(pyroduct::CapturedError::new(format!("Error processing: {}", input)))
}
"#;

    let source = ModuleSource {
        dependencies: ModuleDependencies {
            dependencies: std::collections::BTreeMap::new(),
            capabilities: vec![],
        },
        source: ERROR_MODULE.to_string(),
        ident: None,
    };

    let binary = builder
        .compile(&source)
        .await
        .expect("Module should compile");

    let config = PipelineConfig {
        playbook: Playbook {
            hash: binary.hash(),
            configurations: HashMap::new(),
        },
        wal_capacity: 1000,
        success_log_retention_secs: 3600,
        error_log_retention_secs: 86400 * 7,
        input_dir: std::env::current_dir().unwrap(),
        output_dir: std::env::current_dir().unwrap(),
        log_dir: std::env::current_dir().unwrap(),
    };

    let loaded = config.load(&cache).await.unwrap();
    let pipeline_factory = loaded.factory().unwrap();
    let mut pipeline = pipeline_factory.build().await.unwrap();

    let session_id = 99;
    pipeline.prep_session(session_id, &[], &[]).await.unwrap();

    let input = PyroRow::from([("input", "test".into())]);
    let result = pipeline.call_session(session_id, &input).await;

    assert!(result.is_err(), "Should fail");

    pipeline.close_session(session_id).await.unwrap();
}

fn init_tracing() {
    let _ = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "trace,cranelift_frontend=off,cranelift_codegen=off,wasmtime=off".into()
    });
}
