use std::collections::HashMap;
use std::path::Path;
use serde::{Deserialize, Serialize};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};

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
}

impl LogWal {
    /// Opens a log directory for appending. Creates the directory and files if they don't exist.
    pub async fn open<P: AsRef<Path>>(dir: P, capacity: usize) -> tokio::io::Result<Self> {
        let dir = dir.as_ref().to_path_buf();
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

        Ok(Self {
            dir,
            capacity,
            current_file_index: final_idx,
            current_entries: final_entries,
            total_entries,
            writer: BufWriter::new(final_file),
        })
    }

    async fn rotate(&mut self) -> tokio::io::Result<()> {
        self.writer.flush().await?;
        self.current_file_index += 1;
        self.current_entries = 0;
        let path = self.dir.join(format!("{}.pyrolog", self.current_file_index));
        let file = OpenOptions::new().create(true).append(true).open(path).await?;
        self.writer = BufWriter::new(file);
        Ok(())
    }

    /// Appends a `LogRecord` to the file using JSON serialization and CRC framing.
    pub async fn append(&mut self, record: &LogEntry) -> tokio::io::Result<()> {
        if self.current_entries >= self.capacity {
            self.rotate().await?;
        }

        let payload = serde_json::to_vec(record)
            .map_err(|e| tokio::io::Error::new(tokio::io::ErrorKind::InvalidData, e))?;
        
        let len = payload.len() as u32;
        let crc = crc32c(&payload);

        // Frame: [ len (4) | crc (4) | payload (n) ]
        self.writer.write_all(&len.to_le_bytes()).await?;
        self.writer.write_all(&crc.to_le_bytes()).await?;
        self.writer.write_all(&payload).await?;
        self.writer.flush().await?;

        self.current_entries += 1;
        self.total_entries += 1;

        Ok(())
    }

    /// Ensures all buffered logs are written to disk.
    pub async fn flush(&mut self) -> tokio::io::Result<()> {
        self.writer.flush().await
    }

    /// Returns the underlying file.
    pub fn total_entries(&self) -> usize {
        self.total_entries
    }

    /// Returns the underlying file.
    pub fn into_inner(self) -> File {
        self.writer.into_inner()
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
}
