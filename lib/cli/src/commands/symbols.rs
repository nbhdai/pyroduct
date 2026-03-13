use anyhow::{Context, Result};
use fs_err as fs;
use object::{Object, ObjectSymbol, SymbolKind};
use std::io::Write;
use std::path::Path;

/// Scans a dynamic library (.dylib, .so, .dll) and writes clean FFI symbols to dy_symbols.txt
pub fn dump_dylib_symbols(artifact_path: &Path) -> Result<()> {
    if !artifact_path.exists() {
        return Ok(());
    }

    let bin_data = fs::read(artifact_path)?;
    let file = object::File::parse(&*bin_data)
        .with_context(|| format!("Failed to parse binary object: {:?}", artifact_path))?;

    let output_path = artifact_path.with_file_name("dy_symbols.txt");
    let mut out = fs::File::create(&output_path)?;

    writeln!(
        out,
        "Artifact: {:?}",
        artifact_path.file_name().unwrap_or_default()
    )?;
    writeln!(out, "Format:   {:?}", file.format())?;
    writeln!(out, "--- Exports ---")?;

    let mut count = 0;

    for symbol in file.symbols() {
        if symbol.kind() == SymbolKind::Text && symbol.is_global() && !symbol.is_undefined() {
            let name = symbol.name().unwrap_or("<unknown>");

            if is_noise(name) {
                continue;
            }

            writeln!(out, "[0x{:016x}] {}", symbol.address(), name)?;
            count += 1;
        }
    }

    if count == 0 {
        writeln!(out, "(No matching FFI symbols found)")?;
    }

    println!(
        "  ✓ Wrote {} symbols to {:?}",
        count,
        output_path.file_name().unwrap()
    );
    Ok(())
}

/// Heuristic to filter out mangled names and compiler internals
fn is_noise(name: &str) -> bool {
    // 1. Rust/C++ mangled names often look like _ZN... or __ZN...
    if name.contains("_ZN") || name.contains("__ZN") {
        return true;
    }

    // 2. Rust runtime/panic/allocation internals
    if name.contains("rust_eh_")
        || name.contains("__rust_")
        || name.contains("rust_begin_unwind")
        || name.contains("__rdl_")
    {
        return true;
    }
    // 3. System/Linker symbols (Global Offset Table, BSS, etc.)
    if name == "_init"
        || name == "_fini"
        || name == "__bss_start"
        || name == "_edata"
        || name == "_end"
    {
        return true;
    }
    // 4. Macos System prefix filter
    if name.starts_with("__") {
        return true;
    }

    false
}
