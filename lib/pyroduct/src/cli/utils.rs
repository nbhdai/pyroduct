use anyhow::{Context, Result};
use flate2::write::GzEncoder;
use flate2::Compression;
use fs_err as fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tar::Builder;

use crate::cli::cargo::CapabilityManifest;

/// Central context to reduce argument passing
pub struct ProjectContext<'a> {
    pub root: &'a Path,
    pub output_dir: &'a Path,
    pub name: String,
    pub version: String,
}

impl<'a> ProjectContext<'a> {
   pub fn new(root: &'a Path, output_dir: &'a Path, name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            root,
            output_dir,
            name: name.into(),
            version: version.into(),
        }
    }

    pub fn normalized_name(&self) -> String {
        self.name.replace('-', "_")
    }

    pub fn archive_path(&self, suffix: &str) -> PathBuf {
        self.output_dir.join(format!("{}-{}.{}", self.name, self.version, suffix))
    }
}

pub struct TarballBuilder {
    tar: Builder<GzEncoder<fs::File>>,
    path: PathBuf,
}

impl TarballBuilder {
    pub fn new(path: PathBuf) -> Result<Self> {
        let file = fs::File::create(&path)?;
        let enc = GzEncoder::new(file, Compression::default());
        Ok(Self {
            tar: Builder::new(enc),
            path,
        })
    }

    pub fn add_bytes(&mut self, path: &str, data: &[u8]) -> Result<()> {
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        self.tar.append_data(&mut header, path, data)?;
        Ok(())
    }

    pub fn add_dir(&mut self, host_path: &Path, archive_prefix: &str) -> Result<()> {
        if !host_path.exists() {
            return Ok(());
        }
        
        // Recursive helper
        fn append_recursive<W: std::io::Write>(
            tar: &mut Builder<W>, 
            dir: &Path, 
            prefix: &str
        ) -> Result<()> {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                let name = entry.file_name();
                let archive_path = format!("{}/{}", prefix, name.to_string_lossy());

                if path.is_dir() {
                    append_recursive(tar, &path, &archive_path)?;
                } else {
                    let data = fs::read(&path)?;
                    let mut header = tar::Header::new_gnu();
                    header.set_size(data.len() as u64);
                    header.set_mode(0o644);
                    header.set_cksum();
                    tar.append_data(&mut header, archive_path, data.as_slice())?;
                }
            }
            Ok(())
        }

        append_recursive(&mut self.tar, host_path, archive_prefix)?;
        Ok(())
    }

    pub fn finish(self) -> Result<()> {
        self.tar.into_inner()?.finish()?;
        println!("✓ Created {}", self.path.display());
        Ok(())
    }
}

pub struct InterfaceGenerator {
    cargo_toml_content: String,
    lib_rs_content: String,
}

impl InterfaceGenerator {
    /// Prepares the generator by reading the source and converting the manifest.
    /// This performs the generation logic once, in memory.
    pub fn new(root_path: &Path, manifest: &CapabilityManifest) -> Result<Self> {
        let interface_manifest = manifest.clone().to_interface_manifest();
        let cargo_toml_content = toml::to_string_pretty(&interface_manifest)
            .context("Failed to serialize interface manifest")?;

        let source_path = root_path.join("src").join("lib.rs");
        if !source_path.exists() {
            anyhow::bail!("Source file not found: {:?}", source_path);
        }

        let original_source = fs::read_to_string(&source_path)
            .with_context(|| format!("Failed to read source: {:?}", source_path))?;
            
        let lib_rs_content = capability_core::generate_client(&original_source)
            .context("Failed to generate client code")?;

        Ok(Self {
            cargo_toml_content,
            lib_rs_content,
        })
    }

    /// Writes the generated crate to a physical directory on disk.
    /// Also runs rustfmt on the generated source.
    pub fn write_to_disk(&self, output_dir: &Path) -> Result<()> {
        fs::create_dir_all(output_dir)?;

        fs::write(output_dir.join("Cargo.toml"), &self.cargo_toml_content)?;
        println!("  ✓ Wrote interface/Cargo.toml");
        let src_dir = output_dir.join("src");
        fs::create_dir_all(&src_dir)?;
        
        let lib_path = src_dir.join("lib.rs");
        fs::write(&lib_path, &self.lib_rs_content)?;

        println!("  ✓ Wrote interface/src/lib.rs");
        let _ = Command::new("rustfmt").arg(&lib_path).status();

        Ok(())
    }

    pub fn add_to_archive(&self, tar: &mut TarballBuilder) -> Result<()> {
        tar.add_bytes("Cargo.toml", self.cargo_toml_content.as_bytes())?;
        tar.add_bytes("src/lib.rs", self.lib_rs_content.as_bytes())?;
        Ok(())
    }
}
