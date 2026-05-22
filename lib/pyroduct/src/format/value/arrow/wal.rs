use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::PyroRow;
use crate::captured::CapturedError;
use crate::error::PyroError;
use crate::format::Bridgeable;
use crate::format::header::PyroData;
use crate::format::value::arrow::PreBatch;
use crate::format::wal::{WalManager as RealWalManager, WalReader, WalWriter as RawWalWriter};
use arrow::record_batch::RecordBatch;

/// A high-level WAL writer wrapper that handles serialization of `PyroRow` values.
pub struct WalWriter {
    inner: RawWalWriter<File>,
}

impl WalWriter {
    /// Opens the high-level WAL writer for appending.
    pub fn open(base_path: impl Into<PathBuf>) -> Result<Self, PyroError> {
        let inner = RawWalWriter::open(base_path)
            .map_err(|e| PyroError::local_io(CapturedError::new(e)))?;
        Ok(Self { inner })
    }

    /// Appends a `PyroRow` to the WAL by shipping/serializing it zero-copy.
    pub async fn append(
        &mut self,
        record_index: usize,
        row: &PyroRow<'_>,
    ) -> Result<(), PyroError> {
        let owned_row = row.to_static();
        let vec = owned_row
            .ship()
            .map_err(|e| PyroError::validation(CapturedError::new(e)))?;
        self.inner
            .append(record_index, vec.py_ref())
            .await
            .map_err(|e| PyroError::local_io(CapturedError::new(e)))?;
        Ok(())
    }

    /// Appends multiple `PyroRow` records to the WAL using the underlying RawWalWriter's batching.
    pub async fn append_batch(
        &mut self,
        records: &[(usize, PyroRow<'_>)],
    ) -> Result<(), PyroError> {
        let mut shipped = Vec::with_capacity(records.len());
        let mut vecs = Vec::with_capacity(records.len());
        for &(_, ref row) in records {
            let owned_row = row.to_static();
            let vec = owned_row
                .ship()
                .map_err(|e| PyroError::validation(CapturedError::new(e)))?;
            vecs.push(vec);
        }
        for (i, &(record_index, _)) in records.iter().enumerate() {
            shipped.push((record_index, vecs[i].py_ref()));
        }
        self.inner
            .append_batch(&shipped)
            .await
            .map_err(|e| PyroError::local_io(CapturedError::new(e)))?;
        Ok(())
    }

    /// Retrieves a record by its global `record_index` checking the underlying WAL file.
    pub fn get(&self, record_index: usize) -> Result<Option<PyroRow<'static>>, PyroError> {
        if let Some(path) = self.wal_path() {
            let base_path = path.with_extension("");
            if let Ok(reader) = WalReader::open(&base_path) {
                for frame in reader.frames() {
                    if frame.row_index as usize == record_index {
                        let row_buf = PyroRow::expose_view(frame.packet)
                            .map_err(|e| PyroError::validation(CapturedError::new(e)))?;
                        let row = PyroRow::from(&*row_buf).to_static();
                        return Ok(Some(row));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Returns the number of records written in this session.
    pub fn records_written(&self) -> u64 {
        self.inner.records_written()
    }

    /// Returns the path to the underling `.pyrowal` file on disk.
    pub fn wal_path(&self) -> Option<&Path> {
        self.inner.wal_path()
    }
}

/// Standalone helper to recover all `PyroRow<'static>` records from a WAL file.
pub fn recover(base_path: &Path) -> Result<Vec<PyroRow<'static>>, PyroError> {
    let reader =
        WalReader::open(base_path).map_err(|e| PyroError::local_io(CapturedError::new(e)))?;
    let mut records = Vec::new();
    for frame in reader.frames() {
        let pyref = frame.packet;
        let row_buf = PyroRow::expose_view(pyref)
            .map_err(|e| PyroError::validation(CapturedError::new(e)))?;
        let row = PyroRow::from(&*row_buf).to_static();
        records.push(row);
    }
    Ok(records)
}

/// Standalone helper to recover all `(usize, PyroRow<'static>)` records with their index from a WAL file.
pub fn recover_with_index(base_path: &Path) -> Result<Vec<(usize, PyroRow<'static>)>, PyroError> {
    let reader =
        WalReader::open(base_path).map_err(|e| PyroError::local_io(CapturedError::new(e)))?;
    let mut records = Vec::new();
    for frame in reader.frames() {
        let pyref = frame.packet;
        let row_buf = PyroRow::expose_view(pyref)
            .map_err(|e| PyroError::validation(CapturedError::new(e)))?;
        let row = PyroRow::from(&*row_buf).to_static();
        records.push((frame.row_index as usize, row));
    }
    Ok(records)
}

/// Standalone helper to recover the `CurrentData` (PreBatch + current_wal_rows mapping) from a WAL file.
pub fn recover_current_data(
    base_path: &Path,
    schema: crate::format::value::PyroSchema<'static>,
) -> Result<CurrentData, PyroError> {
    let reader =
        WalReader::open(base_path).map_err(|e| PyroError::local_io(CapturedError::new(e)))?;
    let mut prebatch = PreBatch::new(schema);
    let mut current_wal_rows = HashMap::new();

    for (physical_idx, frame) in reader.frames().enumerate() {
        let pyref = frame.packet;
        let row_buf = PyroRow::expose_view(pyref)
            .map_err(|e| PyroError::validation(CapturedError::new(e)))?;
        let row = PyroRow::from(&*row_buf).to_static();

        prebatch.push(row).map_err(|e| {
            PyroError::validation(
                CapturedError::new("Failed to push recovered record to PreBatch").with_source(e),
            )
        })?;
        current_wal_rows.insert(frame.row_index as usize, physical_idx);
    }

    Ok(CurrentData {
        prebatch,
        current_wal_rows,
    })
}

/// Holds the in-memory state of the active WAL segment.
pub struct CurrentData {
    pub prebatch: PreBatch,
    pub current_wal_rows: HashMap<usize, usize>,
}

/// A thread-safe, asynchronous WAL manager that handles concurrency using background writing,
/// keeping an in-memory `PreBatch` buffer and index mapping (`CurrentData`) in sync.
#[derive(Clone)]
pub struct WalManager {
    pub inner: RealWalManager,
    pub data: Arc<Mutex<CurrentData>>,
}

impl WalManager {
    /// Spawns the manager with a background task.
    pub fn new(
        writer: WalWriter,
        bound: usize,
        schema: crate::format::value::PyroSchema<'static>,
    ) -> Self {
        let inner = RealWalManager::new(writer.inner, bound);
        let data = Arc::new(Mutex::new(CurrentData {
            prebatch: PreBatch::new(schema),
            current_wal_rows: HashMap::new(),
        }));
        Self { inner, data }
    }

    /// Creates a `WalManager` from an existing file by recovering the `CurrentData` first.
    pub fn open_with_recovery(
        base_path: impl Into<PathBuf>,
        bound: usize,
        schema: crate::format::value::PyroSchema<'static>,
    ) -> Result<Self, PyroError> {
        let path: PathBuf = base_path.into();
        let writer = WalWriter::open(&path)?;

        // If the WAL file exists and contains frames, we recover them.
        let wal_file_path = path.with_extension("pyrowal");
        let current_data = if wal_file_path.exists()
            && std::fs::metadata(&wal_file_path)
                .map(|m| m.len())
                .unwrap_or(0)
                > 0
        {
            recover_current_data(&path, schema)?
        } else {
            CurrentData {
                prebatch: PreBatch::new(schema),
                current_wal_rows: HashMap::new(),
            }
        };

        let inner = RealWalManager::new(writer.inner, bound);
        let data = Arc::new(Mutex::new(current_data));
        Ok(Self { inner, data })
    }

    /// Appends a `PyroRow` asynchronously.
    pub async fn append(&self, record_index: usize, row: &PyroRow<'_>) -> Result<(), PyroError> {
        let owned_row = row.to_static();
        let vec = owned_row
            .ship()
            .map_err(|e| PyroError::validation(CapturedError::new(e)))?;

        {
            let mut data = self.data.lock().await;
            let physical_index = data.prebatch.len();
            data.prebatch.push(owned_row.clone()).map_err(|e| {
                PyroError::validation(
                    CapturedError::new("Failed to push row to PreBatch").with_source(e),
                )
            })?;
            data.current_wal_rows.insert(record_index, physical_index);
        }

        // Send to the underlying RealWalManager background writer
        self.inner.send(record_index, vec).await.map_err(|_| {
            PyroError::local_io(CapturedError::new("Unable to send data to the WAL"))
        })?;

        Ok(())
    }

    /// Retrieves a record by its global `record_index` checking the in-memory buffer or WAL file.
    pub async fn get(&self, record_index: usize) -> Result<Option<PyroRow<'static>>, PyroError> {
        // Fast path: check in-memory PreBatch
        {
            let data = self.data.lock().await;
            if let Some(&physical_idx) = data.current_wal_rows.get(&record_index) {
                if let Some(row) = data.prebatch.get(physical_idx) {
                    return Ok(Some(row.clone()));
                }
            }
        }

        Ok(None)
    }

    /// Returns the path to the WAL file.
    pub fn wal_path(&self) -> Option<PathBuf> {
        self.inner.wal_path()
    }

    /// Returns the total number of log entries sent to the manager.
    pub fn total_len(&self) -> usize {
        self.inner.total_len()
    }

    /// Rotates the WAL manager with a new `WalWriter`.
    /// This locks the in-memory data, flushes the current `PreBatch` into an Apache Arrow `RecordBatch`,
    /// extracts the current `current_wal_rows` mapping, and rotates the underlying `self.inner` (RealWalManager).
    /// It resets the `PreBatch` with the new schema and returns the record batch + the index mapping.
    pub async fn rotate(
        &self,
        new_writer: WalWriter,
        new_schema: crate::format::value::PyroSchema<'static>,
    ) -> Result<(RecordBatch, HashMap<usize, usize>), PyroError> {
        let mut data = self.data.lock().await;

        // 1. Flush the old prebatch
        let batch_opt = data
            .prebatch
            .flush()
            .map_err(|e| PyroError::validation(CapturedError::new(e)))?;

        // 2. If it's None (empty), construct an empty RecordBatch with the old schema
        let record_batch = match batch_opt {
            Some(b) => b,
            None => {
                let arrow_schema = data.prebatch.arrow_schema();
                RecordBatch::new_empty(arrow_schema)
            }
        };

        // 3. Take the old WAL rows map
        let old_rows = std::mem::take(&mut data.current_wal_rows);

        // 4. Rotate the underlying low-level WalManager
        self.inner
            .rotate(new_writer.inner)
            .await
            .map_err(|e| PyroError::local_io(CapturedError::new(e)))?;

        // 5. Reset the prebatch with the new schema
        data.prebatch = PreBatch::new(new_schema);

        Ok((record_batch, old_rows))
    }

    /// Shuts down the background writer.
    pub async fn interrupt(&self) -> Result<(), PyroError> {
        self.inner.interrupt().await.map_err(|e| {
            PyroError::local_io(CapturedError::new("Unable to close data WAL").with_source(e))
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PyroValue;
    use crate::format::value::PyroSchema;
    use arrow::datatypes::{DataType, Field, Schema};
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_arrow_wal_manager_rotation() {
        let schema = Arc::new(Schema::new(vec![Field::new("val", DataType::Int32, true)]));
        let pyro_schema = PyroSchema::from_arrow(&schema).unwrap();

        // Create WAL 1
        let tmp_file1 = NamedTempFile::new().unwrap();
        let base_path1 = tmp_file1.path().with_extension("");
        let writer1 = WalWriter::open(&base_path1).unwrap();

        let manager = WalManager::new(writer1, 10, pyro_schema.clone());

        // Append to manager
        let row1 = PyroRow::from([("val", PyroValue::I32(100))]);
        manager.append(10, &row1).await.unwrap();

        // Create WAL 2
        let tmp_file2 = NamedTempFile::new().unwrap();
        let base_path2 = tmp_file2.path().with_extension("");
        let writer2 = WalWriter::open(&base_path2).unwrap();

        // Rotate
        let (batch, rows_map) = manager.rotate(writer2, pyro_schema.clone()).await.unwrap();

        // Verify the record batch returned from rotate has our entry
        assert_eq!(batch.num_rows(), 1);
        let val_col = batch
            .column(0)
            .as_any()
            .downcast_ref::<arrow::array::Int32Array>()
            .unwrap();
        assert_eq!(val_col.value(0), 100);

        // Verify rows_map
        assert_eq!(rows_map.len(), 1);
        assert_eq!(rows_map.get(&10), Some(&0));

        // Append to rotated manager
        let row2 = PyroRow::from([("val", PyroValue::I32(200))]);
        manager.append(20, &row2).await.unwrap();

        // Interrupt manager to flush everything
        manager.interrupt().await.unwrap();

        // Verify WAL 2 contains the new record
        let recovered = recover(&base_path2).unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].get("val"), Some(&PyroValue::I32(200)));
    }

    #[tokio::test]
    async fn test_arrow_wal_manager_rotation_empty() {
        let schema = Arc::new(Schema::new(vec![Field::new("val", DataType::Int32, true)]));
        let pyro_schema = PyroSchema::from_arrow(&schema).unwrap();

        // Create WAL 1
        let tmp_file1 = NamedTempFile::new().unwrap();
        let base_path1 = tmp_file1.path().with_extension("");
        let writer1 = WalWriter::open(&base_path1).unwrap();

        let manager = WalManager::new(writer1, 10, pyro_schema.clone());

        // Create WAL 2
        let tmp_file2 = NamedTempFile::new().unwrap();
        let base_path2 = tmp_file2.path().with_extension("");
        let writer2 = WalWriter::open(&base_path2).unwrap();

        // Rotate empty WAL
        let (batch, rows_map) = manager.rotate(writer2, pyro_schema.clone()).await.unwrap();

        // Verify the record batch is empty
        assert_eq!(batch.num_rows(), 0);
        assert_eq!(batch.schema().fields().len(), 1);
        assert_eq!(batch.schema().field(0).name(), "val");

        // Verify rows_map is empty
        assert!(rows_map.is_empty());
    }
}
