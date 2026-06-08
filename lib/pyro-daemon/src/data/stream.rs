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
        tracing::trace!("stream_playbook_data: starting for playbook '{}', mux_id={:?}", playbook_name, mux_id);
        // 1. Check if the playbook worker exists and is running
        let workers = self.playbooks_manager.workers.lock().await;
        let worker = workers
            .get(playbook_name)
            .ok_or_else(|| pyroduct::capture!("Playbook '{}' is not running", playbook_name))?;

        // 2. Create an mpsc channel to receive rows from the callback
        let (tx, mut rx) = mpsc::channel::<PyroRow<'static>>(100);

        // 3. Register the callback on the worker
        let uuid = Uuid::new_v4();
        let cb = pyroduct::pipeline::Callback::function(move |idx, row| {
            let tx = tx.clone();
            let row = row.to_static();
            tracing::trace!("stream_playbook_data callback: received row index {}, sending to channel", idx);
            Box::pin(async move {
                let _ = tx.send(row).await;
            })
        });

        worker.add_callback(uuid, cb).await?;
        tracing::trace!("stream_playbook_data: registered callback with UUID {}", uuid);
        // Drop workers lock so we don't hold it during streaming
        drop(workers);

        // 4. Send StreamStarted to acknowledge the request
        tracing::trace!("stream_playbook_data: sending StreamStarted response");
        let start_resp = crate::DaemonResponse::Data(DataResponse::StreamStarted);
        let mut resp_vec = start_resp
            .ship()
            .map_err(|e| pyroduct::capture!("{:?}", e))?;
        if let Some(mid) = mux_id {
            resp_vec.set_mux_id(mid);
        }
        if let Err(e) = socket.send(resp_vec.into()).await {
            tracing::trace!("stream_playbook_data: failed to send StreamStarted: {:?}", e);
            // Cleanup callback if send fails
            let workers = self.playbooks_manager.workers.lock().await;
            if let Some(w) = workers.get(playbook_name) {
                let _ = w.delete_callback(uuid).await;
            }
            return Err(e).capture("Failed to send stream started response");
        }

        // 5. Loop receiving from channel and sending to socket
        while let Some(row) = rx.recv().await {
            tracing::trace!("stream_playbook_data: received row from channel, forwarding to socket");
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

        tracing::trace!("stream_playbook_data: loop finished, cleaning up callback");
        // 6. Cleanup callback
        let workers = self.playbooks_manager.workers.lock().await;
        if let Some(w) = workers.get(playbook_name) {
            let _ = w.delete_callback(uuid).await;
        }

        tracing::trace!("stream_playbook_data: returning success");
        Ok(())
    }
}
