use std::path::Path;
use thiserror::Error;

use tokio::process::Command;

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error(
        "Cargo command failed with status {status}. Args: {args:?}\nStdout: {stdout}\nStderr: {stderr}"
    )]
    Cargo {
        status: std::process::ExitStatus,
        args: Vec<String>,
        stdout: String,
        stderr: String,
    },
}

/// Run a cargo command within this environment
pub async fn run_command(
    root: &Path,
    tool_args: &[&str],
    capture: bool,
) -> Result<String, CommandError> {
    let mut cmd = Command::new("cargo");
    cmd.args(tool_args).current_dir(&root);

    if capture {
        let output = cmd.output().await?;

        if !output.status.success() {
            return Err(CommandError::Cargo {
                status: output.status,
                args: tool_args.iter().map(|s| s.to_string()).collect(),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let status = cmd.status().await?;

        if !status.success() {
            return Err(CommandError::Cargo {
                status,
                args: tool_args.iter().map(|s| s.to_string()).collect(),
                stdout: String::from("Not captured"),
                stderr: String::from("Not captured"),
            });
        }
        Ok(String::new())
    }
}
