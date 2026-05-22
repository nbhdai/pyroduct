use anyhow::{Context, Result};
use fs_err as fs;
use pyro_artifacts::{cache::CacheManager, environment::Environment};
use std::path::Path;

pub async fn spec(path: &Path, out: Option<&Path>) -> Result<()> {
    let cache = std::sync::Arc::new(CacheManager::from_env().await?);
    let env = Environment::new(path.to_path_buf(), cache).await?;

    let (json, default_filename) = match &env.manifest {
        pyro_artifacts::cargo::ProjectManifest::Capability(_) => {
            let spec = env
                .capability_spec()
                .await
                .context("Failed to generate capability spec")?;
            (
                serde_json::to_string_pretty(&spec)
                    .context("Failed to serialize capability spec")?,
                "interface.json",
            )
        }
        pyro_artifacts::cargo::ProjectManifest::Module(_) => {
            let spec = env
                .module_spec()
                .await
                .context("Failed to generate module spec")?;
            (
                serde_json::to_string_pretty(&spec).context("Failed to serialize module spec")?,
                "module.json",
            )
        }
    };

    match out {
        Some(output_path) => {
            fs::write(output_path, json).context("Failed to write spec file")?;
            println!("Successfully generated spec at {:?}", output_path);
        }
        None => {
            let output_path = &path.join(default_filename);
            fs::write(output_path, json).context("Failed to write spec file")?;
            println!("Successfully generated spec at {:?}", output_path);
        }
    }

    Ok(())
}
