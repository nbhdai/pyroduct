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

    pub async fn start_replay(
        &self,
        playbook_name: String,
        folder_path: String,
        interval_ms: u64,
        wiggle_ms: u64,
    ) -> Result<usize> {
        let req = DaemonRequest::Data(DataRequest::StartReplay {
            playbook_name,
            folder_path,
            interval_ms,
            wiggle_ms,
        });
        match self.request(req).await? {
            DaemonResponse::Data(DataResponse::ReplayStarted { total_rows }) => Ok(total_rows),
            DaemonResponse::Data(DataResponse::Error { message }) => {
                pyroduct::bail!("{}", message)
            }
            _ => pyroduct::bail!("Unexpected response from daemon"),
        }
    }

    pub async fn start_parallel_replay(
        &self,
        playbook_name: String,
        folder_path: String,
        concurrency: usize,
    ) -> Result<usize> {
        let req = DaemonRequest::Data(DataRequest::StartParallelReplay {
            playbook_name,
            folder_path,
            concurrency,
        });
        match self.request(req).await? {
            DaemonResponse::Data(DataResponse::ReplayStarted { total_rows }) => Ok(total_rows),
            DaemonResponse::Data(DataResponse::Error { message }) => {
                pyroduct::bail!("{}", message)
            }
            _ => pyroduct::bail!("Unexpected response from daemon"),
        }
    }

    pub async fn get_replay_status(
        &self,
        playbook_name: String,
    ) -> Result<(bool, usize, usize, usize, usize, String)> {
        let req = DaemonRequest::Data(DataRequest::GetReplayStatus { playbook_name });
        match self.request(req).await? {
            DaemonResponse::Data(DataResponse::ReplayStatus {
                running,
                total_rows,
                rows_completed,
                successes,
                errors,
                current_file,
            }) => Ok((running, total_rows, rows_completed, successes, errors, current_file)),
            DaemonResponse::Data(DataResponse::Error { message }) => {
                pyroduct::bail!("{}", message)
            }
            _ => pyroduct::bail!("Unexpected response from daemon"),
        }
    }

    pub async fn stop_replay(&self, playbook_name: String) -> Result<()> {
        let req = DaemonRequest::Data(DataRequest::StopReplay { playbook_name });
        match self.request(req).await? {
            DaemonResponse::Data(DataResponse::ReplayStopped) => Ok(()),
            DaemonResponse::Data(DataResponse::Error { message }) => {
                pyroduct::bail!("{}", message)
            }
            _ => pyroduct::bail!("Unexpected response from daemon"),
        }
    }

    pub async fn export_playbook_data(&self, playbook_name: String) -> Result<Vec<u8>> {
        let req = DaemonRequest::Data(DataRequest::ExportPlaybookData { playbook_name });
        match self.request(req).await? {
            DaemonResponse::Data(DataResponse::PlaybookData { ipc_bytes }) => Ok(ipc_bytes),
            DaemonResponse::Data(DataResponse::Error { message }) => pyroduct::bail!("{}", message),
            _ => pyroduct::bail!("Unexpected response from daemon"),
        }
    }
}
