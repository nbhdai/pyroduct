use std::path::{Path, PathBuf};
use fs_err as fs;
use anyhow::Result;
use pyroduct::{ModIdentity, arrow_scalars::ArrowRow, host::{Capabilities, CapabilityDefinition, CompiledModule, HarnessConfig}};
use wasmtime::{Config, Engine};


pub async fn run(config_path: &Path, input_json: Option<&str>) -> Result<()> {
    tracing::info!("Loading config from {:?}", config_path);

    let config_str = fs::read_to_string(config_path)?;
    let config: HarnessConfig = toml::from_str(&config_str)?;
    let config_dir = config_path.parent().unwrap_or(Path::new("."));

    let module_path = if config.module.is_relative() {
        config_dir.join(&config.module)
    } else {
        config.module.clone()
    };

    tracing::info!("Loading WASM module from {:?}", module_path);
    let wasm_bytes = fs::read(&module_path)?;

    let cap_paths: Vec<PathBuf> = config
        .capabilities
        .iter()
        .map(|c| {
            let p = match c {
                CapabilityDefinition::NoConfig(p) => p,
                CapabilityDefinition::Config { path, .. } => path,
            };
            if p.is_relative() { config_dir.join(p) } else { p.clone() }
        })
        .collect();

    tracing::info!("Loading {} capabilities", cap_paths.len());
    let capabilities = Capabilities::load(cap_paths.iter().map(|p: &PathBuf| p.as_path()))?;

    let mut engine_config = Config::new();
    engine_config.async_support(true);
    let engine = Engine::new(&engine_config)?;

    let ident = ModIdentity::from(&config.module_name);

    tracing::info!("Compiling module '{}'", config.module_name);
    let mut compiled = CompiledModule::new(&ident, &engine, &wasm_bytes, &capabilities, &config).await?;

    let input_row: ArrowRow<'static> = if let Some(json) = input_json {
        serde_json::from_str(json)?
    } else {
        ArrowRow::default()
    };

    tracing::info!("Processing input...");
    let result = compiled.process(&input_row).await?;

    match result {
        Ok(output) => {
            tracing::info!("Module completed successfully");
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        Err(e) => {
            tracing::error!("Module returned error: {}", e);
            anyhow::bail!("Module error: {}", e);
        }
    }

    Ok(())
}