//! File I/O and data parsing for Pyroduct.
//!
//! This crate handles reading, writing, and serializing data across multiple formats:
//! - **Arrow IPC** (Feather) — binary format with in-memory and memory-mapped support
//! - **CSV** — with transparent chunking for large datasets
//! - **JSONL** — line-delimited JSON with chunking support
//! - **Parquet** — columnar storage for efficient querying
//!
//! Core types:
//! - [`ArrowIpc`] — memory-mapped or in-memory Arrow IPC data wrapper
//! - [`parse_data_to_batch`] — async entry point for parsing any supported format
//! - [`write_parquet`], [`write_csv`], [`write_jsonl`] — serialization functions
//!
//! All parsing operations return `RecordBatch` data wrapped in `ArrowIpc`, which
//! provides zero-copy deref access and SHA-256 content hashing.

use arrow::array::RecordBatch;
use arrow::buffer::Buffer;
use arrow::datatypes::{Field, Schema};
use arrow::ipc::{
    convert::fb_to_schema,
    reader::{FileDecoder, read_footer_length},
    root_as_footer,
    writer::FileWriter,
};
use arrow::{csv, json as arrow_json};
use bytes::Bytes;
use memmap2::Mmap;
use parquet::arrow::ArrowWriter;
use parquet::file::properties::WriterProperties;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Cursor, Seek, SeekFrom, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::NamedTempFile;
use thiserror::Error;
use tokio::task::spawn_blocking;
use tracing::{debug, info, instrument, warn};

// -----------------------------------------------------------------------------
// 1. Error Definitions
// -----------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum DataError {
    #[error("Empty")]
    Empty,

    #[error("I/O operation failed: {0}")]
    Io(#[from] std::io::Error),

    #[error("Arrow processing failed: {0}")]
    Arrow(#[source] Arc<arrow::error::ArrowError>),

    #[error("Parquet serialization failed: {0}")]
    Parquet(#[source] Arc<parquet::errors::ParquetError>),

    #[error("Unsupported file extension: '{0}'. Supported formats: .json, .csv, .ipc, .arrow")]
    UnsupportedType(String),

    #[error("Invalid data content: {0}")]
    InvalidContent(String),

    #[error("Batch count error: Expected exactly 1 RecordBatch, but found {0}")]
    UnexpectedBatchCount(usize),

    #[error("Failed to execute blocking task: {0}")]
    TaskJoin(String),

    #[error("Schema mismatch: expected {0}, got {1}")]
    SchemaMismatch(String, String),

    #[error("Data serialization failed: {0}")]
    Serialization(String),
}

impl From<arrow::error::ArrowError> for DataError {
    fn from(err: arrow::error::ArrowError) -> Self {
        DataError::Arrow(Arc::new(err))
    }
}

impl From<parquet::errors::ParquetError> for DataError {
    fn from(err: parquet::errors::ParquetError) -> Self {
        DataError::Parquet(Arc::new(err))
    }
}

// -----------------------------------------------------------------------------
// 2. Helpers
// -----------------------------------------------------------------------------

/// Serializes a RecordBatch into Arrow IPC format (File/Feather format).
pub fn record_batch_to_bytes(batch: &RecordBatch) -> Result<Vec<u8>, DataError> {
    let mut buffer = Vec::new();
    {
        let mut writer = FileWriter::try_new(&mut buffer, &batch.schema())?;
        writer.write(batch)?;
        writer.finish()?;
    }
    Ok(buffer)
}

/// Writes a RecordBatch to Arrow IPC format (File/Feather format).
pub fn write_ipc<P: AsRef<Path>>(batch: &RecordBatch, path: P) -> Result<(), DataError> {
    let bytes = record_batch_to_bytes(batch)?;
    std::fs::write(path, bytes)?;
    Ok(())
}

fn get_chunk_path(base_path: &Path, chunk_index: Option<usize>) -> PathBuf {
    match chunk_index {
        Some(idx) => {
            let stem = base_path.file_stem().unwrap_or_default().to_string_lossy();
            let ext = base_path.extension().unwrap_or_default().to_string_lossy();
            let parent = base_path.parent().unwrap_or_else(|| Path::new("."));
            if ext.is_empty() {
                parent.join(format!("{}_part_{}", stem, idx))
            } else {
                parent.join(format!("{}_part_{}.{}", stem, idx, ext))
            }
        }
        None => base_path.to_path_buf(),
    }
}

// -----------------------------------------------------------------------------
// 3. Export Functions
// -----------------------------------------------------------------------------

/// Writes a list of RecordBatches to a single Parquet file.
#[instrument(skip(batches, path), level = "debug")]
pub fn write_parquet<P: AsRef<Path>>(batches: &[RecordBatch], path: P) -> Result<(), DataError> {
    if batches.is_empty() {
        return Ok(());
    }
    let schema = batches[0].schema();

    let file = File::create(path)?;
    let props = WriterProperties::builder().build();
    let mut writer = ArrowWriter::try_new(file, schema, Some(props))?;

    for batch in batches {
        writer.write(batch)?;
    }
    writer.close()?;

    Ok(())
}

/// Writes a list of RecordBatches to CSV.
///
/// Handles chunking transparently: if `chunk_size` is provided, the data from
/// the vector of batches is streamed continuously. If a batch overlaps a chunk
/// boundary, it is sliced so that rows flow into the correct files.
#[instrument(skip(batches, path), level = "debug")]
pub fn write_csv<P: AsRef<Path>>(
    batches: &[RecordBatch],
    path: P,
    chunk_size: Option<usize>,
) -> Result<Vec<PathBuf>, DataError> {
    if batches.is_empty() {
        return Ok(vec![]);
    }

    // Prepare state
    let mut paths = Vec::new();
    let mut current_writer: Option<csv::Writer<File>> = None;
    let mut current_file_idx = 0;
    let mut current_chunk_rows = 0;

    // Iterate over logic slices
    // We use a helper iterator/logic to drive the writer
    let effective_chunk_size = chunk_size.unwrap_or(usize::MAX);
    if effective_chunk_size == 0 {
        return Err(DataError::InvalidContent(
            "Chunk size cannot be zero".into(),
        ));
    }

    for batch in batches {
        let mut offset = 0;
        while offset < batch.num_rows() {
            // 1. Ensure Writer Exists
            if current_writer.is_none() {
                let suffix = if chunk_size.is_some() {
                    Some(current_file_idx)
                } else {
                    None
                };
                let out_path = get_chunk_path(path.as_ref(), suffix);
                let file = File::create(&out_path)?;
                current_writer = Some(csv::WriterBuilder::new().with_header(true).build(file));
                paths.push(out_path);
            }

            // 2. Calculate slice size
            let rows_remaining_in_batch = batch.num_rows() - offset;
            let rows_remaining_in_chunk = effective_chunk_size - current_chunk_rows;
            let write_len = std::cmp::min(rows_remaining_in_batch, rows_remaining_in_chunk);

            // 3. Write slice
            let slice = batch.slice(offset, write_len);
            if let Some(w) = current_writer.as_mut() {
                w.write(&slice)?;
            }

            // 4. Update counters
            offset += write_len;
            current_chunk_rows += write_len;

            // 5. Rotate file if chunk full
            if current_chunk_rows >= effective_chunk_size {
                current_writer = None; // Drop writer (closes file)
                current_file_idx += 1;
                current_chunk_rows = 0;
            }
        }
    }

    Ok(paths)
}

/// Writes a list of RecordBatches to JSONL (NewLine Delimited JSON).
///
/// Handles chunking transparently across multiple batches.
#[instrument(skip(batches, path), level = "debug")]
pub fn write_jsonl<P: AsRef<Path>>(
    batches: &[RecordBatch],
    path: P,
    chunk_size: Option<usize>,
) -> Result<Vec<PathBuf>, DataError> {
    if batches.is_empty() {
        return Ok(vec![]);
    }

    let mut paths = Vec::new();
    let mut current_writer: Option<arrow_json::LineDelimitedWriter<File>> = None;
    let mut current_file_idx = 0;
    let mut current_chunk_rows = 0;

    let effective_chunk_size = chunk_size.unwrap_or(usize::MAX);
    if effective_chunk_size == 0 {
        return Err(DataError::InvalidContent(
            "Chunk size cannot be zero".into(),
        ));
    }

    for batch in batches {
        let mut offset = 0;
        while offset < batch.num_rows() {
            // 1. Ensure Writer Exists
            if current_writer.is_none() {
                let suffix = if chunk_size.is_some() {
                    Some(current_file_idx)
                } else {
                    None
                };
                let out_path = get_chunk_path(path.as_ref(), suffix);
                let file = File::create(&out_path)?;
                current_writer = Some(arrow_json::LineDelimitedWriter::new(file));
                paths.push(out_path);
            }

            // 2. Calculate slice
            let rows_remaining_in_batch = batch.num_rows() - offset;
            let rows_remaining_in_chunk = effective_chunk_size - current_chunk_rows;
            let write_len = std::cmp::min(rows_remaining_in_batch, rows_remaining_in_chunk);

            // 3. Write slice
            let slice = batch.slice(offset, write_len);
            if let Some(w) = current_writer.as_mut() {
                w.write(&slice)?;
            }

            // 4. Update counters
            offset += write_len;
            current_chunk_rows += write_len;

            // 5. Rotate file if chunk full
            if current_chunk_rows >= effective_chunk_size {
                // JSON writer usually needs explicit finish
                if let Some(mut w) = current_writer.take() {
                    w.finish()?;
                }
                current_file_idx += 1;
                current_chunk_rows = 0;
            }
        }
    }

    // Ensure final file is finished properly
    if let Some(mut w) = current_writer {
        w.finish()?;
    }

    Ok(paths)
}

// -----------------------------------------------------------------------------
// 4. ArrowIpc Struct
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ArrowIpc {
    /// The parsed batch, ready for instant O(1) access.
    inner: RecordBatch,
    /// The backing store (always valid IPC bytes).
    source: Bytes,
}

// Result-free access to the batch
impl Deref for ArrowIpc {
    type Target = RecordBatch;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl TryFrom<Vec<u8>> for ArrowIpc {
    type Error = DataError;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let bytes = Bytes::from(bytes);
        let buffer = Buffer::from(bytes.clone());
        let batch = parse_footer_and_batch(buffer)?;

        Ok(Self {
            inner: batch,
            source: bytes,
        })
    }
}

impl TryFrom<Bytes> for ArrowIpc {
    type Error = DataError;

    fn try_from(bytes: Bytes) -> Result<Self, Self::Error> {
        let buffer = Buffer::from(bytes.clone());
        let batch = parse_footer_and_batch(buffer)?;

        Ok(Self {
            inner: batch,
            source: bytes,
        })
    }
}

impl ArrowIpc {
    /// Creates an ArrowIpc from an existing RecordBatch.
    pub fn from_batch(batch: RecordBatch) -> Result<Self, DataError> {
        debug!("Serializing RecordBatch to initialize ArrowIpc");
        let bytes = Bytes::from(record_batch_to_bytes(&batch)?);

        Ok(Self {
            inner: batch,
            source: bytes,
        })
    }

    pub fn to_batch(self) -> RecordBatch {
        self.inner
    }

    pub fn bytes(&self) -> &Bytes {
        &self.source
    }

    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&self.source);
        let result = hasher.finalize();
        result.into()
    }

    #[instrument(skip(self), level = "info")]
    pub fn memmap<P: AsRef<Path> + std::fmt::Debug>(&mut self, path: P) -> Result<(), DataError> {
        let path = path.as_ref();
        info!(path = %path.display(), "Memmapping ArrowIpc data");

        {
            let mut file = File::create(path)?;
            file.write_all(self.bytes())?;
        }

        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        let bytes = Bytes::from_owner(mmap);
        let buffer = Buffer::from(bytes.clone());
        let new_batch = parse_footer_and_batch(buffer)?;

        self.inner = new_batch;
        self.source = bytes;

        Ok(())
    }
}

/// Reads an Arrow IPC file from disk using memory-mapping.
/// This provides zero-copy access to the record batch data.
pub fn parse_ipc_file_mmap<P: AsRef<Path>>(path: P) -> Result<ArrowIpc, DataError> {
    let file = File::open(path)?;
    let mmap = unsafe { Mmap::map(&file)? };
    let bytes = Bytes::from_owner(mmap);
    ArrowIpc::try_from(bytes)
}

// -----------------------------------------------------------------------------
// 5. Main Async Parser
// -----------------------------------------------------------------------------

#[instrument(skip(data), fields(filename = %filename, size = data.len()))]
pub async fn parse_data_to_batch(
    data: Vec<u8>,
    filename: &str,
) -> Result<Vec<ArrowIpc>, DataError> {
    let filename = filename.to_owned();
    let result = spawn_blocking(move || inter_parse_data_to_batch(data, &filename)).await;

    match result {
        Ok(Ok(inner)) => {
            if inner.is_empty() || inner.iter().all(|b| b.num_rows() == 0) {
                Err(DataError::Empty)
            } else {
                Ok(inner)
            }
        }
        Ok(error) => error,
        Err(e) => Err(DataError::TaskJoin(e.to_string())),
    }
}

/// Synchronous version of parse_data_to_batch.
pub fn parse_data_to_batch_sync(data: Vec<u8>, filename: &str) -> Result<Vec<ArrowIpc>, DataError> {
    let inner = inter_parse_data_to_batch(data, filename)?;
    if inner.is_empty() || inner.iter().all(|b| b.num_rows() == 0) {
        Err(DataError::Empty)
    } else {
        Ok(inner)
    }
}

fn inter_parse_data_to_batch(data: Vec<u8>, filename: &str) -> Result<Vec<ArrowIpc>, DataError> {
    let extension = Path::new(filename)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_lowercase();

    match extension.as_str() {
        "ipc" | "arrow" => {
            info!("File detected as IPC/Arrow.");
            Ok(vec![ArrowIpc::try_from(data)?])
        }
        "parquet" => {
            debug!("Parsing Parquet...");
            let bytes = Bytes::from(data);
            let builder =
                parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder::try_new(bytes)?;
            let reader = builder.build()?;
            let mut batches = Vec::new();
            for batch_res in reader {
                let batch = batch_res?;
                batches.push(ArrowIpc::from_batch(batch)?);
            }
            Ok(batches)
        }
        "csv" => {
            debug!("Parsing CSV...");
            let mut temp_file = NamedTempFile::new()?;
            temp_file.write_all(&data)?;
            let path_str = temp_file.path().to_str().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid Path")
            })?;

            let schema = csv::reader::infer_schema_from_files(
                &[path_str.to_string()],
                b',',
                Some(100),
                true,
            )?;
            let fields = schema
                .fields()
                .iter()
                .map(|f| {
                    Arc::new(Field::new(
                        f.name().trim().to_owned(),
                        f.data_type().to_owned(),
                        true,
                    ))
                })
                .collect::<Vec<_>>();
            let schema = Schema::new(fields);

            let file = File::open(temp_file.path())?;
            let reader = csv::ReaderBuilder::new(schema.into())
                .with_header(true)
                .with_delimiter(b',')
                .build(file)?;

            let batches = reader.collect::<Result<Vec<_>, _>>()?;
            batches.into_iter().map(ArrowIpc::from_batch).collect()
        }
        "json" | "jsonl" => {
            debug!("Parsing JSON...");
            let mut cursor = Cursor::new(data);
            let (schema, _) =
                arrow_json::reader::infer_json_schema(BufReader::new(&mut cursor), None)?;
            cursor.seek(SeekFrom::Start(0))?;
            let reader =
                arrow_json::ReaderBuilder::new(schema.into()).build(BufReader::new(cursor))?;

            reader.map(|r| ArrowIpc::from_batch(r?)).collect()
        }
        _ => Err(DataError::UnsupportedType(extension)),
    }
}

fn parse_footer_and_batch(buffer: Buffer) -> Result<RecordBatch, DataError> {
    if buffer.len() < 10 {
        return Err(DataError::InvalidContent(
            "File too small to be valid IPC".to_string(),
        ));
    }

    let trailer_start = buffer.len() - 10;
    let footer_len = read_footer_length(buffer[trailer_start..].try_into().unwrap())?;

    if trailer_start < footer_len {
        return Err(DataError::InvalidContent(
            "Footer length invalid".to_string(),
        ));
    }

    let footer = root_as_footer(&buffer[trailer_start - footer_len..trailer_start])
        .map_err(|e| DataError::InvalidContent(format!("Invalid Flatbuffer footer: {}", e)))?;

    let schema = Arc::new(fb_to_schema(footer.schema().unwrap()));
    let mut decoder = FileDecoder::new(schema, footer.version());

    if let Some(dicts) = footer.dictionaries() {
        for block in dicts.iter() {
            let block_len = block.bodyLength() as usize + block.metaDataLength() as usize;
            let data = buffer.slice_with_length(block.offset() as _, block_len);
            decoder.read_dictionary(block, &data)?;
        }
    }

    let batches_block = footer.recordBatches().unwrap_or_default();
    if batches_block.len() != 1 {
        return Err(DataError::UnexpectedBatchCount(batches_block.len()));
    }

    let block = batches_block.get(0);
    let block_len = block.bodyLength() as usize + block.metaDataLength() as usize;
    let data = buffer.slice_with_length(block.offset() as _, block_len);

    let batch = decoder
        .read_record_batch(block, &data)
        .transpose()
        .ok_or_else(|| DataError::InvalidContent("Failed to read batch".into()))??;

    Ok(batch)
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::StringArray;
    use arrow::datatypes::{DataType, Field, Schema};
    use arrow::ipc::writer::FileWriter;
    use std::fs::read_to_string;
    use std::sync::Arc;
    use tempfile::NamedTempFile;

    fn create_primary_batch(inputs: &[&str], targets: &[&str]) -> RecordBatch {
        let schema = Schema::new(vec![
            Field::new("input", DataType::Utf8, true),
            Field::new("target", DataType::Utf8, true),
        ]);

        let inputs = StringArray::from(inputs.iter().map(|&s| s.to_string()).collect::<Vec<_>>());
        let targets = StringArray::from(targets.iter().map(|&s| s.to_string()).collect::<Vec<_>>());

        RecordBatch::try_new(Arc::new(schema), vec![Arc::new(inputs), Arc::new(targets)]).unwrap()
    }

    #[tokio::test]
    async fn test_try_from_vec_and_as_slice() {
        // 1. Create a batch and serialize it manually
        let batch = create_primary_batch(&["A"], &["1"]);
        let bytes = record_batch_to_bytes(&batch).unwrap();
        let bytes_len = bytes.len();

        // 2. Test TryFrom<Vec<u8>>
        let arrow_ipc = ArrowIpc::try_from(bytes.clone()).expect("TryFrom failed");

        // 3. Test access
        assert_eq!(arrow_ipc.num_rows(), 1);

        // 4. Test as_slice()
        let slice = arrow_ipc.bytes();
        assert_eq!(slice.len(), bytes_len);
        assert_eq!(slice, bytes.as_slice());
    }

    #[tokio::test]
    async fn test_from_batch_logic() {
        // 1. Start with a RecordBatch
        let batch = create_primary_batch(&["B"], &["2"]);

        // 2. Use ArrowIpc::from_batch (should trigger serialization internally)
        let arrow_ipc = ArrowIpc::from_batch(batch).expect("from_batch failed");

        // 4. Ensure we can get the bytes out
        assert!(!arrow_ipc.bytes().is_empty());
    }

    #[tokio::test]
    async fn test_csv_loading_populates_bytes() {
        let csv_data = "input,target\ncsv,test";
        let bytes = csv_data.as_bytes().to_vec();

        let arrow_ipc = parse_data_to_batch(bytes, "test.csv")
            .await
            .expect("Parsing failed")
            .pop()
            .unwrap();

        // Even though we loaded CSV, we requested the struct enforce IPC backing
        // So as_slice should return valid Arrow IPC bytes, not the CSV string.
        let raw_bytes = arrow_ipc.bytes();

        // Simple check: Arrow IPC starts with "ARROW1" or contains "ARROW1" depending on alignment,
        // but it definitely shouldn't be the CSV string anymore.
        assert_ne!(raw_bytes, csv_data.as_bytes());
        assert!(raw_bytes.len() > 10); // Header + Footer + Data
    }

    // -------------------------------------------------------------------------
    // Test Cases
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_parse_valid_csv() {
        let csv_data = "input,target\nhello,world\nfoo,bar";
        let bytes = csv_data.as_bytes().to_vec();

        let result = parse_data_to_batch(bytes, "data.csv").await;
        assert!(result.is_ok());

        let arrow_ipc = result.unwrap().pop().unwrap();

        // Check Deref works
        assert_eq!(arrow_ipc.num_rows(), 2);
        assert_eq!(arrow_ipc.schema().field(0).name(), "input");

        // Verify Content
        let col = arrow_ipc
            .column(0)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert_eq!(col.value(0), "hello");
    }

    #[tokio::test]
    async fn test_parse_valid_json() {
        // Arrow JSON reader expects line-delimited JSON objects
        let json_data = r#"{"input": "alpha", "target": "beta"}
                           {"input": "gamma", "target": "delta"}"#;
        let bytes = json_data.as_bytes().to_vec();

        let result = parse_data_to_batch(bytes, "data.json").await;
        assert!(result.is_ok());

        let arrow_ipc = result.unwrap().pop().unwrap();
        assert_eq!(arrow_ipc.num_rows(), 2);
    }

    #[tokio::test]
    async fn test_parse_ipc_happy_path() {
        // 1. Create a valid Arrow IPC byte stream
        let original_batch = create_primary_batch(&["A", "B"], &["1", "2"]);
        let bytes = record_batch_to_bytes(&original_batch).unwrap();

        // 2. Parse it
        let result = parse_data_to_batch(bytes, "file.arrow").await;
        assert!(result.is_ok());

        let arrow_ipc = result.unwrap().pop().unwrap();

        // 3. Verify data integrity
        assert_eq!(arrow_ipc.num_rows(), 2);
    }

    #[tokio::test]
    async fn test_memmap_persistence() {
        // 1. Start with in-memory CSV
        let csv_data = "input,target\nmemory,disk";
        let bytes = csv_data.as_bytes().to_vec();
        let mut arrow_ipc = parse_data_to_batch(bytes, "data.csv")
            .await
            .unwrap()
            .pop()
            .unwrap();

        // 2. Memmap it to a temp file
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_owned();

        // Windows often locks files if kept open, so we close the handle but keep path
        temp_file.close().unwrap();

        arrow_ipc.memmap(&temp_path).unwrap();

        // 3. Check if file was written
        assert!(std::fs::metadata(&temp_path).unwrap().len() > 0);

        // 4. Verify we can still read the data (via the new mmap pointer)
        assert_eq!(arrow_ipc.num_rows(), 1);
        let col = arrow_ipc
            .column(0)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert_eq!(col.value(0), "memory");
    }

    #[tokio::test]
    async fn test_error_unsupported_extension() {
        let bytes = vec![1, 2, 3];
        let result = parse_data_to_batch(bytes, "image.png").await;

        match result {
            Err(DataError::UnsupportedType(ext)) => assert_eq!(ext, "png"),
            _ => panic!("Expected UnsupportedType error"),
        }
    }

    #[tokio::test]
    async fn test_error_garbage_ipc_content() {
        let bytes = vec![0u8; 100]; // Just null bytes, invalid footer
        let result = parse_data_to_batch(bytes, "data.arrow").await;

        // Even though we "pay at load", we changed the logic to parse immediately
        // for reliability. So this should fail immediately.
        assert!(result.is_err());
        match result.unwrap_err() {
            DataError::InvalidContent(_) | DataError::Arrow(_) => {}
            e => panic!("Wrong error type: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_error_multi_batch_file() {
        // Construct a file with 2 batches
        let batch1 = create_primary_batch(&["A"], &["1"]);
        let batch2 = create_primary_batch(&["B"], &["2"]);

        let mut file_buffer = Vec::new();
        {
            let mut writer = FileWriter::try_new(&mut file_buffer, &batch1.schema()).unwrap();
            writer.write(&batch1).unwrap();
            writer.write(&batch2).unwrap(); // Write second batch
            writer.finish().unwrap();
        }

        let result = parse_data_to_batch(file_buffer, "multi.arrow").await;

        match result {
            Err(DataError::UnexpectedBatchCount(n)) => assert_eq!(n, 2),
            Ok(_) => panic!("Should have failed on multi-batch"),
            Err(e) => panic!("Wrong error type: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_empty_json() {
        let bytes = b"".to_vec();
        let result = parse_data_to_batch(bytes, "data.json").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_write_parquet_multi_batch() {
        let batch1 = create_primary_batch(&["A"], &["1"]);
        let batch2 = create_primary_batch(&["B"], &["2"]);
        let batches = vec![batch1, batch2];

        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_owned();
        temp_file.close().unwrap();

        write_parquet(&batches, &path).unwrap();

        assert!(path.exists());
        let metadata = std::fs::metadata(&path).unwrap();
        assert!(metadata.len() > 0);
    }

    #[tokio::test]
    async fn test_write_csv_multi_batch_chunked() {
        // Batch 1: A, B
        // Batch 2: C, D
        let batch1 = create_primary_batch(&["A", "B"], &["1", "2"]);
        let batch2 = create_primary_batch(&["C", "D"], &["3", "4"]);
        let batches = vec![batch1, batch2];

        let temp_file = NamedTempFile::new().unwrap();
        let base_path = temp_file.path().to_owned();
        temp_file.close().unwrap();

        // Chunk size 3:
        // File 0 should contain A, B, C (split logic handles moving C here)
        // File 1 should contain D
        let paths = write_csv(&batches, &base_path, Some(3)).unwrap();

        assert_eq!(paths.len(), 2);

        // Verify File 0
        let c0 = read_to_string(&paths[0]).unwrap();
        let lines0: Vec<&str> = c0.lines().collect();
        // Header + A + B + C = 4 lines
        assert_eq!(lines0.len(), 4);
        assert!(c0.contains("input,target"));
        assert!(c0.contains("A,1"));
        assert!(c0.contains("B,2"));
        assert!(c0.contains("C,3"));

        // Verify File 1
        let c1 = read_to_string(&paths[1]).unwrap();
        let lines1: Vec<&str> = c1.lines().collect();
        // Header + D = 2 lines
        assert_eq!(lines1.len(), 2);
        assert!(c1.contains("D,4"));
    }

    #[tokio::test]
    async fn test_write_jsonl_multi_batch_continuous() {
        let batch1 = create_primary_batch(&["A"], &["1"]);
        let batch2 = create_primary_batch(&["B"], &["2"]);
        let batches = vec![batch1, batch2];

        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_owned();
        temp_file.close().unwrap();

        // No chunking -> Single file
        let paths = write_jsonl(&batches, &path, None).unwrap();
        assert_eq!(paths.len(), 1);

        let content = read_to_string(&paths[0]).unwrap();
        assert!(content.contains(r#""input":"A""#));
        assert!(content.contains(r#""input":"B""#));
    }
}
