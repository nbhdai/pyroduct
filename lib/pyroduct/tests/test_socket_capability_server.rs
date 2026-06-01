use pyroduct::format::PyroVec;
use pyroduct::format::header::{DataStatus, PyroHeader, PyroHeaderMut};
use pyroduct::transport::socket::{
    PyroListener, PyroSocket,
    capability::{PyroRouter, PyroServer},
};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::test]
async fn test_pyro_server_capability_call() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "trace,cranelift_frontend=off,cranelift_codegen=off,wasmtime=off,mio=off".into()
    });

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).pretty())
        .with(filter)
        .init();

    // 1. Setup Router and Server
    let cache = pyro_artifacts::cache::CacheManager::from_env().await.unwrap();
    let lib_path = cache
        .capability_binary_path("nbhdai", "state", "0.1.0")
        .await
        .unwrap();

    let cap_config = pyro_artifacts::artifacts::CapabilityConfig {
        classes: std::collections::HashMap::from([(
            "counter".to_string(),
            Some(serde_json::json!({"max_value": 100u64})),
        )]),
    };

    let router = PyroRouter::load(
        pyro_artifacts::cargo::CapabilityIdent {
            author: "nbhdai".to_string(),
            package: "state".to_string(),
            version: "0.1.0".to_string(),
        },
        &cap_config,
        &lib_path,
    )
    .await
    .expect("Failed to load capability library");

    let server = PyroServer::new(router);
    let listener = PyroListener::bind_tcp("127.0.0.1:0")
        .await
        .expect("Failed to bind listener");
    let addr = listener.local_addr_tcp().expect("Failed to get local addr");

    // Run server in background
    tokio::spawn(async move {
        if let Err(e) = server.run(listener).await {
            eprintln!("Server error: {:?}", e);
        }
    });

    // 2. Client Connection
    let socket = PyroSocket::connect_tcp(addr)
        .await
        .expect("Failed to connect to server");

    // 3. Fetch Interface (fn_id = 0) - returns the interface spec
    let fetch_req = PyroVec::ok();
    let fetch_resp = socket
        .request(None, None, Some(0), fetch_req.view())
        .await
        .expect("Failed to fetch interface");
    assert!(fetch_resp.is_ok(), "Interface fetch should be successful");

    // 5. Register Client (fn_id = 2) - returns client_id
    let mut reg_req = PyroVec::ok();
    reg_req.set_fn_id(2);
    reg_req.extend_from_slice(&0u64.to_le_bytes());
    reg_req.set_status(DataStatus::RkyvValid);

    let reg_resp = socket
        .request(None, None, Some(2), reg_req.view())
        .await
        .expect("Failed to register client");
    // reg_resp should contain the client_id (u32)
    let client_id = u32::from_le_bytes(reg_resp.as_slice()[0..4].try_into().unwrap());
    assert!(client_id > 0);

    // 6. Reset Class (fn_id = 3)
    let mut reset_req = PyroVec::ok();
    reset_req.set_class_id(0);
    reset_req.set_fn_id(3);

    let reset_resp = socket
        .request(None, Some(0), Some(3), reset_req.view())
        .await
        .expect("Failed to reset class");
    assert!(reset_resp.is_ok(), "Reset should be successful");

    // 7. Call Method (fn_id = 4 for method_index = 0)
    let mut call_req = PyroVec::ok();
    call_req.set_class_id(0);
    call_req.set_fn_id(4);

    let call_resp = socket
        .request(Some(client_id), Some(0), Some(4), call_req.view())
        .await
        .expect("Failed to call method");
    assert!(call_resp.is_ok(), "Method call should be successful");
}
