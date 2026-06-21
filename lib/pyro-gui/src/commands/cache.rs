use serde_json::Value;
use tracing::{trace, debug, info, error};

use pyro_daemon::{DaemonRequest, DaemonResponse, CacheRequest, CacheResponse};
use crate::commands::daemon::connect_to_active_daemon;

// ── helper ────────────────────────────────────────────────────────────────────

async fn cache_request(req: CacheRequest) -> Result<CacheResponse, String> {
    let client = connect_to_active_daemon().await?;
    match client.request(DaemonRequest::Cache(req)).await {
        Ok(DaemonResponse::Cache(resp)) => Ok(resp),
        Ok(DaemonResponse::Error { message }) => Err(message),
        Ok(other) => Err(format!("Unexpected daemon response: {:?}", other)),
        Err(e) => Err(format!("Daemon request failed: {:?}", e)),
    }
}

fn unwrap_ok(resp: CacheResponse, context: &str) -> Result<(), String> {
    match resp {
        CacheResponse::Ok => Ok(()),
        CacheResponse::Error { message } => {
            error!("{}: {}", context, message);
            Err(message)
        }
        other => Err(format!("{}: unexpected response {:?}", context, other)),
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_cache_status() -> Result<Value, String> {
    info!("Tauri command: get_cache_status");

    let (caps_resp, mods_resp, status_resp) = tokio::join!(
        cache_request(CacheRequest::ListCapabilities { author: None, package: None }),
        cache_request(CacheRequest::ListModules { author: None, package: None }),
        cache_request(CacheRequest::Status),
    );

    let caps = match caps_resp? {
        CacheResponse::ArtifactList { items } => items,
        other => return Err(format!("Unexpected response listing capabilities: {:?}", other)),
    };
    let mods = match mods_resp? {
        CacheResponse::ArtifactList { items } => items,
        other => return Err(format!("Unexpected response listing modules: {:?}", other)),
    };
    let cache_root = match status_resp? {
        CacheResponse::Status { cache_root, .. } => cache_root,
        other => return Err(format!("Unexpected response for status: {:?}", other)),
    };

    trace!("Retrieved {} capabilities and {} playbooks", caps.len(), mods.len());

    let caps_json: Vec<Value> = caps.into_iter().map(|(author, name, version)| {
        serde_json::json!({ "author": author, "name": name, "version": version })
    }).collect();
    let mods_json: Vec<Value> = mods.into_iter().map(|(author, name, version)| {
        serde_json::json!({ "author": author, "name": name, "version": version })
    }).collect();

    Ok(serde_json::json!({
        "cache_root": cache_root,
        "capabilities": caps_json,
        "playbooks": mods_json,
    }))
}

#[tauri::command]
pub async fn purge_cache() -> Result<String, String> {
    info!("Tauri command: purge_cache");
    unwrap_ok(cache_request(CacheRequest::PurgeCache).await?, "purge_cache")?;
    info!("Successfully purged entire cache");
    Ok("Cache purged successfully".to_string())
}

#[tauri::command]
pub async fn purge_capabilities_cache() -> Result<String, String> {
    info!("Tauri command: purge_capabilities_cache");
    unwrap_ok(cache_request(CacheRequest::PurgeCapabilities).await?, "purge_capabilities_cache")?;
    info!("Successfully purged capabilities cache");
    Ok("Capabilities cache purged successfully".to_string())
}

#[tauri::command]
pub async fn purge_playbooks_cache() -> Result<String, String> {
    info!("Tauri command: purge_playbooks_cache");
    unwrap_ok(cache_request(CacheRequest::PurgeModules).await?, "purge_playbooks_cache")?;
    info!("Successfully purged playbooks cache");
    Ok("Playbooks cache purged successfully".to_string())
}

#[tauri::command]
pub async fn get_capability_interface_spec(
    author: String,
    name: String,
    version: String,
) -> Result<Value, String> {
    info!("Tauri command: get_capability_interface_spec for {}/{}@{}", author, name, version);
    let resp = cache_request(CacheRequest::GetCapabilityInterfaceSpec {
        author: author.clone(),
        name: name.clone(),
        version: version.clone(),
    }).await?;
    match resp {
        CacheResponse::Text { content } => {
            debug!("Parsing interface spec JSON for {}/{}@{}", author, name, version);
            serde_json::from_str(&content).map_err(|e| {
                error!("Failed to parse interface spec JSON: {:?}", e);
                format!("Failed to parse interface spec JSON: {:?}", e)
            })
        }
        CacheResponse::Error { message } => {
            error!("Failed to get interface spec: {}", message);
            Err(message)
        }
        other => Err(format!("Unexpected response: {:?}", other)),
    }
}

#[tauri::command]
pub async fn get_playbook_spec(author: String, name: String, version: String) -> Result<Value, String> {
    info!("Tauri command: get_playbook_spec for {}/{}@{}", author, name, version);
    let resp = cache_request(CacheRequest::GetPlaybookSpec {
        author: author.clone(),
        name: name.clone(),
        version: version.clone(),
    }).await?;
    match resp {
        CacheResponse::Json { value } => Ok(value),
        CacheResponse::Error { message } => {
            error!("Failed to get playbook spec for {}/{}@{}: {}", author, name, version, message);
            Err(message)
        }
        other => Err(format!("Unexpected response: {:?}", other)),
    }
}

#[tauri::command]
pub async fn get_playbook_configurations(
    author: String,
    name: String,
    version: String,
) -> Result<Value, String> {
    info!("Tauri command: get_playbook_configurations for {}/{}@{}", author, name, version);
    let resp = cache_request(CacheRequest::GetPlaybookConfigurations {
        author: author.clone(),
        name: name.clone(),
        version: version.clone(),
    }).await?;
    match resp {
        CacheResponse::Json { value } => Ok(value),
        CacheResponse::Error { message } => {
            error!("Failed to get configurations for {}/{}@{}: {}", author, name, version, message);
            Err(message)
        }
        other => Err(format!("Unexpected response: {:?}", other)),
    }
}

#[tauri::command]
pub async fn get_playbook_source(
    author: String,
    name: String,
    version: String,
) -> Result<String, String> {
    info!("Tauri command: get_playbook_source for {}/{}@{}", author, name, version);
    let resp = cache_request(CacheRequest::GetPlaybookSource {
        author: author.clone(),
        name: name.clone(),
        version: version.clone(),
    }).await?;
    match resp {
        CacheResponse::Text { content } => {
            trace!("Got playbook source, {} bytes", content.len());
            Ok(content)
        }
        CacheResponse::Error { message } => {
            error!("Failed to get source for {}/{}@{}: {}", author, name, version, message);
            Err(message)
        }
        other => Err(format!("Unexpected response: {:?}", other)),
    }
}

#[tauri::command]
pub async fn get_pyroduct_config() -> Result<Value, String> {
    info!("Tauri command: get_pyroduct_config");
    let resp = cache_request(CacheRequest::GetPyroductConfig).await?;
    match resp {
        CacheResponse::Json { value } => {
            trace!("Got pyroduct config: {:?}", value);
            Ok(value)
        }
        CacheResponse::Error { message } => {
            error!("Failed to get pyroduct config: {}", message);
            Err(message)
        }
        other => Err(format!("Unexpected response: {:?}", other)),
    }
}

#[tauri::command]
pub async fn update_pyroduct_config(author: String, build_slots: Option<usize>) -> Result<(), String> {
    info!("Tauri command: update_pyroduct_config with author={}, build_slots={:?}", author, build_slots);
    // Read the current config from the daemon, merge locally, then write back.
    // Writing config is a local filesystem operation; do it through the daemon's
    // working directory so the daemon's view stays in sync.
    let client = connect_to_active_daemon().await?;

    // 1. Fetch existing config
    let current = match client
        .request(DaemonRequest::Cache(CacheRequest::GetPyroductConfig))
        .await
        .map_err(|e| format!("Failed to fetch config: {:?}", e))?
    {
        DaemonResponse::Cache(CacheResponse::Json { value }) => value,
        DaemonResponse::Cache(CacheResponse::Error { message }) => return Err(message),
        other => return Err(format!("Unexpected response: {:?}", other)),
    };

    // 2. Merge: overwrite only the fields we care about
    let mut config: pyro_artifacts::cache::PyroductConfig =
        serde_json::from_value(current).unwrap_or_else(|_| pyro_artifacts::cache::PyroductConfig {
            author: "anon".to_string(),
            target: None,
            pyroduct: None,
            build_slots: Some(4),
        });

    config.author = author;
    config.build_slots = build_slots;
    debug!("Merged config: {:?}", config);

    // 3. Write back to local disk (config.toml lives in the pyroduct root,
    //    which is a local path regardless of which daemon we're talking to).
    let root = std::env::var("PYRODUCT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .unwrap_or_else(|_| ".".to_string());
            std::path::PathBuf::from(home).join(".pyroduct")
        });
    let config_path = root.join("config.toml");
    let content = toml::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {:?}", e))?;
    tokio::fs::write(&config_path, content)
        .await
        .map_err(|e| format!("Failed to write config file: {:?}", e))?;

    info!("Successfully updated pyroduct config");
    Ok(())
}
