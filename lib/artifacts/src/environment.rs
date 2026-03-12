use crate::cargo::{CapabilityManifest, ModuleManifest};
use crate::utils::{InterfaceGenerator, TarballBuilder};
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

    #[error(
        "Cargo command failed with status {status}. Args: {args:?}\nStdout: {stdout}\nStderr: {stderr}"
    )]
    CargoCommand {
        status: std::process::ExitStatus,
        args: Vec<String>,
        stdout: String,
        stderr: String,
    },

    #[error("Failed to parse cargo metadata: {0}")]
    ParseMetadata(#[from] serde_json::Error),

    #[error("Missing target directory in metadata")]
    MissingTargetDir,

    #[error("Artifact not found: {0}")]
    ArtifactNotFound(PathBuf),

    #[error("Utf8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("Failed to parse manifest: {0}")]
    ParseManifest(String),
}

pub type EnvResult<T> = std::result::Result<T, EnvironmentError>;

pub struct Artifact {
    pub name: String,
    pub data: Vec<u8>,
}

pub struct PackageResult {
    pub name: String,
    pub version: String,
    pub artifacts: Vec<Artifact>,
}

pub enum Manifest {
    Module(ModuleManifest),
    Capability(CapabilityManifest),
    Anon(cargo_toml::Manifest),
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
        let target_dir = Self::get_target_dir(&root).await?;
        let manifest = Self::load_manifest(&root).await?;
        Ok(Self {
            root,
            target_dir,
            manifest,
        })
    }

    pub fn name(&self) -> Option<String> {
        match &self.manifest {
            Manifest::Module(m) => m.module.as_ref().map(|p| p.name.clone()),
            Manifest::Capability(m) => m.capability.as_ref().map(|p| p.name.clone()),
            Manifest::Anon(_) => None,
        }
    }

    pub fn version(&self) -> Option<String> {
        match &self.manifest {
            Manifest::Module(m) => m
                .module
                .as_ref()
                .and_then(|p| p.version.get().ok())
                .cloned(),
            Manifest::Capability(m) => m
                .capability
                .as_ref()
                .and_then(|p| p.version.get().ok())
                .cloned(),
            Manifest::Anon(_) => None,
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

        let cargo_toml = root.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = tokio::fs::read_to_string(&cargo_toml).await?;
            let manifest = cargo_toml::Manifest::from_str(&content)
                .map_err(|e| EnvironmentError::ParseManifest(format!("Cargo.toml: {}", e)))?;
            return Ok(Manifest::Anon(manifest));
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

    /// Run a cargo command within this environment
    pub async fn run_command(&self, tool_args: &[&str], capture: bool) -> EnvResult<String> {
        let mut cmd = Command::new("cargo");
        cmd.args(tool_args).current_dir(&self.root);

        if capture {
            let output = cmd.output().await?;

            if !output.status.success() {
                return Err(EnvironmentError::CargoCommand {
                    status: output.status,
                    args: tool_args.iter().map(|s| s.to_string()).collect(),
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                });
            }
            Ok(String::from_utf8(output.stdout)?)
        } else {
            let status = cmd.status().await?;

            if !status.success() {
                return Err(EnvironmentError::CargoCommand {
                    status,
                    args: tool_args.iter().map(|s| s.to_string()).collect(),
                    stdout: String::from("Not captured"),
                    stderr: String::from("Not captured"),
                });
            }
            Ok(String::new())
        }
    }

    /// Compile the project (defaults to release)
    pub async fn compile(&self, extra_args: &[&str]) -> EnvResult<()> {
        let mut args = vec!["build", "--release"];
        args.extend_from_slice(extra_args);
        self.run_command(&args, false).await?;
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
    pub fn get_library_artifact(&self, name: &str) -> EnvResult<PathBuf> {
        let ext = dylib_extension();
        let path =
            self.target_dir
                .join("release")
                .join(format!("lib{}.{}", name.replace('-', "_"), ext));

        if path.exists() {
            Ok(path)
        } else {
            Err(EnvironmentError::ArtifactNotFound(path))
        }
    }

    // ============================================================
    // Module Packaging
    // ============================================================

    pub async fn package_module(
        &self,
        manifest: ModuleManifest,
        _capture: bool,
    ) -> EnvResult<PackageResult> {
        let name = self.name().ok_or_else(|| {
            EnvironmentError::ParseManifest("Missing name in manifest".to_string())
        })?;
        let version = self.version().ok_or_else(|| {
            EnvironmentError::ParseManifest("Missing version in manifest".to_string())
        })?;

        tracing::info!("Packaging module: {:?}", self.root);

        // 1. Generate Cargo.toml
        let cargo_toml_content = toml::to_string_pretty(&manifest.to_cargo())
            .map_err(|e| EnvironmentError::ParseManifest(e.to_string()))?;

        // 2. Build WASM
        tracing::info!("Compiling WASM module...");
        self.compile(&["--target", "wasm32-unknown-unknown", "-p", &name])
            .await?;

        // 3. Locate Artifact
        let built_wasm = self.get_wasm_artifact(&name)?;
        let wasm_bytes = fs::read(&built_wasm).await?;

        // 4. Generate module spec (module.json)
        let src_path = self.root.join("src").join("lib.rs");
        let module_spec = if src_path.exists() {
            let content = fs::read_to_string(&src_path).await?;
            pyro_core::module::generate_module_spec(&content).map_err(|e| {
                EnvironmentError::ParseManifest(format!("Failed to generate module spec: {}", e))
            })?
        } else {
            None
        };

        // 5. Create Archive
        let mut tar = TarballBuilder::new().map_err(EnvironmentError::Io)?;
        tar.add_bytes("Cargo.toml", cargo_toml_content.as_bytes())
            .map_err(EnvironmentError::Io)?;
        tar.add_bytes("mod.wasm", &wasm_bytes)
            .map_err(EnvironmentError::Io)?;
        tar.add_dir(&self.root.join("src"), "src")
            .map_err(EnvironmentError::Io)?;

        if let Some(spec) = module_spec {
            tar.add_bytes("module.json", spec.as_bytes())
                .map_err(EnvironmentError::Io)?;
            tracing::info!("✓ Added module.json to archive");
        }

        let tar_data = tar.finish().map_err(EnvironmentError::Io)?;
        let artifact_name = format!("{}-{}.module", name, version);

        Ok(PackageResult {
            name,
            version,
            artifacts: vec![Artifact {
                name: artifact_name,
                data: tar_data,
            }],
        })
    }

    // ============================================================
    // Capability Packaging
    // ============================================================

    pub async fn package_capability(
        &self,
        manifest: CapabilityManifest,
        _capture: bool,
    ) -> EnvResult<PackageResult> {
        let name = self.name().ok_or_else(|| {
            EnvironmentError::ParseManifest("Missing name in manifest".to_string())
        })?;
        let version = self.version().ok_or_else(|| {
            EnvironmentError::ParseManifest("Missing version in manifest".to_string())
        })?;

        tracing::info!("Packaging capability: {:?}", self.root);

        // 1. Generate Cargo.toml content
        let cargo_toml_content = toml::to_string_pretty(&manifest.clone().to_capability_manifest())
            .map_err(|e| EnvironmentError::ParseManifest(e.to_string()))?;

        // 2. Build Dynamic Library
        tracing::info!("Compiling capability binary...");
        self.compile(&["--features", "capability", "-p", &name])
            .await?;

        // 3. Locate Artifact
        let built_lib = self.get_library_artifact(&name)?;
        let lib_bytes = fs::read(&built_lib).await?;

        // 4. Create Source Archive (.cap)
        let mut cap_tar = TarballBuilder::new().map_err(EnvironmentError::Io)?;
        cap_tar
            .add_bytes("Cargo.toml", cargo_toml_content.as_bytes())
            .map_err(EnvironmentError::Io)?;
        cap_tar
            .add_bytes(&format!("lib.{}", dylib_extension()), &lib_bytes)
            .map_err(EnvironmentError::Io)?;
        cap_tar
            .add_dir(&self.root.join("src"), "src")
            .map_err(EnvironmentError::Io)?;

        // 5. Interface Generation
        let interface =
            InterfaceGenerator::new(&self.root, &manifest).map_err(EnvironmentError::Io)?;

        // 6. Create Interface Archive (.interface)
        let mut interface_tar = TarballBuilder::new().map_err(EnvironmentError::Io)?;
        interface
            .add_to_archive(&mut interface_tar, true)
            .map_err(EnvironmentError::Io)?;
        interface_tar
            .add_bytes("interface.json", interface.spec().as_bytes())
            .map_err(EnvironmentError::Io)?;

        // 7. Add config spec to .cap
        if let Some(spec) = interface.config() {
            cap_tar
                .add_bytes("config.json", spec.as_bytes())
                .map_err(EnvironmentError::Io)?;
        }

        let cap_data = cap_tar.finish().map_err(EnvironmentError::Io)?;
        let interface_data = interface_tar.finish().map_err(EnvironmentError::Io)?;

        let cap_name = format!("{}-{}.cap", name, version);
        let interface_name = format!("{}-{}.interface", name, version);

        Ok(PackageResult {
            name,
            version,
            artifacts: vec![
                Artifact {
                    name: cap_name,
                    data: cap_data,
                },
                Artifact {
                    name: interface_name,
                    data: interface_data,
                },
            ],
        })
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
