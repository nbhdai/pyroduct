use std::collections::HashMap;
use std::path::Path;
use serde::{Deserialize, Serialize};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use std::io::SeekFrom;
use tokio::io::AsyncSeekExt;
use tracing::{debug, error, info};

use crate::CapturedError;

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
    #[serde(serialize_with = "serialize_cap_logs", deserialize_with = "deserialize_cap_logs")]
    pub capability_logs: HashMap<(String, String), Vec<String>>,
    pub failure: Option<CapturedError>,
    pub success_index: Option<usize>,
}

fn serialize_cap_logs<S>(
    logs: &HashMap<(String, String), Vec<String>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let list: Vec<(&(String, String), &Vec<String>)> = logs.iter().collect();
    list.serialize(serializer)
}

fn deserialize_cap_logs<'de, D>(
    deserializer: D,
) -> Result<HashMap<(String, String), Vec<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let list = Vec::<((String, String), Vec<String>)>::deserialize(deserializer)?;
    Ok(list.into_iter().collect())
}

/// Ensure the index file for a given log file index exists and is up to date.
async fn ensure_index_file(dir: &Path, file_index: usize) -> tokio::io::Result<()> {
    let log_path = dir.join(format!("{}.pyrolog", file_index));
    let idx_path = dir.join(format!("{}.pyrolog.idx", file_index));

    if !tokio::fs::metadata(&log_path).await.is_ok() {
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
            if path.extension().map_or(false, |ext| ext == "pyrolog") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(idx) = stem.parse::<usize>() {
                        files.push(idx);
                    }
                }
            }
        }
        files.sort_unstable();

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
            let f = OpenOptions::new().create(true).append(true).open(&next_path).await?;
            (next_idx, 0, f)
        } else {
            let f = OpenOptions::new().create(true).append(true).open(&path).await?;
            (current_file_index, current_entries, f)
        };

        let total_entries = if final_idx > 0 {
            final_idx * capacity + final_entries
        } else {
            final_entries
        };

        // Ensure index files exist and are built up to final_idx
        for idx in 0..=final_idx {
            ensure_index_file(&dir, idx).await?;
        }

        debug!(
            current_file_index = final_idx,
            total_entries = total_entries,
            "Log WAL initialized"
        );

        let idx_path = dir.join(format!("{}.pyrolog.idx", final_idx));
        let idx_file = OpenOptions::new().create(true).append(true).open(&idx_path).await?;

        let current_offset = if final_entries > 0 {
            tokio::fs::metadata(&path).await?.len()
        } else {
            0
        };

        Ok(Self {
            dir,
            capacity,
            current_file_index: final_idx,
            current_entries: final_entries,
            total_entries,
            writer: BufWriter::new(final_file),
            idx_writer: BufWriter::new(idx_file),
            current_offset,
        })
    }

    async fn rotate(&mut self) -> tokio::io::Result<()> {
        info!(new_file_index = self.current_file_index + 1, "Rotating log WAL to new file");
        self.writer.flush().await?;
        self.idx_writer.flush().await?;

        self.current_file_index += 1;
        self.current_entries = 0;
        self.current_offset = 0;

        let path = self.dir.join(format!("{}.pyrolog", self.current_file_index));
        let file = OpenOptions::new().create(true).append(true).open(path).await?;
        self.writer = BufWriter::new(file);

        let idx_path = self.dir.join(format!("{}.pyrolog.idx", self.current_file_index));
        let idx_file = OpenOptions::new().create(true).append(true).open(idx_path).await?;
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
        self.idx_writer.write_all(&self.current_offset.to_le_bytes()).await?;
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

    /// Ensures all buffered logs are written to disk.
    pub async fn flush(&mut self) -> tokio::io::Result<()> {
        self.idx_writer.flush().await?;
        self.writer.flush().await
    }

    /// Returns the underlying log directory path.
    pub fn dir(&self) -> &Path {
        &self.dir
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
        Ok(Self {
            dir,
            current_file_index: 0,
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

        if !tokio::fs::metadata(&log_path).await.is_ok() {
            return Ok(None);
        }

        // Ensure the index file exists and is rebuilt if needed
        ensure_index_file(&self.dir, file_index).await?;

        if !tokio::fs::metadata(&idx_path).await.is_ok() {
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
            error!(index = index, "Log record CRC mismatch: data corruption detected");
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
                let path = self.dir.join(format!("{}.pyrolog", self.current_file_index));
                if !tokio::fs::metadata(&path).await.is_ok() {
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
                    debug!(new_file_index = self.current_file_index + 1, "Moving to next log file");
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
            capability_logs: HashMap::from([
                (("cap1".to_string(), "val1".to_string()), vec!["cap log 1".to_string()]),
            ]),
            failure: None,
            success_index: None,
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
        let record = reader.next().await.unwrap().expect("Should have one record");

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
        for i in 0..5 {
            assert_eq!(records[i].row_index, i);
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
        let entry = wal.get(1).await.unwrap().expect("Should retrieve successfully after index deletion");
        assert_eq!(entry.row_index, 1);
        assert!(idx_file_0.exists(), "Index file should have been rebuilt");

        // Verify it still works for other records
        let entry_2 = wal.get(2).await.unwrap().expect("Should find record 2");
        assert_eq!(entry_2.row_index, 2);
    }
}
