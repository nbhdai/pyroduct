use anyhow::{bail, Context, Result};
use fs_err as fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::utils::{InterfaceGenerator, ProjectContext, TarballBuilder};

use super::cargo::{CapabilityManifest, ModuleManifest};

fn get_target_dir(path: &Path) -> Result<PathBuf> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version=1", "--no-deps"])
        .current_dir(path)
        .output()
        .context("Failed to run cargo metadata")?;

    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)
        .context("Failed to parse cargo metadata")?;

    metadata["target_directory"]
        .as_str()
        .map(PathBuf::from)
        .context("No target_directory in cargo metadata")
}

fn dylib_extension() -> &'static str {
    if cfg!(target_os = "macos") { "dylib" }
    else if cfg!(target_os = "windows") { "dll" }
    else { "so" }
}

fn run_cargo_command(path: &Path, args: &[&str], error_ctx: &str) -> Result<()> {
    let status = Command::new("cargo")
        .args(args)
        .current_dir(path)
        .status()
        .context(error_ctx.to_string())?;

    if !status.success() {
        bail!("Cargo command failed: {} {:?}", status, args);
    }
    Ok(())
}

// ============================================================
// Module Packaging
// ============================================================

fn package_module(ctx: &ProjectContext, manifest: ModuleManifest) -> Result<()> {
    println!("Packaging module: {:?}", ctx.root);

    // 1. Generate Cargo.toml
    let cargo_toml_content = toml::to_string_pretty(&manifest.to_cargo())?;
    fs::write(ctx.root.join("Cargo.toml"), &cargo_toml_content)?;
    println!("✓ Wrote Cargo.toml");

    // 2. Build WASM
    println!("Compiling WASM module...");
    run_cargo_command(
        ctx.root,
        &["build", "--release", "--target", "wasm32-unknown-unknown", "-p", &ctx.name],
        "Failed to run cargo build"
    )?;

    // 3. Locate and Copy Artifact
    let target_dir = get_target_dir(ctx.root)?;
    let wasm_filename = format!("{}.wasm", ctx.normalized_name());
    let built_wasm = target_dir
        .join("wasm32-unknown-unknown/release")
        .join(&wasm_filename);

    if !built_wasm.exists() {
        bail!("Could not find compiled WASM: {}", built_wasm.display());
    }

    let dest_wasm = ctx.output_dir.join(&wasm_filename);
    fs::copy(&built_wasm, &dest_wasm)?;
    println!("✓ Compiled {}", dest_wasm.display());

    // 4. Create Archive
    let mut tar = TarballBuilder::new(ctx.archive_path("module"))?;
    tar.add_bytes("Cargo.toml", cargo_toml_content.as_bytes())?;
    tar.add_dir(&ctx.root.join("src"), "src")?;
    tar.finish()?;

    Ok(())
}

// ============================================================
// Capability Packaging
// ============================================================

fn package_capability(ctx: &ProjectContext, manifest: CapabilityManifest) -> Result<()> {
    println!("Packaging capability: {:?}", ctx.root);

    // 1. Generate Cargo.toml
    let cargo_toml_content = toml::to_string_pretty(&manifest.clone().to_capability_manifest())?;
    fs::write(ctx.root.join("Cargo.toml"), &cargo_toml_content)?;
    println!("✓ Wrote Cargo.toml");

    // 2. Build Dynamic Library
    println!("Compiling capability binary...");
    run_cargo_command(
        ctx.root,
        &["build", "--release", "--features", "capability", "-p", &ctx.name],
        "Failed to run cargo build"
    )?;

    // 3. Locate and Copy Artifact
    let target_dir = get_target_dir(ctx.root)?;
    let lib_filename = format!("lib{}.{}", ctx.normalized_name(), dylib_extension());
    let built_lib = target_dir.join("release").join(&lib_filename);

    if !built_lib.exists() {
        bail!("Could not find compiled binary: {}", built_lib.display());
    }

    let dest_lib = ctx.output_dir.join(&lib_filename);
    fs::copy(&built_lib, &dest_lib)?;
    println!("✓ Compiled {}", dest_lib.display());

    // 4. Create Source Archive (.cargo)
    let mut cap_tar = TarballBuilder::new(ctx.archive_path("cargo"))?;
    cap_tar.add_bytes("Cargo.toml", cargo_toml_content.as_bytes())?;
    cap_tar.add_dir(&ctx.root.join("src"), "src")?;

    // 5. Create Interface Archive (.interface)
    let mut interface_tar = TarballBuilder::new(ctx.archive_path("interface"))?;
    let interface = InterfaceGenerator::new(ctx.root, &manifest)?;
    interface.add_to_archive(&mut interface_tar)?;

    // 6. Add documentation
    interface_tar.add_bytes("interface.json", interface.spec().as_bytes())?;
    fs::write(ctx.root.join("interface.json"), interface.spec())?;

    // 7. Generate config spec
    if let Some(spec) = interface.config() {
        cap_tar.add_bytes("config.json", spec.as_bytes())?;
        fs::write(ctx.root.join("config.json"), spec)?;
    }

    interface_tar.finish()?;
    cap_tar.finish()?;
    Ok(())
}

// ============================================================
// Entry Points
// ============================================================

fn package_single(path: &Path, output: Option<&Path>) -> Result<()> {
    let output_dir = output.unwrap_or(path);
    fs::create_dir_all(output_dir)?;

    let cap_toml = path.join("Capability.toml");
    let mod_toml = path.join("Module.toml");

    if cap_toml.exists() && mod_toml.exists() {
        bail!("Both Capability.toml and Module.toml found in {:?}", path);
    }

    if cap_toml.exists() {
        let manifest: CapabilityManifest = toml::from_str(&fs::read_to_string(&cap_toml)?)?;
        let pkg = manifest.capability.as_ref().context("Package section missing in Capability.toml")?;
        let ctx = ProjectContext::new(path, output_dir, &pkg.name, pkg.version());
        package_capability(&ctx, manifest)
    } else if mod_toml.exists() {
        let manifest: ModuleManifest = toml::from_str(&fs::read_to_string(&mod_toml)?)?;
        let pkg = manifest.module.as_ref().context("Module section missing in Module.toml")?;
        let ctx = ProjectContext::new(path, output_dir, &pkg.name, pkg.version());
        package_module(&ctx, manifest)
    } else {
        bail!("Neither Capability.toml nor Module.toml found in {:?}", path)
    }
}

pub fn package(path: &Path, output: Option<&Path>) -> Result<()> {
    // 1. Direct package mode
    if path.join("Capability.toml").exists() || path.join("Module.toml").exists() {
        return package_single(path, output);
    }

    // 2. Recursive scan mode
    println!("No manifest found in {:?}, scanning subdirectories...", path);
    let mut errors = Vec::new();
    let mut found_any = false;

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let subpath = entry.path();
        
        if !subpath.is_dir() { continue; }

        if subpath.join("Capability.toml").exists() || subpath.join("Module.toml").exists() {
            found_any = true;
            if let Err(e) = package_single(&subpath, output) {
                errors.push((subpath, e));
            }
        }
    }

    if !found_any {
        bail!("No Capability.toml or Module.toml found in {:?} or subdirectories", path);
    }

    if !errors.is_empty() {
        eprintln!("\nErrors encountered:");
        for (p, e) in &errors {
            eprintln!("  {:?}: {:#}", p, e);
        }
        bail!("{} packaging(s) failed", errors.len());
    }

    Ok(())
}