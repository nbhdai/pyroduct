use pyro_artifacts::{
    artifacts::{ModuleDependencies, ModuleSource, Playbook},
    build::Builder,
    cache::CacheManager,
    cargo::ResolvedCapability,
};
use pyroduct::{
    PyroRow,
    pipeline::{ExecutionRecord, PipelineConfig},
};
use std::collections::{BTreeMap, HashMap};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

const CODE: &'static str = r#"
//! Test module 1: Uses test_cap1 counter capability
//!
//! Simple module that increments a counter and returns the result.

use state::{CounterClient, CounterClientMethods};

#[pyroduct::module(output = (count, incremented))]
pub fn call(input: &str) -> Result<(u64, u64)> {
    let start: u64 = input.parse().map_err(|e| format!("Parse error: {}", e))?;

    let client = CounterClient { start_value: start }.register()?;

    let count = client.get_count()?;
    let incremented = client.increment()?;

    Ok((count, incremented))
}
"#;

#[tokio::test]
async fn test_pipeline_get_record() {
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
            capabilities: vec![ResolvedCapability {
                package: "state".to_string(),
                author: "nbhdai".to_string(),
                version: "0.1.0".to_string(),
            }],
        },
        source: CODE.to_string(),
        ident: None,
    };
    cache.remove_anon(&source.hash()).await.unwrap();

    let binary = builder
        .compile(&source)
        .await
        .expect("Valid module should compile");

    let tmp_dir = tempfile::tempdir().unwrap();
    let tmp_path = tmp_dir.path().to_path_buf();

    let config = PipelineConfig {
        playbook: Playbook {
            hash: binary.hash(),
            configurations: HashMap::from([("state".to_string(), None)]),
        },
        wal_capacity: 5,
        success_log_retention_secs: 3600,
        error_log_retention_secs: 86400 * 7,
        input_dir: tmp_path.clone(),
        output_dir: tmp_path.clone(),
        log_dir: tmp_path.clone(),
    };

    let config = config.load(&cache).await.unwrap();
    let factory = config.factory().unwrap();
    let mut pipeline = factory.build().await.unwrap();

    // 1. Prepare and push input records to input_manager
    let input_0 = PyroRow::from([("input", "10".into())]);
    let input_1 = PyroRow::from([("input", "invalid".into())]); // Will cause a failure
    let input_2 = PyroRow::from([("input", "20".into())]);

    pipeline.input_manager.push_record(&input_0).unwrap();
    pipeline.input_manager.push_record(&input_1).unwrap();
    pipeline.input_manager.push_record(&input_2).unwrap();

    // 2. Process records through the pipeline
    let proc_0 = pipeline.process(0, &input_0).await.unwrap();
    assert!(matches!(proc_0, ExecutionRecord::Success { .. }));

    let proc_1 = pipeline.process(1, &input_1).await.unwrap();
    assert!(matches!(proc_1, ExecutionRecord::Failure { .. }));

    let proc_2 = pipeline.process(2, &input_2).await.unwrap();
    assert!(matches!(proc_2, ExecutionRecord::Success { .. }));

    // Flush to ensure all logs and records are written out if buffered
    pipeline.log_manager.flush().await.unwrap();

    // 3. Test get_record with log WAL active
    let rec_0 = pipeline.get_record(0).await.unwrap();
    match &rec_0 {
        ExecutionRecord::Success {
            row_index,
            input,
            success,
            logs,
        } => {
            assert_eq!(*row_index, 0);
            assert_eq!(input.get_str("input").unwrap(), "10");
            assert_eq!(success.get_u64("count").unwrap(), 0);
            assert_eq!(success.get_u64("incremented").unwrap(), 10);
            // Verify capability logs are present or module logs are not empty
            assert!(!logs.capability_logs.is_empty() || !logs.module_logs.is_empty());
        }
        _ => panic!("Expected Success for index 0"),
    }

    let rec_1 = pipeline.get_record(1).await.unwrap();
    match &rec_1 {
        ExecutionRecord::Failure {
            row_index,
            input,
            failure,
            ..
        } => {
            assert_eq!(*row_index, 1);
            assert_eq!(input.get_str("input").unwrap(), "invalid");
            assert!(failure.is_ok()); // captured module panic/error
        }
        _ => panic!("Expected Failure for index 1"),
    }

    let rec_2 = pipeline.get_record(2).await.unwrap();
    match &rec_2 {
        ExecutionRecord::Success {
            row_index,
            input,
            success,
            ..
        } => {
            assert_eq!(*row_index, 2);
            assert_eq!(input.get_str("input").unwrap(), "20");
            assert_eq!(success.get_u64("count").unwrap(), 1);
            assert_eq!(success.get_u64("incremented").unwrap(), 21);
        }
        _ => panic!("Expected Success for index 2"),
    }
    
    tracing::info!("Clearing log");

    // 4. Clean/delete all log WAL files (.pyrolog) to simulate rotation/cleanup
    for entry in std::fs::read_dir(&tmp_path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "pyrolog") {
            std::fs::remove_file(path).unwrap();
        }
    }

    tracing::info!("Getting 0");

    // 5. Test get_record with log WAL missing/cleaned (should return blank logs)
    let rec_0_clean = pipeline.get_record(0).await.unwrap();
    match &rec_0_clean {
        ExecutionRecord::Success {
            row_index,
            input,
            success,
            logs,
        } => {
            assert_eq!(*row_index, 0);
            assert_eq!(input.get_str("input").unwrap(), "10");
            assert_eq!(success.get_u64("count").unwrap(), 0);
            assert!(logs.module_logs.is_empty());
            assert!(logs.capability_logs.is_empty());
        }
        _ => panic!("Expected Success with blank logs for index 0"),
    }

    tracing::info!("Getting 1");

    let rec_1_clean = pipeline.get_record(1).await.unwrap();
    match &rec_1_clean {
        ExecutionRecord::Failure {
            row_index,
            input,
            logs,
            ..
        } => {
            assert_eq!(*row_index, 1);
            assert_eq!(input.get_str("input").unwrap(), "invalid");
            assert!(logs.module_logs.is_empty());
            assert!(logs.capability_logs.is_empty());
        }
        _ => panic!("Expected Failure with blank logs for index 1"),
    }

    let rec_2_clean = pipeline.get_record(2).await.unwrap();
    match &rec_2_clean {
        ExecutionRecord::Success {
            row_index,
            input,
            success,
            logs,
        } => {
            assert_eq!(*row_index, 2);
            assert_eq!(input.get_str("input").unwrap(), "20");
            assert_eq!(success.get_u64("count").unwrap(), 1);
            assert!(logs.module_logs.is_empty());
            assert!(logs.capability_logs.is_empty());
        }
        _ => panic!("Expected Success with blank logs for index 2"),
    }
}
