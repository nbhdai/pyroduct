use pyroduct::format::PyroVec;
use pyroduct::format::header::{PyroHeader, PyroHeaderMut};
use pyroduct::transport::{PyroListener, PyroRouter, PyroServer, PyroSocket};

#[tokio::test]
async fn test_pyro_server_capability_call() {
    // 1. Setup Router and Server
    // Using one of the existing test dylibs for capability
    let lib_path = "./test/capabilities/nbhdai/state/0.1.0/lib.dylib";
    let router =
        PyroRouter::load("state".into(), lib_path).expect("Failed to load capability library");

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

    // 3. Configure Class (fn_id = 0)
    let mut config_req = PyroVec::ok();
    config_req.set_class_id(0);
    config_req.set_fn_id(0);

    let config_resp = socket
        .request(&config_req.view())
        .await
        .expect("Failed to configure class");
    assert!(config_resp.is_ok(), "Configuration should be successful");

    // 4. Register Client (fn_id = 1)
    let mut reg_req = PyroVec::ok();
    reg_req.set_class_id(0);
    reg_req.set_fn_id(1);

    let reg_resp = socket
        .request(&reg_req.view())
        .await
        .expect("Failed to register client");
    // reg_resp should contain the client_id (u32)
    let client_id = u32::from_le_bytes(reg_resp.as_slice()[0..4].try_into().unwrap());
    assert!(client_id > 0);

    // 5. Call Method (fn_id = 3 for method_index = 1, or fn_id = 2 for reset)
    // Let's try fn_id = 3 (method 1)
    let mut call_req = PyroVec::ok();
    call_req.set_class_id(0);
    call_req.set_fn_id(3);
    call_req.set_client_id(client_id);

    let call_resp = socket
        .request(&call_req.view())
        .await
        .expect("Failed to call method");
    assert!(call_resp.is_ok(), "Method call should be successful");
}
