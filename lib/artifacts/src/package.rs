use anyhow::{Context, Result, bail};
use fs_err as fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cargo::{CapabilityManifest, ModuleManifest};
use crate::utils::{InterfaceGenerator, ProjectContext, TarballBuilder, extract_tarball};

pub struct Artifact {
    pub name: String,
    pub data: Vec<u8>,
}

pub struct PackageResult {
    pub name: String,
    pub version: String,
    pub artifacts: Vec<Artifact>,
}

fn get_target_dir(path: &Path) -> Result<PathBuf> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version=1", "--no-deps"])
        .current_dir(path)
        .output()
        .context("Failed to run cargo metadata")?;

    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("Failed to parse cargo metadata")?;

    metadata["target_directory"]
        .as_str()
        .map(PathBuf::from)
        .context("No target_directory in cargo metadata")
}

fn dylib_extension() -> &'static str {
    if cfg!(target_os = "macos") {
        "dylib"
    } else if cfg!(target_os = "windows") {
        "dll"
    } else {
        "so"
    }
}

pub fn run_cargo_command(
    path: &Path,
    tool_args: &[&str],
    error_ctx: &str,
    capture: bool,
) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.args(tool_args).current_dir(path);

    if capture {
        let output = cmd.output().with_context(|| error_ctx.to_string())?;

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "Cargo command failed with status {}.\nArgs: {:?}\n\nStdout:\n{}\nStderr:\n{}",
                output.status,
                tool_args,
                stdout,
                stderr
            );
        }
    } else {
        // .status() inherits stdio, allowing `cargo build` logs to print directly to the terminal
        let status = cmd.status().with_context(|| error_ctx.to_string())?;

        if !status.success() {
            bail!(
                "Cargo command failed with status {}. Args: {:?}",
                status,
                tool_args,
            );
        }
    }

    Ok(())
}

// ============================================================
// Module Packaging
// ============================================================

fn package_module(
    ctx: &ProjectContext,
    manifest: ModuleManifest,
    capture: bool,
) -> Result<PackageResult> {
    tracing::info!("Packaging module: {:?}", ctx.root);

    // 1. Generate Cargo.toml
    let cargo_toml_content = toml::to_string_pretty(&manifest.to_cargo())?;

    // 2. Build WASM with pass-through args
    tracing::info!("Compiling WASM module...");
    let build_args = vec![
        "build",
        "--release",
        "--target",
        "wasm32-unknown-unknown",
        "-p",
        &ctx.name,
    ];

    run_cargo_command(ctx.root, &build_args, "Failed to run cargo build", capture)?;

    // 3. Locate Artifact
    let target_dir = get_target_dir(ctx.root)?;
    let wasm_filename = format!("{}.wasm", ctx.normalized_name());
    let built_wasm = target_dir
        .join("wasm32-unknown-unknown/release")
        .join(&wasm_filename);

    if !built_wasm.exists() {
        bail!("Could not find compiled WASM: {}", built_wasm.display());
    }

    let wasm_bytes = fs::read(&built_wasm)?;

    // 4. Generate module spec (module.json)
    let src_path = ctx.root.join("src").join("lib.rs");
    let module_spec = if src_path.exists() {
        let content = fs::read_to_string(&src_path)
            .with_context(|| format!("Failed to read {:?}", src_path))?;
        pyro_core::module::generate_module_spec(&content)
            .map_err(|e| anyhow::anyhow!("Failed to generate module spec: {}", e))?
    } else {
        None
    };

    // 5. Create Archive
    let mut tar = TarballBuilder::new()?;
    tar.add_bytes("Cargo.toml", cargo_toml_content.as_bytes())?;
    tar.add_bytes("mod.wasm", &wasm_bytes)?;
    tar.add_dir(&ctx.root.join("src"), "src")?;

    if let Some(spec) = module_spec {
        tar.add_bytes("module.json", spec.as_bytes())?;
        tracing::info!("✓ Added module.json to archive");
    }

    let tar_data = tar.finish()?;
    let artifact_name = format!("{}-{}.module", ctx.name, ctx.version);

    Ok(PackageResult {
        name: ctx.name.clone(),
        version: ctx.version.clone(),
        artifacts: vec![Artifact {
            name: artifact_name,
            data: tar_data,
        }],
    })
}
// ============================================================
// Capability Packaging
// ============================================================

fn package_capability(
    ctx: &ProjectContext,
    manifest: CapabilityManifest,
    capture: bool,
) -> Result<PackageResult> {
    tracing::info!("Packaging capability: {:?}", ctx.root);

    // 1. Generate Cargo.toml content
    let cargo_toml_content = toml::to_string_pretty(&manifest.clone().to_capability_manifest())?;
    // fs::write(ctx.root.join("Cargo.toml"), &cargo_toml_content)?;
    // tracing::info!("✓ Wrote Cargo.toml");

    // 2. Build Dynamic Library with pass-through args
    tracing::info!("Compiling capability binary...");
    let build_args = vec![
        "build",
        "--release",
        "--features",
        "capability",
        "-p",
        &ctx.name,
    ];

    run_cargo_command(ctx.root, &build_args, "Failed to run cargo build", capture)?;

    // 3. Locate Artifact
    let target_dir = get_target_dir(ctx.root)?;
    let lib_filename = format!("lib{}.{}", ctx.normalized_name(), dylib_extension());
    let built_lib = target_dir.join("release").join(&lib_filename);

    if !built_lib.exists() {
        bail!("Could not find compiled binary: {}", built_lib.display());
    }

    let lib_bytes = fs::read(&built_lib)?;

    // 4. Create Source Archive (.cap)
    let mut cap_tar = TarballBuilder::new()?;
    cap_tar.add_bytes("Cargo.toml", cargo_toml_content.as_bytes())?;
    cap_tar.add_bytes(&format!("lib.{}", dylib_extension()), &lib_bytes)?;
    cap_tar.add_dir(&ctx.root.join("src"), "src")?;

    // 5. Interface Generation
    let interface = InterfaceGenerator::new(ctx.root, &manifest)?;

    // 6. Create Interface Archive (.interface)
    let mut interface_tar = TarballBuilder::new()?;
    interface.add_to_archive(&mut interface_tar, true)?;
    interface_tar.add_bytes("interface.json", interface.spec().as_bytes())?;

    // 7. Add config spec to .cap
    if let Some(spec) = interface.config() {
        cap_tar.add_bytes("config.json", spec.as_bytes())?;
    }

    let cap_data = cap_tar.finish()?;
    let interface_data = interface_tar.finish()?;

    let cap_name = format!("{}-{}.cap", ctx.name, ctx.version);
    let interface_name = format!("{}-{}.interface", ctx.name, ctx.version);

    Ok(PackageResult {
        name: ctx.name.clone(),
        version: ctx.version.clone(),
        artifacts: vec![
            Artifact {
                name: cap_name,
                data: cap_data,
            },
            Artifact {
                name: interface_name,
                data: interface_data,
            },
        ],
    })
}
