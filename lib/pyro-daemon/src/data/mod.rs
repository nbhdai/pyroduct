use crate::playbook::PlaybooksManager;
use pyroduct::PyroRow;
use pyroduct::transport::socket::PyroSocket;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub mod client;
pub mod sql;
pub mod stream;
pub mod historic;

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
    Error { message: String },
}

#[derive(Clone)]
pub struct DaemonDataManager {
    base_dir: PathBuf,
    playbooks_manager: Arc<PlaybooksManager>,
}

impl DaemonDataManager {
    pub fn new(base_dir: PathBuf, playbooks_manager: Arc<PlaybooksManager>) -> Self {
        Self {
            base_dir,
            playbooks_manager,
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
        }
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }
}
