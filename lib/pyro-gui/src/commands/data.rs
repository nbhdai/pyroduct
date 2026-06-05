use serde_json::Value;

#[tauri::command]
pub async fn get_playbook_data(playbook_name: String, offset: usize, limit: usize) -> Result<Value, String> {
    let working_dir = pyro_daemon::PyroDaemon::default_working_dir();
    let control_socket_path = working_dir.join("control");

    if !control_socket_path.exists() {
        return Err("Daemon control socket does not exist (offline)".to_string());
    }

    let client = pyro_daemon::client::DaemonClient::connect(&control_socket_path)
        .await
        .map_err(|e| format!("Failed to connect to daemon: {:?}", e))?;

    let ipc_bytes = client
        .get_playbook_data(playbook_name, offset, limit)
        .await
        .map_err(|e| format!("Failed to get playbook data: {:?}", e))?;

    if ipc_bytes.is_empty() {
        return Ok(serde_json::json!({
            "schema": [],
            "rows": []
        }));
    }

    let mut reader = arrow::ipc::reader::FileReader::try_new(std::io::Cursor::new(ipc_bytes), None)
        .map_err(|e| format!("Failed to parse Arrow IPC: {:?}", e))?;

    let mut rows = Vec::new();
    let mut fields = Vec::new();
    let mut first = true;
    while let Some(batch_res) = reader.next() {
        let batch = batch_res.map_err(|e| format!("Failed to read batch: {:?}", e))?;
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
            first = false;
        }

        use pyroduct::format::value::arrow::Rowable;
        for i in 0..batch.num_rows() {
            let row = batch.row(i).map_err(|e| format!("Row extraction failed: {:?}", e))?;
            let json_row = serde_json::to_value(&row).map_err(|e| e.to_string())?;
            rows.push(json_row);
        }
    }

    Ok(serde_json::json!({
        "schema": fields,
        "rows": rows,
    }))
}

#[tauri::command]
pub async fn query_playbook_data(playbook_name: String, sql_query: String) -> Result<Value, String> {
    let working_dir = pyro_daemon::PyroDaemon::default_working_dir();
    let control_socket_path = working_dir.join("control");

    if !control_socket_path.exists() {
        return Err("Daemon control socket does not exist (offline)".to_string());
    }

    let client = pyro_daemon::client::DaemonClient::connect(&control_socket_path)
        .await
        .map_err(|e| format!("Failed to connect to daemon: {:?}", e))?;

    let ipc_bytes = client
        .query_playbook(playbook_name, sql_query)
        .await
        .map_err(|e| format!("Query failed: {:?}", e))?;

    if ipc_bytes.is_empty() {
        return Ok(serde_json::json!({
            "schema": [],
            "rows": []
        }));
    }

    let mut reader = arrow::ipc::reader::FileReader::try_new(std::io::Cursor::new(ipc_bytes), None)
        .map_err(|e| format!("Failed to parse Arrow IPC: {:?}", e))?;

    let mut rows = Vec::new();
    let mut fields = Vec::new();
    let mut first = true;
    while let Some(batch_res) = reader.next() {
        let batch = batch_res.map_err(|e| format!("Failed to read batch: {:?}", e))?;
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
            first = false;
        }

        use pyroduct::format::value::arrow::Rowable;
        for i in 0..batch.num_rows() {
            let row = batch.row(i).map_err(|e| format!("Row extraction failed: {:?}", e))?;
            let json_row = serde_json::to_value(&row).map_err(|e| e.to_string())?;
            rows.push(json_row);
        }
    }

    Ok(serde_json::json!({
        "schema": fields,
        "rows": rows,
    }))
}
