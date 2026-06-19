use std::sync::Arc;
use tempfile::tempdir;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use pyro_artifacts::{
    build::{AnonPlaybook, Builder},
    cache::CacheManager,
};
use pyro_daemon::playbook::PlaybooksManager;
use pyroduct::pipeline::ServerExecutionRecord;

const CHATBOT_SOURCE: &str = r#"
use pyroduct::session::SessionResponse;

#[pyroduct::module(session, output = message)]
fn chatbot(
    prior: Vec<String>,
    input: String,
) -> Result<SessionResponse<String>> {
    let _ = input;
    let count = (prior.len() as u32 / 2) + 1;
    Ok(SessionResponse::Continue(format!("Count: {}", count)))
}
"#;

const CALLER_SOURCE: &str = r#"
use pyroduct::call_playbook;
use pyroduct::call_session;
use pyroduct::format::PyroRow;

#[pyroduct::module(output = message)]
fn caller(
    input: String,
) -> Result<String> {
    let _ = input;
    let target_input = PyroRow::from([("input", "msg 1".into())]);
    let (session_id, target_output) = call_playbook("chatbot", &target_input);
    let reply = target_output.get_str("message").unwrap_or("error").to_string();
    if reply != "Count: 1" {
        panic!("Expected Count: 1, got {}", reply);
    }

    for i in 2..=10 {
        let msg_input = PyroRow::from([("input", format!("msg {}", i).into())]);
        let target_output = call_session("chatbot", session_id, &msg_input);
        let reply = target_output.get_str("message").unwrap_or("error").to_string();
        if reply != format!("Count: {}", i) {
            panic!("Expected Count: {}, got {}", i, reply);
        }
    }
    Ok("success".to_string())
}
"#;

#[tokio::test]
async fn test_interconnect_persistence() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "trace,cranelift_frontend=off,cranelift_codegen=off,wasmtime=off".into()
    });

    let _ = tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).pretty())
        .with(filter)
        .try_init();

    let cache = Arc::new(CacheManager::from_env().await.unwrap());
    let builder = Builder::from_env(cache.clone()).await.unwrap();

    // 1. Compile chatbot session playbook
    let chatbot_package = "test_interconnect_persistence_chatbot";
    let chatbot_source = AnonPlaybook {
        package: chatbot_package.to_string(),
        dependencies: std::collections::BTreeMap::new(),
        configurations: Vec::new(),
        source: CHATBOT_SOURCE.to_string(),
        interconnect: std::collections::BTreeMap::new(),
    };
    let _ = cache.remove_module("anon", chatbot_package, "0.1.0").await;
    let target_binary = builder
        .compile_anon(&chatbot_source)
        .await
        .expect("Chatbot session module should compile");

    // 2. Compile caller session playbook
    let caller_package = "test_interconnect_persistence_caller";
    let mut interconnect_map = std::collections::BTreeMap::new();
    interconnect_map.insert("chatbot".to_string(), target_binary.spec.ident.clone());
    let caller_source = AnonPlaybook {
        package: caller_package.to_string(),
        dependencies: std::collections::BTreeMap::new(),
        configurations: Vec::new(),
        source: CALLER_SOURCE.to_string(),
        interconnect: interconnect_map,
    };
    let _ = cache.remove_module("anon", caller_package, "0.1.0").await;
    let caller_binary = builder
        .compile_anon(&caller_source)
        .await
        .expect("Caller session module should compile");

    let test_dir = tempdir().unwrap();
    let working_dir = test_dir.path().to_path_buf();

    // 3. Initialize PlaybooksManager
    let pm = Arc::new(PlaybooksManager::new(working_dir.clone()));
    pm.start_playbook(
        "caller".to_string(),
        caller_binary.spec.ident.clone(),
        None,
        None,
        Some(working_dir.join("input")),
        Some(working_dir.join("output")),
        None,
        None,
    )
    .await
    .unwrap();

    // Verify 2 running playbooks ("caller" and the automatically started "caller_test_interconnect_persistence_chatbot")
    assert_eq!(pm.active_workers_count().await, 2);

    // Make a dummy call to chatbot to consume session ID 0
    let chatbot_name = format!("caller_{}", target_binary.spec.ident.package);
    let _ = pm
        .call_playbook_record(&chatbot_name, serde_json::json!({ "input": "dummy" }), None)
        .await
        .unwrap();

    // Call caller. Inside caller, it will start session with chatbot and call it 10 times.
    let payload = serde_json::json!({ "input": "go" });
    let result = pm
        .call_playbook_record("caller", payload, None)
        .await
        .unwrap();

    match result {
        ServerExecutionRecord::Normal(pyroduct::pipeline::ExecutionRecord::Success {
            success,
            ..
        }) => {
            let reply = success.get_str("message").unwrap();
            assert_eq!(reply, "success");
        }
        other => panic!("Expected Success, got {:?}", other),
    }

    // Stop workers
    if let Some(worker) = pm.workers.lock().await.remove("caller") {
        worker.shutdown().await.unwrap();
    }
    if let Some(worker) = pm.workers.lock().await.remove(&chatbot_name) {
        worker.shutdown().await.unwrap();
    }
}
