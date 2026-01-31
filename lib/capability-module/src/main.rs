use clap::Parser;
use std::path::PathBuf;
use anyhow::Result;
use capability_module::ClientGenerator;

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

    println!("Reading capability from: {:?}", args.input);
    if !args.output.is_dir() {
        anyhow::bail!("Output needs to be a directory");
    }
    if let Some(parent) = args.output.parent() {
        fs_err::create_dir_all(parent)?;
    }
    
    let output = ClientGenerator::new(&args.input)
        .out_dir(&args.output)
        .generate("lib.rs")?;

    // Write to output
    println!("Successfully wrote client to: {:?}", output);
    
    Ok(())
}