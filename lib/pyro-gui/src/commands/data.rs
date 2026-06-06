use serde_json::Value;
use tracing::{trace, debug, info, error};

#[tauri::command]
pub async fn get_playbook_data(playbook_name: String, offset: usize, limit: usize) -> Result<Value, String> {
    info!("Tauri command: get_playbook_data for playbook='{}' offset={} limit={}", playbook_name, offset, limit);
    let working_dir = pyro_daemon::PyroDaemon::default_working_dir();
    let control_socket_path = working_dir.join("control");

    if !control_socket_path.exists() {
        error!("Daemon control socket does not exist at {:?}", control_socket_path);
        return Err("Daemon control socket does not exist (offline)".to_string());
    }

    debug!("Connecting to daemon at {:?}", control_socket_path);
    let client = pyro_daemon::client::DaemonClient::connect(&control_socket_path)
        .await
        .map_err(|e| {
            error!("Failed to connect to daemon: {:?}", e);
            format!("Failed to connect to daemon: {:?}", e)
        })?;

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

    info!("Successfully retrieved {} rows for playbook '{}'", rows.len(), playbook_name);
    Ok(serde_json::json!({
        "schema": fields,
        "rows": rows,
    }))
}

#[tauri::command]
pub async fn query_playbook_data(playbook_name: String, sql_query: String) -> Result<Value, String> {
    info!("Tauri command: query_playbook_data for playbook='{}', query='{}'", playbook_name, sql_query);
    let working_dir = pyro_daemon::PyroDaemon::default_working_dir();
    let control_socket_path = working_dir.join("control");

    if !control_socket_path.exists() {
        error!("Daemon control socket does not exist at {:?}", control_socket_path);
        return Err("Daemon control socket does not exist (offline)".to_string());
    }

    debug!("Connecting to daemon at {:?}", control_socket_path);
    let client = pyro_daemon::client::DaemonClient::connect(&control_socket_path)
        .await
        .map_err(|e| {
            error!("Failed to connect to daemon: {:?}", e);
            format!("Failed to connect to daemon: {:?}", e)
        })?;

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

    info!("Successfully query returned {} rows for playbook '{}'", rows.len(), playbook_name);
    Ok(serde_json::json!({
        "schema": fields,
        "rows": rows,
    }))
}
