use pyro_artifacts::{
    artifacts::{ModuleDependencies, ModuleSource, Playbook},
    build::Builder,
    cache::CacheManager,
};
use pyroduct::module::sessions::SessionResult;
use pyroduct::{PyroRow, pipeline::PipelineConfig};
use std::collections::HashMap;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// A session module that returns a single string field.
const SIMPLE_SESSION_MODULE: &str = r#"
// Session module v2
use pyroduct::session::SessionResponse;

#[pyroduct::module(session, output = message)]
fn counter(
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
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "trace,cranelift_frontend=off,cranelift_codegen=off,wasmtime=off".into()
    });

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).pretty())
        .with(filter)
        .init();

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

    let tmp_dir = tempfile::tempdir().unwrap();
    let tmp_path = tmp_dir.path().to_path_buf();

    let config = PipelineConfig {
        playbook: Playbook {
            hash: binary.hash(),
            configurations: HashMap::new(),
        },
        wal_capacity: 2,
        success_log_retention_secs: 3600,
        error_log_retention_secs: 86400 * 7,
        input_dir: tmp_path.clone(),
        output_dir: tmp_path.clone(),
        log_dir: tmp_path.clone(),
    };

    let loaded = config.load(&cache).await.unwrap();
    let pipeline_factory = loaded.factory().unwrap();
    let mut pipeline = pipeline_factory.build().await.unwrap();

    let session_id = 42;
    pipeline
        .prep_session(session_id, &[], &[])
        .await
        .expect("Should prep session");

    // Trigger log rotation
    for i in 0..5 {
        let input = PyroRow::from([("input", format!("rot {}", i).into())]);
        pipeline.call_session(session_id, &input).await.unwrap();
    }

    // Verify log rotation
    let log_files = std::fs::read_dir(&tmp_path).unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "log"))
        .count();
    assert!(log_files > 1, "Should have rotated logs, but found only {} file(s)", log_files);

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
