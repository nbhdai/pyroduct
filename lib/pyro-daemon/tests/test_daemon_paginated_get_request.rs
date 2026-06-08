use std::time::Duration;

use pyro_daemon::client::DaemonClient;
use pyro_daemon::{DaemonRequest, PyroDaemon};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::test]
async fn test_daemon_paginated_get_request() {
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

    // Load and configure a playbook
    let cache = std::sync::Arc::new(
        pyro_artifacts::cache::CacheManager::from_env()
            .await
            .unwrap(),
    );

    let binary = cache
        .get_named_binary("nbhdai", "integration_error", "0.1.0")
        .await
        .unwrap();

    // Start playbook
    let req = DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::Start {
        name: "integration_error".to_string(),
        pipeline_config: binary.spec.ident.clone(),
        playbook_socket: None,
        input_dir: Some(working_dir.join("input")),
        output_dir: Some(working_dir.join("output")),
    });
    client.request(req).await.unwrap();

    // Call playbook multiple times to produce data
    for i in 0..5 {
        let payload = serde_json::json!({ "input": format!("call-{}", i) });
        client
            .call_playbook("integration_error".to_string(), payload)
            .await
            .unwrap();
    }

    // Give some time for the records to settle/flush if needed
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Call paginated get request on running playbook: offset=1, limit=3
    let ipc_bytes = client
        .get_playbook_data("integration_error".to_string(), 1, 3)
        .await
        .unwrap();

    // Parse the IPC bytes back to RecordBatch
    let mut reader =
        arrow::ipc::reader::FileReader::try_new(std::io::Cursor::new(ipc_bytes), None).unwrap();
    let batch = reader.next().unwrap().unwrap();
    assert_eq!(batch.num_rows(), 3);

    // Verify the message strings
    let message_col = batch
        .column_by_name("message")
        .unwrap()
        .as_any()
        .downcast_ref::<arrow::array::StringArray>()
        .unwrap();

    assert_eq!(message_col.value(0), "Success: call-1");
    assert_eq!(message_col.value(1), "Success: call-2");
    assert_eq!(message_col.value(2), "Success: call-3");

    // Clean up
    daemon_handle.abort();
}
