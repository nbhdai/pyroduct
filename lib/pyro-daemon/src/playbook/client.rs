use crate::client::DaemonClient;
use crate::playbook::{PlaybookRequest, PlaybookResponse, PlaybookStatus, CallbackMapping};
use crate::{DaemonRequest, DaemonResponse};
use crate::Result;
use std::path::PathBuf;

impl DaemonClient {
    pub async fn start_playbook(
        &self,
        name: String,
        playbook_config_path: PathBuf,
        playbook_socket: Option<String>,
        input_dir: Option<PathBuf>,
        output_dir: Option<PathBuf>,
    ) -> Result<String> {
        let req = DaemonRequest::Playbook(PlaybookRequest::Start {
            name,
            playbook_config_path,
            playbook_socket,
            input_dir,
            output_dir,
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
        let req = DaemonRequest::Playbook(PlaybookRequest::Call { name, payload });
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
}
