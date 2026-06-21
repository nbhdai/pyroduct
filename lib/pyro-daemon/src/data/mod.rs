use crate::playbook::PlaybooksManager;
use pyroduct::PyroRow;
use pyroduct::transport::socket::PyroSocket;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod client;
pub mod historic;
pub mod replay;
pub mod sql;
pub mod stream;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum DataRequest {
    GetBaseDir,
    QueryPlaybook {
        playbook_name: String,
        sql_query: String,
    },
    StreamPlaybook {
        playbook_name: String,
    },
    GetPlaybookData {
        playbook_name: String,
        offset: usize,
        limit: usize,
    },
    GetPlaybookFailures {
        playbook_name: String,
    },
    GetPlaybookExecutionRecord {
        playbook_name: String,
        id: u32,
    },
    StartReplay {
        playbook_name: String,
        folder_path: String,
        interval_ms: u64,
        wiggle_ms: u64,
    },
    StartParallelReplay {
        playbook_name: String,
        folder_path: String,
        concurrency: usize,
    },
    GetReplayStatus {
        playbook_name: String,
    },
    StopReplay {
        playbook_name: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum DataResponse {
    BaseDir { path: String },
    QueryResult { ipc_bytes: Vec<u8> },
    StreamStarted,
    StreamRow { row: PyroRow<'static> },
    StreamFinished,
    PlaybookData { ipc_bytes: Vec<u8> },
    PlaybookFailures { failures: Vec<pyroduct::pipeline::ServerExecutionRecord> },
    PlaybookExecutionRecord { record: pyroduct::pipeline::ServerExecutionRecord },
    ReplayStarted { total_rows: usize },
    ReplayStatus {
        running: bool,
        total_rows: usize,
        rows_completed: usize,
        successes: usize,
        errors: usize,
        current_file: String,
    },
    ReplayStopped,
    Error { message: String },
}

#[derive(Clone)]
pub struct DaemonDataManager {
    base_dir: PathBuf,
    playbooks_manager: Arc<PlaybooksManager>,
    pub(crate) replays: Arc<Mutex<HashMap<String, replay::ReplayHandle>>>,
}

impl DaemonDataManager {
    pub fn new(base_dir: PathBuf, playbooks_manager: Arc<PlaybooksManager>) -> Self {
        Self {
            base_dir,
            playbooks_manager,
            replays: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn handle_request(
        &self,
        req: DataRequest,
        socket: &PyroSocket,
        mux_id: Option<u32>,
    ) -> DataResponse {
        match req {
            DataRequest::GetBaseDir => DataResponse::BaseDir {
                path: self.base_dir().to_string_lossy().to_string(),
            },
            DataRequest::QueryPlaybook {
                playbook_name,
                sql_query,
            } => match self.query_playbook_data(&playbook_name, &sql_query).await {
                Ok(ipc_bytes) => DataResponse::QueryResult { ipc_bytes },
                Err(e) => DataResponse::Error {
                    message: format!("SQL query failed: {:?}", e),
                },
            },
            DataRequest::StreamPlaybook { playbook_name } => {
                match self
                    .stream_playbook_data(&playbook_name, socket, mux_id)
                    .await
                {
                    Ok(()) => DataResponse::StreamFinished,
                    Err(e) => DataResponse::Error {
                        message: format!("Streaming failed: {:?}", e),
                    },
                }
            }
            DataRequest::GetPlaybookData {
                playbook_name,
                offset,
                limit,
            } => match self.get_playbook_data(&playbook_name, offset, limit).await {
                Ok(ipc_bytes) => DataResponse::PlaybookData { ipc_bytes },
                Err(e) => DataResponse::Error {
                    message: format!("Failed to get playbook data: {:?}", e),
                },
            },
            DataRequest::GetPlaybookFailures { playbook_name } => {
                match self.get_playbook_failures(&playbook_name).await {
                    Ok(failures) => DataResponse::PlaybookFailures { failures },
                    Err(e) => DataResponse::Error {
                        message: format!("Failed to get playbook failures: {:?}", e),
                    },
                }
            }
            DataRequest::GetPlaybookExecutionRecord { playbook_name, id } => {
                match self.get_playbook_execution_record(&playbook_name, id).await {
                    Ok(record) => DataResponse::PlaybookExecutionRecord { record },
                    Err(e) => DataResponse::Error {
                        message: format!("Failed to get playbook execution record: {:?}", e),
                    },
                }
            }
            DataRequest::StartReplay {
                playbook_name,
                folder_path,
                interval_ms,
                wiggle_ms,
            } => match self
                .start_replay(&playbook_name, &folder_path, interval_ms, wiggle_ms)
                .await
            {
                Ok(total_rows) => DataResponse::ReplayStarted { total_rows },
                Err(e) => DataResponse::Error {
                    message: format!("Failed to start replay: {:?}", e),
                },
            },
            DataRequest::StartParallelReplay {
                playbook_name,
                folder_path,
                concurrency,
            } => match self
                .start_parallel_replay(&playbook_name, &folder_path, concurrency)
                .await
            {
                Ok(total_rows) => DataResponse::ReplayStarted { total_rows },
                Err(e) => DataResponse::Error {
                    message: format!("Failed to start parallel replay: {:?}", e),
                },
            },
            DataRequest::GetReplayStatus { playbook_name } => {
                match self.get_replay_status(&playbook_name).await {
                    Some(status) => DataResponse::ReplayStatus {
                        running: status.running,
                        total_rows: status.total_rows,
                        rows_completed: status.rows_completed,
                        successes: status.successes,
                        errors: status.errors,
                        current_file: status.current_file,
                    },
                    None => DataResponse::ReplayStatus {
                        running: false,
                        total_rows: 0,
                        rows_completed: 0,
                        successes: 0,
                        errors: 0,
                        current_file: String::new(),
                    },
                }
            }
            DataRequest::StopReplay { playbook_name } => {
                self.stop_replay(&playbook_name).await;
                DataResponse::ReplayStopped
            }
        }
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }
}
