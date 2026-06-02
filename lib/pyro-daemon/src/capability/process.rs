use crate::Result;
use pyro_artifacts::cargo::CapabilityIdent;
use pyroduct::Capture;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

#[derive(Debug)]
pub struct CapabilityProcess {
    pub cap: CapabilityIdent,
    pub socket_path: PathBuf,
    pub child: Child,
}

impl CapabilityProcess {
    pub async fn spawn(
        cap: &CapabilityIdent,
        socket_path: &Path,
        cap_config: Option<&serde_json::Value>,
    ) -> Result<Self> {
        let pyroduct_bin = get_pyroduct_bin();
        tracing::info!(
            cap = %cap,
            bin = %pyroduct_bin.display(),
            socket = %socket_path.display(),
            "Spawning capability runner process"
        );

        let mut cmd = Command::new(pyroduct_bin);
        cmd.arg("serve")
            .arg("--server-type")
            .arg("capability")
            .arg("--socket")
            .arg(socket_path)
            .arg("--cap")
            .arg(cap.to_string());

        if let Some(config) = cap_config {
            let config_json = serde_json::to_string(config)
                .capture("Failed to serialize capability config to JSON")?;
            cmd.arg("--cap-config").arg(config_json);
        }

        // Redirect outputs to avoid cluttering, but capture for tracing
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd
            .spawn()
            .capture("Failed to spawn capability runner child process")?;

        // Start tasks to read stdout/stderr and trace them
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let cap_clone = cap.to_string();

        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::info!(cap = %cap_clone, "[STDOUT] {}", line);
            }
        });

        let cap_clone = cap.to_string();
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::warn!(cap = %cap_clone, "[STDERR] {}", line);
            }
        });

        // Wait for UDS socket file to be created
        let mut retries = 0;
        while !socket_path.exists() {
            if retries > 100 {
                let _ = child.kill().await;
                pyroduct::bail!(
                    "Capability process failed to bind socket at {:?}",
                    socket_path
                );
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            retries += 1;
        }

        Ok(Self {
            cap: cap.clone(),
            socket_path: socket_path.to_path_buf(),
            child,
        })
    }

    pub async fn kill(&mut self) -> Result<()> {
        let _ = self.child.kill().await;
        if self.socket_path.exists() {
            let _ = fs::remove_file(&self.socket_path).await;
        }
        Ok(())
    }
}

impl Drop for CapabilityProcess {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
        if self.socket_path.exists() {
            let _ = std::fs::remove_file(&self.socket_path);
        }
    }
}

fn get_pyroduct_bin() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        let parent = exe.parent().unwrap();
        let candidate = parent.join("pyroduct");
        if candidate.exists() {
            return candidate;
        }
        // Check standard target directory parent for workspace development
        if let Some(grandparent) = parent.parent() {
            let candidate = grandparent.join("pyroduct");
            if candidate.exists() {
                return candidate;
            }
        }
    }
    PathBuf::from("pyroduct")
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyroduct::format::PyroVec;
    use pyroduct::format::header::{DataStatus, PyroHeader, PyroHeaderMut};
    use pyroduct::transport::socket::PyroSocket;
    use tempfile::tempdir;

    #[tracing_test::traced_test]
    #[tokio::test]
    async fn test_capability_process_spawn_and_rpc() {
        // Get path using cache like in socket_capability.rs
        let tmp_dir = tempdir().unwrap();
        let socket_path = tmp_dir.path().join("test_cap_rpc.sock");

        let cap_config = serde_json::json!({
            "classes": {
                "transform": {
                    "uppercase": true,
                    "suffix": "!!!"
                }
            }
        });

        let cap_ident = pyro_artifacts::cargo::CapabilityIdent {
            author: "nbhdai".to_string(),
            package: "config".to_string(),
            version: "0.1.0".to_string(),
        };

        // Spawn capability process
        let mut proc = CapabilityProcess::spawn(&cap_ident, &socket_path, Some(&cap_config))
            .await
            .expect("Failed to spawn capability process");

        assert_eq!(proc.cap, cap_ident);
        assert!(socket_path.exists());

        // Connect to capability process via Unix domain socket
        let client = PyroSocket::connect_unix(&socket_path)
            .await
            .expect("Failed to connect to capability socket");

        // Fetch Interface (fn_id = 0) - returns the interface spec
        let fetch_req = PyroVec::ok();
        let fetch_resp = client
            .request(None, None, Some(0), fetch_req.view())
            .await
            .expect("Failed to fetch interface");
        assert!(fetch_resp.is_ok(), "Interface fetch should be successful");

        // Register Client (fn_id = 2) - returns client_id
        let mut reg_req = PyroVec::ok();
        reg_req.set_fn_id(2);
        reg_req.extend_from_slice(&0u64.to_le_bytes());
        reg_req.set_status(DataStatus::RkyvValid);

        let reg_resp = client
            .request(None, None, Some(2), reg_req.view())
            .await
            .expect("Failed to register client");

        let client_id = u32::from_le_bytes(reg_resp.as_slice()[0..4].try_into().unwrap());
        assert!(client_id > 0);

        // Kill the process
        proc.kill()
            .await
            .expect("Failed to kill capability process");
        assert!(!socket_path.exists());
    }
}
