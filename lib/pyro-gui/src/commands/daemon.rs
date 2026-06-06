use serde_json::Value;
use tracing::{trace, debug, info, error};

#[tauri::command]
pub async fn get_daemon_status() -> Result<Value, String> {
    info!("Tauri command: get_daemon_status");
    let working_dir = pyro_daemon::PyroDaemon::default_working_dir();
    let control_socket_path = working_dir.join("control");
    let socket_exists = control_socket_path.exists();
    trace!("Daemon working directory: {:?}, socket path: {:?}, socket exists: {}", working_dir, control_socket_path, socket_exists);

    if !socket_exists {
        info!("Daemon socket file does not exist, reporting offline");
        return Ok(serde_json::json!({
            "status": "offline",
            "socket_path": control_socket_path.to_string_lossy(),
            "message": "Control socket file does not exist."
        }));
    }

    debug!("Attempting to connect to daemon control socket at: {:?}", control_socket_path);
    match pyro_daemon::client::DaemonClient::connect(&control_socket_path).await {
        Ok(client) => {
            debug!("Connected to daemon. Requesting status details.");
            match client.request(pyro_daemon::DaemonRequest::Status).await {
                Ok(pyro_daemon::DaemonResponse::StatusInfo {
                    active_workers,
                    version,
                    running_playbooks,
                }) => {
                    info!("Daemon is online. Version: {}, Active Workers: {}, Running Playbooks: {:?}", version, active_workers, running_playbooks);
                    Ok(serde_json::json!({
                        "status": "online",
                        "socket_path": control_socket_path.to_string_lossy(),
                        "active_workers": active_workers,
                        "version": version,
                        "running_playbooks": running_playbooks,
                    }))
                }
                Ok(resp) => {
                    error!("Received unexpected status response from daemon: {:?}", resp);
                    Ok(serde_json::json!({
                        "status": "online",
                        "socket_path": control_socket_path.to_string_lossy(),
                        "message": format!("Unexpected response: {:?}", resp)
                    }))
                }
                Err(e) => {
                    error!("Connected to daemon, but status request failed: {:?}", e);
                    Ok(serde_json::json!({
                        "status": "error",
                        "socket_path": control_socket_path.to_string_lossy(),
                        "message": format!("Connected but status request failed: {:?}", e)
                    }))
                }
            }
        }
        Err(e) => {
            error!("Failed to connect to daemon socket: {:?}", e);
            Ok(serde_json::json!({
                "status": "offline",
                "socket_path": control_socket_path.to_string_lossy(),
                "message": format!("Failed to connect: {:?}", e)
            }))
        }
    }
}
