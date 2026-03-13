use crate::artifacts::{AnonModule, Artifact, Artifacts, ModuleDependencies};
use crate::build::{CommandError, run_command};
// Ensure you have this import
use crate::cargo::{CapabilityManifest, ResolvedCapability};
use crate::environment::format_syn_error;
use cargo_toml::Dependency;
use pyro_core::module::generate_module_spec;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
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
    pub fn io(context: &'static str, error: std::io::Error) -> Self {
        BuildError::Io { context, error }
    }
}

pub struct CacheManager {
    pub(crate) root: PathBuf,
    pub target_dir: PathBuf,
    pub pyroduct_dep: Dependency,
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

        let mut manager = Self {
            root,
            target_dir: PathBuf::new(),
            pyroduct_dep: Dependency::Simple("*".to_string()),
        };
        manager.init().await?;
        Ok(manager)
    }

    pub async fn config(&self) -> Result<PyroductConfig, CacheError> {
        let path = self.root.join("config.toml");
        let content = fs::read_to_string(&path)
            .await
            .map_err(|error| CacheError {
                context: format!("Failed to read the configuration"),
                error,
            })?;
        let mut config =
            toml::from_str::<PyroductConfig>(&content).map_err(|error| CacheError {
                context: format!("Failed to parse the configuration"),
                error: io::Error::new(io::ErrorKind::InvalidData, error),
            })?;
        if let Some(dep) = &mut config.pyroduct {
            resolve_dependency_path(dep, &self.root);
        }
        if let Some(target) = &mut config.target {
            if target.is_relative() {
                *target = self.root.join(&target);
            }
        }
        Ok(config)
    }

    /// The folder instantiator
    pub async fn init(&mut self) -> Result<(), CacheError> {
        fs::create_dir_all(self.capabilities_base_dir())
            .await
            .map_err(|error| CacheError {
                context: format!(
                    "Failed to create capabilities cache dir in {:?}",
                    self.capabilities_base_dir()
                ),
                error,
            })?;

        fs::create_dir_all(self.interfaces_base_dir())
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

        let config = self.config().await?;
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
    pub fn interface_dir(&self, author: &str, name: &str, version: &str) -> PathBuf {
        self.interfaces_base_dir()
            .join(author)
            .join(name)
            .join(version)
    }

    pub fn interfaces_base_dir(&self) -> PathBuf {
        self.root.join("interfaces")
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

    pub async fn get_anon(&self, 
        _dependencies: &BTreeMap<String, Dependency>,
        _capabilities: &Vec<ResolvedCapability>,
        code: &str,
    ) -> Result<Option<AnonModule>, BuildError> {
        let mut hasher = Sha256::new();
        hasher.update(&code);
        // TODO The hash should also depend on the dependencies
        // hasher.update(&anon.dependencies);
        let hash = format!("{:x}", hasher.finalize());
        let path = self.root.join("anon").join(hash);
        if path.exists() {
            let module = AnonModule::from_dir(&path).await?;
            Ok(Some(module))
        } else {
            Ok(None)
        }
    }

    /// Compile the module written by `set_build` and store the wasm as an anon
    /// artifact. Returns the hex SHA-256 hash that identifies it in the cache.
    pub async fn compile_anon(
        &self,
        dependencies: BTreeMap<String, Dependency>,
        capabilities: Vec<ResolvedCapability>,
        code: &str,
    ) -> Result<AnonModule, BuildError> {
        if let Some(module) = self.get_anon(&dependencies, &capabilities, code).await? {
            return Ok(module);
        }

        let build_dir = self.root.join("build");
        let src_dir = build_dir.join("src");
        fs::create_dir_all(&src_dir)
            .await
            .map_err(|e| BuildError::io("create src dir", e))?;
        fs::write(src_dir.join("lib.rs"), code)
            .await
            .map_err(|e| BuildError::io("write lib.rs", e))?;

        let basic_toml = r#"
[package]
name = "mod"
version = "0.1.0"
author = "anon"
edition = "2024"

[workspace]

[dependencies]
"#;

        let mut manifest: cargo_toml::Manifest = toml::from_str(basic_toml)
            .map_err(|e| BuildError::Manifest(format!("Couldn't build base manifest: {}", e)))?;
        let mut pyro_dep = self.pyroduct_dep.clone();
        pyro_dep.detail_mut().features.push("module".to_string());
        manifest
            .dependencies
            .insert("pyroduct".to_string(), pyro_dep);
        for (dep_name, dep) in dependencies.iter() {
            manifest.dependencies.insert(dep_name.clone(), dep.clone());
        }
        for cap in capabilities.iter() {
            let path = Path::new("../")
                .join(self.interface_dir(&cap.author, &cap.package, &cap.version))
                .to_string_lossy()
                .into();
            let dep = Dependency::Detailed(Box::new(cargo_toml::DependencyDetail {
                path: Some(path),
                ..Default::default()
            }));
            manifest.dependencies.insert(cap.package.clone(), dep);
        }
        manifest.lib = crate::cargo::ensure_cdylib(manifest.lib.take());

        let cargo_toml_content =
            toml::to_string_pretty(&manifest).map_err(|e| BuildError::Manifest(e.to_string()))?;
        fs::write(build_dir.join("Cargo.toml"), &cargo_toml_content)
            .await
            .map_err(|e| BuildError::io("write Cargo.toml", e))?;

        run_command(
            &build_dir,
            &["build", "--release", "--target", "wasm32-unknown-unknown"],
            true,
        )
        .await?;
        let wasm_path = self
            .target_dir
            .join("wasm32-unknown-unknown")
            .join("release")
            .join("mod.wasm");

        let wasm: Vec<u8> = tokio::fs::read(wasm_path)
            .await
            .map_err(|e| BuildError::io("read compiled wasm", e))?;

        let spec = generate_module_spec(code)
            .map_err(|s| {
                BuildError::Documentation(format_syn_error("Cannot generate docstring", s))
            })?
            .ok_or(BuildError::Documentation(
                "Module main functions is missing".to_string(),
            ))?;

        let dependencies = ModuleDependencies {
            dependencies,
            capabilities,
        };

        let module = AnonModule {
            source: code.to_string(),
            wasm,
            spec,
            dependencies,
        };
        let _ = self.write_artifacts(module.clone().into()).await;
        Ok(module)
    }

    pub async fn write_artifacts(&self, artifacts: Artifacts) -> Result<(), CacheError> {
        match artifacts {
            Artifacts::Capability(capability) => {
                let m: CapabilityManifest =
                    toml::from_str(&capability.manifest).map_err(|e| CacheError {
                        context: "Failed to deserialize Capability.toml".to_string(),
                        error: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
                    })?;
                let path = self.capabilities_dir(
                    &m.capability.author,
                    &m.capability.name,
                    &m.capability.version,
                );
                capability
                    .write_to_directory(&path)
                    .await
                    .map_err(|e| CacheError {
                        context: format!("Failed to write artifacts to {}", path.display()),
                        error: e,
                    })
            }
            Artifacts::Interface(mut interface) => {
                let path = self.interface_dir(
                    &interface.manifest.capability.author,
                    &interface.manifest.capability.name,
                    &interface.manifest.capability.version,
                );
                fs::create_dir_all(&path).await.map_err(|e| CacheError {
                        context: format!("Failed to create  {}", path.display()),
                        error: e,
                    })?;
                interface.manifest.pyroduct = self.pyroduct_dep.clone();
                let cargo_path = path.join("Cargo.toml");
                let cargo = interface.manifest.clone().to_interface_manifest();
                let cargo = toml::to_string_pretty(&cargo).map_err(|e| CacheError {
                    context: format!("Failed to serialize Cargo.toml to {}", cargo_path.display()),
                    error: io::Error::new(io::ErrorKind::InvalidData, e),
                })?;
                fs::write(&cargo_path, cargo)
                    .await
                    .map_err(|e| CacheError {
                        context: format!("Failed to write Cargo.toml to {}", cargo_path.display()),
                        error: e,
                    })?;
                interface
                    .write_to_directory(&path)
                    .await
                    .map_err(|e| CacheError {
                        context: format!("Failed to write artifacts to {}", path.display()),
                        error: e,
                    })
            }
            Artifacts::AnonModule(anon) => {
                let mut hasher = Sha256::new();
                hasher.update(&anon.source);
                // TODO The hash should also depend on the dependencies
                // hasher.update(&anon.dependencies);
                let hash = format!("{:x}", hasher.finalize());
                let path = self.root.join("anon").join(hash);
                anon.write_to_directory(&path)
                    .await
                    .map_err(|e| CacheError {
                        context: format!("Failed to write artifacts to {}", path.display()),
                        error: e,
                    })
            }
        }
    }

    pub async fn target_dir(&self) -> Result<PathBuf, CacheError> {
        Ok(match self.config().await?.target {
            Some(target) => PathBuf::from(target),
            None => self.root.join("target"),
        })
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
