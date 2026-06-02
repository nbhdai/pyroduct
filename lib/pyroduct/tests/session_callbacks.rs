use pyro_artifacts::{
    build::{AnonPlaybook, Builder},
    cache::CacheManager,
};
use pyroduct::module::sessions::SessionResult;
use pyroduct::{PyroRow, pipeline::PipelineConfig};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

const SIMPLE_SESSION_MODULE: &str = r#"
use pyroduct::session::SessionResponse;

#[pyroduct::module(session, output = message)]
fn counter(
    prior: Vec<String>,
    input: String,
) -> Result<SessionResponse<String>> {
    let turn = (prior.len() as u32 + 1) / 2;
    match turn {
        0 => Ok(SessionResponse::Continue(format!("Hello! Turn {}", turn + 1))),
        1 => Ok(SessionResponse::End(format!("Goodbye! Turn {}", turn + 1))),
        _ => Ok(SessionResponse::Terminate),
    }
}
"#;

static CALLBACK_COUNTER: AtomicUsize = AtomicUsize::new(0);

async fn my_session_callback(session_id: usize, row: &PyroRow<'_>) {
    assert_eq!(session_id, 42);
    // In SessionPipeline, rolled_up_row has a "session" field which is a List
    assert!(row.get("session").is_some());
    CALLBACK_COUNTER.fetch_add(1, Ordering::SeqCst);
}

static DIFF_CALLBACK_COUNTER: AtomicUsize = AtomicUsize::new(0);

async fn my_session_diff_callback(session_id: usize, row: &PyroRow<'_>) {
    assert_eq!(session_id, 42);
    // In SessionDiffPipeline, rolled_up_row has "inputs" and "outputs" fields
    assert!(row.get("inputs").is_some());
    assert!(row.get("outputs").is_some());
    DIFF_CALLBACK_COUNTER.fetch_add(1, Ordering::SeqCst);
}

#[tokio::test]
async fn test_session_callbacks() {
    let cache = std::sync::Arc::new(CacheManager::from_env().await.unwrap());
    let builder = Builder::from_env(cache.clone()).await.unwrap();

    let source = AnonPlaybook {
        package: "test_session_cb".to_string(),
        dependencies: std::collections::BTreeMap::new(),
        configurations: Vec::new(),
        source: SIMPLE_SESSION_MODULE.to_string(),
    };
    let _ = cache
        .remove_module("anon", "test_session_cb", "0.1.0")
        .await;

    let binary = builder
        .compile_anon(&source)
        .await
        .expect("Valid session module should compile");

    let tmp_dir = tempfile::tempdir().unwrap();
    let tmp_path = tmp_dir.path().to_path_buf();

    let ident = &binary.spec.ident;

    let config = PipelineConfig {
        playbook: ident.clone(),
        remote: HashMap::new(),
        wal_capacity: 2,
        success_log_retention_secs: 3600,
        error_log_retention_secs: 86400 * 7,
        input_dir: tmp_path.clone(),
        output_dir: tmp_path.clone(),
        log_dir: tmp_path.clone(),
    };

    let loaded = config.load(&cache).await.unwrap();
    let pipeline_factory = loaded.factory().unwrap();

    // 1. Test SessionPipeline Callbacks
    {
        let mut pipeline = pipeline_factory.build_session().await.unwrap();
        pipeline.callbacks.push((
            uuid::Uuid::new_v4(),
            pyroduct::pipeline::Callback::function(|idx, row| {
                let row_static = row.to_static();
                Box::pin(async move {
                    my_session_callback(idx, &row_static).await;
                })
            }),
        ));

        CALLBACK_COUNTER.store(0, Ordering::SeqCst);

        let session_id = 42;
        pipeline
            .prep_session(session_id, &[])
            .await
            .expect("Should prep session");

        // Turn 1: Continue (does not finish the session)
        let turn1_input = PyroRow::from([("input", "Hello!".into())]);
        let result1 = pipeline
            .call(session_id, &turn1_input)
            .await
            .expect("Session call turn 1 should succeed");
        assert!(matches!(result1, SessionResult::Continue { .. }));
        assert_eq!(CALLBACK_COUNTER.load(Ordering::SeqCst), 0);

        // Turn 2: End (finishes the session and does rollup)
        let turn2_input = PyroRow::from([("input", "How are you?".into())]);
        let result2 = pipeline
            .call(session_id, &turn2_input)
            .await
            .expect("Session call turn 2 should succeed");
        assert!(matches!(result2, SessionResult::End { .. }));

        // Callback should have been called exactly once
        assert_eq!(CALLBACK_COUNTER.load(Ordering::SeqCst), 1);
    }

    // 2. Test SessionDiffPipeline Callbacks
    {
        let mut pipeline = pipeline_factory.build_session_diff().await.unwrap();
        pipeline.callbacks.push((
            uuid::Uuid::new_v4(),
            pyroduct::pipeline::Callback::function(|idx, row| {
                let row_static = row.to_static();
                Box::pin(async move {
                    my_session_diff_callback(idx, &row_static).await;
                })
            }),
        ));

        DIFF_CALLBACK_COUNTER.store(0, Ordering::SeqCst);

        let session_id = 42;
        pipeline
            .prep_session(session_id, &[], &[])
            .await
            .expect("Should prep session diff");

        // Turn 1: Continue
        let turn1_input = PyroRow::from([("input", "Hello!".into())]);
        let result1 = pipeline
            .call(session_id, &turn1_input)
            .await
            .expect("Session diff call turn 1 should succeed");
        assert!(matches!(result1, SessionResult::Continue { .. }));
        assert_eq!(DIFF_CALLBACK_COUNTER.load(Ordering::SeqCst), 0);

        // Turn 2: End
        let turn2_input = PyroRow::from([("input", "How are you?".into())]);
        let result2 = pipeline
            .call(session_id, &turn2_input)
            .await
            .expect("Session diff call turn 2 should succeed");
        assert!(matches!(result2, SessionResult::End { .. }));

        // Callback should have been called exactly once
        assert_eq!(DIFF_CALLBACK_COUNTER.load(Ordering::SeqCst), 1);
    }
}
