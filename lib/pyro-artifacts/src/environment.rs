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
    #[tracing::instrument(skip(root, cache_manager), fields(root = %root.display()))]
    pub async fn new(root: PathBuf, cache_manager: Arc<CacheManager>) -> EnvResult<Self> {
        tracing::debug!("Creating Environment instance");
        let res = async {
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
        .await;

        if let Err(ref e) = res {
            tracing::error!(error = ?e, "Failed to create Environment");
        } else {
            tracing::debug!("Environment successfully created");
        }
        res
    }

    /// Write Cargo.toml from Module.toml or Capability.toml if it doesn't exist
    #[tracing::instrument(skip(root, manifest, cache_manager), fields(root = %root.display()))]
    async fn ensure_cargo_toml(
        root: &Path,
        manifest: &crate::cargo::ProjectManifest,
        cache_manager: &CacheManager,
    ) -> EnvResult<()> {
        let cargo_toml_path = root.join("Cargo.toml");
        if cargo_toml_path.exists() {
            tracing::debug!("Cargo.toml already exists, skipping generation");
            return Ok(());
        }
        tracing::debug!("Cargo.toml not found, generating from Pyroduct manifest");
        let contents =
            toml::to_string_pretty(&manifest.clone().to_cargo_manifest(Some(cache_manager)))
                .map_err(|e| {
                    let err = EnvironmentError::ParseManifest(e.to_string());
                    tracing::error!(error = ?err, "Failed to serialize generated Cargo.toml");
                    err
                })?;
        fs::write(&cargo_toml_path, contents).await.map_err(|e| {
            tracing::error!(error = ?e, "Failed to write generated Cargo.toml at {:?}", cargo_toml_path);
            e
        })?;
        tracing::debug!("Successfully wrote Cargo.toml");
        Ok(())
    }

    pub fn package(&self) -> String {
        self.manifest.ident().package.clone()
    }

    pub fn version(&self) -> String {
        self.manifest.ident().version.clone()
    }

    pub fn author(&self) -> String {
        self.manifest.ident().author.clone()
    }

    /// Detect Capability.toml or Module.toml to extract name and version
    #[tracing::instrument(skip(root), fields(root = %root.display()))]
    async fn load_manifest(root: &Path) -> EnvResult<ProjectManifest> {
        tracing::debug!("Loading manifest from {:?}", root);
        let capability_toml = root.join("Capability.toml");
        if capability_toml.exists() {
            tracing::debug!("Detected Capability.toml");
            let content = tokio::fs::read_to_string(&capability_toml).await?;
            let manifest: crate::cargo::CapabilityManifest =
                toml::from_str(&content).map_err(|e| {
                    let err = EnvironmentError::ParseManifest(format!("Capability.toml: {}", e));
                    tracing::error!(error = ?err, "Failed to parse Capability.toml");
                    err
                })?;
            return Ok(crate::cargo::ProjectManifest::Capability(manifest));
        }

        let module_toml = root.join("Module.toml");
        if module_toml.exists() {
            tracing::debug!("Detected Module.toml");
            let content = tokio::fs::read_to_string(&module_toml).await?;
            let manifest: crate::cargo::ModuleManifest = toml::from_str(&content).map_err(|e| {
                let err = EnvironmentError::ParseManifest(format!("Module.toml: {}", e));
                tracing::error!(error = ?err, "Failed to parse Module.toml");
                err
            })?;
            return Ok(crate::cargo::ProjectManifest::Module(manifest));
        }

        // Default for anon compilations or when no package section is found
        let err = EnvironmentError::ParseManifest("No manifest found".to_string());
        tracing::error!(error = ?err, "No Capability.toml or Module.toml manifest found in {:?}", root);
        Err(err)
    }

    /// Run `cargo metadata` to find the target directory
    #[tracing::instrument(skip(path), fields(path = %path.display()))]
    pub async fn get_target_dir(path: &Path) -> EnvResult<PathBuf> {
        tracing::debug!("Retrieving cargo metadata target directory");
        let output = Command::new("cargo")
            .args(["metadata", "--format-version=1", "--no-deps"])
            .current_dir(path)
            .output()
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "Failed to launch cargo metadata");
                e
            })?;

        if !output.status.success() {
            let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();
            tracing::error!(stderr = %stderr_str, "Cargo metadata execution failed");
            return Err(EnvironmentError::Metadata(stderr_str));
        }

        let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)?;

        metadata["target_directory"]
            .as_str()
            .map(PathBuf::from)
            .ok_or_else(|| {
                tracing::error!("Missing target_directory in cargo metadata JSON");
                EnvironmentError::MissingTargetDir
            })
    }

    #[tracing::instrument(skip(self))]
    pub async fn generate_lockfile(&self) -> EnvResult<String> {
        tracing::debug!("Generating Cargo.lock file");
        run_command(&self.root, &["generate-lockfile"], true)
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "Failed to generate Cargo.lock");
                EnvironmentError::CommandError(e)
            })?;

        let lockfile_path = self.root.join("Cargo.lock");
        fs::read_to_string(&lockfile_path).await.map_err(|e| {
            tracing::error!(error = ?e, "Failed to read generated Cargo.lock");
            EnvironmentError::Io(e)
        })
    }

    /// Compile the project (defaults to release)
    #[tracing::instrument(skip(self))]
    pub async fn compile(&self, extra_args: &[&str], capture: bool) -> EnvResult<()> {
        tracing::debug!("Starting Cargo compilation in environment");
        let mut args = vec!["build", "--release"];
        args.extend_from_slice(extra_args);
        if matches!(self.manifest, ProjectManifest::Module(_)) {
            args.push("--target");
            args.push("wasm32-unknown-unknown");
        }
        run_command(&self.root, &args, capture).await.map_err(|e| {
            tracing::error!(error = ?e, "Cargo compilation failed");
            EnvironmentError::CommandError(e)
        })?;
        tracing::debug!("Cargo compilation completed successfully");
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
    #[tracing::instrument(skip(self, target_dir), fields(target_dir = %target_dir.display()))]
    pub async fn load_artifacts_from_target(&self, target_dir: &Path) -> EnvResult<Vec<Artifacts>> {
        tracing::debug!("Loading artifacts from target directory");

        let package = self.package();
        let version = self.version();
        let author = self.author();

        let src_path = self.root.join("src").join("lib.rs");
        let src_lib_rs = if src_path.exists() {
            fs::read_to_string(&src_path).await?
        } else {
            String::new()
        };

        let res = async {
            match &self.manifest {
                crate::cargo::ProjectManifest::Capability(cap_manifest) => {
                    let lib = self.get_library_artifact(&package).await.ok();

                    let lock_path = self.root.join("Cargo.lock");
                    let cargo_lock = if lock_path.exists() {
                        fs::read_to_string(&lock_path).await?
                    } else {
                        String::new()
                    };

                    let (interface_rs, interface) =
                        pyro_macro::ffi::generate_interface(&src_lib_rs, &package, &version)
                            .map_err(|r| {
                                EnvironmentError::InterfaceGeneration(format_syn_error(
                                    &src_lib_rs,
                                    r,
                                ))
                            })?;

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
                                package,
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
                    let wasm_path = self.get_wasm_artifact(&package).ok();

                    let source = crate::artifacts::PlaybookSource {
                        manifest: module_manifest.clone(),
                        source: src_lib_rs.clone(),
                    };
                    let hash = source.hash();

                    let mut artifacts = vec![Artifacts::Playbook(
                        crate::artifacts::Playbook::Source(source.clone()),
                    )];

                    if let Some(path) = wasm_path {
                        let mut dep_interfaces = Vec::new();
                        for cap in source.dependencies().capabilities.iter() {
                            if let Ok(spec_str) = self
                                .cache_manager
                                .capability_interface_spec(&cap.author, &cap.package, &cap.version)
                                .await
                            {
                                if let Ok(spec) =
                                    serde_json::from_str::<pyro_spec::InterfaceSpec>(&spec_str)
                                {
                                    dep_interfaces.push(spec);
                                }
                            }
                        }

                        let spec =
                            pyro_macro::module::generate_module_spec(&src_lib_rs, &dep_interfaces)
                                .map_err(|e| EnvironmentError::InterfaceGeneration(e.to_string()))?
                                .map(|func| crate::artifacts::PlaybookSpec {
                                    ident: source.ident(),
                                    hash,
                                    func,
                                    capabilities: source.dependencies().capabilities,
                                    interconnect: source.manifest.interconnect.clone(),
                                })
                                .ok_or_else(|| {
                                    EnvironmentError::InterfaceGeneration(
                                        "Module main function missing".to_string(),
                                    )
                                })?;

                        let binary = crate::artifacts::PlaybookBinary {
                            wasm: fs::read(path).await?,
                            spec,
                            configurations: source.configurations(),
                        };
                        artifacts.push(Artifacts::Playbook(crate::artifacts::Playbook::Binary(
                            binary,
                        )));
                    }

                    Ok(artifacts)
                }
            }
        }
        .await;

        if let Err(ref e) = res {
            tracing::error!(error = ?e, "Failed to load artifacts from target directory");
        } else {
            tracing::debug!("Successfully loaded artifacts from target directory");
        }
        res
    }

    #[tracing::instrument(skip(self))]
    pub async fn pack(&self, capture: bool) -> EnvResult<Vec<Artifacts>> {
        tracing::debug!("Packaging project artifacts");
        let package = self.package();
        let version = self.version();
        let author = self.author();

        let res = async {
            match &self.manifest {
                crate::cargo::ProjectManifest::Capability(cap_manifest) => {
                    tracing::debug!(dir = ?self.root, "Packaging capability");

                    let cargo_toml =
                        toml::to_string_pretty(&cap_manifest.clone().to_capability_manifest())
                            .map_err(|e| EnvironmentError::ParseManifest(e.to_string()))?;

                    tracing::info!(dir = ?self.root, "Compiling capability binary...");
                    self.compile(&["--features", "capability", "-p", &package], capture)
                        .await?;

                    let lib = self.get_library_artifact(&package).await?;

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

                    tracing::debug!(dir = ?self.root, "Generating interface for capability...");
                    let (interface_rs, interface) =
                        pyro_macro::ffi::generate_interface(&src_lib_rs, &package, &version)
                            .map_err(|r| {
                                EnvironmentError::InterfaceGeneration(format_syn_error(
                                    &src_lib_rs,
                                    r,
                                ))
                            })?;

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
                                package,
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
                    tracing::debug!("Packaging module: {:?}", self.root);

                    tracing::info!("Compiling module binary...");
                    self.compile(&["-p", &package], capture).await?;

                    let wasm_artifact = self.get_wasm_artifact(&package)?;

                    let src_path = self.root.join("src").join("lib.rs");
                    let src_lib_rs = if src_path.exists() {
                        fs::read_to_string(&src_path).await?
                    } else {
                        String::new()
                    };

                    let source = crate::artifacts::PlaybookSource {
                        manifest: module_manifest.clone(),
                        source: src_lib_rs.clone(),
                    };
                    let hash = source.hash();

                    let mut dep_interfaces = Vec::new();
                    for cap in source.dependencies().capabilities.iter() {
                        if let Ok(spec_str) = self
                            .cache_manager
                            .capability_interface_spec(&cap.author, &cap.package, &cap.version)
                            .await
                        {
                            if let Ok(spec) =
                                serde_json::from_str::<pyro_spec::InterfaceSpec>(&spec_str)
                            {
                                dep_interfaces.push(spec);
                            }
                        }
                    }

                    let spec =
                        pyro_macro::module::generate_module_spec(&src_lib_rs, &dep_interfaces)
                            .map_err(|e| EnvironmentError::InterfaceGeneration(e.to_string()))?
                            .map(|func| crate::artifacts::PlaybookSpec {
                                ident: source.ident(),
                                hash,
                                func,
                                capabilities: source.dependencies().capabilities,
                                interconnect: source.manifest.interconnect.clone(),
                            })
                            .ok_or_else(|| {
                                EnvironmentError::InterfaceGeneration(
                                    "Module main function missing".to_string(),
                                )
                            })?;

                    let binary = crate::artifacts::PlaybookBinary {
                        wasm: fs::read(wasm_artifact).await?,
                        spec,
                        configurations: source.configurations(),
                    };

                    Ok(vec![
                        Artifacts::Playbook(crate::artifacts::Playbook::Source(source)),
                        Artifacts::Playbook(crate::artifacts::Playbook::Binary(binary)),
                    ])
                }
            }
        }
        .await;

        if let Err(ref e) = res {
            tracing::error!(error = ?e, "Failed to package project artifacts");
        } else {
            tracing::debug!("Successfully packaged project artifacts");
        }
        res
    }

    #[tracing::instrument(skip(self))]
    pub async fn expand_debug(&self) -> EnvResult<()> {
        let debug_dir = self.root.join("debug");
        fs::create_dir_all(&debug_dir).await?;

        tracing::debug!("Generating expanded debug files");
        let res = async {
            match &self.manifest {
                crate::cargo::ProjectManifest::Capability(cap_manifest) => {
                    tracing::debug!("Generating debug info for capability: {}", self.package());
                    let package = self.package();
                    let version = self.version();

                    let lib = self.get_library_artifact(&package).await?;

                    let src_path = self.root.join("src").join("lib.rs");
                    let src_lib_rs = fs::read_to_string(&src_path)
                        .await
                        .map_err(|_| EnvironmentError::SourceNotFound(src_path))?;

                    let (_, interface) =
                        pyro_macro::ffi::generate_interface(&src_lib_rs, &package, &version)
                            .map_err(|r| {
                                EnvironmentError::InterfaceGeneration(format_syn_error(
                                    &src_lib_rs,
                                    r,
                                ))
                            })?;

                    let binary = CapabilityBinary {
                        ident: cap_manifest.capability.clone(),
                        libs: vec![lib],
                        interface,
                    };

                    let symbols = debug::symbols(&binary);

                    let code = generate_capability(&src_lib_rs, &package, &version)
                        .map_err(|e| EnvironmentError::InterfaceGeneration(e.to_string()))?;

                    let cap_rs = Some(prettyplease::unparse(&code));
                    let debug_info = CapabilityDebug { symbols, cap_rs };
                    debug_info.write_to_directory(&debug_dir).await?;
                }
                crate::cargo::ProjectManifest::Module(module_manifest) => {
                    tracing::debug!("Generating debug info for module: {}", self.package());
                    let package = self.package();

                    let wasm_path = self.get_wasm_artifact(&package)?;
                    let wasm_bytes = fs::read(wasm_path).await?;

                    let src_path = self.root.join("src").join("lib.rs");
                    let src_lib_rs = fs::read_to_string(&src_path)
                        .await
                        .map_err(|_| EnvironmentError::SourceNotFound(src_path))?;

                    let mut resolved_capabilities = Vec::new();
                    let mut dep_interfaces = Vec::new();
                    for cap in module_manifest.capabilities.values() {
                        let cap_ident = CapabilityIdent {
                            author: cap.author.clone(),
                            package: cap.package.clone(),
                            version: cap.version.clone(),
                        };
                        if let Ok(spec_str) = self
                            .cache_manager
                            .capability_interface_spec(
                                &cap_ident.author,
                                &cap_ident.package,
                                &cap_ident.version,
                            )
                            .await
                        {
                            if let Ok(spec) =
                                serde_json::from_str::<pyro_spec::InterfaceSpec>(&spec_str)
                            {
                                dep_interfaces.push(spec);
                            }
                        }
                        resolved_capabilities.push(cap_ident);
                    }

                    let spec =
                        pyro_macro::module::generate_module_spec(&src_lib_rs, &dep_interfaces)
                            .map_err(|e| EnvironmentError::InterfaceGeneration(e.to_string()))?
                            .ok_or_else(|| {
                                EnvironmentError::InterfaceGeneration(
                                    "Module main function missing".to_string(),
                                )
                            })?;

                    let dummy_ident = crate::artifacts::PlaybookIdent {
                        author: "dummy".to_string(),
                        package: "dummy".to_string(),
                        version: "0.0.0".to_string(),
                    };
                    let source = crate::artifacts::PlaybookSource::new(
                        dummy_ident.clone(),
                        crate::artifacts::ModuleDependencies {
                            dependencies: module_manifest.dependencies.clone(),
                            capabilities: resolved_capabilities.clone(),
                        },
                        std::vec::Vec::new(),
                        src_lib_rs.clone(),
                        module_manifest.interconnect.clone(),
                    );
                    let hash = source.hash();

                    let binary = crate::artifacts::PlaybookBinary {
                        wasm: wasm_bytes,
                        spec: crate::artifacts::PlaybookSpec {
                            ident: dummy_ident,
                            hash,
                            func: spec,
                            capabilities: vec![], // Capabilities not needed for WAT/RS generation
                            interconnect: std::collections::BTreeMap::new(),
                        },
                        configurations: std::vec::Vec::new(),
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
        .await;

        if let Err(ref e) = res {
            tracing::error!(error = ?e, "Failed to generate expanded debug files");
        } else {
            tracing::debug!("Successfully generated expanded debug files");
        }
        res
    }

    #[tracing::instrument(skip(self))]
    pub async fn capability_spec(&self) -> EnvResult<InterfaceSpec<'static>> {
        tracing::debug!("Resolving capability spec");
        let package = self.package();
        let version = self.version();
        let src_path = self.root.join("src").join("lib.rs");
        let src_lib_rs = fs::read_to_string(&src_path).await.map_err(|_| {
            let err = EnvironmentError::SourceNotFound(src_path.clone());
            tracing::error!(error = ?err, "Failed to read src/lib.rs for capability spec");
            err
        })?;

        let res = match &self.manifest {
            crate::cargo::ProjectManifest::Capability(_) => {
                let (_, interface) = pyro_macro::ffi::generate_interface(
                    &src_lib_rs,
                    &package,
                    &version,
                )
                .map_err(|r| {
                    let err =
                        EnvironmentError::InterfaceGeneration(format_syn_error(&src_lib_rs, r));
                    tracing::error!(error = ?err, "Failed to generate interface spec");
                    err
                })?;
                Ok(interface)
            }
            crate::cargo::ProjectManifest::Module(_) => {
                let err = EnvironmentError::InterfaceGeneration(
                    "Capability spec is only supported for capabilities".to_string(),
                );
                tracing::error!(error = ?err, "Invalid manifest type for capability spec");
                Err(err)
            }
        };
        res
    }

    #[tracing::instrument(skip(self))]
    pub async fn playbook_spec(&self) -> EnvResult<crate::artifacts::PlaybookSpec> {
        tracing::debug!("Resolving playbook spec");
        let src_path = self.root.join("src").join("lib.rs");
        let src_lib_rs = fs::read_to_string(&src_path).await.map_err(|_| {
            let err = EnvironmentError::SourceNotFound(src_path.clone());
            tracing::error!(error = ?err, "Failed to read src/lib.rs for playbook spec");
            err
        })?;

        let res = match &self.manifest {
            crate::cargo::ProjectManifest::Module(module_manifest) => {
                let mut resolved_capabilities = Vec::new();
                let mut dep_interfaces = Vec::new();
                for cap in module_manifest.capabilities.values() {
                    let cap_ident = CapabilityIdent {
                        author: cap.author.clone(),
                        package: cap.package.clone(),
                        version: cap.version.clone(),
                    };
                    if let Ok(spec_str) = self
                        .cache_manager
                        .capability_interface_spec(
                            &cap_ident.author,
                            &cap_ident.package,
                            &cap_ident.version,
                        )
                        .await
                    {
                        if let Ok(spec) =
                            serde_json::from_str::<pyro_spec::InterfaceSpec>(&spec_str)
                        {
                            dep_interfaces.push(spec);
                        }
                    }
                    resolved_capabilities.push(cap_ident);
                }

                let ident = crate::artifacts::PlaybookIdent {
                    author: self.author(),
                    package: self.package(),
                    version: self.version(),
                };
                let source = crate::artifacts::PlaybookSource::new(
                    ident.clone(),
                    crate::artifacts::ModuleDependencies {
                        dependencies: module_manifest.dependencies.clone(),
                        capabilities: resolved_capabilities.clone(),
                    },
                    std::vec::Vec::new(),
                    src_lib_rs.clone(),
                    module_manifest.interconnect.clone(),
                );
                let hash = source.hash();

                pyro_macro::module::generate_module_spec(&src_lib_rs, &dep_interfaces)
                    .map_err(|e| {
                        let err = EnvironmentError::InterfaceGeneration(e.to_string());
                        tracing::error!(error = ?err, "Failed to generate playbook spec");
                        err
                    })?
                    .map(|func| crate::artifacts::PlaybookSpec {
                        ident,
                        hash,
                        func,
                        capabilities: resolved_capabilities,
                        interconnect: module_manifest.interconnect.clone(),
                    })
                    .ok_or_else(|| {
                        let err = EnvironmentError::InterfaceGeneration(
                            "Module main function missing".to_string(),
                        );
                        tracing::error!(error = ?err, "Playbook spec missing main function");
                        err
                    })
            }
            crate::cargo::ProjectManifest::Capability(_) => {
                let err = EnvironmentError::InterfaceGeneration(
                    "Module spec is only supported for modules".to_string(),
                );
                tracing::error!(error = ?err, "Invalid manifest type for playbook spec");
                Err(err)
            }
        };
        res
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
