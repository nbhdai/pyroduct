use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use uuid::Uuid;

use pyro_daemon::{DaemonRequest, DaemonResponse, PyroDaemon};

#[tokio::test]
async fn test_daemon_control_protocol() {
    let test_dir = tempfile::tempdir().unwrap();
    let control_socket = test_dir.path().join("pyro-daemon-test.sock");

    // 1. Spawn PyroDaemon in the background
    let daemon = PyroDaemon::new(control_socket.clone());

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

    // 3. Connect to the daemon control socket
    let stream = UnixStream::connect(&control_socket).await.unwrap();
    let (reader, mut writer) = stream.into_split();
    let reader = BufReader::new(reader);
    let mut lines = reader.lines();

    // 4. Send "Status" request
    let req = DaemonRequest::Status;
    let req_str = serde_json::to_string(&req).unwrap() + "\n";
    writer.write_all(req_str.as_bytes()).await.unwrap();

    // 5. Read "StatusInfo" response
    let line = lines.next_line().await.unwrap().unwrap();
    let resp: DaemonResponse = serde_json::from_str(&line).unwrap();
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
    let req = DaemonRequest::Playbook(pyro_daemon::playbooks::PlaybookRequest::List);
    let req_str = serde_json::to_string(&req).unwrap() + "\n";
    writer.write_all(req_str.as_bytes()).await.unwrap();

    // 7. Read "Playbooks" response
    let line = lines.next_line().await.unwrap().unwrap();
    let resp: DaemonResponse = serde_json::from_str(&line).unwrap();
    match resp {
        DaemonResponse::Playbook(pyro_daemon::playbooks::PlaybookResponse::Playbooks {
            playbooks,
        }) => {
            assert!(playbooks.is_empty());
        }
        other => panic!("Unexpected response for List request: {:?}", other),
    }

    // 8. Try to stop a non-existent playbook
    let req = DaemonRequest::Playbook(pyro_daemon::playbooks::PlaybookRequest::Stop {
        playbook_id: Uuid::new_v4(),
    });
    let req_str = serde_json::to_string(&req).unwrap() + "\n";
    writer.write_all(req_str.as_bytes()).await.unwrap();

    // 9. Read "Error" response
    let line = lines.next_line().await.unwrap().unwrap();
    let resp: DaemonResponse = serde_json::from_str(&line).unwrap();
    match resp {
        DaemonResponse::Playbook(pyro_daemon::playbooks::PlaybookResponse::Error { message }) => {
            assert!(message.contains("No active playbook worker found"));
        }
        other => panic!("Unexpected response for invalid Stop request: {:?}", other),
    }

    // 10. Shutdown the daemon
    daemon_handle.abort();
}
