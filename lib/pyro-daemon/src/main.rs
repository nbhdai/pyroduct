use std::path::PathBuf;
use anyhow::{Context, Result};
use tracing_subscriber::EnvFilter;

use pyro_daemon::PyroDaemon;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,pyro_daemon=debug,pyroduct=debug")),
        )
        .init();

    tracing::info!("Initializing PyroDaemon...");

    // 2. Parse command-line arguments manually to keep it simple and lightweight
    let mut control_socket = PathBuf::from("/tmp/pyro-daemon.sock");
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--control-socket" | "-s" => {
                if let Some(socket_val) = args.next() {
                    control_socket = PathBuf::from(socket_val);
                } else {
                    anyhow::bail!("Missing value for --control-socket argument");
                }
            }
            "--help" | "-h" => {
                println!("PyroDaemon - Background Playbook and Process Supervisor");
                println!();
                println!("Usage:");
                println!("  pyro-daemond [OPTIONS]");
                println!();
                println!("Options:");
                println!("  -s, --control-socket <PATH>   Path to daemon's control Unix socket (default: /tmp/pyro-daemon.sock)");
                println!("  -h, --help                    Print help info");
                return Ok(());
            }
            other => {
                anyhow::bail!("Unknown argument: {}. Run with --help for options.", other);
            }
        }
    }

    // 3. Handle termination signals for clean shutdown
    let daemon = PyroDaemon::new(control_socket.clone());
    let control_socket_clone = control_socket.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl-c signal");
        tracing::info!("Received shutdown signal (Ctrl+C). Cleaning up control socket...");
        if control_socket_clone.exists() {
            let _ = std::fs::remove_file(control_socket_clone);
        }
        std::process::exit(0);
    });

    // 4. Run the daemon control loop
    daemon.run().await.context("PyroDaemon crashed in control loop")?;

    Ok(())
}
