use anyhow::{Context, Result};
use artifacts::{artifacts::Artifact, debug::CapSymbols, environment::Environment};
use fs_err as fs;
use pyro_core::{ffi::generate_capability, module::generate_module};
use std::path::Path;

pub async fn expand(path: &Path) -> Result<()> {
    let is_cap = path.join("Capability.toml").exists();
    let is_mod = path.join("Module.toml").exists();
    if is_cap || is_mod {
        expand_single(path).await?;
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
        match expand_single(&subpath).await {
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

pub async fn expand_single(path: &Path) -> Result<bool> {
    let output_dir = path.join("artifacts");

    let env = Environment::new(path.to_path_buf()).await?;
    if let Some(interface_artifact) = env.create_interface().await? {
        let interface_dir = output_dir.join("interface");
        interface_artifact
            .write_to_directory(&interface_dir)
            .await?;
    }
    let artifacts = env.package(false).await?;
    artifacts.write_to_directory(&output_dir).await?;
    match &artifacts {
        artifacts::artifacts::Artifacts::Capability(capability) => {
            let symbols = artifacts::debug::symbols(capability);
            for sym in symbols {
                let (name, content) = match sym {
                    Ok(CapSymbols::Elf(sym)) => ("elf.json", sym),
                    Ok(CapSymbols::MachO(sym)) => ("macho.json", sym),
                    Ok(CapSymbols::Pe(sym)) => ("pe.json", sym),
                    Ok(CapSymbols::Unknown(sym)) => ("Unknown.json", sym),
                    Err(error) => {
                        tracing::error!(error, "Unable to get one set of symbols");
                        continue;
                    },
                };
                match serde_json::to_string_pretty(&content) {
                    Ok(content) => fs::write(output_dir.join(name), content)?,
                    Err(error) => {
                        tracing::error!(?error, "Unable to serialize symbols");
                        continue;
                    },
                }
                
            }

            let code = generate_capability(&capability.src_lib_rs, &capability.manifest.capability.name, &capability.manifest.capability.version).context("Capability code")?;
            let code = prettyplease::unparse(&code);
            fs::create_dir_all(&output_dir)?;
            fs::write(output_dir.join("cap.rs"), code)?;
        },
        artifacts::artifacts::Artifacts::Interface(_) => {},
        artifacts::artifacts::Artifacts::Module(module) => {
            match artifacts::debug::wat(module) {
            
                Ok(wat) => fs::write(output_dir.join("mod.wat"), wat)?,
                Err(error) => {
                    tracing::error!(error, "Unable to create wat");
                },
            }

            let code = generate_module(&module.source).context("Module code")?;
            let code = prettyplease::unparse(&code);
            fs::create_dir_all(&output_dir)?;
            fs::write(output_dir.join("cap.rs"), code)?;
        },
    }

    Ok(true)
}
