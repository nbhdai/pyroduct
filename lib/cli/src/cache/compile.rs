use anyhow::{Context, Result, bail};
use cargo_toml::Dependency;
use fs_err as fs;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::process::Command;

use crate::artifacts::cargo::ModuleManifest;
use crate::cache::CacheManager;

fn run_cargo_command(
    path: &Path,
    tool_args: &[&str],
    user_args: &[String],
    error_ctx: &str,
    capture: bool,
) -> Result<()> {
    // 1. Identify flags in the tool_args to prevent user overrides
    let restricted_flags: std::collections::HashSet<_> = tool_args
        .iter()
        .filter(|arg| arg.starts_with('-'))
        .map(|arg| arg.split('=').next().unwrap_or(arg))
        .collect();

    // 2. Check user_args for conflicts
    for user_arg in user_args {
        if user_arg.starts_with('-') {
            let flag_key = user_arg.split('=').next().unwrap_or(user_arg);
            if restricted_flags.contains(flag_key) {
                bail!(
                    "Conflict detected: User argument '{}' overrides internal tool flag '{}'",
                    user_arg,
                    flag_key
                );
            }
        }
    }

    // 3. Combine and execute
    let mut cmd = Command::new("cargo");
    cmd.args(tool_args).args(user_args).current_dir(path);

    if capture {
        let output = cmd.output().with_context(|| error_ctx.to_string())?;

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "Cargo command failed with status {}.\nArgs: {:?} {:?}\n\nStdout:\n{}\nStderr:\n{}",
                output.status,
                tool_args,
                user_args,
                stdout,
                stderr
            );
        }
    } else {
        let status = cmd.status().with_context(|| error_ctx.to_string())?;

        if !status.success() {
            bail!(
                "Cargo command failed with status {}. Args: {:?} {:?}",
                status,
                tool_args,
                user_args
            );
        }
    }

    Ok(())
}

/// Compiles a module inside the cache directory, avoiding modifying the user's workspace.
/// Returns the path to the compiled `.wasm` file in the cache target directory.
pub fn compile_module(
    dependencies: Vec<(String, Dependency)>,
    capabilities: Vec<(String, Dependency)>,
    code: &str,
) -> Result<Vec<u8>> {
    let cache = CacheManager::new()?;
    let config = cache.config();

    let author = config.author.unwrap_or_else(|| "anon".to_string());
    let pyroduct_dep = config
        .pyroduct
        .unwrap_or_else(|| Dependency::Simple("*".to_string()));

    let mut hasher = Sha256::new();
    hasher.update(code.as_bytes());
    let hash = format!("{:x}", hasher.finalize());

    tracing::info!(
        "Compiling module via CacheManager for {} (hash: {})",
        author,
        hash
    );

    // We create a temporary build directory inside the cache.
    let build_dir = cache.root.join("build").join(&hash);
    fs::create_dir_all(&build_dir)?;

    let src_dir = build_dir.join("src");
    fs::create_dir_all(&src_dir)?;
    fs::write(src_dir.join("lib.rs"), code)?;

    let basic_toml = format!(
        r#"
[module]
name = "mod"
version = "0.1.0"
authors = ["{}"]
edition = "2024"

[pyroduct]
version = "*"
"#,
        author
    );

    let mut manifest: ModuleManifest = toml::from_str(&basic_toml)?;
    manifest.pyroduct = pyroduct_dep;
    for (dep_name, dep) in dependencies {
        manifest.dependencies.insert(dep_name, dep);
    }
    for (cap_name, cap) in capabilities {
        manifest.capabilities.insert(cap_name, cap);
    }

    let cargo_toml_content = toml::to_string_pretty(&manifest.to_cargo())?;
    fs::write(build_dir.join("Cargo.toml"), &cargo_toml_content)?;

    let cargo_config_dir = build_dir.join(".cargo");
    fs::create_dir_all(&cargo_config_dir)?;
    let target_dir = build_dir.join("target");
    let config_toml_content = format!("[build]\ntarget-dir = \"{}\"\n", target_dir.display());
    fs::write(cargo_config_dir.join("config.toml"), config_toml_content)?;

    tracing::info!("Compiling WASM module...");
    let build_args = vec!["build", "--release", "--target", "wasm32-unknown-unknown"];

    run_cargo_command(
        &build_dir,
        &build_args,
        &[],
        "Failed to run cargo build",
        true, // always capture
    )?;

    // Return the path to the artifact
    let built_wasm = target_dir.join("wasm32-unknown-unknown/release/mod.wasm");

    if !built_wasm.exists() {
        bail!("Could not find compiled WASM: {}", built_wasm.display());
    }
    let wasm = fs::read(&built_wasm)?;

    cache.add_anon_module(&hash, &wasm)?;

    // Clean up temporary build structure
    let _ = fs::remove_dir_all(&build_dir);

    Ok(wasm)
}
