use serde_json::Value;
use tracing::{debug, error, info, trace};


#[tauri::command]
pub async fn list_active_playbooks() -> Result<Value, String> {
    info!("Tauri command: list_active_playbooks");
    let working_dir = pyro_daemon::PyroDaemon::default_working_dir();
    let control_socket_path = working_dir.join("control");

    if !control_socket_path.exists() {
        error!(
            "Daemon control socket does not exist at {:?}",
            control_socket_path
        );
        return Err("Daemon control socket does not exist (offline)".to_string());
    }

    debug!("Connecting to daemon at {:?}", control_socket_path);
    let client = pyro_daemon::client::DaemonClient::connect(&control_socket_path)
        .await
        .map_err(|e| {
            error!("Failed to connect to daemon: {:?}", e);
            format!("Failed to connect to daemon: {:?}", e)
        })?;

    let req = pyro_daemon::DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::List);
    debug!("Sending playbook list request to daemon");
    match client.request(req).await {
        Ok(pyro_daemon::DaemonResponse::Playbook(
            pyro_daemon::playbook::PlaybookResponse::Playbooks { playbooks },
        )) => {
            info!(
                "Successfully retrieved {} active playbooks",
                playbooks.len()
            );
            trace!("Active playbooks list: {:?}", playbooks);
            Ok(serde_json::to_value(playbooks).map_err(|e| {
                error!("Failed to serialize playbooks list to JSON: {:?}", e);
                e.to_string()
            })?)
        }
        Ok(resp) => {
            error!("Unexpected response type from daemon: {:?}", resp);
            Err(format!("Unexpected response type: {:?}", resp))
        }
        Err(e) => {
            error!("Daemon request failed: {:?}", e);
            Err(format!("Daemon request failed: {:?}", e))
        }
    }
}

#[tauri::command]
pub async fn start_playbook(
    name: String,
    playbook_ident: pyro_artifacts::artifacts::PlaybookIdent,
    playbook_socket: Option<String>,
    input_dir: Option<String>,
    output_dir: Option<String>,
) -> Result<String, String> {
    info!("Tauri command: start_playbook for '{}'", name);
    let working_dir = pyro_daemon::PyroDaemon::default_working_dir();
    let control_socket_path = working_dir.join("control");

    let client = pyro_daemon::client::DaemonClient::connect(&control_socket_path)
        .await
        .map_err(|e| {
            error!("Failed to connect to daemon: {:?}", e);
            format!("Failed to connect to daemon: {:?}", e)
        })?;

    let req = pyro_daemon::DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::Start {
        name: name.clone(),
        pipeline_config: playbook_ident,
        playbook_socket: playbook_socket.clone(),
        input_dir: input_dir.clone().map(std::path::PathBuf::from),
        output_dir: output_dir.clone().map(std::path::PathBuf::from),
    });
    debug!(
        "Sending PlaybookRequest::Start to daemon: name='{}', socket={:?}, input={:?}, output={:?}",
        name, playbook_socket, input_dir, output_dir
    );

    match client.request(req).await {
        Ok(pyro_daemon::DaemonResponse::Playbook(
            pyro_daemon::playbook::PlaybookResponse::Success { message },
        )) => {
            info!("Playbook '{}' started successfully: {}", name, message);
            Ok(message)
        }
        Ok(pyro_daemon::DaemonResponse::Playbook(
            pyro_daemon::playbook::PlaybookResponse::Error { message },
        )) => {
            error!(
                "Daemon returned playbook error response for '{}': {}",
                name, message
            );
            Err(message)
        }
        Ok(resp) => {
            error!("Unexpected response from daemon on start: {:?}", resp);
            Err(format!("Unexpected response from daemon: {:?}", resp))
        }
        Err(e) => {
            error!("Failed to start playbook '{}': {:?}", name, e);
            Err(format!("Failed to start playbook: {:?}", e))
        }
    }
}

#[tauri::command]
pub async fn stop_playbook(name: String) -> Result<String, String> {
    info!("Tauri command: stop_playbook for '{}'", name);
    let working_dir = pyro_daemon::PyroDaemon::default_working_dir();
    let control_socket_path = working_dir.join("control");

    let client = pyro_daemon::client::DaemonClient::connect(&control_socket_path)
        .await
        .map_err(|e| {
            error!("Failed to connect to daemon: {:?}", e);
            format!("Failed to connect to daemon: {:?}", e)
        })?;

    let req = pyro_daemon::DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::Stop {
        name: name.clone(),
    });

    debug!("Sending PlaybookRequest::Stop to daemon for '{}'", name);
    match client.request(req).await {
        Ok(pyro_daemon::DaemonResponse::Playbook(
            pyro_daemon::playbook::PlaybookResponse::Success { message },
        )) => {
            info!("Playbook '{}' stopped successfully: {}", name, message);
            Ok(message)
        }
        Ok(pyro_daemon::DaemonResponse::Playbook(
            pyro_daemon::playbook::PlaybookResponse::Error { message },
        )) => {
            error!(
                "Daemon returned error response on stop playbook '{}': {}",
                name, message
            );
            Err(message)
        }
        Ok(resp) => {
            error!(
                "Unexpected response from daemon on stop playbook '{}': {:?}",
                name, resp
            );
            Err(format!("Unexpected response from daemon: {:?}", resp))
        }
        Err(e) => {
            error!("Failed to stop playbook '{}': {:?}", name, e);
            Err(format!("Failed to stop playbook: {:?}", e))
        }
    }
}

#[tauri::command]
pub async fn delete_playbook(name: String) -> Result<String, String> {
    info!("Tauri command: delete_playbook for '{}'", name);
    let working_dir = pyro_daemon::PyroDaemon::default_working_dir();
    let control_socket_path = working_dir.join("control");

    let client = pyro_daemon::client::DaemonClient::connect(&control_socket_path)
        .await
        .map_err(|e| {
            error!("Failed to connect to daemon: {:?}", e);
            format!("Failed to connect to daemon: {:?}", e)
        })?;

    let req =
        pyro_daemon::DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::Delete {
            name: name.clone(),
        });

    debug!("Sending PlaybookRequest::Delete to daemon for '{}'", name);
    match client.request(req).await {
        Ok(pyro_daemon::DaemonResponse::Playbook(
            pyro_daemon::playbook::PlaybookResponse::Success { message },
        )) => {
            info!("Playbook '{}' deleted successfully: {}", name, message);
            Ok(message)
        }
        Ok(pyro_daemon::DaemonResponse::Playbook(
            pyro_daemon::playbook::PlaybookResponse::Error { message },
        )) => {
            error!(
                "Daemon returned error response on delete playbook '{}': {}",
                name, message
            );
            Err(message)
        }
        Ok(resp) => {
            error!(
                "Unexpected response from daemon on delete playbook '{}': {:?}",
                name, resp
            );
            Err(format!("Unexpected response from daemon: {:?}", resp))
        }
        Err(e) => {
            error!("Failed to delete playbook '{}': {:?}", name, e);
            Err(format!("Failed to delete playbook: {:?}", e))
        }
    }
}

#[tauri::command]
pub async fn call_playbook(
    name: String,
    payload: Value,
    session_id: Option<u32>,
) -> Result<pyroduct::pipeline::ServerExecutionRecord, String> {
    info!("Tauri command: call_playbook for '{}'", name);
    trace!("Call playbook payload: {:?}, session_id: {:?}", payload, session_id);
    let working_dir = pyro_daemon::PyroDaemon::default_working_dir();
    let control_socket_path = working_dir.join("control");

    let client = pyro_daemon::client::DaemonClient::connect(&control_socket_path)
        .await
        .map_err(|e| {
            error!("Failed to connect to daemon: {:?}", e);
            format!("Failed to connect to daemon: {:?}", e)
        })?;

    debug!("Calling playbook record via daemon for '{}' (session: {:?})", name, session_id);
    client
        .call_playbook_record(name, payload, session_id)
        .await
        .map_err(|e| {
            error!("Failed to call playbook: {:?}", e);
            e.to_string()
        })
}
