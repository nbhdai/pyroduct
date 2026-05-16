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
    pub capability_logs: HashMap<(String, String), Vec<String>>,
    pub failure: Option<CapturedError>,
}

/// `LogWal` provides an async file-backed write-ahead log for `LogRecord`s using tokio.
/// 
/// Records are framed using CSC encoding:
/// `[ Length (u32) | CRC-32C (u32) | JSON Payload ]`
pub struct LogWal {
    writer: BufWriter<File>,
}

impl LogWal {
    /// Opens a log file for appending. Creates the file if it doesn't exist.
    pub async fn open<P: AsRef<Path>>(path: P) -> tokio::io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;
        
        Ok(Self {
            writer: BufWriter::new(file),
        })
    }

    /// Appends a `LogRecord` to the file using JSON serialization and CRC framing.
    pub async fn append(&mut self, record: &LogEntry) -> tokio::io::Result<()> {
        let payload = serde_json::to_vec(record)
            .map_err(|e| tokio::io::Error::new(tokio::io::ErrorKind::InvalidData, e))?;
        
        let len = payload.len() as u32;
        let crc = crc32c(&payload);

        // Frame: [ len (4) | crc (4) | payload (n) ]
        self.writer.write_all(&len.to_le_bytes()).await?;
        self.writer.write_all(&crc.to_le_bytes()).await?;
        self.writer.write_all(&payload).await?;
        self.writer.flush().await?;

        Ok(())
    }

    /// Ensures all buffered logs are written to disk.
    pub async fn flush(&mut self) -> tokio::io::Result<()> {
        self.writer.flush().await
    }

    /// Returns the underlying file.
    pub fn into_inner(self) -> File {
        self.writer.into_inner()
    }
}

/// `LogWalReader` provides an async reader to iterate over `LogRecord`s from a log file.
pub struct LogWalReader {
    reader: BufReader<File>,
}

impl LogWalReader {
    /// Opens a log file for reading.
    pub async fn open<P: AsRef<Path>>(path: P) -> tokio::io::Result<Self> {
        let file = File::open(path).await?;
        Ok(Self {
            reader: BufReader::new(file),
        })
    }

    /// Reads the next `LogRecord` from the log file.
    /// Returns `Ok(None)` when the end of the file is reached.
    pub async fn next(&mut self) -> tokio::io::Result<Option<LogEntry>> {
        let mut len_buf = [0u8; 4];
        
        // Attempt to read the length. If we get UnexpectedEof here, it's a clean EOF.
        if let Err(e) = self.reader.read_exact(&mut len_buf).await {
            if e.kind() == tokio::io::ErrorKind::UnexpectedEof {
                return Ok(None);
            }
            return Err(e);
        }
        
        let len = u32::from_le_bytes(len_buf) as usize;
        
        // Read the CRC
        let mut crc_buf = [0u8; 4];
        self.reader.read_exact(&mut crc_buf).await?;
        let expected_crc = u32::from_le_bytes(crc_buf);

        // Read the payload
        let mut payload = vec![0u8; len];
        self.reader.read_exact(&mut payload).await?;
        
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

        Ok(Some(record))
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
    use tempfile::NamedTempFile;

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
        let tmp_file = NamedTempFile::new().unwrap();
        let path = tmp_file.path();

        let mut wal = LogWal::open(path).await.unwrap();
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
        let tmp_file = NamedTempFile::new().unwrap();
        let path = tmp_file.path();

        let mut wal = LogWal::open(path).await.unwrap();
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
        let tmp_file = NamedTempFile::new().unwrap();
        let path = tmp_file.path();

        {
            let mut wal = LogWal::open(path).await.unwrap();
            wal.append(&create_test_entry(0)).await.unwrap();
            wal.flush().await.unwrap();
        }

        // Corrupt the file: Flip a bit in the payload
        let mut data = std::fs::read(path).unwrap();
        if data.len() > 10 {
            data[10] ^= 0xFF;
        }
        std::fs::write(path, data).unwrap();

        let mut reader = LogWalReader::open(path).await.unwrap();
        let result = reader.next().await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("CRC mismatch"));
    }

    #[tokio::test]
    async fn test_log_wal_empty_file() {
        let tmp_file = NamedTempFile::new().unwrap();
        let path = tmp_file.path();

        let mut reader = LogWalReader::open(path).await.unwrap();
        let result = reader.next().await.unwrap();
        assert!(result.is_none());
    }
}
