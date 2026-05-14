use pyro_artifacts::{
    artifacts::{ModuleDependencies, ModuleSource, Playbook},
    build::Builder,
    cache::CacheManager,
};
use pyroduct::module::sessions::{Session, SessionResult};
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

/// Helper to push input and call a session, splitting borrows to avoid conflicts.
async fn push_and_call(
    session: &mut Session,
    instance: &mut pyroduct::module::PyroInstance,
    input: &PyroRow<'_>,
) -> Result<SessionResult, pyroduct::module::sessions::SessionCallError> {
    let memory = instance.memory();
    let instance_clone = instance.instance().clone();
    let store = &mut instance.store_mut();

    eprintln!("[DEBUG] push_input: session_id={}, input_count={}", session.session_id(), session.input_count());

    session
        .push_input(store, &instance_clone, memory, input)
        .await
        .map_err(|e| pyroduct::module::sessions::SessionCallError {
            error: pyroduct::format::PyroFailure {
                result: Err(e.to_string()),
                logs: pyroduct::format::PyroLogs::empty(),
            },
        })?;

    eprintln!("[DEBUG] call: session_id={}, input_count={}, output_count={}",
        session.session_id(), session.input_count(), session.output_count());

    session.call(store, &instance_clone, memory).await
}

/// Helper to free a session, splitting borrows.
async fn free_session(
    session: &Session,
    instance: &mut pyroduct::module::PyroInstance,
) -> Result<(), pyroduct::module::WasmError> {
    // Need to get immutable ref first, then release it before mutable borrow
    let instance_ref = instance.instance().clone();
    session.free(&mut instance.store_mut(), &instance_ref).await
}

/// Test that a session module can be compiled, instantiated, and called via the Session harness.
/// This tests the full lifecycle: push_input → call → continue/end/terminate → free.
#[tokio::test]
async fn test_session_lifecycle() {
    init_tracing();

    let cache = std::sync::Arc::new(CacheManager::from_env().await.unwrap());
    let builder = Builder::from_env(cache.clone()).await.unwrap();

    // Compile the session module (no capabilities)
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

    // Create a pipeline config with no capability configurations
    let config = PipelineConfig {
        playbook: Playbook {
            hash: binary.hash(),
            configurations: HashMap::new(),
        },
        wal_capacity: 1000,
        success_log_retention_secs: 3600,
        error_log_retention_secs: 86400 * 7,
        output_dir: std::env::current_dir().unwrap(),
    };

    let loaded = config.load(&cache).await.unwrap();
    let pipeline_factory = loaded.factory().unwrap();
    let mut instance = pipeline_factory.factory.instantiate().await.unwrap();

    // Create a session
    let session_id = 42;
    let mut session = Session::new(session_id);

    // --- Turn 1: User sends initial message ---
    let turn1_input = PyroRow::from([("input", "Hello!".into())]);

    let result1 = push_and_call(&mut session, &mut instance, &turn1_input)
        .await
        .expect("Session call turn 1 should succeed");

    match result1 {
        SessionResult::Continue(row) => {
            let message = row.get_str("message").unwrap();
            assert_eq!(message, "Hello! Turn 1");
        }
        other => panic!("Expected Continue, got {:?}", other),
    }

    // --- Turn 2: User sends follow-up ---
    let turn2_input = PyroRow::from([("input", "How are you?".into())]);

    let result2 = push_and_call(&mut session, &mut instance, &turn2_input)
        .await
        .expect("Session call turn 2 should succeed");

    match result2 {
        SessionResult::End(row) => {
            let message = row.get_str("message").unwrap();
            assert_eq!(message, "Goodbye! Turn 2");
        }
        other => panic!("Expected End, got {:?}", other),
    }

    // --- Try turn 3: Should terminate ---
    let turn3_input = PyroRow::from([("input", "Wait, don't go!".into())]);

    let result3 = push_and_call(&mut session, &mut instance, &turn3_input)
        .await
        .expect("Session call turn 3 should succeed");

    match result3 {
        SessionResult::Terminate => {
            // Session terminated as expected
        }
        other => panic!("Expected Terminate, got {:?}", other),
    }

    // Verify session state tracking
    assert_eq!(session.input_count(), 3);
    assert_eq!(session.output_count(), 3);

    // Free the session
    free_session(&session, &mut instance)
        .await
        .expect("Should free session");
}

/// Test that multiple independent sessions maintain separate state.
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
        .expect("Valid session module should compile");

    let config = PipelineConfig {
        playbook: Playbook {
            hash: binary.hash(),
            configurations: HashMap::new(),
        },
        wal_capacity: 1000,
        success_log_retention_secs: 3600,
        error_log_retention_secs: 86400 * 7,
        output_dir: std::env::current_dir().unwrap(),
    };

    let loaded = config.load(&cache).await.unwrap();
    let pipeline_factory = loaded.factory().unwrap();
    let mut instance = pipeline_factory.factory.instantiate().await.unwrap();

    // Create two independent sessions
    let mut session_a = Session::new(100);
    let mut session_b = Session::new(200);

    // Both sessions start with turn 1
    let input_a = PyroRow::from([("input", "Hi A".into())]);
    let input_b = PyroRow::from([("input", "Hi B".into())]);

    let result_a = push_and_call(&mut session_a, &mut instance, &input_a)
        .await
        .expect("Session A turn 1 should succeed");

    let result_b = push_and_call(&mut session_b, &mut instance, &input_b)
        .await
        .expect("Session B turn 1 should succeed");

    match result_a {
        SessionResult::Continue(row) => {
            let message = row.get_str("message").unwrap();
            assert_eq!(message, "Hello! Turn 1");
        }
        other => panic!("Session A: Expected Continue, got {:?}", other),
    }

    match result_b {
        SessionResult::Continue(row) => {
            let message = row.get_str("message").unwrap();
            assert_eq!(message, "Hello! Turn 1");
        }
        other => panic!("Session B: Expected Continue, got {:?}", other),
    }

    // Advance session A to turn 2, but keep session B at turn 1
    let input_a2 = PyroRow::from([("input", "A2".into())]);

    let result_a2 = push_and_call(&mut session_a, &mut instance, &input_a2)
        .await
        .expect("Session A turn 2 should succeed");

    match result_a2 {
        SessionResult::End(row) => {
            let message = row.get_str("message").unwrap();
            assert_eq!(message, "Goodbye! Turn 2");
        }
        other => panic!("Session A: Expected End, got {:?}", other),
    }

    // Session B should still be at turn 1 - calling it again gives turn 2
    let input_b2 = PyroRow::from([("input", "B2".into())]);

    let result_b2 = push_and_call(&mut session_b, &mut instance, &input_b2)
        .await
        .expect("Session B turn 2 should succeed");

    match result_b2 {
        SessionResult::End(row) => {
            let message = row.get_str("message").unwrap();
            assert_eq!(message, "Goodbye! Turn 2");
        }
        other => panic!("Session B: Expected End, got {:?}", other),
    }

    // Clean up
    free_session(&session_a, &mut instance)
        .await
        .expect("Should free session A");
    free_session(&session_b, &mut instance)
        .await
        .expect("Should free session B");
}

/// Test session error handling: module returns an error.
#[tokio::test]
async fn test_session_error_handling() {
    init_tracing();

    let cache = std::sync::Arc::new(CacheManager::from_env().await.unwrap());
    let builder = Builder::from_env(cache.clone()).await.unwrap();

    // Module that always errors
    const ERROR_MODULE: &str = r#"
use pyroduct::session::SessionResponse;

#[pyroduct::module(session, output = message)]
fn error_fn<'a>(
    _prior_input: Vec<String>,
    _prior_output: Vec<String>,
    input: String,
) -> Result<SessionResponse<String>> {
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
        .expect("Error module should compile");

    let config = PipelineConfig {
        playbook: Playbook {
            hash: binary.hash(),
            configurations: HashMap::new(),
        },
        wal_capacity: 1000,
        success_log_retention_secs: 3600,
        error_log_retention_secs: 86400 * 7,
        output_dir: std::env::current_dir().unwrap(),
    };

    let loaded = config.load(&cache).await.unwrap();
    let pipeline_factory = loaded.factory().unwrap();
    let mut instance = pipeline_factory.factory.instantiate().await.unwrap();

    let mut session = Session::new(99);

    let input = PyroRow::from([("input", "test".into())]);

    let result = push_and_call(&mut session, &mut instance, &input).await;

    // Should return an error
    assert!(result.is_err(), "Session call with error-producing module should fail");

    // Clean up
    free_session(&session, &mut instance)
        .await
        .expect("Should free session even after error");
}

/// Initialize tracing subscriber for tests (called once per test).
/// Each test function calls this to set up its own tracing context.
fn init_tracing() {
    let _ = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "trace,cranelift_frontend=off,cranelift_codegen=off,wasmtime=off".into()
    });

    // Don't set global default - it can only be done once.
    // Instead, just let tests use the default tracing behavior.
}
