use crate::artifacts::{Artifacts, CapBinary, CapabilityBinary, CapabilitySource, Interface};
use crate::command::{CommandError, format_syn_error, run_command};
use crate::cache::{BuildError, CacheError};
use crate::cargo::{CapabilityIdent, CapabilityManifest};
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

/// Central context to manage cargo compilation environment
pub struct Environment {
    pub root: PathBuf,
    pub target_dir: PathBuf,
    pub manifest: CapabilityManifest,
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
    async fn ensure_cargo_toml(root: &Path, manifest: &CapabilityManifest) -> EnvResult<()> {
        let cargo_toml_path = root.join("Cargo.toml");
        if cargo_toml_path.exists() {
            return Ok(());
        }
        let cargo_manifest = manifest.clone().to_capability_manifest();
        let contents = toml::to_string_pretty(&cargo_manifest)
            .map_err(|e| EnvironmentError::ParseManifest(e.to_string()))?;
        fs::write(&cargo_toml_path, contents).await?;
        Ok(())
    }

    pub fn name(&self) -> String {
        self.manifest.capability.name.clone()
    }

    pub fn version(&self) -> String {
        self.manifest.capability.version.clone()
    }

    pub fn author(&self) -> String {
        self.manifest.capability.author.clone()
    }

    /// Detect Capability.toml to extract name and version
    async fn load_manifest(root: &Path) -> EnvResult<CapabilityManifest> {
        tracing::debug!("Loading manifest from {:?}", root);
        let capability_toml = root.join("Capability.toml");
        if capability_toml.exists() {
            let content = tokio::fs::read_to_string(&capability_toml).await?;
            let manifest: CapabilityManifest = toml::from_str(&content)
                .map_err(|e| EnvironmentError::ParseManifest(format!("Capability.toml: {}", e)))?;
            return Ok(manifest);
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

    pub async fn package(&self, capture: bool) -> EnvResult<Vec<Artifacts>> {
        let name = self.name();
        let version = self.version();
        let author = self.author();

        tracing::info!("Packaging capability: {:?}", self.root);

        let cargo_toml = toml::to_string_pretty(&self.manifest.clone().to_capability_manifest())
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

        let (interface_rs, interface) =
            pyro_macro::ffi::generate_interface(&src_lib_rs, &name, &version).map_err(|r| {
                EnvironmentError::InterfaceGeneration(format_syn_error(&src_lib_rs, r))
            })?;

        let interface_rs = prettyplease::unparse(&interface_rs);

        Ok(vec![
            Artifacts::CapabilitySource(CapabilitySource {
                manifest: self.manifest.clone(),
                cargo_toml,
                cargo_lock,
                src_lib_rs,
            }),
            Artifacts::CapabilityBinary(CapabilityBinary {
                ident: CapabilityIdent {
                    name,
                    version,
                    author,
                },
                libs: vec![lib],
                interface: interface.clone(),
            }),
            Artifacts::Interface(Interface {
                manifest: self.manifest.clone(),
                src_lib_rs: interface_rs,
                interface,
            }),
        ])
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
