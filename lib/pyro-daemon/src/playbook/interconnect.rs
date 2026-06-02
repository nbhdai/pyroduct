use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;

use pyro_artifacts::artifacts::PlaybookSpec;
use pyroduct::CapturedError;
use pyroduct::format::PyroRow;
use pyroduct::module::interconnect::PlaybookInterconnect;
use pyroduct::pipeline::PipelineServer;
use pyro_spec::ModuleFunc;

use super::PlaybooksManager;

pub struct DaemonInterconnect {
    pub playbooks: HashMap<String, ModuleFunc<'static>>,
    pub servers: HashMap<String, PipelineServer>,
}

#[async_trait]
impl PlaybookInterconnect for DaemonInterconnect {
    fn playbooks(&self) -> &HashMap<String, ModuleFunc<'static>> {
        &self.playbooks
    }

    async fn call(
        &self,
        name: &str,
        row: PyroRow<'_>,
    ) -> Result<(u32, PyroRow<'static>), CapturedError> {
        if let Some(server) = self.servers.get(name) {
            server.call(row).await
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
            server.call_session(client_id, row).await
        } else {
            Err(CapturedError::new(format!(
                "Playbook '{}' not found in interconnect collection",
                name
            )))
        }
    }
}

impl PlaybooksManager {
    pub async fn build_interconnect(
        &self,
        spec: &PlaybookSpec,
    ) -> crate::Result<Arc<dyn PlaybookInterconnect>> {
        tracing::debug!(interconnect = ?spec.interconnect, "build_interconnect: starting");
        let workers = self.workers.lock().await;
        let mut playbooks = HashMap::new();
        let mut servers = HashMap::new();

        for (name, expected_ident) in &spec.interconnect {
            tracing::debug!(name = %name, expected_ident = ?expected_ident, "build_interconnect: looking up interconnect playbook");
            let mut found_worker = None;
            for (wkey, worker) in &*workers {
                tracing::debug!(worker_key = %wkey, worker_ident = ?worker.server.spec().ident, "build_interconnect: checking active worker");
                if &worker.server.spec().ident == expected_ident {
                    found_worker = Some(worker);
                    break;
                }
            }

            if let Some(worker) = found_worker {
                tracing::debug!(name = %name, "build_interconnect: found matching worker for interconnect");
                playbooks.insert(name.clone(), worker.server.spec().func.clone());
                servers.insert(name.clone(), worker.server.clone());
            } else {
                return Err(pyroduct::CapturedError::new(format!(
                    "Required interconnect playbook '{}' ({}:{}:{}) is not running",
                    name, expected_ident.author, expected_ident.package, expected_ident.version
                )));
            }
        }

        Ok(Arc::new(DaemonInterconnect { playbooks, servers }))
    }
}
