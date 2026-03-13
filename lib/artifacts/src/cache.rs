use crate::artifacts::{AnonModule, Artifact, Artifacts, Capability, Interface, Module, ModuleDependencies}; use crate::build::{CommandError, run_command};
// Ensure you have this import
use crate::cargo::{CapabilityManifest, ModuleManifest};
use crate::environment::{ResolvedCapability, format_syn_error};
use cargo_toml::Dependency;
use pyro_core::module::generate_module_spec;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, thiserror::Error)]
#[error("{context}: {error}")]
pub struct CacheError {
    pub context: String,
    #[source]
    pub error: std::io::Error,
}

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("IO error — {context}: {error}")]
    Io {
        context: &'static str,
        #[source]
        error: std::io::Error,
    },

    #[error("Cargo error: {0}")]
    Command(#[from] CommandError),

    #[error("Manifest parse error: {0}")]
    Manifest(String),

    #[error("Documentation error: {0}")]
    Documentation(String),
}

impl From<std::io::Error> for BuildError {
    fn from(e: std::io::Error) -> Self {
        BuildError::Io {
            context: "unexpected IO error",
            error: e,
        }
    }
}

impl BuildError {
    pub fn io(context: &'static str,  error: std::io::Error) -> Self {
        BuildError::Io { context, error }
    }
}

pub struct CacheManager {
    pub(crate) root: PathBuf,
    target_dir: PathBuf,
    pyroduct_dep: Dependency,
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

        let mut manager = Self { root, target_dir: PathBuf::new(), pyroduct_dep: Dependency::Simple("*".to_string()) };
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
    pub async fn init(&mut self) -> Result<(), CacheError> {
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
            self.target_dir = target;
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
            self.target_dir = self.root.join("target");
        }
        if let Some(pyroduct_dep) = config.pyroduct.as_ref() {
            self.pyroduct_dep = pyroduct_dep.clone();
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

    /// Compile the module written by `set_build` and store the wasm as an anon
    /// artifact. Returns the hex SHA-256 hash that identifies it in the cache.
    pub async fn compile_anon(&self,
        dependencies: BTreeMap<String, Dependency>,
        capabilities: Vec<ResolvedCapability>,
        code: &str,
    ) -> Result<AnonModule, BuildError> {
        let build_dir = self.root.join("build");
        let src_dir = build_dir.join("src");
        fs::create_dir_all(&src_dir).await.map_err(|e| BuildError::io("create src dir", e))?;
        fs::write(src_dir.join("lib.rs"), code).await.map_err(|e| BuildError::io("write lib.rs", e))?;

        let basic_toml = r#"
[module]
name = "mod"
version = "0.1.0"
authors = ["anon"]
edition = "2024"

[pyroduct]
version = "*"
"#;

        let mut manifest: ModuleManifest = toml::from_str(basic_toml)
            .map_err(|e| BuildError::Manifest(format!("Couldn't build base manifest: {}", e)))?;

        manifest.pyroduct = self.pyroduct_dep.clone();
        for (dep_name, dep) in dependencies.iter() {
            manifest.dependencies.insert(dep_name.clone(), dep.clone());
        }
        for cap in capabilities.iter() {
            let dep = Dependency::Detailed(Box::new(cargo_toml::DependencyDetail {
                path: Some(cap.interface_dir().to_string_lossy().into_owned()),
                ..Default::default()
            }));
            manifest.dependencies.insert(cap.package.clone(), dep);
        }

        let cargo_toml_content =
            toml::to_string_pretty(&manifest).map_err(|e| BuildError::Manifest(e.to_string()))?;
        fs::write(build_dir.join("Cargo.toml"), &cargo_toml_content).await.map_err(|e| BuildError::io("write Cargo.toml", e))?;


        run_command(&build_dir, &["--target", "wasm32-unknown-unknown", "-p", "mod"], false)
            .await?;
        let wasm_path = self
            .target_dir
            .join("wasm32-unknown-unknown")
            .join("release")
            .join("mod.wasm");

        let wasm: Vec<u8> = tokio::fs::read(wasm_path).await.map_err(|e| BuildError::io("read compiled wasm", e))?;

        let spec = generate_module_spec(code).map_err(|s| BuildError::Documentation(format_syn_error("Cannot generate docstring", s)))?
        .ok_or(BuildError::Documentation("Module main functions is missing".to_string()))?;
        
        let dependencies = ModuleDependencies {
            dependencies,
            capabilities,
        };
        

        Ok(AnonModule {source: code.to_string(), wasm, spec, dependencies})
    }

    

    pub async fn write_artifacts(&self, artifacts: Artifacts) -> Result<(), CacheError> {
        // 1. Determine the target directory based on the artifact type
        let dir = match &artifacts {
            Artifacts::Module(Module { manifest, .. }) => {
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
            Artifacts::Capability(Capability { manifest, .. }) => {
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
            Artifacts::Interface(Interface { manifest, .. }) => {
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
            Artifacts::AnonModule(AnonModule { wasm, .. }) => {
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
