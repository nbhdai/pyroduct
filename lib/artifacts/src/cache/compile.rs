use anyhow::{Context, Result, bail};
use cargo_toml::Dependency;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::process::Command;
use tokio::fs;

use crate::cache::CacheManager;
use crate::cargo::ModuleManifest;

fn run_cargo_command(
    path: &Path,
    tool_args: &[&str],
    error_ctx: &str,
    capture: bool,
) -> Result<()> {
    // 3. Combine and execute
    let mut cmd = Command::new("cargo");
    cmd.args(tool_args).current_dir(path);

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

/// A resolved capability for anon module compilation.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ResolvedCapability {
    pub author: String,
    pub package: String,
    pub version: String,
}

/// Compiles a module inside the cache directory, avoiding modifying the user's workspace.
/// Returns the compiled `.wasm` bytes.
///
/// Each capability is resolved from the cache at
/// `capabilities/<author>/<package>/<version>/interface` and referenced via a
/// relative path dependency in the generated Cargo.toml.
pub fn compile_module(
    dependencies: Vec<(String, Dependency)>,
    capabilities: Vec<ResolvedCapability>,
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

    // Resolve each capability as a path dependency pointing at the cached interface crate.
    // The interface lives at: <cache_root>/capabilities/<author>/<package>/<version>/interface
    // We compute the relative path from the build_dir to that interface directory.
    for cap in &capabilities {
        let interface_dir = cache.capability_interface_dir(&cap.author, &cap.package, &cap.version);
        if !interface_dir.exists() {
            bail!(
                "Capability interface not found in cache: {}/{}/{} (expected at {}). Run `pyroduct ship` first.",
                cap.author,
                cap.package,
                cap.version,
                interface_dir.display()
            );
        }

        // Point at the capability version dir, not the interface subdir directly,
        // because ModuleManifest::augment_deps appends "/interface" to path deps.
        let cap_dir = cache.capabilities_dir(&cap.author, &cap.package, &cap.version);
        let rel_path = Path::new("..").join(&cap_dir);

        let dep = Dependency::Detailed(Box::new(cargo_toml::DependencyDetail {
            path: Some(rel_path.to_string_lossy().into_owned()),
            ..Default::default()
        }));
        manifest.capabilities.insert(cap.package.clone(), dep);
    }

    let cargo_toml_content = toml::to_string_pretty(&manifest.to_cargo())?;
    fs::write(build_dir.join("Cargo.toml"), &cargo_toml_content)?;

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
    let built_wasm = cache
        .target_dir()
        .join("wasm32-unknown-unknown/release/mod.wasm");

    if !built_wasm.exists() {
        bail!("Could not find compiled WASM: {}", built_wasm.display());
    }
    let wasm = fs::read(&built_wasm)?;

    cache.add_anon_module(&hash, &wasm)?;

    // Clean up temporary build structure
    let _ = fs::remove_dir_all(&build_dir);

    Ok(wasm)
}
