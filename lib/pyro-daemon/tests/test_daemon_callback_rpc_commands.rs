use std::time::Duration;

use pyro_daemon::client::DaemonClient;
use pyro_daemon::{DaemonRequest, DaemonResponse, PyroDaemon};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::test]
async fn test_daemon_callback_rpc_commands() {
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
