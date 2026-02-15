pub mod cargo;
pub mod clean;
pub mod expand;
pub mod init;
pub mod package;
// pub mod run;
pub mod symbols;
pub mod utils;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

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

        /// Convert WASM binaries to WAT format and gets the symbols from libraries
        #[arg(short, long)]
        bin: bool,

        /// Generates the Cargo.lock files
        #[arg(short, long, default_value = "true")]
        lockfile: bool,
    },
    Package {
        #[arg(value_name = "DIRECTORY")]
        path: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Pass additional arguments to cargo build
        #[arg(last = true)]
        cargo_args: Vec<String>,
    },
    /// Cleans generated artifacts (Cargo.toml, artifacts/, interface/, target/)
    Clean {
        #[arg(value_name = "DIRECTORY", default_value = ".")]
        path: PathBuf,
    },
    // /// Runs the pipeline.
    // /// Automatically detects if INPUT is a file path (Batch Mode) or a JSON string (Single Mode).
    // Run {
    //     /// Path to the harness config TOML file
    //     #[arg(value_name = "CONFIG")]
    //     config: PathBuf,

    //     /// Input: Either a file path (for batch processing) or a raw JSON string.
    //     #[arg(value_name = "INPUT")]
    //     input: String,

    //     /// Output directory for batch processing. Defaults to current directory.
    //     #[arg(short, long, default_value = ".")]
    //     output_dir: PathBuf,

    //     /// Output format (only used for batch processing).
    //     #[arg(long, value_enum, default_value_t = run::OutputFormat::Json)]
    //     format: run::OutputFormat,
    // },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::Init { path, cap } => init::init(path, cap),
        Commands::Expand {
            path,
            bin,
            lockfile,
        } => expand::expand(&path, bin, lockfile),
        Commands::Package {
            path,
            output,
            cargo_args,
        } => package::package(&path, output.as_deref(), &cargo_args),
        Commands::Clean { path } => clean::clean(&path),
        // Commands::Run {
        //     config,
        //     input,
        //     output_dir,
        //     format,
        // } => {
        //     let input_path = Path::new(&input);

        //     if input_path.exists() && input_path.is_file() {
        //         println!(
        //             "📂 Input file detected: {:?}. Running in Batch Mode...",
        //             input_path
        //         );
        //         run::run_batch(&config, input_path, &output_dir, format).await
        //     } else {
        //         run::run(&config, &input).await
        //     }
        // }
    }
}
