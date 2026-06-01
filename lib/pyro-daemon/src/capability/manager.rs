use crate::capability::CapabilityProcess;
use anyhow::Result;
use pyro_artifacts::cache::CacheManager;
use pyro_artifacts::cargo::CapabilityIdent;
use pyro_spec::InterfaceSpec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum CapabilityRequest {
    GetSpec {
        author: String,
        name: String,
        version: String,
    },
    List {
        author: Option<String>,
        package: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CapabilityResponse {
    Spec { spec: InterfaceSpec<'static> },
    Capabilities { capabilities: Vec<CapabilityIdent> },
    Error { message: String },
}

#[derive(Clone)]
pub struct CapabilityManager {
    processes: Arc<Mutex<HashMap<CapabilityIdent, CapabilityProcess>>>,
}

impl CapabilityManager {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn handle_request(&self, req: CapabilityRequest) -> CapabilityResponse {
        match req {
            CapabilityRequest::GetSpec {
                author,
                name,
                version,
            } => match CacheManager::from_env().await {
                Ok(cache) => {
                    match cache
                        .capability_interface_spec(&author, &name, &version)
                        .await
                    {
                        Ok(spec_str) => {
                            match serde_json::from_str::<InterfaceSpec<'static>>(&spec_str) {
                                Ok(spec) => CapabilityResponse::Spec { spec },
                                Err(e) => CapabilityResponse::Error {
                                    message: format!(
                                        "Failed to parse interface spec JSON: {:?}",
                                        e
                                    ),
                                },
                            }
                        }
                        Err(e) => CapabilityResponse::Error {
                            message: format!("Failed to read interface spec: {:?}", e),
                        },
                    }
                }
                Err(e) => CapabilityResponse::Error {
                    message: format!("Failed to load CacheManager: {:?}", e),
                },
            },
            CapabilityRequest::List { author, package } => match CacheManager::from_env().await {
                Ok(cache) => match cache.list_available_capabilities().await {
                    Ok(capabilities) => {
                        let mut list = Vec::new();
                        for (a, p, v) in capabilities {
                            if let Some(ref filter_author) = author {
                                if a != *filter_author {
                                    continue;
                                }
                            }
                            if let Some(ref filter_package) = package {
                                if p != *filter_package {
                                    continue;
                                }
                            }
                            list.push(CapabilityIdent {
                                author: a,
                                package: p,
                                version: v,
                            });
                        }
                        CapabilityResponse::Capabilities { capabilities: list }
                    }
                    Err(e) => CapabilityResponse::Error {
                        message: format!("Failed to list capabilities: {:?}", e),
                    },
                },
                Err(e) => CapabilityResponse::Error {
                    message: format!("Failed to load CacheManager: {:?}", e),
                },
            },
        }
    }

    pub async fn start_capability(
        &self,
        cap: &CapabilityIdent,
        socket_path: &Path,
        cap_config: Option<&serde_json::Value>,
    ) -> Result<()> {
        let proc = CapabilityProcess::spawn(cap, socket_path, cap_config).await?;
        let mut guard = self.processes.lock().await;
        guard.insert(cap.clone(), proc);
        Ok(())
    }

    pub async fn stop_capability(&self, cap: &CapabilityIdent) -> Result<()> {
        let mut guard = self.processes.lock().await;
        if let Some(mut proc) = guard.remove(cap) {
            proc.kill().await?;
        }
        Ok(())
    }

    pub async fn stop_all(&self) -> Result<()> {
        let mut guard = self.processes.lock().await;
        for (_, mut proc) in guard.drain() {
            let _ = proc.kill().await;
        }
        Ok(())
    }
}
