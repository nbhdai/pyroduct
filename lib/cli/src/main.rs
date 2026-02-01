pub mod cargo;
pub mod package;
pub mod expand;
pub mod utils;
pub mod run;

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
    /// Generates the Cargo.toml for the Capability crate itself.
    /// If no manifest is found, attempts to expand each subdirectory.
    /// Usage: pyroduct expand ./capability
    Expand {
        #[arg(value_name = "DIRECTORY")] 
        path: PathBuf,
    },
    /// Packages a module or capability into distributable archives.
    /// For modules: creates a .module archive and .wasm binary
    /// For capabilities: creates .cargo, .capability archives and .so/.dylib binary
    Package {
        #[arg(value_name = "DIRECTORY")]
        path: PathBuf,
        /// Output directory (defaults to input directory)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Runs the pipeline with the given config against the given data
    Run {
        /// Path to the harness config TOML file
        #[arg(value_name = "CONFIG")]
        config: PathBuf,

        /// Input data as JSON string, or a file path
        #[arg(short, long)]
        input: Option<String>,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::Expand { path } => expand::expand(&path),
        Commands::Package { path, output } => package::package(&path, output.as_deref()),
        Commands::Run { config, input } => run::run(&config, input.as_ref().map(|s| s.as_str())).await,
    }
}
