use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use clap::ValueEnum;
use fs_err as fs;

use pyroduct::value::arrow::PreBatch;
use pyroduct::{
    value::PyroRow,
    pipeline::{Pipeline, PipelineConfig, PipelineDef, PipelinePool},
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
fn load_config(config_path: &Path) -> Result<PipelineConfig> {
    tracing::info!("Loading config from {:?}", config_path);
    let config_str = fs::read_to_string(config_path)?;
    let mut config: PipelineConfig =
        toml::from_str(&config_str).context("Failed to parse pipeline TOML")?;

    let config_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
    // Resolve relative paths
    for lib in config.capabilities.values_mut() {
        if lib.path.is_relative() {
            lib.path = config_dir.join(&lib.path);
        }
    }
    for mod_conf in config.modules.values_mut() {
        if mod_conf.path.is_relative() {
            mod_conf.path = config_dir.join(&mod_conf.path);
        }
    }
    Ok(config)
}

/// Processes a single row from a JSON string and prints the result to stdout.
pub async fn run(config_path: &Path, input_json: &str) -> Result<()> {
    // 1. Setup Pipeline (Single instance, no pool needed)
    let config = load_config(config_path)?;
    let pipeline_def = PipelineDef::load(&config).await?;
    let mut pipeline = Pipeline::new(pipeline_def).await?;

    // 2. Parse Input directly to PyroRow
    tracing::debug!("Parsing input JSON directly to PyroRow");
    let input_row: PyroRow<'static> =
        serde_json::from_str(input_json).context("Failed to deserialize input JSON to PyroRow")?;

    // 3. Execute
    tracing::info!("Executing pipeline...");
    let result_row = pipeline.process(input_row).await?;

    // 4. Print Result
    match result_row {
        Ok(row) => println!("{row:#?}"),
        Err(failure) => println!("{failure:#?}"),
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
    let pipeline_def = PipelineDef::load(&config).await?;

    let pipeline = Pipeline::new(pipeline_def).await?;
    let pool = PipelinePool::new(vec![pipeline]);

    tracing::info!("Reading input file: {:?}", input_file);
    let filename = input_file.file_name().unwrap_or_default().to_string_lossy();
    let bytes = fs::read(input_file).context("Failed to read input file")?;

    let input_batch = parse_data_to_batch(bytes, &filename).await?;

    tracing::info!("Processing {} rows...", input_batch[0].num_rows());
    let (successes, failures) = pool
        .process_batch(&input_batch[0].clone().to_batch())
        .await?;

    if !failures.is_empty() {
        if !output_dir.exists() {
            fs::create_dir_all(output_dir)?;
        }

        let error_path = output_dir.join("errors.jsonl");
        tracing::warn!("Writing {} failures to {:?}", failures.len(), error_path);

        let f = fs::File::create(&error_path)?;
        let mut writer = BufWriter::new(f);

        for fail in failures {
            let entry = serde_json::json!({
                "row_index": fail.row_index,
                "error": fail.error,
                "partial_data": fail.partial_data
            });
            serde_json::to_writer(&mut writer, &entry)?;
            writeln!(writer)?;
        }
    }

    if !successes.is_empty() {
        if !output_dir.exists() {
            fs::create_dir_all(output_dir)?;
        }
        let schema = pyroduct::value::PyroSchema::trusted(&successes[0])?;
        let mut prebatch = PreBatch::new(schema);
        for row in successes {
            prebatch
                .push(row)
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