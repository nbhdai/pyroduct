//! Integration test for `QueryPlaybook` with an input/output JOIN.
//!
//! This test verifies that after calling the `basic` module (input: &str → output: String),
//! the `inputs` and `outputs` tables are both populated and can be joined on their
//! metadata index:
//!
//!   SELECT i.input, o.output
//!   FROM inputs  i
//!   JOIN outputs o ON i._input_meta.index = o._output_meta.index
//!   ORDER BY i._input_meta.index

use std::time::Duration;

use arrow::array::StringArray;
use pyro_artifacts::{
    build::{AnonPlaybook, Builder},
    cache::CacheManager,
};
use pyro_daemon::client::DaemonClient;
use pyro_daemon::{DaemonRequest, DaemonResponse, PyroDaemon};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

const JOIN_SQL: &str = "
    SELECT i.input, o.output
    FROM inputs  i
    JOIN outputs o ON i._input_meta.index = o._output_meta.index
    ORDER BY i._input_meta.index
";

#[tokio::test]
async fn test_daemon_sql_input_output_join() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "trace,cranelift_frontend=off,cranelift_codegen=off,wasmtime=off".into()
    });
    let _ = tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).pretty())
        .with(filter)
        .try_init();

    // ── 1. Compile the basic module ───────────────────────────────────────────
    let cache = std::sync::Arc::new(CacheManager::from_env().await.unwrap());
    let builder = Builder::from_env(cache.clone()).await.unwrap();

    let package = "test_sql_join_basic";
    let source_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../modules/basic/src/lib.rs");
    let source_code =
        std::fs::read_to_string(&source_path).expect("Failed to read modules/basic/src/lib.rs");

    let anon = AnonPlaybook {
        package: package.to_string(),
        dependencies: std::collections::BTreeMap::new(),
        configurations: Vec::new(),
        source: source_code,
        interconnect: std::collections::BTreeMap::new(),
    };

    let _ = cache.remove_module("anon", package, "0.1.0").await;
    let binary = builder
        .compile_anon(&anon)
        .await
        .expect("basic module should compile");

    // ── 2. Start daemon ───────────────────────────────────────────────────────
    let test_dir = tempfile::tempdir().unwrap();
    let working_dir = test_dir.path().to_path_buf();
    let control_socket = working_dir.join("control");

    let daemon = PyroDaemon::new(working_dir.clone()).await;
    let daemon_handle = tokio::spawn(async move { daemon.run().await.unwrap() });

    let mut retries = 0;
    while !control_socket.exists() {
        if retries > 50 {
            panic!("Daemon failed to bind control socket in time");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
        retries += 1;
    }

    let client = DaemonClient::connect(&control_socket).await.unwrap();

    // ── 3. Start the basic playbook ───────────────────────────────────────────
    let req = DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::Start {
        name: "basic".to_string(),
        pipeline_config: binary.spec.ident.clone(),
        playbook_socket: None,
        http_address: None,
        input_dir: Some(working_dir.join("input")),
        output_dir: Some(working_dir.join("output")),
        pinned_version: None,
        configurations: None,
        num_workers: None,
    });
    client.request(req).await.unwrap();

    // ── 4. Send two rows via BulkCall ─────────────────────────────────────────
    let csv_content = "input\nhello\nworld\n".as_bytes().to_vec();
    let req = DaemonRequest::Playbook(pyro_daemon::playbook::PlaybookRequest::BulkCall {
        name: "basic".to_string(),
        file_name: "test.csv".to_string(),
        file_content: csv_content,
    });
    let resp = client.request(req).await.unwrap();

    match resp {
        DaemonResponse::Playbook(pyro_daemon::playbook::PlaybookResponse::BulkCallResult {
            results,
        }) => {
            assert_eq!(results.len(), 2, "expected 2 bulk-call results");
        }
        other => panic!("unexpected response to BulkCall: {:?}", other),
    }

    // ── 5. Run the join query ─────────────────────────────────────────────────
    let ipc = client
        .query_playbook("basic".to_string(), JOIN_SQL.to_string())
        .await
        .expect("query_playbook should succeed");

    assert!(!ipc.is_empty(), "IPC bytes should be non-empty");

    // ── 6. Decode Arrow IPC and verify rows ───────────────────────────────────
    let mut reader =
        arrow::ipc::reader::FileReader::try_new(std::io::Cursor::new(ipc), None).unwrap();

    let mut inputs_col: Vec<String> = Vec::new();
    let mut outputs_col: Vec<String> = Vec::new();

    while let Some(Ok(batch)) = reader.next() {
        let input_arr = batch
            .column_by_name("input")
            .expect("column 'input' missing")
            .as_any()
            .downcast_ref::<StringArray>()
            .expect("'input' should be StringArray");

        let output_arr = batch
            .column_by_name("output")
            .expect("column 'output' missing")
            .as_any()
            .downcast_ref::<StringArray>()
            .expect("'output' should be StringArray");

        for i in 0..batch.num_rows() {
            inputs_col.push(input_arr.value(i).to_string());
            outputs_col.push(output_arr.value(i).to_string());
        }
    }

    assert_eq!(inputs_col.len(), 2, "expected 2 joined rows");
    assert_eq!(inputs_col,  vec!["hello", "world"]);
    assert_eq!(outputs_col, vec!["Prefixed: hello", "Prefixed: world"]);

    daemon_handle.abort();
}
