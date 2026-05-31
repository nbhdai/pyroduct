use pyro_artifacts::{
    build::{AnonPlaybook, Builder},
    cache::CacheManager,
};
use pyroduct::module::sessions::SessionResult;
use pyroduct::{
    PyroRow,
    pipeline::{
        PipelineConfig, session::SessionExecutionRecord, session_diff::SessionDiffExecutionRecord,
    },
};
use std::collections::HashMap;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// A session module that returns a single string field.
const RECOVERY_SESSION_MODULE: &str = r#"
// Session module v2
use pyroduct::{session::SessionResponse, tracing};

#[pyroduct::module(session, output = message)]
fn counter(
    prior: Vec<String>,
    input: String,
) -> Result<SessionResponse<String>> {
    tracing::info!(?prior, input, "Calling");
    if input == "panic" {
        panic!("intentional panic");
    }
    if input == "error" {
        return Err(pyroduct::capture!("intentional error"));
    }
    let turn = (prior.len() as u32 + 1) / 2;

    match turn {
        0 => Ok(SessionResponse::Continue(format!("Hello! Turn {}", turn + 1))),
        1 => Ok(SessionResponse::End(format!("Goodbye! Turn {}", turn + 1))),
        _ => Ok(SessionResponse::Terminate),
    }
}
"#;

/// A session diff module that returns a single string field.
const RECOVERY_SESSION_DIFF_MODULE: &str = r#"
// Session module v2
use pyroduct::session::SessionResponse;

#[pyroduct::module(session, output = message)]
fn counter(
    prior_input: Vec<String>,
    prior_output: Vec<String>,
    input: String,
) -> Result<SessionResponse<String>> {
    if input == "panic" {
        panic!("intentional panic");
    }
    if input == "error" {
        return Err(pyroduct::capture!("intentional error"));
    }
    let turn = prior_output.len() as u32 + 1;

    match turn {
        1 => Ok(SessionResponse::Continue(format!("Hello! Turn {}", turn))),
        2 => Ok(SessionResponse::End(format!("Goodbye! Turn {}", turn))),
        _ => Ok(SessionResponse::Terminate),
    }
}
"#;

#[tokio::test]
async fn test_session_recovery_lifecycle() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "trace,cranelift_frontend=off,cranelift_codegen=off,wasmtime=off".into()
    });

    let _ = tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).pretty())
        .with(filter)
        .try_init();

    let cache = std::sync::Arc::new(CacheManager::from_env().await.unwrap());
    let builder = Builder::from_env(cache.clone()).await.unwrap();

    let source = AnonPlaybook {
        package: "test_session_rec".to_string(),
        dependencies: std::collections::BTreeMap::new(),
        configurations: Vec::new(),
        source: RECOVERY_SESSION_MODULE.to_string(),
    };
    cache
        .remove_module("anon", "test_session_rec", "0.1.0")
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
    let mut pipeline = pipeline_factory.build_session().await.unwrap();

    // =========================================================================
    // Scenario 1: Successful multi-turn session (Active -> Succeeded)
    // =========================================================================
    let session_id_1 = 100;
    pipeline
        .prep_session(session_id_1, &[])
        .await
        .expect("Should prep session 1");

    // Turn 1: Continue
    let turn1_input = PyroRow::from([("input", "Hello!".into())]);
    let result1 = pipeline
        .call(session_id_1, &turn1_input)
        .await
        .expect("Call 1 should succeed");

    assert!(matches!(result1, SessionResult::Continue { .. }));

    // Verify state is "active" in SQLite status
    let status_active = pipeline
        .output_manager
        .get_session_status(session_id_1 as usize)
        .unwrap();
    assert_eq!(status_active, Some("active".to_string()));

    // Verify get_record returns Success
    let rec_active = pipeline.get_record(session_id_1).await.unwrap();
    match rec_active {
        SessionExecutionRecord::Success {
            row_index,
            prior,
            input,
            success,
            ..
        } => {
            assert_eq!(row_index, session_id_1 as usize);
            assert!(prior.is_empty());
            assert_eq!(input.get_str("input").unwrap(), "Hello!");
            assert_eq!(success.get_str("message").unwrap(), "Hello! Turn 1");
        }
        other => panic!("Expected Success execution record, got {:?}", other),
    }

    // Turn 2: End
    let turn2_input = PyroRow::from([("input", "How are you?".into())]);
    let result2 = pipeline
        .call(session_id_1, &turn2_input)
        .await
        .expect("Call 2 should succeed");

    assert!(matches!(result2, SessionResult::End { .. }));

    // Verify state is "succeeded" in SQLite status
    let status_success = pipeline
        .output_manager
        .get_session_status(session_id_1 as usize)
        .unwrap();
    assert_eq!(status_success, Some("succeeded".to_string()));

    // Verify get_record returns Success with rolled-up state
    let rec_succeeded = pipeline.get_record(session_id_1).await.unwrap();
    match rec_succeeded {
        SessionExecutionRecord::Success {
            row_index,
            prior,
            input,
            success,
            ..
        } => {
            assert_eq!(row_index, session_id_1 as usize);
            assert_eq!(prior.len(), 2);
            assert_eq!(prior[0].get_str("input").unwrap(), "Hello!");
            assert_eq!(prior[1].get_str("message").unwrap(), "Hello! Turn 1");
            assert_eq!(input.get_str("input").unwrap(), "How are you?");
            assert_eq!(success.get_str("message").unwrap(), "Goodbye! Turn 2");
        }
        other => panic!("Expected Success execution record, got {:?}", other),
    }

    // =========================================================================
    // Scenario 2: Session fails with an error (Active -> Failed)
    // =========================================================================
    let session_id_2 = 200;
    pipeline
        .prep_session(session_id_2, &[])
        .await
        .expect("Should prep session 2");

    let error_input = PyroRow::from([("input", "error".into())]);
    let result_err = pipeline.call(session_id_2, &error_input).await;
    assert!(result_err.is_err());

    // Verify state is "failed" in SQLite status
    let status_failed_err = pipeline
        .output_manager
        .get_session_status(session_id_2 as usize)
        .unwrap();
    assert_eq!(status_failed_err, Some("failed".to_string()));

    // Verify get_record returns Failure containing error description
    let rec_failed_err = pipeline.get_record(session_id_2).await.unwrap();
    match rec_failed_err {
        SessionExecutionRecord::Failure {
            row_index,
            prior,
            input,
            failure,
            ..
        } => {
            assert_eq!(row_index, session_id_2 as usize);
            assert!(prior.is_empty());
            assert_eq!(input.get_str("input").unwrap(), "error");
            assert!(failure.is_ok());
            assert_eq!(
                failure.unwrap().to_string(),
                "Error at src/lib.rs:15:20 - intentional error"
            );
        }
        other => panic!("Expected Failure execution record, got {:?}", other),
    }

    // =========================================================================
    // Scenario 3: Session fails with a panic (Active -> Failed)
    // =========================================================================
    let session_id_3 = 300;
    pipeline
        .prep_session(session_id_3, &[])
        .await
        .expect("Should prep session 3");

    let panic_input = PyroRow::from([("input", "panic".into())]);
    let result_panic = pipeline.call(session_id_3, &panic_input).await;
    assert!(result_panic.is_err());

    // Verify state is "failed" in SQLite status
    let status_failed_panic = pipeline
        .output_manager
        .get_session_status(session_id_3 as usize)
        .unwrap();
    assert_eq!(status_failed_panic, Some("failed".to_string()));

    // Verify get_record returns Failure containing panic details
    let rec_failed_panic = pipeline.get_record(session_id_3).await.unwrap();
    match rec_failed_panic {
        SessionExecutionRecord::Failure {
            row_index,
            prior,
            input,
            failure,
            ..
        } => {
            assert_eq!(row_index, session_id_3 as usize);
            assert!(prior.is_empty());
            assert_eq!(input.get_str("input").unwrap(), "panic");
            assert!(failure.is_ok());
            let captured_err = failure.unwrap();
            assert!(
                captured_err
                    .to_string()
                    .contains("Error at src/lib.rs:12:9 - intentional panic")
            );
        }
        other => panic!("Expected Failure execution record, got {:?}", other),
    }
}

#[tokio::test]
async fn test_session_diff_recovery_lifecycle() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "trace,cranelift_frontend=off,cranelift_codegen=off,wasmtime=off".into()
    });

    let _ = tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).pretty())
        .with(filter)
        .try_init();

    let cache = std::sync::Arc::new(CacheManager::from_env().await.unwrap());
    let builder = Builder::from_env(cache.clone()).await.unwrap();

    let source = AnonPlaybook {
        package: "test_session_diff_rec".to_string(),
        dependencies: std::collections::BTreeMap::new(),
        configurations: Vec::new(),
        source: RECOVERY_SESSION_DIFF_MODULE.to_string(),
    };
    cache
        .remove_module("anon", "test_session_diff_rec", "0.1.0")
        .await
        .unwrap();

    let binary = builder
        .compile_anon(&source)
        .await
        .expect("Valid session diff module should compile");

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

    // =========================================================================
    // Scenario 1: Successful multi-turn session diff (Active -> Succeeded)
    // =========================================================================
    let session_id_1 = 400;
    pipeline
        .prep_session(session_id_1, &[], &[])
        .await
        .expect("Should prep session diff 1");

    // Turn 1: Continue
    let turn1_input = PyroRow::from([("input", "Hello!".into())]);
    let result1 = pipeline
        .call(session_id_1, &turn1_input)
        .await
        .expect("Call 1 should succeed");

    assert!(matches!(result1, SessionResult::Continue { .. }));

    // Verify state is "active" in SQLite status
    let status_active = pipeline
        .output_manager
        .get_session_status(session_id_1 as usize)
        .unwrap();
    assert_eq!(status_active, Some("active".to_string()));

    // Verify get_record returns Success
    let rec_active = pipeline.get_record(session_id_1).await.unwrap();
    match rec_active {
        SessionDiffExecutionRecord::Success {
            row_index,
            prior_input,
            prior_output,
            input,
            success,
            ..
        } => {
            assert_eq!(row_index, session_id_1 as usize);
            assert!(prior_input.is_empty());
            assert!(prior_output.is_empty());
            assert_eq!(input.get_str("input").unwrap(), "Hello!");
            assert_eq!(success.get_str("message").unwrap(), "Hello! Turn 1");
        }
        other => panic!("Expected Success execution record, got {:?}", other),
    }

    // Turn 2: End
    let turn2_input = PyroRow::from([("input", "How are you?".into())]);
    let result2 = pipeline
        .call(session_id_1, &turn2_input)
        .await
        .expect("Call 2 should succeed");

    assert!(matches!(result2, SessionResult::End { .. }));

    // Verify state is "succeeded" in SQLite status
    let status_success = pipeline
        .output_manager
        .get_session_status(session_id_1 as usize)
        .unwrap();
    assert_eq!(status_success, Some("succeeded".to_string()));

    // Verify get_record returns Success with rolled-up state
    let rec_succeeded = pipeline.get_record(session_id_1).await.unwrap();
    match rec_succeeded {
        SessionDiffExecutionRecord::Success {
            row_index,
            prior_input,
            prior_output,
            input,
            success,
            ..
        } => {
            assert_eq!(row_index, session_id_1 as usize);
            assert_eq!(prior_input.len(), 1);
            assert_eq!(prior_output.len(), 1);
            assert_eq!(prior_input[0].get_str("input").unwrap(), "Hello!");
            assert_eq!(prior_output[0].get_str("message").unwrap(), "Hello! Turn 1");
            assert_eq!(input.get_str("input").unwrap(), "How are you?");
            assert_eq!(success.get_str("message").unwrap(), "Goodbye! Turn 2");
        }
        other => panic!("Expected Success execution record, got {:?}", other),
    }

    // =========================================================================
    // Scenario 2: Session fails with an error (Active -> Failed)
    // =========================================================================
    let session_id_2 = 500;
    pipeline
        .prep_session(session_id_2, &[], &[])
        .await
        .expect("Should prep session diff 2");

    let error_input = PyroRow::from([("input", "error".into())]);
    let result_err = pipeline.call(session_id_2, &error_input).await;
    assert!(result_err.is_err());

    // Verify state is "failed" in SQLite status
    let status_failed_err = pipeline
        .output_manager
        .get_session_status(session_id_2 as usize)
        .unwrap();
    assert_eq!(status_failed_err, Some("failed".to_string()));

    // Verify get_record returns Failure containing error description
    let rec_failed_err = pipeline.get_record(session_id_2).await.unwrap();
    match rec_failed_err {
        SessionDiffExecutionRecord::Failure {
            row_index,
            prior_input,
            prior_output,
            input,
            failure,
            ..
        } => {
            assert_eq!(row_index, session_id_2 as usize);
            assert!(prior_input.is_empty());
            assert!(prior_output.is_empty());
            assert_eq!(input.get_str("input").unwrap(), "error");
            assert!(failure.is_ok());
            assert_eq!(
                failure.unwrap().to_string(),
                "Error at src/lib.rs:15:20 - intentional error"
            );
        }
        other => panic!("Expected Failure execution record, got {:?}", other),
    }

    // =========================================================================
    // Scenario 3: Session fails with a panic (Active -> Failed)
    // =========================================================================
    let session_id_3 = 600;
    pipeline
        .prep_session(session_id_3, &[], &[])
        .await
        .expect("Should prep session diff 3");

    let panic_input = PyroRow::from([("input", "panic".into())]);
    let result_panic = pipeline.call(session_id_3, &panic_input).await;
    assert!(result_panic.is_err());

    // Verify state is "failed" in SQLite status
    let status_failed_panic = pipeline
        .output_manager
        .get_session_status(session_id_3 as usize)
        .unwrap();
    assert_eq!(status_failed_panic, Some("failed".to_string()));

    // Verify get_record returns Failure containing panic details
    let rec_failed_panic = pipeline.get_record(session_id_3).await.unwrap();
    match rec_failed_panic {
        SessionDiffExecutionRecord::Failure {
            row_index,
            prior_input,
            prior_output,
            input,
            failure,
            ..
        } => {
            assert_eq!(row_index, session_id_3 as usize);
            assert!(prior_input.is_empty());
            assert!(prior_output.is_empty());
            assert_eq!(input.get_str("input").unwrap(), "panic");
            assert!(failure.is_ok());
            let captured_err = failure.unwrap();
            assert_eq!(
                captured_err.to_string(),
                "Error at src/lib.rs:12:9 - intentional panic"
            );
        }
        other => panic!("Expected Failure execution record, got {:?}", other),
    }
}
