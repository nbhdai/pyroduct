use anyhow::{Context, Result};
use flate2::Compression;
use flate2::write::GzEncoder;
use fs_err as fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tar::Builder;

use crate::cargo::CapabilityManifest;

/// Central context to reduce argument passing
pub struct ProjectContext<'a> {
    pub root: &'a Path,
    pub output_dir: &'a Path,
    pub name: String,
    pub version: String,
}

impl<'a> ProjectContext<'a> {
    pub fn new(
        root: &'a Path,
        output_dir: &'a Path,
        name: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
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
        self.output_dir
            .join(format!("{}-{}.{}", self.name, self.version, suffix))
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
            prefix: &str,
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
    root_path: PathBuf,
    cargo_toml_content: String,
    lib_rs_content: String,
    doc_string: String,
    config_string: Option<String>,
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
        let (cap_name, cap_version) = manifest.name_version()?;

        let original_source = fs::read_to_string(&source_path)
            .with_context(|| format!("Failed to read source: {:?}", source_path))?;

        let (lib_rs_content, doc_string, config_string) =
            capability_core::generate_interface(&original_source, &cap_name, &cap_version)
                .context("Failed to generate client code")?;

        Ok(Self {
            root_path: root_path.to_path_buf(),
            cargo_toml_content,
            lib_rs_content,
            doc_string,
            config_string,
        })
    }

    /// Helper to generate a lockfile string by creating a temporary cargo project.
    fn generate_lockfile_content(&self) -> Result<String> {
        // Create a unique temporary directory INSIDE the project root.
        // This ensures relative paths in Cargo.toml resolve correctly.
        let temp_dir = self
            .root_path
            .join(format!(".interface-gen-{}", std::process::id()));

        if temp_dir.exists() {
            fs::remove_dir_all(&temp_dir)?;
        }
        fs::create_dir_all(&temp_dir)?;

        // Wrap operations in a closure to ensure cleanup runs even if errors occur
        let result = {
            fs::write(temp_dir.join("Cargo.toml"), &self.cargo_toml_content)?;
            let src_dir = temp_dir.join("src");
            fs::create_dir_all(&src_dir)?;
            fs::write(src_dir.join("lib.rs"), &self.lib_rs_content)?;

            let status = Command::new("cargo")
                .arg("generate-lockfile")
                .current_dir(&temp_dir)
                .status()
                .context("Failed to run cargo generate-lockfile")?;

            if !status.success() {
                anyhow::bail!("cargo generate-lockfile failed");
            }

            Ok(fs::read_to_string(temp_dir.join("Cargo.lock"))?)
        };

        let _ = fs::remove_dir_all(&temp_dir);

        result
    }

    /// Writes the generated crate to a physical directory on disk.
    /// Also runs rustfmt on the generated source and generates a lockfile.
    pub fn write_to_disk(&self, output_dir: &Path, lockfile: bool) -> Result<()> {
        fs::create_dir_all(output_dir)?;

        fs::write(output_dir.join("Cargo.toml"), &self.cargo_toml_content)?;
        println!("  ✓ Wrote interface/Cargo.toml");
        let src_dir = output_dir.join("src");
        fs::create_dir_all(&src_dir)?;

        let lib_path = src_dir.join("lib.rs");
        fs::write(&lib_path, &self.lib_rs_content)?;

        println!("  ✓ Wrote interface/src/lib.rs");
        let _ = Command::new("rustfmt").arg(&lib_path).status();
        if lockfile {
            // Generate lockfile in place
            let status = Command::new("cargo")
                .arg("generate-lockfile")
                .current_dir(output_dir)
                .status()
                .context("Failed to run cargo generate-lockfile")?;

            if status.success() {
                println!("  ✓ Generated interface/Cargo.lock");
            } else {
                eprintln!("  ! Warning: Failed to generate interface/Cargo.lock");
            }
        }

        Ok(())
    }

    pub fn add_to_archive(&self, tar: &mut TarballBuilder, lockfile: bool) -> Result<()> {
        tar.add_bytes("Cargo.toml", self.cargo_toml_content.as_bytes())?;
        tar.add_bytes("src/lib.rs", self.lib_rs_content.as_bytes())?;

        if lockfile {
            // Attempt to generate and add the lockfile
            match self.generate_lockfile_content() {
                Ok(lock_content) => {
                    tar.add_bytes("Cargo.lock", lock_content.as_bytes())?;
                }
                Err(e) => {
                    // We warn but do not fail the entire package operation if network/cargo fails
                    eprintln!(
                        "  ! Warning: Could not generate Cargo.lock for archive: {}",
                        e
                    );
                }
            }
        }

        Ok(())
    }

    pub fn spec(&self) -> &str {
        &self.doc_string
    }

    pub fn config(&self) -> Option<&str> {
        self.config_string.as_ref().map(|s| s.as_str())
    }
}
