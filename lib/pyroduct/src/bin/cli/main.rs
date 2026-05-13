pub mod commands;
pub mod tui;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Creates a new Pyroduct project (module or capability)
    Init {
        /// Directory to create the project in. Defaults to current directory.
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,

        /// Create a capability instead of a module
        #[arg(long)]
        cap: bool,
    },
    Expand {
        #[arg(value_name = "DIRECTORY")]
        path: PathBuf,

        #[arg(long)]
        no_compile: bool,
    },
    /// Places package artifacts into the local cache repository
    Ship {
        #[arg(value_name = "DIRECTORY")]
        path: PathBuf,

        #[arg(short, long)]
        debug: bool,

        /// Output directory for artifacts.
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Cleans generated artifacts (Cargo.toml, artifacts/, interface/, target/)
    Clean {
        #[arg(value_name = "DIRECTORY", default_value = ".")]
        path: PathBuf,
    },
    /// Initializes the Pyroduct cache
    Setup,
    /// Generates the interface.json for a capability
    Spec {
        #[arg(value_name = "DIRECTORY", default_value = ".")]
        path: PathBuf,

        /// Output path for the interface spec.
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Streams data from a file to a socket.
    Replay {
        /// Input file containing JSONL data.
        #[arg(value_name = "INPUT")]
        input: PathBuf,

        /// Path to the Unix domain socket or TCP address (e.g., 127.0.0.1:8080).
        #[arg(value_name = "SOCKET")]
        socket: String,
    },
    // /// Runs the pipeline.
    // /// Automatically detects if INPUT is a file path (Batch Mode) or a JSON string (Single Mode).
    Run {
        /// Path to the pipeline config TOML file
        #[arg(value_name = "CONFIG")]
        config: PathBuf,

        /// Input: Either a file path (for batch processing) or a raw JSON string.
        #[arg(value_name = "INPUT")]
        input: String,

        /// Output directory for batch processing. Defaults to current directory.
        #[arg(short, long, default_value = ".")]
        output_dir: PathBuf,

        /// Output format (only used for batch processing).
        #[arg(long, value_enum, default_value_t = commands::run::OutputFormat::Json)]
        format: commands::run::OutputFormat,

        /// Path to a Unix domain socket or TCP address (e.g., 127.0.0.1:8080) to listen on.
        #[arg(long)]
        socket: Option<String>,
    },
    Tui {
        /// Path to the pipeline config YAML file
        #[arg(value_name = "CONFIG", default_value = "pipeline.yaml")]
        config: PathBuf,

        /// Input: a file path for batch processing
        #[arg(value_name = "INPUT")]
        input: PathBuf,
    },
}

pub fn start_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "trace,cranelift_frontend=off,cranelift_codegen=off,wasmtime=off,mio=off".into()
    });

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true))
        .with(filter)
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match &args.command {
        Commands::Tui { .. } => {}
        _ => start_logging(),
    }

    match args.command {
        Commands::Init { path, cap } => commands::init::init(path, cap),
        Commands::Expand { path, no_compile } => commands::expand::expand(&path, no_compile).await,
        Commands::Ship { path, debug, out } => commands::ship::ship(&path, debug, out.as_deref()).await,
        Commands::Clean { path } => commands::clean::clean(&path),
        Commands::Setup => commands::cache::init().await,
        Commands::Spec { path, out } => commands::spec::spec(&path, out.as_deref()).await,
        Commands::Replay { input, socket } => commands::replay::replay(&input, &socket).await,
        Commands::Run {
            config,
            input,
            output_dir,
            format,
            socket,
        } => {
            if let Some(socket_addr) = socket {
                commands::run::run_socket(&config, &socket_addr).await
            } else {
                let input_path = Path::new(&input);

                if input_path.exists() && input_path.is_file() {
                    commands::run::run_batch(&config, input_path, &output_dir, format).await
                } else {
                    commands::run::run(&config, &input).await
                }
            }
        }
        Commands::Tui { config, input } => tui::run_tui(&config, &input).await,
    }
}
