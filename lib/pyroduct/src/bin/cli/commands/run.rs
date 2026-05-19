use std::io::{BufWriter, Write};
use std::path::Path;
use tokio::io::AsyncReadExt;
use tokio::net::{UnixListener, TcpListener};

use anyhow::{Context, Result, anyhow};
use clap::ValueEnum;
use fs_err as fs;

use pyro_artifacts::cache::CacheManager;
use pyroduct::pipeline::pipeline::LoadedPipelineConfig;
use pyroduct::{
    PyroRow,
    format::{value::arrow::PreBatch},
    pipeline::{Pipeline, PipelineConfig, PipelinePool, ExecutionRecord},
};

use pyro_file::{
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
async fn load_config(config_path: &Path) -> Result<LoadedPipelineConfig> {
    tracing::info!("Loading config from {:?}", config_path);
    let config_str = fs::read_to_string(config_path)?;
    let pipeline: PipelineConfig = match config_path.extension().map(|s| s.as_encoded_bytes()) {
        Some(b"toml") => toml::from_str(&config_str).context("Failed to parse pipeline TOML")?,
        Some(b"yaml") => {
            serde_yaml::from_str(&config_str).context("Failed to parse pipeline yaml")?
        }
        Some(b"json") => {
            serde_json::from_str(&config_str).context("Failed to parse pipeline JSON")?
        }
        _ => anyhow::bail!("Unknown extension, supports toml, yaml and json"),
    };
    let cache = CacheManager::from_env().await?;
    Ok(pipeline.load(&cache).await?)
}

/// Processes a single row from a JSON string and prints the result to stdout.
pub async fn run(config_path: &Path, input_json: &str) -> Result<()> {
    let loaded = load_config(config_path).await?;
    let factory = loaded.factory()?;
    let mut pipeline = factory.build().await?;

    process_and_print(&mut pipeline, input_json).await
}

pub async fn run_socket(config_path: &Path, socket_addr: &str) -> Result<()> {
    let loaded = load_config(config_path).await?;
    let factory = loaded.factory()?;
    let mut pipeline = factory.build().await?;

    if let Ok(addr) = socket_addr.parse::<std::net::SocketAddr>() {
        let listener = TcpListener::bind(addr).await.context("Failed to bind to TCP socket")?;
        tracing::info!("Listening on TCP socket {}", addr);

        loop {
            let (mut socket, _) = listener.accept().await?;
            let mut buffer = String::new();
            if let Err(e) = socket.read_to_string(&mut buffer).await {
                tracing::error!("Failed to read from socket: {}", e);
                continue;
            }

            if buffer.is_empty() {
                continue;
            }

            if let Err(e) = process_and_print(&mut pipeline, &buffer).await {
                tracing::error!("Failed to process input from socket: {}", e);
            }
        }
    } else {
        let socket_path = Path::new(socket_addr);
        if socket_path.exists() {
            fs::remove_file(socket_path).context("Failed to remove existing socket file")?;
        }

        let listener = UnixListener::bind(socket_path).context("Failed to bind to Unix socket")?;
        tracing::info!("Listening on Unix socket {:?}", socket_path);

        loop {
            let (mut socket, _) = listener.accept().await?;
            let mut buffer = String::new();
            if let Err(e) = socket.read_to_string(&mut buffer).await {
                tracing::error!("Failed to read from socket: {}", e);
                continue;
            }

            if buffer.is_empty() {
                continue;
            }

            if let Err(e) = process_and_print(&mut pipeline, &buffer).await {
                tracing::error!("Failed to process input from socket: {}", e);
            }
        }
    }
}

async fn process_and_print(pipeline: &mut Pipeline, input_json: &str) -> Result<()> {
    tracing::debug!("Parsing input JSON directly to PyroRow");
    let input_row: PyroRow<'static> =
        serde_json::from_str(input_json).context("Failed to deserialize input JSON to PyroRow")?;

    tracing::info!("Executing pipeline...");
    let result_row = pipeline.process(0, &input_row).await?;

    match &result_row {
        ExecutionRecord::Failure { failure, input, .. } => {
            println!("Pipeline Failed!");
            match failure {
                Ok(err) => println!("Error: {:?}", err),
                Err(err) => println!("Error: {}", err),
            }
            println!("Partial Data:\n{:#?}", input);
        }
        ExecutionRecord::Success { success, .. } => {
            println!("Pipeline Succeeded!");
            println!("Result:\n{:#?}", success);
        }
    }

    let logs = match &result_row {
        ExecutionRecord::Success { logs, .. } => logs,
        ExecutionRecord::Failure { logs, .. } => logs,
    };

    let has_logs = !logs.module_logs.is_empty() || !logs.capability_logs.is_empty();

    if has_logs {
        println!("\n=== Logs ===");
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
    let config = load_config(config_path).await?;
    let factory = config.factory()?;
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
        let logs = match exec {
            ExecutionRecord::Success { logs, .. } => logs,
            ExecutionRecord::Failure { logs, .. } => logs,
        };

        if !logs.module_logs.is_empty() || !logs.capability_logs.is_empty() {
            let logs_dir = output_dir
                .join("logs")
                .join(format!("row_{}", exec.row_index()));
            fs::create_dir_all(&logs_dir)?;

            if !logs.module_logs.is_empty() {
                fs::write(logs_dir.join("module.log"), logs.module_logs.join("\n"))?;
            }

            for ((lib, cap), cap_logs) in &logs.capability_logs {
                if !cap_logs.is_empty() {
                    fs::write(
                        logs_dir.join(format!("{}_{}.log", lib, cap)),
                        cap_logs.join("\n"),
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
            let (error_msg, partial_data) = match &fail {
                ExecutionRecord::Failure { failure, input, .. } => {
                    let msg = match &failure {
                        Ok(cap_err) => cap_err.to_string(),
                        Err(wasm_err) => wasm_err.to_string(),
                    };
                    (msg, input.clone())
                }
                _ => unreachable!(),
            };

            let entry = serde_json::json!({
                "row_index": fail.row_index(),
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
        let schema = match &successes[0] {
            ExecutionRecord::Success { success, .. } => success.schema()?,
            _ => unreachable!(),
        };
        let mut prebatch = PreBatch::new(schema);
        for row in successes {
            if let ExecutionRecord::Success { success, .. } = row {
                prebatch
                    .push(success)
                    .map_err(|e| anyhow!("Row reconstruction failed: {:?}", e))?;
            }
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
