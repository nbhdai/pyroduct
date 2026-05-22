#![cfg(feature = "host")]

use async_trait::async_trait;
use std::any::Any;
use std::sync::Arc;

use arrow::datatypes::SchemaRef;
use datafusion::catalog::Session;
use datafusion::datasource::memory::MemTable;
use datafusion::datasource::{TableProvider, TableType};
use datafusion::error::Result as DFResult;
use datafusion::physical_plan::ExecutionPlan;
use datafusion::prelude::Expr;

pub struct DataManagerTableProvider {
    mem_table: Arc<MemTable>,
    _guards: Vec<crate::pipeline::data::IpcFileGuard>,
}

impl std::fmt::Debug for DataManagerTableProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataManagerTableProvider")
            .field("schema", &self.schema())
            .finish()
    }
}

impl DataManagerTableProvider {
    pub fn new(mem_table: MemTable, guards: Vec<crate::pipeline::data::IpcFileGuard>) -> Self {
        Self {
            mem_table: Arc::new(mem_table),
            _guards: guards,
        }
    }
}

#[async_trait]
impl TableProvider for DataManagerTableProvider {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        self.mem_table.schema()
    }

    fn table_type(&self) -> TableType {
        TableType::Base
    }

    async fn scan(
        &self,
        state: &dyn Session,
        projection: Option<&Vec<usize>>,
        filters: &[Expr],
        limit: Option<usize>,
    ) -> DFResult<Arc<dyn ExecutionPlan>> {
        self.mem_table.scan(state, projection, filters, limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::value::{PrimitiveDataType, PyroField, PyroSchema, PyroType};
    use crate::pipeline::data::DataManager;
    use crate::{PyroRow, PyroValue};
    use datafusion::prelude::SessionContext;
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
    async fn test_sql_provider_scan() {
        let dir = TempDir::new().unwrap();
        let schema = setup_schema();
        let mut manager = DataManager::new(dir.path(), schema);
        manager.set_capacities(2, 2);

        // Push 1 record into the WAL (active in-memory)
        manager
            .push_record(0, &make_success_record(0, 42, "alice"))
            .await
            .unwrap();

        // Get the provider
        let provider = manager.sql_provider().unwrap();

        // Register in DataFusion
        let ctx = SessionContext::new();
        ctx.register_table("data", Arc::new(provider)).unwrap();

        // Execute SQL query
        let df = ctx.sql("SELECT id, name FROM data").await.unwrap();
        let results = df.collect().await.unwrap();

        // Verify rows
        assert_eq!(results.len(), 1);
        let batch = &results[0];
        assert_eq!(batch.num_rows(), 1);
    }

    #[tokio::test]
    async fn test_sql_provider_ipc_guard_deferred_deletion() {
        let dir = TempDir::new().unwrap();
        let schema = setup_schema();
        let mut manager = DataManager::new(dir.path(), schema);
        manager.set_capacities(2, 2);

        // 1. Push 2 records to trigger flush_wal (creates 1 IPC file)
        manager
            .push_record(0, &make_success_record(0, 42, "alice"))
            .await
            .unwrap();
        manager
            .push_record(1, &make_success_record(1, 43, "bob"))
            .await
            .unwrap();

        // Verify we have 1 IPC file path
        assert_eq!(manager.ipc_file_paths.len(), 1);
        let ipc_path = manager.ipc_file_paths[0].clone();
        assert!(ipc_path.exists());

        // 2. Call sql_provider() to lock the IPC file with a guard
        let provider = manager.sql_provider().unwrap();

        // Verify the file is registered in active_readers
        {
            let state = manager.shared_state.lock().unwrap();
            assert_eq!(state.active_readers.get(&ipc_path), Some(&1));
            assert!(state.pending_deletions.is_empty());
        }

        // 3. Roll out to parquet. Since the provider holds the guard, the file should NOT be deleted!
        manager.rollout_to_parquet().unwrap();

        // The file must still exist on disk
        assert!(ipc_path.exists());

        // The path must be in pending_deletions
        {
            let state = manager.shared_state.lock().unwrap();
            assert!(state.pending_deletions.contains(&ipc_path));
        }

        // 4. Drop the provider. This should release the guard and automatically trigger the deletion!
        drop(provider);

        // The file must now be deleted from disk
        assert!(!ipc_path.exists());

        // The shared state must be fully cleared
        {
            let state = manager.shared_state.lock().unwrap();
            assert!(state.active_readers.is_empty());
            assert!(state.pending_deletions.is_empty());
        }
    }
}
