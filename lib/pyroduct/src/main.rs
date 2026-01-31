mod cli;

use anyhow::Result;
use cli::cargo::CapabilityManifest;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use fs_err as fs;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generates the Cargo.toml for the Capability crate itself.
    /// Usage: pyroduct-gen manifest --root ./capability
    Manifest {
        /// The root directory containing Capability.toml
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    
    /// Generates the separate capability_module crate (Cargo.toml + src/lib.rs).
    /// Usage: pyroduct-gen module --input ./capability --output ./capability_module
    Module {
        /// The root directory of the source capability (must contain Capability.toml and src/lib.rs)
        #[arg(short, long)]
        input: PathBuf,

        /// The destination directory for the generated module
        #[arg(short, long)]
        output: PathBuf,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::Manifest { root } => generate_capability_manifest(&root),
        Commands::Module { input, output } => generate_module_crate(&input, &output),
    }
}

// ----------------------------------------------------------------------------
// Command Logic
// ----------------------------------------------------------------------------

fn generate_capability_manifest(root: &Path) -> Result<()> {
    println!("Generating Capability Manifest for: {:?}", root);
    
    let cap_toml_path = root.join("Capability.toml");
    let cargo_toml_path = root.join("Cargo.toml");

    let manifest_str = fs::read_to_string(&cap_toml_path)?;
    let cap_manifest: CapabilityManifest = toml::from_str(&manifest_str)?;

    let standard_manifest = cap_manifest.to_capability_manifest();
    let output_str = toml::to_string_pretty(&standard_manifest)?;

    fs::write(cargo_toml_path, output_str)?;
    println!("✓ Wrote Cargo.toml");
    Ok(())
}

fn generate_module_crate(input: &Path, output: &Path) -> Result<()> {
    println!("Generating Module Crate...");
    println!("  Input:  {:?}", input);
    println!("  Output: {:?}", output);

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