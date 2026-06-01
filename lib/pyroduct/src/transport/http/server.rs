use crate::format::log_wal::LogWal;
use crate::format::value::PyroSchema;
use crate::module::PyroFactory;
use crate::module::SessionResult;
use crate::pipeline::{
    ExecutionRecord, Pipeline, session::SessionPipeline, session_diff::SessionDiffPipeline,
};
use crate::{CapturedError, PyroError, PyroRow, PyroValue};
use axum::response::Response;
use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::post};
use pyro_artifacts::artifacts::PlaybookSpec;
use pyro_artifacts::cache::LoadedPlaybook;
use std::sync::Arc;
use tokio::sync::Mutex;

enum ServerPipeline {
    Normal(Pipeline),
    Session(SessionPipeline),
    SessionDiff(SessionDiffPipeline),
}

/// An HTTP server that runs loaded playbooks using Axum.
pub struct PlaybookHttpServer {
    pipeline: Arc<Mutex<ServerPipeline>>,
    input_schema: PyroSchema<'static>,
    spec: Arc<PlaybookSpec>,
}

impl PlaybookHttpServer {
    /// Create a new HTTP server from a loaded playbook.
    pub async fn new(playbook: &LoadedPlaybook) -> Result<Self, crate::pipeline::PipelineError> {
        let factory = PyroFactory::from_playbook(playbook)?;
        let spec = Arc::new(factory.spec().clone());
        let instance = factory.instantiate().await?;
        let input_schema = factory.spec().func.input.clone().into_owned();
        let output_schema = factory.spec().func.output.clone();
        let kind = factory.spec().func.kind;

        let server_pipeline = match kind {
            pyro_spec::ModuleKind::Normal => {
                let pipeline = Pipeline {
                    step: instance,
                    success_log_retention_secs: 3600,
                    error_log_retention_secs: 86400 * 7,
                    log_manager: LogWal::open(playbook.log_dir.clone(), 1000).await.map_err(
                        |io| {
                            PyroError::local_io(
                                CapturedError::new("Unable to make the log wal").with_source(io),
                            )
                        },
                    )?,
                    input_manager: crate::pipeline::data::DataManager::new(
                        playbook.input_dir.clone(),
                        input_schema.clone(),
                    ),
                    output_manager: crate::pipeline::data::DataManager::new(
                        playbook.output_dir.clone(),
                        output_schema,
                    ),
                    callbacks: Vec::new(),
                };
                ServerPipeline::Normal(pipeline)
            }
            pyro_spec::ModuleKind::Session => {
                let pipeline = SessionPipeline {
                    step: instance,
                    success_log_retention_secs: 3600,
                    error_log_retention_secs: 86400 * 7,
                    log_manager: LogWal::open(playbook.log_dir.clone(), 1000).await.map_err(
                        |io| {
                            PyroError::local_io(
                                CapturedError::new("Unable to make the log wal").with_source(io),
                            )
                        },
                    )?,
                    output_manager: crate::pipeline::data::DataManager::new(
                        playbook.output_dir.clone(),
                        output_schema,
                    ),
                    log_dir: playbook.log_dir.clone(),
                    output_dir: playbook.output_dir.clone(),
                    wal_capacity: 1000,
                    active_sessions: std::collections::HashMap::new(),
                    callbacks: Vec::new(),
                };
                ServerPipeline::Session(pipeline)
            }
            pyro_spec::ModuleKind::SessionDiff => {
                let pipeline = SessionDiffPipeline {
                    step: instance,
                    success_log_retention_secs: 3600,
                    error_log_retention_secs: 86400 * 7,
                    log_manager: LogWal::open(playbook.log_dir.clone(), 1000).await.map_err(
                        |io| {
                            PyroError::local_io(
                                CapturedError::new("Unable to make the log wal").with_source(io),
                            )
                        },
                    )?,
                    output_manager: crate::pipeline::data::DataManager::new(
                        playbook.output_dir.clone(),
                        output_schema,
                    ),
                    log_dir: playbook.log_dir.clone(),
                    output_dir: playbook.output_dir.clone(),
                    wal_capacity: 1000,
                    active_sessions: std::collections::HashMap::new(),
                    callbacks: Vec::new(),
                };
                ServerPipeline::SessionDiff(pipeline)
            }
        };

        Ok(Self {
            pipeline: Arc::new(Mutex::new(server_pipeline)),
            input_schema,
            spec,
        })
    }

    /// Return the Axum `Router` configured for the playbook.
    pub fn router(self) -> Router {
        let shared_state = Arc::new(self);
        Router::new()
            .route("/", post(handle_playbook_query))
            .route("/{session_id}", post(handle_playbook_session_query))
            .route("/schema", axum::routing::get(handle_playbook_schema))
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
    match &mut *pipeline {
        ServerPipeline::Normal(p) => match p.process(0, &repaired_row).await {
            Ok(ExecutionRecord::Success { success, .. }) => match serde_json::to_value(&success) {
                Ok(val) => (StatusCode::OK, Json(val)).into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": format!("Serialization error: {}", e) })),
                )
                    .into_response(),
            },
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
        },
        ServerPipeline::Session(p) => {
            let session_id = p.next_session_id();
            if let Err(e) = p.prep_session(session_id, &[]).await {
                let err_msg = match e.result {
                    Ok(captured) => format!("{:?}", captured),
                    Err(s) => s,
                };
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": err_msg })),
                )
                    .into_response();
            }

            match p.call(session_id, &repaired_row).await {
                Ok(SessionResult::Continue { result, .. })
                | Ok(SessionResult::End { result, .. }) => match serde_json::to_value(&result) {
                    Ok(mut val) => {
                        if let Some(obj) = val.as_object_mut() {
                            obj.insert("session_id".to_string(), serde_json::json!(session_id));
                        }
                        (StatusCode::OK, Json(val)).into_response()
                    }
                    Err(e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({ "error": format!("Serialization error: {}", e) })),
                    )
                        .into_response(),
                },
                Ok(SessionResult::Terminate { .. }) => {
                    let mut val = serde_json::json!({});
                    val.as_object_mut()
                        .unwrap()
                        .insert("session_id".to_string(), serde_json::json!(session_id));
                    (StatusCode::OK, Json(val)).into_response()
                }
                Err(e) => {
                    let err_msg = match e.result {
                        Ok(captured) => format!("{:?}", captured),
                        Err(s) => s,
                    };
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({ "error": err_msg })),
                    )
                        .into_response()
                }
            }
        }
        ServerPipeline::SessionDiff(p) => {
            let session_id = p.next_session_id();
            if let Err(e) = p.prep_session(session_id, &[], &[]).await {
                let err_msg = match e.result {
                    Ok(captured) => format!("{:?}", captured),
                    Err(s) => s,
                };
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": err_msg })),
                )
                    .into_response();
            }

            match p.call(session_id, &repaired_row).await {
                Ok(SessionResult::Continue { result, .. })
                | Ok(SessionResult::End { result, .. }) => match serde_json::to_value(&result) {
                    Ok(mut val) => {
                        if let Some(obj) = val.as_object_mut() {
                            obj.insert("session_id".to_string(), serde_json::json!(session_id));
                        }
                        (StatusCode::OK, Json(val)).into_response()
                    }
                    Err(e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({ "error": format!("Serialization error: {}", e) })),
                    )
                        .into_response(),
                },
                Ok(SessionResult::Terminate { .. }) => {
                    let mut val = serde_json::json!({});
                    val.as_object_mut()
                        .unwrap()
                        .insert("session_id".to_string(), serde_json::json!(session_id));
                    (StatusCode::OK, Json(val)).into_response()
                }
                Err(e) => {
                    let err_msg = match e.result {
                        Ok(captured) => format!("{:?}", captured),
                        Err(s) => s,
                    };
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({ "error": err_msg })),
                    )
                        .into_response()
                }
            }
        }
    }
}

async fn handle_playbook_session_query(
    axum::extract::Path(session_id): axum::extract::Path<u32>,
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

    // 2. Repair input PyroRow
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

    // 3. Process the repaired row in the session pipeline
    let mut pipeline = server.pipeline.lock().await;
    match &mut *pipeline {
        ServerPipeline::Normal(_) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Normal pipeline does not support sessions" })),
        )
            .into_response(),
        ServerPipeline::Session(p) => {
            if !p.active_sessions.contains_key(&session_id) {
                if let Err(e) = p.prep_session(session_id, &[]).await {
                    let err_msg = match e.result {
                        Ok(captured) => format!("{:?}", captured),
                        Err(s) => s,
                    };
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({ "error": err_msg })),
                    )
                        .into_response();
                }
            }

            match p.call(session_id, &repaired_row).await {
                Ok(SessionResult::Continue { result, .. })
                | Ok(SessionResult::End { result, .. }) => match serde_json::to_value(&result) {
                    Ok(mut val) => {
                        if let Some(obj) = val.as_object_mut() {
                            obj.insert("session_id".to_string(), serde_json::json!(session_id));
                        }
                        (StatusCode::OK, Json(val)).into_response()
                    }
                    Err(e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({ "error": format!("Serialization error: {}", e) })),
                    )
                        .into_response(),
                },
                Ok(SessionResult::Terminate { .. }) => {
                    let mut val = serde_json::json!({});
                    val.as_object_mut()
                        .unwrap()
                        .insert("session_id".to_string(), serde_json::json!(session_id));
                    (StatusCode::OK, Json(val)).into_response()
                }
                Err(e) => {
                    let err_msg = match e.result {
                        Ok(captured) => format!("{:?}", captured),
                        Err(s) => s,
                    };
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({ "error": err_msg })),
                    )
                        .into_response()
                }
            }
        }
        ServerPipeline::SessionDiff(p) => {
            if !p.active_sessions.contains_key(&session_id) {
                if let Err(e) = p.prep_session(session_id, &[], &[]).await {
                    let err_msg = match e.result {
                        Ok(captured) => format!("{:?}", captured),
                        Err(s) => s,
                    };
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({ "error": err_msg })),
                    )
                        .into_response();
                }
            }

            match p.call(session_id, &repaired_row).await {
                Ok(SessionResult::Continue { result, .. })
                | Ok(SessionResult::End { result, .. }) => match serde_json::to_value(&result) {
                    Ok(mut val) => {
                        if let Some(obj) = val.as_object_mut() {
                            obj.insert("session_id".to_string(), serde_json::json!(session_id));
                        }
                        (StatusCode::OK, Json(val)).into_response()
                    }
                    Err(e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({ "error": format!("Serialization error: {}", e) })),
                    )
                        .into_response(),
                },
                Ok(SessionResult::Terminate { .. }) => {
                    let mut val = serde_json::json!({});
                    val.as_object_mut()
                        .unwrap()
                        .insert("session_id".to_string(), serde_json::json!(session_id));
                    (StatusCode::OK, Json(val)).into_response()
                }
                Err(e) => {
                    let err_msg = match e.result {
                        Ok(captured) => format!("{:?}", captured),
                        Err(s) => s,
                    };
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({ "error": err_msg })),
                    )
                        .into_response()
                }
            }
        }
    }
}

async fn handle_playbook_schema(State(server): State<Arc<PlaybookHttpServer>>) -> Response {
    Json((*server.spec).clone()).into_response()
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
        _ => {
            let err = "Input JSON must be an object representing a Row".to_string();
            tracing::error!("{}", err);
            Err(err)
        }
    }
}
