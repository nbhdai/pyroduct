use pyro_artifacts::{
    build::{AnonPlaybook, Builder},
    cache::CacheManager,
};
use pyroduct::{
    PyroRow,
    pipeline::{ExecutionRecord, PipelineConfig},
};
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicUsize, Ordering};

const ERROR_MODULE: &str = r#"
use pyroduct;

#[pyroduct::module(output = message)]
pub fn call(input: String) -> Result<String> {
    if input == "panic" {
        panic!("intentional panic");
    }
    if input == "error" {
        return Err(pyroduct::capture!("intentional error"));
    }
    Ok(format!("Success: {}", input))
}
"#;

static CALLBACK_COUNTER: AtomicUsize = AtomicUsize::new(0);

async fn my_callback(row_index: usize, row: &PyroRow<'_>) {
    assert_eq!(row_index, 0);
    assert_eq!(row.get_str("input").unwrap(), "hello");
    CALLBACK_COUNTER.fetch_add(1, Ordering::SeqCst);
}

static ENUM_CALLBACK_COUNTER: AtomicUsize = AtomicUsize::new(0);

async fn my_enum_callback(row_index: usize, row: &PyroRow<'_>) {
    assert_eq!(row_index, 0);
    assert_eq!(row.get_str("input").unwrap(), "hello");
    ENUM_CALLBACK_COUNTER.fetch_add(1, Ordering::SeqCst);
}

#[tokio::test]
async fn test_pipeline_success_callbacks() {
    let cache = std::sync::Arc::new(CacheManager::from_env().await.unwrap());
    let builder = Builder::from_env(cache.clone()).await.unwrap();

    let source = AnonPlaybook {
        package: "test_integration_callback".to_string(),
        dependencies: BTreeMap::new(),
        configurations: Vec::new(),
        source: ERROR_MODULE.to_string(),
        interconnect: std::collections::BTreeMap::new(),
    };
    cache
        .remove_module("anon", "test_integration_callback", "0.1.0")
        .await
        .unwrap();

    let binary = builder
        .compile_anon(&source)
        .await
        .expect("Module should compile");

    let tmp_dir = tempfile::tempdir().unwrap();
    let tmp_path = tmp_dir.path().to_path_buf();

    let ident = &binary.spec.ident;

    let config = PipelineConfig {
        playbook: ident.clone(),
        remote: HashMap::new(),
        wal_capacity: 5,
        success_log_retention_secs: 3600,
        error_log_retention_secs: 86400 * 7,
        input_dir: tmp_path.clone(),
        output_dir: tmp_path.clone(),
        log_dir: tmp_path.clone(),
    };

    let loaded = config.load(&cache).await.unwrap();
    let factory = loaded.factory().unwrap();
    let mut pipeline = factory.build().await.unwrap();

    // Register our function pointer callback wrapped via Callback::function
    pipeline.callbacks.push((
        uuid::Uuid::new_v4(),
        pyroduct::pipeline::Callback::function(|idx, row| {
            let row_static = row.to_static();
            Box::pin(async move {
                my_callback(idx, &row_static).await;
            })
        }),
    ));

    // Register our enum callback
    pipeline.callbacks.push((
        uuid::Uuid::new_v4(),
        pyroduct::pipeline::Callback::function(|idx, row| {
            let row_static = row.to_static();
            Box::pin(async move {
                my_enum_callback(idx, &row_static).await;
            })
        }),
    ));

    // Also register a socket and HTTP callback to verify the connection and HTTP constructors
    if let Ok(cb) = pyroduct::pipeline::Callback::connect_socket_tcp("127.0.0.1:9876").await {
        pipeline.callbacks.push((uuid::Uuid::new_v4(), cb));
    }
    pipeline.callbacks.push((uuid::Uuid::new_v4(), pyroduct::pipeline::Callback::http("http://127.0.0.1:9876/callback")));

    // Test Success
    let input_success = PyroRow::from([("input", "hello".into())]);
    CALLBACK_COUNTER.store(0, Ordering::SeqCst);
    ENUM_CALLBACK_COUNTER.store(0, Ordering::SeqCst);

    let res_success = pipeline.process(0, &input_success).await.unwrap();
    if let ExecutionRecord::Success { success, .. } = &res_success {
        assert_eq!(success.get_str("message").unwrap(), "Success: hello");
    } else {
        panic!("Expected Success");
    }

    assert_eq!(CALLBACK_COUNTER.load(Ordering::SeqCst), 1);
    assert_eq!(ENUM_CALLBACK_COUNTER.load(Ordering::SeqCst), 1);

    // Test that on failure, callbacks are NOT called
    let input_error = PyroRow::from([("input", "error".into())]);
    let res_error = pipeline.process(1, &input_error).await.unwrap();
    assert!(matches!(res_error, ExecutionRecord::Failure { .. }));

    // Counters should still be 1 (not incremented)
    assert_eq!(CALLBACK_COUNTER.load(Ordering::SeqCst), 1);
    assert_eq!(ENUM_CALLBACK_COUNTER.load(Ordering::SeqCst), 1);
}
