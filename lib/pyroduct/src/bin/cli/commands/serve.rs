use anyhow::{Context, Result, anyhow};
use clap::ValueEnum;
use fs_err as fs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use pyro_artifacts::cache::CacheManager;
use pyroduct::format::PyroVec;
use pyroduct::format::format::{PyroFormat, Writer};
use pyroduct::format::json::Json;
use pyroduct::pipeline::factory::PipelineConfig;
use pyroduct::transport::http::PlaybookHttpServer;
use pyroduct::transport::socket::PyroListener;
use pyroduct::transport::socket::capability::{PyroRouter, PyroServer};
use pyroduct::transport::socket::playbook::PlaybookServer;

#[derive(ValueEnum, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServerType {
    Playbook,
    Capability,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServeConfig {
    #[serde(rename = "type")]
    pub server_type: ServerType,
    pub socket: String,
    #[serde(default)]
    pub http: bool,

    // Playbook server configuration:
    pub playbook_config: Option<PathBuf>,
    pub pipeline_config: Option<PipelineConfig>,

    // Capability server configuration:
    pub cap_name: Option<String>,
    pub cap_path: Option<PathBuf>,
    pub cap_config: Option<serde_json::Value>,
}

/// Resolves the unified `ServeConfig` either by loading it from a JSON file/string,
/// or by constructing it from the provided CLI arguments.
pub fn resolve_config(
    config_path: Option<&Path>,
    config_json: Option<&str>,
    server_type: Option<ServerType>,
    socket: Option<String>,
    http: bool,
    playbook_config: Option<&Path>,
    cap_name: Option<&str>,
    cap_path: Option<&Path>,
    cap_config: Option<&str>,
) -> Result<ServeConfig> {
    if let Some(path) = config_path {
        let content = fs::read_to_string(path).context("Failed to read JSON config file")?;
        serde_json::from_str(&content).context("Failed to parse JSON config file")
    } else if let Some(json_str) = config_json {
        serde_json::from_str(json_str).context("Failed to parse JSON config string")
    } else {
        // Build from CLI args
        let server_type = server_type.ok_or_else(|| {
            anyhow!("Either --config, --config-json or --server-type must be provided")
        })?;
        let socket = socket.ok_or_else(|| {
            anyhow!("Either --config, --config-json or --socket must be provided")
        })?;

        let parsed_cap_config = if let Some(cfg_str) = cap_config {
            Some(
                serde_json::from_str(cfg_str)
                    .context("Failed to parse capability configuration JSON string")?,
            )
        } else {
            None
        };

        Ok(ServeConfig {
            server_type,
            socket,
            http,
            playbook_config: playbook_config.map(Path::to_path_buf),
            pipeline_config: None,
            cap_name: cap_name.map(String::from),
            cap_path: cap_path.map(Path::to_path_buf),
            cap_config: parsed_cap_config,
        })
    }
}

/// Starts and runs a Playbook server (TCP, Unix socket, or HTTP) using the provided configuration.
pub async fn serve_playbook(serve_config: &ServeConfig) -> Result<()> {
    // Load playbook configuration
    let cache = CacheManager::from_env().await?;
    let loaded_pipeline_config = if let Some(ref pc) = serve_config.pipeline_config {
        pc.clone()
            .load(&cache)
            .await
            .context("Failed to load inline playbook config")?
    } else if let Some(ref path) = serve_config.playbook_config {
        let config_str = fs::read_to_string(path)?;
        let pipeline: PipelineConfig = match path.extension().map(|s| s.as_encoded_bytes()) {
            Some(b"toml") => {
                toml::from_str(&config_str).context("Failed to parse pipeline TOML")?
            }
            Some(b"yaml") => {
                serde_yaml::from_str(&config_str).context("Failed to parse pipeline yaml")?
            }
            Some(b"json") => {
                serde_json::from_str(&config_str).context("Failed to parse pipeline JSON")?
            }
            _ => {
                anyhow::bail!("Unknown extension for playbook config, supports toml, yaml and json")
            }
        };
        pipeline
            .load(&cache)
            .await
            .context("Failed to load playbook config file")?
    } else {
        anyhow::bail!(
            "No playbook configuration provided. Must provide either --playbook-config or pipeline_config in the JSON configuration."
        );
    };

    if serve_config.http {
        tracing::info!("Starting playbook HTTP server...");
        let addr = serve_config
            .socket
            .parse::<std::net::SocketAddr>()
            .context("HTTP server requires a valid TCP socket address (e.g. 127.0.0.1:8080)")?;
        let tcp_listener = tokio::net::TcpListener::bind(addr)
            .await
            .context("Failed to bind TCP listener for HTTP server")?;
        let server = PlaybookHttpServer::new(&loaded_pipeline_config.playbook)
            .await
            .context("Failed to build HTTP playbook server")?;
        tracing::info!("Playbook HTTP server listening on {}", addr);
        server
            .run(tcp_listener)
            .await
            .map_err(|e| anyhow!("Playbook HTTP server error: {}", e))?;
    } else {
        tracing::info!("Starting playbook TCP/Unix socket server...");
        let server = PlaybookServer::new(&loaded_pipeline_config.playbook)
            .await
            .context("Failed to build playbook server")?;

        let listener = if let Ok(addr) = serve_config.socket.parse::<std::net::SocketAddr>() {
            tracing::info!("Binding playbook TCP listener to {}", addr);
            PyroListener::bind_tcp(addr)
                .await
                .context("Failed to bind TCP listener for playbook server")?
        } else {
            let socket_path = Path::new(&serve_config.socket);
            if socket_path.exists() {
                let _ = std::fs::remove_file(socket_path);
            }
            tracing::info!("Binding playbook Unix listener to {:?}", socket_path);
            PyroListener::bind_unix(socket_path)
                .await
                .context("Failed to bind Unix listener for playbook server")?
        };

        server
            .run(listener)
            .await
            .context("Playbook server run error")?;
    }

    Ok(())
}

/// Starts and runs a Capability server (TCP or Unix socket) using the provided configuration.
pub async fn serve_capability(serve_config: &ServeConfig) -> Result<()> {
    let cap_name = serve_config
        .cap_name
        .as_ref()
        .ok_or_else(|| anyhow!("Capability name is required to run a capability server"))?;
    let cap_path = serve_config
        .cap_path
        .as_ref()
        .ok_or_else(|| anyhow!("Capability library path is required to run a capability server"))?;

    tracing::info!("Loading capability '{}' from {:?}", cap_name, cap_path);
    let mut router = PyroRouter::load(cap_name.clone(), cap_path)
        .context("Failed to load capability library")?;

    // Pre-configure capability classes if config provided
    if let Some(ref cap_config) = serve_config.cap_config {
        if let serde_json::Value::Object(obj) = cap_config {
            for (class_name, class_config) in obj {
                tracing::info!("Pre-configuring capability class '{}'", class_name);
                let writer = Json::<serde_json::Value>::new_writer(PyroVec::with_capacity(300));
                let vec = writer.write(class_config).map_err(|e| {
                    anyhow!(
                        "Failed to serialize config for class '{}': {}",
                        class_name,
                        e
                    )
                })?;
                router
                    .configure(class_name, vec.view())
                    .await
                    .context(format!("Failed to configure class '{}'", class_name))?;
            }
        } else {
            anyhow::bail!(
                "Capability configuration must be a JSON object mapping class names to their configurations"
            );
        }
    }

    tracing::info!("Starting capability TCP/Unix socket server...");
    let server = PyroServer::new(router);

    let listener = if let Ok(addr) = serve_config.socket.parse::<std::net::SocketAddr>() {
        tracing::info!("Binding capability TCP listener to {}", addr);
        PyroListener::bind_tcp(addr)
            .await
            .context("Failed to bind TCP listener for capability server")?
    } else {
        let socket_path = Path::new(&serve_config.socket);
        if socket_path.exists() {
            let _ = std::fs::remove_file(socket_path);
        }
        tracing::info!("Binding capability Unix listener to {:?}", socket_path);
        PyroListener::bind_unix(socket_path)
            .await
            .context("Failed to bind Unix listener for capability server")?
    };

    server
        .run(listener)
        .await
        .context("Capability server run error")?;

    Ok(())
}

/// Main serve command handler that resolves configuration and runs the appropriate server type.
pub async fn serve(
    config_path: Option<&Path>,
    config_json: Option<&str>,
    server_type: Option<ServerType>,
    socket: Option<String>,
    http: bool,
    playbook_config: Option<&Path>,
    cap_name: Option<&str>,
    cap_path: Option<&Path>,
    cap_config: Option<&str>,
) -> Result<()> {
    let serve_config = resolve_config(
        config_path,
        config_json,
        server_type,
        socket,
        http,
        playbook_config,
        cap_name,
        cap_path,
        cap_config,
    )?;

    match serve_config.server_type {
        ServerType::Playbook => serve_playbook(&serve_config).await,
        ServerType::Capability => serve_capability(&serve_config).await,
    }
}
