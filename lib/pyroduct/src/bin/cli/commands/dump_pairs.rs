use anyhow::{Context, Result, anyhow, bail};
use std::path::{Path, PathBuf};

use pyro_file::write_parquet;
use pyroduct::format::log_wal::LogWal;
use pyroduct::format::value::arrow::PreBatch;
use pyroduct::format::value::arrow::Rowable;
use pyroduct::format::value::arrow::wal::recover;
use pyroduct::format::value::{PyroRow, PyroSchema, PyroValue};
use pyroduct::pipeline::data::DataManager;
use pyroduct::pipeline::{ExecutionRecord, Pipeline};

/// Dumps every error-free (input, output) pair for a playbook run into a series of
/// Parquet files, keyed by the same global `row_index` used by the daemon so results
/// can be correlated back to the source data. Rows whose execution failed, or whose
/// index can't be resolved to a success, are skipped.
pub async fn dump_pairs(
    log_dir: &Path,
    input_dir: &Path,
    output_dir: &Path,
    wal_capacity: usize,
    dest_dir: &Path,
    rows_per_file: usize,
) -> Result<()> {
    if rows_per_file == 0 {
        bail!("rows-per-file must be greater than zero");
    }

    std::fs::create_dir_all(dest_dir)
        .with_context(|| format!("Failed to create destination directory {:?}", dest_dir))?;

    let input_schema = infer_dir_schema(input_dir)
        .await
        .context("Failed to infer input schema")?
        .with_context(|| format!("No input data found in {:?} — nothing to extract", input_dir))?;
    let output_schema = infer_dir_schema(output_dir)
        .await
        .context("Failed to infer output schema")?
        .with_context(|| {
            format!(
                "No output data found in {:?} — nothing to extract",
                output_dir
            )
        })?;

    let input_manager = DataManager::new(input_dir, input_schema, wal_capacity);
    input_manager
        .restore()
        .await
        .context("Failed to restore input DataManager state")?;
    let output_manager = DataManager::new(output_dir, output_schema, wal_capacity);
    output_manager
        .restore()
        .await
        .context("Failed to restore output DataManager state")?;
    let log_manager = LogWal::open(log_dir, wal_capacity)
        .await
        .with_context(|| format!("Failed to open log WAL at {:?}", log_dir))?;

    // We only ever call `get_record`, so the shard pool (which would require
    // instantiating the wasm module) is left empty on purpose.
    let pipeline = Pipeline {
        shards: Vec::new(),
        success_log_retention_secs: 0,
        error_log_retention_secs: 0,
        log_manager: tokio::sync::Mutex::new(log_manager),
        input_manager,
        output_manager,
        callbacks: tokio::sync::Mutex::new(Vec::new()),
    };

    let total = pipeline.input_manager.len().await;
    tracing::info!(total, "Scanning rows for error-free input/output pairs");

    let mut buffer: Vec<PyroRow<'static>> = Vec::with_capacity(rows_per_file.min(total.max(1)));
    let mut file_index = 0usize;
    let mut kept = 0usize;
    let mut skipped = 0usize;

    for row_index in 0..total {
        match pipeline.get_record(row_index).await {
            Ok(ExecutionRecord::Success {
                input, success, ..
            }) => {
                let mut pair = PyroRow::new();
                pair.insert("row_index".to_string(), PyroValue::U64(row_index as u64));
                pair.insert("input".to_string(), PyroValue::Group(input));
                pair.insert("output".to_string(), PyroValue::Group(success));
                buffer.push(pair);
                kept += 1;

                if buffer.len() >= rows_per_file {
                    flush_chunk(std::mem::take(&mut buffer), dest_dir, file_index).await?;
                    file_index += 1;
                }
            }
            Ok(ExecutionRecord::Failure { .. }) => {
                skipped += 1;
            }
            Err(pyroduct::PyroError::NotFound(_)) => {
                // Row was never fully recorded (e.g. logs pruned and no output ever
                // written) — treat as unresolved/failed rather than erroring out.
                skipped += 1;
            }
            Err(e) => {
                return Err(e).with_context(|| format!("Failed to read record {}", row_index));
            }
        }
    }

    if !buffer.is_empty() {
        flush_chunk(buffer, dest_dir, file_index).await?;
        file_index += 1;
    }

    tracing::info!(
        kept,
        skipped,
        files = file_index,
        "Finished dumping pairs"
    );
    println!(
        "Wrote {} error-free pair(s) across {} file(s) in {:?} ({} skipped)",
        kept, file_index, dest_dir, skipped
    );

    Ok(())
}

/// Writes one chunk of merged pairs to `dest_dir/pairs_<file_index>.parquet`.
async fn flush_chunk(rows: Vec<PyroRow<'static>>, dest_dir: &Path, file_index: usize) -> Result<()> {
    let dest_dir = dest_dir.to_path_buf();
    tokio::task::spawn_blocking(move || -> Result<()> {
        let schema = PyroRow::infer_schema(&rows)
            .map_err(|e| anyhow!("Failed to infer schema for pairs chunk: {:?}", e))?;
        let mut prebatch = PreBatch::new(schema);
        let row_count = rows.len();
        for row in rows {
            prebatch.push_unchecked(row);
        }

        let record_batch = prebatch
            .flush()
            .map_err(|e| anyhow!("Failed to flush pairs chunk to RecordBatch: {:?}", e))?
            .ok_or_else(|| anyhow!("Pairs chunk flush unexpectedly produced no RecordBatch"))?;

        let path = dest_dir.join(format!("pairs_{}.parquet", file_index));
        write_parquet(&[record_batch], &path)
            .with_context(|| format!("Failed to write parquet chunk to {:?}", path))?;

        tracing::info!(?path, rows = row_count, "Wrote pairs chunk");
        Ok(())
    })
    .await
    .context("Failed to join pairs-chunk writer thread")?
}

/// Infers a `PyroSchema` for a `DataManager` directory (input or output) by reading a
/// single representative row from whichever storage tier currently has data: the
/// active (not-yet-flushed) WAL, an Arrow IPC batch, or a Parquet rollout — in that
/// order, since only the active WAL requires an externally supplied schema to recover.
/// Returns `Ok(None)` if the directory has no data at all.
async fn infer_dir_schema(dir: &Path) -> Result<Option<PyroSchema<'static>>> {
    if !dir.exists() {
        return Ok(None);
    }

    let mut wal_files: Vec<PathBuf> = Vec::new();
    let mut ipc_files: Vec<PathBuf> = Vec::new();
    let mut parquet_files: Vec<PathBuf> = Vec::new();

    for entry in
        std::fs::read_dir(dir).with_context(|| format!("Failed to read directory {:?}", dir))?
    {
        let path = entry
            .with_context(|| format!("Failed to read directory entry in {:?}", dir))?
            .path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        if name.ends_with(".pyrowal") {
            wal_files.push(path);
        } else if name.starts_with("batch_") && name.ends_with(".arrow") {
            ipc_files.push(path);
        } else if name.starts_with("rollout_") && name.ends_with(".parquet") {
            parquet_files.push(path);
        }
    }

    // 1. Active WAL segment (at most one exists at a time).
    if let Some(wal_path) = wal_files.into_iter().next() {
        let base_path = wal_path.with_extension("");
        let rows = tokio::task::spawn_blocking(move || recover(&base_path))
            .await
            .context("Failed to join WAL recovery thread")?
            .context("Failed to recover rows from active WAL file")?;
        if let Some(row) = rows.into_iter().next() {
            return Ok(Some(
                row.schema()
                    .map_err(|e| anyhow!("Failed to infer schema from WAL row: {:?}", e))?,
            ));
        }
    }

    // 2. Arrow IPC batch files.
    ipc_files.sort();
    for path in ipc_files {
        if let Some(schema) = schema_from_batch_file(&path).await? {
            return Ok(Some(schema));
        }
    }

    // 3. Parquet rollout files.
    parquet_files.sort();
    for path in parquet_files {
        if let Some(schema) = schema_from_batch_file(&path).await? {
            return Ok(Some(schema));
        }
    }

    Ok(None)
}

/// Reads the first row out of an Arrow IPC or Parquet file and infers a schema from it.
async fn schema_from_batch_file(path: &Path) -> Result<Option<PyroSchema<'static>>> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || -> Result<Option<PyroSchema<'static>>> {
        let bytes =
            std::fs::read(&path).with_context(|| format!("Failed to read file {:?}", path))?;
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        let batches = pyro_file::parse_data_to_batch_sync(bytes, filename)
            .with_context(|| format!("Failed to parse batch file {:?}", path))?;

        for batch in &batches {
            if let Ok(row) = batch.row(0) {
                let row = row.into_owned();
                return Ok(Some(row.schema().map_err(|e| {
                    anyhow!("Failed to infer schema from batch row: {:?}", e)
                })?));
            }
        }
        Ok(None)
    })
    .await
    .context("Failed to join schema-inference thread")?
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyroduct::format::log_wal::LogEntry;
    use pyroduct::format::value::PyroValue;
    use tempfile::tempdir;

    fn row(id: i32, name: &str) -> PyroRow<'static> {
        PyroRow::from([
            ("id", PyroValue::from(id)),
            ("name", PyroValue::from(name.to_string())),
        ])
    }

    /// Mimics what `Pipeline::process` does for one row, without needing a real wasm
    /// shard: push the input unconditionally, then either push the output + a
    /// success log entry, or just a failure log entry.
    async fn record_row(
        input_manager: &DataManager,
        output_manager: &DataManager,
        log_manager: &mut LogWal,
        row_index: usize,
        input: PyroRow<'static>,
        outcome: Result<PyroRow<'static>, String>,
    ) -> Result<()> {
        input_manager.push_record(row_index, &input).await?;
        match outcome {
            Ok(output) => {
                output_manager.push_record(row_index, &output).await?;
                log_manager
                    .append(&LogEntry {
                        row_index,
                        module_logs: Vec::new(),
                        capability_logs: std::collections::HashMap::new(),
                        failure: None,
                    })
                    .await?;
            }
            Err(msg) => {
                log_manager
                    .append(&LogEntry {
                        row_index,
                        module_logs: Vec::new(),
                        capability_logs: std::collections::HashMap::new(),
                        failure: Some(Err(msg)),
                    })
                    .await?;
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_dump_pairs_skips_failures_and_preserves_index() {
        let log_dir = tempdir().unwrap();
        let input_dir = tempdir().unwrap();
        let output_dir = tempdir().unwrap();
        let dest_dir = tempdir().unwrap();

        let input_schema = row(0, "seed").schema().unwrap();
        let output_schema = row(0, "seed").schema().unwrap();
        let input_manager = DataManager::new(input_dir.path(), input_schema, 1000);
        let output_manager = DataManager::new(output_dir.path(), output_schema, 1000);
        let mut log_manager = LogWal::open(log_dir.path(), 1000).await.unwrap();

        // Row 0: success. Row 1: failure (no output). Row 2: success.
        record_row(
            &input_manager,
            &output_manager,
            &mut log_manager,
            0,
            row(1, "alice-in"),
            Ok(row(10, "alice-out")),
        )
        .await
        .unwrap();
        record_row(
            &input_manager,
            &output_manager,
            &mut log_manager,
            1,
            row(2, "bob-in"),
            Err("boom".to_string()),
        )
        .await
        .unwrap();
        record_row(
            &input_manager,
            &output_manager,
            &mut log_manager,
            2,
            row(3, "carol-in"),
            Ok(row(30, "carol-out")),
        )
        .await
        .unwrap();

        log_manager.flush().await.unwrap();
        input_manager.flush_wal().await.unwrap();
        output_manager.flush_wal().await.unwrap();

        dump_pairs(
            log_dir.path(),
            input_dir.path(),
            output_dir.path(),
            1000,
            dest_dir.path(),
            1000,
        )
        .await
        .unwrap();

        let parquet_path = dest_dir.path().join("pairs_0.parquet");
        assert!(parquet_path.exists());

        let bytes = std::fs::read(&parquet_path).unwrap();
        let batches =
            pyro_file::parse_data_to_batch_sync(bytes, "pairs_0.parquet").unwrap();
        assert_eq!(batches.len(), 1);

        // Only the 2 successful rows should be present, and each must carry its
        // original row_index (0 and 2 — row 1 was a failure and must be absent).
        assert_eq!(batches[0].num_rows(), 2);
        let r0 = batches[0].row(0).unwrap();
        let r1 = batches[0].row(1).unwrap();
        assert_eq!(r0.get("row_index"), Some(&PyroValue::U64(0)));
        assert_eq!(r1.get("row_index"), Some(&PyroValue::U64(2)));
    }
}
