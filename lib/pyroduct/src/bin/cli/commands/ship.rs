use anyhow::{Result, bail};
use pyro_artifacts::{artifacts::{Artifacts, Module}, cache::CacheManager, environment::Environment};
use fs_err as fs;
use std::path::Path;

pub async fn ship_single(cache: &CacheManager, path: &Path, debug: bool) -> Result<()> {
    let env = Environment::new(path.to_path_buf()).await?;
    if let Some(interface_artifact) = env.create_interface().await? {
        cache.write_artifacts(&interface_artifact.into()).await?;
    }

    let artifacts = env.package(false).await?;
    for artifact in &artifacts {
        cache.write_artifacts(artifact).await?;
    }
    if debug {
        for artifact in &artifacts {
            match artifact {
                Artifacts::CapabilitySource(capability) => {
                    let _ = cache
                        .debug_capabilities(
                            &capability.manifest.capability.author,
                            &capability.manifest.capability.name,
                            &capability.manifest.capability.version,
                        )
                        .await;
                }
                Artifacts::Module(Module::Source(source)) => {
                    let _ = cache.debug_module(&source.hash()).await;
                }
                _ => {}
            }
        }
    }

    Ok(())
}

pub async fn ship(path: &Path, debug: bool) -> Result<()> {
    let is_cap = path.join("Capability.toml").exists();
    let is_mod = path.join("Module.toml").exists();
    let cache = CacheManager::from_env().await?;
    // 1. Direct package mode
    if is_cap || is_mod {
        return ship_single(&cache, path, debug).await;
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
            match ship_single(&cache, &subpath, debug).await {
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
