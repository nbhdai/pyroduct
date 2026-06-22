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
    pub metadata_prefix: Option<String>,
}

impl std::fmt::Debug for DataManagerTableProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataManagerTableProvider")
            .field("schema", &self.schema())
            .field("metadata_prefix", &self.metadata_prefix)
            .finish()
    }
}

impl DataManagerTableProvider {
    pub fn new(
        mem_table: MemTable,
        guards: Vec<crate::pipeline::data::IpcFileGuard>,
        metadata_prefix: Option<String>,
    ) -> Self {
        Self {
            mem_table: Arc::new(mem_table),
            _guards: guards,
            metadata_prefix,
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

pub struct DataManagerSql {
    providers: Vec<Arc<DataManagerTableProvider>>,
}

impl DataManagerSql {
    pub fn new(providers: Vec<Arc<DataManagerTableProvider>>) -> Self {
        assert!(
            providers.len() <= 3,
            "DataManagerSql supports up to 3 providers"
        );
        Self { providers }
    }

    fn validate_sql_query(sql: &str) -> Result<(), crate::PyroError> {
        use crate::captured::CapturedError;
        use sqlparser::ast::{SetExpr, Statement, TableFactor};
        use sqlparser::dialect::GenericDialect;
        use sqlparser::parser::Parser;

        let dialect = GenericDialect {};
        let mut statements = Parser::parse_sql(&dialect, sql).map_err(|e| {
            crate::PyroError::validation(CapturedError::new(format!("SQL parse error: {}", e)))
        })?;

        if statements.len() != 1 {
            return Err(crate::PyroError::validation(CapturedError::new(
                "SQL statement must contain exactly one query".to_string(),
            )));
        }

        let stmt = statements.pop().unwrap();
        let query = match stmt {
            Statement::Query(q) => q,
            _ => {
                return Err(crate::PyroError::validation(CapturedError::new(
                    "Only SELECT queries are allowed".to_string(),
                )));
            }
        };

        let select = match *query.body {
            SetExpr::Select(s) => s,
            _ => {
                return Err(crate::PyroError::validation(CapturedError::new(
                    "Only SELECT queries are allowed".to_string(),
                )));
            }
        };

        if select.from.len() != 1 {
            return Err(crate::PyroError::validation(CapturedError::new(
                "Query must select from exactly one table: 'pipeline'".to_string(),
            )));
        }

        let table_with_joins = &select.from[0];

        if !table_with_joins.joins.is_empty() {
            return Err(crate::PyroError::validation(CapturedError::new(
                "Query cannot contain JOIN clauses".to_string(),
            )));
        }

        match &table_with_joins.relation {
            TableFactor::Table { name, .. } => {
                let table_name = name.to_string().to_lowercase();
                if table_name != "pipeline" {
                    return Err(crate::PyroError::validation(CapturedError::new(format!(
                        "Query must select from 'pipeline', but selected from '{}'",
                        table_name
                    ))));
                }
            }
            _ => {
                return Err(crate::PyroError::validation(CapturedError::new(
                    "Query must select from a table".to_string(),
                )));
            }
        }

        Ok(())
    }

    pub async fn execute(
        &self,
        sql: &str,
    ) -> Result<Vec<arrow::array::RecordBatch>, crate::PyroError> {
        use crate::captured::CapturedError;
        use datafusion::prelude::SessionContext;

        // Perform strict SQL validation first
        Self::validate_sql_query(sql)?;

        let ctx = SessionContext::new();

        if self.providers.is_empty() {
            let df = ctx.sql(sql).await.map_err(|e| {
                crate::PyroError::validation(CapturedError::new(format!("SQL query failed: {}", e)))
            })?;
            return df.collect().await.map_err(|e| {
                crate::PyroError::validation(CapturedError::new(format!(
                    "SQL collection failed: {}",
                    e
                )))
            });
        }

        // Register tables and extract prefixes
        let mut prefixes = Vec::new();
        for (idx, provider) in self.providers.iter().enumerate() {
            let table_name = format!("__t{}", idx);
            ctx.register_table(&table_name, provider.clone())
                .map_err(|e| {
                    crate::PyroError::validation(CapturedError::new(format!(
                        "Failed to register table {}: {}",
                        table_name, e
                    )))
                })?;
            prefixes.push(provider.metadata_prefix.clone());
        }

        // Build the CREATE VIEW pipeline AS SELECT ... statement
        let mut view_sql = String::from("CREATE VIEW pipeline AS SELECT ");
        let mut columns = Vec::new();
        let mut seen_names = std::collections::HashSet::new();

        for (idx, provider) in self.providers.iter().enumerate() {
            let table_name = format!("__t{}", idx);
            for field in provider.schema().fields() {
                let col_name = field.name();
                if seen_names.contains(col_name) {
                    let is_meta = prefixes[idx].as_ref() == Some(col_name);
                    if !is_meta {
                        let alias = format!("{}_{}", table_name, col_name);
                        columns.push(format!("{}.{} AS {}", table_name, col_name, alias));
                    }
                } else {
                    seen_names.insert(col_name.clone());
                    columns.push(format!("{}.{}", table_name, col_name));
                }
            }
        }

        view_sql.push_str(&columns.join(", "));
        view_sql.push_str(" FROM __t0");

        for idx in 1..self.providers.len() {
            let table_name = format!("__t{}", idx);
            let cond = match (&prefixes[0], &prefixes[idx]) {
                (Some(p0), Some(p_idx)) => {
                    format!("{}.{}.index = {}.{}.index", "__t0", p0, table_name, p_idx)
                }
                _ => {
                    return Err(crate::PyroError::validation(CapturedError::new(format!(
                        "Cannot join tables: missing metadata prefix on table 0 or table {}",
                        idx
                    ))));
                }
            };
            view_sql.push_str(&format!(" JOIN {} ON {}", table_name, cond));
        }

        ctx.sql(&view_sql).await.map_err(|e| {
            crate::PyroError::validation(CapturedError::new(format!(
                "Failed to create joined view: {}",
                e
            )))
        })?;

        // Execute user SQL against the view
        let df = ctx.sql(sql).await.map_err(|e| {
            crate::PyroError::validation(CapturedError::new(format!("SQL query failed: {}", e)))
        })?;

        df.collect().await.map_err(|e| {
            crate::PyroError::validation(CapturedError::new(format!("SQL collect failed: {}", e)))
        })
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
        let manager = DataManager::new(dir.path(), schema, 1000);
        manager.set_capacities(2, 2).await;

        // Push 1 record into the WAL (active in-memory)
        manager
            .push_record(0, &make_success_record(0, 42, "alice"))
            .await
            .unwrap();

        // Get the provider
        let provider = manager.sql_provider().await.unwrap();

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
        let manager = DataManager::new(dir.path(), schema, 1000);
        manager.set_capacities(2, 2).await;

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
        let ipc_path = {
            let s = manager.state.lock().await;
            assert_eq!(s.ipc_file_paths.len(), 1);
            let p = s.ipc_file_paths[0].clone();
            assert!(p.exists());
            p
        };

        // 2. Call sql_provider() to lock the IPC file with a guard
        let provider = manager.sql_provider().await.unwrap();

        // Verify the file is registered in active_readers
        {
            let state = manager.shared_state.lock().unwrap();
            assert_eq!(state.active_readers.get(&ipc_path), Some(&1));
            assert!(state.pending_deletions.is_empty());
        }

        // 3. Roll out to parquet. Since the provider holds the guard, the file should NOT be deleted!
        manager.rollout_to_parquet().await.unwrap();

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

    #[tokio::test]
    async fn test_sql_provider_metadata() {
        let dir = TempDir::new().unwrap();
        let schema = setup_schema();
        let manager = DataManager::new(dir.path(), schema, 1000);
        let provider = manager.sql_provider().await.unwrap();

        // 1. Verify schema()
        let provider_schema = provider.schema();
        assert_eq!(provider_schema.fields().len(), 2);
        assert_eq!(provider_schema.field(0).name(), "id");
        assert_eq!(provider_schema.field(1).name(), "name");

        // 2. Verify table_type()
        assert_eq!(provider.table_type(), TableType::Base);

        // 3. Verify as_any() downcasting
        let provider_any: &dyn Any = provider.as_any();
        let downcast = provider_any.downcast_ref::<DataManagerTableProvider>();
        assert!(downcast.is_some());

        // 4. Verify fmt::Debug
        let debug_str = format!("{:?}", provider);
        assert!(debug_str.contains("DataManagerTableProvider"));
    }

    #[tokio::test]
    async fn test_sql_provider_scan_all_sources() {
        use arrow::array::{Int32Array, StringArray};

        let dir = TempDir::new().unwrap();
        let schema = setup_schema();
        let manager = DataManager::new(dir.path(), schema, 1000);
        manager.set_capacities(2, 2).await;

        // 1. Push 4 records -> triggers two WAL flushes and one Parquet rollout (records 0..=3)
        manager
            .push_record(0, &make_success_record(0, 10, "ten"))
            .await
            .unwrap();
        manager
            .push_record(1, &make_success_record(1, 11, "eleven"))
            .await
            .unwrap();
        // first flush happens here (created batch_0.arrow)

        manager
            .push_record(2, &make_success_record(2, 12, "twelve"))
            .await
            .unwrap();
        manager
            .push_record(3, &make_success_record(3, 13, "thirteen"))
            .await
            .unwrap();
        // second flush happens here (created batch_2.arrow)
        // because ipc_capacity = 2, rollout_to_parquet() is triggered automatically.
        // records 10, 11, 12, 13 are now in parquet.

        // 2. Push 2 records -> triggers one WAL flush (records 4..=5)
        manager
            .push_record(4, &make_success_record(4, 14, "fourteen"))
            .await
            .unwrap();
        manager
            .push_record(5, &make_success_record(5, 15, "fifteen"))
            .await
            .unwrap();
        // third flush happens here (created batch_4.arrow). It stays as IPC because ipc_files.len() = 1 < 2.

        // 3. Push 1 record -> remains in active in-memory WAL buffer (record 6)
        manager
            .push_record(6, &make_success_record(6, 16, "sixteen"))
            .await
            .unwrap();

        // Verify storage status
        {
            let s = manager.state.lock().await;
            assert_eq!(s.parquet_file_paths.len(), 1);
            assert_eq!(s.ipc_file_paths.len(), 1);
        }
        assert!(manager.get_active_batch().await.unwrap().is_some());

        // 4. Retrieve SQL provider and register in DataFusion
        let provider = manager.sql_provider().await.unwrap();
        let ctx = SessionContext::new();
        ctx.register_table("data", Arc::new(provider)).unwrap();

        // 5. Query and order by id
        let df = ctx
            .sql("SELECT id, name FROM data ORDER BY id")
            .await
            .unwrap();
        let results = df.collect().await.unwrap();

        // 6. Verify total rows and contents across all batches
        let mut total_rows = 0;
        let mut ids = Vec::new();
        let mut names = Vec::new();

        for batch in results {
            total_rows += batch.num_rows();
            let col_id = batch
                .column(0)
                .as_any()
                .downcast_ref::<Int32Array>()
                .unwrap();
            let col_name = batch
                .column(1)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();

            for i in 0..batch.num_rows() {
                ids.push(col_id.value(i));
                names.push(col_name.value(i).to_string());
            }
        }

        assert_eq!(total_rows, 7);
        assert_eq!(ids, vec![10, 11, 12, 13, 14, 15, 16]);
        assert_eq!(
            names,
            vec![
                "ten", "eleven", "twelve", "thirteen", "fourteen", "fifteen", "sixteen"
            ]
        );
    }

    #[tokio::test]
    async fn test_sql_provider_empty() {
        let dir = TempDir::new().unwrap();
        let schema = setup_schema();
        let manager = DataManager::new(dir.path(), schema, 1000);

        let provider = manager.sql_provider().await.unwrap();
        let ctx = SessionContext::new();
        ctx.register_table("data", Arc::new(provider)).unwrap();

        let df = ctx.sql("SELECT id, name FROM data").await.unwrap();
        let results = df.collect().await.unwrap();

        let total_rows: usize = results.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total_rows, 0);
    }

    #[tokio::test]
    async fn test_sql_provider_join_on_metadata() {
        use arrow::array::{Int32Array, StringArray};

        let dir_input = TempDir::new().unwrap();
        let dir_output = TempDir::new().unwrap();

        let input_schema = PyroSchema::new(vec![
            PyroField::new(
                "id",
                PyroType::PrimitiveScalar(PrimitiveDataType::I32),
                false,
            ),
            PyroField::new("value", PyroType::Str, true),
        ]);
        let output_schema = PyroSchema::new(vec![PyroField::new("result", PyroType::Str, true)]);

        let input_manager = DataManager::new(dir_input.path(), input_schema, 1000);
        input_manager.set_metadata_prefix("_input_meta").await;

        let output_manager = DataManager::new(dir_output.path(), output_schema, 1000);
        output_manager.set_metadata_prefix("_output_meta").await;

        // Push records to input_manager
        let input_row_0 = PyroRow::from([
            ("id", PyroValue::from(100i32)),
            ("value", PyroValue::from("hello")),
        ]);
        let input_row_1 = PyroRow::from([
            ("id", PyroValue::from(101i32)),
            ("value", PyroValue::from("world")),
        ]);
        input_manager.push_record(0, &input_row_0).await.unwrap();
        input_manager.push_record(1, &input_row_1).await.unwrap();

        // Push matching records to output_manager
        let output_row_0 = PyroRow::from([("result", PyroValue::from("HELLO"))]);
        let output_row_1 = PyroRow::from([("result", PyroValue::from("WORLD"))]);
        output_manager.push_record(0, &output_row_0).await.unwrap();
        output_manager.push_record(1, &output_row_1).await.unwrap();

        // Retrieve SQL providers
        let input_provider = input_manager.sql_provider().await.unwrap();
        let output_provider = output_manager.sql_provider().await.unwrap();

        let ctx = SessionContext::new();
        ctx.register_table("inputs", Arc::new(input_provider))
            .unwrap();
        ctx.register_table("outputs", Arc::new(output_provider))
            .unwrap();

        // Join using the nested metadata index field
        let df = ctx
            .sql(
                "SELECT i.id, i.value, o.result \
                 FROM inputs i \
                 JOIN outputs o ON i._input_meta.index = o._output_meta.index \
                 ORDER BY i.id",
            )
            .await
            .unwrap();
        let results = df.collect().await.unwrap();

        let mut total_rows = 0;
        let mut ids = Vec::new();
        let mut values = Vec::new();
        let mut results_col = Vec::new();

        for batch in results {
            total_rows += batch.num_rows();
            let col_id = batch
                .column(0)
                .as_any()
                .downcast_ref::<Int32Array>()
                .unwrap();
            let col_value = batch
                .column(1)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let col_result = batch
                .column(2)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();

            for i in 0..batch.num_rows() {
                ids.push(col_id.value(i));
                values.push(col_value.value(i).to_string());
                results_col.push(col_result.value(i).to_string());
            }
        }

        assert_eq!(total_rows, 2);
        assert_eq!(ids, vec![100, 101]);
        assert_eq!(values, vec!["hello", "world"]);
        assert_eq!(results_col, vec!["HELLO", "WORLD"]);
    }

    #[tokio::test]
    async fn test_data_manager_sql_virtual_join() {
        use arrow::array::{Int32Array, StringArray};

        let dir_input = TempDir::new().unwrap();
        let dir_output = TempDir::new().unwrap();

        let input_schema = PyroSchema::new(vec![
            PyroField::new(
                "id",
                PyroType::PrimitiveScalar(PrimitiveDataType::I32),
                false,
            ),
            PyroField::new("value", PyroType::Str, true),
        ]);
        let output_schema = PyroSchema::new(vec![PyroField::new("result", PyroType::Str, true)]);

        let input_manager = DataManager::new(dir_input.path(), input_schema, 1000);
        input_manager.set_metadata_prefix("_input_meta").await;

        let output_manager = DataManager::new(dir_output.path(), output_schema, 1000);
        output_manager.set_metadata_prefix("_output_meta").await;

        // Push records
        let input_row_0 = PyroRow::from([
            ("id", PyroValue::from(100i32)),
            ("value", PyroValue::from("hello")),
        ]);
        let input_row_1 = PyroRow::from([
            ("id", PyroValue::from(101i32)),
            ("value", PyroValue::from("world")),
        ]);
        input_manager.push_record(0, &input_row_0).await.unwrap();
        input_manager.push_record(1, &input_row_1).await.unwrap();

        let output_row_0 = PyroRow::from([("result", PyroValue::from("HELLO"))]);
        let output_row_1 = PyroRow::from([("result", PyroValue::from("WORLD"))]);
        output_manager.push_record(0, &output_row_0).await.unwrap();
        output_manager.push_record(1, &output_row_1).await.unwrap();

        // Construct DataManagerSql
        let input_provider = Arc::new(input_manager.sql_provider().await.unwrap());
        let output_provider = Arc::new(output_manager.sql_provider().await.unwrap());

        let dm_sql = DataManagerSql::new(vec![input_provider, output_provider]);

        // Execute query on the virtual joined "pipeline" table, NO join in SQL query!
        let results = dm_sql
            .execute("SELECT id, value, result FROM pipeline ORDER BY id")
            .await
            .unwrap();

        let mut total_rows = 0;
        let mut ids = Vec::new();
        let mut values = Vec::new();
        let mut results_col = Vec::new();

        for batch in results {
            total_rows += batch.num_rows();
            let col_id = batch
                .column(0)
                .as_any()
                .downcast_ref::<Int32Array>()
                .unwrap();
            let col_value = batch
                .column(1)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let col_result = batch
                .column(2)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();

            for i in 0..batch.num_rows() {
                ids.push(col_id.value(i));
                values.push(col_value.value(i).to_string());
                results_col.push(col_result.value(i).to_string());
            }
        }

        assert_eq!(total_rows, 2);
        assert_eq!(ids, vec![100, 101]);
        assert_eq!(values, vec!["hello", "world"]);
        assert_eq!(results_col, vec!["HELLO", "WORLD"]);
    }

    #[tokio::test]
    async fn test_data_manager_sql_validation() {
        let dir_input = TempDir::new().unwrap();
        let input_schema = setup_schema();
        let input_manager = DataManager::new(dir_input.path(), input_schema, 1000);
        input_manager.set_metadata_prefix("_input_meta").await;

        let input_provider = Arc::new(input_manager.sql_provider().await.unwrap());
        let dm_sql = DataManagerSql::new(vec![input_provider]);

        // 1. Valid simple query (case-insensitive table relation)
        assert!(
            dm_sql
                .execute("SELECT id, name FROM PIPELINE WHERE id > 10")
                .await
                .is_ok()
        );
        assert!(
            dm_sql
                .execute("SELECT id, name FROM pipeline")
                .await
                .is_ok()
        );

        // 2. Rejecting multiple statements
        let res_mult = dm_sql
            .execute("SELECT id FROM pipeline; SELECT name FROM pipeline;")
            .await;
        assert!(res_mult.is_err());

        // 3. Rejecting non-SELECT query
        let res_insert = dm_sql
            .execute("INSERT INTO pipeline (id, name) VALUES (1, 'val')")
            .await;
        assert!(res_insert.is_err());

        // 4. Rejecting selecting from wrong table
        let res_wrong_table = dm_sql.execute("SELECT id FROM different_table").await;
        assert!(res_wrong_table.is_err());

        // 5. Rejecting select with joins
        let res_join = dm_sql
            .execute("SELECT p1.id FROM pipeline p1 JOIN pipeline p2 ON p1.id = p2.id")
            .await;
        assert!(res_join.is_err());
    }
}
