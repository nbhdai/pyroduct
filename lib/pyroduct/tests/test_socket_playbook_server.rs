use pyro_artifacts::{
    build::{AnonPlaybook, Builder},
    cache::CacheManager,
};
use pyroduct::{
    PyroRow,
    pipeline::PipelineConfig,
    transport::socket::{
        PyroListener,
        playbook::{PlaybookClient, PlaybookServer},
    },
};
use std::collections::{BTreeMap, HashMap};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

const CODE: &str = r#"
use pyroduct;

#[pyroduct::module(output = message)]
pub fn call(input: String) -> Result<String> {
    if input == "panic" {
        panic!("intentional panic");
    }
    if input == "error" {
        return Err(pyroduct::capture!("intentional error"));
    }
    Ok(format!("Success: {}", input))
}
"#;

#[tokio::test]
async fn test_playbook_server_client() {
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
        name: "test_socket_playbook".to_string(),
        dependencies: BTreeMap::new(),
        configurations: Vec::new(),
        source: CODE.to_string(),
    };

    let binary = builder
        .compile_anon(&source)
        .await
        .expect("Valid module should compile");

    let ident = &binary.spec.ident;

    let config = PipelineConfig {
        playbook_author: ident.author.clone(),
        playbook_name: ident.name.clone(),
        playbook_version: ident.version.clone(),
        remote: HashMap::new(),
        wal_capacity: 1000,
        success_log_retention_secs: 3600,
        error_log_retention_secs: 86400 * 7,
        input_dir: std::env::current_dir().unwrap(),
        output_dir: std::env::current_dir().unwrap(),
        log_dir: std::env::current_dir().unwrap(),
    };

    let config = config.load(&cache).await.unwrap();
    let loaded_playbook = &config.playbook;

    let server = PlaybookServer::new(loaded_playbook)
        .await
        .expect("Failed to create server");

    let listener = PyroListener::bind_tcp("127.0.0.1:0")
        .await
        .expect("Failed to bind listener");
    let addr = listener.local_addr_tcp().expect("Failed to get local addr");

    // Run server in background
    tokio::spawn(async move {
        if let Err(e) = server.run(listener).await {
            eprintln!("Server error: {:?}", e);
        }
    });

    // Client Connection
    let mut client = PlaybookClient::connect_tcp(addr)
        .await
        .expect("Failed to connect to server");

    // Send first request
    let result1 = client
        .call(&PyroRow::from([("input", "0".into())]))
        .await
        .expect("Failed to call playbook");

    // Result should be the transform message
    assert_eq!(result1.row.get_str("message").unwrap(), "Success: 0");

    // Send second request
    let result2 = client
        .call(&PyroRow::from([("input", "1".into())]))
        .await
        .expect("Failed to call playbook");

    // Result should be the transform message
    assert_eq!(result2.row.get_str("message").unwrap(), "Success: 1");
}
