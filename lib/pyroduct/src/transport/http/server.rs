use crate::PyroRow;
use crate::pipeline::PipelineServer;
use axum::response::Response;
use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::post};

/// Return the Axum `Router` configured for the playbook.
pub fn router(server: PipelineServer) -> Router {
    Router::new()
        .route("/", post(handle_playbook_query))
        .route("/{session_id}", post(handle_playbook_session_query))
        .route("/schema", axum::routing::get(handle_playbook_schema))
        .with_state(server)
}

/// Run the HTTP server in a background task, accepting connections on the provided TCP listener.
/// Returns a `tokio::sync::oneshot::Sender<()>` which can be used to interrupt and stop the server.
pub fn run(
    server: PipelineServer,
    listener: tokio::net::TcpListener,
) -> tokio::sync::oneshot::Sender<()> {
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let app = router(server);

    tokio::spawn(async move {
        loop {
            tokio::select! {
                accept_res = listener.accept() => {
                    let (stream, _addr) = match accept_res {
                        Ok(val) => val,
                        Err(e) => {
                            tracing::error!("Failed to accept TCP connection: {:?}", e);
                            break;
                        }
                    };

                    let io = hyper_util::rt::TokioIo::new(stream);
                    let service = hyper_util::service::TowerToHyperService::new(app.clone());

                    tokio::spawn(async move {
                        if let Err(err) = hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new())
                            .serve_connection(io, service)
                            .await
                        {
                            tracing::debug!("Error serving HTTP connection: {:?}", err);
                        }
                    });
                }
                _ = &mut shutdown_rx => {
                    tracing::info!("HTTP server received shutdown signal");
                    break;
                }
            }
        }
    });

    shutdown_tx
}

async fn handle_playbook_query(
    State(server): State<PipelineServer>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    // 1. Convert JSON to PyroRow
    let input_row: PyroRow<'static> = match serde_json::from_value(payload) {
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
    let repaired_row = match input_row.project_repair(server.spec().func.input.fields()) {
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

    // 3. Process the repaired row in the pipeline server
    let kind = server.spec().func.kind;
    match server.call(repaired_row).await.and_then(|rec| rec.into_result()) {
        Ok((session_id, success_row)) => match serde_json::to_value(&success_row) {
            Ok(mut val) => {
                if kind != pyro_spec::ModuleKind::Normal {
                    if let Some(obj) = val.as_object_mut() {
                        obj.insert("session_id".to_string(), serde_json::json!(session_id));
                    }
                }
                (StatusCode::OK, Json(val)).into_response()
            }
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Serialization error: {}", e) })),
            )
                .into_response(),
        },
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("{:?}", e) })),
        )
            .into_response(),
    }
}

async fn handle_playbook_session_query(
    axum::extract::Path(session_id): axum::extract::Path<u32>,
    State(server): State<PipelineServer>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    // 1. Convert JSON to PyroRow
    let input_row: PyroRow<'static> = match serde_json::from_value(payload) {
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
    let repaired_row = match input_row.project_repair(server.spec().func.input.fields()) {
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

    // 3. Validate that the pipeline supports sessions
    let kind = server.spec().func.kind;
    if kind == pyro_spec::ModuleKind::Normal {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Normal pipeline does not support sessions" })),
        )
            .into_response();
    }

    // 4. Process the repaired row in the session pipeline
    match server.call_session(session_id, repaired_row).await.and_then(|rec| rec.into_result().map(|(_, r)| r)) {
        Ok(success_row) => match serde_json::to_value(&success_row) {
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
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("{:?}", e) })),
        )
            .into_response(),
    }
}

async fn handle_playbook_schema(State(server): State<PipelineServer>) -> Response {
    Json(server.spec().as_ref().clone()).into_response()
}
