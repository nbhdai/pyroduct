// lib/pyroduct/src/cli/package.rs

use anyhow::{Context, Result};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::path::Path;
use std::process::Command;
use tar::Builder;
use fs_err as fs;

use crate::cli::expand::InterfaceGenerator;
use super::cargo::{CapabilityManifest, ModuleManifest};

// ============================================================
// Helpers
// ============================================================

fn get_target_dir(path: &Path) -> Result<String> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version=1", "--no-deps"])
        .current_dir(path)
        .output()
        .context("Failed to run cargo metadata")?;

    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)
        .context("Failed to parse cargo metadata")?;

    metadata["target_directory"]
        .as_str()
        .map(String::from)
        .context("No target_directory in cargo metadata")
}

fn lib_extension() -> &'static str {
    if cfg!(target_os = "macos") { "dylib" } 
    else if cfg!(target_os = "windows") { "dll" } 
    else { "so" }
}

fn generate_module_source(generator: &InterfaceGenerator) -> Result<String> {
    use capability_core::generate_client;
    let content = fs::read_to_string(&generator.source_path)
        .with_context(|| format!("Failed to read: {:?}", generator.source_path))?;
    generate_client(&content)
}

// ============================================================
// Tar helpers
// ============================================================

fn add_bytes_to_tar<W: std::io::Write>(
    tar: &mut Builder<W>,
    path: &str,
    data: &[u8],
) -> Result<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(data.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(&mut header, path, data)?;
    Ok(())
}

fn add_dir_to_tar<W: std::io::Write>(
    tar: &mut Builder<W>,
    dir: &Path,
    prefix: &str,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let archive_path = format!("{}/{}", prefix, name.to_string_lossy());

        if path.is_dir() {
            add_dir_to_tar(tar, &path, &archive_path)?;
        } else {
            let data = fs::read(&path)?;
            add_bytes_to_tar(tar, &archive_path, &data)?;
        }
    }
    Ok(())
}

// ============================================================
// Module packaging
// ============================================================

fn write_module_cargo_toml(path: &Path, manifest: ModuleManifest) -> Result<String> {
    let cargo_manifest = manifest.to_cargo();
    let content = toml::to_string_pretty(&cargo_manifest)?;
    fs::write(path.join("Cargo.toml"), &content)?;
    println!("✓ Wrote Cargo.toml");
    Ok(content)
}

fn compile_wasm_module(path: &Path, name: &str) -> Result<std::path::PathBuf> {
    println!("Compiling WASM module...");
    
    let status = Command::new("cargo")
        .args(["build", "--release", "--target", "wasm32-unknown-unknown", "-p", name])
        .current_dir(path)
        .status()
        .context("Failed to run cargo build")?;

    if !status.success() {
        anyhow::bail!("Cargo build failed with status: {}", status);
    }

    let target_dir = get_target_dir(path)?;
    let wasm_name = format!("{}.wasm", name.replace('-', "_"));
    let wasm_path = Path::new(&target_dir)
        .join("wasm32-unknown-unknown/release")
        .join(&wasm_name);

    if !wasm_path.exists() {
        anyhow::bail!("Could not find compiled WASM: {}", wasm_path.display());
    }

    Ok(wasm_path)
}

fn copy_wasm(wasm_path: &Path, output: &Path, name: &str) -> Result<()> {
    let wasm_name = format!("{}.wasm", name.replace('-', "_"));
    let output_path = output.join(&wasm_name);
    fs::copy(wasm_path, &output_path)?;
    println!("✓ Compiled {}", output_path.display());
    Ok(())
}

fn create_module_archive(
    path: &Path,
    output: &Path,
    name: &str,
    version: &str,
    cargo_toml_content: &str,
) -> Result<()> {
    let archive_name = format!("{}-{}.module", name, version);
    let archive_path = output.join(&archive_name);

    let file = fs::File::create(&archive_path)?;
    let enc = GzEncoder::new(file, Compression::default());
    let mut tar = Builder::new(enc);

    add_bytes_to_tar(&mut tar, "Cargo.toml", cargo_toml_content.as_bytes())?;

    let src_dir = path.join("src");
    if src_dir.exists() {
        add_dir_to_tar(&mut tar, &src_dir, "src")?;
    }

    tar.finish()?;
    println!("✓ Created {}", archive_path.display());
    Ok(())
}

fn package_module(path: &Path, output: &Path) -> Result<()> {
    println!("Packaging module: {:?}", path);

    let mod_toml_path = path.join("Module.toml");
    if !mod_toml_path.exists() {
        anyhow::bail!("Module.toml not found in {:?}", path);
    }

    let manifest_str = fs::read_to_string(&mod_toml_path)?;
    let manifest: ModuleManifest = toml::from_str(&manifest_str)?;

    let pkg = manifest.module.as_ref()
        .context("Module section required in Module.toml")?;
    let name = &pkg.name.clone();
    let version = pkg.version().to_string();

    let cargo_toml_content = write_module_cargo_toml(path, manifest)?;
    let wasm_path = compile_wasm_module(path, name)?;
    copy_wasm(&wasm_path, output, &name)?;

    create_module_archive(path, output, name, &version, &cargo_toml_content)?;

    Ok(())
}

// ============================================================
// Capability packaging
// ============================================================

fn write_capability_cargo_toml(path: &Path, manifest: &CapabilityManifest) -> Result<String> {
    let content = toml::to_string_pretty(&manifest.clone().to_capability_manifest())?;
    fs::write(path.join("Cargo.toml"), &content)?;
    println!("✓ Wrote Cargo.toml");
    Ok(content)
}

fn compile_capability_binary(path: &Path, name: &str) -> Result<std::path::PathBuf> {
    println!("Compiling capability binary...");
    
    let status = Command::new("cargo")
        .args(["build", "--release", "--features", "capability", "-p", name])
        .current_dir(path)
        .status()
        .context("Failed to run cargo build")?;

    if !status.success() {
        anyhow::bail!("Cargo build failed with status: {}", status);
    }

    let target_dir = get_target_dir(path)?;
    let lib_name = format!("lib{}.{}", name.replace('-', "_"), lib_extension());
    let binary_path = Path::new(&target_dir).join("release").join(&lib_name);

    if !binary_path.exists() {
        anyhow::bail!("Could not find compiled binary: {}", binary_path.display());
    }

    Ok(binary_path)
}

fn copy_capability_binary(binary_path: &Path, output: &Path, name: &str) -> Result<()> {
    let lib_name = format!("lib{}.{}", name.replace('-', "_"), lib_extension());
    let output_path = output.join(&lib_name);
    fs::copy(binary_path, &output_path)?;
    println!("✓ Compiled {}", output_path.display());
    Ok(())
}

fn create_cargo_archive(
    path: &Path,
    output: &Path,
    name: &str,
    version: &str,
    cargo_toml_content: &str,
) -> Result<()> {
    let archive_name = format!("{}-{}.cargo", name, version);
    let archive_path = output.join(&archive_name);

    let file = fs::File::create(&archive_path)?;
    let enc = GzEncoder::new(file, Compression::default());
    let mut tar = Builder::new(enc);

    add_bytes_to_tar(&mut tar, "Cargo.toml", cargo_toml_content.as_bytes())?;

    let src_dir = path.join("src");
    if src_dir.exists() {
        add_dir_to_tar(&mut tar, &src_dir, "src")?;
    }

    tar.finish()?;
    println!("✓ Created {}", archive_path.display());
    Ok(())
}

fn generate_rustdoc_json(path: &Path, name: &str) -> Result<Option<Vec<u8>>> {
    println!("Generating rustdoc JSON...");
    
    let module_path = path.join("module");
    if !module_path.exists() {
        println!("  Skipping: no module/ directory");
        return Ok(None);
    }

    let module_name = format!("{}-module", name.replace('_', "-"));
    
    let output = Command::new("cargo")
        .args([
            "+nightly",
            "rustdoc",
            "-p", &module_name,
            "--",
            "-Z", "unstable-options", 
            "--output-format", "json",
        ])
        .current_dir(path)
        .output()
        .context("Failed to run cargo rustdoc (is nightly installed?)")?;

    if !output.status.success() {
        eprintln!("Warning: rustdoc JSON generation failed");
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        return Ok(None);
    }

    let target_dir = get_target_dir(path)?;
    let json_name = format!("{}.json", module_name.replace('-', "_"));
    let json_path = Path::new(&target_dir).join("doc").join(&json_name);

    if json_path.exists() {
        let content = fs::read(&json_path)?;
        println!("✓ Generated interface.json ({} bytes)", content.len());
        Ok(Some(content))
    } else {
        eprintln!("Warning: expected {} not found", json_path.display());
        Ok(None)
    }
}

fn create_capability_archive(
    path: &Path,
    output: &Path,
    name: &str,
    version: &str,
    manifest: &CapabilityManifest,
) -> Result<()> {
    let module_manifest = manifest.clone().to_interface_manifest();
    let module_cargo_content = toml::to_string_pretty(&module_manifest)?;

    let archive_name = format!("{}-{}.capability", name, version);
    let archive_path = output.join(&archive_name);

    let file = fs::File::create(&archive_path)?;
    let enc = GzEncoder::new(file, Compression::default());
    let mut tar = Builder::new(enc);

    add_bytes_to_tar(&mut tar, "Cargo.toml", module_cargo_content.as_bytes())?;

    let source_rs = path.join("src").join("lib.rs");
    if source_rs.exists() {
        let generator = InterfaceGenerator::new(&source_rs);
        let generated_code = generate_module_source(&generator)?;
        add_bytes_to_tar(&mut tar, "src/lib.rs", generated_code.as_bytes())?;
    }

    if let Some(json_bytes) = generate_rustdoc_json(path, name)? {
        add_bytes_to_tar(&mut tar, "interface.json", &json_bytes)?;
    }

    tar.finish()?;
    println!("✓ Created {}", archive_path.display());
    Ok(())
}

fn package_capability(path: &Path, output: &Path) -> Result<()> {
    println!("Packaging capability: {:?}", path);

    let cap_toml_path = path.join("Capability.toml");
    if !cap_toml_path.exists() {
        anyhow::bail!("Capability.toml not found in {:?}", path);
    }

    let manifest_str = fs::read_to_string(&cap_toml_path)?;
    let manifest: CapabilityManifest = toml::from_str(&manifest_str)?;

    let pkg = manifest.capability.as_ref()
        .context("Package section required in Capability.toml")?;
    let name = &pkg.name;
    let version = pkg.version();

    let cargo_toml_content = write_capability_cargo_toml(path, &manifest)?;
    let binary_path = compile_capability_binary(path, name)?;
    
    copy_capability_binary(&binary_path, output, name)?;
    create_cargo_archive(path, output, name, version, &cargo_toml_content)?;
    create_capability_archive(path, output, name, version, &manifest)?;

    Ok(())
}

// ============================================================
// Entry points
// ============================================================

fn package_single(path: &Path, output: Option<&Path>) -> Result<()> {
    let cap_toml_path = path.join("Capability.toml");
    let mod_toml_path = path.join("Module.toml");

    let output_dir = output.unwrap_or(path);
    fs::create_dir_all(output_dir)?;

    match (cap_toml_path.exists(), mod_toml_path.exists()) {
        (true, true) => anyhow::bail!("Both Capability.toml and Module.toml found"),
        (true, false) => package_capability(path, output_dir),
        (false, true) => package_module(path, output_dir),
        (false, false) => anyhow::bail!("Neither Capability.toml nor Module.toml found"),
    }
}

pub fn package(path: &Path, output: Option<&Path>) -> Result<()> {
    let cap_toml_path = path.join("Capability.toml");
    let mod_toml_path = path.join("Module.toml");

    if cap_toml_path.exists() || mod_toml_path.exists() {
        return package_single(path, output);
    }

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
            if let Err(e) = package_single(&subpath, output) {
                errors.push((subpath, e));
            }
        }
    }

    if !found_any {
        anyhow::bail!(
            "No Capability.toml or Module.toml found in {:?} or its subdirectories", 
            path
        );
    }

    if !errors.is_empty() {
        eprintln!("\nErrors encountered:");
        for (path, err) in &errors {
            eprintln!("  {:?}: {}", path, err);
        }
        anyhow::bail!("{} packaging(s) failed", errors.len());
    }

    Ok(())
}