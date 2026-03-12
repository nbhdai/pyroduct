use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use clap::ValueEnum;
use fs_err as fs;

use pyroduct::{
    PyroRow,
    format::value::arrow::PreBatch,
    pipeline::{PipelineConfig, PipelineFactory, PipelinePool},
};

// Use arrow-file to handle reading/writing data formats
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
            serde_json::from_str(&config_str).context("Failed to parse pipeline TOML")?
        }
        _ => anyhow::bail!("Unknown extension, supports toml, yaml and json"),
    };

    let config_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
    // Resolve relative paths
    for module in config.pipeline.iter_mut() {
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

/// Processes a single row from a JSON string and prints the result to stdout.
pub async fn run(config_path: &Path, input_json: &str) -> Result<()> {
    // 1. Setup Pipeline (Single instance, no pool needed)
    let config = load_config(config_path)?;
    let mut factory = PipelineFactory::load(&config).await?;
    let mut pipeline = factory.build().await?;

    // 2. Parse Input directly to PyroRow
    tracing::debug!("Parsing input JSON directly to PyroRow");
    let input_row: PyroRow<'static> =
        serde_json::from_str(input_json).context("Failed to deserialize input JSON to PyroRow")?;

    // 3. Execute
    tracing::info!("Executing pipeline...");
    let result_row = pipeline.process(&input_row).await;

    // 4. Print Result
    // 4. Print Result
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
    let mut factory = PipelineFactory::load(&config).await?;

    let pipeline = factory.build().await?;
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
