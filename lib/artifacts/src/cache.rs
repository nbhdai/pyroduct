use crate::artifacts::Artifacts; // Ensure you have this import
use crate::cargo::{CapabilityManifest, ModuleManifest};
use cargo_toml::Dependency;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, thiserror::Error)]
#[error("{context}: {error}")]
pub struct CacheError {
    pub context: String,
    #[source]
    pub error: std::io::Error,
}

pub struct CacheManager {
    pub(crate) root: PathBuf,
}

impl CacheManager {
    pub async fn new() -> Result<Self, CacheError> {
        let root = std::env::var("PYRODUCT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME")
                    .or_else(|_| std::env::var("USERPROFILE"))
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| PathBuf::from("."));
                home.join(".pyroduct")
            });

        let manager = Self { root };
        manager.init().await?;
        Ok(manager)
    }

    pub async fn config(&self) -> PyroductConfig {
        let path = self.root.join("config.toml");
        // Using std::fs here so it can be evaluated synchronously without an .await
        if let Ok(content) = fs::read_to_string(&path).await {
            if let Ok(mut config) = toml::from_str::<PyroductConfig>(&content) {
                // If the pyroduct dependency uses a relative path, resolve it
                // to an absolute path anchored at the pyroduct root directory.
                if let Some(dep) = &mut config.pyroduct {
                    resolve_dependency_path(dep, &self.root);
                }
                return config;
            }
        }
        PyroductConfig {
            author: None,
            target: None,
            pyroduct: None,
        }
    }

    /// The folder instantiator
    pub async fn init(&self) -> Result<(), CacheError> {
        fs::create_dir_all(self.capabilities_base_dir())
            .await
            .map_err(|error| CacheError {
                context: "Failed to create capabilities cache dir".to_string(),
                error,
            })?;

        fs::create_dir_all(self.interfaces_dir())
            .await
            .map_err(|error| CacheError {
                context: "Failed to create interfaces cache dir".to_string(),
                error,
            })?;

        let module_dir = self.root.join("modules");
        fs::create_dir_all(&module_dir)
            .await
            .map_err(|error| CacheError {
                context: "Failed to create modules cache dir".to_string(),
                error,
            })?;

        let anon_dir = self.root.join("anon");
        fs::create_dir_all(&anon_dir)
            .await
            .map_err(|error| CacheError {
                context: "Failed to create anon cache dir".to_string(),
                error,
            })?;

        let build_dir = self.root.join("build");
        fs::create_dir_all(build_dir)
            .await
            .map_err(|error| CacheError {
                context: "Failed to create build dir".to_string(),
                error,
            })?;

        let cargo_dir = self.root.join(".cargo");
        fs::create_dir_all(&cargo_dir)
            .await
            .map_err(|error| CacheError {
                context: "Failed to create .cargo dir".to_string(),
                error,
            })?;

        let config = self.config().await;
        if let Some(target) = config.target {
            fs::write(
                cargo_dir.join("config.toml"),
                format!("[build]\ntarget-dir = \"{}\"", target.display()),
            )
            .await
            .map_err(|error| CacheError {
                context: "Failed to write target config.toml".to_string(),
                error,
            })?;
        } else {
            fs::write(
                cargo_dir.join("config.toml"),
                "[build]\ntarget-dir = \"target\"",
            )
            .await
            .map_err(|error| CacheError {
                context: "Failed to write target config.toml".to_string(),
                error,
            })?;
        }
        Ok(())
    }

    pub fn capabilities_base_dir(&self) -> PathBuf {
        self.root.join("capabilities")
    }

    pub fn capabilities_dir(&self, author: &str, name: &str, version: &str) -> PathBuf {
        self.capabilities_base_dir()
            .join(author)
            .join(name)
            .join(version)
    }

    /// Returns the path to the interface crate inside a capability's cache directory.
    pub fn capability_interface_dir(&self, author: &str, name: &str, version: &str) -> PathBuf {
        self.capabilities_dir(author, name, version)
            .join("interface")
    }

    pub fn interfaces_dir(&self) -> PathBuf {
        self.root.join("interfaces")
    }

    pub fn interface_dir(&self, author: &str, name: &str, version: &str) -> PathBuf {
        self.interfaces_dir().join(author).join(name).join(version)
    }

    /// Returns the interface documentation (interface.json) for a shipped capability.
    pub async fn capability_interface_spec(
        &self,
        author: &str,
        name: &str,
        version: &str,
    ) -> Result<String, CacheError> {
        let path = self
            .capabilities_dir(author, name, version)
            .join("interface.json");
        fs::read_to_string(&path).await.map_err(|error| CacheError {
            context: format!("Failed to read interface.json from {}", path.display()),
            error,
        })
    }

    /// Returns the config documentation (config.json) for a shipped capability, if it exists.
    pub async fn capability_config_spec(
        &self,
        author: &str,
        name: &str,
        version: &str,
    ) -> Result<Option<String>, CacheError> {
        let path = self
            .capabilities_dir(author, name, version)
            .join("config.json");
        if path.exists() {
            let content = fs::read_to_string(&path)
                .await
                .map_err(|error| CacheError {
                    context: format!("Failed to read config.json from {}", path.display()),
                    error,
                })?;
            Ok(Some(content))
        } else {
            Ok(None)
        }
    }

    pub async fn add_anon_module(&self, hash: &str, wasm: &[u8]) -> Result<(), CacheError> {
        // Updated to match the new directory structure: anon/{hash}/mod.wasm
        let module_dir = self.root.join("anon").join(hash);
        fs::create_dir_all(&module_dir)
            .await
            .map_err(|error| CacheError {
                context: "Failed to create anon module dir".to_string(),
                error,
            })?;

        let module_path = module_dir.join("mod.wasm");
        fs::write(module_path, wasm)
            .await
            .map_err(|error| CacheError {
                context: "Failed to write module".to_string(),
                error,
            })?;
        Ok(())
    }

    pub async fn write_artifacts(&self, artifacts: Artifacts) -> Result<(), CacheError> {
        // 1. Determine the target directory based on the artifact type
        let dir = match &artifacts {
            Artifacts::Module { manifest, .. } => {
                let m: ModuleManifest = toml::from_str(manifest).map_err(|e| CacheError {
                    context: "Failed to deserialize Module.toml".to_string(),
                    error: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
                })?;
                self.root
                    .join("modules")
                    .join(&m.module.author)
                    .join(&m.module.name)
                    .join(&m.module.version)
            }
            Artifacts::Capability { manifest, .. } => {
                let m: CapabilityManifest = toml::from_str(manifest).map_err(|e| CacheError {
                    context: "Failed to deserialize Capability.toml".to_string(),
                    error: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
                })?;
                self.capabilities_dir(
                    &m.capability.author,
                    &m.capability.name,
                    &m.capability.version,
                )
            }
            Artifacts::Interface { manifest, .. } => {
                let m: CapabilityManifest = toml::from_str(manifest).map_err(|e| CacheError {
                    context: "Failed to deserialize Capability.toml (interface)".to_string(),
                    error: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
                })?;
                self.interface_dir(
                    &m.capability.author,
                    &m.capability.name,
                    &m.capability.version,
                )
            }
            Artifacts::AnonModule { wasm, .. } => {
                let mut hasher = Sha256::new();
                hasher.update(wasm);
                let hash = format!("{:x}", hasher.finalize());
                self.root.join("anon").join(hash)
            }
        };

        // 2. Delegate the actual file writing to the artifact
        artifacts
            .write_to_directory(&dir)
            .await
            .map_err(|e| CacheError {
                context: format!("Failed to write artifacts to {}", dir.display()),
                error: e,
            })?;

        Ok(())
    }

    pub async fn module_dir(
        &self,
        author: &str,
        name: &str,
        version: &str,
    ) -> Result<PathBuf, CacheError> {
        let dir = self
            .root
            .join("modules")
            .join(author)
            .join(name)
            .join(version);
        fs::create_dir_all(&dir).await.map_err(|error| CacheError {
            context: "Failed to create module dir".to_string(),
            error,
        })?;
        Ok(dir)
    }

    pub async fn target_dir(&self) -> PathBuf {
        match self.config().await.target {
            Some(target) => PathBuf::from(target),
            None => self.root.join("target"),
        }
    }
}

/// If a `Dependency` has a relative path, resolve it to absolute
/// with respect to the given `base` directory.
fn resolve_dependency_path(dep: &mut Dependency, base: &std::path::Path) {
    if let Dependency::Detailed(detail) = dep {
        if let Some(ref mut p) = detail.path {
            let path = std::path::Path::new(p.as_str());
            if path.is_relative() {
                let absolute = base.join(&path);
                // Canonicalize if it exists on disk, otherwise just use the joined path.
                *p = absolute
                    .canonicalize()
                    .unwrap_or(absolute)
                    .to_string_lossy()
                    .into_owned();
            }
        }
    }
}

#[derive(serde::Deserialize)]
pub struct PyroductConfig {
    pub author: Option<String>,
    pub target: Option<PathBuf>,
    pub pyroduct: Option<Dependency>,
}
