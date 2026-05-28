use pyro_artifacts::{
    artifacts::{ModuleDependencies, ModuleSource, Playbook, CapabilityConfig},
    build::Builder,
    cache::CacheManager,
    cargo::ResolvedCapability,
};
use pyroduct::{pipeline::PipelineConfig, transport::http::PlaybookHttpServer};
use std::collections::{BTreeMap, HashMap};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

const CODE: &str = r#"
//! Test module: Uses state counter capability and receives string input.
use state::{CounterClient, CounterClientMethods};

#[pyroduct::module(output = (count, incremented))]
pub fn call(input: &str) -> Result<(u64, u64)> {
    let start: u64 = input.parse().map_err(|e| format!("Parse error: {}", e))?;

    let client = CounterClient { start_value: start }.register()?;

    let count = client.get_count()?;
    let incremented = client.increment()?;

    Ok((count, incremented))
}
"#;

async fn post_json(
    addr: std::net::SocketAddr,
    path: &str,
    body: &str,
) -> Result<(u16, String), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let url = format!("http://{}{}", addr, path);
    let json_value: serde_json::Value = serde_json::from_str(body)?;
    let resp = client.post(&url).json(&json_value).send().await?;
    let status = resp.status().as_u16();
    let resp_text = resp.text().await?;
    Ok((status, resp_text))
}

#[tokio::test]
async fn test_http_playbook_server_and_repair() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "trace,cranelift_frontend=off,cranelift_codegen=off,wasmtime=off".into()
    });

    let _ = tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).pretty())
        .with(filter)
        .try_init();

    let cache = std::sync::Arc::new(CacheManager::from_env().await.unwrap());
    cache.init().await.unwrap();
    let builder = Builder::from_env(cache.clone()).await.unwrap();

    let source = ModuleSource {
        dependencies: ModuleDependencies {
            dependencies: BTreeMap::new(),
            capabilities: vec![ResolvedCapability {
                package: "state".to_string(),
                author: "nbhdai".to_string(),
                version: "0.1.0".to_string(),
            }],
        },
        source: CODE.to_string(),
        ident: None,
    };

    let binary = builder
        .compile(&source)
        .await
        .expect("Valid module should compile");

    let config = PipelineConfig {
        playbook: Playbook {
            hash: binary.hash(),
            configurations: HashMap::from([(
                "state".to_string(),
                CapabilityConfig {
                    classes: HashMap::from([("CounterClient".to_string(), None)]),
                },
            )]),
        },
        wal_capacity: 1000,
        success_log_retention_secs: 3600,
        error_log_retention_secs: 86400 * 7,
        input_dir: std::env::current_dir().unwrap(),
        output_dir: std::env::current_dir().unwrap(),
        log_dir: std::env::current_dir().unwrap(),
    };

    let config = config.load(&cache).await.unwrap();
    let loaded_playbook = &config.playbook;

    let server = PlaybookHttpServer::new(loaded_playbook)
        .await
        .expect("Failed to create HTTP playbook server");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind listener");
    let addr = listener.local_addr().expect("Failed to get local addr");

    // Run server in background
    tokio::spawn(async move {
        if let Err(e) = server.run(listener).await {
            eprintln!("HTTP Server error: {:?}", e);
        }
    });

    // Test Case 1: Standard, well-formed JSON object query
    let body_ok = r#"{"input": "0"}"#;
    let (status, resp_body) = post_json(addr, "/", body_ok).await.unwrap();
    assert_eq!(status, 200);

    let json_val: serde_json::Value = serde_json::from_str(&resp_body).unwrap();
    assert_eq!(json_val["count"], 0);
    assert_eq!(json_val["incremented"], 0);

    // Test Case 2: JSON query requiring repair (coercion from integer 42 to string "42")
    let body_repair = r#"{"input": 42}"#;
    let (status_repair, resp_body_repair) = post_json(addr, "/", body_repair).await.unwrap();
    assert_eq!(status_repair, 200);

    let json_val_repair: serde_json::Value = serde_json::from_str(&resp_body_repair).unwrap();
    // Since start_value was coerced to 42, count starts at 42 and gets incremented
    assert_eq!(json_val_repair["count"], 1);
    assert_eq!(json_val_repair["incremented"], 43);

    // Test Case 3: Invalid JSON object format (should fail with 400 Bad Request)
    let body_invalid = r#"{"input": {"nested": "not_convertible_to_string"}}"#;
    let (status_invalid, resp_body_invalid) = post_json(addr, "/", body_invalid).await.unwrap();
    assert_eq!(status_invalid, 400);
    assert!(resp_body_invalid.contains("error"));
}
