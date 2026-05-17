use std::fs::File;
use std::path::{Path, PathBuf};

use crate::PyroRow;
use crate::error::PyroError;
use crate::captured::CapturedError;
use crate::format::Bridgeable;
use crate::format::header::PyroData;
use crate::format::wal::{WalWriter as RawWalWriter, WalReader};

/// A high-level WAL writer wrapper that handles serialization of `PyroRow` values.
pub struct WalWriter {
    inner: RawWalWriter<File>,
}

impl WalWriter {
    /// Opens the high-level WAL writer for appending.
    pub fn open(base_path: impl Into<PathBuf>) -> Result<Self, PyroError> {
        let inner = RawWalWriter::open(base_path).map_err(|e| PyroError::local_io(CapturedError::new(e)))?;
        Ok(Self { inner })
    }

    /// Appends a `PyroRow` to the WAL by shipping/serializing it zero-copy.
    pub fn append(&mut self, record_index: usize, row: &PyroRow<'_>) -> Result<(), PyroError> {
        let owned_row = row.to_static();
        let vec = owned_row.ship().map_err(|e| PyroError::validation(CapturedError::new(e)))?;
        self.inner.append(record_index, vec.py_ref()).map_err(|e| PyroError::local_io(CapturedError::new(e)))?;
        Ok(())
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
    let reader = WalReader::open(base_path).map_err(|e| PyroError::local_io(CapturedError::new(e)))?;
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
