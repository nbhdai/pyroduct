use anyhow::{Context, Result, bail};
use fs_err as fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::cargo::{CapabilityManifest, ModuleManifest};
use crate::artifacts::utils::{
    InterfaceGenerator, ProjectContext, TarballBuilder, pyroduct_compile_dir,
};

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
        // .output() executes the command as a child process and strictly captures everything
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
        // .status() inherits stdio, allowing `cargo build` logs to print directly to the terminal
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

// ============================================================
// Module Packaging
// ============================================================

fn package_module(
    ctx: &ProjectContext,
    mut manifest: ModuleManifest,
    cargo_args: &[String],
    capture: bool,
) -> Result<()> {
    tracing::info!("Packaging module: {:?}", ctx.root);

    let compile_dir = pyroduct_compile_dir();
    let module_dir = compile_dir.join("module");
    fs::create_dir_all(&module_dir)?;

    let interfaces_dir = compile_dir.join("interfaces");
    fs::create_dir_all(&interfaces_dir)?;

    let cargo_config_dir = compile_dir.join(".cargo");
    fs::create_dir_all(&cargo_config_dir)?;
    let target_dir = compile_dir.join("target");
    let config_toml_content = format!("[build]\ntarget-dir = \"{}\"\n", target_dir.display());
    fs::write(cargo_config_dir.join("config.toml"), config_toml_content)?;

    // Make all path dependencies absolute so they don't break when moved to module_dir
    let make_absolute = |dep: &mut cargo_toml::Dependency| {
        if let cargo_toml::Dependency::Detailed(detail) = dep {
            if let Some(rel_path) = &detail.path {
                let abs_path = ctx
                    .root
                    .join(rel_path)
                    .canonicalize()
                    .unwrap_or_else(|_| ctx.root.join(rel_path));
                detail.path = Some(abs_path.to_string_lossy().to_string());
            }
        }
    };

    if let cargo_toml::Dependency::Detailed(detail) = &mut manifest.pyroduct {
        if let Some(rel_path) = &detail.path {
            let abs_path = ctx
                .root
                .join(rel_path)
                .canonicalize()
                .unwrap_or_else(|_| ctx.root.join(rel_path));
            detail.path = Some(abs_path.to_string_lossy().to_string());
        }
    }

    for dep in manifest.dependencies.values_mut() {
        make_absolute(dep);
    }
    for dep in manifest.build_dependencies.values_mut() {
        make_absolute(dep);
    }
    for dep in manifest.dev_dependencies.values_mut() {
        make_absolute(dep);
    }

    // For capabilities, generate interface and update path
    for dep in manifest.capabilities.values_mut() {
        if let cargo_toml::Dependency::Detailed(detail) = dep {
            if let Some(rel_path) = &detail.path {
                let cap_path = ctx.root.join(rel_path);
                let cap_toml_path = cap_path.join("Capability.toml");
                if cap_toml_path.exists() {
                    if let Ok(cap_manifest_str) = fs::read_to_string(&cap_toml_path) {
                        if let Ok(cap_manifest) = toml::from_str::<
                            crate::artifacts::cargo::CapabilityManifest,
                        >(&cap_manifest_str)
                        {
                            if let Ok((cap_name, cap_version)) = cap_manifest.name_version() {
                                let interface_name_version =
                                    format!("{}_{}", cap_name, cap_version);
                                let interface_path = interfaces_dir
                                    .join(&interface_name_version)
                                    .join("interface");

                                if let Ok(generator) =
                                    InterfaceGenerator::new(&cap_path, &cap_manifest)
                                {
                                    let _ = generator.write_to_disk(&interface_path, false);
                                }

                                let new_rel_path =
                                    format!("../interfaces/{}", interface_name_version);
                                detail.path = Some(new_rel_path);
                            }
                        }
                    }
                } else {
                    let abs_path = cap_path.canonicalize().unwrap_or(cap_path);
                    detail.path = Some(abs_path.to_string_lossy().to_string());
                }
            }
        }
    }

    // Write Module.toml to module_dir
    let module_toml_content = toml::to_string_pretty(&manifest)?;
    fs::write(module_dir.join("Module.toml"), module_toml_content)?;

    // 1. Generate Cargo.toml
    let cargo_toml_content = toml::to_string_pretty(&manifest.to_cargo())?;
    fs::write(module_dir.join("Cargo.toml"), &cargo_toml_content)?;
    tracing::info!("✓ Wrote Cargo.toml");

    // Copy src directory
    let src_dir = ctx.root.join("src");
    if src_dir.exists() {
        let dest_src = module_dir.join("src");
        fs::create_dir_all(&dest_src)?;
        for entry in fs::read_dir(src_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                fs::copy(&path, dest_src.join(entry.file_name()))?;
            }
        }
    }

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

    run_cargo_command(
        &module_dir,
        &build_args,
        cargo_args,
        "Failed to run cargo build",
        capture,
    )?;

    // 3. Locate and Copy Artifact
    let wasm_filename = format!("{}.wasm", ctx.normalized_name());
    let built_wasm = target_dir
        .join("wasm32-unknown-unknown/release")
        .join(&wasm_filename);

    if !built_wasm.exists() {
        bail!("Could not find compiled WASM: {}", built_wasm.display());
    }

    let dest_wasm = ctx.output_dir.join("mod.wasm");
    fs::copy(&built_wasm, &dest_wasm)?;
    tracing::info!("✓ Compiled {}", dest_wasm.display());

    // 4. Create Archive
    let mut tar = TarballBuilder::new(ctx.archive_path("module"))?;
    tar.add_bytes("Cargo.toml", cargo_toml_content.as_bytes())?;
    tar.add_dir(&module_dir.join("src"), "src")?;
    tar.finish()?;

    Ok(())
}

// ============================================================
// Capability Packaging
// ============================================================

fn package_capability(
    ctx: &ProjectContext,
    manifest: CapabilityManifest,
    cargo_args: &[String],
    capture: bool,
) -> Result<()> {
    tracing::info!("Packaging capability: {:?}", ctx.root);

    // 1. Generate Cargo.toml
    let cargo_toml_content = toml::to_string_pretty(&manifest.clone().to_capability_manifest())?;
    fs::write(ctx.root.join("Cargo.toml"), &cargo_toml_content)?;
    tracing::info!("✓ Wrote Cargo.toml");

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

    run_cargo_command(
        ctx.root,
        &build_args,
        cargo_args,
        "Failed to run cargo build",
        capture,
    )?;

    // 3. Locate and Copy Artifact
    let target_dir = get_target_dir(ctx.root)?;
    let lib_filename = format!("lib{}.{}", ctx.normalized_name(), dylib_extension());
    let built_lib = target_dir.join("release").join(&lib_filename);

    if !built_lib.exists() {
        bail!("Could not find compiled binary: {}", built_lib.display());
    }

    let dest_lib = ctx.output_dir.join(format!("lib.{}", dylib_extension()));
    fs::copy(&built_lib, &dest_lib)?;
    tracing::info!("✓ Compiled {}", dest_lib.display());

    // 4. Create Source Archive (.cargo)
    let mut cap_tar = TarballBuilder::new(ctx.archive_path("cargo"))?;
    cap_tar.add_bytes("Cargo.toml", cargo_toml_content.as_bytes())?;
    cap_tar.add_dir(&ctx.root.join("src"), "src")?;

    // 5. Create Interface Archive (.interface)
    let mut interface_tar = TarballBuilder::new(ctx.archive_path("interface"))?;
    let interface = InterfaceGenerator::new(ctx.root, &manifest)?;
    interface.add_to_archive(&mut interface_tar, true)?;

    // 6. Add documentation
    interface_tar.add_bytes("interface.json", interface.spec().as_bytes())?;
    fs::write(ctx.output_dir.join("interface.json"), interface.spec())?;

    // 7. Generate config spec
    if let Some(spec) = interface.config() {
        cap_tar.add_bytes("config.json", spec.as_bytes())?;
        fs::write(ctx.output_dir.join("config.json"), spec)?;
    }

    interface_tar.finish()?;
    cap_tar.finish()?;
    Ok(())
}

// ============================================================
// Entry Points
// ============================================================

fn package_single(
    path: &Path,
    output: Option<&Path>,
    cargo_args: &[String],
    capture: bool,
) -> Result<()> {
    let output_dir = output
        .map(|p| p.to_path_buf())
        .unwrap_or(path.join("artifacts"));
    fs::create_dir_all(&output_dir)?;
    let cap_toml = path.join("Capability.toml");
    let mod_toml = path.join("Module.toml");

    if cap_toml.exists() && mod_toml.exists() {
        bail!("Both Capability.toml and Module.toml found in {:?}", path);
    }

    if cap_toml.exists() {
        let manifest: CapabilityManifest = toml::from_str(&fs::read_to_string(&cap_toml)?)?;
        let pkg = manifest
            .capability
            .as_ref()
            .context("Package section missing in Capability.toml")?;

        let ctx = ProjectContext::new(path, output_dir.as_path(), &pkg.name, pkg.version());
        package_capability(&ctx, manifest, cargo_args, capture)
    } else if mod_toml.exists() {
        let manifest: ModuleManifest = toml::from_str(&fs::read_to_string(&mod_toml)?)?;
        let pkg = manifest
            .module
            .as_ref()
            .context("Module section missing in Module.toml")?;
        let ctx = ProjectContext::new(path, output_dir.as_path(), &pkg.name, pkg.version());
        package_module(&ctx, manifest, cargo_args, capture)
    } else {
        bail!(
            "Neither Capability.toml nor Module.toml found in {:?}",
            path
        )
    }
}

pub fn package(
    path: &Path,
    output: Option<&Path>,
    cargo_args: &[String],
    capture: bool,
) -> Result<()> {
    // 1. Direct package mode
    if path.join("Capability.toml").exists() || path.join("Module.toml").exists() {
        return package_single(path, output, cargo_args, capture);
    }

    // 2. Recursive scan mode
    if !capture {
        tracing::info!(
            "No manifest found in {:?}, scanning subdirectories...",
            path
        );
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
            if let Err(e) = package_single(&subpath, output, cargo_args, capture) {
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
        bail!("{} packaging(s) failed. {}", errors.len(), err_msg);
    }

    Ok(())
}
