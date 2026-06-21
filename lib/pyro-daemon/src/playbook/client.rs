use crate::client::DaemonClient;
use crate::playbook::{PlaybookRequest, PlaybookResponse, PlaybookStatus, CallbackMapping, SessionInfo};
use crate::{DaemonRequest, DaemonResponse};
use crate::Result;
use std::path::PathBuf;
use pyroduct::Capture;

impl DaemonClient {
    pub async fn start_playbook(
        &self,
        name: String,
        pipeline_config: pyro_artifacts::artifacts::PlaybookIdent,
        playbook_socket: Option<String>,
        http_address: Option<String>,
        input_dir: Option<PathBuf>,
        output_dir: Option<PathBuf>,
        pinned_version: Option<String>,
    ) -> Result<String> {
        let req = DaemonRequest::Playbook(PlaybookRequest::Start {
            name,
            pipeline_config,
            playbook_socket,
            http_address,
            input_dir,
            output_dir,
            pinned_version,
            configurations: None,
            num_workers: None,
        });
        match self.request(req).await? {
            DaemonResponse::Playbook(PlaybookResponse::Success { message }) => Ok(message),
            DaemonResponse::Playbook(PlaybookResponse::Error { message }) => pyroduct::bail!("{}", message),
            _ => pyroduct::bail!("Unexpected response from daemon"),
        }
    }

    pub async fn stop_playbook(&self, name: String) -> Result<String> {
        let req = DaemonRequest::Playbook(PlaybookRequest::Stop { name });
        match self.request(req).await? {
            DaemonResponse::Playbook(PlaybookResponse::Success { message }) => Ok(message),
            DaemonResponse::Playbook(PlaybookResponse::Error { message }) => pyroduct::bail!("{}", message),
            _ => pyroduct::bail!("Unexpected response from daemon"),
        }
    }

    pub async fn resume_playbook(&self, name: String) -> Result<String> {
        let req = DaemonRequest::Playbook(PlaybookRequest::Resume { name });
        match self.request(req).await? {
            DaemonResponse::Playbook(PlaybookResponse::Success { message }) => Ok(message),
            DaemonResponse::Playbook(PlaybookResponse::Error { message }) => pyroduct::bail!("{}", message),
            _ => pyroduct::bail!("Unexpected response from daemon"),
        }
    }

    pub async fn delete_playbook(&self, name: String) -> Result<String> {
        let req = DaemonRequest::Playbook(PlaybookRequest::Delete { name });
        match self.request(req).await? {
            DaemonResponse::Playbook(PlaybookResponse::Success { message }) => Ok(message),
            DaemonResponse::Playbook(PlaybookResponse::Error { message }) => pyroduct::bail!("{}", message),
            _ => pyroduct::bail!("Unexpected response from daemon"),
        }
    }

    pub async fn list_playbooks(&self) -> Result<Vec<PlaybookStatus>> {
        let req = DaemonRequest::Playbook(PlaybookRequest::List);
        match self.request(req).await? {
            DaemonResponse::Playbook(PlaybookResponse::Playbooks { playbooks }) => Ok(playbooks),
            DaemonResponse::Playbook(PlaybookResponse::Error { message }) => pyroduct::bail!("{}", message),
            _ => pyroduct::bail!("Unexpected response from daemon"),
        }
    }

    pub async fn call_playbook(&self, name: String, payload: serde_json::Value) -> Result<serde_json::Value> {
        let req = DaemonRequest::Playbook(PlaybookRequest::Call { name, payload, session_id: None });
        match self.request(req).await? {
            DaemonResponse::Playbook(PlaybookResponse::CallResult { result }) => {
                match result {
                    pyroduct::pipeline::ServerExecutionRecord::Normal(pyroduct::pipeline::ExecutionRecord::Success { success, .. }) => {
                        let val = serde_json::to_value(&success).capture("Failed to serialize row")?;
                        Ok(val)
                    }
                    pyroduct::pipeline::ServerExecutionRecord::Session(pyroduct::pipeline::session::SessionExecutionRecord::Success { success, .. }) => {
                        let val = serde_json::to_value(&success).capture("Failed to serialize row")?;
                        Ok(val)
                    }
                    pyroduct::pipeline::ServerExecutionRecord::SessionDiff(pyroduct::pipeline::session_diff::SessionDiffExecutionRecord::Success { success, .. }) => {
                        let val = serde_json::to_value(&success).capture("Failed to serialize row")?;
                        Ok(val)
                    }
                    pyroduct::pipeline::ServerExecutionRecord::Normal(pyroduct::pipeline::ExecutionRecord::Failure { failure, .. }) => {
                        let msg = match failure {
                            Ok(captured) => captured.to_string(),
                            Err(s) => s,
                        };
                        pyroduct::bail!("{}", msg);
                    }
                    pyroduct::pipeline::ServerExecutionRecord::Session(pyroduct::pipeline::session::SessionExecutionRecord::Failure { failure, .. }) => {
                        let msg = match failure {
                            Ok(captured) => captured.to_string(),
                            Err(s) => s,
                        };
                        pyroduct::bail!("{}", msg);
                    }
                    pyroduct::pipeline::ServerExecutionRecord::SessionDiff(pyroduct::pipeline::session_diff::SessionDiffExecutionRecord::Failure { failure, .. }) => {
                        let msg = match failure {
                            Ok(captured) => captured.to_string(),
                            Err(s) => s,
                        };
                        pyroduct::bail!("{}", msg);
                    }
                }
            }
            DaemonResponse::Playbook(PlaybookResponse::Error { message }) => pyroduct::bail!("{}", message),
            _ => pyroduct::bail!("Unexpected response from daemon"),
        }
    }

    pub async fn call_playbook_record(&self, name: String, payload: serde_json::Value, session_id: Option<u32>) -> Result<pyroduct::pipeline::ServerExecutionRecord> {
        let req = DaemonRequest::Playbook(PlaybookRequest::Call { name, payload, session_id });
        match self.request(req).await? {
            DaemonResponse::Playbook(PlaybookResponse::CallResult { result }) => Ok(result),
            DaemonResponse::Playbook(PlaybookResponse::Error { message }) => pyroduct::bail!("{}", message),
            _ => pyroduct::bail!("Unexpected response from daemon"),
        }
    }

    pub async fn add_http_callback(&self, source: String, url: String) -> Result<String> {
        let req = DaemonRequest::Playbook(PlaybookRequest::AddHttpCallback { source, url });
        match self.request(req).await? {
            DaemonResponse::Playbook(PlaybookResponse::Success { message }) => Ok(message),
            DaemonResponse::Playbook(PlaybookResponse::Error { message }) => pyroduct::bail!("{}", message),
            _ => pyroduct::bail!("Unexpected response from daemon"),
        }
    }

    pub async fn add_socket_callback(&self, source: String, socket_path: String) -> Result<String> {
        let req = DaemonRequest::Playbook(PlaybookRequest::AddSocketCallback { source, socket_path });
        match self.request(req).await? {
            DaemonResponse::Playbook(PlaybookResponse::Success { message }) => Ok(message),
            DaemonResponse::Playbook(PlaybookResponse::Error { message }) => pyroduct::bail!("{}", message),
            _ => pyroduct::bail!("Unexpected response from daemon"),
        }
    }

    pub async fn add_playbook_callback(&self, source: String, target_playbook: String) -> Result<String> {
        let req = DaemonRequest::Playbook(PlaybookRequest::AddPlaybookCallback { source, target_playbook });
        match self.request(req).await? {
            DaemonResponse::Playbook(PlaybookResponse::Success { message }) => Ok(message),
            DaemonResponse::Playbook(PlaybookResponse::Error { message }) => pyroduct::bail!("{}", message),
            _ => pyroduct::bail!("Unexpected response from daemon"),
        }
    }

    pub async fn list_callbacks(&self, source: String) -> Result<Vec<CallbackMapping>> {
        let req = DaemonRequest::Playbook(PlaybookRequest::ListCallbacks { source });
        match self.request(req).await? {
            DaemonResponse::Playbook(PlaybookResponse::Callbacks { callbacks }) => Ok(callbacks),
            DaemonResponse::Playbook(PlaybookResponse::Error { message }) => pyroduct::bail!("{}", message),
            _ => pyroduct::bail!("Unexpected response from daemon"),
        }
    }

    pub async fn delete_callback(&self, uuid: uuid::Uuid) -> Result<String> {
        let req = DaemonRequest::Playbook(PlaybookRequest::DeleteCallback { uuid });
        match self.request(req).await? {
            DaemonResponse::Playbook(PlaybookResponse::Success { message }) => Ok(message),
            DaemonResponse::Playbook(PlaybookResponse::Error { message }) => pyroduct::bail!("{}", message),
            _ => pyroduct::bail!("Unexpected response from daemon"),
        }
    }

    pub async fn list_sessions(&self, name: String, status: Option<pyroduct::pipeline::session::SessionStatusFilter>) -> Result<Vec<SessionInfo>> {
        let req = DaemonRequest::Playbook(PlaybookRequest::ListSessions { name, status });
        match self.request(req).await? {
            DaemonResponse::Playbook(PlaybookResponse::Sessions { sessions }) => Ok(sessions),
            DaemonResponse::Playbook(PlaybookResponse::Error { message }) => pyroduct::bail!("{}", message),
            _ => pyroduct::bail!("Unexpected response from daemon"),
        }
    }
}
