//! Write-Ahead Log for streaming per-step execution results to disk.
//!
//! The WAL stores `PyroSuccess` or `PyroFailure` records, each tagged with
//! a `row_index` so they can be reassembled later. The `.pyrowal` file
//! contains just the raw PyroVec packet bytes preceded by a 16-byte aligned 
//! prefix block to ensure 100% zero-copy mapping.
//!
//! ```text
//! ┌────────────────────── Frame ──────────────────────┐
//! │ row_index  : u32   ← 4 bytes                      │
//! │ padding    : [u8]  ← 12 bytes (16-byte alignment) │
//! │ packet     : [u8]  ← PyroVec header + payload     │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! The WAL manages a single `AtomicU32` for all views. Iteration yields
//! lifetime-bound `PyroRef<'a>`, but you can request an owned `PyroView` 
//! that safely increments the atomic counter.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU32, Ordering};

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::format::vec_buf::PyroRef;
use crate::format::{PyroFailure, PyroLogs, PyroSuccess, PyroView, get_ref};
use crate::format::header::PyroHeaderMut;
use crate::PyroRow;

// =============================================================================
// Log record (.pyrolog) — per-row logs, JSON framed
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRecord {
    pub row_index: usize,
    pub step_logs: Vec<LogEntry>,
    pub failure_logs: Option<LogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub module_logs: Vec<String>,
    pub capability_logs: HashMap<(String, String), Vec<String>>,
}

// =============================================================================
// CRC-32C & Alignment
// =============================================================================

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

#[inline]
fn align16(n: usize) -> usize {
    (n + 15) & !15
}

// =============================================================================
// Log file writer/reader (JSON framed, internal use)
// =============================================================================

struct LogFrameWriter {
    writer: BufWriter<File>,
}

impl LogFrameWriter {
    fn open(path: &Path) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self { writer: BufWriter::new(file) })
    }

    fn write_frame(&mut self, payload: &[u8]) -> io::Result<()> {
        let len = payload.len() as u32;
        let crc = crc32c(payload);
        self.writer.write_all(&len.to_le_bytes())?;
        self.writer.write_all(&crc.to_le_bytes())?;
        self.writer.write_all(payload)?;
        self.writer.flush()?;
        Ok(())
    }
}

struct LogFrameReader {
    reader: BufReader<File>,
}

impl LogFrameReader {
    fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        Ok(Self { reader: BufReader::new(file) })
    }

    fn next_frame(&mut self) -> Option<Vec<u8>> {
        let mut len_buf = [0u8; 4];
        if self.reader.read_exact(&mut len_buf).is_err() { return None; }
        let len = u32::from_le_bytes(len_buf) as usize;
        if len > 256 * 1024 * 1024 { return None; }

        let mut crc_buf = [0u8; 4];
        if self.reader.read_exact(&mut crc_buf).is_err() { return None; }
        
        let mut payload = vec![0u8; len];
        if self.reader.read_exact(&mut payload).is_err() { return None; }
        if crc32c(&payload) != u32::from_le_bytes(crc_buf) { return None; }

        Some(payload)
    }

    fn read_all_indexed(&mut self) -> HashMap<usize, LogRecord> {
        let mut map = HashMap::new();
        while let Some(payload) = self.next_frame() {
            if let Ok(rec) = serde_json::from_slice::<LogRecord>(&payload) {
                map.insert(rec.row_index, rec);
            }
        }
        map
    }
}

// =============================================================================
// WalRecord
// =============================================================================

#[derive(Clone)]
pub enum WalRecord {
    Success {
        row_index: usize,
        success: PyroSuccess,
    },
    Failure {
        row_index: usize,
        failure: PyroFailure,
    },
}

impl WalRecord {
    pub fn row_index(&self) -> usize {
        match self {
            WalRecord::Success { row_index, .. } => *row_index,
            WalRecord::Failure { row_index, .. } => *row_index,
        }
    }
}

// =============================================================================
// WalWriter
// =============================================================================

pub struct WalWriter {
    wal_file: BufWriter<File>,
    log_writer: LogFrameWriter,
    wal_path: PathBuf,
    log_path: PathBuf,
    records_written: u64,
}

impl WalWriter {
    pub fn open(base_path: impl Into<PathBuf>) -> io::Result<Self> {
        let base = base_path.into();
        let wal_path = base.with_extension("pyrowal");
        let log_path = base.with_extension("pyrolog");

        if let Some(parent) = wal_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let wal_file = BufWriter::new(OpenOptions::new().create(true).append(true).open(&wal_path)?);
        let log_writer = LogFrameWriter::open(&log_path)?;

        info!(wal = %wal_path.display(), log = %log_path.display(), "WAL opened for writing");

        Ok(Self { wal_file, log_writer, wal_path, log_path, records_written: 0 })
    }

    pub fn append(&mut self, record: &WalRecord) -> io::Result<()> {
        let row_index = record.row_index() as u32;

        // 1. Write 16-byte prefix [row_index (4) | padding (12)]
        // This ensures the packet that follows is naturally 16-byte aligned.
        let mut prefix = [0u8; 16];
        prefix[0..4].copy_from_slice(&row_index.to_le_bytes());
        self.wal_file.write_all(&prefix)?;

        // 2. Ship the record into a PyroVec packet
        let packet = match record {
            WalRecord::Success { success, .. } => success.row.to_wire().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?,
            WalRecord::Failure { failure, .. } => {
                let msg = match &failure.result { Ok(err) => err.message.clone(), Err(msg) => msg.clone() };
                let mut packet = crate::PyroValue::from(msg).to_wire().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
                packet.set_status_u8(1);
                packet
            }
        };

        let raw = packet.as_raw_slice();
        let padded = align16(raw.len());
        
        let mut buf = Vec::with_capacity(padded);
        buf.extend_from_slice(raw);
        buf.resize(padded, 0);
        self.wal_file.write_all(&buf)?;

        self.wal_file.flush()?;
        self.wal_file.get_ref().sync_data()?;

        // 3. Write logs
        if let Some(log_record) = LogRecord::from_record(record) {
            if let Ok(log_bytes) = serde_json::to_vec(&log_record) {
                let _ = self.log_writer.write_frame(&log_bytes);
            }
        }

        self.records_written += 1;
        Ok(())
    }

    pub fn records_written(&self) -> u64 { self.records_written }
    pub fn wal_path(&self) -> &Path { &self.wal_path }
    pub fn log_path(&self) -> &Path { &self.log_path }
}

// =============================================================================
// WalBuffer & WalInner
// =============================================================================

pub enum WalBuffer {
    Mmap(memmap2::Mmap),
    Vec(Vec<u8>),
    Static(&'static [u8]),
}

impl Deref for WalBuffer {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        match self {
            WalBuffer::Mmap(m) => m,
            WalBuffer::Vec(v) => v,
            WalBuffer::Static(s) => s,
        }
    }
}

#[repr(C, align(16))]
pub struct WalInner {
    pub ref_count: AtomicU32,
    pub buffer: WalBuffer,
}

// =============================================================================
// WalInner Dropper
// =============================================================================

unsafe extern "C" fn wal_inner_dropper(ptr: *mut u8, _capacity: u32) {
    let _ = unsafe { Box::from_raw(ptr as *mut WalInner) };
}

// =============================================================================
// WalReader (The single source of truth)
// =============================================================================

/// Reads a `.pyrowal` file/buffer and acts as the master owner of the memory.
///
/// It hands out zero-copy `PyroRef`s or explicitly tracked `PyroView`s.
pub struct WalReader {
    inner: NonNull<WalInner>,
    pub logs: HashMap<usize, LogRecord>,
    pub path: Option<PathBuf>,
}

unsafe impl Send for WalReader {}
unsafe impl Sync for WalReader {}

impl WalReader {
    /// Memory maps the WAL file from disk and loads any optional companion logs.
    pub fn open(base_path: impl Into<PathBuf>) -> io::Result<Self> {
        let base = base_path.into();
        let wal_path = base.with_extension("pyrowal");
        let log_path = base.with_extension("pyrolog");

        let file = File::open(&wal_path)?;
        let mmap = unsafe { memmap2::MmapOptions::new().map(&file)? };

        let logs = if log_path.exists() {
            LogFrameReader::open(&log_path)
                .map(|mut r| r.read_all_indexed())
                .unwrap_or_default()
        } else {
            HashMap::new()
        };

        let inner = Box::new(WalInner {
            ref_count: AtomicU32::new(1),
            buffer: WalBuffer::Mmap(mmap),
        });

        info!(path = %wal_path.display(), bytes = inner.buffer.len(), "WAL file mapped");

        Ok(Self {
            inner: unsafe { NonNull::new_unchecked(Box::into_raw(inner)) },
            logs,
            path: Some(wal_path),
        })
    }

    /// Creates a WAL reader from an owned memory buffer.
    pub fn from_vec(data: Vec<u8>) -> Self {
        let inner = Box::new(WalInner {
            ref_count: AtomicU32::new(1),
            buffer: WalBuffer::Vec(data),
        });

        Self {
            inner: unsafe { NonNull::new_unchecked(Box::into_raw(inner)) },
            logs: HashMap::new(),
            path: None,
        }
    }

    /// Iterate over frames yielding zero-copy `PyroRef`s bound to the reader's lifetime.
    pub fn frames(&self) -> WalFrameIter<'_> {
        let inner = unsafe { self.inner.as_ref() };
        WalFrameIter::new(&inner.buffer)
    }

    /// Acquires a persistent, reference-counted `PyroView` for a packet starting at `offset`.
    /// 
    /// `offset` MUST be the byte index of the 16-byte Data Header, which is provided
    /// by the `WalFrame::packet_offset` field during iteration.
    pub fn view_at(&self, offset: usize) -> crate::PyroResult<PyroView> {
        let inner = unsafe { self.inner.as_ref() };
        let reference = get_ref(&inner.buffer, offset)?;
        unsafe {
            crate::format::vec_buf::make_view(&inner.ref_count, reference, wal_inner_dropper)
        }
    }

    /// Recovers all data into owned `WalRecord` structs. 
    /// (Useful for loading small runs directly into memory).
    pub fn recover_all(&self) -> Vec<WalRecord> {
        self.frames()
            .filter_map(|frame| WalRecord::from_frame(&frame, self.logs.get(&frame.row_index)))
            .collect()
    }
}

impl Drop for WalReader {
    fn drop(&mut self) {
        unsafe {
            let inner = self.inner.as_ref();
            let refs = inner.ref_count.fetch_sub(1, Ordering::AcqRel);

            if refs == 1 {
                // We were the last owner
                let _ = Box::from_raw(self.inner.as_ptr());
            } else {
                // Other references (PyroViews) still exist
                if cfg!(debug_assertions) {
                    panic!(
                        "CRITICAL ERROR: Dropping WalReader while {} references (PyroView) still exist. Memory leaked.",
                        refs - 1
                    );
                } else {
                    tracing::error!(
                        ref_count = refs - 1,
                        "CRITICAL ERROR: Dropping WalReader while references still exist. Memory leaked."
                    );
                }
            }
        }
    }
}

// =============================================================================
// WalFrameIter
// =============================================================================

pub struct WalFrame<'a> {
    pub row_index: usize,
    pub packet: PyroRef<'a>,
    /// The exact byte offset into the WAL buffer where the 16-byte header starts.
    /// You can pass this to `WalReader::view_at(offset)`.
    pub packet_offset: usize, 
}

pub struct WalFrameIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> WalFrameIter<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }
}

impl<'a> Iterator for WalFrameIter<'a> {
    type Item = WalFrame<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // Require at least the 16 byte prefix
        if self.data.len() - self.offset < 16 {
            return None;
        }

        let row_index = u32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().ok()?) as usize;
        
        // Step over the 16-byte prefix to land right on the 16-byte aligned packet start
        self.offset += 16;
        let packet_start = self.offset;

        // Try generating the safe slice view
        let packet = match PyroRef::try_from_slice(&self.data[packet_start..]) {
            Ok(p) => p,
            Err(e) => {
                warn!(offset = packet_start, error = ?e, "Corrupt or invalid PyroRef in WAL");
                return None;
            }
        };

        // Advance the iterator offset by padded packet length
        let total_pkt = packet.as_raw_slice().len();
        self.offset += align16(total_pkt);

        Some(WalFrame {
            row_index,
            packet,
            packet_offset: packet_start,
        })
    }
}

// =============================================================================
// Helper: frame → WalRecord
// =============================================================================

impl WalRecord {
    fn from_frame(frame: &WalFrame, logs: Option<&LogRecord>) -> Option<Self> {
        use crate::format::header::PyroHeader;

        let pkt = frame.packet;
        let status = pkt.status_u8();

        let mut extracted_logs = PyroLogs::empty();
        if let Some(l) = logs {
            if status == 0 {
                if let Some(entry) = l.step_logs.first() {
                    extracted_logs.module_logs = entry.module_logs.clone();
                    extracted_logs.capability_logs = entry.capability_logs.clone();
                }
            } else if let Some(entry) = &l.failure_logs {
                extracted_logs.module_logs = entry.module_logs.clone();
                extracted_logs.capability_logs = entry.capability_logs.clone();
            }
        }

        if status == 0 {
            let row = PyroRow::parse_wire(&pkt).ok()?.to_static();
            Some(WalRecord::Success {
                row_index: frame.row_index,
                success: PyroSuccess { row, logs: extracted_logs },
            })
        } else {
            let msg = String::from_utf8_lossy(pkt.as_slice()).to_string();
            Some(WalRecord::Failure {
                row_index: frame.row_index,
                failure: PyroFailure {
                    result: Err(msg),
                    logs: extracted_logs,
                },
            })
        }
    }
}

// =============================================================================
// LogRecord Helper
// =============================================================================

impl LogRecord {
    pub fn from_record(record: &WalRecord) -> Option<Self> {
        match record {
            WalRecord::Success { success, .. } => {
                let has_logs = !success.logs.module_logs.is_empty() || !success.logs.capability_logs.is_empty();
                if !has_logs { return None; }
                Some(LogRecord {
                    row_index: record.row_index(),
                    step_logs: vec![LogEntry {
                        module_logs: success.logs.module_logs.clone(),
                        capability_logs: success.logs.capability_logs.clone(),
                    }],
                    failure_logs: None,
                })
            }
            WalRecord::Failure { failure, .. } => {
                let has_logs = !failure.logs.module_logs.is_empty() || !failure.logs.capability_logs.is_empty();
                if !has_logs { return None; }
                Some(LogRecord {
                    row_index: record.row_index(),
                    step_logs: Vec::new(),
                    failure_logs: Some(LogEntry {
                        module_logs: failure.logs.module_logs.clone(),
                        capability_logs: failure.logs.capability_logs.clone(),
                    }),
                })
            }
        }
    }
}

// =============================================================================
// Full recovery Helper
// =============================================================================

pub fn recover(base_path: impl Into<PathBuf>) -> io::Result<Vec<WalRecord>> {
    let reader = WalReader::open(base_path)?;
    Ok(reader.recover_all())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::{PyroRow, PyroValue, format::header::PyroData};
    use super::*;
    use tempfile::TempDir;

    fn make_success_record(row_index: usize) -> WalRecord {
        let row = PyroRow::from([
            ("id", PyroValue::from(row_index as i32)),
            ("name", PyroValue::from("test")),
        ]).into_owned();

        WalRecord::Success {
            row_index,
            success: PyroSuccess {
                row,
                logs: PyroLogs {
                    module_logs: vec![format!("processing row {}", row_index)],
                    capability_logs: HashMap::new(),
                },
            },
        }
    }

    fn make_failure_record(row_index: usize) -> WalRecord {
        WalRecord::Failure {
            row_index,
            failure: PyroFailure {
                result: Err(format!("row {} failed", row_index)),
                logs: PyroLogs::empty(),
            },
        }
    }

    #[test]
    fn test_roundtrip_via_file_reader() {
        let dir = TempDir::new().unwrap();
        let base = dir.path().join("test_run");

        let records: Vec<_> = (0..10).map(|i| {
            if i % 3 == 0 { make_failure_record(i) } else { make_success_record(i) }
        }).collect();

        // Write
        let mut wal = WalWriter::open(&base).unwrap();
        for record in &records {
            wal.append(record).unwrap();
        }
        assert_eq!(wal.records_written(), 10);

        // Recover
        let recovered = recover(&base).unwrap();
        assert_eq!(recovered.len(), 10);

        for (i, record) in recovered.iter().enumerate() {
            assert_eq!(record.row_index(), i);
        }
    }

    #[test]
    fn test_wal_data_views() {
        let dir = TempDir::new().unwrap();
        let base = dir.path().join("view_test");

        let mut writer = WalWriter::open(&base).unwrap();
        writer.append(&make_success_record(42)).unwrap();

        let reader = WalReader::open(&base).unwrap();
        let mut iter = reader.frames();
        let frame = iter.next().unwrap();
        
        assert_eq!(frame.row_index, 42);

        // Get a tracked view from the reader
        let view = reader.view_at(frame.packet_offset).expect("Should get view");
        let pyref = view.py_ref();

        let recovered_row = PyroRow::parse_wire(&pyref).expect("parse should work");
        assert_eq!(recovered_row.get("id"), Some(&PyroValue::from(42i32)));
    }
}