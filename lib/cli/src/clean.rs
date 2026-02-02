use anyhow::Result;
use fs_err as fs;
use std::path::Path;

pub fn clean(path: &Path) -> Result<()> {
    // 1. Direct clean mode
    if path.join("Capability.toml").exists() || path.join("Module.toml").exists() {
        return clean_single(path);
    }

    // 2. Recursive scan mode
    println!(
        "No manifest found in {:?}, scanning subdirectories...",
        path
    );

    let mut cleaned_any = false;
    let mut errors = Vec::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let subpath = entry.path();

        if !subpath.is_dir() {
            continue;
        }

        if subpath.join("Capability.toml").exists() || subpath.join("Module.toml").exists() {
            cleaned_any = true;
            if let Err(e) = clean_single(&subpath) {
                errors.push((subpath, e));
            }
        }
    }

    if !cleaned_any {
        anyhow::bail!(
            "No Capability.toml or Module.toml found in {:?} or subdirectories",
            path
        );
    }

    if !errors.is_empty() {
        eprintln!("\nErrors encountered:");
        for (p, e) in &errors {
            eprintln!("  {:?}: {:#}", p, e);
        }
        anyhow::bail!("{} clean operation(s) failed", errors.len());
    }

    Ok(())
}

fn clean_single(path: &Path) -> Result<()> {
    println!("Cleaning: {:?}", path);

    // List of files/directories to remove
    let targets = [
        "Cargo.toml",
        "Cargo.lock",
        "artifacts",
        "interface",
        "target",
    ];

    for target in targets {
        let target_path = path.join(target);
        if target_path.exists() {
            if target_path.is_dir() {
                fs::remove_dir_all(&target_path)?;
                println!("  ✓ Removed directory: {}", target);
            } else {
                fs::remove_file(&target_path)?;
                println!("  ✓ Removed file: {}", target);
            }
        }
    }

    Ok(())
}
