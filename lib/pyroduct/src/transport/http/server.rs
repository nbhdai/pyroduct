use std::sync::Arc;
use tokio::sync::Mutex;
use axum::{
    Router,
    routing::post,
    extract::State,
    Json,
    http::StatusCode,
    response::IntoResponse,
};
use pyro_artifacts::cache::LoadedPlaybook;
use crate::{CapturedError, PyroError, PyroRow, PyroValue};
use crate::format::log_wal::LogWal;
use crate::module::PyroFactory;
use crate::pipeline::{ExecutionRecord, Pipeline};
use crate::format::value::PyroSchema;

/// An HTTP server that runs loaded playbooks using Axum.
pub struct PlaybookHttpServer {
    pipeline: Arc<Mutex<Pipeline>>,
    input_schema: PyroSchema<'static>,
}

impl PlaybookHttpServer {
    /// Create a new HTTP server from a loaded playbook.
    pub async fn new(playbook: &LoadedPlaybook) -> Result<Self, crate::pipeline::PipelineError> {
        let factory = PyroFactory::from_playbook(playbook)?;
        let instance = factory.instantiate().await?;
        let input_schema = factory.spec().func.input.clone().into_owned();
        let output_schema = factory.spec().func.output.clone();

        let pipeline = Pipeline {
            step: instance,
            success_log_retention_secs: 3600,
            error_log_retention_secs: 86400 * 7,
            log_manager: LogWal::open(playbook.log_dir.clone(), 1000)
                .await
                .map_err(|io| {
                    PyroError::local_io(
                        CapturedError::new("Unable to make the log wal").with_source(io),
                    )
                })?,
            input_manager: crate::pipeline::data::DataManager::new(
                playbook.input_dir.clone(),
                input_schema.clone(),
            ),
            output_manager: crate::pipeline::data::DataManager::new(
                playbook.output_dir.clone(),
                output_schema,
            ),
        };
        Ok(Self {
            pipeline: Arc::new(Mutex::new(pipeline)),
            input_schema,
        })
    }

    /// Return the Axum `Router` configured for the playbook.
    pub fn router(self) -> Router {
        let shared_state = Arc::new(self);
        Router::new()
            .route("/", post(handle_playbook_query))
            .with_state(shared_state)
    }

    /// Run the server, accepting connections on the provided TCP listener.
    pub async fn run(self, listener: tokio::net::TcpListener) -> Result<(), axum::BoxError> {
        let app = self.router();
        axum::serve(listener, app).await?;
        Ok(())
    }
}

async fn handle_playbook_query(
    State(server): State<Arc<PlaybookHttpServer>>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    // 1. Convert JSON to PyroRow
    let input_row = match json_to_pyro_row(payload) {
        Ok(row) => row,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("Invalid JSON query: {}", e) })),
            )
                .into_response();
        }
    };

    // 2. Repair input PyroRow based on the input schema from the playbook spec!
    let repaired_row = match input_row.project_repair(server.input_schema.fields()) {
        Ok(row) => row,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("Failed to repair input JSON according to module spec: {:?}", e)
                })),
            )
                .into_response();
        }
    };

    // 3. Process the repaired row in the pipeline
    let mut pipeline = server.pipeline.lock().await;
    match pipeline.process(0, &repaired_row).await {
        Ok(ExecutionRecord::Success { success, .. }) => {
            match serde_json::to_value(&success) {
                Ok(val) => (StatusCode::OK, Json(val)).into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": format!("Serialization error: {}", e) })),
                )
                    .into_response(),
            }
        }
        Ok(ExecutionRecord::Failure { failure, .. }) => {
            let err_msg = match failure {
                Ok(captured) => format!("{:?}", captured),
                Err(s) => s,
            };
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": err_msg })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("{:?}", e) })),
        )
            .into_response(),
    }
}

fn json_to_pyro_value(value: serde_json::Value) -> PyroValue<'static> {
    match value {
        serde_json::Value::Null => PyroValue::Null,
        serde_json::Value::Bool(b) => PyroValue::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                PyroValue::I64(i)
            } else if let Some(u) = n.as_u64() {
                PyroValue::U64(u)
            } else if let Some(f) = n.as_f64() {
                PyroValue::F64(f)
            } else {
                PyroValue::Null
            }
        }
        serde_json::Value::String(s) => PyroValue::Str(std::borrow::Cow::Owned(s)),
        serde_json::Value::Array(arr) => {
            let items: Vec<PyroValue<'static>> = arr.into_iter().map(json_to_pyro_value).collect();
            PyroValue::List(items)
        }
        serde_json::Value::Object(obj) => {
            let mut items = Vec::with_capacity(obj.len());
            for (key, val) in obj {
                items.push(crate::format::value::RowItem {
                    key: std::borrow::Cow::Owned(key),
                    value: json_to_pyro_value(val),
                });
            }
            PyroValue::Group(PyroRow(items))
        }
    }
}

fn json_to_pyro_row(value: serde_json::Value) -> Result<PyroRow<'static>, String> {
    match value {
        serde_json::Value::Object(obj) => {
            let mut items = Vec::with_capacity(obj.len());
            for (key, val) in obj {
                items.push(crate::format::value::RowItem {
                    key: std::borrow::Cow::Owned(key),
                    value: json_to_pyro_value(val),
                });
            }
            Ok(PyroRow(items))
        }
        _ => Err("Input JSON must be an object representing a Row".to_string()),
    }
}
