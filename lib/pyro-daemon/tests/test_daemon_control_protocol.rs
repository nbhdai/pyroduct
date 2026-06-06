use std::time::Duration;

use pyro_daemon::client::DaemonClient;
use pyro_daemon::{DaemonRequest, DaemonResponse, PyroDaemon};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::test]
async fn test_daemon_control_protocol() {
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
