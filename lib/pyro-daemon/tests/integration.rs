use std::time::Duration;

use pyro_daemon::client::DaemonClient;
use pyro_daemon::{DaemonRequest, DaemonResponse, PyroDaemon};

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
        } => {
            assert_eq!(active_workers, 0);
            assert_eq!(version, env!("CARGO_PKG_VERSION"));
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

    // 10. Shutdown the daemon
    daemon_handle.abort();
}

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
