use anyhow::{Context, Result, anyhow, bail};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use pyro_file::write_parquet;
use pyroduct::format::log_wal::LogWalReader;
use pyroduct::format::value::arrow::PreBatch;
use pyroduct::format::value::arrow::Rowable;
use pyroduct::format::value::arrow::wal::recover;
use pyroduct::format::value::{PyroRow, PyroValue};

/// Recovers every error-free (input, output) pair for a playbook run into a series of
/// Parquet files.
///
/// This deliberately does NOT use `DataManager`'s O(1) index lookups: if the pipeline
/// was restarted mid-run (a daemon restart, a crash, etc.) without restoring prior
/// state, the row_index counter resets to 0 and subsequent rows silently overwrite the
/// SQLite index and colliding WAL/IPC segment files for the old low indices — that
/// bookkeeping can no longer be trusted, and the input/output rows themselves carry no
/// index of their own (no `set_metadata_prefix` was used for this run).
///
/// What *is* trustworthy is `log_dir`: `LogWal::open` scans and correctly continues
/// from existing `.pyrolog` files across restarts (unlike `DataManager`), and every
/// `LogEntry` embeds its own `row_index` — written unconditionally, once per row,
/// exactly once per `Pipeline::process` call, in lockstep with the input row and (on
/// success only) the output row. So we recover ordering from the log instead:
///
/// 1. Read `input_dir`, `output_dir`, and `log_dir` each in physical write order
///    (oldest mtime first for the data dirs; the log's own file-index order, which is
///    reliable).
/// 2. Zip *every* log entry positionally with the input rows (both happen once per
///    row, always) to recover each input row's true row_index and success/failure.
/// 3. Zip the *success-only* log entries positionally with the output rows (output is
///    only ever pushed on success, in that same relative order) to recover each
///    output row's true row_index.
/// 4. Join input and output on that recovered row_index.
///
/// Whenever the log's row_index goes backwards instead of increasing, that means a
/// restart happened and the counter rolled back over — we bump a "generation" counter
/// (recorded as a column on every row) so a rollover never gets silently confused
/// with genuine index reuse, even though all recovered rows land in one output file.
pub async fn dump_pairs(
    log_dir: &Path,
    input_dir: &Path,
    output_dir: &Path,
    output_path: &Path,
) -> Result<()> {
    if let Some(parent) = output_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create destination directory {:?}", parent))?;
    }

    tracing::info!(?log_dir, "Reading log entries in physical write order");
    let log_entries = read_log_entries_in_order(log_dir)
        .await
        .with_context(|| format!("Failed to read log entries from {:?}", log_dir))?;
    tracing::info!(?input_dir, "Reading input rows in physical write order");
    let input_rows = read_dir_rows_in_order(input_dir)
        .await
        .with_context(|| format!("Failed to read input rows from {:?}", input_dir))?;
    tracing::info!(?output_dir, "Reading output rows in physical write order");
    let output_rows = read_dir_rows_in_order(output_dir)
        .await
        .with_context(|| format!("Failed to read output rows from {:?}", output_dir))?;

    if log_entries.is_empty() {
        bail!(
            "No log entries found in {:?} — nothing to recover from",
            log_dir
        );
    }
    if input_rows.len() != log_entries.len() {
        tracing::warn!(
            input_rows = input_rows.len(),
            log_entries = log_entries.len(),
            "Input row count doesn't match log entry count — pairing only as far as the shorter of the two"
        );
    }

    // Tag every log entry with its generation (bumped whenever row_index rolls back).
    let mut generation = 0usize;
    let mut last_row_index: Option<usize> = None;
    let tagged_log: Vec<(usize, usize, bool)> = log_entries
        .iter()
        .map(|entry| {
            if let Some(last) = last_row_index
                && entry.row_index <= last
            {
                generation += 1;
            }
            last_row_index = Some(entry.row_index);
            (generation, entry.row_index, entry.failure.is_none())
        })
        .collect();

    let success_log: Vec<(usize, usize)> = tagged_log
        .iter()
        .filter(|(_, _, success)| *success)
        .map(|(generation, idx, _)| (*generation, *idx))
        .collect();
    if output_rows.len() != success_log.len() {
        tracing::warn!(
            output_rows = output_rows.len(),
            successful_log_entries = success_log.len(),
            "Output row count doesn't match successful log entry count — pairing only as far as the shorter of the two"
        );
    }

    let mut output_map: HashMap<(usize, usize), PyroRow<'static>> = HashMap::new();
    for ((generation, local_index), row) in success_log.into_iter().zip(output_rows) {
        output_map.insert((generation, local_index), row);
    }

    let mut buffer: Vec<PyroRow<'static>> = Vec::new();
    let mut kept = 0usize;
    let mut skipped = 0usize;
    let mut generations_seen = 0usize;
    let mut current_generation: Option<usize> = None;
    let mut true_index = 0usize;

    for ((generation, local_index, _), input_row) in tagged_log.into_iter().zip(input_rows) {
        if current_generation != Some(generation) {
            if current_generation.is_some() {
                tracing::warn!(
                    previous_generation = current_generation.unwrap(),
                    new_generation = generation,
                    true_index,
                    "Index rollover detected (playbook was likely restarted) — recovered rows still land in one file, tagged with a generation column"
                );
            }
            current_generation = Some(generation);
            generations_seen += 1;
        }

        if let Some(output_row) = output_map.remove(&(generation, local_index)) {
            let mut pair = PyroRow::new();
            pair.insert("row_index".to_string(), PyroValue::U64(true_index as u64));
            pair.insert("generation".to_string(), PyroValue::U64(generation as u64));
            pair.insert("local_index".to_string(), PyroValue::U64(local_index as u64));
            pair.insert("input".to_string(), PyroValue::Group(input_row));
            pair.insert("output".to_string(), PyroValue::Group(output_row));
            buffer.push(pair);
            kept += 1;
        } else {
            skipped += 1;
        }

        true_index += 1;
    }

    if buffer.is_empty() {
        bail!("No error-free pairs recovered — nothing to write");
    }
    write_pairs(buffer, output_path).await?;

    tracing::info!(kept, skipped, generations_seen, "Finished recovering pairs");
    println!(
        "Wrote {} error-free pair(s) to {:?} ({} skipped, {} generation(s)/rollover(s) detected)",
        kept, output_path, skipped, generations_seen
    );

    Ok(())
}

/// Reads every `LogEntry` out of `log_dir` in physical append order. `LogWal`'s file
/// naming/continuation is reliable across restarts (unlike `DataManager`'s), so a
/// plain sequential read (not the O(1) indexed `get`) is all that's needed here.
async fn read_log_entries_in_order(
    log_dir: &Path,
) -> Result<Vec<pyroduct::format::log_wal::LogEntry>> {
    if !log_dir.exists() {
        return Ok(Vec::new());
    }
    let mut reader = LogWalReader::open(log_dir)
        .await
        .with_context(|| format!("Failed to open log WAL reader at {:?}", log_dir))?;
    reader
        .read_all()
        .await
        .with_context(|| format!("Failed to read log entries from {:?}", log_dir))
}

/// Reads every row out of `dir` (Parquet rollouts, Arrow IPC batches, and the active
/// WAL segment, if any) in oldest-mtime-first order — i.e. the physical order they
/// were actually written in, regardless of what their (possibly reused/colliding)
/// on-disk file names are.
async fn read_dir_rows_in_order(dir: &Path) -> Result<Vec<PyroRow<'static>>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut files: Vec<(PathBuf, SystemTime)> = Vec::new();
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

        let is_data_file = name.ends_with(".pyrowal")
            || (name.starts_with("batch_") && name.ends_with(".arrow"))
            || (name.starts_with("rollout_") && name.ends_with(".parquet"));
        if !is_data_file {
            continue;
        }

        let mtime = std::fs::metadata(&path)
            .with_context(|| format!("Failed to stat {:?}", path))?
            .modified()
            .with_context(|| format!("Failed to read mtime of {:?}", path))?;
        files.push((path, mtime));
    }
    files.sort_by_key(|(_, mtime)| *mtime);
    let paths: Vec<PathBuf> = files.into_iter().map(|(p, _)| p).collect();

    tokio::task::spawn_blocking(move || -> Result<Vec<PyroRow<'static>>> {
        let mut rows = Vec::new();
        for path in paths {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();

            if name.ends_with(".pyrowal") {
                let base_path = path.with_extension("");
                let wal_rows = recover(&base_path)
                    .with_context(|| format!("Failed to recover active WAL file {:?}", path))?;
                rows.extend(wal_rows);
            } else {
                let bytes = std::fs::read(&path)
                    .with_context(|| format!("Failed to read file {:?}", path))?;
                let batches = pyro_file::parse_data_to_batch_sync(bytes, &name)
                    .with_context(|| format!("Failed to parse file {:?}", path))?;
                for b in batches {
                    let record_batch = b.to_batch();
                    for row in record_batch.rows() {
                        let row = row.map_err(|e| {
                            anyhow!("Failed to read row from {:?}: {:?}", path, e)
                        })?;
                        rows.push(row.into_owned());
                    }
                }
            }
        }
        Ok(rows)
    })
    .await
    .context("Failed to join directory scan thread")?
}

/// Writes every recovered pair to a single Parquet file at `output_path`.
async fn write_pairs(rows: Vec<PyroRow<'static>>, output_path: &Path) -> Result<()> {
    let output_path = output_path.to_path_buf();
    tokio::task::spawn_blocking(move || -> Result<()> {
        let schema = PyroRow::infer_schema(&rows)
            .map_err(|e| anyhow!("Failed to infer schema for pairs: {:?}", e))?;
        let mut prebatch = PreBatch::new(schema);
        let row_count = rows.len();
        for row in rows {
            prebatch.push_unchecked(row);
        }

        let record_batch = prebatch
            .flush()
            .map_err(|e| anyhow!("Failed to flush pairs to RecordBatch: {:?}", e))?
            .ok_or_else(|| anyhow!("Pairs flush unexpectedly produced no RecordBatch"))?;

        write_parquet(&[record_batch], &output_path)
            .with_context(|| format!("Failed to write parquet file to {:?}", output_path))?;

        tracing::info!(?output_path, rows = row_count, "Wrote pairs file");
        Ok(())
    })
    .await
    .context("Failed to join pairs writer thread")?
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyroduct::format::log_wal::{LogEntry, LogWal};
    use pyroduct::pipeline::data::DataManager;
    use tempfile::tempdir;

    fn row(id: i32, name: &str) -> PyroRow<'static> {
        PyroRow::from([
            ("id", PyroValue::from(id)),
            ("name", PyroValue::from(name.to_string())),
        ])
    }

    /// Mimics what `Pipeline::process` does for one row (without a real wasm shard):
    /// push the input unconditionally, log unconditionally, and push the output only
    /// on success.
    async fn record_row(
        input_manager: &DataManager,
        output_manager: &DataManager,
        log_manager: &mut LogWal,
        row_index: usize,
        input: PyroRow<'static>,
        outcome: Result<PyroRow<'static>, String>,
    ) {
        input_manager.push_record(row_index, &input).await.unwrap();
        match outcome {
            Ok(output) => {
                output_manager.push_record(row_index, &output).await.unwrap();
                log_manager
                    .append(&LogEntry {
                        row_index,
                        module_logs: Vec::new(),
                        capability_logs: std::collections::HashMap::new(),
                        failure: None,
                    })
                    .await
                    .unwrap();
            }
            Err(msg) => {
                log_manager
                    .append(&LogEntry {
                        row_index,
                        module_logs: Vec::new(),
                        capability_logs: std::collections::HashMap::new(),
                        failure: Some(Err(msg)),
                    })
                    .await
                    .unwrap();
            }
        }
    }

    #[tokio::test]
    async fn test_dump_pairs_recovers_across_a_simulated_restart() {
        let log_dir = tempdir().unwrap();
        let input_dir = tempdir().unwrap();
        let output_dir = tempdir().unwrap();
        let dest_dir = tempdir().unwrap();
        let output_path = dest_dir.path().join("pairs.parquet");

        let schema = row(0, "seed").schema().unwrap();

        // "Generation 1": indices 0 (success), 1 (failure, no output), 2 (success).
        {
            let input_manager = DataManager::new(input_dir.path(), schema.clone(), 1000);
            let output_manager = DataManager::new(output_dir.path(), schema.clone(), 1000);
            let mut log_manager = LogWal::open(log_dir.path(), 1000).await.unwrap();

            record_row(&input_manager, &output_manager, &mut log_manager, 0, row(1, "alice-in"), Ok(row(10, "alice-out"))).await;
            record_row(&input_manager, &output_manager, &mut log_manager, 1, row(2, "bob-in"), Err("boom".to_string())).await;
            record_row(&input_manager, &output_manager, &mut log_manager, 2, row(3, "carol-in"), Ok(row(30, "carol-out"))).await;

            log_manager.flush().await.unwrap();
            input_manager.flush_wal().await.unwrap();
            output_manager.flush_wal().await.unwrap();
            // Roll generation 1's data safely out of the way so generation 2 (which
            // will also produce a batch_1.arrow of its own) can't overwrite it —
            // mirrors the real world, where old data only survives a rollover if it
            // was already rolled out before the collision would hit.
            input_manager.rollout_to_parquet().await.unwrap();
            output_manager.rollout_to_parquet().await.unwrap();
        }

        // Simulate the restart: fresh, non-restored managers reusing index 0, 1.
        // The log WAL, however, is reopened and correctly continues (this is the
        // part of the on-disk state that stays trustworthy across a restart).
        {
            let input_manager = DataManager::new(input_dir.path(), schema.clone(), 1000);
            let output_manager = DataManager::new(output_dir.path(), schema, 1000);
            let mut log_manager = LogWal::open(log_dir.path(), 1000).await.unwrap();

            record_row(&input_manager, &output_manager, &mut log_manager, 0, row(4, "dave-in"), Ok(row(40, "dave-out"))).await;
            record_row(&input_manager, &output_manager, &mut log_manager, 1, row(5, "erin-in"), Ok(row(50, "erin-out"))).await;

            log_manager.flush().await.unwrap();
            input_manager.flush_wal().await.unwrap();
            output_manager.flush_wal().await.unwrap();
        }

        dump_pairs(
            log_dir.path(),
            input_dir.path(),
            output_dir.path(),
            &output_path,
        )
        .await
        .unwrap();

        assert!(output_path.exists());

        let bytes = std::fs::read(&output_path).unwrap();
        let batches: Vec<arrow::record_batch::RecordBatch> =
            pyro_file::parse_data_to_batch_sync(bytes, "pairs.parquet")
                .unwrap()
                .into_iter()
                .map(|b| b.to_batch())
                .collect();
        assert_eq!(batches.len(), 1);

        // Row 1 failed and generation 0 only had 3 rows total, so we expect 4
        // pairs total: true row_index 0, 2 (generation 0) and 3, 4 (generation 1,
        // continuing right after generation 0's last row despite reusing local
        // indices 0 and 1 after the rollover).
        assert_eq!(batches[0].num_rows(), 4);
        let r0 = batches[0].row(0).unwrap();
        let r1 = batches[0].row(1).unwrap();
        let r2 = batches[0].row(2).unwrap();
        let r3 = batches[0].row(3).unwrap();

        assert_eq!(r0.get("row_index"), Some(&PyroValue::U64(0)));
        assert_eq!(r0.get("generation"), Some(&PyroValue::U64(0)));
        assert_eq!(r0.get("local_index"), Some(&PyroValue::U64(0)));

        assert_eq!(r1.get("row_index"), Some(&PyroValue::U64(2)));
        assert_eq!(r1.get("generation"), Some(&PyroValue::U64(0)));
        assert_eq!(r1.get("local_index"), Some(&PyroValue::U64(2)));

        assert_eq!(r2.get("row_index"), Some(&PyroValue::U64(3)));
        assert_eq!(r2.get("generation"), Some(&PyroValue::U64(1)));
        assert_eq!(r2.get("local_index"), Some(&PyroValue::U64(0)));

        assert_eq!(r3.get("row_index"), Some(&PyroValue::U64(4)));
        assert_eq!(r3.get("generation"), Some(&PyroValue::U64(1)));
        assert_eq!(r3.get("local_index"), Some(&PyroValue::U64(1)));
    }
}
