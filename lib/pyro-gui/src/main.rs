#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use serde_json::Value;

async fn get_cache_manager() -> Result<pyro_artifacts::cache::CacheManager, String> {
    match pyro_artifacts::cache::CacheManager::from_env().await {
        Ok(mgr) => Ok(mgr),
        Err(_) => {
            // Fallback to default ~/.pyroduct
            let home = std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let root = home.join(".pyroduct");
            pyro_artifacts::cache::CacheManager::new(&root, None, "anon".to_string())
                .await
                .map_err(|e| e.to_string())
        }
    }
}

#[tauri::command]
async fn get_daemon_status() -> Result<Value, String> {
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
            }) => Ok(serde_json::json!({
                "status": "online",
                "socket_path": control_socket_path.to_string_lossy(),
                "active_workers": active_workers,
                "version": version
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

#[tauri::command]
async fn get_cache_status() -> Result<Value, String> {
    let mgr = get_cache_manager().await?;

    let caps = mgr
        .list_available_capabilities()
        .await
        .map_err(|e| format!("Failed to list capabilities: {:?}", e))?;
    let mods = mgr
        .list_available_modules()
        .await
        .map_err(|e| format!("Failed to list modules: {:?}", e))?;

    let caps_json: Vec<Value> = caps.into_iter().map(|(author, name, version)| {
        serde_json::json!({ "author": author, "name": name, "version": version })
    }).collect();

    let mods_json: Vec<Value> = mods.into_iter().map(|(author, name, version)| {
        serde_json::json!({ "author": author, "name": name, "version": version })
    }).collect();

    Ok(serde_json::json!({
        "cache_root": mgr.root.to_string_lossy(),
        "capabilities": caps_json,
        "modules": mods_json
    }))
}

#[tauri::command]
async fn purge_cache() -> Result<String, String> {
    let mgr = get_cache_manager().await?;
    mgr.purge()
        .await
        .map_err(|e| format!("Failed to purge cache: {:?}", e))?;
    Ok("Cache purged successfully".to_string())
}

#[tauri::command]
async fn list_active_playbooks() -> Result<Value, String> {
    let working_dir = pyro_daemon::PyroDaemon::default_working_dir();
    let control_socket_path = working_dir.join("control");

    if !control_socket_path.exists() {
        return Ok(serde_json::json!([]));
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
async fn start_playbook(
    name: String,
    config_path: String,
    playbook_socket: Option<String>,
    input_dir: Option<String>,
    output_dir: Option<String>,
) -> Result<String, String> {
    let working_dir = pyro_daemon::PyroDaemon::default_working_dir();
    let control_socket_path = working_dir.join("control");

    let client = pyro_daemon::client::DaemonClient::connect(&control_socket_path)
        .await
        .map_err(|e| format!("Failed to connect to daemon: {:?}", e))?;

    let req = pyro_daemon::DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::Start {
        name,
        playbook_config_path: std::path::PathBuf::from(config_path),
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
async fn stop_playbook(name: String) -> Result<String, String> {
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
async fn delete_playbook(name: String) -> Result<String, String> {
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
async fn call_playbook(name: String, payload: Value) -> Result<Value, String> {
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

#[tauri::command]
async fn get_capability_interface_spec(
    author: String,
    name: String,
    version: String,
) -> Result<Value, String> {
    let mgr = get_cache_manager().await?;
    let spec_str = mgr
        .capability_interface_spec(&author, &name, &version)
        .await
        .map_err(|e| format!("Failed to read interface spec: {:?}", e))?;
    let spec: Value = serde_json::from_str(&spec_str)
        .map_err(|e| format!("Failed to parse interface spec JSON: {:?}", e))?;
    Ok(spec)
}

#[tauri::command]
async fn get_playbook_spec(author: String, name: String, version: String) -> Result<Value, String> {
    let mgr = get_cache_manager().await?;
    let path = mgr.module_dir(&author, &name, &version).join("spec.json");
    let spec_str = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| format!("Failed to read playbook spec: {:?}", e))?;
    let spec: Value = serde_json::from_str(&spec_str)
        .map_err(|e| format!("Failed to parse playbook spec JSON: {:?}", e))?;
    Ok(spec)
}

#[tauri::command]
async fn get_pyroduct_config() -> Result<Value, String> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    let root = std::env::var("PYRODUCT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| home.join(".pyroduct"));
    let config_path = root.join("config.toml");
    if !config_path.exists() {
        return Ok(serde_json::json!({
            "author": "anon",
            "build_slots": 4
        }));
    }
    let content = tokio::fs::read_to_string(&config_path)
        .await
        .map_err(|e| format!("Failed to read config file: {:?}", e))?;
    let config: Value =
        toml::from_str(&content).map_err(|e| format!("Failed to parse config file: {:?}", e))?;
    Ok(config)
}

#[tauri::command]
async fn update_pyroduct_config(author: String, build_slots: Option<usize>) -> Result<(), String> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    let root = std::env::var("PYRODUCT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| home.join(".pyroduct"));
    let config_path = root.join("config.toml");

    let mut config = if config_path.exists() {
        let content = tokio::fs::read_to_string(&config_path)
            .await
            .map_err(|e| format!("Failed to read config file: {:?}", e))?;
        toml::from_str::<pyro_artifacts::cache::PyroductConfig>(&content)
            .map_err(|e| format!("Failed to parse config file: {:?}", e))?
    } else {
        pyro_artifacts::cache::PyroductConfig {
            author: "anon".to_string(),
            target: None,
            pyroduct: None,
            build_slots: Some(4),
        }
    };

    config.author = author;
    config.build_slots = build_slots;

    let content = toml::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {:?}", e))?;

    tokio::fs::write(&config_path, content)
        .await
        .map_err(|e| format!("Failed to write config file: {:?}", e))?;

    Ok(())
}

#[tauri::command]
async fn purge_capabilities_cache() -> Result<String, String> {
    let mgr = get_cache_manager().await?;
    mgr.purge_capabilities()
        .await
        .map_err(|e| format!("Failed to purge capabilities: {:?}", e))?;
    Ok("Capabilities cache purged successfully".to_string())
}

#[tauri::command]
async fn purge_modules_cache() -> Result<String, String> {
    let mgr = get_cache_manager().await?;
    mgr.purge_modules()
        .await
        .map_err(|e| format!("Failed to purge modules: {:?}", e))?;
    Ok("Modules cache purged successfully".to_string())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_daemon_status,
            get_cache_status,
            purge_cache,
            list_active_playbooks,
            start_playbook,
            stop_playbook,
            delete_playbook,
            call_playbook,
            get_capability_interface_spec,
            get_playbook_spec,
            get_pyroduct_config,
            update_pyroduct_config,
            purge_capabilities_cache,
            purge_modules_cache
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
