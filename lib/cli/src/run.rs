use std::collections::BTreeMap;
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use artifacts::{
    cache::CacheManager,
    cargo::{CapabilityManifest, ModuleManifest, ResolvedCapability},
    environment::dylib_extension,
};
use clap::ValueEnum;
use fs_err as fs;

use pyroduct::{
    PyroRow,
    format::value::arrow::PreBatch,
    module::{PyroFactory, PyroModule, capability::CapabilityLibrary},
    pipeline::{Pipeline, PipelineConfig, PipelinePool},
};

use arrow_file::{
    parse_data_to_batch, record_batch_to_bytes, write_csv, write_jsonl, write_parquet,
};

#[derive(ValueEnum, Clone, Debug, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Csv,
    Ipc,
    Parquet,
}

impl OutputFormat {
    fn extension(&self) -> &'static str {
        match self {
            OutputFormat::Json => "jsonl",
            OutputFormat::Csv => "csv",
            OutputFormat::Ipc => "arrow",
            OutputFormat::Parquet => "parquet",
        }
    }
}

/// Helper to load config and resolve paths
pub fn load_config(config_path: &Path) -> Result<PipelineConfig> {
    tracing::info!("Loading config from {:?}", config_path);
    let config_str = fs::read_to_string(config_path)?;
    let mut config: PipelineConfig = match config_path.extension().map(|s| s.as_encoded_bytes()) {
        Some(b"toml") => toml::from_str(&config_str).context("Failed to parse pipeline TOML")?,
        Some(b"yaml") => {
            serde_yaml::from_str(&config_str).context("Failed to parse pipeline yaml")?
        }
        Some(b"json") => {
            serde_json::from_str(&config_str).context("Failed to parse pipeline JSON")?
        }
        _ => anyhow::bail!("Unknown extension, supports toml, yaml and json"),
    };

    let config_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
    for module in config.pipeline.values_mut() {
        for path in module.libraries.iter_mut() {
            if path.is_relative() {
                *path = config_dir.join(&path);
            }
        }
        if module.path.is_relative() {
            module.path = config_dir.join(&module.path);
        }
    }

    Ok(config)
}

/// Build a `Pipeline` by compiling each module via the cache and loading
/// capability libraries from the cache.
async fn build_pipeline_from_cache(
    config: &PipelineConfig,
    cache: &CacheManager,
) -> Result<Pipeline> {
    let mut wasmtime_cfg = wasmtime::Config::new();
    wasmtime_cfg.async_support(true);
    let engine = wasmtime::Engine::new(&wasmtime_cfg)
        .map_err(|e| anyhow!("Failed to create wasmtime engine: {}", e))?;

    let lib_file = format!("lib.{}", dylib_extension());
    let mut steps = Vec::new();

    for (name, mod_conf) in &config.pipeline {
        // 1. Read source code
        let src_path = mod_conf.path.join("src/lib.rs");
        let source_code = fs::read_to_string(&src_path)
            .with_context(|| format!("Failed to read source for module '{}'", name))?;

        // 2. Read dependencies from Module.toml
        let mut dependencies = BTreeMap::new();
        let mod_toml_path = mod_conf.path.join("Module.toml");
        if mod_toml_path.exists() {
            let toml_content = fs::read_to_string(&mod_toml_path)?;
            let manifest: ModuleManifest = toml::from_str(&toml_content)
                .with_context(|| format!("Failed to parse Module.toml for '{}'", name))?;
            dependencies = manifest.dependencies;
        }

        // 3. Resolve capabilities from Capability.toml files
        let mut capabilities: Vec<ResolvedCapability> = Vec::new();
        for lib_path in &mod_conf.libraries {
            let cap_toml_path = lib_path.join("Capability.toml");
            if cap_toml_path.exists() {
                let toml_content = fs::read_to_string(&cap_toml_path)?;
                let manifest: CapabilityManifest = toml::from_str(&toml_content)
                    .with_context(|| {
                        format!("Failed to parse Capability.toml at {:?}", cap_toml_path)
                    })?;
                capabilities.push(ResolvedCapability {
                    author: manifest.capability.author.clone(),
                    package: manifest.capability.name.clone(),
                    version: manifest.capability.version.clone(),
                });
            }
        }

        // 4. Compile via cache
        tracing::info!("Compiling module '{}' via cache...", name);
        let artifact = cache
            .compile_anon(dependencies, capabilities.clone(), &source_code)
            .await
            .with_context(|| format!("Compilation failed for module '{}'", name))?;

        // 5. Load capability libraries from cache
        let mut libs = Vec::new();
        for cap in &capabilities {
            let cap_dir = cache.capabilities_dir(&cap.author, &cap.package, &cap.version);
            let artifact_path = cap_dir.join(&lib_file);
            let library = CapabilityLibrary::load(cap.package.clone(), &artifact_path)
                .await
                .with_context(|| {
                    format!(
                        "Failed to load capability library from cache: {}",
                        artifact_path.display()
                    )
                })?;
            libs.push(library);
        }

        // 6. Instantiate
        let wasmtime_module = wasmtime::Module::from_binary(&engine, &artifact.wasm)
            .map_err(|e| anyhow!("Failed to compile WASM for '{}': {}", name, e))?;
        let pyro_module = PyroModule::new(wasmtime_module)?;

        let mut factory =
            PyroFactory::new(libs, mod_conf.configurations.clone(), pyro_module)
                .map_err(|e| anyhow!("Failed to create PyroFactory for '{}': {}", name, e))?;

        let instance = factory
            .instantiate()
            .await
            .map_err(|e| anyhow!("Failed to instantiate module '{}': {}", name, e))?;

        steps.push(instance);
    }

    Ok(Pipeline { steps })
}

/// Processes a single row from a JSON string and prints the result to stdout.
pub async fn run(config_path: &Path, input_json: &str) -> Result<()> {
    let config = load_config(config_path)?;
    let cache = CacheManager::new().await?;
    let mut pipeline = build_pipeline_from_cache(&config, &cache).await?;

    tracing::debug!("Parsing input JSON directly to PyroRow");
    let input_row: PyroRow<'static> =
        serde_json::from_str(input_json).context("Failed to deserialize input JSON to PyroRow")?;

    tracing::info!("Executing pipeline...");
    let result_row = pipeline.process(&input_row).await;

    if let Some(failure) = &result_row.failure {
        println!("Pipeline Failed!");
        match &failure.result {
            Ok(err) => println!("Error: {}", err),
            Err(err) => println!("Error: {}", err),
        }
        if let Some(last_success) = result_row.steps.last() {
            println!("Partial Data:\n{:#?}", last_success.row);
        }
    } else if let Some(last_success) = result_row.steps.last() {
        println!("Pipeline Succeeded!");
        println!("Result:\n{:#?}", last_success.row);
    }

    let mut has_logs = false;
    for step in &result_row.steps {
        if !step.logs.module_logs.is_empty() || !step.logs.capability_logs.is_empty() {
            has_logs = true;
            break;
        }
    }
    if let Some(fail) = &result_row.failure {
        if !fail.logs.module_logs.is_empty() || !fail.logs.capability_logs.is_empty() {
            has_logs = true;
        }
    }

    if has_logs {
        println!("\n=== Logs ===");
        let print_step_logs = |step_idx: usize, logs: &pyroduct::module::PyroLogs| {
            if logs.module_logs.is_empty() && logs.capability_logs.is_empty() {
                return;
            }
            println!("Step {}:", step_idx);
            if !logs.module_logs.is_empty() {
                println!("  Module:");
                for log in &logs.module_logs {
                    println!("    {}", log);
                }
            }
            if !logs.capability_logs.is_empty() {
                println!("  Capabilities:");
                for ((lib, cap), cap_logs) in &logs.capability_logs {
                    println!("    [{lib}::{cap}]");
                    for log in cap_logs {
                        println!("      {}", log);
                    }
                }
            }
        };

        for (i, step) in result_row.steps.iter().enumerate() {
            print_step_logs(i, &step.logs);
        }
        if let Some(failure) = &result_row.failure {
            print_step_logs(result_row.steps.len(), &failure.logs);
        }
    }

    Ok(())
}

/// Processes a file of data using a thread pool and batch semantics.
pub async fn run_batch(
    config_path: &Path,
    input_file: &Path,
    output_dir: &Path,
    format: OutputFormat,
) -> Result<()> {
    let config = load_config(config_path)?;
    let cache = CacheManager::new().await?;
    let pipeline = build_pipeline_from_cache(&config, &cache).await?;
    let pool = PipelinePool::new(vec![pipeline]);

    tracing::info!("Reading input file: {:?}", input_file);
    let filename = input_file.file_name().unwrap_or_default().to_string_lossy();
    let bytes = fs::read(input_file).context("Failed to read input file")?;

    let input_batch = parse_data_to_batch(bytes, &filename).await?;

    tracing::info!("Processing {} rows...", input_batch[0].num_rows());

    let (successes, failures) = pool
        .process_batch(&input_batch[0].clone().to_batch())
        .await?;

    for exec in successes.iter().chain(failures.iter()) {
        let mut all_module_logs = Vec::new();
        let mut all_cap_logs: std::collections::HashMap<(String, String), Vec<String>> =
            std::collections::HashMap::new();

        for step in &exec.steps {
            all_module_logs.extend(step.logs.module_logs.iter().cloned());
            for (k, v) in &step.logs.capability_logs {
                all_cap_logs
                    .entry(k.clone())
                    .or_default()
                    .extend(v.iter().cloned());
            }
        }

        if let Some(fail) = &exec.failure {
            all_module_logs.extend(fail.logs.module_logs.iter().cloned());
            for (k, v) in &fail.logs.capability_logs {
                all_cap_logs
                    .entry(k.clone())
                    .or_default()
                    .extend(v.iter().cloned());
            }
        }

        if !all_module_logs.is_empty() || !all_cap_logs.is_empty() {
            let logs_dir = output_dir
                .join("logs")
                .join(format!("row_{}", exec.row_index));
            fs::create_dir_all(&logs_dir)?;

            if !all_module_logs.is_empty() {
                fs::write(logs_dir.join("module.log"), all_module_logs.join("\n"))?;
            }

            for ((lib, cap), logs) in all_cap_logs {
                if !logs.is_empty() {
                    fs::write(
                        logs_dir.join(format!("{}_{}.log", lib, cap)),
                        logs.join("\n"),
                    )?;
                }
            }
        }
    }

    if !failures.is_empty() {
        if !output_dir.exists() {
            fs::create_dir_all(output_dir)?;
        }

        let error_path = output_dir.join("errors.jsonl");
        tracing::warn!("Writing {} failures to {:?}", failures.len(), error_path);

        let f = fs::File::create(&error_path)?;
        let mut writer = BufWriter::new(f);

        for fail in failures {
            let error_msg = match &fail.failure.as_ref().unwrap().result {
                Ok(cap_err) => cap_err.to_string(),
                Err(wasm_err) => wasm_err.to_string(),
            };
            let partial_data = fail.steps.last().map(|s| s.row.clone()).unwrap_or_default();

            let entry = serde_json::json!({
                "row_index": fail.row_index,
                "error": error_msg,
                "partial_data": partial_data
            });
            serde_json::to_writer(&mut writer, &entry)?;
            writeln!(writer)?;
        }
    }

    if !successes.is_empty() {
        if !output_dir.exists() {
            fs::create_dir_all(output_dir)?;
        }
        let schema = successes[0].steps.last().unwrap().row.schema()?;
        let mut prebatch = PreBatch::new(schema);
        for row in successes {
            prebatch
                .push(row.steps.last().unwrap().row.clone())
                .map_err(|e| anyhow!("Row reconstruction failed: {:?}", e))?;
        }

        let output_batch = prebatch
            .flush()
            .map_err(|e| anyhow!("Batch flush failed: {:?}", e))?
            .ok_or_else(|| anyhow!("Resulting batch was empty"))?;
        let out_path = output_dir.join(format!("success.{}", format.extension()));
        tracing::info!(
            "Writing {} successful rows to {:?}",
            output_batch.num_rows(),
            out_path
        );

        match format {
            OutputFormat::Parquet => write_parquet(&[output_batch], out_path)?,
            OutputFormat::Csv => {
                write_csv(&[output_batch], out_path, None)?;
            }
            OutputFormat::Json => {
                write_jsonl(&[output_batch], out_path, None)?;
            }
            OutputFormat::Ipc => {
                let bytes = record_batch_to_bytes(&output_batch)?;
                fs::write(out_path, bytes)?;
            }
        }
    } else {
        tracing::warn!("No successful rows produced.");
    }

    Ok(())
}