use serde_json::Value;
use std::sync::OnceLock;
use tracing::{debug, error, info};

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonConnection {
    Unix { path: std::path::PathBuf },
    Tcp { address: String },
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
pub struct GuiSettings {
    pub selected_daemon: Option<String>,
    pub daemons: std::collections::HashMap<String, DaemonConnection>,
}

impl GuiSettings {
    pub fn file_path() -> std::path::PathBuf {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        let root = std::env::var("PYRODUCT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from(home).join(".pyroduct"));
        root.join("gui_config.json")
    }

    pub async fn load() -> Self {
        let path = Self::file_path();
        if !path.exists() {
            return Self::default();
        }
        match tokio::fs::read_to_string(&path).await {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub async fn save(&self) -> Result<(), String> {
        let path = Self::file_path();
        if let Some(parent) = path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Serialization error: {}", e))?;
        tokio::fs::write(&path, content)
            .await
            .map_err(|e| format!("Failed to write gui_config.json: {}", e))?;
        Ok(())
    }
}

static CACHED_CLIENT: OnceLock<tokio::sync::Mutex<Option<pyro_daemon::client::DaemonClient>>> =
    OnceLock::new();

fn client_cache() -> &'static tokio::sync::Mutex<Option<pyro_daemon::client::DaemonClient>> {
    CACHED_CLIENT.get_or_init(|| tokio::sync::Mutex::new(None))
}

pub async fn connect_to_active_daemon() -> Result<pyro_daemon::client::DaemonClient, String> {
    let mut guard = client_cache().lock().await;

    // Return cached client if the connection is still alive
    if let Some(ref client) = *guard {
        if client.is_connected() {
            return Ok(client.clone());
        }
        debug!("Cached daemon connection is stale, reconnecting");
    }

    let client = establish_daemon_connection().await?;
    *guard = Some(client.clone());
    Ok(client)
}

/// Invalidate the cached connection (e.g. after settings change).
pub async fn invalidate_cached_connection() {
    *client_cache().lock().await = None;
}

async fn establish_daemon_connection() -> Result<pyro_daemon::client::DaemonClient, String> {
    let settings = GuiSettings::load().await;
    if let Some(selected) = settings.selected_daemon {
        if let Some(conn) = settings.daemons.get(&selected) {
            match conn {
                DaemonConnection::Unix { path } => {
                    return pyro_daemon::client::DaemonClient::connect(path)
                        .await
                        .map_err(|e| {
                            format!(
                                "Failed to connect to Unix daemon '{}' at {:?}: {:?}",
                                selected, path, e
                            )
                        });
                }
                DaemonConnection::Tcp { address } => {
                    return pyro_daemon::client::DaemonClient::connect_tcp(address)
                        .await
                        .map_err(|e| {
                            format!(
                                "Failed to connect to TCP daemon '{}' at {}: {:?}",
                                selected, address, e
                            )
                        });
                }
            }
        }
    }

    // Fallback: use default local Unix socket control path
    let working_dir = pyro_daemon::PyroDaemon::default_working_dir();
    let control_socket_path = working_dir.join("control");
    pyro_daemon::client::DaemonClient::connect(&control_socket_path)
        .await
        .map_err(|e| {
            format!(
                "Failed to connect to default daemon at {:?}: {:?}",
                control_socket_path, e
            )
        })
}

#[tauri::command]
pub async fn get_gui_settings() -> Result<GuiSettings, String> {
    info!("Tauri command: get_gui_settings");
    Ok(GuiSettings::load().await)
}

#[tauri::command]
pub async fn update_gui_settings(settings: GuiSettings) -> Result<(), String> {
    info!("Tauri command: update_gui_settings: {:?}", settings);
    settings.save().await?;
    // Invalidate cached connection since daemon settings may have changed
    invalidate_cached_connection().await;
    Ok(())
}

#[tauri::command]
pub async fn get_daemon_status() -> Result<Value, String> {
    info!("Tauri command: get_daemon_status");

    let settings = GuiSettings::load().await;
    let mut selected_name = "Default (Local)".to_string();
    let mut selected_conn = DaemonConnection::Unix {
        path: pyro_daemon::PyroDaemon::default_working_dir().join("control"),
    };

    if let Some(ref selected) = settings.selected_daemon {
        if let Some(conn) = settings.daemons.get(selected) {
            selected_name = selected.clone();
            selected_conn = conn.clone();
        }
    }

    let socket_description = match &selected_conn {
        DaemonConnection::Unix { path } => path.to_string_lossy().to_string(),
        DaemonConnection::Tcp { address } => address.clone(),
    };

    match connect_to_active_daemon().await {
        Ok(client) => {
            debug!("Connected to active daemon. Requesting status details.");
            match client.request(pyro_daemon::DaemonRequest::Status).await {
                Ok(pyro_daemon::DaemonResponse::StatusInfo {
                    active_workers,
                    version,
                    running_playbooks,
                }) => {
                    info!(
                        "Daemon is online. Version: {}, Active Workers: {}, Running Playbooks: {:?}",
                        version, active_workers, running_playbooks
                    );
                    Ok(serde_json::json!({
                        "status": "online",
                        "socket_path": socket_description,
                        "daemon_name": selected_name,
                        "active_workers": active_workers,
                        "version": version,
                        "running_playbooks": running_playbooks,
                    }))
                }
                Ok(resp) => {
                    error!(
                        "Received unexpected status response from daemon: {:?}",
                        resp
                    );
                    Ok(serde_json::json!({
                        "status": "online",
                        "socket_path": socket_description,
                        "daemon_name": selected_name,
                        "message": format!("Unexpected response: {:?}", resp)
                    }))
                }
                Err(e) => {
                    error!("Connected to daemon, but status request failed: {:?}", e);
                    Ok(serde_json::json!({
                        "status": "error",
                        "socket_path": socket_description,
                        "daemon_name": selected_name,
                        "message": format!("Connected but status request failed: {:?}", e)
                    }))
                }
            }
        }
        Err(e) => {
            error!("Failed to connect to daemon socket: {:?}", e);
            Ok(serde_json::json!({
                "status": "offline",
                "socket_path": socket_description,
                "daemon_name": selected_name,
                "message": format!("Failed to connect: {:?}", e)
            }))
        }
    }
}
