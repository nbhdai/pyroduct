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
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use tokio::sync::{Mutex, mpsc, oneshot};

use tracing::{info, warn};

use crate::format::header::PyroData;
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
// Alignment
// =============================================================================

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

    pub async fn append(&mut self, record_index: usize, record: PyroRef<'_>) -> io::Result<()> {
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

    /// Appends multiple records to the WAL, flushing and syncing all changes to disk at the end of the batch.
    pub async fn append_batch(&mut self, records: &[(usize, PyroRef<'_>)]) -> io::Result<()> {
        if records.is_empty() {
            return Ok(());
        }

        for &(record_index, ref record) in records {
            let row_index = record_index as u32;

            let mut prefix = [0u8; 16];
            prefix[0..4].copy_from_slice(&row_index.to_le_bytes());
            self.wal_writer.write_all(&prefix)?;

            let raw_slice = record.as_raw_slice();
            let raw_len = raw_slice.len();
            let align_val = align16(raw_len);
            let padding_len = align_val - raw_len;

            self.wal_writer.write_all(raw_slice)?;
            if padding_len > 0 {
                let padding = [0u8; 15];
                self.wal_writer.write_all(&padding[..padding_len])?;
            }

            self.records_written += 1;
        }

        self.wal_writer.flush()?;
        self.wal_writer.get_ref().sync_data()?;

        Ok(())
    }

    pub fn records_written(&self) -> u64 {
        self.records_written
    }
    pub fn wal_path(&self) -> Option<&Path> {
        self.wal_path.as_deref()
    }

    pub fn into_inner(self) -> W {
        match self.wal_writer.into_inner() {
            Ok(inner) => inner,
            Err(e) => panic!("failed to flush wal writer: {e}"),
        }
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

use crate::format::vec_buf::PyroVec;

/// `WalManager` coordinates multiple asynchronous log writers feeding into a single `WalWriter` on a background thread.
#[derive(Clone)]
pub struct WalManager {
    bound: usize,
    sender: Arc<std::sync::RwLock<mpsc::Sender<(usize, PyroVec)>>>,
    total_len: Arc<AtomicUsize>,
    wal_path: Arc<std::sync::RwLock<Option<PathBuf>>>,
    inner: Arc<Mutex<WalManagerInner>>,
}

struct WalManagerInner {
    shutdown_tx: Option<oneshot::Sender<()>>,
    join_handle: Option<tokio::task::JoinHandle<tokio::io::Result<()>>>,
}

impl WalManager {
    /// Creates and spawns a new `WalManager` that writes log entries to a `WalWriter` in a background Tokio task.
    ///
    /// The manager holds a bounded sender queue of the specified `bound` size.
    pub fn new(mut wal_writer: WalWriter<File>, bound: usize) -> Self {
        let (sender, mut receiver) = mpsc::channel::<(usize, PyroVec)>(bound);
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

        let initial_len = wal_writer.records_written() as usize;
        let total_len = Arc::new(AtomicUsize::new(initial_len));
        let total_len_clone = Arc::clone(&total_len);
        let wal_path = wal_writer.wal_path().map(|p| p.to_path_buf());

        let join_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    msg = receiver.recv() => {
                        match msg {
                            Some((idx, record)) => {
                                wal_writer.append(idx, record.py_ref()).await?;
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
            // Drain remaining messages in receiver
            while let Ok((idx, record)) = receiver.try_recv() {
                wal_writer.append(idx, record.py_ref()).await?;
            }
            Ok(())
        });

        Self {
            bound,
            sender: Arc::new(std::sync::RwLock::new(sender)),
            total_len: total_len_clone,
            wal_path: Arc::new(std::sync::RwLock::new(wal_path)),
            inner: Arc::new(Mutex::new(WalManagerInner {
                shutdown_tx: Some(shutdown_tx),
                join_handle: Some(join_handle),
            })),
        }
    }

    /// Sends a `PyroVec` record to the WAL background task.
    ///
    /// Increments the `total_len` counter if the message is successfully enqueued.
    pub async fn send(
        &self,
        index: usize,
        record: PyroVec,
    ) -> Result<(), mpsc::error::SendError<(usize, PyroVec)>> {
        let sender = self.sender.read().unwrap().clone();
        sender.send((index, record)).await?;
        self.total_len.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    /// Returns the total number of log entries sent to the manager.
    pub fn total_len(&self) -> usize {
        self.total_len.load(Ordering::SeqCst)
    }

    /// Returns the path to the current WAL file.
    pub fn wal_path(&self) -> Option<PathBuf> {
        self.wal_path.read().unwrap().clone()
    }

    /// Retrieves a single `PyroView` by its global index by scanning the WAL frames.
    pub fn get(&self, index: usize) -> tokio::io::Result<Option<PyroView>> {
        if let Some(path) = self.wal_path() {
            // WalReader::open uses the base path without extension
            let reader = WalReader::open(path.with_extension(""))?;
            for frame in reader.frames() {
                if frame.row_index == index {
                    let view = reader
                        .view_at(frame.packet_offset)
                        .map_err(|e| tokio::io::Error::new(io::ErrorKind::InvalidData, e))?;
                    let owned_vec = view.clone_to_vec();
                    drop(view);
                    return Ok(Some(owned_vec.view()));
                }
            }
        }
        Ok(None)
    }

    /// Rotates the `WalManager` with a new `WalWriter`.
    /// This shuts down the old write loop (waiting for it to finish flushing and draining)
    /// and replaces it with a new loop using the new writer.
    pub async fn rotate(&self, mut new_wal_writer: WalWriter<File>) -> io::Result<()> {
        let mut inner = self.inner.lock().await;

        let (new_sender, mut new_receiver) = mpsc::channel::<(usize, PyroVec)>(self.bound);
        let (new_shutdown_tx, mut new_shutdown_rx) = oneshot::channel::<()>();

        {
            // 1. Lock the sender for writing to prevent any thread from obtaining the old sender
            let mut sender_guard = self.sender.write().unwrap();

            // 2. Swap the old sender with the new sender
            let old_sender = std::mem::replace(&mut *sender_guard, new_sender);

            // 3. Drop the sender guard so other threads can start sending to the new loop
            // 4. Drop our copy of the old sender.
            // Any transient senders in active `send()` calls will finish sending and be dropped.
            // Once they are all dropped, the old receiver will receive `None` and naturally terminate the old loop,
            // ensuring complete draining of all messages.
            drop(sender_guard);
            drop(old_sender);
        }

        // 6. Wait for the old loop to complete (which drains its queue and flushes to disk)
        if let Some(handle) = inner.join_handle.take() {
            match handle.await {
                Ok(res) => res?,
                Err(join_err) => {
                    return Err(io::Error::other(format!(
                        "Background task join error during rotation: {:?}",
                        join_err
                    )));
                }
            }
        }

        // 7. Update the path in the shared `wal_path` field
        let new_wal_path = new_wal_writer.wal_path().map(|p| p.to_path_buf());
        {
            let mut path_guard = self.wal_path.write().unwrap();
            *path_guard = new_wal_path;
        }

        let new_records_written = new_wal_writer.records_written();

        // 8. Spawn the new background loop
        let new_join_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    msg = new_receiver.recv() => {
                        match msg {
                            Some((idx, record)) => {
                                new_wal_writer.append(idx, record.py_ref()).await?;
                            }
                            None => {
                                break;
                            }
                        }
                    }
                    _ = &mut new_shutdown_rx => {
                        break;
                    }
                }
            }
            // Drain remaining messages in receiver
            while let Ok((idx, record)) = new_receiver.try_recv() {
                new_wal_writer.append(idx, record.py_ref()).await?;
            }
            Ok(())
        });

        // 9. Update the inner state
        inner.shutdown_tx = Some(new_shutdown_tx);
        inner.join_handle = Some(new_join_handle);

        // 10. Update total_len to match the new segment's records_written
        self.total_len
            .store(new_records_written as usize, Ordering::SeqCst);

        Ok(())
    }

    /// Signals the background task to shut down and waits for completion.
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
    use crate::format::header::PyroData;
    use crate::format::vec_buf::PyroVec;
    use tempfile::NamedTempFile;

    fn make_pyro_record(data: &[u8]) -> PyroVec {
        let mut vec = PyroVec::with_capacity(data.len());
        vec.extend_from_slice(data);
        vec
    }

    #[tokio::test]
    async fn test_wal_roundtrip() {
        let tmp_file = NamedTempFile::new().unwrap();
        let path = tmp_file.path().with_extension("pyrowal");

        // We need a base path because WalWriter::open adds .pyrowal
        let base_path = path.with_extension("");

        let mut wal = WalWriter::open(&base_path).unwrap();
        let record = make_pyro_record(b"test data");

        wal.append(42, record.py_ref()).await.unwrap();

        let reader = WalReader::open(&base_path).unwrap();
        let mut frames = reader.frames();

        let frame = frames.next().expect("Should have one frame");
        assert_eq!(frame.row_index, 42);
        assert_eq!(frame.packet.as_slice(), b"test data");
        assert!(frames.next().is_none());
    }

    #[tokio::test]
    async fn test_wal_multiple_records() {
        let tmp_file = NamedTempFile::new().unwrap();
        let base_path = tmp_file.path().with_extension("");

        let mut wal = WalWriter::open(&base_path).unwrap();
        for i in 0..5 {
            let record = make_pyro_record(format!("data {}", i).as_bytes());
            wal.append(i, record.py_ref()).await.unwrap();
        }

        let reader = WalReader::open(&base_path).unwrap();
        let frames: Vec<_> = reader.frames().collect();

        assert_eq!(frames.len(), 5);
        for (i, frame) in frames.iter().enumerate() {
            assert_eq!(frame.row_index, i);
            assert_eq!(frame.packet.as_slice(), format!("data {}", i).as_bytes());
        }
    }

    #[tokio::test]
    async fn test_wal_corruption() {
        let tmp_file = NamedTempFile::new().unwrap();
        let base_path = tmp_file.path().with_extension("");

        {
            let mut wal = WalWriter::open(&base_path).unwrap();
            let record = make_pyro_record(b"test data");
            wal.append(0, record.py_ref()).await.unwrap();
        }

        // Corrupt the file: the first 16 bytes are the prefix.
        // The next 16 bytes are the Pyro header (first 4 bytes are length).
        let wal_path = base_path.with_extension("pyrowal");
        let mut data = std::fs::read(&wal_path).unwrap();
        if data.len() > 16 {
            data[16] ^= 0xFF; // Corrupt the length in the Pyro header
        }
        std::fs::write(&wal_path, data).unwrap();

        let reader = WalReader::open(base_path).unwrap();
        let mut frames = reader.frames();

        // The iterator should return None if it encounters a corrupt PyroRef
        assert!(frames.next().is_none());
    }

    #[tokio::test]
    async fn test_wal_empty_file() {
        let tmp_file = NamedTempFile::new().unwrap();
        let base_path = tmp_file.path().with_extension("");
        let wal_path = base_path.with_extension("pyrowal");
        std::fs::write(&wal_path, "").unwrap();

        let reader = WalReader::open(&base_path).unwrap();
        let mut frames = reader.frames();
        assert!(frames.next().is_none());
    }

    #[tokio::test]
    async fn test_wal_view_at() {
        let tmp_file = NamedTempFile::new().unwrap();
        let base_path = tmp_file.path().with_extension("");

        let mut wal = WalWriter::open(&base_path).unwrap();
        let record = make_pyro_record(b"view test");
        wal.append(0, record.py_ref()).await.unwrap();

        let reader = WalReader::open(&base_path).unwrap();
        let frame = reader.frames().next().unwrap();

        let view = reader
            .view_at(frame.packet_offset)
            .expect("Should create view");
        assert_eq!(view.as_slice(), b"view test");
    }

    #[tokio::test]
    async fn test_wal_append_batch() {
        let tmp_file = NamedTempFile::new().unwrap();
        let base_path = tmp_file.path().with_extension("");

        let mut wal = WalWriter::open(&base_path).unwrap();
        let r1 = make_pyro_record(b"data 1");
        let r2 = make_pyro_record(b"data 2");

        let batch = vec![(10, r1.py_ref()), (20, r2.py_ref())];
        wal.append_batch(&batch).await.unwrap();

        let reader = WalReader::open(&base_path).unwrap();
        let frames: Vec<_> = reader.frames().collect();

        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].row_index, 10);
        assert_eq!(frames[0].packet.as_slice(), b"data 1");
        assert_eq!(frames[1].row_index, 20);
        assert_eq!(frames[1].packet.as_slice(), b"data 2");
    }

    #[tokio::test]
    async fn test_wal_manager_basic() {
        let tmp_file = NamedTempFile::new().unwrap();
        let base_path = tmp_file.path().with_extension("");

        let wal = WalWriter::open(&base_path).unwrap();
        let manager = WalManager::new(wal, 10);

        assert_eq!(manager.total_len(), 0);

        let r1 = make_pyro_record(b"mgr 1");
        manager.send(42, r1).await.unwrap();
        assert_eq!(manager.total_len(), 1);

        manager.interrupt().await.unwrap();

        // Retrieve entry via get
        let retrieved = manager.get(42).unwrap().expect("Should find entry");
        assert_eq!(retrieved.as_slice(), b"mgr 1");

        // Verify reopening
        let reader = WalReader::open(&base_path).unwrap();
        let frames: Vec<_> = reader.frames().collect();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].row_index, 42);
        assert_eq!(frames[0].packet.as_slice(), b"mgr 1");
    }

    #[tokio::test]
    async fn test_wal_manager_concurrent() {
        let tmp_file = NamedTempFile::new().unwrap();
        let base_path = tmp_file.path().with_extension("");

        let wal = WalWriter::open(&base_path).unwrap();
        let manager = WalManager::new(wal, 5);

        let mut tasks = Vec::new();
        for i in 0..10 {
            let m = manager.clone();
            let rec = make_pyro_record(format!("rec {}", i).as_bytes());
            tasks.push(tokio::spawn(async move {
                m.send(i, rec).await.unwrap();
            }));
        }

        for t in tasks {
            t.await.unwrap();
        }

        assert_eq!(manager.total_len(), 10);

        manager.interrupt().await.unwrap();

        // Verify reopening
        let reader = WalReader::open(&base_path).unwrap();
        let frames: Vec<_> = reader.frames().collect();
        assert_eq!(frames.len(), 10);
    }

    #[tokio::test]
    async fn test_wal_manager_rotation() {
        let tmp_file1 = NamedTempFile::new().unwrap();
        let base_path1 = tmp_file1.path().with_extension("");
        let tmp_file2 = NamedTempFile::new().unwrap();
        let base_path2 = tmp_file2.path().with_extension("");

        let wal1 = WalWriter::open(&base_path1).unwrap();
        let manager = WalManager::new(wal1, 5);

        // Write 3 entries to the first segment
        for i in 0..3 {
            let record = make_pyro_record(format!("rec {}", i).as_bytes());
            manager.send(i, record).await.unwrap();
        }
        assert_eq!(manager.total_len(), 3);

        // Rotate to the second segment
        let wal2 = WalWriter::open(&base_path2).unwrap();
        manager.rotate(wal2).await.unwrap();

        // Write 2 entries to the second segment
        for i in 3..5 {
            let record = make_pyro_record(format!("rec {}", i).as_bytes());
            manager.send(i, record).await.unwrap();
        }
        assert_eq!(manager.total_len(), 2);

        manager.interrupt().await.unwrap();

        // Verify first segment
        let reader1 = WalReader::open(&base_path1).unwrap();
        let frames1: Vec<_> = reader1.frames().collect();
        assert_eq!(frames1.len(), 3);
        for (i, frame) in frames1.iter().enumerate() {
            assert_eq!(frame.row_index, i);
            assert_eq!(frame.packet.as_slice(), format!("rec {}", i).as_bytes());
        }

        // Verify second segment
        let reader2 = WalReader::open(&base_path2).unwrap();
        let frames2: Vec<_> = reader2.frames().collect();
        assert_eq!(frames2.len(), 2);
        for i in 3..5 {
            assert_eq!(frames2[i - 3].row_index, i);
            assert_eq!(
                frames2[i - 3].packet.as_slice(),
                format!("rec {}", i).as_bytes()
            );
        }
    }
}
