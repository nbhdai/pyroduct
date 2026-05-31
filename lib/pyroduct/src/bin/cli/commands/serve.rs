use anyhow::{Context, Result, anyhow};
use clap::ValueEnum;
use fs_err as fs;
use pyro_artifacts::cargo::CapabilityIdent;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use pyro_artifacts::cache::CacheManager;
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
#[serde(untagged)]
pub enum PlaybookServerConfig {
    Path(PathBuf),
    Ident(pyro_artifacts::artifacts::PlaybookIdent),
    Inline(PipelineConfig),
}

fn default_wal_capacity() -> usize {
    1000
}
fn default_success_retention() -> u64 {
    3600
}
fn default_error_retention() -> u64 {
    86400 * 7
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServeConfig {
    #[serde(rename = "type")]
    pub server_type: ServerType,
    pub socket: String,
    #[serde(default)]
    pub http: bool,

    // Playbook server configuration:
    pub playbook: Option<PlaybookServerConfig>,

    // Extra settings for playbook/pipeline config:
    #[serde(default)]
    pub remote: std::collections::HashMap<CapabilityIdent, pyro_artifacts::cache::RemoteAddress>,
    #[serde(default = "default_wal_capacity")]
    pub wal_capacity: usize,
    #[serde(default = "default_success_retention")]
    pub success_log_retention_secs: u64,
    #[serde(default = "default_error_retention")]
    pub error_log_retention_secs: u64,
    pub log_dir: Option<PathBuf>,
    pub input_dir: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,

    // Capability server configuration:
    pub cap: Option<CapabilityIdent>,
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
    playbook_ident: Option<pyro_artifacts::artifacts::PlaybookIdent>,
    remote: std::collections::HashMap<CapabilityIdent, pyro_artifacts::cache::RemoteAddress>,
    wal_capacity: Option<usize>,
    success_log_retention_secs: Option<u64>,
    error_log_retention_secs: Option<u64>,
    log_dir: Option<PathBuf>,
    input_dir: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    cap: Option<CapabilityIdent>,
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

        let playbook = if let Some(p) = playbook_config {
            Some(PlaybookServerConfig::Path(p.to_path_buf()))
        } else if let Some(ident) = playbook_ident {
            Some(PlaybookServerConfig::Ident(ident))
        } else {
            None
        };

        Ok(ServeConfig {
            server_type,
            socket,
            http,
            playbook,
            remote,
            wal_capacity: wal_capacity.unwrap_or_else(default_wal_capacity),
            success_log_retention_secs: success_log_retention_secs
                .unwrap_or_else(default_success_retention),
            error_log_retention_secs: error_log_retention_secs
                .unwrap_or_else(default_error_retention),
            log_dir,
            input_dir,
            output_dir,
            cap,
            cap_config: parsed_cap_config,
        })
    }
}

/// Starts and runs a Playbook server (TCP, Unix socket, or HTTP) using the provided configuration.
pub async fn serve_playbook(serve_config: &ServeConfig) -> Result<()> {
    // Load playbook configuration
    let cache = CacheManager::from_env().await?;
    let loaded_pipeline_config = match &serve_config.playbook {
        Some(PlaybookServerConfig::Inline(pc)) => pc
            .clone()
            .load(&cache)
            .await
            .context("Failed to load inline playbook config")?,
        Some(PlaybookServerConfig::Ident(playbook_ident)) => {
            let log_dir = serve_config.log_dir.clone().ok_or_else(|| {
                anyhow!("log_dir is required when configuring with a playbook identity")
            })?;
            let input_dir = serve_config.input_dir.clone().ok_or_else(|| {
                anyhow!("input_dir is required when configuring with a playbook identity")
            })?;
            let output_dir = serve_config.output_dir.clone().ok_or_else(|| {
                anyhow!("output_dir is required when configuring with a playbook identity")
            })?;
            let pipeline_config = PipelineConfig {
                playbook: playbook_ident.clone(),
                remote: serve_config.remote.clone(),
                wal_capacity: serve_config.wal_capacity,
                success_log_retention_secs: serve_config.success_log_retention_secs,
                error_log_retention_secs: serve_config.error_log_retention_secs,
                log_dir,
                input_dir,
                output_dir,
            };
            pipeline_config
                .load(&cache)
                .await
                .context("Failed to load pipeline config built from playbook identity")?
        }
        Some(PlaybookServerConfig::Path(path)) => {
            let config_str = fs::read_to_string(path)?;
            let pipeline: PipelineConfig =
                match path.extension().map(|s| s.as_encoded_bytes()) {
                    Some(b"toml") => {
                        toml::from_str(&config_str).context("Failed to parse pipeline TOML")?
                    }
                    Some(b"yaml") => serde_yaml::from_str(&config_str)
                        .context("Failed to parse pipeline yaml")?,
                    Some(b"json") => serde_json::from_str(&config_str)
                        .context("Failed to parse pipeline JSON")?,
                    _ => {
                        anyhow::bail!(
                            "Unknown extension for playbook config, supports toml, yaml and json"
                        )
                    }
                };
            pipeline
                .load(&cache)
                .await
                .context("Failed to load playbook config file")?
        }
        None => {
            anyhow::bail!(
                "No playbook configuration provided. Must provide either --playbook-config, --playbook (with extra settings), or inline playbook in the configuration."
            );
        }
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
    let cap = serve_config
        .cap
        .as_ref()
        .ok_or_else(|| anyhow!("Capability identity is required to run a capability server"))?;

    tracing::info!("Loading capability '{}'", cap);
    let cache = CacheManager::from_env().await?;
    let cap_path = cache
        .capability_binary_path(&cap.author, &cap.package, &cap.version)
        .await
        .context("Failed to find capability binary path in CacheManager")?;

    let mut router =
        PyroRouter::load(cap.clone(), cap_path).context("Failed to load capability library")?;

    // Pre-configure capability classes if config provided
    if let Some(ref cap_config) = serve_config.cap_config {
        let capability_config = if let Ok(parsed) = serde_json::from_value::<
            pyro_artifacts::artifacts::CapabilityConfig,
        >(cap_config.clone())
        {
            parsed
        } else if let serde_json::Value::Object(obj) = cap_config {
            let classes = obj
                .clone()
                .into_iter()
                .map(|(class_name, class_config)| (class_name, Some(class_config)))
                .collect();
            pyro_artifacts::artifacts::CapabilityConfig { classes }
        } else {
            anyhow::bail!(
                "Capability configuration must be a JSON object mapping class names to their configurations"
            );
        };

        router
            .configure(&capability_config)
            .await
            .context("Failed to configure capability classes")?;
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
    playbook_ident: Option<pyro_artifacts::artifacts::PlaybookIdent>,
    remote: std::collections::HashMap<CapabilityIdent, pyro_artifacts::cache::RemoteAddress>,
    wal_capacity: Option<usize>,
    success_log_retention_secs: Option<u64>,
    error_log_retention_secs: Option<u64>,
    log_dir: Option<PathBuf>,
    input_dir: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    cap_name: Option<pyro_artifacts::cargo::CapabilityIdent>,
    cap_config: Option<&str>,
) -> Result<()> {
    let serve_config = resolve_config(
        config_path,
        config_json,
        server_type,
        socket,
        http,
        playbook_config,
        playbook_ident,
        remote,
        wal_capacity,
        success_log_retention_secs,
        error_log_retention_secs,
        log_dir,
        input_dir,
        output_dir,
        cap_name,
        cap_config,
    )?;

    match serve_config.server_type {
        ServerType::Playbook => serve_playbook(&serve_config).await,
        ServerType::Capability => serve_capability(&serve_config).await,
    }
}
