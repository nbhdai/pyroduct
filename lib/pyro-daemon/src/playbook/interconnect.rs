use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

use pyro_artifacts::artifacts::PlaybookSpec;
use pyro_spec::ModuleFunc;
use pyroduct::CapturedError;
use pyroduct::format::PyroRow;
use pyroduct::module::interconnect::PlaybookInterconnect;
use pyroduct::pipeline::PipelineServer;

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

impl PlaybooksManager {
    pub fn build_interconnect(
        self: &Arc<Self>,
        main_name: &str,
        spec: &PlaybookSpec,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = crate::Result<Arc<dyn PlaybookInterconnect>>> + Send>,
    > {
        let this = self.clone();
        let main_name = main_name.to_string();
        let spec = spec.clone();
        Box::pin(async move {
            tracing::debug!(interconnect = ?spec.interconnect, "build_interconnect: starting");
            let mut playbooks = HashMap::new();
            let mut servers = HashMap::new();

            for (name, expected_ident) in &spec.interconnect {
                tracing::debug!(name = %name, expected_ident = ?expected_ident, "build_interconnect: looking up interconnect playbook");
                let mut found_info = None;
                {
                    let workers = this.workers.lock().await;
                    for worker in workers.values() {
                        tracing::debug!(worker_ident = ?worker.server.spec().ident, "build_interconnect: checking active worker");
                        if &worker.server.spec().ident == expected_ident {
                            found_info =
                                Some((worker.server.spec().func.clone(), worker.server.clone()));
                            break;
                        }
                    }
                }

                let (func, server) = match found_info {
                    Some(info) => info,
                    None => {
                        let sub_name = format!("{}_{}", main_name, expected_ident.package);
                        tracing::info!(%sub_name, ?expected_ident, "build_interconnect: interconnect playbook not running, launching it");
                        this.start_playbook(
                            sub_name.clone(),
                            expected_ident.clone(),
                            None,
                            None,
                            None,
                        )
                        .await?;

                        let workers = this.workers.lock().await;
                        workers
                            .get(&sub_name)
                            .map(|w| (w.server.spec().func.clone(), w.server.clone()))
                            .ok_or_else(|| {
                                pyroduct::CapturedError::new(format!(
                                    "Failed to retrieve launched interconnect playbook '{}'",
                                    sub_name
                                ))
                            })?
                    }
                };

                playbooks.insert(name.clone(), func);
                servers.insert(name.clone(), server);
            }

            let interconnect: Arc<dyn PlaybookInterconnect> =
                Arc::new(DaemonInterconnect { playbooks, servers });
            Ok(interconnect)
        })
    }
}
