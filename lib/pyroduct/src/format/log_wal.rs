use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::SeekFrom;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncSeekExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::sync::{Mutex, mpsc, oneshot};
use tracing::{debug, error, info};

use crate::CapturedError;

use super::execution::{deserialize_cap_logs, serialize_cap_logs};

/// Computes the CRC-32C checksum of the given data.
fn crc32c(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0x82F6_3B78;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub row_index: usize,
    pub module_logs: Vec<String>,
    #[serde(
        serialize_with = "serialize_cap_logs",
        deserialize_with = "deserialize_cap_logs"
    )]
    pub capability_logs: HashMap<(String, String), Vec<String>>,
    pub failure: Option<Result<CapturedError, String>>,
}

/// Ensure the index file for a given log file index exists and is up to date.
async fn ensure_index_file(dir: &Path, file_index: usize) -> tokio::io::Result<()> {
    let log_path = dir.join(format!("{}.pyrolog", file_index));
    let idx_path = dir.join(format!("{}.pyrolog.idx", file_index));

    if tokio::fs::metadata(&log_path).await.is_err() {
        return Ok(());
    }

    let log_meta_len = tokio::fs::metadata(&log_path).await?.len();
    let idx_meta_opt = tokio::fs::metadata(&idx_path).await.ok();

    let should_rebuild = match idx_meta_opt {
        None => true,
        Some(m) => {
            let idx_len = m.len();
            idx_len % 8 != 0 || (log_meta_len > 0 && idx_len == 0)
        }
    };

    if should_rebuild {
        info!(dir = ?dir, file_index = file_index, "Rebuilding log index file");
        let mut log_file = File::open(&log_path).await?;
        let mut idx_file = File::create(&idx_path).await?;

        let mut offset: u64 = 0;
        let mut len_buf = [0u8; 4];
        while log_file.read_exact(&mut len_buf).await.is_ok() {
            idx_file.write_all(&offset.to_le_bytes()).await?;

            let len = u32::from_le_bytes(len_buf) as u64;
            let record_len = 4 + 4 + len;
            offset += record_len;

            log_file.seek(std::io::SeekFrom::Start(offset)).await?;
        }
        idx_file.flush().await?;
    }
    Ok(())
}

/// `LogWal` provides an async file-backed write-ahead log for `LogRecord`s using tokio.
///
/// Records are framed using CSC encoding:
/// `[ Length (u32) | CRC-32C (u32) | JSON Payload ]`
pub struct LogWal {
    dir: std::path::PathBuf,
    capacity: usize,
    current_file_index: usize,
    oldest_file_index: usize,
    current_entries: usize,
    total_entries: usize,
    writer: BufWriter<File>,
    idx_writer: BufWriter<File>,
    current_offset: u64,
}

impl LogWal {
    /// Opens a log directory for appending. Creates the directory and files if they don't exist.
    pub async fn open<P: AsRef<Path>>(dir: P, capacity: usize) -> tokio::io::Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        info!(dir = ?dir, capacity = capacity, "Opening log WAL");
        tokio::fs::create_dir_all(&dir).await?;

        let mut entries = tokio::fs::read_dir(&dir).await?;
        let mut files = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "pyrolog")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                && let Ok(idx) = stem.parse::<usize>()
            {
                files.push(idx);
            }
        }
        files.sort_unstable();

        let oldest_file_index = files.first().cloned().unwrap_or(0);
        let current_file_index = files.last().cloned().unwrap_or(0);
        let path = dir.join(format!("{}.pyrolog", current_file_index));

        let mut current_entries = 0;
        if tokio::fs::metadata(&path).await.is_ok() {
            let mut reader = BufReader::new(File::open(&path).await?);
            let mut len_buf = [0u8; 4];
            while reader.read_exact(&mut len_buf).await.is_ok() {
                let len = u32::from_le_bytes(len_buf) as usize;
                let mut skip_buf = vec![0u8; 4 + len];
                if reader.read_exact(&mut skip_buf).await.is_err() {
                    break;
                }
                current_entries += 1;
            }
        }

        let (final_idx, final_entries, final_file) = if current_entries >= capacity {
            let next_idx = current_file_index + 1;
            let next_path = dir.join(format!("{}.pyrolog", next_idx));
            let f = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&next_path)
                .await?;
            (next_idx, 0, f)
        } else {
            let f = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .await?;
            (current_file_index, current_entries, f)
        };

        let total_entries = if final_idx > 0 {
            final_idx * capacity + final_entries
        } else {
            final_entries
        };

        // Ensure index files exist and are built up to final_idx
        for idx in oldest_file_index..=final_idx {
            ensure_index_file(&dir, idx).await?;
        }

        debug!(
            current_file_index = final_idx,
            total_entries = total_entries,
            "Log WAL initialized"
        );

        let idx_path = dir.join(format!("{}.pyrolog.idx", final_idx));
        let idx_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&idx_path)
            .await?;

        let current_offset = if final_entries > 0 {
            tokio::fs::metadata(&path).await?.len()
        } else {
            0
        };

        Ok(Self {
            dir,
            capacity,
            current_file_index: final_idx,
            oldest_file_index,
            current_entries: final_entries,
            total_entries,
            writer: BufWriter::new(final_file),
            idx_writer: BufWriter::new(idx_file),
            current_offset,
        })
    }

    async fn rotate(&mut self) -> tokio::io::Result<()> {
        info!(
            new_file_index = self.current_file_index + 1,
            "Rotating log WAL to new file"
        );
        self.writer.flush().await?;
        self.idx_writer.flush().await?;

        self.current_file_index += 1;
        self.current_entries = 0;
        self.current_offset = 0;

        let path = self
            .dir
            .join(format!("{}.pyrolog", self.current_file_index));
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;
        self.writer = BufWriter::new(file);

        let idx_path = self
            .dir
            .join(format!("{}.pyrolog.idx", self.current_file_index));
        let idx_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(idx_path)
            .await?;
        self.idx_writer = BufWriter::new(idx_file);

        Ok(())
    }

    /// Appends a `LogRecord` to the file using JSON serialization and CRC framing.
    pub async fn append(&mut self, record: &LogEntry) -> tokio::io::Result<()> {
        if self.current_entries >= self.capacity {
            self.rotate().await?;
        }

        debug!(row_index = record.row_index, "Appending log entry");
        let payload = serde_json::to_vec(record)
            .map_err(|e| tokio::io::Error::new(tokio::io::ErrorKind::InvalidData, e))?;

        let len = payload.len() as u32;
        let crc = crc32c(&payload);

        // Write offset to index file
        self.idx_writer
            .write_all(&self.current_offset.to_le_bytes())
            .await?;
        self.idx_writer.flush().await?;

        // Frame: [ len (4) | crc (4) | payload (n) ]
        self.writer.write_all(&len.to_le_bytes()).await?;
        self.writer.write_all(&crc.to_le_bytes()).await?;
        self.writer.write_all(&payload).await?;
        self.writer.flush().await?;

        self.current_offset += 4 + 4 + payload.len() as u64;
        self.current_entries += 1;
        self.total_entries += 1;

        Ok(())
    }

    /// Appends multiple `LogEntry` records to the log, rotating segment files as needed,
    /// and flushes all changes to disk at the end of the batch.
    pub async fn append_batch(&mut self, records: &[LogEntry]) -> tokio::io::Result<()> {
        if records.is_empty() {
            return Ok(());
        }

        for record in records {
            if self.current_entries >= self.capacity {
                self.rotate().await?;
            }

            debug!(row_index = record.row_index, "Appending batch log entry");
            let payload = serde_json::to_vec(record)
                .map_err(|e| tokio::io::Error::new(tokio::io::ErrorKind::InvalidData, e))?;

            let len = payload.len() as u32;
            let crc = crc32c(&payload);

            // Write offset to index file
            self.idx_writer
                .write_all(&self.current_offset.to_le_bytes())
                .await?;

            // Frame: [ len (4) | crc (4) | payload (n) ]
            self.writer.write_all(&len.to_le_bytes()).await?;
            self.writer.write_all(&crc.to_le_bytes()).await?;
            self.writer.write_all(&payload).await?;

            self.current_offset += 4 + 4 + payload.len() as u64;
            self.current_entries += 1;
            self.total_entries += 1;
        }

        self.flush().await?;

        Ok(())
    }

    /// Ensures all buffered logs are written to disk.
    pub async fn flush(&mut self) -> tokio::io::Result<()> {
        self.idx_writer.flush().await?;
        self.writer.flush().await
    }

    /// Returns the underlying log directory path.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Returns the capacity of the log segment.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the underlying file.
    pub fn total_entries(&self) -> usize {
        self.total_entries
    }

    /// Returns the underlying file.
    pub fn into_inner(self) -> File {
        self.writer.into_inner()
    }

    /// Retrieve a single LogEntry by its global index in O(1) file access.
    pub async fn get(&self, index: usize) -> tokio::io::Result<Option<LogEntry>> {
        let reader = LogWalReader::open(&self.dir).await?;
        reader.get(index, self.capacity).await
    }

    /// Returns the global index of the oldest log entry currently present in the WAL, if any.
    pub fn oldest_log(&self) -> Option<usize> {
        if self.total_entries == 0 {
            None
        } else {
            Some(self.oldest_file_index * self.capacity)
        }
    }

    /// Returns the global index of the youngest log entry currently present in the WAL, if any.
    pub fn youngest_log(&self) -> Option<usize> {
        if self.total_entries == 0 {
            None
        } else {
            Some(self.total_entries - 1)
        }
    }

    /// Deletes logs older than the last `keep_count` entries.
    /// Returns the number of segment files deleted.
    pub async fn delete_older_than(&mut self, keep_count: usize) -> tokio::io::Result<usize> {
        let cutoff = self.total_entries.saturating_sub(keep_count);
        if cutoff == 0 && keep_count != 0 {
            return Ok(0);
        }

        let mut deleted_count = 0;
        let mut files_to_delete = Vec::new();

        let mut entries = tokio::fs::read_dir(&self.dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "pyrolog")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                && let Ok(idx) = stem.parse::<usize>()
            {
                // The segment stores entries from idx * capacity to (idx + 1) * capacity - 1.
                // If all entries in this segment are strictly less than cutoff, we can delete it.
                // We should never delete the active segment being written to.
                if idx < self.current_file_index && (idx + 1) * self.capacity <= cutoff {
                    files_to_delete.push(idx);
                }
            }
        }

        for idx in &files_to_delete {
            let log_path = self.dir.join(format!("{}.pyrolog", idx));
            let idx_path = self.dir.join(format!("{}.pyrolog.idx", idx));
            if tokio::fs::metadata(&log_path).await.is_ok() {
                tokio::fs::remove_file(&log_path).await?;
                deleted_count += 1;
            }
            if tokio::fs::metadata(&idx_path).await.is_ok() {
                let _ = tokio::fs::remove_file(&idx_path).await;
            }
        }

        // If keep_count is 0, we clear the active segment completely to simulate a full purge.
        if keep_count == 0 {
            let log_path = self
                .dir
                .join(format!("{}.pyrolog", self.current_file_index));
            let idx_path = self
                .dir
                .join(format!("{}.pyrolog.idx", self.current_file_index));

            let _ = self.writer.flush().await;
            let _ = self.idx_writer.flush().await;

            let file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&log_path)
                .await?;
            self.writer = BufWriter::new(file);

            let idx_file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&idx_path)
                .await?;
            self.idx_writer = BufWriter::new(idx_file);

            self.current_entries = 0;
            self.total_entries = 0;
            self.current_offset = 0;
            self.oldest_file_index = self.current_file_index;
            deleted_count += 1;
        } else if deleted_count > 0 {
            // Update oldest_file_index
            let mut remaining_files = Vec::new();
            let mut entries = tokio::fs::read_dir(&self.dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "pyrolog")
                    && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                    && let Ok(idx) = stem.parse::<usize>()
                {
                    remaining_files.push(idx);
                }
            }
            remaining_files.sort_unstable();
            self.oldest_file_index = remaining_files.first().cloned().unwrap_or(0);
        }

        Ok(deleted_count)
    }
}

/// `LogManager` coordinates multiple asynchronous log writers feeding into a single `LogWal` on a background thread.
#[derive(Clone)]
pub struct LogManager {
    sender: mpsc::Sender<LogEntry>,
    total_len: Arc<AtomicUsize>,
    reader: Arc<LogWalReader>,
    capacity: usize,
    inner: Arc<Mutex<LogManagerInner>>,
}

struct LogManagerInner {
    shutdown_tx: Option<oneshot::Sender<()>>,
    join_handle: Option<tokio::task::JoinHandle<tokio::io::Result<()>>>,
}

impl LogManager {
    /// Creates and spawns a new `LogManager` that writes log entries to a `LogWal` in a background Tokio task.
    ///
    /// The manager holds a bounded sender queue of the specified `bound` size.
    pub async fn new(mut log_wal: LogWal, bound: usize) -> tokio::io::Result<Self> {
        let (sender, mut receiver) = mpsc::channel::<LogEntry>(bound);
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

        let initial_len = log_wal.total_entries();
        let total_len = Arc::new(AtomicUsize::new(initial_len));
        let total_len_clone = Arc::clone(&total_len);

        let reader = Arc::new(LogWalReader::open(log_wal.dir()).await?);
        let capacity = log_wal.capacity;

        let join_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    msg = receiver.recv() => {
                        match msg {
                            Some(entry) => {
                                log_wal.append(&entry).await?;
                            }
                            None => {
                                break;
                            }
                        }
                    }
                    _ = &mut shutdown_rx => {
                        break;
                    }
                }
            }
            log_wal.flush().await?;
            Ok(())
        });

        Ok(Self {
            sender,
            total_len: total_len_clone,
            reader,
            capacity,
            inner: Arc::new(Mutex::new(LogManagerInner {
                shutdown_tx: Some(shutdown_tx),
                join_handle: Some(join_handle),
            })),
        })
    }

    /// Sends a `LogEntry` to the `LogWal` writer.
    ///
    /// This method is asynchronous and can block if the bounded channel is full.
    /// Increments the `total_len` counter if the message is successfully enqueued.
    pub async fn send(&self, entry: LogEntry) -> Result<(), mpsc::error::SendError<LogEntry>> {
        self.sender.send(entry).await?;
        self.total_len.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    /// Returns the total number of log entries sent to the manager.
    pub fn total_len(&self) -> usize {
        self.total_len.load(Ordering::SeqCst)
    }

    /// Retrieves a single `LogEntry` by its global index in O(1) file access.
    pub async fn get(&self, index: usize) -> tokio::io::Result<Option<LogEntry>> {
        self.reader.get(index, self.capacity).await
    }

    /// Signals the background task to shut down, flushes the WAL, and waits for completion.
    pub async fn interrupt(&self) -> tokio::io::Result<()> {
        let mut inner = self.inner.lock().await;
        if let Some(tx) = inner.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = inner.join_handle.take() {
            match handle.await {
                Ok(res) => res?,
                Err(join_err) => {
                    return Err(tokio::io::Error::other(format!(
                        "Background task join error: {:?}",
                        join_err
                    )));
                }
            }
        }
        Ok(())
    }
}

/// `LogWalReader` provides an async reader to iterate over `LogRecord`s from a log directory.
pub struct LogWalReader {
    dir: std::path::PathBuf,
    current_file_index: usize,
    reader: Option<BufReader<File>>,
}

impl LogWalReader {
    /// Opens a log directory for reading.
    pub async fn open<P: AsRef<Path>>(dir: P) -> tokio::io::Result<Self> {
        let dir = dir.as_ref().to_path_buf();

        let mut entries = tokio::fs::read_dir(&dir).await?;
        let mut files = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "pyrolog")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                && let Ok(idx) = stem.parse::<usize>()
            {
                files.push(idx);
            }
        }
        files.sort_unstable();
        let first_file_index = files.first().cloned().unwrap_or(0);

        Ok(Self {
            dir,
            current_file_index: first_file_index,
            reader: None,
        })
    }

    /// Retrieve a single LogEntry by its global index in O(1) file access.
    pub async fn get(&self, index: usize, capacity: usize) -> tokio::io::Result<Option<LogEntry>> {
        debug!(index = index, "Retrieving log entry by index");
        if capacity == 0 {
            return Ok(None);
        }
        let file_index = index / capacity;
        let entry_index = index % capacity;

        let log_path = self.dir.join(format!("{}.pyrolog", file_index));
        let idx_path = self.dir.join(format!("{}.pyrolog.idx", file_index));

        if tokio::fs::metadata(&log_path).await.is_err() {
            return Ok(None);
        }

        // Ensure the index file exists and is rebuilt if needed
        ensure_index_file(&self.dir, file_index).await?;

        if tokio::fs::metadata(&idx_path).await.is_err() {
            return Ok(None);
        }

        let mut idx_file = File::open(&idx_path).await?;
        let idx_offset = (entry_index * 8) as u64;

        if idx_file.seek(SeekFrom::Start(idx_offset)).await.is_err() {
            return Ok(None);
        }

        let mut offset_buf = [0u8; 8];
        if idx_file.read_exact(&mut offset_buf).await.is_err() {
            return Ok(None);
        }
        let start_offset = u64::from_le_bytes(offset_buf);

        let mut log_file = File::open(&log_path).await?;
        if log_file.seek(SeekFrom::Start(start_offset)).await.is_err() {
            return Ok(None);
        }

        // Now read the target entry
        let mut len_buf = [0u8; 4];
        if log_file.read_exact(&mut len_buf).await.is_err() {
            return Ok(None);
        }
        let len = u32::from_le_bytes(len_buf) as usize;

        let mut crc_buf = [0u8; 4];
        log_file.read_exact(&mut crc_buf).await?;
        let expected_crc = u32::from_le_bytes(crc_buf);

        let mut payload = vec![0u8; len];
        log_file.read_exact(&mut payload).await?;

        if crc32c(&payload) != expected_crc {
            error!(
                index = index,
                "Log record CRC mismatch: data corruption detected"
            );
            return Err(tokio::io::Error::new(
                tokio::io::ErrorKind::InvalidData,
                "Log record CRC mismatch: data corruption detected",
            ));
        }

        let record = serde_json::from_slice::<LogEntry>(&payload)?;
        Ok(Some(record))
    }

    /// Reads the next `LogRecord` from the log directory.
    /// Returns `Ok(None)` when the end of the log directory is reached.
    pub async fn next(&mut self) -> tokio::io::Result<Option<LogEntry>> {
        loop {
            if self.reader.is_none() {
                let path = self
                    .dir
                    .join(format!("{}.pyrolog", self.current_file_index));
                if tokio::fs::metadata(&path).await.is_err() {
                    // Check if there are any higher file indices
                    let mut entries = tokio::fs::read_dir(&self.dir).await?;
                    let mut next_exists = false;
                    let mut min_higher = usize::MAX;
                    while let Some(entry) = entries.next_entry().await? {
                        let p = entry.path();
                        if p.extension().is_some_and(|ext| ext == "pyrolog")
                            && let Some(stem) = p.file_stem().and_then(|s| s.to_str())
                            && let Ok(idx) = stem.parse::<usize>()
                            && idx > self.current_file_index
                            && idx < min_higher
                        {
                            min_higher = idx;
                            next_exists = true;
                        }
                    }
                    if next_exists {
                        self.current_file_index = min_higher;
                        continue;
                    }
                    // No more log files
                    return Ok(None);
                }
                self.reader = Some(BufReader::new(File::open(path).await?));
            }

            let reader = self.reader.as_mut().unwrap();
            let mut len_buf = [0u8; 4];

            if let Err(e) = reader.read_exact(&mut len_buf).await {
                if e.kind() == tokio::io::ErrorKind::UnexpectedEof {
                    // End of current file, move to next
                    debug!(
                        new_file_index = self.current_file_index + 1,
                        "Moving to next log file"
                    );
                    self.reader = None;
                    self.current_file_index += 1;
                    continue;
                }
                return Err(e);
            }

            let len = u32::from_le_bytes(len_buf) as usize;

            // Read the CRC
            let mut crc_buf = [0u8; 4];
            reader.read_exact(&mut crc_buf).await?;
            let expected_crc = u32::from_le_bytes(crc_buf);

            // Read the payload
            let mut payload = vec![0u8; len];
            reader.read_exact(&mut payload).await?;

            // Verify checksum
            if crc32c(&payload) != expected_crc {
                error!("Log record CRC mismatch: data corruption detected");
                return Err(tokio::io::Error::new(
                    tokio::io::ErrorKind::InvalidData,
                    "Log record CRC mismatch: data corruption detected",
                ));
            }

            // Deserialize JSON
            let record = serde_json::from_slice::<LogEntry>(&payload)
                .map_err(|e| tokio::io::Error::new(tokio::io::ErrorKind::InvalidData, e))?;

            return Ok(Some(record));
        }
    }

    /// Reads all records from the current position to the end of the file.
    pub async fn read_all(&mut self) -> tokio::io::Result<Vec<LogEntry>> {
        let mut records = Vec::new();
        while let Some(record) = self.next().await? {
            records.push(record);
        }
        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_entry(row_index: usize) -> LogEntry {
        LogEntry {
            row_index,
            module_logs: vec!["test module log".to_string()],
            capability_logs: HashMap::from([(
                ("cap1".to_string(), "val1".to_string()),
                vec!["cap log 1".to_string()],
            )]),
            failure: None,
        }
    }

    #[tokio::test]
    async fn test_log_wal_roundtrip() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path();

        let mut wal = LogWal::open(path, 100).await.unwrap();
        let entry = create_test_entry(42);

        wal.append(&entry).await.unwrap();
        wal.flush().await.unwrap();

        let mut reader = LogWalReader::open(path).await.unwrap();
        let record = reader
            .next()
            .await
            .unwrap()
            .expect("Should have one record");

        assert_eq!(record.row_index, 42);
        assert_eq!(record.module_logs[0], "test module log");
        assert!(reader.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_log_wal_multiple_records() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path();

        let mut wal = LogWal::open(path, 100).await.unwrap();
        for i in 0..5 {
            wal.append(&create_test_entry(i)).await.unwrap();
        }
        wal.flush().await.unwrap();

        let mut reader = LogWalReader::open(path).await.unwrap();
        let records = reader.read_all().await.unwrap();

        assert_eq!(records.len(), 5);
        for (i, record) in records.iter().enumerate() {
            assert_eq!(record.row_index, i);
        }
    }

    #[tokio::test]
    async fn test_log_wal_append_batch() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path();

        let mut wal = LogWal::open(path, 100).await.unwrap();
        let mut entries = Vec::new();
        for i in 0..5 {
            entries.push(create_test_entry(i));
        }
        wal.append_batch(&entries).await.unwrap();

        let mut reader = LogWalReader::open(path).await.unwrap();
        let records = reader.read_all().await.unwrap();

        assert_eq!(records.len(), 5);
        for (i, record) in records.iter().enumerate() {
            assert_eq!(record.row_index, i);
        }
    }

    #[tokio::test]
    async fn test_log_wal_corruption() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path();

        {
            let mut wal = LogWal::open(path, 100).await.unwrap();
            wal.append(&create_test_entry(0)).await.unwrap();
            wal.flush().await.unwrap();
        }

        // Corrupt the file: Flip a bit in the payload
        let log_file = path.join("0.pyrolog");
        let mut data = std::fs::read(&log_file).unwrap();
        if data.len() > 10 {
            data[10] ^= 0xFF;
        }
        std::fs::write(&log_file, data).unwrap();

        let mut reader = LogWalReader::open(path).await.unwrap();
        let result = reader.next().await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("CRC mismatch"));
    }

    #[tokio::test]
    async fn test_log_wal_empty_file() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path();

        let mut reader = LogWalReader::open(path).await.unwrap();
        let result = reader.next().await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_log_wal_get() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path();

        // capacity is 2, so index 0 and 1 go in file 0, index 2 and 3 in file 1, etc.
        let mut wal = LogWal::open(path, 2).await.unwrap();

        for i in 0..5 {
            wal.append(&create_test_entry(i)).await.unwrap();
        }
        wal.flush().await.unwrap();

        // 1. Get each record by index
        for i in 0..5 {
            let entry = wal.get(i).await.unwrap().expect("Should find record");
            assert_eq!(entry.row_index, i);
        }

        // 2. Index past the end should return None
        assert!(wal.get(5).await.unwrap().is_none());

        // 3. Delete an index file to verify auto-rebuild of index file
        let idx_file_0 = path.join("0.pyrolog.idx");
        assert!(idx_file_0.exists());
        tokio::fs::remove_file(&idx_file_0).await.unwrap();
        assert!(!idx_file_0.exists());

        // Call get: should trigger auto-rebuild of the index file and successfully retrieve
        let entry = wal
            .get(1)
            .await
            .unwrap()
            .expect("Should retrieve successfully after index deletion");
        assert_eq!(entry.row_index, 1);
        assert!(idx_file_0.exists(), "Index file should have been rebuilt");

        // Verify it still works for other records
        let entry_2 = wal.get(2).await.unwrap().expect("Should find record 2");
        assert_eq!(entry_2.row_index, 2);
    }

    #[tokio::test]
    async fn test_log_wal_retention_and_ranges() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path();

        // capacity is 2
        let mut wal = LogWal::open(path, 2).await.unwrap();

        // Range should be empty/None initially
        assert_eq!(wal.oldest_log(), None);
        assert_eq!(wal.youngest_log(), None);

        // Append 5 entries
        for i in 0..5 {
            wal.append(&create_test_entry(i)).await.unwrap();
        }
        wal.flush().await.unwrap();

        // Range should be 0..4
        assert_eq!(wal.oldest_log(), Some(0));
        assert_eq!(wal.youngest_log(), Some(4));

        // Segment files:
        // 0.pyrolog (entries 0, 1)
        // 1.pyrolog (entries 2, 3)
        // 2.pyrolog (entry 4) -> active segment

        // Let's delete logs older than 3 entries (so cutoff is 5 - 3 = 2).
        // Segment 0 (entries 0, 1) has max possible entry index = 1. Since (0+1)*2 <= 2 (2 <= 2), segment 0 can be deleted.
        // Segment 1 (entries 2, 3) has max possible entry index = 3. Since (1+1)*2 = 4 > 2, it is kept.
        // Segment 2 (active) is kept.
        let deleted = wal.delete_older_than(3).await.unwrap();
        assert_eq!(deleted, 1);

        // Oldest log should now be 1 * capacity = 2
        assert_eq!(wal.oldest_log(), Some(2));
        assert_eq!(wal.youngest_log(), Some(4));

        // Verify reader starts at segment 1 instead of segment 0 and iterates successfully!
        let mut reader = LogWalReader::open(path).await.unwrap();
        let records = reader.read_all().await.unwrap();
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].row_index, 2);
        assert_eq!(records[1].row_index, 3);
        assert_eq!(records[2].row_index, 4);

        // Try deleting again with keep_count = 1 (cutoff is 5 - 1 = 4).
        // Segment 1 (entries 2, 3) max is 3. Since (1+1)*2 <= 4 (4 <= 4), segment 1 can be deleted!
        let deleted2 = wal.delete_older_than(1).await.unwrap();
        assert_eq!(deleted2, 1);

        // Oldest log should now be 2 * capacity = 4
        assert_eq!(wal.oldest_log(), Some(4));
        assert_eq!(wal.youngest_log(), Some(4));

        let mut reader2 = LogWalReader::open(path).await.unwrap();
        let records2 = reader2.read_all().await.unwrap();
        assert_eq!(records2.len(), 1);
        assert_eq!(records2[0].row_index, 4);
    }

    #[tokio::test]
    async fn test_log_manager_basic() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path();

        let wal = LogWal::open(path, 100).await.unwrap();
        let manager = LogManager::new(wal, 10).await.unwrap();

        assert_eq!(manager.total_len(), 0);

        let entry = create_test_entry(42);
        manager.send(entry).await.unwrap();
        assert_eq!(manager.total_len(), 1);

        // Retrieve the entry via get
        let retrieved = manager.get(0).await.unwrap().expect("Should find entry");
        assert_eq!(retrieved.row_index, 42);

        manager.interrupt().await.unwrap();

        // Verify it was persisted by reopening
        let mut reader = LogWalReader::open(path).await.unwrap();
        let records = reader.read_all().await.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].row_index, 42);
    }

    #[tokio::test]
    async fn test_log_manager_concurrent() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path();

        let wal = LogWal::open(path, 100).await.unwrap();
        let manager = LogManager::new(wal, 5).await.unwrap();

        let mut tasks = Vec::new();
        for i in 0..10 {
            let m = manager.clone();
            tasks.push(tokio::spawn(async move {
                m.send(create_test_entry(i)).await.unwrap();
            }));
        }

        for t in tasks {
            t.await.unwrap();
        }

        assert_eq!(manager.total_len(), 10);

        // Verify we can retrieve them concurrently
        for i in 0..10 {
            let retrieved = manager.get(i).await.unwrap();
            assert!(retrieved.is_some());
        }

        manager.interrupt().await.unwrap();

        // Verify reopening shows all 10 entries
        let mut reader = LogWalReader::open(path).await.unwrap();
        let records = reader.read_all().await.unwrap();
        assert_eq!(records.len(), 10);
    }
}
