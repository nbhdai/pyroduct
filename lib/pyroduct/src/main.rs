mod cli;

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
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::Expand { path } => cli::expand::expand(&path),
        Commands::Package { path, output } => cli::package::package(&path, output.as_deref()),
    }
}
