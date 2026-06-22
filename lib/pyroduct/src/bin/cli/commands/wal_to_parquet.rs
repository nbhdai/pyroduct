use anyhow::{Context, Result, anyhow};
use std::path::Path;
use pyroduct::format::value::arrow::wal::recover;
use pyroduct::format::value::arrow::PreBatch;
use pyroduct::format::value::PyroRow;
use pyro_file::write_parquet;

pub async fn wal_to_parquet(wal_path: &Path, output_path: &Path) -> Result<()> {
    tracing::info!("Converting WAL {:?} to Parquet {:?}", wal_path, output_path);

    // If the user specified the WAL file directly with the `.pyrowal` extension,
    // strip it because the recover function expects the base path.
    let base_path = if wal_path.extension().map_or(false, |ext| ext == "pyrowal") {
        wal_path.with_extension("")
    } else {
        wal_path.to_path_buf()
    };

    let actual_wal_file = base_path.with_extension("pyrowal");
    if !actual_wal_file.exists() {
        return Err(anyhow!("WAL file not found: {:?}", actual_wal_file));
    }

    // Run the recover operation in a blocking task since it maps and reads files synchronously.
    let base_path_clone = base_path.clone();
    let rows = tokio::task::spawn_blocking(move || {
        recover(&base_path_clone)
    })
    .await
    .context("Failed to join WAL recovery thread")?
    .context("Failed to recover rows from WAL file")?;

    if rows.is_empty() {
        return Err(anyhow!("No rows found in WAL file: {:?}", actual_wal_file));
    }

    // Infer schema from recovered rows.
    let schema = PyroRow::infer_schema(&rows)
        .context("Failed to infer schema from WAL rows")?;

    // Load rows into PreBatch.
    let mut prebatch = PreBatch::new(schema);
    for row in rows {
        prebatch.push_unchecked(row);
    }

    let record_batch = prebatch
        .flush()
        .context("Failed to flush rows to record batch")?
        .ok_or_else(|| anyhow!("Failed to flush RecordBatch: returned empty"))?;

    // Write RecordBatch to Parquet file.
    let output_path_clone = output_path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        write_parquet(&[record_batch], &output_path_clone)
    })
    .await
    .context("Failed to join Parquet writer thread")?
    .context("Failed to write Parquet file")?;

    tracing::info!("Successfully converted WAL to Parquet file at {:?}", output_path);
    println!("Successfully converted WAL to Parquet file at {:?}", output_path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use pyroduct::format::wal::WalWriter;
    use pyroduct::format::Bridgeable;
    use pyroduct::format::header::PyroData;
    use pyroduct::format::value::PyroValue;
    use arrow::array::Int32Array;

    #[tokio::test]
    async fn test_wal_to_parquet_conversion() {
        let dir = tempdir().unwrap();
        let base_path = dir.path().join("session_val_0");
        let parquet_path = dir.path().join("output.parquet");

        // 1. Create a WAL with some rows
        {
            let mut writer = WalWriter::open(&base_path).unwrap();
            for i in 0..5 {
                let row = PyroRow::from([
                    ("id", PyroValue::I32(i as i32)),
                    ("name", PyroValue::from(format!("name-{}", i))),
                ]);
                let vec = row.ship().unwrap();
                writer.append(i, vec.py_ref()).await.unwrap();
            }
        }

        // 2. Call the conversion function
        let wal_file = base_path.with_extension("pyrowal");
        wal_to_parquet(&wal_file, &parquet_path).await.expect("Conversion failed");

        // 3. Verify parquet file exists
        assert!(parquet_path.exists());

        // 4. Read the parquet file back and check data
        let parquet_bytes = std::fs::read(&parquet_path).unwrap();
        let batches = pyro_file::parse_data_to_batch_sync(parquet_bytes, "output.parquet")
            .expect("Failed to parse output parquet");
        assert_eq!(batches.len(), 1);
        let batch = batches[0].clone().to_batch();

        assert_eq!(batch.num_rows(), 5);
        let id_col = batch
            .column_by_name("id")
            .unwrap()
            .as_any()
            .downcast_ref::<Int32Array>()
            .unwrap();
        assert_eq!(id_col.value(0), 0);
        assert_eq!(id_col.value(4), 4);
    }
}

