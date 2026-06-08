use crate::Result;
use crate::client::DaemonClient;
use crate::data::{DataRequest, DataResponse};
use crate::{DaemonRequest, DaemonResponse};
use pyroduct::Capture;
use pyroduct::format::Bridgeable;

impl DaemonClient {
    pub async fn get_base_dir(&self) -> Result<String> {
        let req = DaemonRequest::Data(DataRequest::GetBaseDir);
        match self.request(req).await? {
            DaemonResponse::Data(DataResponse::BaseDir { path }) => Ok(path),
            DaemonResponse::Data(DataResponse::Error { message }) => pyroduct::bail!("{}", message),
            _ => pyroduct::bail!("Unexpected response from daemon"),
        }
    }

    pub async fn query_playbook(
        &self,
        playbook_name: String,
        sql_query: String,
    ) -> Result<Vec<u8>> {
        let req = DaemonRequest::Data(DataRequest::QueryPlaybook {
            playbook_name,
            sql_query,
        });
        match self.request(req).await? {
            DaemonResponse::Data(DataResponse::QueryResult { ipc_bytes }) => Ok(ipc_bytes),
            DaemonResponse::Data(DataResponse::Error { message }) => pyroduct::bail!("{}", message),
            _ => pyroduct::bail!("Unexpected response from daemon"),
        }
    }

    pub async fn get_playbook_data(
        &self,
        playbook_name: String,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<u8>> {
        let req = DaemonRequest::Data(DataRequest::GetPlaybookData {
            playbook_name,
            offset,
            limit,
        });
        match self.request(req).await? {
            DaemonResponse::Data(DataResponse::PlaybookData { ipc_bytes }) => Ok(ipc_bytes),
            DaemonResponse::Data(DataResponse::Error { message }) => pyroduct::bail!("{}", message),
            _ => pyroduct::bail!("Unexpected response from daemon"),
        }
    }

    pub async fn get_playbook_failures(
        &self,
        playbook_name: String,
    ) -> Result<Vec<pyroduct::pipeline::ServerExecutionRecord>> {
        let req = DaemonRequest::Data(DataRequest::GetPlaybookFailures { playbook_name });
        match self.request(req).await? {
            DaemonResponse::Data(DataResponse::PlaybookFailures { failures }) => Ok(failures),
            DaemonResponse::Data(DataResponse::Error { message }) => pyroduct::bail!("{}", message),
            _ => pyroduct::bail!("Unexpected response from daemon"),
        }
    }

    pub async fn get_playbook_execution_record(
        &self,
        playbook_name: String,
        id: u32,
    ) -> Result<pyroduct::pipeline::ServerExecutionRecord> {
        let req =
            DaemonRequest::Data(DataRequest::GetPlaybookExecutionRecord { playbook_name, id });
        match self.request(req).await? {
            DaemonResponse::Data(DataResponse::PlaybookExecutionRecord { record }) => Ok(record),
            DaemonResponse::Data(DataResponse::Error { message }) => pyroduct::bail!("{}", message),
            _ => pyroduct::bail!("Unexpected response from daemon"),
        }
    }

    pub async fn stream_playbook(
        &self,
        playbook_name: String,
    ) -> Result<tokio::sync::mpsc::Receiver<Result<pyroduct::PyroRow<'static>>>> {
        let req = DaemonRequest::Data(DataRequest::StreamPlaybook { playbook_name });
        let req_vec = req.ship().capture("Failed to ship client request")?;

        let mut stream = self
            .socket
            .request_stream(None, None, None, req_vec.view())
            .await
            .capture("Failed to request stream from daemon")?;

        let (tx, rx) = tokio::sync::mpsc::channel(100);
        tokio::spawn(async move {
            let first_frame = match stream.recv().await {
                Some(v) => v,
                None => {
                    let _ = tx
                        .send(Err(pyroduct::capture!(
                            "Daemon disconnected before stream started"
                        )))
                        .await;
                    return;
                }
            };

            let first_exposed = match DaemonResponse::expose(first_frame) {
                Ok(exposed) => exposed,
                Err(e) => {
                    let _ = tx
                        .send(Err(pyroduct::capture!(
                            "Failed to expose first frame: {:?}",
                            e
                        )))
                        .await;
                    return;
                }
            };

            match &*first_exposed {
                DaemonResponse::Data(DataResponse::StreamStarted) => {}
                DaemonResponse::Data(DataResponse::Error { message }) => {
                    let _ = tx.send(Err(pyroduct::capture!("{}", message))).await;
                    return;
                }
                _ => {
                    let _ = tx
                        .send(Err(pyroduct::capture!("Unexpected response from daemon")))
                        .await;
                    return;
                }
            }

            while let Some(view) = stream.recv().await {
                let exposed = match DaemonResponse::expose(view) {
                    Ok(exp) => exp,
                    Err(e) => {
                        let _ = tx
                            .send(Err(pyroduct::capture!(
                                "Failed to expose stream frame: {:?}",
                                e
                            )))
                            .await;
                        break;
                    }
                };

                match &*exposed {
                    DaemonResponse::Data(DataResponse::StreamRow { row }) => {
                        if let Err(e) = tx.send(Ok(row.to_static())).await {
                            tracing::warn!("Failed to send row to receiver: {}", e);
                            break;
                        }
                    }
                    DaemonResponse::Data(DataResponse::StreamFinished) => {
                        break;
                    }
                    DaemonResponse::Data(DataResponse::Error { message }) => {
                        let _ = tx.send(Err(pyroduct::capture!("{}", message))).await;
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(rx)
    }
}
