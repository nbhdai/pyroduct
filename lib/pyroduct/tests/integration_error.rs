use pyro_artifacts::{
    artifacts::{ModuleDependencies, ModuleSource, Playbook},
    build::Builder,
    cache::CacheManager,
};
use pyroduct::{
    PyroRow,
    pipeline::{ExecutionRecord, PipelineConfig},
};
use std::collections::{BTreeMap, HashMap};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

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

#[tokio::test]
async fn test_module_errors_and_panics() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "trace,cranelift_frontend=off,cranelift_codegen=off,wasmtime=off".into()
    });

    let _ = tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).pretty())
        .with(filter)
        .try_init();

    let cache = std::sync::Arc::new(CacheManager::from_env().await.unwrap());
    let builder = Builder::from_env(cache.clone()).await.unwrap();

    let source = ModuleSource {
        dependencies: ModuleDependencies {
            dependencies: BTreeMap::new(),
            capabilities: vec![],
        },
        source: ERROR_MODULE.to_string(),
        ident: None,
    };
    cache.remove_anon(&source.hash()).await.unwrap();

    let binary = builder
        .compile(&source)
        .await
        .expect("Module should compile");

    let tmp_dir = tempfile::tempdir().unwrap();
    let tmp_path = tmp_dir.path().to_path_buf();

    let config = PipelineConfig {
        playbook: Playbook {
            hash: binary.hash(),
            configurations: HashMap::new(),
        },
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

    // Test Success
    let input_success = PyroRow::from([("input", "hello".into())]);
    let res_success = pipeline.process(0, &input_success).await.unwrap();
    println!("{:?}", res_success);
    if let ExecutionRecord::Success { success, .. } = &res_success {
        assert_eq!(success.get_str("message").unwrap(), "Success: hello");
    } else {
        panic!("Expected Success");
    }
    let rec_success = pipeline
        .get_record(0)
        .await
        .expect("Should retrieve success record");
    assert_eq!(res_success, rec_success);

    // Test Error: return Err(pyroduct::capture!(...))
    let input_error = PyroRow::from([("input", "error".into())]);
    let res_error = pipeline.process(1, &input_error).await.unwrap();
    println!("{:?}", res_error);
    if let ExecutionRecord::Failure { failure, .. } = &res_error {
        // Following the pattern in integration_session_recovery.rs for capture!
        assert!(failure.is_ok());
        let err_msg = failure.as_ref().unwrap().to_string();
        assert!(!err_msg.contains("Remote Code Panic"));
        assert!(err_msg.contains("intentional error"));
    } else {
        panic!("Expected Failure for error");
    }
    let rec_error = pipeline
        .get_record(1)
        .await
        .expect("Should retrieve error record");
    assert_eq!(res_error, rec_error);

    // Test Panic: panic!(...)
    let input_panic = PyroRow::from([("input", "panic".into())]);
    let res_panic = pipeline.process(2, &input_panic).await.unwrap();
    println!("{:?}", res_panic);
    if let ExecutionRecord::Failure { failure, .. } = &res_panic {
        // Following the pattern in integration_session_recovery.rs for panic!
        assert!(failure.is_ok());
        let captured_err = failure.as_ref().unwrap();
        assert!(
            captured_err
                .to_string()
                .contains("Error at src/lib.rs:7:9 - intentional panic")
        );
    } else {
        panic!("Expected Failure for panic");
    }
    let rec_panic = pipeline
        .get_record(2)
        .await
        .expect("Should retrieve panic record");
    assert_eq!(res_panic, rec_panic);
}
