use serde_json::Value;
use tracing::{trace, debug, info, error};

async fn get_cache_manager() -> Result<pyro_artifacts::cache::CacheManager, String> {
    trace!("Initializing cache manager from environment");
    match pyro_artifacts::cache::CacheManager::from_env().await {
        Ok(mgr) => {
            trace!("Successfully loaded cache manager from env: {:?}", mgr.root);
            Ok(mgr)
        }
        Err(e) => {
            debug!("Failed to load cache manager from env: {:?}", e);
            // Fallback to default ~/.pyroduct
            let home = std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let root = home.join(".pyroduct");
            debug!("Falling back to cache root path: {:?}", root);
            pyro_artifacts::cache::CacheManager::new(&root, None, "anon".to_string())
                .await
                .map_err(|err| {
                    error!("Failed to initialize fallback cache manager at {:?}: {:?}", root, err);
                    err.to_string()
                })
        }
    }
}

#[tauri::command]
pub async fn get_cache_status() -> Result<Value, String> {
    info!("Tauri command: get_cache_status");
    let mgr = get_cache_manager().await?;

    debug!("Listing available capabilities and modules from cache root: {:?}", mgr.root);
    let caps = mgr
        .list_available_capabilities()
        .await
        .map_err(|e| {
            error!("Failed to list capabilities from cache: {:?}", e);
            format!("Failed to list capabilities: {:?}", e)
        })?;
    let mods = mgr
        .list_available_modules()
        .await
        .map_err(|e| {
            error!("Failed to list playbooks from cache: {:?}", e);
            format!("Failed to list playbooks: {:?}", e)
        })?;

    trace!("Retrieved {} capabilities and {} playbooks", caps.len(), mods.len());

    let caps_json: Vec<Value> = caps.into_iter().map(|(author, name, version)| {
        serde_json::json!({ "author": author, "name": name, "version": version })
    }).collect();

    let mods_json: Vec<Value> = mods.into_iter().map(|(author, name, version)| {
        serde_json::json!({ "author": author, "name": name, "version": version })
    }).collect();

    let response = serde_json::json!({
        "cache_root": mgr.root.to_string_lossy(),
        "capabilities": caps_json,
        "playbooks": mods_json
    });
    trace!("Cache status response: {:?}", response);

    Ok(response)
}

#[tauri::command]
pub async fn purge_cache() -> Result<String, String> {
    info!("Tauri command: purge_cache");
    let mgr = get_cache_manager().await?;
    debug!("Purging all cache data under root: {:?}", mgr.root);
    mgr.purge()
        .await
        .map_err(|e| {
            error!("Failed to purge cache under {:?}: {:?}", mgr.root, e);
            format!("Failed to purge cache: {:?}", e)
        })?;
    info!("Successfully purged entire cache");
    Ok("Cache purged successfully".to_string())
}

#[tauri::command]
pub async fn purge_capabilities_cache() -> Result<String, String> {
    info!("Tauri command: purge_capabilities_cache");
    let mgr = get_cache_manager().await?;
    debug!("Purging capabilities cache under root: {:?}", mgr.root);
    mgr.purge_capabilities()
        .await
        .map_err(|e| {
            error!("Failed to purge capabilities cache: {:?}", e);
            format!("Failed to purge capabilities: {:?}", e)
        })?;
    info!("Successfully purged capabilities cache");
    Ok("Capabilities cache purged successfully".to_string())
}

#[tauri::command]
pub async fn purge_playbooks_cache() -> Result<String, String> {
    info!("Tauri command: purge_playbooks_cache");
    let mgr = get_cache_manager().await?;
    debug!("Purging playbooks cache under root: {:?}", mgr.root);
    mgr.purge_modules()
        .await
        .map_err(|e| {
            error!("Failed to purge playbooks cache: {:?}", e);
            format!("Failed to purge playbooks: {:?}", e)
        })?;
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
    let mgr = get_cache_manager().await?;
    debug!("Reading interface spec for {}/{}@{}", author, name, version);
    let spec_str = mgr
        .capability_interface_spec(&author, &name, &version)
        .await
        .map_err(|e| {
            error!("Failed to read interface spec for {}/{}@{}: {:?}", author, name, version, e);
            format!("Failed to read interface spec: {:?}", e)
        })?;
    trace!("Parsing interface spec JSON string: {}", spec_str);
    let spec: Value = serde_json::from_str(&spec_str)
        .map_err(|e| {
            error!("Failed to parse interface spec JSON: {:?}", e);
            format!("Failed to parse interface spec JSON: {:?}", e)
        })?;
    Ok(spec)
}

#[tauri::command]
pub async fn get_playbook_spec(author: String, name: String, version: String) -> Result<Value, String> {
    info!("Tauri command: get_playbook_spec for {}/{}@{}", author, name, version);
    let mgr = get_cache_manager().await?;
    let path = mgr.module_dir(&author, &name, &version).join("spec.json");
    debug!("Reading playbook spec file from path: {:?}", path);
    let spec_str = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| {
            error!("Failed to read playbook spec file from path {:?}: {:?}", path, e);
            format!("Failed to read playbook spec: {:?}", e)
        })?;
    trace!("Parsing playbook spec JSON: {}", spec_str);
    let spec: Value = serde_json::from_str(&spec_str)
        .map_err(|e| {
            error!("Failed to parse playbook spec JSON: {:?}", e);
            format!("Failed to parse playbook spec JSON: {:?}", e)
        })?;
    Ok(spec)
}

#[tauri::command]
pub async fn get_playbook_configurations(
    author: String,
    name: String,
    version: String,
) -> Result<Value, String> {
    info!("Tauri command: get_playbook_configurations for {}/{}@{}", author, name, version);
    let mgr = get_cache_manager().await?;
    let binary = mgr
        .get_named_binary(&author, &name, &version)
        .await
        .map_err(|e| {
            error!("Failed to load playbook binary for {}/{}@{}: {:?}", author, name, version, e);
            format!("Failed to load playbook binary: {:?}", e)
        })?;
    let configs = serde_json::to_value(&binary.configurations)
        .map_err(|e| {
            error!("Failed to serialize configurations: {:?}", e);
            format!("Failed to serialize configurations: {:?}", e)
        })?;
    Ok(configs)
}

#[tauri::command]
pub async fn get_playbook_source(
    author: String,
    name: String,
    version: String,
) -> Result<String, String> {
    info!("Tauri command: get_playbook_source for {}/{}@{}", author, name, version);
    let mgr = get_cache_manager().await?;
    let path = mgr.module_dir(&author, &name, &version).join("source.rs");
    debug!("Checking if playbook source file exists at path: {:?}", path);
    if !path.exists() {
        debug!("Playbook source file does not exist at path: {:?}", path);
        return Ok("".to_string());
    }
    debug!("Reading playbook source from path: {:?}", path);
    let src = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| {
            error!("Failed to read playbook source file: {:?}", e);
            format!("Failed to read playbook source: {:?}", e)
        })?;
    trace!("Successfully read playbook source, size: {} bytes", src.len());
    Ok(src)
}

#[tauri::command]
pub async fn get_pyroduct_config() -> Result<Value, String> {
    info!("Tauri command: get_pyroduct_config");
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    let root = std::env::var("PYRODUCT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| home.join(".pyroduct"));
    let config_path = root.join("config.toml");
    debug!("Pyroduct config path: {:?}", config_path);
    if !config_path.exists() {
        debug!("Pyroduct config file does not exist. Returning default configuration.");
        return Ok(serde_json::json!({
            "author": "anon",
            "build_slots": 4
        }));
    }
    debug!("Reading config file content from path: {:?}", config_path);
    let content = tokio::fs::read_to_string(&config_path)
        .await
        .map_err(|e| {
            error!("Failed to read config file: {:?}", e);
            format!("Failed to read config file: {:?}", e)
        })?;
    let config: Value =
        toml::from_str(&content).map_err(|e| {
            error!("Failed to parse config file: {:?}", e);
            format!("Failed to parse config file: {:?}", e)
        })?;
    trace!("Parsed config content: {:?}", config);
    Ok(config)
}

#[tauri::command]
pub async fn update_pyroduct_config(author: String, build_slots: Option<usize>) -> Result<(), String> {
    info!("Tauri command: update_pyroduct_config with author={}, build_slots={:?}", author, build_slots);
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    let root = std::env::var("PYRODUCT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| home.join(".pyroduct"));
    let config_path = root.join("config.toml");
    debug!("Target pyroduct config path for update: {:?}", config_path);

    let mut config = if config_path.exists() {
        debug!("Reading existing config file for merge");
        let content = tokio::fs::read_to_string(&config_path)
            .await
            .map_err(|e| {
                error!("Failed to read existing config: {:?}", e);
                format!("Failed to read config file: {:?}", e)
            })?;
        toml::from_str::<pyro_artifacts::cache::PyroductConfig>(&content)
            .map_err(|e| {
                error!("Failed to parse existing config: {:?}", e);
                format!("Failed to parse config file: {:?}", e)
            })?
    } else {
        debug!("Creating a new default configuration structure");
        pyro_artifacts::cache::PyroductConfig {
            author: "anon".to_string(),
            target: None,
            pyroduct: None,
            build_slots: Some(4),
        }
    };

    config.author = author;
    config.build_slots = build_slots;
    trace!("Updated config struct: {:?}", config);

    let content = toml::to_string_pretty(&config)
        .map_err(|e| {
            error!("Failed to serialize config to TOML: {:?}", e);
            format!("Failed to serialize config: {:?}", e)
        })?;

    debug!("Writing updated config to path: {:?}", config_path);
    tokio::fs::write(&config_path, content)
        .await
        .map_err(|e| {
            error!("Failed to write updated config file: {:?}", e);
            format!("Failed to write config file: {:?}", e)
        })?;

    info!("Successfully updated pyroduct config");
    Ok(())
}
