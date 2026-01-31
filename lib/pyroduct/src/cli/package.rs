use anyhow::{Context, Result};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::path::Path;
use std::process::Command;
use tar::Builder;
use fs_err as fs;

use crate::cli::expand::ModuleGenerator;

use super::cargo::{CapabilityManifest, ModuleManifest};

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

/// Packages a module directory into a .module archive and compiles the WASM binary.
pub fn package_module(path: &Path, output: &Path) -> Result<()> {
    println!("Packaging module: {:?}", path);

    let mod_toml_path = path.join("Module.toml");
    if !mod_toml_path.exists() {
        anyhow::bail!("Module.toml not found in {:?}", path);
    }

    let manifest_str = fs::read_to_string(&mod_toml_path)?;
    let mod_manifest: ModuleManifest = toml::from_str(&manifest_str)?;
    let cargo_manifest = mod_manifest.to_cargo();
    let cargo_toml_content = toml::to_string_pretty(&cargo_manifest)?;

    let pkg = cargo_manifest.package.as_ref()
        .context("Package section required in Module.toml")?;
    let name = &pkg.name;
    let version = pkg.version();

    // Write expanded Cargo.toml
    let cargo_toml_path = path.join("Cargo.toml");
    fs::write(&cargo_toml_path, &cargo_toml_content)?;
    println!("✓ Wrote Cargo.toml");

    // Compile WASM
    println!("Compiling WASM module...");
    let status = Command::new("cargo")
        .args(["build", "--release", "--target", "wasm32-unknown-unknown", "-p", name])
        .current_dir(path)
        .status()
        .context("Failed to run cargo build")?;

    if !status.success() {
        anyhow::bail!("Cargo build failed with status: {}", status);
    }

    // Find and copy the WASM binary
    let target_dir = get_target_dir(path)?;
    let wasm_name = format!("{}.wasm", name.replace('-', "_"));
    let wasm_path = Path::new(&target_dir)
        .join("wasm32-unknown-unknown/release")
        .join(&wasm_name);

    if !wasm_path.exists() {
        anyhow::bail!("Could not find compiled WASM: {}", wasm_path.display());
    }

    let output_wasm = output.join(&wasm_name);
    fs::copy(&wasm_path, &output_wasm)?;
    println!("✓ Compiled {}", output_wasm.display());

    // Create .module archive
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

/// Packages a capability directory into both .cargo and .capability archives,
/// and compiles the host plugin binary.
pub fn package_capability(path: &Path, output: &Path) -> Result<()> {
    println!("Packaging capability: {:?}", path);

    let cap_toml_path = path.join("Capability.toml");
    if !cap_toml_path.exists() {
        anyhow::bail!("Capability.toml not found in {:?}", path);
    }

    let manifest_str = fs::read_to_string(&cap_toml_path)?;
    let cap_manifest: CapabilityManifest = toml::from_str(&manifest_str)?;

    let pkg = cap_manifest.capability.as_ref()
        .context("Package section required in Capability.toml")?;
    let name = &pkg.name;
    let version = pkg.version();

    // 1. Write expanded Cargo.toml to the directory for compilation
    let cargo_manifest = cap_manifest.clone().to_capability_manifest();
    let cargo_toml_content = toml::to_string_pretty(&cargo_manifest)?;
    let cargo_toml_path = path.join("Cargo.toml");
    fs::write(&cargo_toml_path, &cargo_toml_content)?;
    println!("✓ Wrote Cargo.toml");

    // 2. Compile the capability binary with --features capability
    println!("Compiling capability binary...");
    let status = Command::new("cargo")
        .args(["build", "--release", "--features", "capability", "-p", name])
        .current_dir(path)
        .status()
        .context("Failed to run cargo build")?;

    if !status.success() {
        anyhow::bail!("Cargo build failed with status: {}", status);
    }

    // Find the compiled binary
    let target_dir = get_target_dir(path)?;
    let lib_ext = if cfg!(target_os = "macos") { "dylib" } else if cfg!(target_os = "windows") { "dll" } else { "so" };
    let lib_name = format!("lib{}.{}", name.replace('-', "_"), lib_ext);
    let binary_path = Path::new(&target_dir).join("release").join(&lib_name);

    if !binary_path.exists() {
        anyhow::bail!("Could not find compiled binary: {}", binary_path.display());
    }

    // Copy binary to output
    let output_binary = output.join(&lib_name);
    fs::copy(&binary_path, &output_binary)?;
    println!("✓ Compiled {}", output_binary.display());

    // 3. Create .cargo archive (original code + expanded Cargo.toml)
    let cargo_archive_name = format!("{}-{}.cargo", name, version);
    let cargo_archive_path = output.join(&cargo_archive_name);

    let file = fs::File::create(&cargo_archive_path)?;
    let enc = GzEncoder::new(file, Compression::default());
    let mut tar = Builder::new(enc);

    add_bytes_to_tar(&mut tar, "Cargo.toml", cargo_toml_content.as_bytes())?;

    let src_dir = path.join("src");
    if src_dir.exists() {
        add_dir_to_tar(&mut tar, &src_dir, "src")?;
    }

    tar.finish()?;
    println!("✓ Created {}", cargo_archive_path.display());

    // 4. Create .capability archive (module directory contents)
    let module_manifest = cap_manifest.clone().to_module_manifest();
    let module_cargo_content = toml::to_string_pretty(&module_manifest)?;

    let cap_archive_name = format!("{}-{}.capability", name, version);
    let cap_archive_path = output.join(&cap_archive_name);

    let file = fs::File::create(&cap_archive_path)?;
    let enc = GzEncoder::new(file, Compression::default());
    let mut tar = Builder::new(enc);

    add_bytes_to_tar(&mut tar, "Cargo.toml", module_cargo_content.as_bytes())?;

    let source_rs = path.join("src").join("lib.rs");
    if source_rs.exists() {
        let generator = ModuleGenerator::new(&source_rs);
        let generated_code = generate_module_source(&generator)?;
        add_bytes_to_tar(&mut tar, "src/lib.rs", generated_code.as_bytes())?;
    }

    tar.finish()?;
    println!("✓ Created {}", cap_archive_path.display());

    Ok(())
}

/// Main entry point for the package command
pub fn package(path: &Path, output: Option<&Path>) -> Result<()> {
    let cap_toml_path = path.join("Capability.toml");
    let mod_toml_path = path.join("Module.toml");

    if cap_toml_path.exists() || mod_toml_path.exists() {
        return package_single(path, output);
    }

    // No manifest found, try packaging subdirectories
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
        anyhow::bail!("No Capability.toml or Module.toml found in {:?} or its subdirectories", path);
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

fn package_single(path: &Path, output: Option<&Path>) -> Result<()> {
    let cap_toml_path = path.join("Capability.toml");
    let mod_toml_path = path.join("Module.toml");

    let output_dir = output.unwrap_or(path);
    fs::create_dir_all(output_dir)?;

    match (cap_toml_path.exists(), mod_toml_path.exists()) {
        (true, true) => anyhow::bail!("Both 'Capability.toml' and 'Module.toml' found."),
        (true, false) => package_capability(path, output_dir),
        (false, true) => package_module(path, output_dir),
        (false, false) => anyhow::bail!("Neither 'Capability.toml' nor 'Module.toml' found."),
    }
}

fn generate_module_source(generator: &ModuleGenerator) -> Result<String> {
    use capability_core::generate_client;
    let content = fs::read_to_string(&generator.source_path)
        .with_context(|| format!("Failed to read: {:?}", generator.source_path))?;
    generate_client(&content)
}

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