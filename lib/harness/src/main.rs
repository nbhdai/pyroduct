use anyhow::{Context, Result};
use arrow_scalars::ArrowRow;
use pyroduct::host::{CompiledModule};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(serde::Deserialize, Debug)]
struct HarnessConfig {
    module_name: String,
    /// Path to the WASM module to run
    module: PathBuf,
    /// List of paths to dynamic library capabilities (.so/.dylib/.dll)
    capabilities: Vec<CapabilityConfig>,
    /// List of inputs to process
    inputs: Vec<InputConfig>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(untagged)]
pub enum CapabilityConfig {
    NoConfig(PathBuf),
    Config {
        path: PathBuf,
        config: serde_json::Value,
    },
}

#[derive(serde::Deserialize, Debug)]
struct InputConfig {
    /// Input data as JSON - will be deserialized into ArrowRow
    input: serde_json::Value,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true))
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            "info,harness=debug,pyroduct=debug,cranelift_codegen=off,cranelift_wasm=off,wasmtime_cranelift=off,wasmtime_internal_cranelift=off,wasmtime=off".into()
        }))
        .init();

    info!("Starting Harness...");

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <config_file.json>", args[0]);
        std::process::exit(1);
    }
    let config_path = &args[1];

    // Load and parse configuration
    info!("Loading config from: {}", config_path);
    let config_content = fs::read_to_string(config_path).context("Failed to read config file")?;
    let config: HarnessConfig =
        serde_json::from_str(&config_content).context("Failed to parse config JSON")?;

    info!("Configuration loaded successfully");
    info!("Module: {:?}", config.module);
    info!("Module name: {}", config.module_name);
    info!("Capabilities: {} loaded", config.capabilities.len());
    info!("Inputs: {} to process", config.inputs.len());

    // Load dynamic capabilities
    let mut loaded_caps: Vec<Arc<dyn pyroduct::host::Capability>> = Vec::new();
    let mut loaded_configs: Vec<Option<serde_json::Value>> = Vec::new();

    for cap_config in &config.capabilities {
        let (path, cap_json) = match cap_config {
            CapabilityConfig::NoConfig(path) => (path, None),
            CapabilityConfig::Config { path, config } => (path, Some(config.clone())),
        };

        info!(" - Loading Capability: {:?}", path);
        let cap = unsafe {
            DynamicCapability::load(path)
                .with_context(|| format!("Failed to load plugin at {:?}", path))?
        };
        loaded_caps.push(Arc::new(cap));
        loaded_configs.push(cap_json);
    }

    info!("All capabilities loaded successfully");

    // Load WASM module bytes
    info!("Loading WASM module from: {:?}", config.module);
    let wasm_bytes = fs::read(&config.module)
        .with_context(|| format!("Failed to read WASM module at {:?}", config.module))?;

    info!("WASM module loaded, size: {} bytes", wasm_bytes.len());

    let mut engine_config = wasmtime::Config::new();
    engine_config.async_support(true);
    let engine = wasmtime::Engine::new(&engine_config)?;

    // Create the WASM module with capabilities
    info!("Initializing WASM module with capabilities...");
    let mut wasm_module = CompiledModule::new(
        &config.module_name,
        &config.module,
        &engine,
        &wasm_bytes,
        loaded_caps,
        loaded_configs.iter().map(|c| c.as_ref()).collect(),
    )
    .await
    .context("Failed to create WASM module")?;

    info!("WASM module initialized successfully");

    // Process each input
    for (idx, input_config) in config.inputs.iter().enumerate() {
        info!("===== Processing input {} =====", idx + 1);

        // Deserialize JSON into ArrowRow
        let arrow_row: ArrowRow = serde_json::from_value(input_config.input.clone())
            .with_context(|| format!("Failed to deserialize input {} into ArrowRow", idx + 1))?;

        info!("Input deserialized: {:?}", arrow_row);

        // Process through WASM module
        info!("Calling WASM module...");
        match wasm_module.process(&arrow_row).await {
            Ok(result) => {
                match result {
                    Ok(output) => {
                        info!("✓ Success! Output: {:?}", output);

                        // Pretty print the output as JSON
                        match serde_json::to_string_pretty(&output) {
                            Ok(json) => println!("{}", json),
                            Err(e) => error!("Failed to serialize output to JSON: {}", e),
                        }
                    }
                    Err(e) => {
                        error!("✗ WASM module returned error: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("✗ Failed to process input: {:?}", e);
            }
        }

        println!(); // Empty line between outputs
    }

    info!("All inputs processed. Harness complete.");
    Ok(())
}
