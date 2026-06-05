use std::time::Duration;

use pyro_daemon::client::DaemonClient;
use pyro_daemon::{DaemonRequest, DaemonResponse, PyroDaemon};

#[tracing_test::traced_test]
#[tokio::test]
async fn test_daemon_control_protocol() {
    let test_dir = tempfile::tempdir().unwrap();
    let working_dir = test_dir.path().to_path_buf();
    let control_socket = working_dir.join("control");

    // 1. Spawn PyroDaemon in the background
    let daemon = PyroDaemon::new(working_dir.clone());

    let daemon_handle = tokio::spawn(async move {
        daemon.run().await.unwrap();
    });

    // 2. Wait for the socket file to be created
    let mut retries = 0;
    while !control_socket.exists() {
        if retries > 50 {
            panic!("Daemon failed to bind control socket in time");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
        retries += 1;
    }

    // 3. Connect to the daemon control socket using programmatic DaemonClient
    let client = DaemonClient::connect(&control_socket).await.unwrap();

    // 4. Send "Status" request
    let req = DaemonRequest::Status;
    let resp = client.request(req).await.unwrap();
    match resp {
        DaemonResponse::StatusInfo {
            active_workers,
            version,
            running_playbooks,
        } => {
            assert_eq!(active_workers, 0);
            assert_eq!(version, env!("CARGO_PKG_VERSION"));
            assert!(running_playbooks.is_empty());
        }
        other => panic!("Unexpected response for Status request: {:?}", other),
    }

    // 6. Send "ListPlaybooks" request
    let req = DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::List);
    let resp = client.request(req).await.unwrap();
    match resp {
        DaemonResponse::Playbook(pyro_daemon::playbook::PlaybookResponse::Playbooks {
            playbooks,
        }) => {
            assert!(playbooks.is_empty());
        }
        other => panic!("Unexpected response for List request: {:?}", other),
    }

    // 8. Try to stop a non-existent playbook
    let req = DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::Stop {
        name: "not_here".to_string(),
    });
    let resp = client.request(req).await.unwrap();
    match resp {
        DaemonResponse::Playbook(pyro_daemon::playbook::PlaybookResponse::Error { message }) => {
            assert!(message.contains("No active playbook worker found"));
        }
        other => panic!("Unexpected response for invalid Stop request: {:?}", other),
    }

    // 9a. Try to resume a non-existent playbook
    let req = DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::Resume {
        name: "not_here".to_string(),
    });
    let resp = client.request(req).await.unwrap();
    match resp {
        DaemonResponse::Playbook(pyro_daemon::playbook::PlaybookResponse::Error { message }) => {
            assert!(message.contains("does not exist"));
        }
        other => panic!(
            "Unexpected response for invalid Resume request: {:?}",
            other
        ),
    }

    // 9b. Try to delete a non-existent playbook
    let req = DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::Delete {
        name: "not_here".to_string(),
    });
    let resp = client.request(req).await.unwrap();
    match resp {
        DaemonResponse::Playbook(pyro_daemon::playbook::PlaybookResponse::Success { message }) => {
            assert!(message.contains("deleted successfully"));
        }
        other => panic!("Unexpected response for Delete request: {:?}", other),
    }

    // 9c. Load and start two playbooks, then check Status command
    // First, configure CacheManager to inherit from the environment

    let cache = std::sync::Arc::new(
        pyro_artifacts::cache::CacheManager::from_env()
            .await
            .unwrap(),
    );

    let binary_a = cache
        .get_named_binary("nbhdai", "counter", "0.1.0")
        .await
        .unwrap();
    let binary_b = cache
        .get_named_binary("nbhdai", "integration_error", "0.1.0")
        .await
        .unwrap();

    let config_a_path = working_dir.join("config_a.toml");
    let pipeline_config_a = pyroduct::pipeline::factory::PipelineConfig {
        playbook: binary_a.spec.ident.clone(),
        remote: std::collections::HashMap::new(),
        wal_capacity: 10,
        success_log_retention_secs: 3600,
        error_log_retention_secs: 86400 * 7,
        input_dir: working_dir.join("input_a"),
        output_dir: working_dir.join("output_a"),
        log_dir: working_dir.join("log_a"),
    };
    tokio::fs::write(
        &config_a_path,
        toml::to_string_pretty(&pipeline_config_a).unwrap(),
    )
    .await
    .unwrap();

    let config_b_path = working_dir.join("config_b.toml");
    let pipeline_config_b = pyroduct::pipeline::factory::PipelineConfig {
        playbook: binary_b.spec.ident.clone(),
        remote: std::collections::HashMap::new(),
        wal_capacity: 10,
        success_log_retention_secs: 3600,
        error_log_retention_secs: 86400 * 7,
        input_dir: working_dir.join("input_b"),
        output_dir: working_dir.join("output_b"),
        log_dir: working_dir.join("log_b"),
    };
    tokio::fs::write(
        &config_b_path,
        toml::to_string_pretty(&pipeline_config_b).unwrap(),
    )
    .await
    .unwrap();

    // Start playbook A
    let req = DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::Start {
        name: "counter".to_string(),
        playbook_config_path: config_a_path,
        playbook_socket: None,
        input_dir: None,
        output_dir: None,
    });
    let resp = client.request(req).await.unwrap();
    match resp {
        DaemonResponse::Playbook(pyro_daemon::playbook::PlaybookResponse::Success { .. }) => {}
        other => panic!("Unexpected response for starting playbook A: {:?}", other),
    }

    // Start playbook B
    let req = DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::Start {
        name: "integration_error".to_string(),
        playbook_config_path: config_b_path,
        playbook_socket: None,
        input_dir: None,
        output_dir: None,
    });
    let resp = client.request(req).await.unwrap();
    match resp {
        DaemonResponse::Playbook(pyro_daemon::playbook::PlaybookResponse::Success { .. }) => {}
        other => panic!("Unexpected response for starting playbook B: {:?}", other),
    }

    // Check Status info shows both running playbooks
    let req = DaemonRequest::Status;
    let resp = client.request(req).await.unwrap();
    match resp {
        DaemonResponse::StatusInfo {
            active_workers,
            version,
            running_playbooks,
        } => {
            assert_eq!(active_workers, 2);
            assert_eq!(version, env!("CARGO_PKG_VERSION"));
            assert_eq!(running_playbooks.len(), 2);
            assert!(running_playbooks.iter().any(|pb| pb.name == "counter"));
            assert!(
                running_playbooks
                    .iter()
                    .any(|pb| pb.name == "integration_error")
            );
        }
        other => panic!(
            "Unexpected response for Status request after starting playbooks: {:?}",
            other
        ),
    }

    // Clean up playbooks by stopping them
    let req = DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::Stop {
        name: "counter".to_string(),
    });
    let _ = client.request(req).await.unwrap();

    let req = DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::Stop {
        name: "integration_error".to_string(),
    });
    let _ = client.request(req).await.unwrap();

    // 10. Shutdown the daemon
    daemon_handle.abort();
}

#[tracing_test::traced_test]
#[tokio::test]
async fn test_daemon_callback_persistence_and_restore() {
    let test_dir = tempfile::tempdir().unwrap();
    let working_dir = test_dir.path().to_path_buf();

    // 1. Spawn PyroDaemon
    let daemon = PyroDaemon::new(working_dir.clone());
    let db = daemon.playbooks_manager.db.clone();

    // 2. Add some mock callback mappings directly to the DB to verify persist/restore
    let uuid1 = uuid::Uuid::new_v4();
    let uuid2 = uuid::Uuid::new_v4();
    db.add_callback_mapping(
        uuid1,
        "test_playbook",
        "http",
        "http://127.0.0.1:9999/callback",
    )
    .await
    .unwrap();
    db.add_callback_mapping(uuid2, "test_playbook", "socket", "127.0.0.1:8888")
        .await
        .unwrap();

    // 3. Query the callback mappings to verify list_callbacks query works
    let callbacks = daemon
        .playbooks_manager
        .list_callbacks("test_playbook".to_string())
        .await
        .unwrap();
    assert_eq!(callbacks.len(), 2);
    assert_eq!(callbacks[0].uuid, uuid1);
    assert_eq!(callbacks[0].callback_type, "http");
    assert_eq!(callbacks[0].target, "http://127.0.0.1:9999/callback");
    assert_eq!(callbacks[1].uuid, uuid2);
    assert_eq!(callbacks[1].callback_type, "socket");
    assert_eq!(callbacks[1].target, "127.0.0.1:8888");

    // 4. Test deleting one callback mapping by UUID
    db.delete_callback_mapping(uuid1).await.unwrap();
    let callbacks_after_one_deleted = daemon
        .playbooks_manager
        .list_callbacks("test_playbook".to_string())
        .await
        .unwrap();
    assert_eq!(callbacks_after_one_deleted.len(), 1);
    assert_eq!(callbacks_after_one_deleted[0].uuid, uuid2);

    // 5. Test deleting playbook cleans up callback mappings
    db.delete_playbook("test_playbook").await.unwrap();
    let callbacks_after_delete = daemon
        .playbooks_manager
        .list_callbacks("test_playbook".to_string())
        .await
        .unwrap();
    assert_eq!(callbacks_after_delete.len(), 0);
}

#[tracing_test::traced_test]
#[tokio::test]
async fn test_daemon_callback_rpc_commands() {
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

    // 1. Send AddHttpCallback request for a mock playbook "rpc_test"
    let req = DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::AddHttpCallback {
        source: "rpc_test".to_string(),
        url: "http://example.com/cb".to_string(),
    });
    let resp = client.request(req).await.unwrap();
    match resp {
        DaemonResponse::Playbook(pyro_daemon::playbook::PlaybookResponse::Success { message }) => {
            assert!(message.contains("HTTP callback added successfully"));
        }
        other => panic!("Unexpected response: {:?}", other),
    }

    // 2. Send AddSocketCallback request
    let req = DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::AddSocketCallback {
        source: "rpc_test".to_string(),
        socket_path: "127.0.0.1:12345".to_string(),
    });
    let resp = client.request(req).await.unwrap();
    match resp {
        DaemonResponse::Playbook(pyro_daemon::playbook::PlaybookResponse::Success { message }) => {
            assert!(message.contains("Socket callback added successfully"));
        }
        other => panic!("Unexpected response: {:?}", other),
    }

    // 3. Send AddPlaybookCallback request (from rpc_test to target_playbook)
    let req = DaemonRequest::Playbook(
        pyro_daemon::playbook::PlaybookRequest::AddPlaybookCallback {
            source: "rpc_test".to_string(),
            target_playbook: "target_playbook".to_string(),
        },
    );
    let resp = client.request(req).await.unwrap();
    match resp {
        DaemonResponse::Playbook(pyro_daemon::playbook::PlaybookResponse::Success { message }) => {
            assert!(message.contains("Playbook callback added successfully"));
        }
        other => panic!("Unexpected response: {:?}", other),
    }

    // 4. Send ListCallbacks request
    let req = DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::ListCallbacks {
        source: "rpc_test".to_string(),
    });
    let resp = client.request(req).await.unwrap();
    let uuid_to_delete;
    match resp {
        DaemonResponse::Playbook(pyro_daemon::playbook::PlaybookResponse::Callbacks {
            callbacks,
        }) => {
            assert_eq!(callbacks.len(), 3);
            assert_eq!(callbacks[0].callback_type, "http");
            assert_eq!(callbacks[0].target, "http://example.com/cb");
            assert_eq!(callbacks[1].callback_type, "socket");
            assert_eq!(callbacks[1].target, "127.0.0.1:12345");
            assert_eq!(callbacks[2].callback_type, "playbook");
            assert_eq!(callbacks[2].target, "target_playbook");
            uuid_to_delete = callbacks[0].uuid;
        }
        other => panic!("Unexpected response: {:?}", other),
    }

    // 5. Delete callback by UUID via RPC
    let uuid = uuid_to_delete;
    let req =
        DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::DeleteCallback { uuid });
    let resp = client.request(req).await.unwrap();
    match resp {
        DaemonResponse::Playbook(pyro_daemon::playbook::PlaybookResponse::Success { message }) => {
            assert!(message.contains("Callback deleted successfully"));
        }
        other => panic!("Unexpected response for DeleteCallback: {:?}", other),
    }

    // 6. List callbacks again to verify deletion
    let req = DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::ListCallbacks {
        source: "rpc_test".to_string(),
    });
    let resp = client.request(req).await.unwrap();
    match resp {
        DaemonResponse::Playbook(pyro_daemon::playbook::PlaybookResponse::Callbacks {
            callbacks,
        }) => {
            assert_eq!(callbacks.len(), 2);
            assert_eq!(callbacks[0].callback_type, "socket");
            assert_eq!(callbacks[1].callback_type, "playbook");
        }
        other => panic!("Unexpected response: {:?}", other),
    }

    daemon_handle.abort();
}

#[tracing_test::traced_test]
#[tokio::test]
async fn test_daemon_data_streaming() {
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

    let config_a_path = working_dir.join("config_a.toml");
    let pipeline_config_a = pyroduct::pipeline::factory::PipelineConfig {
        playbook: binary_a.spec.ident.clone(),
        remote: std::collections::HashMap::new(),
        wal_capacity: 10,
        success_log_retention_secs: 3600,
        error_log_retention_secs: 86400 * 7,
        input_dir: working_dir.join("input_a"),
        output_dir: working_dir.join("output_a"),
        log_dir: working_dir.join("log_a"),
    };
    tokio::fs::write(
        &config_a_path,
        toml::to_string_pretty(&pipeline_config_a).unwrap(),
    )
    .await
    .unwrap();

    // Start playbook
    let req = DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::Start {
        name: "integration_error".to_string(),
        playbook_config_path: config_a_path,
        playbook_socket: None,
        input_dir: None,
        output_dir: None,
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

#[tracing_test::traced_test]
#[tokio::test]
async fn test_daemon_paginated_get_request() {
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

    let config_path = working_dir.join("config.toml");
    let pipeline_config = pyroduct::pipeline::factory::PipelineConfig {
        playbook: binary.spec.ident.clone(),
        remote: std::collections::HashMap::new(),
        wal_capacity: 10,
        success_log_retention_secs: 3600,
        error_log_retention_secs: 86400 * 7,
        input_dir: working_dir.join("input"),
        output_dir: working_dir.join("output"),
        log_dir: working_dir.join("log"),
    };
    tokio::fs::write(
        &config_path,
        toml::to_string_pretty(&pipeline_config).unwrap(),
    )
    .await
    .unwrap();

    // Start playbook
    let req = DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::Start {
        name: "integration_error".to_string(),
        playbook_config_path: config_path,
        playbook_socket: None,
        input_dir: None,
        output_dir: None,
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
