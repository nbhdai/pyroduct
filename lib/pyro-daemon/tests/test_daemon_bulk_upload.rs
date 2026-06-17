use std::time::Duration;

use pyro_artifacts::{
    build::{AnonPlaybook, Builder},
    cache::CacheManager,
};
use pyro_daemon::client::DaemonClient;
use pyro_daemon::{DaemonRequest, DaemonResponse, PyroDaemon};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::test]
async fn test_daemon_bulk_upload() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "trace,cranelift_frontend=off,cranelift_codegen=off,wasmtime=off".into()
    });

    let _ = tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).pretty())
        .with(filter)
        .try_init();

    // 1. Setup cache & builder to compile the basic module
    let cache = std::sync::Arc::new(CacheManager::from_env().await.unwrap());
    let builder = Builder::from_env(cache.clone()).await.unwrap();

    let basic_package = "test_bulk_upload_basic";
    
    // Read source from modules/basic/src/lib.rs
    let basic_source_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../modules/basic/src/lib.rs");
    let basic_source_code = std::fs::read_to_string(basic_source_path)
        .expect("Failed to read modules/basic/src/lib.rs");

    let basic_source = AnonPlaybook {
        package: basic_package.to_string(),
        dependencies: std::collections::BTreeMap::new(),
        configurations: Vec::new(),
        source: basic_source_code,
        interconnect: std::collections::BTreeMap::new(),
    };

    let _ = cache.remove_module("anon", basic_package, "0.1.0").await;
    let basic_binary = builder
        .compile_anon(&basic_source)
        .await
        .expect("Basic module should compile");

    // 2. Setup temp directory and daemon
    let test_dir = tempfile::tempdir().unwrap();
    let working_dir = test_dir.path().to_path_buf();
    let control_socket = working_dir.join("control");

    // Spawn Daemon
    let daemon = PyroDaemon::new(working_dir.clone());
    let daemon_handle = tokio::spawn(async move {
        daemon.run().await.unwrap();
    });

    // Wait for control socket to bind
    let mut retries = 0;
    while !control_socket.exists() {
        if retries > 50 {
            panic!("Daemon failed to bind control socket in time");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
        retries += 1;
    }

    // Connect
    let client = DaemonClient::connect(&control_socket).await.unwrap();

    // 3. Start the basic playbook
    let req = DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::Start {
        name: "basic".to_string(),
        pipeline_config: basic_binary.spec.ident.clone(),
        playbook_socket: None,
        input_dir: Some(working_dir.join("input")),
        output_dir: Some(working_dir.join("output")),
    });
    client.request(req).await.unwrap();

    // 4. Construct CSV content and send BulkCall request
    let csv_content = "input\nhello\nworld\n".as_bytes().to_vec();
    let req = DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::BulkCall {
        name: "basic".to_string(),
        file_name: "test.csv".to_string(),
        file_content: csv_content,
    });
    
    let resp = client.request(req).await.unwrap();
    
    // 5. Verify the response
    match resp {
        DaemonResponse::Playbook(pyro_daemon::playbook::PlaybookResponse::BulkCallResult { results }) => {
            assert_eq!(results.len(), 2);
            
            let (idx1, row1) = results[0].clone().into_result().unwrap();
            let (idx2, row2) = results[1].clone().into_result().unwrap();
            
            assert_eq!(idx1, 0);
            assert_eq!(idx2, 1);
            
            let output1 = row1.get_str("output").unwrap();
            let output2 = row2.get_str("output").unwrap();
            
            assert_eq!(output1, "Prefixed: hello");
            assert_eq!(output2, "Prefixed: world");
        }
        other => panic!("Unexpected response variant: {:?}", other),
    }

    // Clean up
    daemon_handle.abort();
}
