use pyroduct::Capture;
use crate::Result;
use pyroduct::format::Bridgeable;
use pyroduct::format::format::Wrapper;
use pyroduct::format::header::{PyroHeader, PyroHeaderMut};
use pyroduct::transport::socket::{PyroListener, PyroSocket};
use tokio::fs;

use std::sync::Arc;
use pyro_artifacts::cache::CacheManager;
use crate::capability::CapabilityManager;
use crate::data::DaemonDataManager;
use crate::playbook::PlaybooksManager;
use crate::{DaemonRequest, DaemonResponse, PyroDaemon};

impl PyroDaemon {
    pub async fn run(&self) -> Result<()> {
        fs::create_dir_all(&self.working_dir)
            .await
            .capture("Failed to create working directory")?;
        fs::create_dir_all(self.working_dir.join("data"))
            .await
            .capture("Failed to create data directory")?;

        // Auto-resume active playbooks on startup
        if let Err(e) = self.playbooks_manager.resume_active_playbooks().await {
            tracing::error!("Failed to resume active playbooks on startup: {:?}", e);
        }

        // Spawn periodic auto-update loop for non-pinned playbooks
        let update_manager = self.playbooks_manager.clone();
        tokio::spawn(async move {
            update_manager
                .run_update_loop(std::time::Duration::from_secs(60))
                .await;
        });

        let listener = if let Some(ref addr) = self.bind_tcp {
            tracing::info!(address = %addr, "PyroDaemon binding control listener to TCP");
            PyroListener::bind_tcp(addr)
                .await
                .capture("Failed to bind PyroListener TCP control listener")?
        } else {
            if self.control_socket_path.exists() {
                fs::remove_file(&self.control_socket_path)
                    .await
                    .capture("Failed to clean up existing control socket file")?;
            }
            tracing::info!(socket = %self.control_socket_path.display(), "PyroDaemon binding control listener to Unix socket");
            PyroListener::bind_unix(&self.control_socket_path)
                .await
                .capture("Failed to bind PyroListener Unix control listener")?
        };

        tracing::info!("PyroDaemon listening for control commands");

        loop {
            let socket = match listener.accept().await {
                Ok(conn) => conn,
                Err(e) => {
                    tracing::error!("Failed to accept incoming control connection: {:?}", e);
                    continue;
                }
            };

            let playbooks_clone = self.playbooks_manager.clone();
            let capability_clone = self.capability_manager.clone();
            let data_clone = self.data_manager.clone();
            let cache_clone = self.cache_manager.clone();
            tokio::spawn(async move {
                if let Err(e) =
                    handle_client(socket, playbooks_clone, capability_clone, data_clone, cache_clone).await
                {
                    tracing::error!("Error handling control client: {:?}", e);
                }
            });
        }
    }
}

async fn handle_client(
    socket: PyroSocket,
    playbooks_manager: std::sync::Arc<PlaybooksManager>,
    capability_manager: CapabilityManager,
    data_manager: DaemonDataManager,
    cache_manager: Arc<CacheManager>,
) -> Result<()> {
    loop {
        let view = match socket.recv().await {
            Ok(v) => v,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::ConnectionAborted
                    || e.kind() == std::io::ErrorKind::UnexpectedEof
                {
                    break;
                }
                return Err(e).capture("Failed to receive from control socket");
            }
        };

        let playbooks_manager = playbooks_manager.clone();
        let capability_manager = capability_manager.clone();
        let data_manager = data_manager.clone();
        let cache_manager = cache_manager.clone();
        let socket = socket.clone();

        tokio::spawn(async move {
            let typed = match DaemonRequest::expose(view) {
                Ok(t) => t,
                Err(e) => {
                    let err_resp = DaemonResponse::Error {
                        message: format!("Invalid JSON request: {}", e),
                    };
                    if let Ok(resp_vec) = err_resp.ship() {
                        let _ = socket.send(resp_vec.into()).await;
                    }
                    return;
                }
            };

            let req = (*typed).clone();
            let mux_id = typed.data().mux_id();

            let response = match req {
                DaemonRequest::Playbook(playbook_req) => {
                    DaemonResponse::Playbook(playbooks_manager.handle_request(playbook_req).await)
                }
                DaemonRequest::Capability(capability_req) => {
                    DaemonResponse::Capability(capability_manager.handle_request(capability_req).await)
                }
                DaemonRequest::Cache(cache_req) => {
                    DaemonResponse::Cache(crate::cache::handle_request(&cache_manager, cache_req).await)
                }
                DaemonRequest::Data(data_req) => {
                    DaemonResponse::Data(data_manager.handle_request(data_req, &socket, Some(mux_id)).await)
                }
                DaemonRequest::Status => {
                    let count = playbooks_manager.active_workers_count().await;
                    let playbooks = playbooks_manager.list_playbooks().await;
                    DaemonResponse::StatusInfo {
                        active_workers: count,
                        version: env!("CARGO_PKG_VERSION").to_string(),
                        running_playbooks: playbooks,
                    }
                }
            };

            if let Ok(mut resp_vec) = response.ship() {
                resp_vec.set_mux_id(mux_id);
                if let Err(e) = socket.send(resp_vec.into()).await {
                    tracing::error!("Failed to send response for mux_id {}: {:?}", mux_id, e);
                }
            }
        });
    }

    Ok(())
}
