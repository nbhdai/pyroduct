use anyhow::{Context, Result, bail};
use fs_err as fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::cargo::{CapabilityManifest, ModuleManifest};
use crate::utils::{InterfaceGenerator, ProjectContext, TarballBuilder};

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
    capture: bool
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
    cmd.args(tool_args)
       .args(user_args)
       .current_dir(path);

    if capture {
        // .output() executes the command as a child process and strictly captures everything
        let output = cmd.output().with_context(|| error_ctx.to_string())?;
        
        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "Cargo command failed with status {}.\nArgs: {:?} {:?}\n\nStdout:\n{}\nStderr:\n{}",
                output.status, tool_args, user_args, stdout, stderr
            );
        }
    } else {
        // .status() inherits stdio, allowing `cargo build` logs to print directly to the terminal
        let status = cmd.status().with_context(|| error_ctx.to_string())?;
        
        if !status.success() {
            bail!(
                "Cargo command failed with status {}. Args: {:?} {:?}",
                status, tool_args, user_args
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
    cargo_args: &[String],
    capture: bool,
) -> Result<()> {
    if !capture { println!("Packaging module: {:?}", ctx.root); }

    // 1. Generate Cargo.toml
    let cargo_toml_content = toml::to_string_pretty(&manifest.to_cargo())?;
    fs::write(ctx.root.join("Cargo.toml"), &cargo_toml_content)?;
    if !capture { println!("✓ Wrote Cargo.toml"); }

    // 2. Build WASM with pass-through args
    if !capture { println!("Compiling WASM module..."); }
    let build_args = vec![
        "build",
        "--release",
        "--target",
        "wasm32-unknown-unknown",
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
    let wasm_filename = format!("{}.wasm", ctx.normalized_name());
    let built_wasm = target_dir
        .join("wasm32-unknown-unknown/release")
        .join(&wasm_filename);

    if !built_wasm.exists() {
        bail!("Could not find compiled WASM: {}", built_wasm.display());
    }

    let dest_wasm = ctx.output_dir.join("mod.wasm");
    fs::copy(&built_wasm, &dest_wasm)?;
    if !capture { println!("✓ Compiled {}", dest_wasm.display()); }

    // 4. Create Archive
    let mut tar = TarballBuilder::new(ctx.archive_path("module"))?;
    tar.add_bytes("Cargo.toml", cargo_toml_content.as_bytes())?;
    tar.add_dir(&ctx.root.join("src"), "src")?;
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
    if !capture { println!("Packaging capability: {:?}", ctx.root); }

    // 1. Generate Cargo.toml
    let cargo_toml_content = toml::to_string_pretty(&manifest.clone().to_capability_manifest())?;
    fs::write(ctx.root.join("Cargo.toml"), &cargo_toml_content)?;
    if !capture { println!("✓ Wrote Cargo.toml"); }

    // 2. Build Dynamic Library with pass-through args
    if !capture { println!("Compiling capability binary..."); }
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
    if !capture { println!("✓ Compiled {}", dest_lib.display()); }

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

fn package_single(path: &Path, output: Option<&Path>, cargo_args: &[String], capture: bool) -> Result<()> {
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

pub fn package(path: &Path, output: Option<&Path>, cargo_args: &[String], capture: bool) -> Result<()> {
    // 1. Direct package mode
    if path.join("Capability.toml").exists() || path.join("Module.toml").exists() {
        return package_single(path, output, cargo_args, capture);
    }

    // 2. Recursive scan mode
    if !capture {
        println!(
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
        
        if !capture {
            eprintln!("{}", err_msg);
        }
        bail!("{} packaging(s) failed. {}", errors.len(), err_msg);
    }

    Ok(())
}