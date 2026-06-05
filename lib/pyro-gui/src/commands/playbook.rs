use serde_json::Value;

#[derive(serde::Deserialize)]
pub struct RemoteCapabilityConfig {
    capability: pyro_artifacts::cargo::CapabilityIdent,
    address: pyro_artifacts::cache::RemoteAddress,
}

#[tauri::command]
pub async fn list_active_playbooks() -> Result<Value, String> {
    let working_dir = pyro_daemon::PyroDaemon::default_working_dir();
    let control_socket_path = working_dir.join("control");

    if !control_socket_path.exists() {
        return Err("Daemon control socket does not exist (offline)".to_string());
    }

    let client = pyro_daemon::client::DaemonClient::connect(&control_socket_path)
        .await
        .map_err(|e| format!("Failed to connect to daemon: {:?}", e))?;

    let req = pyro_daemon::DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::List);
    match client.request(req).await {
        Ok(pyro_daemon::DaemonResponse::Playbook(
            pyro_daemon::playbook::PlaybookResponse::Playbooks { playbooks },
        )) => Ok(serde_json::to_value(playbooks).map_err(|e| e.to_string())?),
        Ok(resp) => Err(format!("Unexpected response type: {:?}", resp)),
        Err(e) => Err(format!("Daemon request failed: {:?}", e)),
    }
}

#[tauri::command]
pub async fn start_playbook(
    name: String,
    config_path: Option<String>,
    playbook_ident: Option<pyro_artifacts::artifacts::PlaybookIdent>,
    remote: Option<Vec<RemoteCapabilityConfig>>,
    wal_capacity: Option<usize>,
    success_log_retention_secs: Option<u64>,
    error_log_retention_secs: Option<u64>,
    playbook_socket: Option<String>,
    input_dir: Option<String>,
    output_dir: Option<String>,
) -> Result<String, String> {
    let working_dir = pyro_daemon::PyroDaemon::default_working_dir();
    let control_socket_path = working_dir.join("control");

    let client = pyro_daemon::client::DaemonClient::connect(&control_socket_path)
        .await
        .map_err(|e| format!("Failed to connect to daemon: {:?}", e))?;

    let path_to_use = if let Some(path) = config_path.filter(|p| !p.trim().is_empty()) {
        std::path::PathBuf::from(path)
    } else {
        let ident = playbook_ident
            .ok_or_else(|| "Must provide either config_path or playbook_ident".to_string())?;

        let playbook_dir = working_dir.join("playbooks").join(&name);
        tokio::fs::create_dir_all(&playbook_dir)
            .await
            .map_err(|e| format!("Failed to create playbook directory: {:?}", e))?;

        let config_file_path = playbook_dir.join("config.toml");

        let mut remote_map = std::collections::HashMap::new();
        if let Some(remotes) = remote {
            for entry in remotes {
                remote_map.insert(entry.capability, entry.address);
            }
        }

        let pipeline_config = pyroduct::pipeline::factory::PipelineConfig {
            playbook: ident,
            remote: remote_map,
            wal_capacity: wal_capacity.unwrap_or(1000),
            success_log_retention_secs: success_log_retention_secs.unwrap_or(3600),
            error_log_retention_secs: error_log_retention_secs.unwrap_or(86400 * 7),
            log_dir: std::path::PathBuf::from(""),
            input_dir: std::path::PathBuf::from(""),
            output_dir: std::path::PathBuf::from(""),
        };

        let toml_string = toml::to_string_pretty(&pipeline_config)
            .map_err(|e| format!("Failed to serialize PipelineConfig: {:?}", e))?;

        tokio::fs::write(&config_file_path, toml_string)
            .await
            .map_err(|e| format!("Failed to write PipelineConfig: {:?}", e))?;

        config_file_path
    };

    let req = pyro_daemon::DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::Start {
        name,
        playbook_config_path: path_to_use,
        playbook_socket,
        input_dir: input_dir.map(std::path::PathBuf::from),
        output_dir: output_dir.map(std::path::PathBuf::from),
    });

    match client.request(req).await {
        Ok(pyro_daemon::DaemonResponse::Playbook(
            pyro_daemon::playbook::PlaybookResponse::Success { message },
        )) => Ok(message),
        Ok(pyro_daemon::DaemonResponse::Playbook(
            pyro_daemon::playbook::PlaybookResponse::Error { message },
        )) => Err(message),
        Ok(resp) => Err(format!("Unexpected response from daemon: {:?}", resp)),
        Err(e) => Err(format!("Failed to start playbook: {:?}", e)),
    }
}

#[tauri::command]
pub async fn stop_playbook(name: String) -> Result<String, String> {
    let working_dir = pyro_daemon::PyroDaemon::default_working_dir();
    let control_socket_path = working_dir.join("control");

    let client = pyro_daemon::client::DaemonClient::connect(&control_socket_path)
        .await
        .map_err(|e| format!("Failed to connect to daemon: {:?}", e))?;

    let req =
        pyro_daemon::DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::Stop { name });

    match client.request(req).await {
        Ok(pyro_daemon::DaemonResponse::Playbook(
            pyro_daemon::playbook::PlaybookResponse::Success { message },
        )) => Ok(message),
        Ok(pyro_daemon::DaemonResponse::Playbook(
            pyro_daemon::playbook::PlaybookResponse::Error { message },
        )) => Err(message),
        Ok(resp) => Err(format!("Unexpected response from daemon: {:?}", resp)),
        Err(e) => Err(format!("Failed to stop playbook: {:?}", e)),
    }
}

#[tauri::command]
pub async fn delete_playbook(name: String) -> Result<String, String> {
    let working_dir = pyro_daemon::PyroDaemon::default_working_dir();
    let control_socket_path = working_dir.join("control");

    let client = pyro_daemon::client::DaemonClient::connect(&control_socket_path)
        .await
        .map_err(|e| format!("Failed to connect to daemon: {:?}", e))?;

    let req =
        pyro_daemon::DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::Delete {
            name,
        });

    match client.request(req).await {
        Ok(pyro_daemon::DaemonResponse::Playbook(
            pyro_daemon::playbook::PlaybookResponse::Success { message },
        )) => Ok(message),
        Ok(pyro_daemon::DaemonResponse::Playbook(
            pyro_daemon::playbook::PlaybookResponse::Error { message },
        )) => Err(message),
        Ok(resp) => Err(format!("Unexpected response from daemon: {:?}", resp)),
        Err(e) => Err(format!("Failed to delete playbook: {:?}", e)),
    }
}

#[tauri::command]
pub async fn call_playbook(name: String, payload: Value) -> Result<Value, String> {
    let working_dir = pyro_daemon::PyroDaemon::default_working_dir();
    let control_socket_path = working_dir.join("control");

    let client = pyro_daemon::client::DaemonClient::connect(&control_socket_path)
        .await
        .map_err(|e| format!("Failed to connect to daemon: {:?}", e))?;

    let req = pyro_daemon::DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::Call {
        name,
        payload,
    });

    match client.request(req).await {
        Ok(pyro_daemon::DaemonResponse::Playbook(
            pyro_daemon::playbook::PlaybookResponse::CallResult { result },
        )) => Ok(result),
        Ok(pyro_daemon::DaemonResponse::Playbook(
            pyro_daemon::playbook::PlaybookResponse::Error { message },
        )) => Err(message),
        Ok(resp) => Err(format!("Unexpected response from daemon: {:?}", resp)),
        Err(e) => Err(format!("Failed to call playbook: {:?}", e)),
    }
}
