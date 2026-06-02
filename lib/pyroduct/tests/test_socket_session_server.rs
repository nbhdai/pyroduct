use pyro_artifacts::{
    build::{AnonPlaybook, Builder},
    cache::CacheManager,
};
use pyroduct::{
    PyroRow,
    pipeline::{PipelineConfig, PipelineServer},
    transport::socket::{
        PyroListener,
        playbook::{PlaybookClient, run as run_socket},
    },
};
use std::collections::HashMap;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

const SIMPLE_SESSION_MODULE: &str = r#"
// Session module v2
use pyroduct::{session::SessionResponse, tracing};

#[pyroduct::module(session, output = message)]
fn counter(
    prior: Vec<String>,
    input: String,
) -> Result<SessionResponse<String>> {
    tracing::info!(?prior, input, "Calling");
    let turn = (prior.len() as u32 + 1) / 2;

    match turn {
        0 => Ok(SessionResponse::Continue(format!("Hello! Turn {}", turn + 1))),
        1 => Ok(SessionResponse::End(format!("Goodbye! Turn {}", turn + 1))),
        _ => Ok(SessionResponse::Terminate),
    }
}
"#;

#[tokio::test]
async fn test_socket_session_server_client() {
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
        package: "test_socket_session".to_string(),
        dependencies: std::collections::BTreeMap::new(),
        configurations: Vec::new(),
        source: SIMPLE_SESSION_MODULE.to_string(),
    };
    cache.remove_module("anon", "test_socket_session", "0.1.0").await.unwrap();

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
        wal_capacity: 1000,
        success_log_retention_secs: 3600,
        error_log_retention_secs: 86400 * 7,
        input_dir: tmp_path.clone(),
        output_dir: tmp_path.clone(),
        log_dir: tmp_path.clone(),
    };

    let config = config.load(&cache).await.unwrap();
    let loaded_playbook = &config.playbook;

    let server = PipelineServer::new(loaded_playbook)
        .await
        .expect("Failed to create server");

    let listener = PyroListener::bind_tcp("127.0.0.1:0")
        .await
        .expect("Failed to bind listener");
    let addr = listener.local_addr_tcp().expect("Failed to get local addr");

    let shutdown_tx = run_socket(server, listener);

    // Client Connection
    let mut client = PlaybookClient::connect_tcp(addr)
        .await
        .expect("Failed to connect to server");

    let session_id_1 = 12345;
    let session_id_2 = 67890;

    // --- Session 1: Turn 1 ---
    let result1_t1 = client
        .call_session(
            session_id_1,
            &PyroRow::from([("input", "Hello S1!".into())]),
        )
        .await
        .expect("Session 1 call turn 1 should succeed");
    assert_eq!(result1_t1.row.get_str("message").unwrap(), "Hello! Turn 1");

    // --- Session 2: Turn 1 ---
    let result2_t1 = client
        .call_session(
            session_id_2,
            &PyroRow::from([("input", "Hello S2!".into())]),
        )
        .await
        .expect("Session 2 call turn 1 should succeed");
    assert_eq!(result2_t1.row.get_str("message").unwrap(), "Hello! Turn 1");

    // --- Session 1: Turn 2 ---
    let result1_t2 = client
        .call_session(
            session_id_1,
            &PyroRow::from([("input", "S1 Turn 2".into())]),
        )
        .await
        .expect("Session 1 call turn 2 should succeed");
    assert_eq!(
        result1_t2.row.get_str("message").unwrap(),
        "Goodbye! Turn 2"
    );

    // --- Session 2: Turn 2 ---
    let result2_t2 = client
        .call_session(
            session_id_2,
            &PyroRow::from([("input", "S2 Turn 2".into())]),
        )
        .await
        .expect("Session 2 call turn 2 should succeed");
    assert_eq!(
        result2_t2.row.get_str("message").unwrap(),
        "Goodbye! Turn 2"
    );

    // --- Session 1: Turn 3 ---
    let result1_t3 = client
        .call_session(
            session_id_1,
            &PyroRow::from([("input", "S1 Terminate".into())]),
        )
        .await
        .expect("Session 1 call turn 3 should succeed");
    assert!(result1_t3.row.is_empty());

    // --- Session 2: Turn 3 ---
    let result2_t3 = client
        .call_session(
            session_id_2,
            &PyroRow::from([("input", "S2 Terminate".into())]),
        )
        .await
        .expect("Session 2 call turn 3 should succeed");
    assert!(result2_t3.row.is_empty());

    let _ = shutdown_tx.send(());
}
