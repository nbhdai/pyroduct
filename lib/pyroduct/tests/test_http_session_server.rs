use pyro_artifacts::{
    build::{AnonPlaybook, Builder},
    cache::CacheManager,
};
use pyroduct::{
    pipeline::{PipelineConfig, PipelineServer},
    transport::http::run as run_http,
};
use std::collections::{BTreeMap, HashMap};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

const CHAT_SESSION_MODULE: &str = r#"
use pyroduct::{session::SessionResponse, tracing};

#[pyroduct::magma]
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[pyroduct::module(session, output = ChatMessage)]
fn counter(
    prior: Vec<ChatMessage>,
    input: ChatMessage,
) -> Result<SessionResponse<ChatMessage>> {
    tracing::info!(?prior, ?input, "Calling");
    let turn = (prior.len() as u32 + 1) / 2;

    match turn {
        0 => Ok(SessionResponse::Continue(ChatMessage {
            role: "assistant".to_string(),
            content: format!("Hello! Turn {}", turn + 1),
        })),
        1 => Ok(SessionResponse::End(ChatMessage {
            role: "assistant".to_string(),
            content: format!("Goodbye! Turn {}", turn + 1),
        })),
        _ => Ok(SessionResponse::Terminate),
    }
}
"#;

async fn post_json(
    addr: std::net::SocketAddr,
    path: &str,
    body: &serde_json::Value,
) -> (u16, String) {
    let client = reqwest::Client::new();
    let url = format!("http://{}{}", addr, path);
    let resp = client.post(&url).json(body).send().await.unwrap();
    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap();
    (status, text)
}

#[tokio::test]
async fn test_http_session_bare_json() {
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
        package: "test_http_session".to_string(),
        dependencies: BTreeMap::new(),
        configurations: Vec::new(),
        source: CHAT_SESSION_MODULE.to_string(),
        interconnect: BTreeMap::new(),
    };
    cache
        .remove_module("anon", "test_http_session", "0.1.0")
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
        wal_capacity: 1000,
        success_log_retention_secs: 3600,
        error_log_retention_secs: 86400 * 7,
        num_workers: 4,
        input_dir: tmp_path.clone(),
        output_dir: tmp_path.clone(),
        log_dir: tmp_path.clone(),
    };

    let config = config.load(&cache).await.unwrap();
    let server = PipelineServer::new(&config.playbook)
        .await
        .expect("Failed to create server");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind listener");
    let addr = listener.local_addr().expect("Failed to get local addr");

    let shutdown_tx = run_http(server, listener);

    // --- Turn 1: bare JSON, no "input" wrapper ---
    let (status, body) = post_json(
        addr,
        "/",
        &serde_json::json!({"role": "user", "content": "Hello!"}),
    )
    .await;

    eprintln!("Turn 1 response ({}): {}", status, body);
    assert_eq!(status, 200);
    let resp: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(resp["role"], "assistant");
    assert_eq!(resp["content"], "Hello! Turn 1");
    let session_id = resp["session_id"].as_u64().expect("Should have session_id");

    // --- Turn 2: continue the session ---
    let (status, body) = post_json(
        addr,
        &format!("/{}", session_id),
        &serde_json::json!({"role": "user", "content": "How are you?"}),
    )
    .await;

    eprintln!("Turn 2 response ({}): {}", status, body);
    assert_eq!(status, 200);
    let resp: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(resp["role"], "assistant");
    assert_eq!(resp["content"], "Goodbye! Turn 2");

    let _ = shutdown_tx.send(());
    drop(tmp_dir);
}
