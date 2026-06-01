use std::path::PathBuf;
use anyhow::{Context, Result};
use tracing_subscriber::EnvFilter;
use clap::Parser;

use pyro_daemon::PyroDaemon;

#[derive(Parser, Debug)]
#[command(
    name = "pyro-daemond",
    about = "PyroDaemon - Background Playbook and Process Supervisor",
    version
)]
struct Args {
    /// Path to daemon's working directory
    #[arg(short = 'd', long = "working-dir")]
    working_dir: Option<PathBuf>,
}

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

    // 2. Parse command-line arguments using clap
    let args = Args::parse();
    let working_dir = args.working_dir.unwrap_or_else(PyroDaemon::default_working_dir);

    // 3. Handle termination signals for clean shutdown
    let daemon = PyroDaemon::new(working_dir);
    let control_socket_clone = daemon.control_socket_path.clone();

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
