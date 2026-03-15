use crate::artifacts::{
    Artifact, Artifacts, CapabilityBinary, CapabilitySource, Module, ModuleBinary, ModuleSource,
    ModuleSpec,
};
use crate::build::{CommandError, format_syn_error, run_command};
use crate::debug::{self, CapabilityDebug, ModuleDebug};
use cargo_toml::Dependency;
use pyro_core::{
    ffi::generate_capability,
    module::{generate_module, generate_module_spec},
};
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

#[derive(serde::Deserialize)]
pub struct PyroductConfig {
    pub author: Option<String>,
    pub target: Option<PathBuf>,
    pub pyroduct: Option<Dependency>,
}

pub struct CacheManager {
    pub root: PathBuf,
    pub target_dir: PathBuf,
    pub pyroduct_dep: Dependency,
    pub config: PyroductConfig,
}

impl CacheManager {
    pub async fn new(root: &Path, mut config: PyroductConfig) -> Result<Self, CacheError> {
        fs::create_dir_all(&root).await.map_err(|e| CacheError {
            context: "Failed to create cache root".to_string(),
            error: e,
        })?;
        let pyroduct_dep = if let Some(dep) = &mut config.pyroduct {
            resolve_dependency_path(dep, &root);
            dep.clone()
        } else {
            Dependency::Simple("*".to_string())
        };
        let target_dir = if let Some(target) = &mut config.target {
            if target.is_relative() {
                *target = root.join(&target);
            }
            target.clone()
        } else {
            root.join("target")
        };
        let manager = Self {
            root: root.to_path_buf(),
            target_dir,
            pyroduct_dep,
            config,
        };

        manager.init().await?;
        Ok(manager)
    }

    /// The previous new() logic, now moved to from_env()
    pub async fn from_env() -> Result<Self, CacheError> {
        let root = std::env::var("PYRODUCT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME")
                    .or_else(|_| std::env::var("USERPROFILE"))
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| PathBuf::from("."));
                home.join(".pyroduct")
            });

        let config_path = root.join("config.toml");
        let content = fs::read_to_string(&config_path)
            .await
            .map_err(|error| CacheError {
                context: format!("Failed to read the configuration"),
                error,
            })?;
        let config = toml::from_str::<PyroductConfig>(&content).map_err(|error| CacheError {
            context: format!("Failed to parse the configuration"),
            error: io::Error::new(io::ErrorKind::InvalidData, error),
        })?;

        Self::new(&root, config).await
    }

    /// The folder instantiator
    pub async fn init(&self) -> Result<(), CacheError> {
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

        fs::write(
            cargo_dir.join("config.toml"),
            format!("[build]\ntarget-dir = \"{}\"", self.target_dir.display()),
        )
        .await
        .map_err(|error| CacheError {
            context: "Failed to write target config.toml".to_string(),
            error,
        })?;

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

    pub async fn capability_binary_path(
        &self,
        author: &str,
        name: &str,
        version: &str,
    ) -> Result<PathBuf, CacheError> {
        let base_dir = self.capabilities_dir(author, name, version);

        #[cfg(target_os = "linux")]
        let lib_file = "lib.so";
        #[cfg(target_os = "macos")]
        let lib_file = "lib.dylib";
        #[cfg(target_os = "windows")]
        let lib_file = "lib.dll";
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        let lib_file = "lib.so";

        let path = base_dir.join(lib_file);
        if !path.exists() {
            Err(CacheError {
                context: format!("Missing {} binary for this system", path.display()),
                error: io::Error::new(io::ErrorKind::NotFound, "Not Found"),
            })
        } else {
            Ok(path)
        }
    }

    pub async fn debug_capabilities(
        &self,
        author: &str,
        name: &str,
        version: &str,
    ) -> Result<CapabilityDebug, BuildError> {
        let path = self.capabilities_dir(author, name, version);
        let binary = CapabilityBinary::from_dir(&path).await.map_err(|error| {
            BuildError::io("failed to load capability binary from cache", error)
        })?;

        let symbols = debug::symbols(&binary);

        let source = CapabilitySource::from_dir(&path).await.map_err(|error| {
            BuildError::io("failed to load capability source from cache", error)
        })?;

        let code = generate_capability(
            &source.src_lib_rs,
            &source.manifest.capability.name,
            &source.manifest.capability.version,
        )
        .map_err(|e| {
            BuildError::Documentation(format!("Capability code generation error: {}", e))
        })?;
        let cap_rs = Some(prettyplease::unparse(&code));

        let debug = CapabilityDebug { symbols, cap_rs };
        debug.write_to_directory(&path).await?;

        Ok(debug)
    }

    pub async fn debug_module(&self, hash: &str) -> Result<ModuleDebug, BuildError> {
        let path = self.root.join("anon").join(hash);
        let source = ModuleSource::from_dir(&path)
            .await
            .map_err(|error| BuildError::io("failed to load module source from cache", error))?;
        let binary = ModuleBinary::from_dir(&path)
            .await
            .map_err(|error| BuildError::io("failed to load module binary from cache", error))?;

        let wat = match debug::wat(&binary) {
            Ok(wat) => Some(wat),
            Err(error) => {
                tracing::error!(error, "Unable to create wat");
                None
            }
        };

        let generated_code = generate_module(&source.source).map_err(|e| {
            BuildError::Documentation(format!("Module code generation error: {}", e))
        })?;
        let cap_rs = Some(prettyplease::unparse(&generated_code));

        let debug = ModuleDebug { wat, cap_rs };
        debug.write_to_directory(&path).await?;

        Ok(debug)
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

    pub async fn get_binary(&self, hash: &str) -> Result<ModuleBinary, CacheError> {
        let path = self.root.join("anon").join(hash);
        if path.exists() {
            let binary = ModuleBinary::from_dir(&path)
                .await
                .map_err(|error| CacheError {
                    context: "Unable to load binary".to_string(),
                    error,
                })?;
            Ok(binary)
        } else {
            Err(CacheError {
                context: format!("Missing {} binary", path.display()),
                error: io::Error::new(io::ErrorKind::NotFound, "Not Found"),
            })
        }
    }

    pub async fn get_source(&self, hash: &str) -> Result<ModuleSource, CacheError> {
        let path = self.root.join("anon").join(hash);
        if path.exists() {
            let source = ModuleSource::from_dir(&path)
                .await
                .map_err(|error| CacheError {
                    context: "Unable to load source".to_string(),
                    error,
                })?;
            Ok(source)
        } else {
            Err(CacheError {
                context: format!("Missing {} source", path.display()),
                error: io::Error::new(io::ErrorKind::NotFound, "Not Found"),
            })
        }
    }

    /// Compile the module written by `set_build` and store the wasm as an anon
    /// artifact. Returns the hex SHA-256 hash that identifies it in the cache.
    pub async fn compile(&self, source: &ModuleSource) -> Result<ModuleBinary, BuildError> {
        let hash = source.hash();
        if let Ok(binary) = self.get_binary(&hash).await {
            return Ok(binary);
        }

        let build_dir = self.root.join("build");
        let src_dir = build_dir.join("src");
        fs::create_dir_all(&src_dir)
            .await
            .map_err(|e| BuildError::io("create src dir", e))?;
        fs::write(src_dir.join("lib.rs"), &source.source)
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
        for (dep_name, dep) in source.dependencies.dependencies.iter() {
            manifest.dependencies.insert(dep_name.clone(), dep.clone());
        }
        for cap in source.dependencies.capabilities.iter() {
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

        let func = generate_module_spec(&source.source)
            .map_err(|s| {
                BuildError::Documentation(format_syn_error("Cannot generate docstring", s))
            })?
            .ok_or(BuildError::Documentation(
                "Module main functions is missing".to_string(),
            ))?;
        let spec = ModuleSpec {
            func,
            capabilities: source.dependencies.capabilities.clone(),
        };

        let binary = ModuleBinary { hash, wasm, spec };

        let _ = self.write_artifacts(&source.clone().into()).await;
        let _ = self.write_artifacts(&binary.clone().into()).await;

        Ok(binary)
    }

    pub async fn write_artifacts(&self, artifacts: &Artifacts) -> Result<(), CacheError> {
        match &artifacts {
            Artifacts::CapabilityBinary(capability) => {
                let path = self.capabilities_dir(
                    &capability.manifest.capability.author,
                    &capability.manifest.capability.name,
                    &capability.manifest.capability.version,
                );
                capability
                    .write_to_directory(&path)
                    .await
                    .map_err(|e| CacheError {
                        context: format!("Failed to write artifacts to {}", path.display()),
                        error: e,
                    })
            }
            Artifacts::CapabilitySource(capability) => {
                let path = self.capabilities_dir(
                    &capability.manifest.capability.author,
                    &capability.manifest.capability.name,
                    &capability.manifest.capability.version,
                );
                capability
                    .write_to_directory(&path)
                    .await
                    .map_err(|e| CacheError {
                        context: format!("Failed to write artifacts to {}", path.display()),
                        error: e,
                    })
            }
            Artifacts::Interface(interface) => {
                let path = self.interface_dir(
                    &interface.manifest.capability.author,
                    &interface.manifest.capability.name,
                    &interface.manifest.capability.version,
                );
                fs::create_dir_all(&path).await.map_err(|e| CacheError {
                    context: format!("Failed to create  {}", path.display()),
                    error: e,
                })?;
                let mut manifest = interface.manifest.clone();
                manifest.pyroduct = self.pyroduct_dep.clone();
                let cargo_path = path.join("Cargo.toml");
                let cargo = manifest.clone().to_interface_manifest();
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
            Artifacts::Module(Module::Binary(binary)) => {
                let path = self.root.join("anon").join(&binary.hash);
                binary
                    .write_to_directory(&path)
                    .await
                    .map_err(|e| CacheError {
                        context: format!("Failed to write artifacts to {}", path.display()),
                        error: e,
                    })
            }
            Artifacts::Module(Module::Source(source)) => {
                let hash = source.hash();
                let path = self.root.join("anon").join(hash);
                source
                    .write_to_directory(&path)
                    .await
                    .map_err(|e| CacheError {
                        context: format!("Failed to write artifacts to {}", path.display()),
                        error: e,
                    })
            }
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
