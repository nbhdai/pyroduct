use anyhow::{Context, Result, bail};
use fs_err as fs;
use std::path::Path;

use crate::artifacts::cargo::{CapabilityManifest, ModuleManifest};
use crate::artifacts::utils::dylib_extension;
use crate::cache::CacheManager;

fn ship_capability(path: &Path, manifest: CapabilityManifest) -> Result<()> {
    let pkg = manifest
        .capability
        .as_ref()
        .context("Missing capability metadata")?;
    let author = pkg
        .authors
        .get()
        .ok()
        .and_then(|a| a.first().map(|s| s.as_str()))
        .unwrap_or("unknown");
    let name = &pkg.name;
    let version = pkg.version();

    let cache = CacheManager::new()?;
    let dest_dir = cache.capabilities_dir(author, name, version);
    fs::create_dir_all(&dest_dir)?;

    let artifacts_dir = path.join("artifacts");
    if !artifacts_dir.exists() {
        bail!("No artifacts directory found. Run `cargo run -- package .` first.");
    }

    let lib_name = format!("lib.{}", dylib_extension());
    let built_lib = artifacts_dir.join(&lib_name);
    let cargo_tar = artifacts_dir.join(format!("{}-{}-cargo.tar.gz", name, version));
    let interface_tar = artifacts_dir.join(format!("{}-{}-interface.tar.gz", name, version));

    if !built_lib.exists() || !cargo_tar.exists() || !interface_tar.exists() {
        bail!("Missing built capability artifacts. Run `cargo run -- package .` first.");
    }

    fs::copy(&built_lib, dest_dir.join(lib_name))?;
    fs::copy(&cargo_tar, dest_dir.join(cargo_tar.file_name().unwrap()))?;
    fs::copy(
        &interface_tar,
        dest_dir.join(interface_tar.file_name().unwrap()),
    )?;

    // Ideally, extract the interface to the global interfaces dir if needed for local usage
    let interface_name_version = format!("{}_{}", name, version);
    let global_interface_dir = cache.interfaces_dir().join(&interface_name_version);
    fs::create_dir_all(&global_interface_dir)?;

    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(fs::File::open(
        &interface_tar,
    )?));
    archive.unpack(&global_interface_dir)?;

    tracing::info!("✓ Shipped capability to {}", dest_dir.display());
    Ok(())
}

fn ship_module(path: &Path, manifest: ModuleManifest) -> Result<()> {
    let pkg = manifest
        .module
        .as_ref()
        .context("Missing module metadata")?;
    let author = pkg
        .authors
        .get()
        .ok()
        .and_then(|a| a.first().map(|s| s.as_str()))
        .unwrap_or("unknown");
    let name = &pkg.name;
    let version = pkg.version();

    let cache = CacheManager::new()?;
    let dest_dir = cache.module_dir(author, name, version)?;
    fs::create_dir_all(&dest_dir)?;

    let artifacts_dir = path.join("artifacts");
    if !artifacts_dir.exists() {
        bail!("No artifacts directory found. Run `cargo run -- package .` first.");
    }

    let wasm_file = artifacts_dir.join("mod.wasm");
    let module_tar = artifacts_dir.join(format!("{}-{}-module.tar.gz", name, version));

    if !wasm_file.exists() || !module_tar.exists() {
        bail!("Missing built module artifacts. Run `cargo run -- package .` first.");
    }

    fs::copy(&wasm_file, dest_dir.join("mod.wasm"))?;
    fs::copy(&module_tar, dest_dir.join(module_tar.file_name().unwrap()))?;

    tracing::info!("✓ Shipped module to {}", dest_dir.display());
    Ok(())
}

fn ship_single(path: &Path) -> Result<()> {
    let cap_toml = path.join("Capability.toml");
    let mod_toml = path.join("Module.toml");

    if cap_toml.exists() && mod_toml.exists() {
        bail!("Both Capability.toml and Module.toml found in {:?}", path);
    }

    if cap_toml.exists() {
        let manifest: CapabilityManifest = toml::from_str(&fs::read_to_string(&cap_toml)?)?;
        ship_capability(path, manifest)
    } else if mod_toml.exists() {
        let manifest: ModuleManifest = toml::from_str(&fs::read_to_string(&mod_toml)?)?;
        ship_module(path, manifest)
    } else {
        bail!(
            "Neither Capability.toml nor Module.toml found in {:?}",
            path
        )
    }
}

pub fn ship(path: &Path) -> Result<()> {
    if path.join("Capability.toml").exists() || path.join("Module.toml").exists() {
        return ship_single(path);
    }

    let mut errors = Vec::new();
    let mut found_any = false;

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let subpath = entry.path();

        if !subpath.is_dir() {
            continue;
        }

        if subpath.join("Capability.toml").exists() || subpath.join("Module.toml").exists() {
            found_any = true;
            if let Err(e) = ship_single(&subpath) {
                errors.push((subpath, e));
            }
        }
    }

    if !found_any {
        bail!(
            "No Capability.toml or Module.toml found in {:?} or subdirectories",
            path
        );
    }

    if !errors.is_empty() {
        let mut err_msg = String::from("\nErrors encountered:\n");
        for (p, e) in &errors {
            err_msg.push_str(&format!("  {:?}: {:#}\n", p, e));
        }

        tracing::error!("{}", err_msg);
        bail!("{} shipping(s) failed. {}", errors.len(), err_msg);
    }

    Ok(())
}
