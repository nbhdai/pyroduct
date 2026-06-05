use serde_json::Value;

#[tauri::command]
pub async fn get_daemon_status() -> Result<Value, String> {
    let working_dir = pyro_daemon::PyroDaemon::default_working_dir();
    let control_socket_path = working_dir.join("control");
    let socket_exists = control_socket_path.exists();

    if !socket_exists {
        return Ok(serde_json::json!({
            "status": "offline",
            "socket_path": control_socket_path.to_string_lossy(),
            "message": "Control socket file does not exist."
        }));
    }

    match pyro_daemon::client::DaemonClient::connect(&control_socket_path).await {
        Ok(client) => match client.request(pyro_daemon::DaemonRequest::Status).await {
            Ok(pyro_daemon::DaemonResponse::StatusInfo {
                active_workers,
                version,
                running_playbooks,
            }) => Ok(serde_json::json!({
                "status": "online",
                "socket_path": control_socket_path.to_string_lossy(),
                "active_workers": active_workers,
                "version": version,
                "running_playbooks": running_playbooks,
            })),
            Ok(resp) => Ok(serde_json::json!({
                "status": "online",
                "socket_path": control_socket_path.to_string_lossy(),
                "message": format!("Unexpected response: {:?}", resp)
            })),
            Err(e) => Ok(serde_json::json!({
                "status": "error",
                "socket_path": control_socket_path.to_string_lossy(),
                "message": format!("Connected but status request failed: {:?}", e)
            })),
        },
        Err(e) => Ok(serde_json::json!({
            "status": "offline",
            "socket_path": control_socket_path.to_string_lossy(),
            "message": format!("Failed to connect: {:?}", e)
        })),
    }
}
