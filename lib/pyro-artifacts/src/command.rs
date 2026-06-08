// ! The cargo and macro expand tools

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
#[tracing::instrument(skip(root), fields(root = %root.display(), tool_args = ?tool_args))]
pub async fn run_command(
    root: &Path,
    tool_args: &[&str],
    capture: bool,
) -> Result<String, CommandError> {
    tracing::debug!("Executing cargo command: cargo {:?}", tool_args);
    let mut cmd = Command::new("cargo");
    cmd.args(tool_args).current_dir(root);

    if capture {
        let output = match cmd.output().await {
            Ok(o) => o,
            Err(e) => {
                tracing::error!(error = ?e, "Failed to launch cargo command");
                return Err(CommandError::Io(e));
            }
        };

        if !output.status.success() {
            let err = CommandError::Cargo {
                status: output.status,
                args: tool_args.iter().map(|s| s.to_string()).collect(),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            };
            tracing::error!(status = ?output.status, "Cargo command failed with exit status");
            return Err(err);
        }
        tracing::debug!("Cargo command completed successfully (captured output)");
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let status = match cmd.status().await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(error = ?e, "Failed to launch cargo command");
                return Err(CommandError::Io(e));
            }
        };

        if !status.success() {
            let err = CommandError::Cargo {
                status,
                args: tool_args.iter().map(|s| s.to_string()).collect(),
                stdout: String::from("Not captured"),
                stderr: String::from("Not captured"),
            };
            tracing::error!(status = ?status, "Cargo command failed with exit status");
            return Err(err);
        }
        tracing::debug!("Cargo command completed successfully");
        Ok(String::new())
    }
}

/// Format a syn::Error with source context
pub fn format_syn_error(source: &str, err: syn::Error) -> String {
    let span = err.span();
    let start = span.start();
    let msg = err.to_string();

    let lines: Vec<&str> = source.lines().collect();
    let line_num = start.line;
    let col = start.column;

    let mut output = String::new();
    output.push_str(&format!("error: {}\n", msg));
    output.push_str(&format!("  --> src/lib.rs:{}:{}\n", line_num, col + 1));
    output.push_str("   |\n");

    // Show context: line before, error line, line after
    let start_line = line_num.saturating_sub(2);
    let end_line = (line_num + 1).min(lines.len());

    for i in start_line..end_line {
        let line_content = lines.get(i).unwrap_or(&"");
        let display_num = i + 1;

        if display_num == line_num {
            output.push_str(&format!("{:3} | {}\n", display_num, line_content));
            // Add caret pointing to the column
            output.push_str(&format!("    | {}^\n", " ".repeat(col)));
        } else {
            output.push_str(&format!("{:3} | {}\n", display_num, line_content));
        }
    }
    output.push_str("   |\n");

    output
}
