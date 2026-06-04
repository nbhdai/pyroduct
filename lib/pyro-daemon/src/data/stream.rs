use super::{DaemonDataManager, DataResponse};
use crate::Result;
use pyroduct::format::Bridgeable;
use pyroduct::format::header::PyroHeaderMut;
use pyroduct::transport::socket::PyroSocket;
use pyroduct::{Capture, PyroRow};
use tokio::sync::mpsc;
use uuid::Uuid;

impl DaemonDataManager {
    pub async fn stream_playbook_data(
        &self,
        playbook_name: &str,
        socket: &PyroSocket,
        mux_id: Option<u32>,
    ) -> Result<()> {
        // 1. Check if the playbook worker exists and is running
        let workers = self.playbooks_manager.workers.lock().await;
        let worker = workers
            .get(playbook_name)
            .ok_or_else(|| pyroduct::capture!("Playbook '{}' is not running", playbook_name))?;

        // 2. Create an mpsc channel to receive rows from the callback
        let (tx, mut rx) = mpsc::channel::<PyroRow<'static>>(100);

        // 3. Register the callback on the worker
        let uuid = Uuid::new_v4();
        let cb = pyroduct::pipeline::Callback::function(move |_idx, row| {
            let tx = tx.clone();
            let row = row.to_static();
            Box::pin(async move {
                let _ = tx.send(row).await;
            })
        });

        worker.add_callback(uuid, cb).await?;
        // Drop workers lock so we don't hold it during streaming
        drop(workers);

        // 4. Send StreamStarted to acknowledge the request
        let start_resp = crate::DaemonResponse::Data(DataResponse::StreamStarted);
        let mut resp_vec = start_resp
            .ship()
            .map_err(|e| pyroduct::capture!("{:?}", e))?;
        if let Some(mid) = mux_id {
            resp_vec.set_mux_id(mid);
        }
        if let Err(e) = socket.send(resp_vec.into()).await {
            // Cleanup callback if send fails
            let workers = self.playbooks_manager.workers.lock().await;
            if let Some(w) = workers.get(playbook_name) {
                let _ = w.delete_callback(uuid).await;
            }
            return Err(e).capture("Failed to send stream started response");
        }

        // 5. Loop receiving from channel and sending to socket
        while let Some(row) = rx.recv().await {
            let row_resp = crate::DaemonResponse::Data(DataResponse::StreamRow { row });
            let mut resp_vec = row_resp.ship().map_err(|e| pyroduct::capture!("{:?}", e))?;
            if let Some(mid) = mux_id {
                resp_vec.set_mux_id(mid);
            }
            if let Err(e) = socket.send(resp_vec.into()).await {
                tracing::error!("Failed to send stream row to client: {:?}", e);
                break;
            }
        }

        // 6. Cleanup callback
        let workers = self.playbooks_manager.workers.lock().await;
        if let Some(w) = workers.get(playbook_name) {
            let _ = w.delete_callback(uuid).await;
        }

        // 7. Send StreamFinished
        let end_resp = crate::DaemonResponse::Data(DataResponse::StreamFinished);
        let mut resp_vec = end_resp.ship().map_err(|e| pyroduct::capture!("{:?}", e))?;
        if let Some(mid) = mux_id {
            resp_vec.set_mux_id(mid);
        }
        let _ = socket.send(resp_vec.into()).await;

        Ok(())
    }
}
