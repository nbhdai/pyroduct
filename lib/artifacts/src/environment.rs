use crate::artifacts::{Artifact, Artifacts, CapBinary, Capability, Interface};
use crate::build::{CommandError, run_command};
use crate::cache::{BuildError, CacheError, CacheManager};
use crate::cargo::{CapabilityManifest, ModuleManifest};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::fs;
use tokio::process::Command;

#[derive(Error, Debug)]
pub enum EnvironmentError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Cargo metadata failed: {0}")]
    Metadata(String),

    #[error(transparent)]
    CommandError(#[from] CommandError),

    #[error("Failed to parse or write: {0}")]
    Serde(String),

    #[error("Missing target directory in metadata")]
    MissingTargetDir,

    #[error("Artifact not found: {0}")]
    ArtifactNotFound(PathBuf),

    #[error("Utf8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("Failed to parse manifest: {0}")]
    ParseManifest(String),

    #[error("Interface generation failed: {0}")]
    InterfaceGeneration(String),

    #[error("Source not found: {0}")]
    SourceNotFound(PathBuf),

    #[error("Cache error: {0}")]
    Cache(#[from] CacheError),

    #[error("Build error: {0}")]
    Build(#[from] BuildError),
}

impl From<serde_json::Error> for EnvironmentError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value.to_string())
    }
}

pub type EnvResult<T> = std::result::Result<T, EnvironmentError>;

pub enum Manifest {
    Module(ModuleManifest),
    Capability(CapabilityManifest),
    Interface(CapabilityManifest),
}

/// Central context to manage cargo compilation environment
pub struct Environment {
    pub root: PathBuf,
    pub target_dir: PathBuf,
    pub manifest: Manifest,
}

impl Environment {
    /// Create a new Environment by fetching metadata and detecting manifests from the given root
    pub async fn new(root: PathBuf) -> EnvResult<Self> {
        let manifest = Self::load_manifest(&root).await?;
        Self::ensure_cargo_toml(&root, &manifest).await?;
        let target_dir = Self::get_target_dir(&root).await?;
        Ok(Self {
            root,
            target_dir,
            manifest,
        })
    }

    /// Write Cargo.toml from Module.toml or Capability.toml if it doesn't exist
    async fn ensure_cargo_toml(root: &Path, manifest: &Manifest) -> EnvResult<()> {
        let cargo_toml_path = root.join("Cargo.toml");
        if cargo_toml_path.exists() {
            return Ok(());
        }
        let cargo_manifest = match manifest {
            Manifest::Module(m) => m.clone().to_cargo(),
            Manifest::Capability(m) => m.clone().to_capability_manifest(),
            Manifest::Interface(m) => m.clone().to_capability_manifest(),
        };
        let contents = toml::to_string_pretty(&cargo_manifest)
            .map_err(|e| EnvironmentError::ParseManifest(e.to_string()))?;
        fs::write(&cargo_toml_path, contents).await?;
        Ok(())
    }

    pub fn is_capability(&self) -> bool {
        match self.manifest {
            Manifest::Capability(_) => true,
            _ => false,
        }
    }

    pub fn name(&self) -> Option<String> {
        match &self.manifest {
            Manifest::Module(m) => Some(m.module.name.clone()),
            Manifest::Capability(m) => Some(m.capability.name.clone()),
            Manifest::Interface(_) => None,
        }
    }

    pub fn version(&self) -> Option<String> {
        match &self.manifest {
            Manifest::Module(m) => Some(m.module.version.clone()),
            Manifest::Capability(m) => Some(m.capability.version.clone()),
            Manifest::Interface(_) => None,
        }
    }

    pub fn author(&self) -> Option<String> {
        match &self.manifest {
            Manifest::Module(m) => Some(m.module.author.clone()),
            Manifest::Capability(m) => Some(m.capability.author.clone()),
            Manifest::Interface(_) => None,
        }
    }

    /// Detect Module.toml, Capability.toml, or Cargo.toml to extract name and version
    async fn load_manifest(root: &Path) -> EnvResult<Manifest> {
        let module_toml = root.join("Module.toml");
        if module_toml.exists() {
            let content = tokio::fs::read_to_string(&module_toml).await?;
            let manifest: ModuleManifest = toml::from_str(&content)
                .map_err(|e| EnvironmentError::ParseManifest(format!("Module.toml: {}", e)))?;
            return Ok(Manifest::Module(manifest));
        }

        let capability_toml = root.join("Capability.toml");
        if capability_toml.exists() {
            let content = tokio::fs::read_to_string(&capability_toml).await?;
            let manifest: CapabilityManifest = toml::from_str(&content)
                .map_err(|e| EnvironmentError::ParseManifest(format!("Capability.toml: {}", e)))?;
            return Ok(Manifest::Capability(manifest));
        }

        // Default for anon compilations or when no package section is found
        Err(EnvironmentError::ParseManifest(
            "No manifest found".to_string(),
        ))
    }

    /// Run `cargo metadata` to find the target directory
    async fn get_target_dir(path: &Path) -> EnvResult<PathBuf> {
        let output = Command::new("cargo")
            .args(["metadata", "--format-version=1", "--no-deps"])
            .current_dir(path)
            .output()
            .await?;

        if !output.status.success() {
            return Err(EnvironmentError::Metadata(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)?;

        metadata["target_directory"]
            .as_str()
            .map(PathBuf::from)
            .ok_or(EnvironmentError::MissingTargetDir)
    }

    pub async fn generate_lockfile(&self) -> EnvResult<String> {
        run_command(&self.root, &["generate-lockfile"], true).await?;

        Ok(fs::read_to_string(self.root.join("Cargo.lock")).await?)
    }

    /// Compile the project (defaults to release)
    pub async fn compile(&self, extra_args: &[&str], capture: bool) -> EnvResult<()> {
        let mut args = vec!["build", "--release"];
        args.extend_from_slice(extra_args);
        run_command(&self.root, &args, capture).await?;
        Ok(())
    }

    /// Get path to the compiled wasm artifact
    pub fn get_wasm_artifact(&self, name: &str) -> EnvResult<PathBuf> {
        let path = self
            .target_dir
            .join("wasm32-unknown-unknown")
            .join("release")
            .join(format!("{}.wasm", name.replace('-', "_")));

        if path.exists() {
            Ok(path)
        } else {
            Err(EnvironmentError::ArtifactNotFound(path))
        }
    }

    /// Get path to the compiled library artifact (dylib/so/dll)
    pub async fn get_library_artifact(&self, name: &str) -> EnvResult<CapBinary> {
        let ext = dylib_extension();
        let path =
            self.target_dir
                .join("release")
                .join(format!("lib{}.{}", name.replace('-', "_"), ext));
        if path.exists() {
            match ext {
                "dylib" => Ok(CapBinary::MachO(fs::read(&path).await?)),
                "so" => Ok(CapBinary::Elf(fs::read(&path).await?)),
                "dll" => Ok(CapBinary::Pe(fs::read(&path).await?)),
                _ => Err(EnvironmentError::ArtifactNotFound(path)),
            }
        } else {
            Err(EnvironmentError::ArtifactNotFound(path))
        }
    }

    pub async fn package(&self, capture: bool) -> EnvResult<Artifacts> {
        let name = self.name().ok_or_else(|| {
            EnvironmentError::ParseManifest("Missing name in manifest".to_string())
        })?;

        let artifacts = match &self.manifest {
            Manifest::Module(manifest) => {
                let src_path = self.root.join("src").join("lib.rs");
                if !src_path.exists() {
                    return Err(EnvironmentError::SourceNotFound(src_path));
                }
                let code = fs::read_to_string(&src_path).await?;
                let manager = CacheManager::new().await?;
                let anon = manager
                    .compile_anon(
                        manifest.dependencies.clone(),
                        manifest.capabilities.values().cloned().collect(),
                        &code,
                    )
                    .await?;
                Artifacts::Module(anon)
            }
            Manifest::Capability(manifest) => {
                tracing::info!("Packaging capability: {:?}", self.root);

                let cargo_toml = toml::to_string_pretty(&manifest.clone().to_capability_manifest())
                    .map_err(|e| EnvironmentError::ParseManifest(e.to_string()))?;

                tracing::info!("Compiling capability binary...");
                self.compile(&["--features", "capability", "-p", &name], capture)
                    .await?;

                let lib = self.get_library_artifact(&name).await?;

                let lock_path = self.root.join("Cargo.lock");
                let cargo_lock = if lock_path.exists() {
                    fs::read_to_string(&lock_path).await?
                } else {
                    String::new()
                };

                let src_path = self.root.join("src").join("lib.rs");
                let src_lib_rs = if src_path.exists() {
                    fs::read_to_string(&src_path).await?
                } else {
                    String::new()
                };

                let spec_path = self.root.join("interface.json");
                let interface_json = if spec_path.exists() {
                    fs::read_to_string(&spec_path).await?
                } else {
                    String::new()
                };

                let config_path = self.root.join("config.json");
                let config_json = if config_path.exists() {
                    Some(fs::read_to_string(&config_path).await?)
                } else {
                    None
                };

                Artifacts::Capability(Capability {
                    manifest: manifest.clone(),
                    libs: vec![lib],
                    cargo_toml,
                    cargo_lock,
                    src_lib_rs,
                    interface_json,
                    config_json,
                })
            }
            Manifest::Interface(manifest) => {
                tracing::info!("Packaging interface: {:?}", self.root);

                let src_path = self.root.join("src").join("lib.rs");
                let src_lib_rs = if src_path.exists() {
                    fs::read_to_string(&src_path).await?
                } else {
                    String::new()
                };

                let spec_path = self.root.join("interface.json");
                let interface_json = if spec_path.exists() {
                    fs::read_to_string(&spec_path).await?
                } else {
                    String::new()
                };

                let config_path = self.root.join("config.json");
                let config_json = if config_path.exists() {
                    Some(fs::read_to_string(&config_path).await?)
                } else {
                    None
                };

                Artifacts::Interface(Interface {
                    manifest: manifest.clone(),
                    src_lib_rs,
                    interface_json,
                    config_json,
                })
            }
        };

        Ok(artifacts)
    }

    /// Creates an interface environment from a capability environment in the directory specified.
    pub async fn create_interface(&self) -> EnvResult<Option<Interface>> {
        let manifest = match &self.manifest {
            Manifest::Capability(capability_manifest) => capability_manifest,
            _ => return Ok(None),
        };

        let cap_name = manifest.capability.name.clone();
        let cap_version = manifest.capability.version.clone();

        let source_path = self.root.join("src").join("lib.rs");
        if !source_path.exists() {
            return Err(EnvironmentError::SourceNotFound(source_path));
        }

        let original_source = fs::read_to_string(&source_path).await?;

        let (lib_rs_file, spec_res, config_res) =
            pyro_core::ffi::generate_interface(&original_source, &cap_name, &cap_version).map_err(
                |r| EnvironmentError::InterfaceGeneration(format_syn_error(&original_source, r)),
            )?;

        let lib_rs_content = prettyplease::unparse(&lib_rs_file);
        let spec = spec_res.map_err(|e| EnvironmentError::Serde(e.to_string()))?;
        let config = config_res
            .transpose()
            .map_err(|e| EnvironmentError::Serde(e.to_string()))?;

        Ok(Some(Interface {
            manifest: manifest.clone(),
            src_lib_rs: lib_rs_content,
            interface_json: spec,
            config_json: config,
        }))
    }

    pub async fn new_interface(root: PathBuf, interface: Interface) -> EnvResult<Self> {
        interface.write_to_directory(&root).await?;
        Self::new(root).await
    }
}

pub fn dylib_extension() -> &'static str {
    if cfg!(target_os = "macos") {
        "dylib"
    } else if cfg!(target_os = "windows") {
        "dll"
    } else {
        "so"
    }
}

/// Format a syn::Error with source context
pub fn format_syn_error(source: &str, err: syn::Error) -> String {
    let span = err.span();
    let start = span.start();
    let msg = err.to_string();

    let lines: Vec<&str> = source.lines().collect();
    let line_num = start.line;
    let col = start.column;

    let mut output = String::new();
    output.push_str(&format!("error: {}\n", msg));
    output.push_str(&format!("  --> src/lib.rs:{}:{}\n", line_num, col + 1));
    output.push_str("   |\n");

    // Show context: line before, error line, line after
    let start_line = line_num.saturating_sub(2);
    let end_line = (line_num + 1).min(lines.len());

    for i in start_line..end_line {
        let line_content = lines.get(i).unwrap_or(&"");
        let display_num = i + 1;

        if display_num == line_num {
            output.push_str(&format!("{:3} | {}\n", display_num, line_content));
            // Add caret pointing to the column
            output.push_str(&format!("    | {}^\n", " ".repeat(col)));
        } else {
            output.push_str(&format!("{:3} | {}\n", display_num, line_content));
        }
    }
    output.push_str("   |\n");

    output
}
