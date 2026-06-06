use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use pyro_artifacts::cache::CacheManager;
use pyro_daemon::playbook::PlaybooksManager;
use pyroduct::pipeline::factory::PipelineConfig;
use tempfile::tempdir;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::test]
async fn test_daemon_auto_resume() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "trace,cranelift_frontend=off,cranelift_codegen=off,wasmtime=off".into()
    });

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).pretty())
        .with(filter)
        .init();

    let test_dir = tempdir().unwrap();
    let working_dir = test_dir.path().to_path_buf();

    let cache = std::sync::Arc::new(CacheManager::from_env().await.unwrap());
    let binary = cache
        .get_named_binary("nbhdai", "integration_error", "0.1.0")
        .await
        .unwrap();

    let config_path = working_dir.join("config.toml");
    let pipeline_config = PipelineConfig {
        playbook: binary.spec.ident.clone(),
        remote: HashMap::new(),
        wal_capacity: 10,
        success_log_retention_secs: 3600,
        error_log_retention_secs: 86400 * 7,
        input_dir: working_dir.join("input"),
        output_dir: working_dir.join("output"),
        log_dir: working_dir.join("log"),
    };
    std::fs::write(
        &config_path,
        toml::to_string_pretty(&pipeline_config).unwrap(),
    )
    .unwrap();

    let pm1 = Arc::new(PlaybooksManager::new(working_dir.clone()));
    pm1.start_playbook(
        "integration_error".to_string(),
        config_path,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    assert_eq!(pm1.active_workers_count().await, 1);

    // Stop pm1 worker to release file and WAL locks
    let worker = pm1
        .workers
        .lock()
        .await
        .remove("integration_error")
        .unwrap();
    worker.shutdown().await.unwrap();

    // Give background tasks a brief moment to completely drop the database connections and locks
    tokio::time::sleep(Duration::from_millis(200)).await;

    drop(pm1);

    let pm2 = Arc::new(PlaybooksManager::new(working_dir.clone()));
    assert_eq!(pm2.active_workers_count().await, 0);

    pm2.resume_active_playbooks().await.unwrap();

    assert_eq!(pm2.active_workers_count().await, 1);
    let active = pm2.list_playbooks().await;
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].name, "integration_error");
}
