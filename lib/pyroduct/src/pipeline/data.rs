use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

use arrow::array::RecordBatch;
use pyro_file;
use crate::error::PyroError;
use crate::format::wal::{ExecutionRecord, WalWriter, recover};
use crate::format::value::arrow::PreBatch;
use crate::format::value::PyroSchema;
use crate::captured::CapturedError;

/// Manages the persistence of pipeline data, handling the transition from 
/// WAL -> Arrow IPC (memmapable) -> Parquet.
pub struct DataManager {
    output_dir: PathBuf,
    schema: PyroSchema<'static>,
    
    /// The in-memory buffer for accumulating rows before flushing to IPC.
    wal_data: PreBatch,
    
    /// The current WAL ID
    current_wal_id: usize,
    /// The wal writer for the current WAL.
    wal_writer: Option<WalWriter<std::fs::File, std::fs::File>>,
    
    /// List of IPC files (by ID) that have been flushed from WAL and are ready for Parquet rollout.
    ipc_files: Vec<usize>,
    /// Row counts for pending IPC files.
    ipc_row_counts: HashMap<usize, usize>,
    /// Total row count of all rolled-out Parquet files.
    total_parquet_rows: usize,

    /// How many rows get rolled up into an IPC file
    wal_capacity: usize,
    /// How many IPC files get rolled up into a parquet file
    ipc_capacity: usize,
}

impl DataManager {
    pub fn new(output_dir: PathBuf, schema: PyroSchema<'static>) -> Self {
        Self {
            output_dir,
            wal_data: PreBatch::new(schema.clone()),
            schema,
            current_wal_id: 0,
            wal_writer: None,
            ipc_files: Vec::new(),
            ipc_row_counts: HashMap::new(),
            total_parquet_rows: 0,
            wal_capacity: 10_000,
            ipc_capacity: 10,
        }
    }

    /// Scans the output directory to restore state from existing files.
    /// This populates row counts for Parquet and IPC files and determines the current WAL ID.
    pub fn restore(&mut self) -> Result<(), PyroError> {
        let mut max_wal_id = 0;
        let mut total_parquet = 0;
        let mut ipc_counts = HashMap::new();

        let entries = std::fs::read_dir(&self.output_dir).map_err(|e| PyroError::local_io(CapturedError::new(e)))?;

        for entry in entries {
            let entry = entry.map_err(|e| PyroError::local_io(CapturedError::new(e)))?;
            let path = entry.path();
            let filename = path.file_name().unwrap().to_string_lossy();

            if filename.starts_with("wal_") && filename.contains(".pyrowal") {
                // Extract ID from wal_N.pyrowal
                if let Some(id_str) = filename.strip_prefix("wal_").and_then(|s| s.split('.').next()) {
                    if let Ok(id) = id_str.parse::<usize>() {
                        max_wal_id = max_wal_id.max(id);
                    }
                }
            } else if filename.starts_with("batch_") && filename.ends_with(".arrow") {
                // Extract ID from batch_N.arrow
                if let Some(id_str) = filename.strip_prefix("batch_").and_then(|s| s.split('.').next()) {
                    if let Ok(id) = id_str.parse::<usize>() {
                        let bytes = std::fs::read(&path).map_err(|e| PyroError::local_io(CapturedError::new(e)))?;
                        let batches = pyro_file::parse_data_to_batch_sync(bytes, &filename)
                            .map_err(|e| PyroError::validation(CapturedError::new(e)))?;
                        let count: usize = batches.iter().map(|b| b.num_rows()).sum();
                        ipc_counts.insert(id, count);
                    }
                }
            } else if filename.starts_with("rollout_") && filename.ends_with(".parquet") {
                // For Parquet, we need to read it to get row count.
                let bytes = std::fs::read(&path).map_err(|e| PyroError::local_io(CapturedError::new(e)))?;
                let filename_str = filename.to_string();
                let batches = pyro_file::parse_data_to_batch_sync(bytes, &filename_str)
                    .map_err(|e| PyroError::validation(CapturedError::new(e)))?;
                total_parquet += batches.iter().map(|b| b.num_rows()).sum::<usize>();
            }
        }

        self.current_wal_id = max_wal_id;
        self.total_parquet_rows = total_parquet;
        self.ipc_row_counts = ipc_counts;
        
        // Also recover the current WAL if it exists
        if self.current_wal_id > 0 {
            self.recover_wal(self.current_wal_id)?;
        }

        Ok(())
    }

    /// Returns the total number of rows across all stages:
    /// Parquet files + IPC files + current in-memory buffer.
    pub fn len(&self) -> usize {
        let ipc_sum: usize = self.ipc_row_counts.values().sum();
        self.total_parquet_rows + ipc_sum + self.wal_data.len()
    }

    pub fn set_capacities(&mut self, wal_capacity: usize, ipc_capacity: usize) {
        self.wal_capacity = wal_capacity;
        self.ipc_capacity = ipc_capacity;
    }

    /// Pushes a single WAL record to the current WAL and the in-memory buffer.
    pub fn push_record(&mut self, record: ExecutionRecord) -> Result<(), PyroError> {
        if self.wal_writer.is_none() {
            self.open_next_wal()?;
        }

        // 1. Write to WAL (Durability)
        if let Some(wal) = &mut self.wal_writer {
            wal.append(&record).map_err(|e| PyroError::local_io(CapturedError::new(e)))?;
        }

        // 2. Push to in-memory PreBatch (Performance)
        if let ExecutionRecord::Success { success, .. } = record {
            self.wal_data.push(success.row.clone()).map_err(|e| PyroError::validation(CapturedError::new(e)))?;
        }

        // 3. Check capacity for flush
        if self.wal_data.len() >= self.wal_capacity {
            self.flush_wal()?;
        }
        
        Ok(())
    }

    fn open_next_wal(&mut self) -> Result<(), PyroError> {
        self.current_wal_id += 1;
        let base_path = self.output_dir.join(format!("wal_{}", self.current_wal_id));
        let writer = WalWriter::open(base_path).map_err(|e| PyroError::local_io(CapturedError::new(e)))?;
        self.wal_writer = Some(writer);
        Ok(())
    }

    /// Rolls up the in-memory buffer into a memmapable Arrow IPC file.
    pub fn flush_wal(&mut self) -> Result<(), PyroError> {
        let wal_id = self.current_wal_id;
        
        // Close current writer to ensure all data is flushed to disk
        self.wal_writer = None;

        // Flush the in-memory prebatch
        match self.wal_data.flush() {
            Ok(Some(batch)) => {
                let row_count = batch.num_rows();
                self.write_arrow_ipc(wal_id, &batch)?;
                self.ipc_files.push(wal_id);
                self.ipc_row_counts.insert(wal_id, row_count);
                
                if self.ipc_files.len() >= self.ipc_capacity {
                    self.rollout_to_parquet()?;
                }
                Ok(())
            }
            Ok(None) => Ok(()),
            Err(e) => Err(PyroError::validation(CapturedError::new(e))),
        }
    }

    /// Recovery method to populate `wal_data` from a WAL file on disk (e.g. after crash).
    pub fn recover_wal(&mut self, wal_id: usize) -> Result<(), PyroError> {
        let base_path = self.output_dir.join(format!("wal_{}", wal_id));
        let records = recover(&base_path).map_err(|e| PyroError::local_io(CapturedError::new(e)))?;
        
        for rec in records {
            if let ExecutionRecord::Success { success, .. } = rec {
                self.wal_data.push(success.row.clone()).map_err(|e| PyroError::validation(CapturedError::new(e)))?;
            }
        }
        Ok(())
    }

    /// Writes a RecordBatch to an Arrow IPC file.
    fn write_arrow_ipc(&self, wal_id: usize, batch: &RecordBatch) -> Result<(), PyroError> {
        let path = self.output_dir.join(format!("batch_{}.arrow", wal_id));
        pyro_file::write_ipc(batch, &path).map_err(|e| PyroError::local_io(CapturedError::new(e)))?;
        info!("Written Arrow IPC file: {:?}", path);
        Ok(())
    }

    /// Converts pending Arrow IPC files into a single Parquet file.
    pub fn rollout_to_parquet(&mut self) -> Result<(), PyroError> {
        if self.ipc_files.is_empty() {
            return Ok(());
        }

        let rollout_id = self.current_wal_id;
        let mut all_batches = Vec::new();
        let mut rows_rolled_out = 0;

        let files_to_process: Vec<usize> = self.ipc_files.drain(..).collect();

        for wal_id in files_to_process {
            let arrow_path = self.output_dir.join(format!("batch_{}.arrow", wal_id));
            if !arrow_path.exists() {
                warn!("Arrow file not found: {:?}", arrow_path);
                continue;
            }

            let bytes = std::fs::read(&arrow_path).map_err(|e| PyroError::local_io(CapturedError::new(e)))?;
            let filename = arrow_path.file_name().unwrap().to_string_lossy();
            let batches_ipc = pyro_file::parse_data_to_batch_sync(bytes, &filename)
                .map_err(|e| PyroError::validation(CapturedError::new(e)))?;
            
            for b in batches_ipc {
                all_batches.push(b.to_batch());
            }

            // Track row count from the IPC file we are rolling out
            if let Some(count) = self.ipc_row_counts.remove(&wal_id) {
                rows_rolled_out += count;
            }

            let _ = std::fs::remove_file(&arrow_path);
        }
        
        if !all_batches.is_empty() {
            let parquet_path = self.output_dir.join(format!("rollout_{}.parquet", rollout_id));
            pyro_file::write_parquet(&all_batches, &parquet_path)
                .map_err(|e| PyroError::local_io(CapturedError::new(e)))?;
            info!("Converted Arrow IPCs to Parquet: {:?}", parquet_path);
            self.total_parquet_rows += rows_rolled_out;
        }

        Ok(())
    }

    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PyroRow, PyroValue};
    use crate::format::value::{PyroSchema, PyroField, PyroType, PrimitiveDataType};
    use tempfile::TempDir;

    fn setup_schema() -> PyroSchema<'static> {
        PyroSchema::new(vec![
            PyroField::new("id", PyroType::PrimitiveScalar(PrimitiveDataType::I32), false),
            PyroField::new("name", PyroType::Str, true),
        ])
    }

    fn make_success_record(row_index: usize, id: i32, name: &str) -> ExecutionRecord {
        let row = PyroRow::from([
            ("id", PyroValue::from(id)),
            ("name", PyroValue::from(name)),
        ])
        .into_owned();

        ExecutionRecord::Success {
            row_index,
            success: crate::format::PyroSuccess {
                row,
                logs: crate::format::PyroLogs::empty(),
            },
        }
    }

    #[test]
    fn test_data_manager_flow() {
        let dir = TempDir::new().unwrap();
        let schema = setup_schema();
        let mut manager = DataManager::new(dir.path().to_path_buf(), schema);
        manager.set_capacities(2, 2);

        // 1. Push records
        manager.push_record(make_success_record(0, 1, "alice")).unwrap();
        manager.push_record(make_success_record(1, 2, "bob")).unwrap();
        
        // Should have triggered flush_wal because capacity is 2
        // Current WAL id should be 1, wal_writer should be None (flushed)
        assert!(manager.wal_writer.is_none());
        assert_eq!(manager.ipc_files.len(), 1);
        
        // 2. Push more to trigger second IPC
        manager.push_record(make_success_record(2, 3, "charlie")).unwrap();
        manager.push_record(make_success_record(3, 4, "david")).unwrap();
        
        // Should have triggered flush_wal again
        assert_eq!(manager.ipc_files.len(), 2);
        
        // Now it should have triggered rollout_to_parquet because ipc_capacity is 2
        assert_eq!(manager.ipc_files.len(), 0);
        
        // Check if parquet file exists
        let parquet_path = dir.path().join("rollout_2.parquet");
        assert!(parquet_path.exists());
    }

    #[test]
    fn test_recovery() {
        let dir = TempDir::new().unwrap();
        let schema = setup_schema();
        let mut manager = DataManager::new(dir.path().to_path_buf(), schema);
        
        // Manually create a WAL file via push
        manager.push_record(make_success_record(0, 1, "alice")).unwrap();
        manager.push_record(make_success_record(1, 2, "bob")).unwrap();
        
        // We don't flush, so wal_data has 2 rows.
        // Now let's simulate a crash by creating a new manager and recovering
        let mut manager2 = DataManager::new(dir.path().to_path_buf(), setup_schema());
        manager2.restore().unwrap();
        
        assert_eq!(manager2.wal_data.len(), 2);
    }

    #[test]
    fn test_manual_flush_and_rollout() {
        let dir = TempDir::new().unwrap();
        let schema = setup_schema();
        let mut manager = DataManager::new(dir.path().to_path_buf(), schema);
        
        manager.push_record(make_success_record(0, 1, "alice")).unwrap();
        manager.flush_wal().unwrap();
        
        assert_eq!(manager.ipc_files.len(), 1);
        
        manager.rollout_to_parquet().unwrap();
        assert_eq!(manager.ipc_files.len(), 0);
        
        let parquet_path = dir.path().join("rollout_1.parquet");
        assert!(parquet_path.exists());
    }
}
