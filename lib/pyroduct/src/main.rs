mod cli;

use anyhow::Result;
use cli::cargo::CapabilityManifest;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use fs_err as fs;

use crate::cli::cargo::ModuleManifest;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generates the Cargo.toml for the Capability crate itself.
    /// Usage: pyroduct expand ./capability
    Expand {
        /// The root directory of the source capability
        // By leaving out 'long' and 'short', this becomes a positional argument
        #[arg(value_name = "DIRECTORY")] 
        path: PathBuf,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::Expand { path } => expand_capability(&path),
    }
}

// ----------------------------------------------------------------------------
// Command Logic
// ----------------------------------------------------------------------------

fn expand_capability(path: &Path) -> Result<()> {
    println!("Generating Capability Manifest for: {:?}", path);
    
    let cap_toml_path = path.join("Capability.toml");
    let mod_toml_path = path.join("Module.toml");
    let cargo_toml_path = path.join("Cargo.toml");
    match (cap_toml_path.exists(), mod_toml_path.exists()) {
        (true, true) => anyhow::bail!("Both 'Capability.toml' and 'Module.toml' found."),
        (true, false) => {
            println!("Expanding capability");
            let module_path = path.join("module");
            let manifest_str = fs::read_to_string(&cap_toml_path)?;
            let cap_manifest: CapabilityManifest = toml::from_str(&manifest_str)?;

            let standard_manifest = cap_manifest.to_capability_manifest();
            let output_str = toml::to_string_pretty(&standard_manifest)?;
            fs::write(cargo_toml_path, output_str)?;
            println!("✓ Wrote Cargo.toml");

            generate_module_crate(path, &module_path)?;
        },
        (false, true) => {
            println!("Expanding module");
            let manifest_str = fs::read_to_string(&cap_toml_path)?;
            let cap_manifest: ModuleManifest = toml::from_str(&manifest_str)?;

            let standard_manifest = cap_manifest.to_cargo();
            let output_str = toml::to_string_pretty(&standard_manifest)?;
            fs::write(cargo_toml_path, output_str)?;
            println!("✓ Wrote Cargo.toml");
        },
        (false, false) => anyhow::bail!("Neither 'Capability.toml' or 'Module.toml' found."),
    }
    

    

    
    Ok(())
}

fn generate_module_crate(input: &Path, output: &Path) -> Result<()> {
    println!("Generating Module Crate...");

    fs::create_dir_all(output)?;

    // 1. Generate Module Cargo.toml
    let cap_toml_path = input.join("Capability.toml");
    let manifest_str = fs::read_to_string(&cap_toml_path)?;
    let cap_manifest: CapabilityManifest = toml::from_str(&manifest_str)?;

    let module_manifest = cap_manifest.to_module_manifest();
    let cargo_out = toml::to_string_pretty(&module_manifest)?;
    fs::write(output.join("Cargo.toml"), cargo_out)?;
    println!("✓ Wrote module/Cargo.toml");

    // 2. Generate Module Source (lib.rs)
    let source_rs = input.join("src").join("lib.rs");
    let dest_rs = output.join("src").join("lib.rs");

    if !source_rs.exists() {
        anyhow::bail!("Source file not found: {:?}", source_rs);
    }

    let generator = cli::ModuleGenerator::new(source_rs);
    generator.generate_rust_source(dest_rs)?;
    println!("✓ Wrote module/src/lib.rs");

    Ok(())
}