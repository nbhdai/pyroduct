use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

use crate::{PyroRow, PyroValue};
use crate::captured::CapturedError;
use crate::error::PyroError;
use crate::format::value::PyroSchema;
use crate::format::value::arrow::PreBatch;
use crate::format::value::arrow::Rowable;
use crate::format::value::arrow::wal::{WalWriter, recover};
use arrow::array::RecordBatch;
use pyro_file;

/// Manages the persistence of pipeline data, handling the transition from
/// WAL -> Arrow IPC (memmapable) -> Parquet.
pub struct DataManager {
    output_dir: PathBuf,
    _schema: PyroSchema<'static>,

    /// The in-memory buffer for accumulating rows before flushing to IPC.
    wal_data: PreBatch,

    /// The current WAL ID
    current_wal_id: usize,
    /// The wal writer for the current WAL.
    wal_writer: Option<WalWriter>,

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

    /// SQLite connection for indexing
    pub sqlite_conn: rusqlite::Connection,
    /// Optional metadata prefix for row data injection
    pub metadata_prefix: Option<String>,

    /// List of IPC file paths on disk.
    pub ipc_file_paths: Vec<PathBuf>,
    /// List of Parquet file paths on disk.
    pub parquet_file_paths: Vec<PathBuf>,
    /// Maps a global row_index to the index within the current active wal_data buffer.
    current_wal_rows: HashMap<usize, usize>,
}

impl DataManager {
    pub fn new(output_dir: impl AsRef<Path>, schema: PyroSchema<'static>) -> Self {
        let output_dir_buf = output_dir.as_ref().to_path_buf();
        let _ = std::fs::create_dir_all(&output_dir_buf);
        let db_path = output_dir_buf.join("index.db");
        let conn = rusqlite::Connection::open(db_path)
            .expect("Failed to open SQLite database for indexing");

        let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS wal_index (
                row_index INTEGER PRIMARY KEY,
                wal_id INTEGER NOT NULL,
                wal_index INTEGER NOT NULL,
                timestamp INTEGER NOT NULL
            )",
            [],
        );

        let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS ipc_index (
                wal_id INTEGER PRIMARY KEY
            )",
            [],
        );

        let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS parquet_index (
                parquet_id INTEGER PRIMARY KEY
            )",
            [],
        );

        let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS wal_to_parquet (
                wal_id INTEGER PRIMARY KEY,
                parquet_id INTEGER NOT NULL,
                FOREIGN KEY(wal_id) REFERENCES ipc_index(wal_id),
                FOREIGN KEY(parquet_id) REFERENCES parquet_index(parquet_id)
            )",
            [],
        );

        Self {
            output_dir: output_dir_buf,
            wal_data: PreBatch::new(schema.clone()),
            _schema: schema,
            current_wal_id: 0,
            wal_writer: None,
            ipc_files: Vec::new(),
            ipc_row_counts: HashMap::new(),
            total_parquet_rows: 0,
            wal_capacity: 10_000,
            ipc_capacity: 10,
            sqlite_conn: conn,
            metadata_prefix: None,
            ipc_file_paths: Vec::new(),
            parquet_file_paths: Vec::new(),
            current_wal_rows: HashMap::new(),
        }
    }

    /// Scans the output directory to restore state from existing files.
    /// This populates row counts for Parquet and IPC files and determines the current WAL ID.
    pub fn restore(&mut self) -> Result<(), PyroError> {
        let mut max_wal_id = 0;
        let mut total_parquet = 0;
        let mut ipc_counts = HashMap::new();

        let entries = std::fs::read_dir(&self.output_dir)
            .map_err(|e| PyroError::local_io(CapturedError::new(e)))?;

        for entry in entries {
            let entry = entry.map_err(|e| PyroError::local_io(CapturedError::new(e)))?;
            let path = entry.path();
            let filename = path.file_name().unwrap().to_string_lossy();

            if filename.starts_with("wal_") && filename.contains(".pyrowal") {
                // Extract ID from wal_N.pyrowal
                if let Some(id_str) = filename
                    .strip_prefix("wal_")
                    .and_then(|s| s.split('.').next())
                {
                    if let Ok(id) = id_str.parse::<usize>() {
                        max_wal_id = max_wal_id.max(id);
                    }
                }
            } else if filename.starts_with("batch_") && filename.ends_with(".arrow") {
                // Extract ID from batch_N.arrow
                if let Some(id_str) = filename
                    .strip_prefix("batch_")
                    .and_then(|s| s.split('.').next())
                {
                    if let Ok(id) = id_str.parse::<usize>() {
                        let bytes = std::fs::read(&path)
                            .map_err(|e| PyroError::local_io(CapturedError::new(e)))?;
                        let batches = pyro_file::parse_data_to_batch_sync(bytes, &filename)
                            .map_err(|e| PyroError::validation(CapturedError::new(e)))?;
                        let count: usize = batches.iter().map(|b| b.num_rows()).sum();
                        ipc_counts.insert(id, count);
                        self.ipc_file_paths.push(path.clone());

                        // Backfill SQLite ipc_index
                        let _ = self.sqlite_conn.execute(
                            "INSERT OR IGNORE INTO ipc_index (wal_id) VALUES (?1)",
                            rusqlite::params![id as i64],
                        );
                    }
                }
            } else if filename.starts_with("rollout_") && filename.ends_with(".parquet") {
                // Extract ID from rollout_N.parquet
                if let Some(id_str) = filename
                    .strip_prefix("rollout_")
                    .and_then(|s| s.split('.').next())
                {
                    if let Ok(id) = id_str.parse::<usize>() {
                        let bytes =
                            std::fs::read(&path).map_err(|e| PyroError::local_io(CapturedError::new(e)))?;
                        let filename_str = filename.to_string();
                        let batches = pyro_file::parse_data_to_batch_sync(bytes, &filename_str)
                            .map_err(|e| PyroError::validation(CapturedError::new(e)))?;
                        total_parquet += batches.iter().map(|b| b.num_rows()).sum::<usize>();
                        self.parquet_file_paths.push(path.clone());

                        // Backfill SQLite parquet_index
                        let _ = self.sqlite_conn.execute(
                            "INSERT OR IGNORE INTO parquet_index (parquet_id) VALUES (?1)",
                            rusqlite::params![id as i64],
                        );
                    }
                }
            }
        }

        self.ipc_file_paths.sort();
        self.parquet_file_paths.sort();

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

    pub fn set_metadata_prefix(&mut self, prefix: &str) {
        use crate::format::value::{PyroField, PyroSchema, PyroType, PrimitiveDataType};

        let metadata_field = PyroField::new(
            prefix.to_string(),
            PyroType::Group(std::borrow::Cow::Owned(vec![
                PyroField::new(
                    "index",
                    PyroType::PrimitiveScalar(PrimitiveDataType::U64),
                    false,
                ),
                PyroField::new(
                    "timestamp",
                    PyroType::Timestamp,
                    false,
                ),
            ])),
            false,
        );

        let mut fields = self._schema.fields().to_vec();
        fields.push(metadata_field);
        let new_schema = PyroSchema::new(fields);

        self.wal_data = PreBatch::new(new_schema.clone());
        self._schema = new_schema;
        self.metadata_prefix = Some(prefix.to_string());
    }

    /// Pushes a single WAL record to the current WAL and the in-memory buffer.
    pub fn push_record(&mut self, row_index: usize, record: &PyroRow<'_>) -> Result<(), PyroError> {
        if self.wal_writer.is_none() {
            self.open_next_wal()?;
        }

        let now = chrono::Utc::now();
        let timestamp = crate::format::value::Time::from(now);

        let mut record_to_push = record.clone().into_owned();

        if let Some(prefix) = &self.metadata_prefix {
            let metadata_row = PyroRow::from([
                ("index", PyroValue::U64(row_index as u64)),
                ("timestamp", PyroValue::Timestamp(timestamp)),
            ]);
            record_to_push.insert(prefix.clone(), PyroValue::Group(metadata_row));
        }

        let wal_index = self.wal_data.len();

        // 1. Write to WAL (Durability)
        if let Some(wal) = &mut self.wal_writer {
            wal.append(wal_index, &record_to_push)?;
        }

        // 2. Push to in-memory PreBatch (Performance)
        self.wal_data
            .push(record_to_push.clone())
            .map_err(|e| PyroError::validation(CapturedError::new(e)))?;

        // Populate fast lookup map for current active WAL
        self.current_wal_rows.insert(row_index, wal_index);

        // 3. Store in SQLite index
        let timestamp_nanos = timestamp.0;
        let _ = self.sqlite_conn.execute(
            "INSERT OR REPLACE INTO wal_index (row_index, wal_id, wal_index, timestamp) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![row_index as i64, self.current_wal_id as i64, wal_index as i64, timestamp_nanos as i64],
        );

        // 4. Check capacity for flush
        if self.wal_data.len() >= self.wal_capacity {
            self.flush_wal()?;
        }

        Ok(())
    }

    fn open_next_wal(&mut self) -> Result<(), PyroError> {
        self.current_wal_id += 1;
        let base_path = self.output_dir.join(format!("wal_{}", self.current_wal_id));
        let writer = WalWriter::open(base_path)?;
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

                let path = self.output_dir.join(format!("batch_{}.arrow", wal_id));
                self.ipc_file_paths.push(path);

                let _ = self.sqlite_conn.execute(
                    "INSERT OR IGNORE INTO ipc_index (wal_id) VALUES (?1)",
                    rusqlite::params![wal_id as i64],
                );

                self.current_wal_rows.clear();

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
        let records = recover(&base_path)?;

        for rec in records {
            self.wal_data
                .push(rec)
                .map_err(|e| PyroError::validation(CapturedError::new(e)))?;
        }

        let mut stmt = self.sqlite_conn.prepare(
            "SELECT row_index, wal_index FROM wal_index WHERE wal_id = ?"
        ).map_err(|e| PyroError::validation(CapturedError::new(e)))?;

        let rows = stmt.query_map([wal_id as i64], |r| {
            Ok((r.get::<_, i64>(0)? as usize, r.get::<_, i64>(1)? as usize))
        }).map_err(|e| PyroError::validation(CapturedError::new(e)))?;

        for row in rows {
            if let Ok((row_index, wal_index)) = row {
                self.current_wal_rows.insert(row_index, wal_index);
            }
        }

        Ok(())
    }

    /// Writes a RecordBatch to an Arrow IPC file.
    fn write_arrow_ipc(&self, wal_id: usize, batch: &RecordBatch) -> Result<(), PyroError> {
        let path = self.output_dir.join(format!("batch_{}.arrow", wal_id));
        pyro_file::write_ipc(batch, &path)
            .map_err(|e| PyroError::local_io(CapturedError::new(e)))?;
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

        for &wal_id in &files_to_process {
            let arrow_path = self.output_dir.join(format!("batch_{}.arrow", wal_id));
            if !arrow_path.exists() {
                warn!("Arrow file not found: {:?}", arrow_path);
                continue;
            }

            let bytes = std::fs::read(&arrow_path)
                .map_err(|e| PyroError::local_io(CapturedError::new(e)))?;
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

            self.ipc_file_paths.retain(|p| p != &arrow_path);
            let _ = std::fs::remove_file(&arrow_path);
        }

        if !all_batches.is_empty() {
            let parquet_path = self
                .output_dir
                .join(format!("rollout_{}.parquet", rollout_id));
            pyro_file::write_parquet(&all_batches, &parquet_path)
                .map_err(|e| PyroError::local_io(CapturedError::new(e)))?;
            info!("Converted Arrow IPCs to Parquet: {:?}", parquet_path);
            self.total_parquet_rows += rows_rolled_out;

            self.parquet_file_paths.push(parquet_path.clone());

            let _ = self.sqlite_conn.execute(
                "INSERT OR IGNORE INTO parquet_index (parquet_id) VALUES (?1)",
                rusqlite::params![rollout_id as i64],
            );

            for wal_id in &files_to_process {
                let _ = self.sqlite_conn.execute(
                    "INSERT OR REPLACE INTO wal_to_parquet (wal_id, parquet_id) VALUES (?1, ?2)",
                    rusqlite::params![*wal_id as i64, rollout_id as i64],
                );
            }
        }

        Ok(())
    }

    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }

    pub fn get_record(&self, index: usize) -> Result<PyroRow<'static>, PyroError> {
        // 1. Fast path: check in-memory current wal rows map (using index as global row_index)
        if let Some(&wal_idx) = self.current_wal_rows.get(&index) {
            if let Some(row) = self.wal_data.get(wal_idx) {
                return Ok(row.clone());
            }
        }

        // 2. Query sqlite by row_index (global) or offset (sequential)
        let mut stmt_global = self.sqlite_conn.prepare(
            "SELECT wal_id, wal_index FROM wal_index WHERE row_index = ?"
        ).map_err(|e| PyroError::validation(CapturedError::new(e)))?;

        let mut row_opt = stmt_global.query_row([index as i64], |r| {
            Ok((r.get::<_, i64>(0)? as usize, r.get::<_, i64>(1)? as usize))
        });

        if row_opt.is_err() {
            // Fallback: try sequential index (offset)
            let mut stmt_seq = self.sqlite_conn.prepare(
                "SELECT wal_id, wal_index FROM wal_index ORDER BY wal_id ASC, wal_index ASC LIMIT 1 OFFSET ?"
            ).map_err(|e| PyroError::validation(CapturedError::new(e)))?;

            row_opt = stmt_seq.query_row([index as i64], |r| {
                Ok((r.get::<_, i64>(0)? as usize, r.get::<_, i64>(1)? as usize))
            });
        }

        match row_opt {
            Ok((wal_id, wal_index)) => {
                // 3. Check if wal_id is rolled up in a parquet file
                let mut parquet_stmt = self.sqlite_conn.prepare(
                    "SELECT parquet_id FROM wal_to_parquet WHERE wal_id = ?"
                ).map_err(|e| PyroError::validation(CapturedError::new(e)))?;

                let parquet_opt = parquet_stmt.query_row([wal_id as i64], |r| {
                    Ok(r.get::<_, i64>(0)? as usize)
                });

                match parquet_opt {
                    Ok(parquet_id) => {
                        // Rolled up in Parquet file
                        let parquet_path = self.output_dir.join(format!("rollout_{}.parquet", parquet_id));
                        let bytes = std::fs::read(&parquet_path)
                            .map_err(|e| PyroError::local_io(CapturedError::new(e)))?;
                        let filename = parquet_path.file_name().unwrap().to_string_lossy().into_owned();
                        let batches = pyro_file::parse_data_to_batch_sync(bytes, &filename)
                            .map_err(|e| PyroError::validation(CapturedError::new(e)))?;

                        // Query DB for rows count in the same parquet file with wal_id < target_wal_id
                        let mut count_stmt = self.sqlite_conn.prepare(
                            "SELECT COUNT(*) FROM wal_index WHERE wal_id IN (
                                SELECT wal_id FROM wal_to_parquet WHERE parquet_id = ?1 AND wal_id < ?2
                            )"
                        ).map_err(|e| PyroError::validation(CapturedError::new(e)))?;

                        let prev_wals_row_count = count_stmt.query_row(
                            rusqlite::params![parquet_id as i64, wal_id as i64],
                            |r| r.get::<_, i64>(0)
                        ).unwrap_or(0) as usize;

                        let target_offset = prev_wals_row_count + wal_index;

                        let mut current_offset = 0;
                        for batch in batches {
                            let num_rows = batch.num_rows();
                            if target_offset >= current_offset && target_offset < current_offset + num_rows {
                                let batch_index = target_offset - current_offset;
                                let row = batch.row(batch_index)
                                    .map_err(|e| PyroError::validation(CapturedError::new(e)))?
                                    .into_owned();
                                return Ok(row);
                            }
                            current_offset += num_rows;
                        }

                        Err(PyroError::validation(CapturedError::new(format!(
                            "Offset {} not found in Parquet file {:?}",
                            target_offset,
                            parquet_path
                        ))))
                    }
                    Err(rusqlite::Error::QueryReturnedNoRows) => {
                        // Not rolled up in Parquet. Could be in current active in-memory WAL or Arrow IPC file.
                        if wal_id == self.current_wal_id {
                            if let Some(row) = self.wal_data.get(wal_index) {
                                return Ok(row.clone());
                            }
                        }

                        // Must be in IPC (Arrow) file
                        let arrow_path = self.output_dir.join(format!("batch_{}.arrow", wal_id));
                        let bytes = std::fs::read(&arrow_path)
                            .map_err(|e| PyroError::local_io(CapturedError::new(e)))?;
                        let filename = arrow_path.file_name().unwrap().to_string_lossy().into_owned();
                        let batches = pyro_file::parse_data_to_batch_sync(bytes, &filename)
                            .map_err(|e| PyroError::validation(CapturedError::new(e)))?;

                        let mut current_offset = 0;
                        for batch in batches {
                            let num_rows = batch.num_rows();
                            if wal_index >= current_offset && wal_index < current_offset + num_rows {
                                let batch_index = wal_index - current_offset;
                                let row = batch.row(batch_index)
                                    .map_err(|e| PyroError::validation(CapturedError::new(e)))?
                                    .into_owned();
                                return Ok(row);
                            }
                            current_offset += num_rows;
                        }

                        Err(PyroError::validation(CapturedError::new(format!(
                            "Offset {} not found in Arrow file {:?}",
                            wal_index,
                            arrow_path
                        ))))
                    }
                    Err(e) => Err(PyroError::validation(CapturedError::new(e))),
                }
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                Err(PyroError::validation(CapturedError::new(format!(
                    "Index {} out of bounds for DataManager",
                    index
                ))))
            }
            Err(e) => Err(PyroError::validation(CapturedError::new(e))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::value::{PrimitiveDataType, PyroField, PyroSchema, PyroType};
    use crate::{PyroRow, PyroValue};
    use tempfile::TempDir;

    fn setup_schema() -> PyroSchema<'static> {
        PyroSchema::new(vec![
            PyroField::new(
                "id",
                PyroType::PrimitiveScalar(PrimitiveDataType::I32),
                false,
            ),
            PyroField::new("name", PyroType::Str, true),
        ])
    }

    fn make_success_record(row_index: usize, id: i32, name: &'static str) -> PyroRow<'static> {
        PyroRow::from([("id", PyroValue::from(id)), ("index", PyroValue::from(row_index as u32)), ("name", PyroValue::from(name))])
    }

    #[test]
    fn test_data_manager_flow() {
        let dir = TempDir::new().unwrap();
        let schema = setup_schema();
        let mut manager = DataManager::new(dir.path(), schema);
        manager.set_capacities(2, 2);

        // 1. Push records
        manager
            .push_record(0, &make_success_record(0, 1, "alice"))
            .unwrap();
        manager
            .push_record(1, &make_success_record(1, 2, "bob"))
            .unwrap();

        // Should have triggered flush_wal because capacity is 2
        // Current WAL id should be 1, wal_writer should be None (flushed)
        assert!(manager.wal_writer.is_none());
        assert_eq!(manager.ipc_files.len(), 1);

        // 2. Push more to trigger second IPC
        manager
            .push_record(2, &make_success_record(2, 3, "charlie"))
            .unwrap();
        manager
            .push_record(3, &make_success_record(3, 4, "david"))
            .unwrap();

        // Should have triggered flush_wal again and automatically rolled out to parquet because ipc_capacity is 2
        assert_eq!(manager.ipc_files.len(), 0);

        // Check if parquet file exists
        let parquet_path = dir.path().join("rollout_2.parquet");
        assert!(parquet_path.exists());
    }

    #[test]
    fn test_recovery() {
        let dir = TempDir::new().unwrap();
        let schema = setup_schema();
        let mut manager = DataManager::new(dir.path(), schema);

        // Manually create a WAL file via push
        manager
            .push_record(0, &make_success_record(0, 1, "alice"))
            .unwrap();
        manager
            .push_record(1, &make_success_record(1, 2, "bob"))
            .unwrap();

        // We don't flush, so wal_data has 2 rows.
        // Now let's simulate a crash by creating a new manager and recovering
        let mut manager2 = DataManager::new(dir.path(), setup_schema());
        manager2.restore().unwrap();

        assert_eq!(manager2.wal_data.len(), 2);
    }

    #[test]
    fn test_manual_flush_and_rollout() {
        let dir = TempDir::new().unwrap();
        let schema = setup_schema();
        let mut manager = DataManager::new(dir.path(), schema);

        manager
            .push_record(0, &make_success_record(0, 1, "alice"))
            .unwrap();
        manager.flush_wal().unwrap();

        assert_eq!(manager.ipc_files.len(), 1);

        manager.rollout_to_parquet().unwrap();
        assert_eq!(manager.ipc_files.len(), 0);

        let parquet_path = dir.path().join("rollout_1.parquet");
        assert!(parquet_path.exists());
    }

    #[test]
    fn test_sqlite_index_and_metadata_prefix() {
        let dir = TempDir::new().unwrap();
        let schema = setup_schema();
        let mut manager = DataManager::new(dir.path(), schema);
        manager.set_metadata_prefix("pyro");

        manager
            .push_record(42, &make_success_record(42, 123, "test_metadata"))
            .unwrap();

        // Verify that the record inside wal_data has the metadata field
        let row = manager.get_record(0).unwrap();
        let pyro_group = row.get("pyro").unwrap();
        if let PyroValue::Group(group_row) = pyro_group {
            assert_eq!(group_row.get("index"), Some(&PyroValue::U64(42)));
            assert!(group_row.get("timestamp").is_some());
        } else {
            panic!("Expected PyroValue::Group under 'pyro' key");
        }

        // Verify SQLite contents
        let mut stmt = manager
            .sqlite_conn
            .prepare("SELECT row_index, wal_id, wal_index, timestamp FROM wal_index WHERE row_index = ?")
            .unwrap();
        let mut rows = stmt
            .query_map([42i64], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, i64>(3)?,
                ))
            })
            .unwrap();
        
        let row_data = rows.next().unwrap().unwrap();
        assert_eq!(row_data.0, 42); // row_index
        assert_eq!(row_data.1, 1);  // wal_id
        assert_eq!(row_data.2, 0);  // wal_index
        assert!(row_data.3 > 0);    // timestamp nanos i64
    }
}
