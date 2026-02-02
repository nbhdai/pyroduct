use anyhow::Result;
use std::path::Path;

use fs_err as fs;

use crate::cargo::{CapabilityManifest, ModuleManifest};
use crate::utils::InterfaceGenerator;

use capability_core::generate_capability;
use module_core::generate_module;

pub fn expand(path: &Path, wat_mode: bool, lockfile: bool) -> Result<()> {
    let is_cap = path.join("Capability.toml").exists();
    let is_mod = path.join("Module.toml").exists();

    if (is_cap || is_mod) && !wat_mode {
        return expand_single(path, lockfile);
    }
    if is_mod && wat_mode {
        wat_project(path)?;
    }

    // No manifest found, try expanding subdirectories
    println!(
        "No manifest found in {:?}, scanning subdirectories...",
        path
    );

    let mut found_any = false;
    let mut errors = Vec::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let subpath = entry.path();

        if !subpath.is_dir() {
            continue;
        }

        let is_cap = subpath.join("Capability.toml").exists();
        let is_mod = subpath.join("Module.toml").exists();

        if (is_cap || is_mod) && !wat_mode {
            found_any = true;
            if let Err(e) = expand_single(&subpath, lockfile) {
                errors.push((subpath.clone(), e));
            }
        }
        if is_mod && wat_mode {
            match wat_project(&subpath) {
                Ok(true) => found_any = true,
                Ok(false) => {},
                Err(e) => errors.push((subpath, e)),
            }
        }
    }

    if !found_any && !wat_mode {
        anyhow::bail!(
            "No Capability.toml or Module.toml found in {:?} or its subdirectories",
            path
        );
    }

    if !found_any && wat_mode {
        anyhow::bail!(
            "No Module.toml found in {:?} or its subdirectories",
            path
        );
    }

    if !errors.is_empty() {
        eprintln!("\nErrors encountered:");
        for (path, err) in &errors {
            eprintln!("  {:?}: {}", path, err);
            let mut source = err.source();
            while let Some(cause) = source {
                eprintln!("    caused by: {}", cause);
                source = cause.source();
            }
        }
        anyhow::bail!("{} expansion(s) failed", errors.len());
    }

    Ok(())
}


fn wat_project(path: &Path) -> Result<bool> {
    let wasm_path = path.join("artifacts").join("mod.wasm");
    if wasm_path.exists() {
        wat(&wasm_path)?;
        println!("  ✓ Wrote artifacts/mod.wat");
        Ok(true)
    } else {
        println!("  x No WASM binary found at {:?}", wasm_path);
        Ok(false)
    }
}

fn expand_single(path: &Path, lockfile: bool) -> Result<()> {
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

            generate_capability_artifacts(path, &cap_manifest)?;
            generate_interface_crate(path, &module_path, cap_manifest, lockfile)?;
        }
        (false, true) => {
            let manifest_str = fs::read_to_string(&mod_toml_path)?;
            let mod_manifest: ModuleManifest = toml::from_str(&manifest_str)?;

            let standard_manifest = mod_manifest.to_cargo();
            let output_str = toml::to_string_pretty(&standard_manifest)?;
            generate_module_artifacts(path)?;
            fs::write(cargo_toml_path, output_str)?;

            let wasm_artifact_path = path.join("artifact").join("mod.wasm");
            if wasm_artifact_path.exists() {
                wat(&wasm_artifact_path)?;
            }

            println!("  ✓ Wrote Cargo.toml");
        }
        (false, false) => anyhow::bail!("Neither 'Capability.toml' nor 'Module.toml' found."),
    }

    Ok(())
}

fn generate_interface_crate(
    input: &Path,
    output: &Path,
    cap_manifest: CapabilityManifest,
    lockfile: bool,
) -> Result<()> {
    let generator = InterfaceGenerator::new(input, &cap_manifest)?;
    generator.write_to_disk(output, lockfile)?;

    Ok(())
}


fn generate_capability_artifacts(path: &Path, cap_manifest: &CapabilityManifest) -> Result<()> {
    let src_path = path.join("src/lib.rs");
    let artifacts_dir = path.join("artifacts");
    let output_path = artifacts_dir.join("capability.rs");

    if !src_path.exists() {
        anyhow::bail!("Source file not found: {:?}", src_path);
    }

    let content = fs::read_to_string(&src_path)?;
    let (cap_name, cap_version) = cap_manifest.name_version()?;

    let generated_code = generate_capability(&content, &cap_name, &cap_version)?;

    fs::create_dir_all(&artifacts_dir)?;
    fs::write(&output_path, generated_code)?;
    println!("  ✓ Wrote artifacts/capability.rs");

    Ok(())
}

fn generate_module_artifacts(path: &Path) -> Result<()> {
    let src_path = path.join("src/lib.rs");
    let artifacts_dir = path.join("artifacts");
    let output_path = artifacts_dir.join("module.rs");

    if !src_path.exists() {
        anyhow::bail!("Source file not found: {:?}", src_path);
    }

    let content = fs::read_to_string(&src_path)?;

    let generated_code = generate_module(&content)?;

    fs::create_dir_all(&artifacts_dir)?;
    fs::write(&output_path, generated_code)?;
    println!("  ✓ Wrote artifacts/module.rs");

    Ok(())
}


pub fn wat(input: &Path) -> anyhow::Result<()> {
    use std::io::Write;
    let output = input.with_extension("wat");

    if input.extension().and_then(|e| e.to_str()) != Some("wasm") {
        eprintln!(
            "Warning: Input file '{}' does not have .wasm extension",
            input.display()
        );
    }

    let wasm_bytes = fs_err::read(&input)?;

    let wat = wasmprinter::print_bytes(&wasm_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to convert WASM to WAT: {}", e))?;


    // Write the WAT output
    let mut file = fs_err::File::create(&output)?;
    file.write_all(wat.as_bytes())?;

    println!("Converted {} -> {}", input.display(), output.display());

    Ok(())
}