use std::path::Path;
use anyhow::Result;

use fs_err as fs;

use crate::cli::cargo::{CapabilityManifest, ModuleManifest};
use crate::cli::utils::InterfaceGenerator;

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
            eprintln!("  {:?}: Failed to generate client code\n{}", path, err);
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
            let module_path = path.join("interface");
            let manifest_str = fs::read_to_string(&cap_toml_path)?;
            let cap_manifest: CapabilityManifest = toml::from_str(&manifest_str)?;

            let standard_manifest = cap_manifest.clone().to_capability_manifest();
            let output_str = toml::to_string_pretty(&standard_manifest)?;
            fs::write(&cargo_toml_path, output_str)?;
            println!("  ✓ Wrote Cargo.toml");

            generate_interface_crate(path, &module_path, cap_manifest)?;
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

fn generate_interface_crate(
    input: &Path, 
    output: &Path, 
    cap_manifest: CapabilityManifest
) -> Result<()> {
    let generator = InterfaceGenerator::new(input, &cap_manifest)?;
    generator.write_to_disk(output)?;
    
    Ok(())
}