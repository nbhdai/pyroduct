use serde_json::Value;
use tracing::{debug, error, info, trace};

#[tauri::command]
pub async fn get_playbook_data(
    playbook_name: String,
    offset: usize,
    limit: usize,
) -> Result<Value, String> {
    info!(
        "Tauri command: get_playbook_data for playbook='{}' offset={} limit={}",
        playbook_name, offset, limit
    );
    let client = super::connect_to_active_daemon().await?;

    debug!("Querying daemon for playbook data");
    let ipc_bytes = client
        .get_playbook_data(playbook_name.clone(), offset, limit)
        .await
        .map_err(|e| {
            error!("Failed to get playbook data from daemon: {:?}", e);
            format!("Failed to get playbook data: {:?}", e)
        })?;

    trace!("Received {} IPC bytes from daemon", ipc_bytes.len());
    if ipc_bytes.is_empty() {
        debug!("IPC bytes are empty, returning empty schema and rows");
        return Ok(serde_json::json!({
            "schema": [],
            "rows": []
        }));
    }

    debug!("Parsing Arrow IPC bytes");
    let mut reader = arrow::ipc::reader::FileReader::try_new(std::io::Cursor::new(ipc_bytes), None)
        .map_err(|e| {
            error!("Failed to parse Arrow IPC: {:?}", e);
            format!("Failed to parse Arrow IPC: {:?}", e)
        })?;

    let mut rows = Vec::new();
    let mut fields = Vec::new();
    let mut first = true;
    while let Some(batch_res) = reader.next() {
        let batch = batch_res.map_err(|e| {
            error!("Failed to read Arrow batch: {:?}", e);
            format!("Failed to read batch: {:?}", e)
        })?;
        trace!("Processing batch with {} rows", batch.num_rows());
        if first {
            let schema = batch.schema();
            fields = schema
                .fields()
                .iter()
                .map(|f| {
                    serde_json::json!({
                        "name": f.name(),
                        "type": format!("{:?}", f.data_type()),
                        "nullable": f.is_nullable(),
                    })
                })
                .collect();
            trace!("Extracted fields schema: {:?}", fields);
            first = false;
        }

        use pyroduct::format::value::arrow::Rowable;
        for i in 0..batch.num_rows() {
            let row = batch.row(i).map_err(|e| {
                error!("Row extraction failed at index {}: {:?}", i, e);
                format!("Row extraction failed: {:?}", e)
            })?;
            let json_row = serde_json::to_value(&row).map_err(|e| {
                error!("JSON serialization of row failed: {:?}", e);
                e.to_string()
            })?;
            trace!("Row {} data: {:?}", i, json_row);
            rows.push(json_row);
        }
    }

    info!(
        "Successfully retrieved {} rows for playbook '{}'",
        rows.len(),
        playbook_name
    );
    Ok(serde_json::json!({
        "schema": fields,
        "rows": rows,
    }))
}

#[tauri::command]
pub async fn query_playbook_data(
    playbook_name: String,
    sql_query: String,
) -> Result<Value, String> {
    info!(
        "Tauri command: query_playbook_data for playbook='{}', query='{}'",
        playbook_name, sql_query
    );
    let client = super::connect_to_active_daemon().await?;

    debug!("Sending SQL query to daemon");
    let ipc_bytes = client
        .query_playbook(playbook_name.clone(), sql_query)
        .await
        .map_err(|e| {
            error!("SQL Query failed on daemon: {:?}", e);
            format!("Query failed: {:?}", e)
        })?;

    trace!("Received {} IPC bytes from daemon", ipc_bytes.len());
    if ipc_bytes.is_empty() {
        debug!("IPC bytes are empty, returning empty schema and rows");
        return Ok(serde_json::json!({
            "schema": [],
            "rows": []
        }));
    }

    debug!("Parsing Arrow IPC bytes");
    let mut reader = arrow::ipc::reader::FileReader::try_new(std::io::Cursor::new(ipc_bytes), None)
        .map_err(|e| {
            error!("Failed to parse Arrow IPC: {:?}", e);
            format!("Failed to parse Arrow IPC: {:?}", e)
        })?;

    let mut rows = Vec::new();
    let mut fields = Vec::new();
    let mut first = true;
    while let Some(batch_res) = reader.next() {
        let batch = batch_res.map_err(|e| {
            error!("Failed to read Arrow batch: {:?}", e);
            format!("Failed to read batch: {:?}", e)
        })?;
        trace!("Processing batch with {} rows", batch.num_rows());
        if first {
            let schema = batch.schema();
            fields = schema
                .fields()
                .iter()
                .map(|f| {
                    serde_json::json!({
                        "name": f.name(),
                        "type": format!("{:?}", f.data_type()),
                        "nullable": f.is_nullable(),
                    })
                })
                .collect();
            trace!("Extracted fields schema: {:?}", fields);
            first = false;
        }

        use pyroduct::format::value::arrow::Rowable;
        for i in 0..batch.num_rows() {
            let row = batch.row(i).map_err(|e| {
                error!("Row extraction failed at index {}: {:?}", i, e);
                format!("Row extraction failed: {:?}", e)
            })?;
            let json_row = serde_json::to_value(&row).map_err(|e| {
                error!("JSON serialization of row failed: {:?}", e);
                e.to_string()
            })?;
            trace!("Row {} data: {:?}", i, json_row);
            rows.push(json_row);
        }
    }

    info!(
        "Successfully query returned {} rows for playbook '{}'",
        rows.len(),
        playbook_name
    );
    Ok(serde_json::json!({
        "schema": fields,
        "rows": rows,
    }))
}

#[tauri::command]
pub async fn get_playbook_failures(playbook_name: String) -> Result<Value, String> {
    info!(
        "Tauri command: get_playbook_failures for playbook='{}'",
        playbook_name
    );
    let client = super::connect_to_active_daemon().await?;

    match client.get_playbook_failures(playbook_name).await {
        Ok(failures) => serde_json::to_value(failures).map_err(|e| e.to_string()),
        Err(e) => {
            error!("Failed to get playbook failures: {:?}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn get_playbook_execution_record(
    playbook_name: String,
    id: u32,
) -> Result<Value, String> {
    info!(
        "Tauri command: get_playbook_execution_record for playbook='{}' id={}",
        playbook_name, id
    );
    let client = super::connect_to_active_daemon().await?;

    match client
        .get_playbook_execution_record(playbook_name, id)
        .await
    {
        Ok(record) => serde_json::to_value(record).map_err(|e| e.to_string()),
        Err(e) => {
            error!("Failed to get playbook execution record: {:?}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn run_bulk_playbook(
    playbook_name: String,
    file_name: String,
    file_content: Vec<u8>,
) -> Result<serde_json::Value, String> {
    info!(
        "Tauri command: run_bulk_playbook, playbook='{}', file='{}', len={}",
        playbook_name,
        file_name,
        file_content.len()
    );
    let client = super::connect_to_active_daemon().await?;

    let req =
        pyro_daemon::DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::BulkCall {
            name: playbook_name,
            file_name,
            file_content,
        });

    match client.request(req).await {
        Ok(pyro_daemon::DaemonResponse::Playbook(
            pyro_daemon::playbook::PlaybookResponse::BulkCallResult { results },
        )) => serde_json::to_value(results).map_err(|e| format!("Serialization error: {}", e)),
        Ok(pyro_daemon::DaemonResponse::Playbook(
            pyro_daemon::playbook::PlaybookResponse::Error { message },
        )) => Err(message),
        Ok(resp) => Err(format!("Unexpected response: {:?}", resp)),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn start_folder_replay(
    playbook_name: String,
    folder_path: String,
    interval_ms: u64,
    wiggle_ms: u64,
) -> Result<serde_json::Value, String> {
    info!(
        "Tauri command: start_folder_replay for playbook='{}', folder='{}', interval={}ms, wiggle={}ms",
        playbook_name, folder_path, interval_ms, wiggle_ms
    );
    let client = super::connect_to_active_daemon().await?;

    match client
        .start_replay(playbook_name, folder_path, interval_ms, wiggle_ms)
        .await
    {
        Ok(total_rows) => {
            info!("Replay started with {} total rows", total_rows);
            Ok(serde_json::json!({ "total_rows": total_rows }))
        }
        Err(e) => {
            error!("Failed to start replay: {:?}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn get_replay_status(
    playbook_name: String,
) -> Result<serde_json::Value, String> {
    trace!("Tauri command: get_replay_status for playbook='{}'", playbook_name);
    let client = super::connect_to_active_daemon().await?;

    match client.get_replay_status(playbook_name).await {
        Ok((running, total_rows, rows_completed, successes, errors, current_file)) => {
            Ok(serde_json::json!({
                "running": running,
                "total_rows": total_rows,
                "rows_completed": rows_completed,
                "successes": successes,
                "errors": errors,
                "current_file": current_file,
            }))
        }
        Err(e) => {
            error!("Failed to get replay status: {:?}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn stop_folder_replay(
    playbook_name: String,
) -> Result<String, String> {
    info!("Tauri command: stop_folder_replay for playbook='{}'", playbook_name);
    let client = super::connect_to_active_daemon().await?;

    match client.stop_replay(playbook_name).await {
        Ok(()) => {
            info!("Replay stopped successfully");
            Ok("Replay stopped".to_string())
        }
        Err(e) => {
            error!("Failed to stop replay: {:?}", e);
            Err(e.to_string())
        }
    }
}

