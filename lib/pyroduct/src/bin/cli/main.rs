pub mod commands;

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
    /// Purges the Pyroduct cache (capabilities, interfaces, modules, anon)
    Purge,
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
    /// Starts a playbook server or capability server.
    Serve {
        /// Path to JSON configuration file
        #[arg(long)]
        config: Option<PathBuf>,

        /// JSON encoded configuration string
        #[arg(long)]
        config_json: Option<String>,

        /// Server type to run (playbook or capability)
        #[arg(long, value_enum)]
        server_type: Option<commands::serve::ServerType>,

        /// Address or path to listen on (e.g. 127.0.0.1:8080 or /tmp/pyro.sock)
        #[arg(long)]
        socket: Option<String>,

        /// Run playbook server as an HTTP server instead of TCP/Unix
        #[arg(long)]
        http: bool,

        /// Path to the playbook config file (playbook only)
        #[arg(long)]
        playbook_config: Option<PathBuf>,

        /// Playbook identifier (author:package:version) (playbook only)
        #[arg(long, value_parser = parse_playbook_ident)]
        playbook: Option<pyro_artifacts::artifacts::PlaybookIdent>,

        /// Remote capability mappings in format "author:package:version=addr"
        #[arg(long, value_parser = parse_remote_mapping)]
        remote: Vec<(
            pyro_artifacts::cargo::CapabilityIdent,
            pyro_artifacts::cache::RemoteAddress,
        )>,

        /// WAL capacity
        #[arg(long)]
        wal_capacity: Option<usize>,

        /// Success log retention in seconds
        #[arg(long)]
        success_log_retention_secs: Option<u64>,

        /// Error log retention in seconds
        #[arg(long)]
        error_log_retention_secs: Option<u64>,

        /// Directory for logs
        #[arg(long)]
        log_dir: Option<PathBuf>,

        /// Directory for inputs
        #[arg(long)]
        input_dir: Option<PathBuf>,

        /// Directory for outputs
        #[arg(long)]
        output_dir: Option<PathBuf>,

        /// Name of the capability library (capability only) in format author:package:version
        #[arg(long, value_parser = parse_capability_ident)]
        cap: Option<pyro_artifacts::cargo::CapabilityIdent>,

        /// Capability's class configuration as a JSON string (capability only)
        #[arg(long)]
        cap_config: Option<String>,
    },
    /// Converts a dataset WAL (.pyrowal) file to a Parquet file
    WalToParquet {
        /// Path to the dataset WAL file
        #[arg(value_name = "WAL_PATH")]
        wal: PathBuf,

        /// Path to the output Parquet file
        #[arg(value_name = "PARQUET_PATH")]
        output: PathBuf,
    },
    /// Recovers every error-free (input, output) pair for a playbook run into a single
    /// Parquet file, reconstructing row order from the execution log rather than
    /// trusting the (possibly restart-corrupted) DataManager/SQLite index.
    DumpPairs {
        /// Directory containing the playbook's execution logs
        #[arg(long)]
        log_dir: PathBuf,

        /// Directory containing the playbook's input DataManager
        #[arg(long)]
        input_dir: PathBuf,

        /// Directory containing the playbook's output DataManager
        #[arg(long)]
        output_dir: PathBuf,

        /// Path to the output Parquet file
        #[arg(long)]
        output: PathBuf,
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

    start_logging();

    match args.command {
        Commands::Init { path, cap } => commands::init::init(path, cap),
        Commands::Expand { path, no_compile } => commands::expand::expand(&path, no_compile).await,
        Commands::Ship { path, debug, out } => {
            commands::ship::ship(&path, debug, out.as_deref()).await
        }
        Commands::Clean { path } => commands::clean::clean(&path),
        Commands::Setup => commands::cache::init().await,
        Commands::Purge => commands::cache::purge().await,
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
        Commands::Serve {
            config,
            config_json,
            server_type,
            socket,
            http,
            playbook_config,
            playbook,
            remote,
            wal_capacity,
            success_log_retention_secs,
            error_log_retention_secs,
            log_dir,
            input_dir,
            output_dir,
            cap,
            cap_config,
        } => {
            let remote_map = remote.into_iter().collect();
            commands::serve::serve(
                config.as_deref(),
                config_json.as_deref(),
                server_type,
                socket,
                http,
                playbook_config.as_deref(),
                playbook,
                remote_map,
                wal_capacity,
                success_log_retention_secs,
                error_log_retention_secs,
                log_dir,
                input_dir,
                output_dir,
                cap,
                cap_config.as_deref(),
            )
            .await
        }
        Commands::WalToParquet { wal, output } => {
            commands::wal_to_parquet::wal_to_parquet(&wal, &output).await
        }
        Commands::DumpPairs {
            log_dir,
            input_dir,
            output_dir,
            output,
        } => {
            commands::dump_pairs::dump_pairs(&log_dir, &input_dir, &output_dir, &output).await
        }
    }
}

fn parse_capability_ident(s: &str) -> Result<pyro_artifacts::cargo::CapabilityIdent, String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 3 {
        return Err(format!(
            "Invalid capability identifier '{}'. Must be in format '{{author}}:{{name}}:{{version}}', e.g. 'my_author:my_cap:0.1.0'",
            s
        ));
    }
    if parts[0].is_empty() || parts[1].is_empty() || parts[2].is_empty() {
        return Err(format!(
            "Capability identifier parts cannot be empty. Got '{}'",
            s
        ));
    }
    Ok(pyro_artifacts::cargo::CapabilityIdent {
        author: parts[0].to_string(),
        package: parts[1].to_string(),
        version: parts[2].to_string(),
    })
}

fn parse_playbook_ident(s: &str) -> Result<pyro_artifacts::artifacts::PlaybookIdent, String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 3 {
        return Err(format!(
            "Invalid playbook identifier '{}'. Must be in format '{{author}}:{{package}}:{{version}}', e.g. 'my_author:my_playbook:0.1.0'",
            s
        ));
    }
    if parts[0].is_empty() || parts[1].is_empty() || parts[2].is_empty() {
        return Err(format!(
            "Playbook identifier parts cannot be empty. Got '{}'",
            s
        ));
    }
    Ok(pyro_artifacts::artifacts::PlaybookIdent {
        author: parts[0].to_string(),
        package: parts[1].to_string(),
        version: parts[2].to_string(),
    })
}

fn parse_remote_mapping(
    s: &str,
) -> Result<
    (
        pyro_artifacts::cargo::CapabilityIdent,
        pyro_artifacts::cache::RemoteAddress,
    ),
    String,
> {
    let parts: Vec<&str> = s.split('=').collect();
    if parts.len() != 2 {
        return Err(format!(
            "Invalid remote mapping '{}'. Must be in format '{{author}}:{{package}}:{{version}}={{addr}}'",
            s
        ));
    }
    let cap_ident = parse_capability_ident(parts[0])?;
    let addr_str = parts[1];
    let remote_addr = if let Some(stripped) = addr_str.strip_prefix("tcp://") {
        pyro_artifacts::cache::RemoteAddress::Tcp(stripped.to_string())
    } else if let Some(stripped) = addr_str.strip_prefix("unix://") {
        pyro_artifacts::cache::RemoteAddress::Unix(PathBuf::from(stripped))
    } else if addr_str.contains(':') {
        pyro_artifacts::cache::RemoteAddress::Tcp(addr_str.to_string())
    } else {
        pyro_artifacts::cache::RemoteAddress::Unix(PathBuf::from(addr_str))
    };
    Ok((cap_ident, remote_addr))
}
