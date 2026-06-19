use std::time::Duration;

use pyro_daemon::client::DaemonClient;
use pyro_daemon::{DaemonRequest, DaemonResponse, PyroDaemon};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::test]
async fn test_daemon_data_streaming() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "trace,cranelift_frontend=off,cranelift_codegen=off,wasmtime=off".into()
    });

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).pretty())
        .with(filter)
        .init();

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

    // Load and configure a playbook using the environment's cache

    let cache = std::sync::Arc::new(
        pyro_artifacts::cache::CacheManager::from_env()
            .await
            .unwrap(),
    );

    let binary_a = cache
        .get_named_binary("nbhdai", "integration_error", "0.1.0")
        .await
        .unwrap();

    // Start playbook
    let req = DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::Start {
        name: "integration_error".to_string(),
        pipeline_config: binary_a.spec.ident.clone(),
        playbook_socket: None,
        http_address: None,
        input_dir: Some(working_dir.join("input_a")),
        output_dir: Some(working_dir.join("output_a")),
        pinned_version: None,
        configurations: None,
    });
    let resp = client.request(req).await.unwrap();
    match resp {
        DaemonResponse::Playbook(pyro_daemon::playbook::PlaybookResponse::Success { .. }) => {}
        other => panic!("Unexpected response for starting playbook: {:?}", other),
    }

    // Now start streaming playbook
    let mut rx = client
        .stream_playbook("integration_error".to_string())
        .await
        .unwrap();

    // Let's invoke/call the playbook in the background
    let client_clone = client.clone();
    let call_handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let payload = serde_json::json!({ "input": "streaming" });
        client_clone
            .call_playbook("integration_error".to_string(), payload)
            .await
            .unwrap();
    });

    // Receive the streamed row
    let row = rx.recv().await.unwrap().unwrap();
    println!("{:?}", row);
    // Verify the contents of the row
    let msg_val = row.get("message").unwrap();
    match msg_val {
        pyroduct::PyroValue::Str(s) => {
            assert_eq!(s.as_ref(), "Success: streaming");
        }
        other => panic!("Unexpected value type for message: {:?}", other),
    }

    call_handle.await.unwrap();
    daemon_handle.abort();
}
