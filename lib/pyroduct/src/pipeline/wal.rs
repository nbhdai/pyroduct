//! Write-Ahead Log for streaming `PipelineExecution` results to disk.
//!
//! Results are split across two files:
//!
//! - **`.pyrowal`** — row data as raw `PyroVec` packet bytes (rkyv zero-copy
//!   friendly) plus a small metadata envelope per record.
//! - **`.pyrolog`** — module and capability logs (JSON, best-effort).
//!
//! # `.pyrowal` on-disk frame format
//!
//! Each record is a contiguous frame. All multi-byte integers are little-endian.
//!
//! ```text
//! ┌──────────────────────── Frame ────────────────────────┐
//! │ meta_len : u32     ← byte length of the JSON envelope │
//! │ meta_crc : u32     ← CRC-32C of the JSON envelope     │
//! │ meta     : [u8]    ← JSON-encoded WalMeta              │
//! │ padding  : [u8]    ← 0..15 bytes to reach 16-byte align│
//! │ packet₀  : [u8]    ← raw PyroVec packet (header+data)  │
//! │ packet₁  : [u8]    ← …one per step                     │
//! │ …                                                       │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! The `WalMeta` envelope stores `row_index`, per-step packet sizes, and
//! optional failure metadata. On recovery, a memory-mapped `.pyrowal` file
//! can be walked frame-by-frame; each `packetₙ` is a valid PyroVec packet
//! that can be handed directly to `get_view()` / `expose_view()` for
//! **zero-copy** access to the archived `PyroRow`.
//!
//! # Recovery
//!
//! ```rust,ignore
//! // Zero-copy recovery via mmap
//! let mmap = unsafe { memmap2::Mmap::map(&file)? };
//! let executions = WalMmapReader::new(&mmap).recover();
//!
//! // Owned recovery (no mmap)
//! let executions = recover("path/to/run")?;
//! ```

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::format::PyroVec;
use crate::format::PyroView;
use crate::format::header::PyroParser;
use crate::module::{PyroFailure, PyroLogs, PyroSuccess};
use crate::{CapturedError, PyroRow};

use super::wasm_execute::PipelineExecution;

// =============================================================================
// WAL metadata envelope (small JSON per frame)
// =============================================================================

/// The JSON metadata stored at the start of each `.pyrowal` frame.
///
/// This is deliberately small — the bulk of the data lives in the raw
/// PyroVec packets that follow it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalMeta {
    /// Original row index in the input batch.
    pub row_index: usize,
    /// Byte length of each step's PyroVec packet (in order).
    /// `packet_sizes[i]` = `HEADER_SIZE + rkyv_payload_len` for step `i`.
    pub packet_sizes: Vec<usize>,
    /// Failure metadata, if the row did not complete the pipeline.
    pub failure: Option<WalFailure>,
}

/// Failure metadata stored in the WAL envelope (no logs — those go to `.pyrolog`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalFailure {
    /// `Ok(CapturedError)` for user-level module errors,
    /// `Err(String)` for infrastructure / pyroduct errors.
    pub result: Result<CapturedError, String>,
}

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

impl LogRecord {
    pub fn from_execution(exec: &PipelineExecution) -> Option<Self> {
        let step_logs: Vec<LogEntry> = exec
            .steps
            .iter()
            .map(|s| LogEntry {
                module_logs: s.logs.module_logs.clone(),
                capability_logs: s.logs.capability_logs.clone(),
            })
            .collect();

        let failure_logs = exec.failure.as_ref().map(|f| LogEntry {
            module_logs: f.logs.module_logs.clone(),
            capability_logs: f.logs.capability_logs.clone(),
        });

        let has_any = step_logs
            .iter()
            .any(|l| !l.module_logs.is_empty() || !l.capability_logs.is_empty())
            || failure_logs.as_ref().map_or(false, |l| {
                !l.module_logs.is_empty() || !l.capability_logs.is_empty()
            });

        if !has_any {
            return None;
        }

        Some(LogRecord {
            row_index: exec.row_index,
            step_logs,
            failure_logs,
        })
    }
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
// Log file writer/reader (JSON framed, unchanged from before)
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
// WalWriter — writes .pyrowal + .pyrolog
// =============================================================================

/// Streams `PipelineExecution` results into a `.pyrowal` / `.pyrolog` pair.
///
/// Each step's `PyroRow` is rkyv-serialized via `Bridgeable::ship()` and
/// written as a raw PyroVec packet (16-byte header + payload). The packets
/// are laid out at 16-byte aligned offsets so that an mmap reader can hand
/// them directly to `get_view()` for zero-copy access.
pub struct WalWriter {
    wal_file: BufWriter<File>,
    log_writer: LogFrameWriter,
    wal_path: PathBuf,
    log_path: PathBuf,
    records_written: u64,
}

impl WalWriter {
    /// Open or create a WAL + log file pair.
    ///
    /// Given a base path like `/tmp/run`, this creates:
    /// - `/tmp/run.pyrowal` (rkyv packets, fsync'd)
    /// - `/tmp/run.pyrolog` (JSON logs, best-effort)
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

    /// Append a `PipelineExecution` result.
    ///
    /// Row data is rkyv-serialized and written as raw PyroVec packets.
    /// Logs go to the `.pyrolog` sidecar file.
    pub fn append(&mut self, execution: &PipelineExecution) -> io::Result<()> {
        // 1. Ship each step's row into a PyroVec
        let packets: Vec<PyroVec> = execution
            .steps
            .iter()
            .map(|s| {
                s.row
                    .to_wire()
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
            })
            .collect::<io::Result<Vec<_>>>()?;

        let packet_sizes: Vec<usize> = packets
            .iter()
            .map(|p| PyroParser::HEADER_SIZE + p.len())
            .collect();

        // 2. Build the metadata envelope
        let meta = WalMeta {
            row_index: execution.row_index,
            packet_sizes: packet_sizes.clone(),
            failure: execution.failure.as_ref().map(|f| WalFailure {
                result: f.result.clone(),
            }),
        };
        let meta_bytes =
            serde_json::to_vec(&meta).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let meta_crc = crc32c(&meta_bytes);

        // 3. Write: [meta_len: u32][meta_crc: u32][meta bytes][padding to 16]
        self.wal_file
            .write_all(&(meta_bytes.len() as u32).to_le_bytes())?;
        self.wal_file.write_all(&meta_crc.to_le_bytes())?;
        self.wal_file.write_all(&meta_bytes)?;

        // Pad so that the first packet starts on a 16-byte boundary.
        // The meta prefix is 4 + 4 + meta_bytes.len() = 8 + meta_bytes.len().
        let meta_total = 8 + meta_bytes.len();
        let padded = align16(meta_total);
        let pad_len = padded - meta_total;
        if pad_len > 0 {
            self.wal_file.write_all(&vec![0u8; pad_len])?;
        }

        // 4. Write each PyroVec packet (already internally 16-byte aligned data)
        for packet in &packets {
            let raw = packet.as_packet_slice();
            self.wal_file.write_all(raw)?;
            // Pad each packet to 16-byte alignment so next packet is aligned
            let pkt_pad = align16(raw.len()) - raw.len();
            if pkt_pad > 0 {
                self.wal_file.write_all(&vec![0u8; pkt_pad])?;
            }
        }

        // 5. Flush + fsync the WAL
        self.wal_file.flush()?;
        self.wal_file.get_ref().sync_data()?;

        // 6. Write logs (best-effort)
        if let Some(log_record) = LogRecord::from_execution(execution) {
            if let Ok(log_bytes) = serde_json::to_vec(&log_record) {
                if let Err(e) = self.log_writer.write_frame(&log_bytes) {
                    warn!(
                        error = %e,
                        row_index = execution.row_index,
                        "Failed to write log record (non-fatal)"
                    );
                }
            }
        }

        self.records_written += 1;
        debug!(
            row_index = execution.row_index,
            records_written = self.records_written,
            failed = execution.failure.is_some(),
            steps = execution.steps.len(),
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
// WalMmapReader — zero-copy recovery from a memory-mapped .pyrowal
// =============================================================================

/// Walks a memory-mapped `.pyrowal` file and yields `(WalMeta, &[&[u8]])`
/// tuples where each inner slice is a raw PyroVec packet that can be
/// passed to `get_view()` / `PyroRowOwned::expose_view()` without copying.
pub struct WalMmapReader<'a> {
    data: &'a [u8],
    offset: usize,
    records_read: u64,
}

/// A single recovered WAL frame referencing borrowed packet data.
pub struct WalFrame<'a> {
    pub meta: WalMeta,
    /// Raw PyroVec packet slices (header + rkyv payload) — borrow from the mmap.
    pub packets: Vec<&'a [u8]>,
}

impl<'a> WalMmapReader<'a> {
    /// Create a reader over a memory-mapped `.pyrowal` file.
    ///
    /// # Safety
    /// The caller must ensure `data` remains valid for the lifetime `'a`.
    /// Typically `data` is `&Mmap` from `memmap2`.
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            offset: 0,
            records_read: 0,
        }
    }

    /// Read the next frame. Returns `None` on EOF or corruption.
    pub fn next_frame(&mut self) -> Option<WalFrame<'a>> {
        let remaining = self.data.len().checked_sub(self.offset)?;
        if remaining < 8 {
            return None; // Not enough room for meta_len + meta_crc
        }

        // Read meta_len
        let meta_len =
            u32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().ok()?) as usize;
        // Read meta_crc
        let meta_crc = u32::from_le_bytes(
            self.data[self.offset + 4..self.offset + 8]
                .try_into()
                .ok()?,
        );

        // Sanity
        if meta_len > 16 * 1024 * 1024 {
            warn!(meta_len, offset = self.offset, "WAL meta length too large");
            return None;
        }

        let meta_start = self.offset + 8;
        let meta_end = meta_start + meta_len;
        if meta_end > self.data.len() {
            return None;
        }

        // CRC check
        let meta_bytes = &self.data[meta_start..meta_end];
        if crc32c(meta_bytes) != meta_crc {
            warn!(offset = self.offset, "WAL meta CRC mismatch");
            return None;
        }

        // Deserialize meta
        let meta: WalMeta = serde_json::from_slice(meta_bytes).ok()?;

        // Advance past meta + padding to 16-byte alignment
        let meta_total = 8 + meta_len;
        let mut cursor = self.offset + align16(meta_total);

        // Read packets
        let mut packets = Vec::with_capacity(meta.packet_sizes.len());
        for &pkt_size in &meta.packet_sizes {
            let pkt_end = cursor + pkt_size;
            if pkt_end > self.data.len() {
                warn!(
                    offset = cursor,
                    pkt_size, "WAL packet extends past end of file"
                );
                return None;
            }

            let pkt_slice = &self.data[cursor..pkt_end];

            // Validate the PyroVec header is intact
            if PyroParser::check(pkt_slice).is_err() {
                warn!(offset = cursor, "WAL packet has invalid PyroVec header");
                return None;
            }

            packets.push(pkt_slice);

            // Advance past packet + alignment padding
            cursor += align16(pkt_size);
        }

        self.offset = cursor;
        self.records_read += 1;
        Some(WalFrame { meta, packets })
    }

    /// Read all valid frames.
    pub fn read_all(&mut self) -> Vec<WalFrame<'a>> {
        let mut out = Vec::new();
        while let Some(frame) = self.next_frame() {
            out.push(frame);
        }
        info!(records = out.len(), "WAL mmap read complete");
        out
    }

    /// Recover all frames as `PipelineExecution`s (owned, deserialized).
    /// Joins with log records if provided.
    pub fn recover_owned(
        &mut self,
        log_index: &HashMap<usize, LogRecord>,
    ) -> Vec<PipelineExecution> {
        let mut executions = Vec::new();
        for frame in self.read_all() {
            if let Some(exec) = frame_to_execution(frame, log_index) {
                executions.push(exec);
            }
        }
        executions.sort_by_key(|e| e.row_index);
        executions
    }

    pub fn records_read(&self) -> u64 {
        self.records_read
    }
}

/// Convert a `WalFrame` (borrowed packet data) into an owned `PipelineExecution`
/// by deserializing each packet via the `Bridgeable` rkyv path.
fn frame_to_execution(
    frame: WalFrame<'_>,
    log_index: &HashMap<usize, LogRecord>,
) -> Option<PipelineExecution> {
    let log = log_index.get(&frame.meta.row_index);

    let steps: Vec<PyroSuccess> = frame
        .packets
        .iter()
        .enumerate()
        .filter_map(|(i, &pkt_slice)| {
            let view = PyroView::try_from(pkt_slice).ok()?;
            let row = PyroRow::parse_wire(view).ok()?.to_static();

            let logs = log
                .and_then(|l| l.step_logs.get(i))
                .map(|entry| PyroLogs {
                    module_logs: entry.module_logs.clone(),
                    capability_logs: entry.capability_logs.clone(),
                })
                .unwrap_or_else(PyroLogs::empty);

            Some(PyroSuccess { row, logs })
        })
        .collect();

    let failure = frame.meta.failure.map(|f| {
        let logs = log
            .and_then(|l| l.failure_logs.as_ref())
            .map(|entry| PyroLogs {
                module_logs: entry.module_logs.clone(),
                capability_logs: entry.capability_logs.clone(),
            })
            .unwrap_or_else(PyroLogs::empty);
        PyroFailure {
            result: f.result,
            logs,
        }
    });

    Some(PipelineExecution {
        row_index: frame.meta.row_index,
        steps,
        failure,
    })
}

// =============================================================================
// WalFileReader — owned recovery without mmap
// =============================================================================

/// Reads a `.pyrowal` file into memory and recovers records.
/// Use `WalMmapReader` for zero-copy access instead.
pub struct WalFileReader {
    data: Vec<u8>,
}

impl WalFileReader {
    pub fn open(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        let data = fs::read(&path)?;
        info!(path = %path.display(), bytes = data.len(), "WAL file loaded");
        Ok(Self { data })
    }

    pub fn read_all(&self) -> Vec<WalFrame<'_>> {
        let mut reader = WalMmapReader::new(&self.data);
        reader.read_all()
    }

    pub fn recover(&self, log_index: &HashMap<usize, LogRecord>) -> Vec<PipelineExecution> {
        let mut reader = WalMmapReader::new(&self.data);
        reader.recover_owned(log_index)
    }
}

// =============================================================================
// Full recovery: .pyrowal + .pyrolog -> Vec<PipelineExecution>
// =============================================================================

/// Recover all committed results from a `.pyrowal` / `.pyrolog` pair.
///
/// This loads the WAL into memory. For zero-copy recovery, mmap the
/// `.pyrowal` file and use `WalMmapReader` directly.
pub fn recover(base_path: impl Into<PathBuf>) -> io::Result<Vec<PipelineExecution>> {
    let base = base_path.into();
    let wal_path = base.with_extension("pyrowal");
    let log_path = base.with_extension("pyrolog");

    if !wal_path.exists() {
        return Ok(Vec::new());
    }

    let wal = WalFileReader::open(&wal_path)?;

    let log_index = if log_path.exists() {
        LogReader::open(&log_path)
            .map(|mut r| r.read_all_indexed())
            .unwrap_or_default()
    } else {
        HashMap::new()
    };

    let executions = wal.recover(&log_index);
    info!(recovered = executions.len(), "Full recovery complete");
    Ok(executions)
}

// =============================================================================
// Convenience: streaming batch with WAL
// =============================================================================

/// Wraps `PipelinePool::process_batch` to stream results into a WAL.
///
/// Both successes and failures are recorded. The pipeline continues past
/// individual row failures — only infrastructure-level errors bubble up.
pub async fn process_batch_with_wal(
    pool: &super::PipelinePool,
    batch: &arrow::array::RecordBatch,
    base_path: impl Into<PathBuf>,
) -> super::PipelineResult<(Vec<PipelineExecution>, Vec<PipelineExecution>)> {
    let base = base_path.into();
    let wal_path = base.with_extension("pyrowal");

    // Check for existing WAL (crash recovery)
    if wal_path.exists() {
        info!(path = %wal_path.display(), "Found existing WAL, attempting recovery");
        match recover(&base) {
            Ok(executions) if executions.len() >= batch.num_rows() => {
                info!("WAL contains all expected rows, skipping re-execution");
                let (successes, failures): (Vec<_>, Vec<_>) =
                    executions.into_iter().partition(|e| e.failure.is_none());
                return Ok((successes, failures));
            }
            Ok(partial) => {
                warn!(
                    recovered = partial.len(),
                    expected = batch.num_rows(),
                    "Partial WAL recovery — re-executing full batch"
                );
                let _ = fs::remove_file(&wal_path);
                let _ = fs::remove_file(base.with_extension("pyrolog"));
            }
            Err(e) => {
                warn!(error = %e, "WAL recovery failed, starting fresh");
                let _ = fs::remove_file(&wal_path);
                let _ = fs::remove_file(base.with_extension("pyrolog"));
            }
        }
    }

    let mut wal = WalWriter::open(&base)
        .map_err(|e| super::PipelineError::Config(format!("Failed to open WAL: {}", e)))?;

    let (successes, failures) = pool.process_batch(batch).await?;

    for exec in successes.iter().chain(failures.iter()) {
        if let Err(e) = wal.append(exec) {
            warn!(error = %e, row_index = exec.row_index, "Failed to write WAL record");
        }
    }

    info!(
        successes = successes.len(),
        failures = failures.len(),
        wal = %wal.wal_path().display(),
        "Batch complete, WAL committed"
    );

    Ok((successes, failures))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::{PyroRow, PyroValue};

    use super::*;
    use tempfile::TempDir;

    fn make_execution(row_index: usize, failed: bool) -> PipelineExecution {
        let row = PyroRow::from([
            ("id", PyroValue::from(row_index as i32)),
            ("name", PyroValue::from("test")),
        ])
        .into_owned();

        let step = PyroSuccess {
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
        };

        let failure = if failed {
            Some(PyroFailure {
                result: Err(format!("row {} failed", row_index)),
                logs: PyroLogs {
                    module_logs: vec![format!("error in row {}", row_index)],
                    capability_logs: HashMap::new(),
                },
            })
        } else {
            None
        };

        PipelineExecution {
            row_index,
            steps: vec![step],
            failure,
        }
    }

    #[test]
    fn test_roundtrip_via_file_reader() {
        let dir = TempDir::new().unwrap();
        let base = dir.path().join("test_run");

        let executions: Vec<_> = (0..10).map(|i| make_execution(i, i % 3 == 0)).collect();

        // Write
        {
            let mut wal = WalWriter::open(&base).unwrap();
            for exec in &executions {
                wal.append(exec).unwrap();
            }
            assert_eq!(wal.records_written(), 10);
        }

        assert!(base.with_extension("pyrowal").exists());
        assert!(base.with_extension("pyrolog").exists());

        // Full recovery
        let recovered = recover(&base).unwrap();
        assert_eq!(recovered.len(), 10);

        for (i, exec) in recovered.iter().enumerate() {
            assert_eq!(exec.row_index, i);

            // Verify row data survived the rkyv roundtrip
            let row = &exec.steps[0].row;
            assert_eq!(row.get("id"), Some(&PyroValue::from(i as i32)));
            assert_eq!(row.get("name"), Some(&PyroValue::from("test")));

            // Verify failure status
            if i % 3 == 0 {
                assert!(exec.failure.is_some());
            } else {
                assert!(exec.failure.is_none());
            }
        }

        // Verify logs were reattached
        let row0 = &recovered[0];
        assert_eq!(row0.steps[0].logs.module_logs, vec!["processing row 0"]);
    }

    #[test]
    fn test_mmap_reader_zero_copy() {
        let dir = TempDir::new().unwrap();
        let base = dir.path().join("mmap_test");

        // Write 5 records
        {
            let mut wal = WalWriter::open(&base).unwrap();
            for i in 0..5 {
                wal.append(&make_execution(i, false)).unwrap();
            }
        }

        // Read the file into memory (simulating mmap)
        let data = fs::read(base.with_extension("pyrowal")).unwrap();

        // Use the mmap reader
        let mut reader = WalMmapReader::new(&data);
        let frames = reader.read_all();
        assert_eq!(frames.len(), 5);

        // Verify we can get zero-copy PyroViews from the packets
        for (i, frame) in frames.iter().enumerate() {
            assert_eq!(frame.meta.row_index, i);
            assert_eq!(frame.packets.len(), 1);

            // This is the zero-copy path: packet slice -> PyroView -> expose_view
            let pkt = frame.packets[0];
            let view = PyroView::try_from(pkt).expect("valid PyroView from WAL packet");
            let row = PyroRow::parse_wire(view).expect("expose_view should succeed on WAL packet");

            assert_eq!(row.get("id"), Some(&PyroValue::from(i as i32)));
        }
    }

    #[test]
    fn test_corrupt_wal_stops_cleanly() {
        let dir = TempDir::new().unwrap();
        let base = dir.path().join("corrupt");

        {
            let mut wal = WalWriter::open(&base).unwrap();
            for i in 0..5 {
                wal.append(&make_execution(i, false)).unwrap();
            }
        }

        // Append garbage
        {
            let mut f = OpenOptions::new()
                .append(true)
                .open(base.with_extension("pyrowal"))
                .unwrap();
            f.write_all(&[0xDE, 0xAD, 0xBE, 0xEF, 0xFF, 0xFF]).unwrap();
        }

        let recovered = recover(&base).unwrap();
        assert_eq!(recovered.len(), 5);
    }

    #[test]
    fn test_missing_pyrolog_recovers_with_empty_logs() {
        let dir = TempDir::new().unwrap();
        let base = dir.path().join("nolog");

        {
            let mut wal = WalWriter::open(&base).unwrap();
            wal.append(&make_execution(0, false)).unwrap();
        }

        fs::remove_file(base.with_extension("pyrolog")).unwrap();

        let recovered = recover(&base).unwrap();
        assert_eq!(recovered.len(), 1);
        // Logs are empty because .pyrolog is missing
        assert!(recovered[0].steps[0].logs.module_logs.is_empty());
    }

    #[test]
    fn test_failures_and_successes_interleaved() {
        let dir = TempDir::new().unwrap();
        let base = dir.path().join("mixed");

        {
            let mut wal = WalWriter::open(&base).unwrap();
            for i in 0..20 {
                wal.append(&make_execution(i, i % 4 == 0)).unwrap();
            }
        }

        let recovered = recover(&base).unwrap();
        assert_eq!(recovered.len(), 20);

        let (successes, failures): (Vec<_>, Vec<_>) =
            recovered.iter().partition(|e| e.failure.is_none());
        assert_eq!(failures.len(), 5); // 0, 4, 8, 12, 16
        assert_eq!(successes.len(), 15);
    }

    #[test]
    fn test_multi_step_pipeline() {
        let dir = TempDir::new().unwrap();
        let base = dir.path().join("multistep");

        let exec = PipelineExecution {
            row_index: 0,
            steps: vec![
                PyroSuccess {
                    row: PyroRow::from([("input", PyroValue::from("hello"))]).into_owned(),
                    logs: PyroLogs::empty(),
                },
                PyroSuccess {
                    row: PyroRow::from([("enriched", PyroValue::from(true))]).into_owned(),
                    logs: PyroLogs {
                        module_logs: vec!["enrichment complete".into()],
                        capability_logs: HashMap::new(),
                    },
                },
                PyroSuccess {
                    row: PyroRow::from([("score", PyroValue::from(0.95f64))]).into_owned(),
                    logs: PyroLogs::empty(),
                },
            ],
            failure: None,
        };

        {
            let mut wal = WalWriter::open(&base).unwrap();
            wal.append(&exec).unwrap();
        }

        let recovered = recover(&base).unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].steps.len(), 3);

        assert_eq!(
            recovered[0].steps[0].row.get("input"),
            Some(&PyroValue::from("hello"))
        );
        assert_eq!(
            recovered[0].steps[1].row.get("enriched"),
            Some(&PyroValue::from(true))
        );
        assert_eq!(
            recovered[0].steps[2].row.get("score"),
            Some(&PyroValue::from(0.95f64))
        );
    }

    #[test]
    fn test_captured_error_in_failure() {
        let dir = TempDir::new().unwrap();
        let base = dir.path().join("caperr");

        let exec = PipelineExecution {
            row_index: 42,
            steps: vec![PyroSuccess {
                row: PyroRow::from([("x", PyroValue::from(1i32))]).into_owned(),
                logs: PyroLogs::empty(),
            }],
            failure: Some(PyroFailure {
                result: Ok(CapturedError::new("module panicked")),
                logs: PyroLogs {
                    module_logs: vec!["trace: entered fn".into()],
                    capability_logs: {
                        let mut m = HashMap::new();
                        m.insert(
                            ("db".to_string(), "query".to_string()),
                            vec!["SELECT * FROM t".into()],
                        );
                        m
                    },
                },
            }),
        };

        {
            let mut wal = WalWriter::open(&base).unwrap();
            wal.append(&exec).unwrap();
        }

        let recovered = recover(&base).unwrap();
        let rec = &recovered[0];
        assert_eq!(rec.row_index, 42);

        let fail = rec.failure.as_ref().unwrap();
        assert!(
            fail.result
                .as_ref()
                .unwrap()
                .message
                .contains("module panicked")
        );
        assert_eq!(fail.logs.module_logs, vec!["trace: entered fn"]);
        assert!(
            fail.logs
                .capability_logs
                .contains_key(&("db".to_string(), "query".to_string()))
        );
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
        // Verify that align16 works correctly
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
