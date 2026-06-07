use crate::CapturedError;
use crate::format::PyroRow;
use crate::module::interconnect::PlaybookInterconnect;
use crate::pipeline::PipelineServer;
use pyro_spec::ModuleFunc;
use std::collections::HashMap;

/// A collection of playbooks acting as an interconnect.
pub struct PlaybookCollection {
    pub playbooks: HashMap<String, ModuleFunc<'static>>,
    pub servers: HashMap<String, PipelineServer>,
}

#[async_trait::async_trait]
impl PlaybookInterconnect for PlaybookCollection {
    fn playbooks(&self) -> &HashMap<String, ModuleFunc<'static>> {
        &self.playbooks
    }

    async fn call(
        &self,
        name: &str,
        row: PyroRow<'_>,
    ) -> Result<(u32, PyroRow<'static>), CapturedError> {
        if let Some(server) = self.servers.get(name) {
            server.call(row).await.and_then(|rec| rec.into_result())
        } else {
            Err(CapturedError::new(format!(
                "Playbook '{}' not found in interconnect collection",
                name
            )))
        }
    }

    async fn call_session(
        &self,
        name: &str,
        client_id: u32,
        row: PyroRow<'_>,
    ) -> Result<PyroRow<'static>, CapturedError> {
        if let Some(server) = self.servers.get(name) {
            server
                .call_session(client_id, row)
                .await
                .and_then(|rec| rec.into_result().map(|(_, r)| r))
        } else {
            Err(CapturedError::new(format!(
                "Playbook '{}' not found in interconnect collection",
                name
            )))
        }
    }
}
