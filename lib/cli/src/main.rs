pub mod cargo;
pub mod package;
pub mod expand;
pub mod utils;
pub mod run;

use anyhow::{Result, anyhow};
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
    Expand {
        #[arg(value_name = "DIRECTORY")] 
        path: PathBuf,
    },
    
    /// Packages a module or capability into distributable archives.
    Package {
        #[arg(value_name = "DIRECTORY")]
        path: PathBuf,
        /// Output directory (defaults to input directory)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Runs the pipeline.
    /// Can run in single mode (via --json) or batch mode (via --input-file).
    Run {
        /// Path to the harness config TOML file
        #[arg(value_name = "CONFIG")]
        config: PathBuf,

        // --- Mode 1: Single Run ---
        
        /// Input data as a JSON string. 
        /// Mutually exclusive with --input-file.
        #[arg(long, group = "input_source")]
        json: Option<String>,

        // --- Mode 2: Batch Run ---

        /// Input file path for batch processing.
        /// Mutually exclusive with --json. Requires --output-dir.
        #[arg(long, group = "input_source", requires = "output_dir")]
        input_file: Option<PathBuf>,

        /// Output directory for batch processing.
        #[arg(long)]
        output_dir: Option<PathBuf>,

        /// Output format for batch processing.
        #[arg(long, value_enum, default_value_t = run::OutputFormat::Json)]
        format: run::OutputFormat,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::Expand { path } => expand::expand(&path),
        Commands::Package { path, output } => package::package(&path, output.as_deref()),
        
        Commands::Run { 
            config, 
            json, 
            input_file, 
            output_dir, 
            format 
        } => {
            if let Some(json_str) = json {
                run::run(&config, &json_str).await
            } else if let Some(input_path) = input_file {
                let output_path = output_dir.expect("Output directory is required for batch mode");
                
                run::run_batch(&config, &input_path, &output_path, format).await
            } else {
                Err(anyhow!("Please provide either --json for a single run or --input-file for a batch run."))
            }
        }
    }
}