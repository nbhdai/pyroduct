use std::path::{Path, PathBuf};
use std::io::Write;
use anyhow::{Context, Result};

pub mod cargo;

// NOTE: You need to expose the parsing logic from capability-core or duplicate the structs here.
// For this example, we assume `capability_core` is available.
use capability_core::generate_client;

pub struct ModuleGenerator {
    source_path: PathBuf,
}

impl ModuleGenerator {
    pub fn new(source_path: impl AsRef<Path>) -> Self {
        Self {
            source_path: source_path.as_ref().to_path_buf(),
        }
    }

    /// Generates the Rust client code and writes it to the destination path.
    pub fn generate_rust_source(&self, dest_path: impl AsRef<Path>) -> Result<()> {
        let content = std::fs::read_to_string(&self.source_path)
            .with_context(|| format!("Failed to read capability source: {:?}", self.source_path))?;
        let generated_code = generate_client(&content)?;
        // Ensure parent dir exists
        let dest = dest_path.as_ref();
        if let Some(parent) = dest.parent() {
            fs_err::create_dir_all(parent)?;
        }

        let mut out_file = fs_err::File::create(dest)
            .with_context(|| format!("Failed to create output file: {:?}", dest))?;
        
        out_file.write_all(generated_code.as_bytes())?;
        
        // Format with rustfmt if available (optional polish)
        let _ = std::process::Command::new("rustfmt").arg(dest).status();

        Ok(())
    }
}
