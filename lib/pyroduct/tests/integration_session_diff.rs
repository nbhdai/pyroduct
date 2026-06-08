use pyro_artifacts::{
    build::{AnonPlaybook, Builder},
    cache::CacheManager,
};
use pyroduct::{
    PyroRow,
    pipeline::{PipelineConfig, session_diff::SessionDiffExecutionRecord},
};
use std::collections::HashMap;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// A session module that returns a single string field.
const SIMPLE_SESSION_MODULE: &str = r#"
// Session module v2
use pyroduct::session::SessionResponse;

#[pyroduct::module(session, output = message)]
fn counter(
    prior_input: Vec<String>,
    prior_output: Vec<String>,
    input: String,
) -> Result<SessionResponse<String>> {
    if input == "terminate" {
        return Ok(SessionResponse::Terminate);
    }
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

    let source = AnonPlaybook {
        package: "test_session_diff".to_string(),
        dependencies: std::collections::BTreeMap::new(),
        configurations: Vec::new(),
        source: SIMPLE_SESSION_MODULE.to_string(),
        interconnect: std::collections::BTreeMap::new(),
    };
    cache
        .remove_module("anon", "test_session_diff", "0.1.0")
        .await
        .unwrap();

    let binary = builder
        .compile_anon(&source)
        .await
        .expect("Valid session module should compile");

    let tmp_dir = tempfile::tempdir().unwrap();
    let tmp_path = tmp_dir.path().to_path_buf();

    let ident = &binary.spec.ident;

    let config = PipelineConfig {
        playbook: ident.clone(),
        remote: HashMap::new(),
        wal_capacity: 2,
        success_log_retention_secs: 3600,
        error_log_retention_secs: 86400 * 7,
        input_dir: tmp_path.clone(),
        output_dir: tmp_path.clone(),
        log_dir: tmp_path.clone(),
    };

    let loaded = config.load(&cache).await.unwrap();
    let pipeline_factory = loaded.factory().unwrap();
    let mut pipeline = pipeline_factory.build_session_diff().await.unwrap();

    let session_id = 42;
    pipeline
        .prep_session(session_id, &[], &[])
        .await
        .expect("Should prep session");

    // --- Turn 1 ---
    let turn1_input = PyroRow::from([("input", "Hello!".into())]);
    let result1 = pipeline
        .call(session_id, &turn1_input)
        .await
        .expect("Session call turn 1 should succeed");

    match result1 {
        SessionDiffExecutionRecord::Success { success: row, .. } => {
            assert_eq!(row.get_str("message").unwrap(), "Hello! Turn 1");
        }
        other => panic!("Expected Success, got {:?}", other),
    }

    // --- Turn 2 ---
    let turn2_input = PyroRow::from([("input", "How are you?".into())]);
    let result2 = pipeline
        .call(session_id, &turn2_input)
        .await
        .expect("Session call turn 2 should succeed");

    match result2 {
        SessionDiffExecutionRecord::Success { success: row, .. } => {
            assert_eq!(row.get_str("message").unwrap(), "Goodbye! Turn 2");
        }
        other => panic!("Expected Success, got {:?}", other),
    }

    // --- Session ID 43 (Terminate flow) ---
    let session_id_term = 43;
    pipeline
        .prep_session(session_id_term, &[], &[])
        .await
        .expect("Should prep session 43");

    // --- Turn 1 ---
    let turn1_input_t = PyroRow::from([("input", "Hello!".into())]);
    let result1_t = pipeline
        .call(session_id_term, &turn1_input_t)
        .await
        .expect("Session call turn 1 should succeed");

    match result1_t {
        SessionDiffExecutionRecord::Success { success: row, .. } => {
            assert_eq!(row.get_str("message").unwrap(), "Hello! Turn 1");
        }
        other => panic!("Expected Success, got {:?}", other),
    }

    // --- Turn 2 ---
    let turn2_input_t = PyroRow::from([("input", "terminate".into())]);
    let result2_t = pipeline
        .call(session_id_term, &turn2_input_t)
        .await
        .expect("Session call turn 2 should succeed");

    match result2_t {
        SessionDiffExecutionRecord::Success { success: row, .. } => {
            assert!(row.is_empty());
        }
        other => panic!("Expected Success, got {:?}", other),
    }

    let (in_len, out_len) = pipeline
        .session_lengths(session_id)
        .expect("Session should exist");
    assert_eq!(in_len, 2);
    assert_eq!(out_len, 2);

    let (in_len_t, out_len_t) = pipeline
        .session_lengths(session_id_term)
        .expect("Session should exist");
    assert_eq!(in_len_t, 2);
    assert_eq!(out_len_t, 2);

    // Trigger log rotation
    let mut rot_session_id = 10;
    for _ in 0..15 {
        let input = PyroRow::from([("input", "rot".into())]);
        pipeline.call(rot_session_id, &input).await.unwrap();
        pipeline.close_session(rot_session_id).await.unwrap();
        rot_session_id += 1;
    }

    // Verify log rotation
    let log_files = std::fs::read_dir(&tmp_path)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "pyrolog"))
        .count();
    assert!(
        log_files > 1,
        "Should have rotated logs, but found only {} file(s)",
        log_files
    );

    // Verify active_sessions and get_session
    let active = pipeline.active_sessions();
    assert!(
        active.is_empty(),
        "Expected no active sessions at this point, got {:?}",
        active
    );

    // Make 2 active sessions and call each once so they return Continue and remain active
    let s1 = 100;
    let s2 = 101;
    pipeline
        .prep_session(s1, &[], &[])
        .await
        .expect("Should prep session 100");
    pipeline
        .prep_session(s2, &[], &[])
        .await
        .expect("Should prep session 101");

    pipeline
        .call(s1, &PyroRow::from([("input", "Active 1".into())]))
        .await
        .expect("Call s1");
    pipeline
        .call(s2, &PyroRow::from([("input", "Active 2".into())]))
        .await
        .expect("Call s2");

    // Assert that active_sessions returns these two active sessions
    let active = pipeline.active_sessions();
    assert_eq!(active, vec![s1, s2]);

    // Assert get on active session retrieves correct row history (using get)
    let s1_record = pipeline.get(s1).await.unwrap();
    match s1_record {
        SessionDiffExecutionRecord::Success {
            prior_input,
            prior_output,
            input,
            success,
            ..
        } => {
            assert!(prior_input.is_empty());
            assert!(prior_output.is_empty());
            assert_eq!(input.get_str("input").unwrap(), "Active 1");
            assert_eq!(success.get_str("message").unwrap(), "Hello! Turn 1");
        }
        other => panic!("Expected Success, got {:?}", other),
    }

    // Assert get on finished session 42 correctly retrieves execution record from database
    let record = pipeline.get(session_id).await.unwrap();
    match record {
        SessionDiffExecutionRecord::Success {
            prior_input,
            prior_output,
            input,
            success,
            ..
        } => {
            assert_eq!(prior_input.len(), 1);
            assert_eq!(prior_output.len(), 1);
            assert_eq!(prior_input[0].get_str("input").unwrap(), "Hello!");
            assert_eq!(prior_output[0].get_str("message").unwrap(), "Hello! Turn 1");
            assert_eq!(input.get_str("input").unwrap(), "How are you?");
            assert_eq!(success.get_str("message").unwrap(), "Goodbye! Turn 2");
        }
        other => panic!("Expected Success, got {:?}", other),
    }

    pipeline
        .close_session(s1)
        .await
        .expect("Should close session s1");
    pipeline
        .close_session(s2)
        .await
        .expect("Should close session s2");

    pipeline
        .close_session(session_id)
        .await
        .expect("Should close session");
    pipeline
        .close_session(session_id_term)
        .await
        .expect("Should close session term");
}
