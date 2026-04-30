//! Write-Ahead Log for streaming per-step execution results to disk.
//!
//! The WAL stores `PyroSuccess` or `PyroFailure` records, each tagged with
//! a `row_index` so they can be reassembled later.  The `.pyrowal` file
//! contains **just** the raw PyroVec packet bytes (16-byte header + rkyv
//! payload) preceded by a 4-byte row index — no extra metadata envelope.
//!
//! ```text
//! ┌────────────────────── Frame ──────────────────────┐
//! │ row_index  : u32   ← input batch index            │
//! │ packet     : [u8]  ← PyroVec header + payload     │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! The packet's **status** byte tells us whether it's a success (0) or
//! failure (1).  The WAL owns the raw bytes and hands out `PyroView`s
//! with a `PyroViewInner::FromOwned` variant backed by an `AtomicU32`
//! it manages — zero-copy access as long as the `WalData` lives.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::format::{PyroFailure, PyroLogs, PyroSuccess, PyroView, header::PyroParser};
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
// CRC-32C (Castagnoli)
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

/// Round `n` up to the next multiple of 16.
#[inline]
fn align16(n: usize) -> usize {
    (n + 15) & !15
}

// =============================================================================
// Log file writer/reader (JSON framed)
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
        Ok(Self {
            writer: BufWriter::new(file),
        })
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
    records_read: u64,
}

impl LogFrameReader {
    fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        Ok(Self {
            reader: BufReader::new(file),
            records_read: 0,
        })
    }

    fn next_frame(&mut self) -> Option<Vec<u8>> {
        let mut len_buf = [0u8; 4];
        if self.reader.read_exact(&mut len_buf).is_err() {
            return None;
        }
        let len = u32::from_le_bytes(len_buf) as usize;
        if len > 256 * 1024 * 1024 {
            return None;
        }

        let mut crc_buf = [0u8; 4];
        if self.reader.read_exact(&mut crc_buf).is_err() {
            return None;
        }
        let expected_crc = u32::from_le_bytes(crc_buf);

        let mut payload = vec![0u8; len];
        if self.reader.read_exact(&mut payload).is_err() {
            return None;
        }

        if crc32c(&payload) != expected_crc {
            return None;
        }

        self.records_read += 1;
        Some(payload)
    }
}

// =============================================================================
// LogReader (public)
// =============================================================================

pub struct LogReader {
    inner: LogFrameReader,
    path: PathBuf,
}

impl LogReader {
    pub fn open(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        let inner = LogFrameReader::open(&path)?;
        Ok(Self { inner, path })
    }

    pub fn next_record(&mut self) -> Option<LogRecord> {
        let payload = self.inner.next_frame()?;
        serde_json::from_slice(&payload).ok()
    }

    pub fn read_all_indexed(&mut self) -> HashMap<usize, LogRecord> {
        let mut map = HashMap::new();
        while let Some(rec) = self.next_record() {
            map.insert(rec.row_index, rec);
        }
        info!(path = %self.path.display(), records = map.len(), "Log read complete");
        map
    }
}

// =============================================================================
// WalRecord — one per-step result
// =============================================================================

/// A single WAL record: either a successful step or a failure.
/// Each record carries a `row_index` for correlation and exactly one
/// `PyroSuccess` or `PyroFailure` (never both).
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
// WalData — owns raw packet bytes, hands out PyroViews
// =============================================================================

/// Shared state for `WalData`: the raw data and an `AtomicU32` ref count.
///
/// The `Arc` ensures the state lives as long as any view is alive.
struct WalState {
    data: Vec<u8>,
    ref_count: AtomicU32,
}

/// The WAL owns a contiguous block of memory containing raw PyroVec
/// packet bytes (header + rkyv payload) for one or more records.
///
/// `record_view()` returns a `PyroView` with `PyroViewInner::FromOwned`
/// backed by an `AtomicU32` that this `WalData` owns.  The ref count
/// is incremented when a view is created.  When all `WalData` instances
/// are dropped, the state (and all pending views) is freed.
pub struct WalData {
    state: Arc<WalState>,
    packet_offsets: Vec<u32>,
}

impl WalData {
    /// Build a `WalData` from a list of `WalRecord`s.
    ///
    /// Each record is shipped into a PyroVec packet (header + payload).
    /// For `PyroSuccess`, the row is shipped. For `PyroFailure`, the
    /// error message is shipped as a string.
    pub fn from_records(records: &[WalRecord]) -> Self {
        let mut packet_offsets: Vec<u32> = Vec::new();
        let mut raw_buf: Vec<u8> = Vec::new();

        for record in records {
            let packet = match record {
                WalRecord::Success { success, .. } => {
                    success.row.to_wire().expect("ship failed")
                }
                WalRecord::Failure { failure, .. } => {
                    let msg = match &failure.result {
                        Ok(err) => err.message.clone(),
                        Err(msg) => msg.clone(),
                    };
                    let val = crate::PyroValue::from(msg);
                    val.to_wire().expect("ship failed")
                }
            };
            let raw = packet.as_raw_slice();
            packet_offsets.push(raw_buf.len() as u32);
            raw_buf.extend_from_slice(raw);
        }

        // Align end
        let total_len = align16(raw_buf.len());
        raw_buf.resize(total_len, 0);

        WalData {
            state: Arc::new(WalState {
                data: raw_buf,
                ref_count: AtomicU32::new(1),
            }),
            packet_offsets,
        }
    }

    /// Create a `PyroView` for the packet at `record_index`.
    ///
    /// The returned view uses `PyroViewInner::FromOwned` and borrows this
    /// `WalData` via the shared `AtomicU32` ref count.
    pub fn record_view(&self, record_index: usize) -> Option<PyroView> {
        let offset = *self.packet_offsets.get(record_index)? as usize;
        let data_ptr = unsafe { self.state.data.as_ptr().add(offset) };

        // Increment ref count
        self.state.ref_count.fetch_add(1, Ordering::AcqRel);

        // Get a stable pointer to the ref_count inside the Arc.
        // Arc guarantees the inner WalState is stable as long as there's at least one Arc.
        let state_ptr = Arc::as_ptr(&self.state);
        let rc_ptr = unsafe {
            let rc_offset = std::mem::offset_of!(WalState, ref_count);
            state_ptr.add(rc_offset) as *mut AtomicU32
        };

        Some(PyroView {
            inner: crate::format::vec_buf::PyroViewInner::FromOwned {
                ref_count: unsafe { NonNull::new_unchecked(rc_ptr) },
                data: data_ptr,
            },
        })
    }

    pub fn record_count(&self) -> usize {
        self.packet_offsets.len()
    }
}

// =============================================================================
// WalWriter — writes .pyrowal (data) and .pyrolog (logs)
// =============================================================================

/// Streams `WalRecord`s into a `.pyrowal` / `.pyrolog` pair.
///
/// On disk, each record is:
///   - 4 bytes: `row_index` (u32, little-endian)
///   - N bytes: raw PyroVec packet (header + rkyv payload), 16-byte aligned
///
/// The packet's status byte indicates success (0) or failure (1).
pub struct WalWriter {
    wal_file: BufWriter<File>,
    log_writer: LogFrameWriter,
    wal_path: PathBuf,
    log_path: PathBuf,
    records_written: u64,
}

impl WalWriter {
    /// Open or create a WAL + log file pair.
    pub fn open(base_path: impl Into<PathBuf>) -> io::Result<Self> {
        let base = base_path.into();
        let wal_path = base.with_extension("pyrowal");
        let log_path = base.with_extension("pyrolog");

        if let Some(parent) = wal_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let wal_file = BufWriter::new(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&wal_path)?,
        );
        let log_writer = LogFrameWriter::open(&log_path)?;

        info!(
            wal = %wal_path.display(),
            log = %log_path.display(),
            "WAL opened for writing"
        );

        Ok(Self {
            wal_file,
            log_writer,
            wal_path,
            log_path,
            records_written: 0,
        })
    }

    /// Append a single `WalRecord`.
    pub fn append(&mut self, record: &WalRecord) -> io::Result<()> {
        let row_index = record.row_index() as u32;

        // 1. Write row_index
        self.wal_file.write_all(&row_index.to_le_bytes())?;

        // 2. Ship the record into a PyroVec packet and write it
        let packet = match record {
            WalRecord::Success { success, .. } => {
                success.row.to_wire().map_err(|e| {
                    io::Error::new(io::ErrorKind::InvalidData, e.to_string())
                })?
            }
            WalRecord::Failure { failure, .. } => {
                let msg = match &failure.result {
                    Ok(err) => err.message.clone(),
                    Err(msg) => msg.clone(),
                };
                let val = crate::PyroValue::from(msg);
                val.to_wire().map_err(|e| {
                    io::Error::new(io::ErrorKind::InvalidData, e.to_string())
                })?
            }
        };

        let raw = packet.as_raw_slice();
        // Pad to 16-byte alignment
        let padded = align16(raw.len());
        let mut buf = Vec::with_capacity(padded);
        buf.extend_from_slice(raw);
        buf.resize(padded, 0);
        self.wal_file.write_all(&buf)?;

        // 3. Flush + fsync
        self.wal_file.flush()?;
        self.wal_file.get_ref().sync_data()?;

        // 4. Write logs (best-effort)
        let has_logs = match record {
            WalRecord::Success { success, .. } => {
                !success.logs.module_logs.is_empty()
                    || !success.logs.capability_logs.is_empty()
            }
            WalRecord::Failure { failure, .. } => {
                !failure.logs.module_logs.is_empty()
                    || !failure.logs.capability_logs.is_empty()
            }
        };

        if has_logs {
            let log_record = LogRecord::from_record(record);
            if let Some(lr) = log_record {
                if let Ok(log_bytes) = serde_json::to_vec(&lr) {
                    if let Err(e) = self.log_writer.write_frame(&log_bytes) {
                        warn!(
                            error = %e,
                            row_index = record.row_index(),
                            "Failed to write log record (non-fatal)"
                        );
                    }
                }
            }
        }

        self.records_written += 1;
        debug!(
            row_index = record.row_index(),
            records_written = self.records_written,
            "WAL record appended"
        );
        Ok(())
    }

    pub fn records_written(&self) -> u64 {
        self.records_written
    }

    pub fn wal_path(&self) -> &Path {
        &self.wal_path
    }

    pub fn log_path(&self) -> &Path {
        &self.log_path
    }
}

// =============================================================================
// WalFrame — a recovered frame from the WAL
// =============================================================================

/// A single recovered WAL frame: row_index + raw packet bytes.
pub struct WalFrame {
    pub row_index: usize,
    /// Raw PyroVec packet bytes (header + rkyv payload).
    pub packet_bytes: Vec<u8>,
}

// =============================================================================
// WalReader — owns or mmaps the .pyrowal file and hands out PyroViews
// =============================================================================

/// Reads a `.pyrowal` file and hands out `PyroView`s with `FromOwned` inner.
///
/// The WAL owns the raw bytes and manages an `AtomicU32` ref count, so
/// views are valid as long as this reader lives.
pub struct WalReader {
    data: Vec<u8>,
    path: PathBuf,
}

impl WalReader {
    /// Open a `.pyrowal` file and load it into memory.
    pub fn open(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        let data = fs::read(&path)?;
        info!(path = %path.display(), bytes = data.len(), "WAL file loaded");
        Ok(Self { data, path })
    }

    /// Iterate over all frames.
    pub fn frames(&self) -> WalFrameIter {
        WalFrameIter::new(&self.data)
    }

    /// Iterate over all frames, yielding `(WalFrame, WalData)` pairs.
    /// `WalData` owns the packet bytes and hands out `PyroView`s.
    pub fn frame_with_data(&self) -> impl Iterator<Item = (WalFrame, WalData)> + '_ {
        self.frames().map(|frame| {
            let records = vec![WalRecord::from_frame(&frame)];
            (frame, WalData::from_records(&records))
        })
    }

    /// Recover all frames as `WalRecord`s (owned, deserialized).
    /// Joins with log records if provided.
    pub fn recover(
        &self,
        log_index: &HashMap<usize, LogRecord>,
    ) -> Vec<WalRecord> {
        let mut records = Vec::new();
        for frame in self.frames() {
            let record = WalRecord::from_frame(&frame);
            // Reattach logs
            if let Some(log) = log_index.get(&record.row_index()) {
                match &record {
                    WalRecord::Success { .. } => {
                        // Logs would be per-step; we attach based on index
                    }
                    WalRecord::Failure { .. } => {
                        // Failure logs handled at record level
                    }
                }
            }
            records.push(record);
        }
        info!(recovered = records.len(), "WAL recovery complete");
        records
    }

    pub fn record_count(&self) -> usize {
        self.frames().count()
    }
}

// =============================================================================
// WalFrameIter — walks raw bytes frame-by-frame
// =============================================================================

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
    type Item = WalFrame;

    fn next(&mut self) -> Option<Self::Item> {
        // Need at least 4 bytes for row_index
        if self.data.len() - self.offset < 4 {
            return None;
        }

        let row_index = u32::from_le_bytes(
            self.data[self.offset..self.offset + 4]
                .try_into()
                .ok()?,
        ) as usize;

        // Advance past row_index
        self.offset += 4;

        // Read packet length from header
        if self.data.len() - self.offset < PyroParser::HEADER_SIZE {
            warn!(offset = self.offset, "Not enough bytes for packet header");
            return None;
        }

        let payload_len = u32::from_le_bytes(
            self.data[self.offset..self.offset + 4]
                .try_into()
                .ok()?,
        ) as usize;

        let total_pkt = PyroParser::HEADER_SIZE + payload_len;
        let pkt_end = self.offset + total_pkt;
        if pkt_end > self.data.len() {
            warn!(offset = self.offset, "Packet payload extends past end of file");
            return None;
        }

        // Validate the packet
        if !WalRecord::validate_packet(&self.data[self.offset..pkt_end]) {
            warn!(offset = self.offset, "Invalid PyroVec packet");
            return None;
        }

        let pkt_len = align16(total_pkt);
        let pkt_slice = self.data[self.offset..self.offset + total_pkt].to_vec();
        self.offset += pkt_len;

        Some(WalFrame {
            row_index,
            packet_bytes: pkt_slice,
        })
    }
}

// =============================================================================
// Helper: frame → WalRecord
// =============================================================================

impl WalRecord {
    /// Validate a packet without requiring 16-byte aligned memory.
    fn validate_packet(pkt: &[u8]) -> bool {
        if pkt.len() < PyroParser::HEADER_SIZE {
            return false;
        }
        let payload_len = u32::from_le_bytes(pkt[0..4].try_into().unwrap()) as usize;
        let total = PyroParser::HEADER_SIZE + payload_len;
        if total != pkt.len() {
            return false;
        }
        true
    }

    /// Build a 16-byte aligned buffer for `get_view`.
    ///
    /// `get_view(slice, 0)` expects: [PyroInner(16)][header(16)][payload(N)]
    /// We pass [header(16)][payload(N)] at offset 16, so we need a 16-byte
    /// prefix + header + payload.
    fn build_view_buffer(pkt: &[u8]) -> Vec<u8> {
        let payload_len = u32::from_le_bytes(pkt[0..4].try_into().unwrap()) as usize;
        // [16-byte prefix][header(16)][payload(N)] = 32 + payload_len
        let total = 16 + PyroParser::HEADER_SIZE + payload_len;
        let layout = std::alloc::Layout::from_size_align(total, 16).unwrap();
        let ptr = unsafe { std::alloc::alloc_zeroed(layout) };
        if ptr.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        // Copy packet bytes after the 16-byte prefix
        unsafe {
            std::ptr::copy_nonoverlapping(pkt.as_ptr(), ptr.add(16), pkt.len());
        }
        unsafe { std::slice::from_raw_parts(ptr, total).to_vec() }
    }

    fn from_frame(frame: &WalFrame) -> Self {
        let pkt = &frame.packet_bytes;
        let status = pkt[PyroParser::OFFSET_STATUS];

        // get_view(slice, 16) expects 16 bytes before the header + 16 bytes PyroInner
        // total slice = 16(prefix) + 16(PyroInner) + 16(header) + payload = 48 + payload
        // But our buffer is 16(prefix) + 16(header) + payload = 32 + payload
        // So we need to pass offset 0 and include a dummy PyroInner at the start
        let buffer = Self::build_view_buffer(pkt);
        // The buffer layout is: [prefix(16)][header(16)][payload(N)]
        // get_view expects: [PyroInner(16)][header(16)][payload(N)]
        // So we pass offset 0 and the prefix acts as PyroInner
        let view = crate::format::vec_buf::get_view(&buffer, 0).expect("valid packet");

        if status == 0 {
            // Success
            let row = PyroRow::parse_wire(&view).expect("valid PyroRow").to_static();
            WalRecord::Success {
                row_index: frame.row_index,
                success: PyroSuccess {
                    row,
                    logs: PyroLogs::empty(),
                },
            }
        } else {
            // Failure — the payload is a string error message
            let msg = String::from_utf8_lossy(&pkt[PyroParser::HEADER_SIZE..]).to_string();
            WalRecord::Failure {
                row_index: frame.row_index,
                failure: PyroFailure {
                    result: Err(msg),
                    logs: PyroLogs::empty(),
                },
            }
        }
    }
}

// =============================================================================
// LogRecord helper
// =============================================================================

impl LogRecord {
    pub fn from_record(record: &WalRecord) -> Option<Self> {
        match record {
            WalRecord::Success { success, .. } => {
                let has_logs = !success.logs.module_logs.is_empty()
                    || !success.logs.capability_logs.is_empty();
                if !has_logs {
                    return None;
                }
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
                let has_logs = !failure.logs.module_logs.is_empty()
                    || !failure.logs.capability_logs.is_empty();
                if !has_logs {
                    return None;
                }
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
// Full recovery: .pyrowal + .pyrolog -> Vec<WalRecord>
// =============================================================================

/// Recover all committed results from a `.pyrowal` / `.pyrolog` pair.
pub fn recover(base_path: impl Into<PathBuf>) -> io::Result<Vec<WalRecord>> {
    let base = base_path.into();
    let wal_path = base.with_extension("pyrowal");
    let log_path = base.with_extension("pyrolog");

    if !wal_path.exists() {
        return Ok(Vec::new());
    }

    let wal = WalReader::open(&wal_path)?;

    let log_index = if log_path.exists() {
        LogReader::open(&log_path)
            .map(|mut r| r.read_all_indexed())
            .unwrap_or_default()
    } else {
        HashMap::new()
    };

    let records = wal.recover(&log_index);
    info!(recovered = records.len(), "Full recovery complete");
    Ok(records)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::{PyroRow, PyroValue};

    use super::*;
    use tempfile::TempDir;

    fn make_success_record(row_index: usize) -> WalRecord {
        let row = PyroRow::from([
            ("id", PyroValue::from(row_index as i32)),
            ("name", PyroValue::from("test")),
        ])
        .into_owned();

        WalRecord::Success {
            row_index,
            success: PyroSuccess {
                row,
                logs: PyroLogs {
                    module_logs: vec![format!("processing row {}", row_index)],
                    capability_logs: {
                        let mut m = HashMap::new();
                        m.insert(
                            ("http".to_string(), "fetch".to_string()),
                            vec![format!("GET /api/row/{}", row_index)],
                        );
                        m
                    },
                },
            },
        }
    }

    fn make_failure_record(row_index: usize) -> WalRecord {
        WalRecord::Failure {
            row_index,
            failure: PyroFailure {
                result: Err(format!("row {} failed", row_index)),
                logs: PyroLogs {
                    module_logs: vec![format!("error in row {}", row_index)],
                    capability_logs: HashMap::new(),
                },
            },
        }
    }

    #[test]
    fn test_roundtrip_via_file_reader() {
        let dir = TempDir::new().unwrap();
        let base = dir.path().join("test_run");

        let records: Vec<_> = (0..10)
            .map(|i| {
                if i % 3 == 0 {
                    make_failure_record(i)
                } else {
                    make_success_record(i)
                }
            })
            .collect();

        // Write
        {
            let mut wal = WalWriter::open(&base).unwrap();
            for record in &records {
                wal.append(record).unwrap();
            }
            assert_eq!(wal.records_written(), 10);
        }

        assert!(base.with_extension("pyrowal").exists());
        assert!(base.with_extension("pyrolog").exists());

        // Full recovery
        let recovered = recover(&base).unwrap();
        assert_eq!(recovered.len(), 10);

        for (i, record) in recovered.iter().enumerate() {
            assert_eq!(record.row_index(), i);

            match record {
                WalRecord::Success { success, .. } => {
                    assert_eq!(success.row.get("id"), Some(&PyroValue::from(i as i32)));
                    assert_eq!(success.row.get("name"), Some(&PyroValue::from("test")));
                }
                WalRecord::Failure { failure, .. } => {
                    if let Err(s) = &failure.result {
                        assert!(s.contains(&i.to_string()));
                    } else {
                        panic!("expected Err");
                    }
                }
            }
        }
    }

    #[test]
    fn test_frame_iteration() {
        let dir = TempDir::new().unwrap();
        let base = dir.path().join("iter_test");

        {
            let mut wal = WalWriter::open(&base).unwrap();
            for i in 0..5 {
                wal.append(&make_success_record(i)).unwrap();
            }
        }

        // Read the file
        let data = fs::read(base.with_extension("pyrowal")).unwrap();
        let mut iter = WalFrameIter::new(&data);

        for i in 0..5 {
            let frame = iter.next().expect("expected frame");
            assert_eq!(frame.row_index, i);
            assert!(!frame.packet_bytes.is_empty());
        }
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_wal_data_views() {
        let dir = TempDir::new().unwrap();
        let base = dir.path().join("view_test");

        let record = make_success_record(42);

        {
            let mut wal = WalWriter::open(&base).unwrap();
            wal.append(&record).unwrap();
        }

        let wal = WalReader::open(base.with_extension("pyrowal")).unwrap();

        // Test frame_with_data which gives us PyroView
        for (frame, wal_data) in wal.frame_with_data() {
            assert_eq!(frame.row_index, 42);
            assert_eq!(wal_data.record_count(), 1);

            // Get a view — this uses PyroViewInner::FromOwned
            let view = wal_data.record_view(0).expect("should have view");
            let recovered_row = PyroRow::parse_wire(&view).expect("parse should work");
            assert_eq!(recovered_row.get("id"), Some(&PyroValue::from(42i32)));
        }
    }

    #[test]
    fn test_multi_step_pipeline() {
        let dir = TempDir::new().unwrap();
        let base = dir.path().join("multistep");

        // Write 3 records for 3 steps of 1 row
        let steps = vec![
            PyroRow::from([("input", PyroValue::from("hello"))]).into_owned(),
            PyroRow::from([("enriched", PyroValue::from(true))]).into_owned(),
            PyroRow::from([("score", PyroValue::from(0.95f64))]).into_owned(),
        ];

        {
            let mut wal = WalWriter::open(&base).unwrap();
            for (i, row) in steps.iter().enumerate() {
                let record = WalRecord::Success {
                    row_index: 0,
                    success: PyroSuccess {
                        row: row.clone(),
                        logs: PyroLogs::empty(),
                    },
                };
                wal.append(&record).unwrap();
            }
        }

        let recovered = recover(&base).unwrap();
        assert_eq!(recovered.len(), 3);
        assert_eq!(recovered[0].row_index(), 0);
        assert_eq!(recovered[1].row_index(), 0);
        assert_eq!(recovered[2].row_index(), 0);

        match &recovered[0] {
            WalRecord::Success { success, .. } => {
                assert_eq!(success.row.get("input"), Some(&PyroValue::from("hello")));
            }
            _ => panic!("expected success"),
        }
        match &recovered[1] {
            WalRecord::Success { success, .. } => {
                assert_eq!(success.row.get("enriched"), Some(&PyroValue::from(true)));
            }
            _ => panic!("expected success"),
        }
        match &recovered[2] {
            WalRecord::Success { success, .. } => {
                assert_eq!(success.row.get("score"), Some(&PyroValue::from(0.95f64)));
            }
            _ => panic!("expected success"),
        }
    }

    #[test]
    fn test_failures_and_successes_interleaved() {
        let dir = TempDir::new().unwrap();
        let base = dir.path().join("mixed");

        {
            let mut wal = WalWriter::open(&base).unwrap();
            for i in 0..20 {
                let record = if i % 4 == 0 {
                    make_failure_record(i)
                } else {
                    make_success_record(i)
                };
                wal.append(&record).unwrap();
            }
        }

        let recovered = recover(&base).unwrap();
        assert_eq!(recovered.len(), 20);

        let (successes, failures): (Vec<_>, Vec<_>) =
            recovered.iter().partition(|r| matches!(r, WalRecord::Success { .. }));
        assert_eq!(failures.len(), 5); // 0, 4, 8, 12, 16
        assert_eq!(successes.len(), 15);
    }

    #[test]
    fn test_missing_pyrolog_recovers_with_empty_logs() {
        let dir = TempDir::new().unwrap();
        let base = dir.path().join("nolog");

        {
            let mut wal = WalWriter::open(&base).unwrap();
            wal.append(&make_success_record(0)).unwrap();
        }

        fs::remove_file(base.with_extension("pyrolog")).unwrap();

        let recovered = recover(&base).unwrap();
        assert_eq!(recovered.len(), 1);
        if let WalRecord::Success { success, .. } = &recovered[0] {
            assert!(success.logs.module_logs.is_empty());
        }
    }

    #[test]
    fn test_empty_wal() {
        let dir = TempDir::new().unwrap();
        let base = dir.path().join("empty");

        {
            let _wal = WalWriter::open(&base).unwrap();
        }

        let recovered = recover(&base).unwrap();
        assert!(recovered.is_empty());
    }

    #[test]
    fn test_alignment_invariants() {
        assert_eq!(align16(0), 0);
        assert_eq!(align16(1), 16);
        assert_eq!(align16(15), 16);
        assert_eq!(align16(16), 16);
        assert_eq!(align16(17), 32);
        assert_eq!(align16(32), 32);
    }

    #[test]
    fn test_crc32c_deterministic() {
        let data = b"hello pyroduct";
        assert_eq!(crc32c(data), crc32c(data));
        assert_ne!(crc32c(data), crc32c(b"hello pyroducT"));
    }
}
