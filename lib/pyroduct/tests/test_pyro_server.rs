use pyroduct::format::PyroVec;
use pyroduct::format::header::{PyroHeader, PyroHeaderMut};
use pyroduct::format::tokio::RequestInner;
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

    // 3. Fetch Interface (fn_id = 0) - returns the interface spec
    let fetch_req = PyroVec::ok();
    let fetch_resp = socket
        .request(None, None, Some(0), RequestInner::Vec(fetch_req))
        .await
        .expect("Failed to fetch interface");
    assert!(fetch_resp.is_ok(), "Interface fetch should be successful");

    // 4. Configure Class (fn_id = 1) - instantiates the class
    let mut config_req = PyroVec::ok();
    config_req.set_class_id(0);
    config_req.set_fn_id(1);

    let config_resp = socket
        .request(None, Some(0), Some(1), RequestInner::Vec(config_req))
        .await
        .expect("Failed to configure class");
    assert!(config_resp.is_ok(), "Configuration should be successful");

    // 5. Register Client (fn_id = 2) - returns client_id
    let mut reg_req = PyroVec::ok();
    reg_req.set_fn_id(2);

    let reg_resp = socket
        .request(None, None, Some(2), RequestInner::Vec(reg_req))
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
        .request(None, Some(0), Some(3), RequestInner::Vec(reset_req))
        .await
        .expect("Failed to reset class");
    assert!(reset_resp.is_ok(), "Reset should be successful");

    // 7. Call Method (fn_id = 4 for method_index = 0)
    let mut call_req = PyroVec::ok();
    call_req.set_class_id(0);
    call_req.set_fn_id(4);

    let call_resp = socket
        .request(
            Some(client_id),
            Some(0),
            Some(4),
            RequestInner::Vec(call_req),
        )
        .await
        .expect("Failed to call method");
    assert!(call_resp.is_ok(), "Method call should be successful");
}
