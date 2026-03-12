use anyhow::{Context, Result, bail};
use fs_err as fs;
use std::path::Path;

use crate::artifacts::cargo::{CapabilityManifest, ModuleManifest};
use crate::artifacts::package::PackageResult;
use crate::artifacts::utils::extract_tarball;
use crate::cache::CacheManager;

fn ship_capability(manifest: CapabilityManifest, result: &PackageResult) -> Result<()> {
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

    for artifact in &result.artifacts {
        let dest_path = dest_dir.join(&artifact.name);
        fs::write(&dest_path, &artifact.data)?;
        tracing::info!("✓ Saved {} to cache", artifact.name);

        if artifact.name.ends_with(".interface") {
            let interface_dir = cache.capability_interface_dir(author, name, version);
            if interface_dir.exists() {
                fs::remove_dir_all(&interface_dir)?;
            }
            extract_tarball(&artifact.data, &interface_dir)?;

            // Also extract to the global interfaces dir for backwards compatibility
            let interface_name_version = format!("{}_{}", name, version);
            let global_interface_dir = cache.interfaces_dir().join(&interface_name_version);
            extract_tarball(&artifact.data, &global_interface_dir)?;
        } else if artifact.name.ends_with(".cap") {
            extract_tarball(&artifact.data, &dest_dir)?;
        }
    }

    tracing::info!("✓ Shipped capability to {}", dest_dir.display());
    Ok(())
}

fn ship_module(manifest: ModuleManifest, result: &PackageResult) -> Result<()> {
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

    for artifact in &result.artifacts {
        let dest_path = dest_dir.join(&artifact.name);
        fs::write(&dest_path, &artifact.data)?;
        tracing::info!("✓ Saved {} to cache", artifact.name);

        if artifact.name.ends_with(".module") {
            extract_tarball(&artifact.data, &dest_dir)?;
        }
    }

    tracing::info!("✓ Shipped module to {}", dest_dir.display());
    Ok(())
}

fn ship_single(path: &Path, results: &[PackageResult]) -> Result<()> {
    let cap_toml = path.join("Capability.toml");
    let mod_toml = path.join("Module.toml");

    if cap_toml.exists() && mod_toml.exists() {
        bail!("Both Capability.toml and Module.toml found in {:?}", path);
    }

    let result = results.first().context("No package results found")?;

    if cap_toml.exists() {
        let manifest: CapabilityManifest = toml::from_str(&fs::read_to_string(&cap_toml)?)?;
        ship_capability(manifest, result)
    } else if mod_toml.exists() {
        let manifest: ModuleManifest = toml::from_str(&fs::read_to_string(&mod_toml)?)?;
        ship_module(manifest, result)
    } else {
        bail!(
            "Neither Capability.toml nor Module.toml found in {:?}",
            path
        )
    }
}

pub fn ship(path: &Path) -> Result<()> {
    if path.join("Capability.toml").exists() || path.join("Module.toml").exists() {
        let results = crate::artifacts::package::package_single(path, None, false)?;
        return ship_single(path, &results);
    }

    let mut found_any = false;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let subpath = entry.path();

        if !subpath.is_dir() {
            continue;
        }

        if subpath.join("Capability.toml").exists() || subpath.join("Module.toml").exists() {
            let results = crate::artifacts::package::package_single(&subpath, None, false)?;
            found_any = true;
            ship_single(&subpath, &results)?;
        }
    }

    if !found_any {
        bail!(
            "No Capability.toml or Module.toml found in {:?} or subdirectories",
            path
        );
    }

    Ok(())
}
