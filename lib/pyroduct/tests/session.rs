use pyro_artifacts::{
    artifacts::{ModuleDependencies, ModuleSource, Playbook},
    build::Builder,
    cache::CacheManager,
};
use pyroduct::module::sessions::SessionResult;
use pyroduct::{PyroRow, pipeline::PipelineConfig};
use tracing_subscriber::EnvFilter;
use std::collections::HashMap;

/// A session module that returns a single string field.
const SIMPLE_SESSION_MODULE: &str = r#"
// Session module v2
use pyroduct::session::SessionResponse;

#[pyroduct::module(session, output = message)]
fn counter(
    prior: Vec<String>,
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

    let tmp_dir = tempfile::tempdir().unwrap();
    let tmp_path = tmp_dir.path().to_path_buf();

    let config = PipelineConfig {
        playbook: Playbook {
            hash: binary.hash(),
            configurations: HashMap::new(),
        },
        wal_capacity: 1000,
        success_log_retention_secs: 3600,
        error_log_retention_secs: 86400 * 7,
        input_dir: tmp_path.clone(),
        output_dir: tmp_path.clone(),
        log_dir: tmp_path.clone(),
    };

    let loaded = config.load(&cache).await.unwrap();
    let pipeline_factory = loaded.factory().unwrap();
    let mut pipeline = pipeline_factory.build_session().await.unwrap();

    let id_a = 100;
    let id_b = 200;
    pipeline.prep_session(id_a, &[]).await.unwrap();
    pipeline.prep_session(id_b, &[]).await.unwrap();

    let input_a = PyroRow::from([("input", "Hi A".into())]);
    let input_b = PyroRow::from([("input", "Hi B".into())]);

    let res_a = pipeline.call(id_a, &input_a).await.unwrap();
    let res_b = pipeline.call(id_b, &input_b).await.unwrap();

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
    let res_a2 = pipeline.call(id_a, &input_a2).await.unwrap();
    if let SessionResult::End(row) = res_a2 {
        assert_eq!(row.get_str("message").unwrap(), "Goodbye! Turn 2");
    } else {
        panic!("A should end")
    }

    let input_b2 = PyroRow::from([("input", "B2".into())]);
    let res_b2 = pipeline.call(id_b, &input_b2).await.unwrap();
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
fn error_fn<'a>(_prior_input: Vec<String>, _prior_output: Vec<String>, input: String) -> Result<SessionResponse<String>> {
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

    let tmp_dir = tempfile::tempdir().unwrap();
    let tmp_path = tmp_dir.path().to_path_buf();

    let config = PipelineConfig {
        playbook: Playbook {
            hash: binary.hash(),
            configurations: HashMap::new(),
        },
        wal_capacity: 1000,
        success_log_retention_secs: 3600,
        error_log_retention_secs: 86400 * 7,
        input_dir: tmp_path.clone(),
        output_dir: tmp_path.clone(),
        log_dir: tmp_path.clone(),
    };

    let loaded = config.load(&cache).await.unwrap();
    let pipeline_factory = loaded.factory().unwrap();
    let mut pipeline = pipeline_factory.build_session().await.unwrap();

    let session_id = 99;
    pipeline.prep_session(session_id, &[]).await.unwrap();

    let input = PyroRow::from([("input", "test".into())]);
    let result = pipeline.call(session_id, &input).await;

    assert!(result.is_err(), "Should fail");

    pipeline.close_session(session_id).await.unwrap();
}

fn init_tracing() {
    let _ = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "trace,cranelift_frontend=off,cranelift_codegen=off,wasmtime=off".into()
    });
}
