use std::path::{Path, PathBuf};
use std::io::Write;
use anyhow::{Context, Result};

use capability_core::generate_client;
use fs_err as fs;

use crate::cli::cargo::{CapabilityManifest, ModuleManifest};

pub struct ModuleGenerator {
    pub source_path: PathBuf,
}

impl ModuleGenerator {
    pub fn new(source_path: impl AsRef<Path>) -> Self {
        Self {
            source_path: source_path.as_ref().to_path_buf(),
        }
    }

    /// Generates the Rust client code and writes it to the destination path.
    pub fn generate_rust_source(&self, dest_path: impl AsRef<Path>) -> Result<()> {
        let content = std::fs::read_to_string(&self.source_path)
            .with_context(|| format!("Failed to read capability source: {:?}", self.source_path))?;
        let generated_code = generate_client(&content)?;
        let dest = dest_path.as_ref();
        if let Some(parent) = dest.parent() {
            fs_err::create_dir_all(parent)?;
        }

        let mut out_file = fs_err::File::create(dest)
            .with_context(|| format!("Failed to create output file: {:?}", dest))?;
        
        out_file.write_all(generated_code.as_bytes())?;
        
        let _ = std::process::Command::new("rustfmt").arg(dest).status();

        Ok(())
    }
}

pub fn expand(path: &Path) -> Result<()> {
    let cap_toml = path.join("Capability.toml");
    let mod_toml = path.join("Module.toml");

    if cap_toml.exists() || mod_toml.exists() {
        return expand_single(path);
    }

    // No manifest found, try expanding subdirectories
    println!("No manifest found in {:?}, scanning subdirectories...", path);
    
    let mut found_any = false;
    let mut errors = Vec::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let subpath = entry.path();
        
        if !subpath.is_dir() {
            continue;
        }

        let sub_cap = subpath.join("Capability.toml");
        let sub_mod = subpath.join("Module.toml");

        if sub_cap.exists() || sub_mod.exists() {
            found_any = true;
            if let Err(e) = expand_single(&subpath) {
                errors.push((subpath, e));
            }
        }
    }

    if !found_any {
        anyhow::bail!("No Capability.toml or Module.toml found in {:?} or its subdirectories", path);
    }

    if !errors.is_empty() {
        eprintln!("\nErrors encountered:");
        for (path, err) in &errors {
            eprintln!("  {:?}: {}", path, err);
        }
        anyhow::bail!("{} expansion(s) failed", errors.len());
    }

    Ok(())
}

fn expand_single(path: &Path) -> Result<()> {
    println!("Expanding: {:?}", path);
    
    let cap_toml_path = path.join("Capability.toml");
    let mod_toml_path = path.join("Module.toml");
    let cargo_toml_path = path.join("Cargo.toml");

    match (cap_toml_path.exists(), mod_toml_path.exists()) {
        (true, true) => anyhow::bail!("Both 'Capability.toml' and 'Module.toml' found."),
        (true, false) => {
            let module_path = path.join("module");
            let manifest_str = fs::read_to_string(&cap_toml_path)?;
            let cap_manifest: CapabilityManifest = toml::from_str(&manifest_str)?;

            let standard_manifest = cap_manifest.clone().to_capability_manifest();
            let output_str = toml::to_string_pretty(&standard_manifest)?;
            fs::write(&cargo_toml_path, output_str)?;
            println!("  ✓ Wrote Cargo.toml");

            generate_module_crate(path, &module_path, cap_manifest)?;
        },
        (false, true) => {
            let manifest_str = fs::read_to_string(&mod_toml_path)?;
            let mod_manifest: ModuleManifest = toml::from_str(&manifest_str)?;

            let standard_manifest = mod_manifest.to_cargo();
            let output_str = toml::to_string_pretty(&standard_manifest)?;
            fs::write(cargo_toml_path, output_str)?;
            println!("  ✓ Wrote Cargo.toml");
        },
        (false, false) => anyhow::bail!("Neither 'Capability.toml' nor 'Module.toml' found."),
    }

    Ok(())
}

fn generate_module_crate(input: &Path, output: &Path, cap_manifest: CapabilityManifest) -> Result<()> {
    fs::create_dir_all(output)?;

    let module_manifest = cap_manifest.to_module_manifest();
    let cargo_out = toml::to_string_pretty(&module_manifest)?;
    fs::write(output.join("Cargo.toml"), cargo_out)?;
    println!("  ✓ Wrote module/Cargo.toml");

    let source_rs = input.join("src").join("lib.rs");
    let dest_rs = output.join("src").join("lib.rs");

    if !source_rs.exists() {
        anyhow::bail!("Source file not found: {:?}", source_rs);
    }

    let generator = ModuleGenerator::new(source_rs);
    generator.generate_rust_source(dest_rs)?;
    println!("  ✓ Wrote module/src/lib.rs");

    Ok(())
}