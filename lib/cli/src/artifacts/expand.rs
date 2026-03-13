use super::symbols;
use anyhow::{Result, Context};
use artifacts::{cargo::ModuleManifest, environment::format_syn_error, cargo::{CapabilityManifest}};
use fs_err as fs;
use pyro_core::{ffi::generate_capability, module::generate_module};
use std::path::Path;

pub fn expand_single(path: &Path) -> Result<bool> {
    let is_cap = path.join("Capability.toml").exists();
    let is_mod = path.join("Module.toml").exists();

    if is_cap {
        let cap_toml_path = path.join("Capability.toml");
        let manifest_str = fs::read_to_string(&cap_toml_path).context("Unable to read manifest")?;
        let cap_manifest: CapabilityManifest = toml::from_str(&manifest_str).context("Unable to deserialize manifest")?;
        
        let author = cap_manifest.capability.author;
        let name = cap_manifest.capability.name;
        let version = cap_manifest.capability.version;
        
        let source_path = path.join("src/lib.rs");
        let source = fs::read_to_string(&source_path).context("Unable to read source")?;
        let code = generate_capability(&source, &name, &version).map_err(|s| format_syn_error("Capability code", s))?;
        let code = prettyplease::unparse(&code);
        let artifacts_dir = path.join("artifacts");
        fs::create_dir_all(&artifacts_dir)?;
        fs::write(artifacts_dir.join("cap.rs"), code)?;

        dylib_project(path)?;
        return Ok(true);
    }

    if is_mod {
        let source_path = path.join("src/lib.rs");
        let source = fs::read_to_string(&source_path).context("Unable to read source")?;
        let code = generate_module(&source).map_err(|s| format_syn_error("Capability code", s))?;
        let code = prettyplease::unparse(&code);
        let artifacts_dir = path.join("artifacts");
        fs::create_dir_all(&artifacts_dir)?;
        fs::write(artifacts_dir.join("cap.rs"), code)?;

        wat_project(path)?;
        return Ok(true);
    }
    Ok(false)
}

pub fn expand(path: &Path) -> Result<()> {
    if expand_single(path)? {
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

        match expand_single(&subpath) {
            Ok(true) => found_any = true,
            Ok(false) => {},
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

fn wat_project(path: &Path) -> Result<bool> {
    let wasm_path = path.join("artifacts").join("mod.wasm");
    if wasm_path.exists() {
        wat(&wasm_path)?;
        println!("  ✓ Wrote artifacts/mod.wat");
        Ok(true)
    } else {
        println!("  x No WASM binary found at {:?}", wasm_path);
        Ok(false)
    }
}

fn dylib_project(path: &Path) -> Result<bool> {
    let artifacts = path.join("artifacts");
    if !artifacts.exists() {
        return Ok(false);
    }

    let mut found = false;
    // Scan for any common shared library extension
    for entry in fs::read_dir(artifacts)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ["dylib", "so", "dll"].contains(&ext) {
                // Ensure we don't try to parse the symbols file itself or other garbage
                if !path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .contains("symbols")
                {
                    symbols::dump_dylib_symbols(&path)?;
                    found = true;
                }
            }
        }
    }

    if found {
        Ok(true)
    } else {
        // Silent return if no binary artifacts found (expected if not built yet)
        Ok(false)
    }
}

pub fn wat(input: &Path) -> anyhow::Result<()> {
    use std::io::Write;
    let output = input.with_extension("wat");

    if input.extension().and_then(|e| e.to_str()) != Some("wasm") {
        eprintln!(
            "Warning: Input file '{}' does not have .wasm extension",
            input.display()
        );
    }

    let wasm_bytes = fs_err::read(&input)?;

    let wat = wasmprinter::print_bytes(&wasm_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to convert WASM to WAT: {}", e))?;

    // Write the WAT output
    let mut file = fs_err::File::create(&output)?;
    file.write_all(wat.as_bytes())?;

    println!("Converted {} -> {}", input.display(), output.display());

    Ok(())
}
