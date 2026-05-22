use anyhow::{Result, bail};
use fs_err as fs;
use pyro_artifacts::{artifacts::Artifact, cache::CacheManager, environment::Environment};
use std::path::Path;

pub async fn ship_single(
    cache: std::sync::Arc<CacheManager>,
    path: &Path,
    debug: bool,
    out: Option<&Path>,
) -> Result<()> {
    let env = Environment::new(path.to_path_buf(), cache.clone()).await?;

    let artifacts = env.package(false).await?;
    for artifact in &artifacts {
        if let Some(out) = out {
            artifact.write_to_directory(out).await?;
        } else {
            cache.write_artifacts(artifact).await?;
        }
    }
    if debug {
        env.expand_debug().await?;
    }

    Ok(())
}

pub async fn ship(path: &Path, debug: bool, out: Option<&Path>) -> Result<()> {
    let is_cap = path.join("Capability.toml").exists();
    let is_mod = path.join("Module.toml").exists();

    let cache = std::sync::Arc::new(CacheManager::from_env().await?);

    // 1. Direct package mode
    if is_cap || is_mod {
        return ship_single(cache.clone(), path, debug, out).await;
    }

    let mut errors = Vec::new();
    let mut found_any = false;

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let subpath = entry.path();

        if !subpath.is_dir() {
            continue;
        }
        let is_cap = subpath.join("Capability.toml").exists();
        let is_mod = subpath.join("Module.toml").exists();

        if is_cap || is_mod {
            found_any = true;
            match ship_single(cache.clone(), &subpath, debug, out).await {
                Ok(()) => {}
                Err(e) => errors.push((subpath, e)),
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
        bail!("{} packaging(s) failed. {}", errors.len(), err_msg);
    }

    Ok(())
}
