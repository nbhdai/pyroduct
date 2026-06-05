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
pub async fn get_cache_status() -> Result<Value, String> {
    let mgr = get_cache_manager().await?;

    let caps = mgr
        .list_available_capabilities()
        .await
        .map_err(|e| format!("Failed to list capabilities: {:?}", e))?;
    let mods = mgr
        .list_available_modules()
        .await
        .map_err(|e| format!("Failed to list playbooks: {:?}", e))?;

    let caps_json: Vec<Value> = caps.into_iter().map(|(author, name, version)| {
        serde_json::json!({ "author": author, "name": name, "version": version })
    }).collect();

    let mods_json: Vec<Value> = mods.into_iter().map(|(author, name, version)| {
        serde_json::json!({ "author": author, "name": name, "version": version })
    }).collect();

    Ok(serde_json::json!({
        "cache_root": mgr.root.to_string_lossy(),
        "capabilities": caps_json,
        "playbooks": mods_json
    }))
}

#[tauri::command]
pub async fn purge_cache() -> Result<String, String> {
    let mgr = get_cache_manager().await?;
    mgr.purge()
        .await
        .map_err(|e| format!("Failed to purge cache: {:?}", e))?;
    Ok("Cache purged successfully".to_string())
}

#[tauri::command]
pub async fn purge_capabilities_cache() -> Result<String, String> {
    let mgr = get_cache_manager().await?;
    mgr.purge_capabilities()
        .await
        .map_err(|e| format!("Failed to purge capabilities: {:?}", e))?;
    Ok("Capabilities cache purged successfully".to_string())
}

#[tauri::command]
pub async fn purge_playbooks_cache() -> Result<String, String> {
    let mgr = get_cache_manager().await?;
    mgr.purge_modules()
        .await
        .map_err(|e| format!("Failed to purge playbooks: {:?}", e))?;
    Ok("Playbooks cache purged successfully".to_string())
}

#[tauri::command]
pub async fn get_capability_interface_spec(
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
pub async fn get_playbook_spec(author: String, name: String, version: String) -> Result<Value, String> {
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
pub async fn get_playbook_source(
    author: String,
    name: String,
    version: String,
) -> Result<String, String> {
    let mgr = get_cache_manager().await?;
    let path = mgr.module_dir(&author, &name, &version).join("source.rs");
    if !path.exists() {
        return Ok("".to_string());
    }
    let src = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| format!("Failed to read playbook source: {:?}", e))?;
    Ok(src)
}

#[tauri::command]
pub async fn get_pyroduct_config() -> Result<Value, String> {
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
pub async fn update_pyroduct_config(author: String, build_slots: Option<usize>) -> Result<(), String> {
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
