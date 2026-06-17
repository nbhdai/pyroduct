use clap::Parser;
use pyro_daemon::Result;
use pyroduct::Capture;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

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

    /// Bind control API to TCP socket address (e.g. "127.0.0.1:9000")
    #[arg(long = "bind-tcp")]
    bind_tcp: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            "trace,cranelift_frontend=off,mio=off,cranelift_codegen=off,wasmtime=off,pyroduct=debug".into()
        }))
        .init();

    tracing::info!("Initializing PyroDaemon...");

    // 2. Parse command-line arguments using clap
    let args = Args::parse();
    let working_dir = args
        .working_dir
        .unwrap_or_else(PyroDaemon::default_working_dir);

    // 3. Handle termination signals for clean shutdown
    let daemon = PyroDaemon::new(working_dir).with_bind_tcp(args.bind_tcp);
    let control_socket_clone = daemon.control_socket_path.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for ctrl-c signal");
        tracing::info!("Received shutdown signal (Ctrl+C). Cleaning up control socket...");
        if control_socket_clone.exists() {
            let _ = std::fs::remove_file(control_socket_clone);
        }
        std::process::exit(0);
    });

    // 4. Run the daemon control loop
    daemon
        .run()
        .await
        .capture("PyroDaemon crashed in control loop")?;

    Ok(())
}
