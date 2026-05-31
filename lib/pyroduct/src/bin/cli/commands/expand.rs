use anyhow::{Context, Result};
use fs_err as fs;
use pyro_artifacts::{
    artifacts::{Artifact, Artifacts, Playbook},
    cache::CacheManager,
    debug::CapSymbols,
    environment::Environment,
};
use pyro_macro::{ffi::generate_capability, module::generate_module};
use std::path::Path;

pub async fn expand(path: &Path, no_compile: bool) -> Result<()> {
    let is_cap = path.join("Capability.toml").exists();
    let is_mod = path.join("Module.toml").exists();
    if is_cap || is_mod {
        expand_single(path, no_compile).await?;
        return Ok(());
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
        if !is_cap && !is_mod {
            continue;
        }
        match expand_single(&subpath, no_compile).await {
            Ok(true) => found_any = true,
            Ok(false) => {}
            Err(error) => errors.push((subpath, error)),
        }
    }

    if !found_any {
        anyhow::bail!(
            "No Capability.toml or Module.toml found in {:?} or its subdirectories",
            path
        );
    }

    if !found_any {
        anyhow::bail!("No Module.toml found in {:?} or its subdirectories", path);
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

pub async fn expand_single(path: &Path, no_compile: bool) -> Result<bool> {
    let output_dir = path.join("artifacts");
    let cache = std::sync::Arc::new(CacheManager::from_env().await?);

    let env = Environment::new(path.to_path_buf(), cache).await?;
    let artifacts = if no_compile {
        // Try to load artifacts from target/release
        let target_dir = Environment::get_target_dir(path).await?;
        env.load_artifacts_from_target(&target_dir).await?
    } else {
        env.pack(false).await?
    };
    for artifact in &artifacts {
        artifact.write_to_directory(&output_dir).await?;
    }

    for artifact in &artifacts {
        match artifact {
            Artifacts::CapabilityBinary(binary) => {
                let source = artifacts
                    .iter()
                    .find_map(|a| match a {
                        Artifacts::CapabilitySource(s) => Some(s),
                        _ => None,
                    })
                    .context("Missing CapabilitySource for CapabilityBinary")?;

                let symbols = pyro_artifacts::debug::symbols(binary);
                for sym in symbols {
                    let (name, content) = match sym {
                        Ok(CapSymbols::Elf(sym)) => ("elf.json", sym),
                        Ok(CapSymbols::MachO(sym)) => ("macho.json", sym),
                        Ok(CapSymbols::Pe(sym)) => ("pe.json", sym),
                        Ok(CapSymbols::Unknown(sym)) => ("Unknown.json", sym),
                        Err(error) => {
                            tracing::error!(error, "Unable to get one set of symbols");
                            continue;
                        }
                    };
                    match serde_json::to_string_pretty(&content) {
                        Ok(content) => fs::write(output_dir.join(name), content)?,
                        Err(error) => {
                            tracing::error!(?error, "Unable to serialize symbols");
                            continue;
                        }
                    }
                }

                let code = generate_capability(
                    &source.src_lib_rs,
                    &source.manifest.capability.package,
                    &source.manifest.capability.version,
                )
                .context("Capability code")?;
                let code = prettyplease::unparse(&code);
                fs::create_dir_all(&output_dir)?;
                fs::write(output_dir.join("cap.rs"), code)?;
            }
            Artifacts::Playbook(Playbook::Source(source)) => {
                let code = generate_module(&source.source).context("Module code")?;
                let code = prettyplease::unparse(&code);
                fs::create_dir_all(&output_dir)?;
                fs::write(output_dir.join("cap.rs"), code)?;
            }
            Artifacts::Playbook(Playbook::Binary(binary)) => match pyro_artifacts::debug::wat(binary) {
                Ok(wat) => fs::write(output_dir.join("mod.wat"), wat)?,
                Err(error) => {
                    tracing::error!(error, "Unable to create wat");
                }
            },
            _ => {}
        }
    }

    Ok(true)
}
