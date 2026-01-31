use clap::Parser;
use std::path::PathBuf;
use anyhow::Result;
use capability_module::generate_client_code;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the input Rust file containing the capability definition
    #[arg(short, long)]
    input: PathBuf,

    /// Path where the generated client code should be written
    #[arg(short, long)]
    output: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("reading capability from: {:?}", args.input);
    
    // Use the logic from lib.rs
    let generated_code = generate_client_code(&args.input)?;

    // Write to output
    if let Some(parent) = args.output.parent() {
        fs_err::create_dir_all(parent)?;
    }
    fs_err::write(&args.output, generated_code)?;

    println!("successfully wrote client to: {:?}", args.output);
    
    Ok(())
}