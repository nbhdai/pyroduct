use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::format::{PyroFailure, PyroSuccess, value::PyroRow};
use crate::module::SessionResult;
use crate::pipeline::{Pipeline, session::SessionPipeline, session_diff::SessionDiffPipeline};

/// A runtime repository that holds active running `Pipeline`s, `SessionPipeline`s,
/// and `SessionDiffPipeline`s, providing thread-safe execution and WAL/DataManager access.
#[derive(Clone)]
pub struct PlaybookRepository {
    pipelines: Arc<RwLock<HashMap<String, Pipeline>>>,
    session_pipelines: Arc<RwLock<HashMap<String, SessionPipeline>>>,
    session_diff_pipelines: Arc<RwLock<HashMap<String, SessionDiffPipeline>>>,
}

impl PlaybookRepository {
    /// Create a new, empty PlaybookRepository.
    pub fn new() -> Self {
        Self {
            pipelines: Arc::new(RwLock::new(HashMap::new())),
            session_pipelines: Arc::new(RwLock::new(HashMap::new())),
            session_diff_pipelines: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Insertion API
    // ─────────────────────────────────────────────────────────────────────────

    /// Insert an active `Pipeline` into the repository.
    pub async fn insert_pipeline(&self, key: String, pipeline: Pipeline) {
        let mut guard = self.pipelines.write().await;
        guard.insert(key, pipeline);
    }

    /// Insert an active `SessionPipeline` into the repository.
    pub async fn insert_session_pipeline(&self, key: String, pipeline: SessionPipeline) {
        let mut guard = self.session_pipelines.write().await;
        guard.insert(key, pipeline);
    }

    /// Insert an active `SessionDiffPipeline` into the repository.
    pub async fn insert_session_diff_pipeline(&self, key: String, pipeline: SessionDiffPipeline) {
        let mut guard = self.session_diff_pipelines.write().await;
        guard.insert(key, pipeline);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Execution API
    // ─────────────────────────────────────────────────────────────────────────

    /// Call a normal `Pipeline` by key, running it over the input row.
    pub async fn call_pipeline(
        &self,
        key: &str,
        input: &PyroRow<'_>,
    ) -> Result<Option<PyroSuccess>, PyroFailure> {
        let mut guard = self.pipelines.write().await;
        if let Some(pipeline) = guard.get_mut(key) {
            let record = pipeline.call(input).await?;
            Ok(Some(record))
        } else {
            Ok(None)
        }
    }

    /// Call a `SessionPipeline` by key, running it over the input row in a given session.
    pub async fn call_session_pipeline(
        &self,
        key: &str,
        session_id: u32,
        input: &PyroRow<'_>,
    ) -> Result<Option<SessionResult>, PyroFailure> {
        let mut guard = self.session_pipelines.write().await;
        if let Some(pipeline) = guard.get_mut(key) {
            let record = pipeline.call(session_id, input).await?;
            Ok(Some(record))
        } else {
            Ok(None)
        }
    }

    /// Call a `SessionDiffPipeline` by key, running it over the input row in a given session.
    pub async fn call_session_diff_pipeline(
        &self,
        key: &str,
        session_id: u32,
        input: &PyroRow<'_>,
    ) -> Result<Option<SessionResult>, PyroFailure> {
        let mut guard = self.session_diff_pipelines.write().await;
        if let Some(pipeline) = guard.get_mut(key) {
            let record = pipeline.call(session_id, input).await?;
            Ok(Some(record))
        } else {
            Ok(None)
        }
    }

    /// Get the next available session ID for a `SessionPipeline` by key.
    pub async fn next_session_id(&self, key: &str) -> Option<u32> {
        let guard = self.session_pipelines.read().await;
        guard.get(key).map(|p| p.next_session_id())
    }

    /// Get the next available session ID for a `SessionDiffPipeline` by key.
    pub async fn next_session_diff_id(&self, key: &str) -> Option<u32> {
        let guard = self.session_diff_pipelines.read().await;
        guard.get(key).map(|p| p.next_session_id())
    }

    /// Prepare/register a session for a `SessionPipeline` by key.
    pub async fn prep_session(
        &self,
        key: &str,
        session_id: u32,
        prior: &[PyroRow<'_>],
    ) -> Result<(), PyroFailure> {
        let mut guard = self.session_pipelines.write().await;
        if let Some(pipeline) = guard.get_mut(key) {
            pipeline.prep_session(session_id, prior).await
        } else {
            Err(PyroFailure {
                result: Err(format!("Session pipeline not found for key: {}", key)),
                logs: crate::format::PyroLogs::empty(),
            })
        }
    }

    /// Prepare/register a session for a `SessionDiffPipeline` by key.
    pub async fn prep_session_diff(
        &self,
        key: &str,
        session_id: u32,
        inputs: &[PyroRow<'_>],
        outputs: &[PyroRow<'_>],
    ) -> Result<(), PyroFailure> {
        let mut guard = self.session_diff_pipelines.write().await;
        if let Some(pipeline) = guard.get_mut(key) {
            pipeline.prep_session(session_id, inputs, outputs).await
        } else {
            Err(PyroFailure {
                result: Err(format!("Session diff pipeline not found for key: {}", key)),
                logs: crate::format::PyroLogs::empty(),
            })
        }
    }

    /// Close a session for a `SessionPipeline` by key.
    pub async fn close_session(&self, key: &str, session_id: u32) -> Result<(), PyroFailure> {
        let mut guard = self.session_pipelines.write().await;
        if let Some(pipeline) = guard.get_mut(key) {
            pipeline.close_session(session_id).await
        } else {
            Err(PyroFailure {
                result: Err(format!("Session pipeline not found for key: {}", key)),
                logs: crate::format::PyroLogs::empty(),
            })
        }
    }

    /// Close a session for a `SessionDiffPipeline` by key.
    pub async fn close_session_diff(&self, key: &str, session_id: u32) -> Result<(), PyroFailure> {
        let mut guard = self.session_diff_pipelines.write().await;
        if let Some(pipeline) = guard.get_mut(key) {
            pipeline.close_session(session_id).await
        } else {
            Err(PyroFailure {
                result: Err(format!("Session diff pipeline not found for key: {}", key)),
                logs: crate::format::PyroLogs::empty(),
            })
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // WAL / DataManager Access API (Closure-based for borrowing safety)
    // ─────────────────────────────────────────────────────────────────────────

    /// Run a closure with access to the pipeline's WAL and DataManagers under lock.
    pub async fn with_pipeline_wals<R, F>(&self, key: &str, f: F) -> Option<R>
    where
        F: FnOnce(
            &crate::format::log_wal::LogWal,
            &crate::pipeline::data::DataManager,
            &crate::pipeline::data::DataManager,
        ) -> R,
    {
        let guard = self.pipelines.read().await;
        guard
            .get(key)
            .map(|p| f(&p.log_manager, &p.input_manager, &p.output_manager))
    }

    /// Run a closure with mutable access to the pipeline's WAL and DataManagers under lock.
    pub async fn with_pipeline_wals_mut<R, F>(&self, key: &str, f: F) -> Option<R>
    where
        F: FnOnce(
            &mut crate::format::log_wal::LogWal,
            &mut crate::pipeline::data::DataManager,
            &mut crate::pipeline::data::DataManager,
        ) -> R,
    {
        let mut guard = self.pipelines.write().await;
        guard.get_mut(key).map(|p| {
            f(
                &mut p.log_manager,
                &mut p.input_manager,
                &mut p.output_manager,
            )
        })
    }

    /// Run a closure with access to the session pipeline's WALs and DataManager under lock.
    pub async fn with_session_pipeline_wals<R, F>(&self, key: &str, f: F) -> Option<R>
    where
        F: FnOnce(
            &crate::format::log_wal::LogWal,
            &crate::pipeline::data::DataManager,
            &HashMap<u32, crate::pipeline::session::ActiveSession>,
        ) -> R,
    {
        let guard = self.session_pipelines.read().await;
        guard
            .get(key)
            .map(|p| f(&p.log_manager, &p.output_manager, &p.active_sessions))
    }

    /// Run a closure with mutable access to the session pipeline's WALs and DataManager under lock.
    pub async fn with_session_pipeline_wals_mut<R, F>(&self, key: &str, f: F) -> Option<R>
    where
        F: FnOnce(
            &mut crate::format::log_wal::LogWal,
            &mut crate::pipeline::data::DataManager,
            &mut HashMap<u32, crate::pipeline::session::ActiveSession>,
        ) -> R,
    {
        let mut guard = self.session_pipelines.write().await;
        guard.get_mut(key).map(|p| {
            f(
                &mut p.log_manager,
                &mut p.output_manager,
                &mut p.active_sessions,
            )
        })
    }

    /// Run a closure with access to the session diff pipeline's WALs and DataManager under lock.
    pub async fn with_session_diff_pipeline_wals<R, F>(&self, key: &str, f: F) -> Option<R>
    where
        F: FnOnce(
            &crate::format::log_wal::LogWal,
            &crate::pipeline::data::DataManager,
            &HashMap<u32, crate::pipeline::session_diff::ActiveSession>,
        ) -> R,
    {
        let guard = self.session_diff_pipelines.read().await;
        guard
            .get(key)
            .map(|p| f(&p.log_manager, &p.output_manager, &p.active_sessions))
    }

    /// Run a closure with mutable access to the session diff pipeline's WALs and DataManager under lock.
    pub async fn with_session_diff_pipeline_wals_mut<R, F>(&self, key: &str, f: F) -> Option<R>
    where
        F: FnOnce(
            &mut crate::format::log_wal::LogWal,
            &mut crate::pipeline::data::DataManager,
            &mut HashMap<u32, crate::pipeline::session_diff::ActiveSession>,
        ) -> R,
    {
        let mut guard = self.session_diff_pipelines.write().await;
        guard.get_mut(key).map(|p| {
            f(
                &mut p.log_manager,
                &mut p.output_manager,
                &mut p.active_sessions,
            )
        })
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Deletion API
    // ─────────────────────────────────────────────────────────────────────────

    /// Remove a pipeline from the repository by key.
    pub async fn remove_pipeline(&self, key: &str) -> bool {
        let mut guard = self.pipelines.write().await;
        guard.remove(key).is_some()
    }

    /// Remove a session pipeline from the repository by key.
    pub async fn remove_session_pipeline(&self, key: &str) -> bool {
        let mut guard = self.session_pipelines.write().await;
        guard.remove(key).is_some()
    }

    /// Remove a session diff pipeline from the repository by key.
    pub async fn remove_session_diff_pipeline(&self, key: &str) -> bool {
        let mut guard = self.session_diff_pipelines.write().await;
        guard.remove(key).is_some()
    }
}

impl Default for PlaybookRepository {
    fn default() -> Self {
        Self::new()
    }
}
