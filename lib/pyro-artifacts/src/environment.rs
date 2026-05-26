use crate::artifacts::{
    Artifact, Artifacts, CapBinary, CapabilityBinary, CapabilitySource, Interface,
};
use crate::cache::{CacheError, CacheManager};
use crate::cargo::{CapabilityIdent, ProjectManifest};
use crate::debug::{self, CapabilityDebug, ModuleDebug};
use crate::{
    build::BuildError,
    command::{CommandError, format_syn_error, run_command},
};
use pyro_macro::{ffi::generate_capability, module::generate_module};
use pyro_spec::InterfaceSpec;
use std::path::{Path, PathBuf};
use std::sync::Arc;
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
    pub manifest: crate::cargo::ProjectManifest,
    pub cache_manager: Arc<CacheManager>,
}

impl Environment {
    /// Create a new Environment by fetching metadata and detecting manifests from the given root
    pub async fn new(root: PathBuf, cache_manager: Arc<CacheManager>) -> EnvResult<Self> {
        let manifest = Self::load_manifest(&root).await?;
        Self::ensure_cargo_toml(&root, &manifest, &cache_manager).await?;
        let target_dir = Self::get_target_dir(&root).await?;
        Ok(Self {
            root,
            target_dir,
            manifest,
            cache_manager,
        })
    }

    /// Write Cargo.toml from Module.toml or Capability.toml if it doesn't exist
    async fn ensure_cargo_toml(
        root: &Path,
        manifest: &crate::cargo::ProjectManifest,
        cache_manager: &CacheManager,
    ) -> EnvResult<()> {
        let cargo_toml_path = root.join("Cargo.toml");
        if cargo_toml_path.exists() {
            return Ok(());
        }
        let contents =
            toml::to_string_pretty(&manifest.clone().to_cargo_manifest(Some(cache_manager)))
                .map_err(|e| EnvironmentError::ParseManifest(e.to_string()))?;
        fs::write(&cargo_toml_path, contents).await?;
        Ok(())
    }

    pub fn name(&self) -> String {
        self.manifest.ident().name.clone()
    }

    pub fn version(&self) -> String {
        self.manifest.ident().version.clone()
    }

    pub fn author(&self) -> String {
        self.manifest.ident().author.clone()
    }

    /// Detect Capability.toml or Module.toml to extract name and version
    async fn load_manifest(root: &Path) -> EnvResult<ProjectManifest> {
        tracing::debug!("Loading manifest from {:?}", root);
        let capability_toml = root.join("Capability.toml");
        if capability_toml.exists() {
            let content = tokio::fs::read_to_string(&capability_toml).await?;
            let manifest: crate::cargo::CapabilityManifest = toml::from_str(&content)
                .map_err(|e| EnvironmentError::ParseManifest(format!("Capability.toml: {}", e)))?;
            return Ok(crate::cargo::ProjectManifest::Capability(manifest));
        }

        let module_toml = root.join("Module.toml");
        if module_toml.exists() {
            let content = tokio::fs::read_to_string(&module_toml).await?;
            let manifest: crate::cargo::ModuleManifest = toml::from_str(&content)
                .map_err(|e| EnvironmentError::ParseManifest(format!("Module.toml: {}", e)))?;
            return Ok(crate::cargo::ProjectManifest::Module(manifest));
        }

        // Default for anon compilations or when no package section is found
        Err(EnvironmentError::ParseManifest(
            "No manifest found".to_string(),
        ))
    }

    /// Run `cargo metadata` to find the target directory
    pub async fn get_target_dir(path: &Path) -> EnvResult<PathBuf> {
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

    /// Load artifacts from an existing target directory without compiling
    pub async fn load_artifacts_from_target(&self, target_dir: &Path) -> EnvResult<Vec<Artifacts>> {
        tracing::info!("Loading artifacts from target directory: {:?}", target_dir);

        let name = self.name();
        let version = self.version();
        let author = self.author();

        let src_path = self.root.join("src").join("lib.rs");
        let src_lib_rs = if src_path.exists() {
            fs::read_to_string(&src_path).await?
        } else {
            String::new()
        };

        match &self.manifest {
            crate::cargo::ProjectManifest::Capability(cap_manifest) => {
                let lib = self.get_library_artifact(&name).await.ok();

                let lock_path = self.root.join("Cargo.lock");
                let cargo_lock = if lock_path.exists() {
                    fs::read_to_string(&lock_path).await?
                } else {
                    String::new()
                };

                let (interface_rs, interface) =
                    pyro_macro::ffi::generate_interface(&src_lib_rs, &name, &version).map_err(
                        |r| EnvironmentError::InterfaceGeneration(format_syn_error(&src_lib_rs, r)),
                    )?;

                let interface_rs = prettyplease::unparse(&interface_rs);

                let mut artifacts = vec![
                    Artifacts::CapabilitySource(CapabilitySource {
                        manifest: cap_manifest.clone(),
                        cargo_toml: toml::to_string_pretty(
                            &cap_manifest.clone().to_capability_manifest(),
                        )
                        .map_err(|e| EnvironmentError::ParseManifest(e.to_string()))?,
                        cargo_lock,
                        src_lib_rs,
                    }),
                    Artifacts::Interface(Interface {
                        manifest: cap_manifest.clone(),
                        src_lib_rs: interface_rs,
                        interface: interface.clone(),
                    }),
                ];

                if let Some(lib) = lib {
                    artifacts.push(Artifacts::CapabilityBinary(CapabilityBinary {
                        ident: CapabilityIdent {
                            name,
                            version,
                            author,
                        },
                        libs: vec![lib],
                        interface: interface.clone(),
                    }));
                }

                Ok(artifacts)
            }
            crate::cargo::ProjectManifest::Module(module_manifest) => {
                let wasm_path = self.get_wasm_artifact(&name).ok();

                let ident = crate::artifacts::ModuleIdent {
                    author: self.author(),
                    name: self.name(),
                    version: self.version(),
                };

                let source = crate::artifacts::ModuleSource {
                    ident: Some(ident.clone()),
                    dependencies: crate::artifacts::ModuleDependencies {
                        dependencies: module_manifest.dependencies.clone(),
                        capabilities: module_manifest.capabilities.values().cloned().collect(),
                    },
                    source: src_lib_rs.clone(),
                };
                let hash = source.hash();

                let mut artifacts =
                    vec![Artifacts::Module(crate::artifacts::Module::Source(source))];

                if let Some(path) = wasm_path {
                    let spec = pyro_macro::module::generate_module_spec(&src_lib_rs)
                        .map_err(|e| EnvironmentError::InterfaceGeneration(e.to_string()))?
                        .map(|func| crate::artifacts::ModuleSpec {
                            hash,
                            func,
                            capabilities: module_manifest.capabilities.values().cloned().collect(),
                            ident: Some(ident.clone()),
                        })
                        .ok_or_else(|| {
                            EnvironmentError::InterfaceGeneration(
                                "Module main function missing".to_string(),
                            )
                        })?;

                    let binary = crate::artifacts::ModuleBinary {
                        ident: Some(ident),
                        wasm: fs::read(path).await?,
                        spec,
                    };
                    artifacts.push(Artifacts::Module(crate::artifacts::Module::Binary(binary)));
                }

                Ok(artifacts)
            }
        }
    }

    pub async fn package(&self, capture: bool) -> EnvResult<Vec<Artifacts>> {
        let name = self.name();
        let version = self.version();
        let author = self.author();

        match &self.manifest {
            crate::cargo::ProjectManifest::Capability(cap_manifest) => {
                tracing::info!("Packaging capability: {:?}", self.root);

                let cargo_toml =
                    toml::to_string_pretty(&cap_manifest.clone().to_capability_manifest())
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
                    pyro_macro::ffi::generate_interface(&src_lib_rs, &name, &version).map_err(
                        |r| EnvironmentError::InterfaceGeneration(format_syn_error(&src_lib_rs, r)),
                    )?;

                let interface_rs = prettyplease::unparse(&interface_rs);

                Ok(vec![
                    Artifacts::CapabilitySource(CapabilitySource {
                        manifest: cap_manifest.clone(),
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
                        manifest: cap_manifest.clone(),
                        src_lib_rs: interface_rs,
                        interface: interface.clone(),
                    }),
                ])
            }
            crate::cargo::ProjectManifest::Module(module_manifest) => {
                tracing::info!("Packaging module: {:?}", self.root);

                tracing::info!("Compiling module binary...");
                self.compile(&["-p", &name], capture).await?;

                let wasm_artifact = self.get_wasm_artifact(&name)?;

                let src_path = self.root.join("src").join("lib.rs");
                let src_lib_rs = if src_path.exists() {
                    fs::read_to_string(&src_path).await?
                } else {
                    String::new()
                };

                let ident = crate::artifacts::ModuleIdent {
                    author: self.author(),
                    name: self.name(),
                    version: self.version(),
                };

                let source = crate::artifacts::ModuleSource {
                    ident: Some(ident.clone()),
                    dependencies: crate::artifacts::ModuleDependencies {
                        dependencies: module_manifest.dependencies.clone(),
                        capabilities: module_manifest.capabilities.values().cloned().collect(),
                    },
                    source: src_lib_rs.clone(),
                };
                let hash = source.hash();

                let spec = pyro_macro::module::generate_module_spec(&src_lib_rs)
                    .map_err(|e| EnvironmentError::InterfaceGeneration(e.to_string()))?
                    .map(|func| crate::artifacts::ModuleSpec {
                        hash,
                        func,
                        capabilities: module_manifest.capabilities.values().cloned().collect(),
                        ident: Some(ident.clone()),
                    })
                    .ok_or_else(|| {
                        EnvironmentError::InterfaceGeneration(
                            "Module main function missing".to_string(),
                        )
                    })?;

                let binary = crate::artifacts::ModuleBinary {
                    ident: Some(ident),
                    wasm: fs::read(wasm_artifact).await?,
                    spec,
                };

                Ok(vec![
                    Artifacts::Module(crate::artifacts::Module::Source(source)),
                    Artifacts::Module(crate::artifacts::Module::Binary(binary)),
                ])
            }
        }
    }

    pub async fn expand_debug(&self) -> EnvResult<()> {
        let debug_dir = self.root.join("debug");
        fs::create_dir_all(&debug_dir).await?;

        match &self.manifest {
            crate::cargo::ProjectManifest::Capability(cap_manifest) => {
                tracing::info!("Generating debug info for capability: {}", self.name());
                let name = self.name();
                let version = self.version();

                let lib = self.get_library_artifact(&name).await?;

                let src_path = self.root.join("src").join("lib.rs");
                let src_lib_rs = fs::read_to_string(&src_path)
                    .await
                    .map_err(|_| EnvironmentError::SourceNotFound(src_path))?;

                let (_, interface) =
                    pyro_macro::ffi::generate_interface(&src_lib_rs, &name, &version).map_err(
                        |r| EnvironmentError::InterfaceGeneration(format_syn_error(&src_lib_rs, r)),
                    )?;

                let binary = CapabilityBinary {
                    ident: cap_manifest.capability.clone(),
                    libs: vec![lib],
                    interface,
                };

                let symbols = debug::symbols(&binary);

                let code = generate_capability(&src_lib_rs, &name, &version)
                    .map_err(|e| EnvironmentError::InterfaceGeneration(e.to_string()))?;

                let cap_rs = Some(prettyplease::unparse(&code));
                let debug_info = CapabilityDebug { symbols, cap_rs };
                debug_info.write_to_directory(&debug_dir).await?;
            }
            crate::cargo::ProjectManifest::Module(module_manifest) => {
                tracing::info!("Generating debug info for module: {}", self.name());
                let name = self.name();

                let wasm_path = self.get_wasm_artifact(&name)?;
                let wasm_bytes = fs::read(wasm_path).await?;

                let src_path = self.root.join("src").join("lib.rs");
                let src_lib_rs = fs::read_to_string(&src_path)
                    .await
                    .map_err(|_| EnvironmentError::SourceNotFound(src_path))?;

                let spec = pyro_macro::module::generate_module_spec(&src_lib_rs)
                    .map_err(|e| EnvironmentError::InterfaceGeneration(e.to_string()))?
                    .ok_or_else(|| {
                        EnvironmentError::InterfaceGeneration(
                            "Module main function missing".to_string(),
                        )
                    })?;

                let source = crate::artifacts::ModuleSource {
                    ident: None,
                    dependencies: crate::artifacts::ModuleDependencies {
                        dependencies: module_manifest.dependencies.clone(),
                        capabilities: module_manifest.capabilities.values().cloned().collect(),
                    },
                    source: src_lib_rs.clone(),
                };
                let hash = source.hash();

                let binary = crate::artifacts::ModuleBinary {
                    ident: None,
                    wasm: wasm_bytes,
                    spec: crate::artifacts::ModuleSpec {
                        hash,
                        func: spec,
                        capabilities: vec![], // Capabilities not needed for WAT/RS generation
                        ident: None,
                    },
                };

                let wat = debug::wat(&binary).map_err(EnvironmentError::InterfaceGeneration)?;

                let generated_code = generate_module(&src_lib_rs).map_err(|e| {
                    EnvironmentError::InterfaceGeneration(format!(
                        "Module code generation error: {}",
                        e
                    ))
                })?;
                let cap_rs = Some(prettyplease::unparse(&generated_code));

                let debug_info = ModuleDebug {
                    wat: Some(wat),
                    cap_rs,
                };
                debug_info.write_to_directory(&debug_dir).await?;
            }
        }

        Ok(())
    }

    pub async fn capability_spec(&self) -> EnvResult<InterfaceSpec<'static>> {
        let name = self.name();
        let version = self.version();
        let src_path = self.root.join("src").join("lib.rs");
        let src_lib_rs = fs::read_to_string(&src_path)
            .await
            .map_err(|_| EnvironmentError::SourceNotFound(src_path))?;

        match &self.manifest {
            crate::cargo::ProjectManifest::Capability(_) => {
                let (_, interface) =
                    pyro_macro::ffi::generate_interface(&src_lib_rs, &name, &version).map_err(
                        |r| EnvironmentError::InterfaceGeneration(format_syn_error(&src_lib_rs, r)),
                    )?;
                Ok(interface)
            }
            crate::cargo::ProjectManifest::Module(_) => Err(EnvironmentError::InterfaceGeneration(
                "Capability spec is only supported for capabilities".to_string(),
            )),
        }
    }

    pub async fn module_spec(&self) -> EnvResult<crate::artifacts::ModuleSpec> {
        let src_path = self.root.join("src").join("lib.rs");
        let src_lib_rs = fs::read_to_string(&src_path)
            .await
            .map_err(|_| EnvironmentError::SourceNotFound(src_path))?;

        match &self.manifest {
            crate::cargo::ProjectManifest::Module(module_manifest) => {
                let source = crate::artifacts::ModuleSource {
                    ident: None,
                    dependencies: crate::artifacts::ModuleDependencies {
                        dependencies: module_manifest.dependencies.clone(),
                        capabilities: module_manifest.capabilities.values().cloned().collect(),
                    },
                    source: src_lib_rs.clone(),
                };
                let hash = source.hash();

                pyro_macro::module::generate_module_spec(&src_lib_rs)
                    .map_err(|e| EnvironmentError::InterfaceGeneration(e.to_string()))?
                    .map(|func| crate::artifacts::ModuleSpec {
                        hash,
                        func,
                        capabilities: module_manifest.capabilities.values().cloned().collect(),
                        ident: Some(crate::artifacts::ModuleIdent {
                            author: self.author(),
                            name: self.name(),
                            version: self.version(),
                        }),
                    })
                    .ok_or_else(|| {
                        EnvironmentError::InterfaceGeneration(
                            "Module main function missing".to_string(),
                        )
                    })
            }
            crate::cargo::ProjectManifest::Capability(_) => {
                Err(EnvironmentError::InterfaceGeneration(
                    "Module spec is only supported for modules".to_string(),
                ))
            }
        }
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
