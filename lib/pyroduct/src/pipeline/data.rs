use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, warn};

use crate::captured::CapturedError;
use crate::error::PyroError;
use crate::format::value::PyroSchema;
use crate::format::value::arrow::PreBatch;
use crate::format::value::arrow::Rowable;
use crate::format::value::arrow::wal::{WalManager, WalWriter};
use crate::{PyroRow, PyroValue};
use arrow::array::RecordBatch;
use pyro_file;

/// Manages the persistence of pipeline data, handling the transition from
/// WAL -> Arrow IPC (memmapable) -> Parquet.
pub struct DataManager {
    output_dir: PathBuf,
    _schema: PyroSchema<'static>,

    /// The thread-safe WAL manager
    wal_manager: Option<WalManager>,

    /// The current WAL ID
    current_wal_id: usize,

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
    /// Shared state for tracking active readers and pending deletions of IPC files.
    pub shared_state: Arc<Mutex<DataManagerSharedState>>,
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
                wal_index INTEGER NOT NULL
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

        let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS session_status (
                session_id INTEGER PRIMARY KEY,
                status TEXT NOT NULL
            )",
            [],
        );

        Self {
            output_dir: output_dir_buf,
            _schema: schema,
            wal_manager: None,
            current_wal_id: 0,
            ipc_files: Vec::new(),
            ipc_row_counts: HashMap::new(),
            total_parquet_rows: 0,
            wal_capacity: 10_000,
            ipc_capacity: 10,
            sqlite_conn: conn,
            metadata_prefix: None,
            ipc_file_paths: Vec::new(),
            parquet_file_paths: Vec::new(),
            shared_state: Arc::new(Mutex::new(DataManagerSharedState::default())),
        }
    }

    /// Scans the output directory to restore state from existing files.
    /// This populates row counts for Parquet and IPC files and determines the current WAL ID.
    pub fn restore(&mut self) -> Result<(), PyroError> {
        let mut max_wal_id = 0;
        let mut total_parquet = 0;
        let mut ipc_counts = HashMap::new();

        let entries = std::fs::read_dir(&self.output_dir).map_err(|e| {
            PyroError::local_io(
                CapturedError::new("Failed to read output directory").with_source(e),
            )
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                PyroError::local_io(
                    CapturedError::new("Failed to read directory entry").with_source(e),
                )
            })?;
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
                        let bytes = std::fs::read(&path).map_err(|e| {
                            PyroError::local_io(
                                CapturedError::new("Failed to read Arrow IPC file").with_source(e),
                            )
                        })?;
                        let batches = pyro_file::parse_data_to_batch_sync(bytes, &filename)
                            .map_err(|e| {
                                PyroError::validation(
                                    CapturedError::new(
                                        "Failed to parse Arrow IPC file data to batches",
                                    )
                                    .with_source(e),
                                )
                            })?;
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
                        let bytes = std::fs::read(&path).map_err(|e| {
                            PyroError::local_io(
                                CapturedError::new("Failed to read Parquet rollout file")
                                    .with_source(e),
                            )
                        })?;
                        let filename_str = filename.to_string();
                        let batches = pyro_file::parse_data_to_batch_sync(bytes, &filename_str)
                            .map_err(|e| {
                                PyroError::validation(
                                    CapturedError::new(
                                        "Failed to parse Parquet rollout file data to batches",
                                    )
                                    .with_source(e),
                                )
                            })?;
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
            let wal_file_path = self
                .output_dir
                .join(format!("wal_{}.pyrowal", self.current_wal_id));
            if wal_file_path.exists() {
                let base_path = self.output_dir.join(format!("wal_{}", self.current_wal_id));
                let wm = WalManager::open_with_recovery(
                    &base_path,
                    self.wal_capacity,
                    self._schema.clone(),
                )?;
                self.wal_manager = Some(wm);
            }
        }

        Ok(())
    }

    /// Returns the total number of rows across all stages:
    /// Parquet files + IPC files + current in-memory buffer.
    pub fn len(&self) -> usize {
        let ipc_sum: usize = self.ipc_row_counts.values().sum();
        let active_len = if let Some(wm) = &self.wal_manager {
            let data = loop {
                if let Ok(g) = wm.data.try_lock() {
                    break g;
                }
                std::thread::yield_now();
            };
            data.prebatch.len()
        } else {
            0
        };
        self.total_parquet_rows + ipc_sum + active_len
    }

    pub fn set_capacities(&mut self, wal_capacity: usize, ipc_capacity: usize) {
        self.wal_capacity = wal_capacity;
        self.ipc_capacity = ipc_capacity;
    }

    pub fn set_metadata_prefix(&mut self, prefix: &str) {
        use crate::format::value::{PrimitiveDataType, PyroField, PyroSchema, PyroType};

        let metadata_field = PyroField::new(
            prefix.to_string(),
            PyroType::Group(std::borrow::Cow::Owned(vec![
                PyroField::new(
                    "index",
                    PyroType::PrimitiveScalar(PrimitiveDataType::U64),
                    false,
                ),
                PyroField::new("timestamp", PyroType::Timestamp, false),
            ])),
            false,
        );

        let mut fields = self._schema.fields().to_vec();
        fields.push(metadata_field);
        let new_schema = PyroSchema::new(fields);

        self._schema = new_schema;
        self.metadata_prefix = Some(prefix.to_string());

        if let Some(wm) = &mut self.wal_manager {
            let mut data = loop {
                if let Ok(g) = wm.data.try_lock() {
                    break g;
                }
                std::thread::yield_now();
            };
            data.prebatch = PreBatch::new(self._schema.clone());
        }
    }

    pub fn get_active_batch(&self) -> Result<Option<RecordBatch>, PyroError> {
        if let Some(wm) = &self.wal_manager {
            let data = loop {
                if let Ok(g) = wm.data.try_lock() {
                    break g;
                }
                std::thread::yield_now();
            };
            data.prebatch.to_record_batch().map_err(|e| {
                PyroError::validation(
                    CapturedError::new("Failed to serialize WAL to RecordBatch").with_source(e),
                )
            })
        } else {
            Ok(None)
        }
    }

    #[cfg(feature = "host")]
    pub fn sql_provider(
        &self,
    ) -> Result<crate::pipeline::sql::DataManagerTableProvider, PyroError> {
        let mut batches = Vec::new();

        // 1. Eagerly load Parquet files
        for path in &self.parquet_file_paths {
            let bytes = std::fs::read(path).map_err(|e| {
                PyroError::local_io(
                    CapturedError::new("Failed to read Parquet file for SQL provider")
                        .with_source(e),
                )
            })?;
            let filename = path.file_name().unwrap().to_string_lossy().into_owned();
            let parsed_batches =
                pyro_file::parse_data_to_batch_sync(bytes, &filename).map_err(|e| {
                    PyroError::validation(
                        CapturedError::new("Failed to parse Parquet file for SQL provider")
                            .with_source(e),
                    )
                })?;
            for b in parsed_batches {
                batches.push(b.to_batch());
            }
        }

        // 2. Eagerly load IPC files and track active readers with guards
        let mut guards = Vec::new();
        for path in &self.ipc_file_paths {
            // Register guard
            let guard = IpcFileGuard::new(path.clone(), self.shared_state.clone());
            guards.push(guard);

            let bytes = std::fs::read(path).map_err(|e| {
                PyroError::local_io(
                    CapturedError::new("Failed to read Arrow IPC file for SQL provider")
                        .with_source(e),
                )
            })?;
            let filename = path.file_name().unwrap().to_string_lossy().into_owned();
            let parsed_batches =
                pyro_file::parse_data_to_batch_sync(bytes, &filename).map_err(|e| {
                    PyroError::validation(
                        CapturedError::new("Failed to parse Arrow IPC file for SQL provider")
                            .with_source(e),
                    )
                })?;
            for b in parsed_batches {
                batches.push(b.to_batch());
            }
        }

        // 3. Eagerly load active WAL record batch
        if let Some(active_batch) = self.get_active_batch()? {
            batches.push(active_batch);
        }

        // 4. Construct MemTable
        let schema = std::sync::Arc::new(self._schema.to_arrow());
        let mem_table = datafusion::datasource::memory::MemTable::try_new(schema, vec![batches])
            .map_err(|e| {
                PyroError::validation(
                    CapturedError::new("Failed to create MemTable for SQL provider").with_source(e),
                )
            })?;

        Ok(crate::pipeline::sql::DataManagerTableProvider::new(
            mem_table, guards,
        ))
    }

    fn ensure_wal_manager(&mut self) -> Result<WalManager, PyroError> {
        if self.wal_manager.is_none() {
            self.current_wal_id += 1;
            let base_path = self.output_dir.join(format!("wal_{}", self.current_wal_id));
            let writer = WalWriter::open(&base_path)?;
            let wm = WalManager::new(writer, self.wal_capacity, self._schema.clone());
            self.wal_manager = Some(wm);
        }
        Ok(self.wal_manager.clone().unwrap())
    }

    /// Pushes a single WAL record to the current WAL and the in-memory buffer.
    pub async fn push_record(
        &mut self,
        row_index: usize,
        record: &PyroRow<'_>,
    ) -> Result<(), PyroError> {
        let wal_manager = self.ensure_wal_manager()?;

        debug!(
            row_index,
            wal_id = self.current_wal_id,
            "push_record: inserting record"
        );

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

        // 1. Write to WAL & Push to in-memory PreBatch
        wal_manager.append(row_index, &record_to_push).await?;

        // 2. Check capacity for flush
        let current_len = {
            let data = wal_manager.data.lock().await;
            data.prebatch.len()
        };

        if current_len >= self.wal_capacity {
            debug!(
                row_index,
                wal_data_len = current_len,
                wal_capacity = self.wal_capacity,
                "push_record: capacity reached, flushing WAL"
            );
            self.flush_wal().await?;
        }

        Ok(())
    }

    /// Rolls up the in-memory buffer into a memmapable Arrow IPC file.
    pub async fn flush_wal(&mut self) -> Result<(), PyroError> {
        let wal_id = self.current_wal_id;
        if self.wal_manager.is_none() {
            debug!(wal_id, "flush_wal: no active wal manager, nothing to flush");
            return Ok(());
        }

        debug!(wal_id, "flush_wal: flushing WAL to Arrow IPC");

        // 1. Prepare next WAL ID and writer
        let next_wal_id = wal_id + 1;
        let base_path = self.output_dir.join(format!("wal_{}", next_wal_id));
        let next_writer = WalWriter::open(&base_path)?;

        // 2. Rotate the WAL manager
        let (batch, old_rows) = {
            let wm = self.wal_manager.as_ref().unwrap();
            wm.rotate(next_writer, self._schema.clone()).await?
        };

        let row_count = batch.num_rows();

        // 3. Write Arrow IPC file
        debug!(wal_id, row_count, "flush_wal: writing record batch to IPC");
        self.write_arrow_ipc(wal_id, &batch)?;

        // 4. Batch insert of the mapping old_rows (HashMap<usize, usize>) into SQLite
        let tx_res: Result<(), PyroError> = {
            let tx = self.sqlite_conn.transaction().map_err(|e| {
                PyroError::validation(
                    CapturedError::new("Failed to start transaction").with_source(e),
                )
            })?;
            {
                let mut stmt = tx.prepare(
                    "INSERT OR REPLACE INTO wal_index (row_index, wal_id, wal_index) VALUES (?1, ?2, ?3)"
                ).map_err(|e| {
                    PyroError::validation(CapturedError::new("Failed to prepare insert statement").with_source(e))
                })?;
                for (&row_index, &wal_index) in &old_rows {
                    stmt.execute(rusqlite::params![
                        row_index as i64,
                        wal_id as i64,
                        wal_index as i64
                    ])
                    .map_err(|e| {
                        PyroError::validation(
                            CapturedError::new("Failed to execute insert statement").with_source(e),
                        )
                    })?;
                }
            }
            tx.commit().map_err(|e| {
                PyroError::validation(
                    CapturedError::new("Failed to commit transaction").with_source(e),
                )
            })?;
            Ok(())
        };

        if let Err(e) = tx_res {
            error!(wal_id, error = ?e, "flush_wal: failed to insert WAL rows into SQLite");
            return Err(e);
        }

        // 5. Delete the old WAL file
        let old_wal_path = self.output_dir.join(format!("wal_{}.pyrowal", wal_id));
        if old_wal_path.exists() {
            std::fs::remove_file(&old_wal_path).map_err(|e| {
                PyroError::local_io(CapturedError::new("Failed to delete WAL file").with_source(e))
            })?;
            debug!(
                wal_id,
                "flush_wal: successfully deleted WAL file {:?}", old_wal_path
            );
        }

        // 6. Update track variables
        self.current_wal_id = next_wal_id;
        self.ipc_files.push(wal_id);
        self.ipc_row_counts.insert(wal_id, row_count);

        let path = self.output_dir.join(format!("batch_{}.arrow", wal_id));
        self.ipc_file_paths.push(path);

        let _ = self.sqlite_conn.execute(
            "INSERT OR IGNORE INTO ipc_index (wal_id) VALUES (?1)",
            rusqlite::params![wal_id as i64],
        );

        if self.ipc_files.len() >= self.ipc_capacity {
            debug!(
                ipc_files_count = self.ipc_files.len(),
                ipc_capacity = self.ipc_capacity,
                "flush_wal: IPC capacity reached, rolling out to Parquet"
            );
            self.rollout_to_parquet()?;
        }

        Ok(())
    }

    /// Writes a RecordBatch to an Arrow IPC file.
    fn write_arrow_ipc(&self, wal_id: usize, batch: &RecordBatch) -> Result<(), PyroError> {
        let path = self.output_dir.join(format!("batch_{}.arrow", wal_id));
        pyro_file::write_ipc(batch, &path).map_err(|e| {
            PyroError::local_io(
                CapturedError::new("Failed to write RecordBatch to Arrow IPC file").with_source(e),
            )
        })?;
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

            let bytes = std::fs::read(&arrow_path).map_err(|e| {
                PyroError::local_io(
                    CapturedError::new("Failed to read Arrow IPC file for rollout").with_source(e),
                )
            })?;
            let filename = arrow_path.file_name().unwrap().to_string_lossy();
            let batches_ipc =
                pyro_file::parse_data_to_batch_sync(bytes, &filename).map_err(|e| {
                    PyroError::validation(
                        CapturedError::new("Failed to parse Arrow IPC file during rollout")
                            .with_source(e),
                    )
                })?;

            for b in batches_ipc {
                all_batches.push(b.to_batch());
            }

            // Track row count from the IPC file we are rolling out
            if let Some(count) = self.ipc_row_counts.remove(&wal_id) {
                rows_rolled_out += count;
            }

            self.ipc_file_paths.retain(|p| p != &arrow_path);
            {
                let mut state = self.shared_state.lock().unwrap();
                if state.active_readers.contains_key(&arrow_path) {
                    state.pending_deletions.insert(arrow_path.clone());
                    debug!(
                        "IPC file {:?} is currently being read. Deferring deletion.",
                        arrow_path
                    );
                } else {
                    let _ = std::fs::remove_file(&arrow_path);
                }
            }
        }

        if !all_batches.is_empty() {
            let parquet_path = self
                .output_dir
                .join(format!("rollout_{}.parquet", rollout_id));
            pyro_file::write_parquet(&all_batches, &parquet_path).map_err(|e| {
                PyroError::local_io(
                    CapturedError::new("Failed to write Parquet rollout file").with_source(e),
                )
            })?;
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

    pub fn schema(&self) -> &PyroSchema<'static> {
        &self._schema
    }

    pub fn set_session_status(&self, session_id: usize, status: &str) -> Result<(), PyroError> {
        self.sqlite_conn
            .execute(
                "INSERT OR REPLACE INTO session_status (session_id, status) VALUES (?1, ?2)",
                rusqlite::params![session_id as i64, status],
            )
            .map_err(|e| {
                PyroError::validation(
                    CapturedError::new("Failed to set session status").with_source(e),
                )
            })?;
        Ok(())
    }

    pub fn get_session_status(&self, session_id: usize) -> Result<Option<String>, PyroError> {
        let mut stmt = self
            .sqlite_conn
            .prepare("SELECT status FROM session_status WHERE session_id = ?")
            .map_err(|e| {
                PyroError::validation(
                    CapturedError::new("Failed to prepare SELECT statement for session_status")
                        .with_source(e),
                )
            })?;
        let status_opt = stmt.query_row([session_id as i64], |r| r.get::<_, String>(0));
        match status_opt {
            Ok(status) => Ok(Some(status)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(PyroError::validation(
                CapturedError::new("Failed to query session status").with_source(e),
            )),
        }
    }

    pub fn get_record(&self, index: usize) -> Result<PyroRow<'static>, PyroError> {
        debug!(index, "get_record: starting lookup");

        // 1. Fast path: check in-memory current wal rows map (using index as global row_index)
        if let Some(wm) = &self.wal_manager {
            let data = loop {
                if let Ok(g) = wm.data.try_lock() {
                    break g;
                }
                std::thread::yield_now();
            };
            if let Some(&wal_idx) = data.current_wal_rows.get(&index) {
                if let Some(row) = data.prebatch.get(wal_idx) {
                    debug!(
                        index,
                        wal_idx, "get_record: fast-path hit in current active WAL buffer"
                    );
                    return Ok(row.clone());
                }
            }
        }

        // 2. Query sqlite by row_index (global)
        debug!(index, "get_record: querying SQLite wal_index");
        let mut stmt_global = self
            .sqlite_conn
            .prepare("SELECT wal_id, wal_index FROM wal_index WHERE row_index = ?")
            .map_err(|e| {
                PyroError::validation(
                    CapturedError::new("Failed to prepare SQLite global index statement")
                        .with_source(e),
                )
            })?;

        let row_opt = stmt_global.query_row([index as i64], |r| {
            Ok((r.get::<_, i64>(0)? as usize, r.get::<_, i64>(1)? as usize))
        });

        match row_opt {
            Ok((wal_id, wal_index)) => {
                debug!(
                    index,
                    wal_id, wal_index, "get_record: SQLite index lookup succeeded"
                );

                let arrow_path = self.output_dir.join(format!("batch_{}.arrow", wal_id));
                if arrow_path.exists() {
                    let arrow_ipc = pyro_file::parse_ipc_file_mmap(&arrow_path).map_err(|e| {
                        PyroError::validation(
                            CapturedError::new("Failed to memory-map Arrow IPC file")
                                .with_source(e),
                        )
                    })?;

                    let mut stmt_min = self
                        .sqlite_conn
                        .prepare("SELECT MIN(wal_index) FROM wal_index WHERE wal_id = ?")
                        .map_err(|e| {
                            PyroError::validation(
                                CapturedError::new("Failed to prepare MIN wal_index statement")
                                    .with_source(e),
                            )
                        })?;
                    let min_wal_index: usize = stmt_min
                        .query_row([wal_id as i64], |r| r.get::<_, i64>(0))
                        .map_err(|e| {
                            PyroError::validation(
                                CapturedError::new("Failed to query MIN(wal_index)").with_source(e),
                            )
                        })? as usize;

                    let relative_idx = wal_index - min_wal_index;
                    let row = arrow_ipc.row(relative_idx).map_err(|e| {
                        PyroError::validation(
                            CapturedError::new("Failed to read row from memory-mapped Arrow IPC")
                                .with_source(e),
                        )
                    })?;
                    return Ok(row.into_owned());
                }

                // If the IPC file is missing, then it was rolled up into a parquet file
                let mut stmt_parquet = self
                    .sqlite_conn
                    .prepare("SELECT parquet_id FROM wal_to_parquet WHERE wal_id = ?")
                    .map_err(|e| {
                        PyroError::validation(
                            CapturedError::new("Failed to prepare statement for wal_to_parquet")
                                .with_source(e),
                        )
                    })?;
                let parquet_id: usize = match stmt_parquet
                    .query_row([wal_id as i64], |r| r.get::<_, i64>(0))
                {
                    Ok(id) => id as usize,
                    Err(rusqlite::Error::QueryReturnedNoRows) => {
                        return Err(PyroError::NotFound(format!("Index {} not found", index)));
                    }
                    Err(e) => {
                        return Err(PyroError::validation(
                            CapturedError::new("Failed to query parquet_id from wal_to_parquet")
                                .with_source(e),
                        ));
                    }
                };

                let parquet_path = self
                    .output_dir
                    .join(format!("rollout_{}.parquet", parquet_id));
                if !parquet_path.exists() {
                    return Err(PyroError::NotFound(format!(
                        "Parquet rollout file {:?} not found",
                        parquet_path
                    )));
                }

                let bytes = std::fs::read(&parquet_path).map_err(|e| {
                    PyroError::local_io(
                        CapturedError::new(
                            "Failed to read Parquet rollout file for record retrieval",
                        )
                        .with_source(e),
                    )
                })?;
                let filename = parquet_path.file_name().unwrap().to_string_lossy();
                let arrow_ipcs =
                    pyro_file::parse_data_to_batch_sync(bytes, &filename).map_err(|e| {
                        PyroError::validation(
                            CapturedError::new(
                                "Failed to parse Parquet rollout file data during record retrieval",
                            )
                            .with_source(e),
                        )
                    })?;

                let mut stmt_min_parquet = self.sqlite_conn
                    .prepare("SELECT MIN(wal_index) FROM wal_index WHERE wal_id IN (SELECT wal_id FROM wal_to_parquet WHERE parquet_id = ?)")
                    .map_err(|e| PyroError::validation(CapturedError::new("Failed to prepare statement for MIN(wal_index) in Parquet").with_source(e)))?;
                let min_parquet_index: usize = stmt_min_parquet
                    .query_row([parquet_id as i64], |r| r.get::<_, i64>(0))
                    .map_err(|e| {
                        PyroError::validation(
                            CapturedError::new("Failed to query MIN(wal_index) in Parquet rollout")
                                .with_source(e),
                        )
                    })? as usize;

                let relative_idx = wal_index - min_parquet_index;

                let mut current_offset = 0;
                for arrow_ipc in &arrow_ipcs {
                    let num_rows = arrow_ipc.num_rows();
                    if relative_idx >= current_offset && relative_idx < current_offset + num_rows {
                        let row_idx_in_batch = relative_idx - current_offset;
                        let row = arrow_ipc.row(row_idx_in_batch).map_err(|e| {
                            PyroError::validation(
                                CapturedError::new("Failed to read row from Parquet batch")
                                    .with_source(e),
                            )
                        })?;
                        return Ok(row.into_owned());
                    }
                    current_offset += num_rows;
                }

                Err(PyroError::NotFound(format!("Index {} not found", index)))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                debug!(
                    index,
                    "get_record: SQLite lookup returned no rows; returning PyroError::NotFound"
                );
                Err(PyroError::NotFound(format!("Index {} not found", index)))
            }
            Err(e) => {
                error!(index, error = ?e, "get_record: SQLite global query error");
                Err(PyroError::validation(
                    CapturedError::new("Failed to query global index in SQLite").with_source(e),
                ))
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct DataManagerSharedState {
    /// Maps file path to the number of active queries/table providers currently reading it.
    pub active_readers: HashMap<PathBuf, usize>,
    /// Files that have been rolled out to Parquet but are still held by active readers.
    /// These will be deleted when their reader count drops to 0.
    pub pending_deletions: HashSet<PathBuf>,
}

impl DataManagerSharedState {
    pub fn add_reader(&mut self, path: PathBuf) {
        *self.active_readers.entry(path).or_insert(0) += 1;
    }

    pub fn remove_reader(&mut self, path: &Path) {
        if let std::collections::hash_map::Entry::Occupied(mut entry) =
            self.active_readers.entry(path.to_path_buf())
        {
            *entry.get_mut() -= 1;
            if *entry.get() == 0 {
                entry.remove();
                if self.pending_deletions.remove(path) {
                    if path.exists() {
                        let _ = std::fs::remove_file(path);
                        tracing::debug!("Deferred deletion of IPC file finished: {:?}", path);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct IpcFileGuard {
    pub path: PathBuf,
    pub shared_state: Arc<Mutex<DataManagerSharedState>>,
}

impl IpcFileGuard {
    pub fn new(path: PathBuf, shared_state: Arc<Mutex<DataManagerSharedState>>) -> Self {
        if let Ok(mut state) = shared_state.lock() {
            state.add_reader(path.clone());
        }
        Self { path, shared_state }
    }
}

impl Clone for IpcFileGuard {
    fn clone(&self) -> Self {
        if let Ok(mut state) = self.shared_state.lock() {
            state.add_reader(self.path.clone());
        }
        Self {
            path: self.path.clone(),
            shared_state: self.shared_state.clone(),
        }
    }
}

impl Drop for IpcFileGuard {
    fn drop(&mut self) {
        if let Ok(mut state) = self.shared_state.lock() {
            state.remove_reader(&self.path);
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
        PyroRow::from([
            ("id", PyroValue::from(id)),
            ("index", PyroValue::from(row_index as u32)),
            ("name", PyroValue::from(name)),
        ])
    }

    #[tokio::test]
    async fn test_data_manager_flow() {
        let dir = TempDir::new().unwrap();
        let schema = setup_schema();
        let mut manager = DataManager::new(dir.path(), schema);
        manager.set_capacities(2, 2);

        // 1. Push records
        manager
            .push_record(0, &make_success_record(0, 1, "alice"))
            .await
            .unwrap();
        manager
            .push_record(1, &make_success_record(1, 2, "bob"))
            .await
            .unwrap();

        // Should have triggered flush_wal because capacity is 2
        // Current WAL id should be 2, wal_manager should be Some (rotated to wal_2)
        assert!(manager.wal_manager.is_some());
        assert_eq!(manager.current_wal_id, 2);
        assert_eq!(manager.ipc_files.len(), 1);

        // 2. Push more to trigger second IPC
        manager
            .push_record(2, &make_success_record(2, 3, "charlie"))
            .await
            .unwrap();
        manager
            .push_record(3, &make_success_record(3, 4, "david"))
            .await
            .unwrap();

        // Should have triggered flush_wal again and automatically rolled out to parquet because ipc_capacity is 2
        assert_eq!(manager.ipc_files.len(), 0);

        // Check if parquet file exists
        let parquet_path = dir.path().join("rollout_3.parquet");
        assert!(parquet_path.exists());
    }

    #[tokio::test]
    async fn test_recovery() {
        let dir = TempDir::new().unwrap();
        let schema = setup_schema();
        let mut manager = DataManager::new(dir.path(), schema);

        // Manually create a WAL file via push
        manager
            .push_record(0, &make_success_record(0, 1, "alice"))
            .await
            .unwrap();
        manager
            .push_record(1, &make_success_record(1, 2, "bob"))
            .await
            .unwrap();

        // Gracefully shut down the first manager's WAL writer to flush everything to disk
        if let Some(wm) = manager.wal_manager.take() {
            wm.interrupt().await.unwrap();
        }

        // We don't flush, so WAL has 2 rows.
        // Now let's simulate a crash by creating a new manager and recovering
        let mut manager2 = DataManager::new(dir.path(), setup_schema());
        manager2.restore().unwrap();

        assert_eq!(manager2.len(), 2);
        let active_len = {
            let data = loop {
                if let Ok(g) = manager2.wal_manager.as_ref().unwrap().data.try_lock() {
                    break g;
                }
                std::thread::yield_now();
            };
            data.prebatch.len()
        };
        assert_eq!(active_len, 2);
    }

    #[tokio::test]
    async fn test_manual_flush_and_rollout() {
        let dir = TempDir::new().unwrap();
        let schema = setup_schema();
        let mut manager = DataManager::new(dir.path(), schema);

        manager
            .push_record(0, &make_success_record(0, 1, "alice"))
            .await
            .unwrap();
        manager.flush_wal().await.unwrap();

        assert_eq!(manager.ipc_files.len(), 1);

        manager.rollout_to_parquet().unwrap();
        assert_eq!(manager.ipc_files.len(), 0);

        let parquet_path = dir.path().join("rollout_2.parquet");
        assert!(parquet_path.exists());
    }

    #[tokio::test]
    async fn test_sqlite_index_and_metadata_prefix() {
        let dir = TempDir::new().unwrap();
        let schema = setup_schema();
        let mut manager = DataManager::new(dir.path(), schema);
        manager.set_metadata_prefix("pyro");

        manager
            .push_record(42, &make_success_record(42, 123, "test_metadata"))
            .await
            .unwrap();

        // Verify that the record inside in-memory WAL has the metadata field
        let row = manager.get_record(42).unwrap();
        let pyro_group = row.get("pyro").unwrap();
        if let PyroValue::Group(group_row) = pyro_group {
            assert_eq!(group_row.get("index"), Some(&PyroValue::U64(42)));
            assert!(group_row.get("timestamp").is_some());
        } else {
            panic!("Expected PyroValue::Group under 'pyro' key");
        }

        // Flush WAL to batch insert the row index mapping to SQLite and write Arrow IPC file
        manager.flush_wal().await.unwrap();

        // Verify SQLite contents
        let mut stmt = manager
            .sqlite_conn
            .prepare("SELECT row_index, wal_id, wal_index FROM wal_index WHERE row_index = ?")
            .unwrap();
        let mut rows = stmt
            .query_map([42i64], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })
            .unwrap();

        let row_data = rows.next().unwrap().unwrap();
        assert_eq!(row_data.0, 42); // row_index
        assert_eq!(row_data.1, 1); // wal_id
        assert_eq!(row_data.2, 0); // wal_index
    }

    #[tokio::test]
    async fn test_get_record_not_found_on_odd_entries() {
        let dir = TempDir::new().unwrap();
        let schema = setup_schema();
        let mut manager = DataManager::new(dir.path(), schema);

        // Put data with indices 0, 2, 4
        manager
            .push_record(0, &make_success_record(0, 10, "alice"))
            .await
            .unwrap();
        manager
            .push_record(2, &make_success_record(2, 20, "bob"))
            .await
            .unwrap();
        manager
            .push_record(4, &make_success_record(4, 30, "charlie"))
            .await
            .unwrap();

        // Get 0, 1, 2, 3, 4, 5
        // 0 -> Success
        let r0 = manager.get_record(0).unwrap();
        assert_eq!(r0.get("id"), Some(&PyroValue::from(10)));

        // 1 -> NotFound
        let r1 = manager.get_record(1);
        assert!(matches!(r1, Err(PyroError::NotFound(_))));

        // 2 -> Success
        let r2 = manager.get_record(2).unwrap();
        assert_eq!(r2.get("id"), Some(&PyroValue::from(20)));

        // 3 -> NotFound
        let r3 = manager.get_record(3);
        assert!(matches!(r3, Err(PyroError::NotFound(_))));

        // 4 -> Success
        let r4 = manager.get_record(4).unwrap();
        assert_eq!(r4.get("id"), Some(&PyroValue::from(30)));

        // 5 -> NotFound
        let r5 = manager.get_record(5);
        assert!(matches!(r5, Err(PyroError::NotFound(_))));
    }
}
