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

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU32, Ordering};

use tracing::{info, warn};

use crate::format::vec_buf::PyroRef;
use crate::format::{PyroView, get_ref};

// =============================================================================
// WAL Writer Trait
// =============================================================================

pub trait WalWriterInner: Write {
    fn sync_data(&self) -> io::Result<()>;
}

impl WalWriterInner for File {
    fn sync_data(&self) -> io::Result<()> {
        self.sync_data()
    }
}

impl WalWriterInner for Vec<u8> {
    fn sync_data(&self) -> io::Result<()> {
        Ok(())
    }
}

// =============================================================================
// Log record (.pyrolog) — per-row logs, JSON framed
// =============================================================================



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
// WalWriter
// =============================================================================

pub struct WalWriter<W: WalWriterInner> {
    wal_writer: BufWriter<W>,
    wal_path: Option<PathBuf>,
    records_written: u64,
}

impl<W: WalWriterInner> WalWriter<W> {
    pub fn new(wal_writer: W) -> Self {
        Self {
            wal_writer: BufWriter::new(wal_writer),
            wal_path: None,
            records_written: 0,
        }
    }

    pub fn append(&mut self, record_index: usize, record: PyroRef<'_>) -> io::Result<()> {
        let row_index = record_index as u32;

        // 1. Write 16-byte prefix [row_index (4) | padding (12)]
        // This ensures the packet that follows is naturally 16-byte aligned.
        let mut prefix = [0u8; 16];
        prefix[0..4].copy_from_slice(&row_index.to_le_bytes());
        self.wal_writer.write_all(&prefix)?;

        let raw_slice = record.as_raw_slice();
        let raw_len = raw_slice.len();
        let padded_len = align16(raw_len);
        let padding_len = padded_len - raw_len;

        self.wal_writer.write_all(raw_slice)?;
        if padding_len > 0 {
            let padding = [0u8; 15];
            self.wal_writer.write_all(&padding[..padding_len])?;
        }

        self.wal_writer.flush()?;
        self.wal_writer.get_ref().sync_data()?;


        self.records_written += 1;
        Ok(())
    }

    pub fn records_written(&self) -> u64 {
        self.records_written
    }
    pub fn wal_path(&self) -> Option<&Path> {
        self.wal_path.as_deref()
    }

    pub fn into_inner(self) -> W {
        let w = match self.wal_writer.into_inner() {
            Ok(inner) => inner,
            Err(e) => panic!("failed to flush wal writer: {e}"),
        };
        w
    }
}

impl WalWriter<File> {
    pub fn open(base_path: impl Into<PathBuf>) -> io::Result<Self> {
        let base = base_path.into();
        let wal_path = base.with_extension("pyrowal");

        if let Some(parent) = wal_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let wal_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&wal_path)?;

        info!(wal = %wal_path.display(), "WAL opened for writing");

        Ok(Self {
            wal_writer: BufWriter::new(wal_file),
            wal_path: Some(wal_path),
            records_written: 0,
        })
    }
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
// WalReader (The single source of truth)
// =============================================================================

/// Reads a `.pyrowal` file/buffer and acts as the master owner of the memory.
///
/// It hands out zero-copy `PyroRef`s or explicitly tracked `PyroView`s.
pub struct WalReader {
    inner: NonNull<WalInner>,
    pub path: Option<PathBuf>,
}

unsafe impl Send for WalReader {}
unsafe impl Sync for WalReader {}

impl WalReader {
    /// Memory maps the WAL file from disk and loads any optional companion logs.
    pub fn open(base_path: impl Into<PathBuf>) -> io::Result<Self> {
        let base = base_path.into();
        let wal_path = base.with_extension("pyrowal");

        let file = File::open(&wal_path)?;
        let mmap = unsafe { memmap2::MmapOptions::new().map(&file)? };

        let inner = Box::new(WalInner {
            ref_count: AtomicU32::new(1),
            buffer: WalBuffer::Mmap(mmap),
        });

        info!(path = %wal_path.display(), bytes = inner.buffer.len(), "WAL file mapped");

        Ok(Self {
            inner: unsafe { NonNull::new_unchecked(Box::into_raw(inner)) },
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
        unsafe { crate::format::vec_buf::make_view(&inner.ref_count, reference) }
    }

    /// Recovers all data into owned `ExecutionRecord` structs.
    /// (Useful for loading small runs directly into memory).
    pub fn recover_all(&self) -> Vec<(usize, PyroView)> {
        let mut results = Vec::new();
        for frame in self.frames() {
            if let Ok(view) = self.view_at(frame.packet_offset) {
                results.push((frame.row_index, view));
            }
        }
        results
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

        let row_index =
            u32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().ok()?) as usize;

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
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::vec_buf::PyroVec;
    use crate::format::header::PyroData;
    use tempfile::NamedTempFile;

    fn make_pyro_record(data: &[u8]) -> PyroVec {
        let mut vec = PyroVec::with_capacity(data.len());
        vec.extend_from_slice(data);
        vec
    }

    #[test]
    fn test_wal_roundtrip() {
        let tmp_file = NamedTempFile::new().unwrap();
        let path = tmp_file.path().with_extension("pyrowal");
        
        // We need a base path because WalWriter::open adds .pyrowal
        let base_path = path.with_extension(""); 
        
        let mut wal = WalWriter::open(&base_path).unwrap();
        let record = make_pyro_record(b"test data");
        
        wal.append(42, record.py_ref()).unwrap();
        
        let reader = WalReader::open(&base_path).unwrap();
        let mut frames = reader.frames();
        
        let frame = frames.next().expect("Should have one frame");
        assert_eq!(frame.row_index, 42);
        assert_eq!(frame.packet.as_slice(), b"test data");
        assert!(frames.next().is_none());
    }

    #[test]
    fn test_wal_multiple_records() {
        let tmp_file = NamedTempFile::new().unwrap();
        let base_path = tmp_file.path().with_extension("");
        
        let mut wal = WalWriter::open(&base_path).unwrap();
        for i in 0..5 {
            let record = make_pyro_record(format!("data {}", i).as_bytes());
            wal.append(i, record.py_ref()).unwrap();
        }
        
        let reader = WalReader::open(&base_path).unwrap();
        let frames: Vec<_> = reader.frames().collect();
        
        assert_eq!(frames.len(), 5);
        for i in 0..5 {
            assert_eq!(frames[i].row_index, i);
            assert_eq!(frames[i].packet.as_slice(), format!("data {}", i).as_bytes());
        }
    }

    #[test]
    fn test_wal_corruption() {
        let tmp_file = NamedTempFile::new().unwrap();
        let base_path = tmp_file.path().with_extension("");
        
        {
            let mut wal = WalWriter::open(&base_path).unwrap();
            let record = make_pyro_record(b"test data");
            wal.append(0, record.py_ref()).unwrap();
        }

        // Corrupt the file: the first 16 bytes are the prefix. 
        // The next 16 bytes are the Pyro header.
        let wal_path = base_path.with_extension("pyrowal");
        let mut data = std::fs::read(&wal_path).unwrap();
        if data.len() > 20 {
            data[20] ^= 0xFF; // Corrupt the Pyro header or payload
        }
        std::fs::write(&wal_path, data).unwrap();

        let reader = WalReader::open(base_path).unwrap();
        let mut frames = reader.frames();
        
        // The iterator should return None if it encounters a corrupt PyroRef
        assert!(frames.next().is_none());
    }

    #[test]
    fn test_wal_empty_file() {
        let tmp_file = NamedTempFile::new().unwrap();
        let base_path = tmp_file.path().with_extension("");
        let wal_path = base_path.with_extension("pyrowal");
        std::fs::write(&wal_path, "").unwrap();

        let reader = WalReader::open(&base_path).unwrap();
        let mut frames = reader.frames();
        assert!(frames.next().is_none());
    }

    #[test]
    fn test_wal_view_at() {
        let tmp_file = NamedTempFile::new().unwrap();
        let base_path = tmp_file.path().with_extension("");
        
        let mut wal = WalWriter::open(&base_path).unwrap();
        let record = make_pyro_record(b"view test");
        wal.append(0, record.py_ref()).unwrap();
        
        let reader = WalReader::open(&base_path).unwrap();
        let frame = reader.frames().next().unwrap();
        
        let view = reader.view_at(frame.packet_offset).expect("Should create view");
        assert_eq!(view.as_slice(), b"view test");
    }
}