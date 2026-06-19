use super::DaemonDataManager;
use crate::Result;
use pyroduct::Capture;
use rand::Rng;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{Mutex, watch};

/// Shared replay progress state.
#[derive(Debug, Clone)]
pub struct ReplayStatus {
    pub running: bool,
    pub total_rows: usize,
    pub rows_completed: usize,
    pub successes: usize,
    pub errors: usize,
    pub current_file: String,
}

/// Handle to a background replay task.
pub struct ReplayHandle {
    pub status: Arc<Mutex<ReplayStatus>>,
    pub cancel_tx: watch::Sender<bool>,
}

/// File extensions supported for replay.
const SUPPORTED_EXTENSIONS: &[&str] = &["csv", "json", "jsonl", "parquet", "ipc", "arrow"];

fn is_supported_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| SUPPORTED_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

impl DaemonDataManager {
    pub async fn start_replay(
        &self,
        playbook_name: &str,
        folder_path: &str,
        interval_ms: u64,
        wiggle_ms: u64,
    ) -> Result<usize> {
        // 1. Check if a replay is already running for this playbook
        {
            let replays = self.replays.lock().await;
            if let Some(handle) = replays.get(playbook_name) {
                let status = handle.status.lock().await;
                if status.running {
                    pyroduct::bail!(
                        "A replay is already running for playbook '{}'",
                        playbook_name
                    );
                }
            }
        }

        // 2. Validate playbook exists and is running
        let server = {
            let workers = self.playbooks_manager.workers.lock().await;
            let worker = workers.get(playbook_name).ok_or_else(|| {
                pyroduct::capture!("Playbook '{}' is not running", playbook_name)
            })?;

            // Reject session playbooks (matching BulkCall behavior)
            let spec = worker.server.spec();
            if spec.func.kind == pyro_spec::ModuleKind::Session
                || spec.func.kind == pyro_spec::ModuleKind::SessionDiff
            {
                pyroduct::bail!("Replay is not supported for session playbooks");
            }

            worker.server.clone()
        };

        // 3. Scan and sort files
        let folder = Path::new(folder_path);
        if !folder.is_dir() {
            pyroduct::bail!("'{}' is not a directory", folder_path);
        }

        let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(folder)
            .capture("Failed to read replay folder")?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|p| p.is_file() && is_supported_file(p))
            .collect();

        files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

        if files.is_empty() {
            pyroduct::bail!(
                "No supported data files found in '{}'",
                folder_path
            );
        }

        // 4. Count total rows across all files
        let mut total_rows = 0usize;
        let mut file_batches: Vec<(String, Vec<pyro_file::ArrowIpc>)> = Vec::new();

        for file_path in &files {
            let file_name = file_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            let data = std::fs::read(file_path)
                .map_err(|e| pyroduct::capture!("Failed to read file '{}': {}", file_name, e))?;

            let batches = pyro_file::parse_data_to_batch(data, &file_name)
                .await
                .map_err(|e| {
                    pyroduct::capture!("Failed to parse file '{}': {:?}", file_name, e)
                })?;

            let file_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
            total_rows += file_rows;
            file_batches.push((file_name, batches));
        }

        if total_rows == 0 {
            pyroduct::bail!("All files in '{}' are empty", folder_path);
        }

        // 5. Set up cancel channel and shared status
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let status = Arc::new(Mutex::new(ReplayStatus {
            running: true,
            total_rows,
            rows_completed: 0,
            successes: 0,
            errors: 0,
            current_file: String::new(),
        }));

        let handle = ReplayHandle {
            status: status.clone(),
            cancel_tx,
        };

        // Store handle
        {
            let mut replays = self.replays.lock().await;
            replays.insert(playbook_name.to_string(), handle);
        }

        // 6. Spawn background replay task
        let spec = server.spec();
        let playbook_name_owned = playbook_name.to_string();
        let replays_ref = self.replays.clone();

        tokio::spawn(async move {
            let result = run_replay_loop(
                server,
                &spec,
                file_batches,
                interval_ms,
                wiggle_ms,
                status.clone(),
                cancel_rx,
            )
            .await;

            // Mark as finished
            {
                let mut s = status.lock().await;
                s.running = false;
            }

            if let Err(e) = result {
                tracing::error!(
                    playbook = %playbook_name_owned,
                    "Replay task failed: {:?}",
                    e
                );
            } else {
                tracing::info!(
                    playbook = %playbook_name_owned,
                    "Replay task completed"
                );
            }

            // Clean up handle after a delay so status can still be polled
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            let mut replays = replays_ref.lock().await;
            // Only remove if it's still the same (not replaced by a new replay)
            if let Some(h) = replays.get(&playbook_name_owned) {
                let s = h.status.lock().await;
                if !s.running {
                    drop(s);
                    replays.remove(&playbook_name_owned);
                }
            }
        });

        Ok(total_rows)
    }

    pub async fn get_replay_status(&self, playbook_name: &str) -> Option<ReplayStatus> {
        let replays = self.replays.lock().await;
        if let Some(handle) = replays.get(playbook_name) {
            let status = handle.status.lock().await;
            Some(status.clone())
        } else {
            None
        }
    }

    pub async fn stop_replay(&self, playbook_name: &str) {
        let replays = self.replays.lock().await;
        if let Some(handle) = replays.get(playbook_name) {
            let _ = handle.cancel_tx.send(true);
            tracing::info!(playbook = %playbook_name, "Sent cancel signal to replay task");
        }
    }
}

async fn run_replay_loop(
    server: pyroduct::pipeline::PipelineServer,
    spec: &std::sync::Arc<pyro_artifacts::artifacts::PlaybookSpec>,
    file_batches: Vec<(String, Vec<pyro_file::ArrowIpc>)>,
    interval_ms: u64,
    wiggle_ms: u64,
    status: Arc<Mutex<ReplayStatus>>,
    cancel_rx: watch::Receiver<bool>,
) -> Result<()> {
    use pyroduct::format::value::arrow::Rowable;

    for (file_name, batches) in file_batches {
        // Update current file
        {
            let mut s = status.lock().await;
            s.current_file = file_name.clone();
        }

        for batch_ipc in batches {
            let batch = batch_ipc.to_batch();
            for i in 0..batch.num_rows() {
                // Check cancellation
                if *cancel_rx.borrow() {
                    tracing::info!("Replay cancelled by user");
                    return Ok(());
                }

                // Extract and repair row
                let pyro_row = batch.row(i).map_err(|e| {
                    pyroduct::capture!("Row extraction failed at index {}: {:?}", i, e)
                })?;

                let repaired_row = pyro_row
                    .project_repair(spec.func.input.fields())
                    .map_err(|e| {
                        pyroduct::capture!(
                            "Failed to repair input according to module spec: {:?}",
                            e
                        )
                    })?;

                // Call playbook
                match server.call(repaired_row).await {
                    Ok(_rec) => {
                        let mut s = status.lock().await;
                        s.rows_completed += 1;
                        s.successes += 1;
                    }
                    Err(e) => {
                        tracing::warn!(
                            file = %file_name,
                            row = i,
                            "Replay row failed: {:?}",
                            e
                        );
                        let mut s = status.lock().await;
                        s.rows_completed += 1;
                        s.errors += 1;
                    }
                }

                // Sleep with wiggle
                let delay = if wiggle_ms > 0 {
                    let mut rng = rand::rng();
                    let jitter = rng.random_range(0..=wiggle_ms);
                    // Randomly add or subtract jitter, clamped to min 0
                    if rng.random_bool(0.5) {
                        interval_ms.saturating_add(jitter)
                    } else {
                        interval_ms.saturating_sub(jitter)
                    }
                } else {
                    interval_ms
                };

                if delay > 0 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                }
            }
        }
    }

    Ok(())
}
