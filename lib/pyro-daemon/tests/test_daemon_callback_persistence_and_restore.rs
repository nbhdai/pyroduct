use pyro_daemon::PyroDaemon;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::test]
async fn test_daemon_callback_persistence_and_restore() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "trace,cranelift_frontend=off,cranelift_codegen=off,wasmtime=off".into()
    });

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).pretty())
        .with(filter)
        .init();

    let test_dir = tempfile::tempdir().unwrap();
    let working_dir = test_dir.path().to_path_buf();

    // 1. Spawn PyroDaemon
    let daemon = PyroDaemon::new(working_dir.clone());
    let db = daemon.playbooks_manager.db.clone();

    // 2. Add some mock callback mappings directly to the DB to verify persist/restore
    let uuid1 = uuid::Uuid::new_v4();
    let uuid2 = uuid::Uuid::new_v4();
    db.add_callback_mapping(
        uuid1,
        "test_playbook",
        "http",
        "http://127.0.0.1:9999/callback",
    )
    .await
    .unwrap();
    db.add_callback_mapping(uuid2, "test_playbook", "socket", "127.0.0.1:8888")
        .await
        .unwrap();

    // 3. Query the callback mappings to verify list_callbacks query works
    let callbacks = daemon
        .playbooks_manager
        .list_callbacks("test_playbook".to_string())
        .await
        .unwrap();
    assert_eq!(callbacks.len(), 2);
    assert_eq!(callbacks[0].uuid, uuid1);
    assert_eq!(callbacks[0].callback_type, "http");
    assert_eq!(callbacks[0].target, "http://127.0.0.1:9999/callback");
    assert_eq!(callbacks[1].uuid, uuid2);
    assert_eq!(callbacks[1].callback_type, "socket");
    assert_eq!(callbacks[1].target, "127.0.0.1:8888");

    // 4. Test deleting one callback mapping by UUID
    db.delete_callback_mapping(uuid1).await.unwrap();
    let callbacks_after_one_deleted = daemon
        .playbooks_manager
        .list_callbacks("test_playbook".to_string())
        .await
        .unwrap();
    assert_eq!(callbacks_after_one_deleted.len(), 1);
    assert_eq!(callbacks_after_one_deleted[0].uuid, uuid2);

    // 5. Test deleting playbook cleans up callback mappings
    db.delete_playbook("test_playbook").await.unwrap();
    let callbacks_after_delete = daemon
        .playbooks_manager
        .list_callbacks("test_playbook".to_string())
        .await
        .unwrap();
    assert_eq!(callbacks_after_delete.len(), 0);
}
