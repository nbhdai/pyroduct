use crate::artifacts::{PlaybookBinary, PlaybookSource, PlaybookSpec};
use crate::cache::{CacheError, CacheManager, PyroductConfig};
use crate::cargo::{ConfiguredCapability, ensure_cdylib};
use crate::command::{CommandError, format_syn_error, run_command};
use cargo_toml::Dependency;
use pyro_macro::module::generate_module_spec;

use crate::artifacts::PlaybookIdent;

#[derive(Debug, Clone, PartialEq)]
pub struct AnonPlaybook {
    pub package: String,
    pub dependencies: std::collections::BTreeMap<String, Dependency>,
    pub configurations: Vec<ConfiguredCapability>,
    pub source: String,
    pub interconnect: std::collections::BTreeMap<String, PlaybookIdent>,
}
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs as tfs;

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

    #[error("No build slot available: {0}")]
    NoSlot(String),
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

pub struct Builder {
    pub root: PathBuf,
    pub target_dir: PathBuf,
    pub pyroduct_dep: Dependency,
    pub config: PyroductConfig,
    pub build_slots: usize,
    pub cache_manager: Arc<CacheManager>,
}

impl Builder {
    #[tracing::instrument(skip(root, cache_manager), fields(root = %root.display()))]
    pub async fn new(
        root: &Path,
        mut config: PyroductConfig,
        cache_manager: Arc<CacheManager>,
    ) -> Result<Self, CacheError> {
        tracing::debug!("Creating Builder instance");
        tfs::create_dir_all(root).await.map_err(|e| {
            let err = CacheError::Io {
                context: "Failed to create build root".to_string(),
                error: e,
            };
            tracing::error!(error = ?err, "Failed to create build root directory");
            err
        })?;

        let pyroduct_dep = if let Some(dep) = &mut config.pyroduct {
            crate::cache::resolve_dependency_path(dep, root);
            dep.clone()
        } else {
            Dependency::Simple("*".to_string())
        };

        let target_dir = if let Some(target) = &config.target {
            if target.is_relative() {
                root.join(target)
            } else {
                target.clone()
            }
        } else {
            root.join("target")
        };

        let build_slots = config.build_slots.unwrap_or(4).max(1);
        tracing::debug!(?root, "Setup Build directory");

        let builder = Self {
            root: root.to_path_buf(),
            target_dir,
            pyroduct_dep,
            config,
            build_slots,
            cache_manager,
        };

        builder.init().await?;
        Ok(builder)
    }

    #[tracing::instrument(skip(cache_manager))]
    pub async fn from_env(cache_manager: Arc<CacheManager>) -> Result<Self, CacheError> {
        tracing::debug!("Loading Builder from environment");
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
        let content = tfs::read_to_string(&config_path).await.map_err(|error| {
            let err = CacheError::Io {
                context: "Failed to read the configuration".to_string(),
                error,
            };
            tracing::error!(error = ?err, "Failed to read config file at {:?}", config_path);
            err
        })?;
        let config = toml::from_str::<PyroductConfig>(&content).map_err(|error| {
            let err = CacheError::Io {
                context: "Failed to parse the configuration".to_string(),
                error: io::Error::new(io::ErrorKind::InvalidData, error),
            };
            tracing::error!(error = ?err, "Failed to parse configuration toml");
            err
        })?;

        Self::new(&root, config, cache_manager).await
    }

    fn build_base_dir(&self) -> &Path {
        &self.root
    }

    #[tracing::instrument(skip(self))]
    async fn init(&self) -> Result<(), CacheError> {
        tracing::debug!("Initializing Builder directories");
        // Create all build slot directories
        let build_base = self.build_base_dir();
        for i in 0..self.build_slots {
            let slot_dir = build_base.join(i.to_string());
            tfs::create_dir_all(&slot_dir).await.map_err(|error| {
                let err = CacheError::Io {
                    context: format!("Failed to create build slot dir {}", i),
                    error,
                };
                tracing::error!(error = ?err, "Failed to create build slot directory {}", i);
                err
            })?;
        }

        let cargo_dir = self.root.join(".cargo");
        tfs::create_dir_all(&cargo_dir).await.map_err(|error| {
            let err = CacheError::Io {
                context: "Failed to create .cargo dir".to_string(),
                error,
            };
            tracing::error!(error = ?err, "Failed to create .cargo directory");
            err
        })?;

        tfs::write(
            cargo_dir.join("config.toml"),
            format!("[build]\ntarget-dir = \"{}\"", self.target_dir.display()),
        )
        .await
        .map_err(|error| {
            let err = CacheError::Io {
                context: "Failed to write target config.toml".to_string(),
                error,
            };
            tracing::error!(error = ?err, "Failed to write .cargo/config.toml");
            err
        })?;
        Ok(())
    }

    #[cfg(feature = "compiler")]
    #[tracing::instrument(skip(self, playbook))]
    pub async fn compile_anon(
        &self,
        playbook: &AnonPlaybook,
    ) -> Result<PlaybookBinary, BuildError> {
        let source = self.cache_manager.convert_anon_playbook(playbook.clone());
        self.compile(&source).await
    }

    #[cfg(feature = "compiler")]
    #[tracing::instrument(skip(self, source), fields(source_hash = %source.hash()))]
    pub async fn compile(&self, source: &PlaybookSource) -> Result<PlaybookBinary, BuildError> {
        let hash = source.hash();
        let source_ident = source.ident();
        if source_ident.package == "anon" {
            return Err(BuildError::Manifest(
                "Playbook name cannot be 'anon'".to_string(),
            ));
        }

        let mut resolved_version = source_ident.version.clone();
        let mut found_existing = false;

        if source_ident.author == "anon" {
            let mut version_num = 1;
            loop {
                let version_str = format!("0.{}.0", version_num);
                match self
                    .cache_manager
                    .get_named_source("anon", &source_ident.package, &version_str)
                    .await
                {
                    Ok(existing_source) => {
                        if existing_source.hash() == hash {
                            resolved_version = version_str;
                            found_existing = true;
                            break;
                        } else {
                            version_num += 1;
                        }
                    }
                    Err(_) => {
                        resolved_version = version_str;
                        break;
                    }
                }
            }
        } else {
            if let Ok(binary) = self
                .cache_manager
                .get_named_binary(
                    &source_ident.author,
                    &source_ident.package,
                    &source_ident.version,
                )
                .await
            {
                if binary.spec.hash == hash {
                    tracing::debug!("Named playbook binary found in cache, skipping compilation");
                    return Ok(binary);
                }
            }
        }

        if found_existing {
            if let Ok(binary) = self
                .cache_manager
                .get_named_binary("anon", &source_ident.package, &resolved_version)
                .await
            {
                tracing::debug!(
                    "Playbook binary found in cache (conflict resolved), skipping compilation"
                );
                return Ok(binary);
            }
        }

        // Acquire a file-locked build slot
        let slot = BuildSlot::acquire_any(self.build_base_dir(), self.build_slots).await?;
        tracing::info!(slot = slot.index, hash = %hash, "Compiling in build slot");

        let build_dir = &slot.dir;
        let src_dir = build_dir.join("src");
        tfs::create_dir_all(&src_dir).await.map_err(|e| {
            let err = BuildError::io("create src dir", e);
            tracing::error!(error = ?err, "Failed to create src directory in slot");
            err
        })?;
        tfs::write(src_dir.join("lib.rs"), &source.source)
            .await
            .map_err(|e| {
                let err = BuildError::io("write lib.rs", e);
                tracing::error!(error = ?err, "Failed to write lib.rs in slot");
                err
            })?;

        let crate_name = format!("mod_slot{}", slot.index);
        let author = &source_ident.author;
        let basic_toml = format!(
            r#"
[package]
name = "{crate_name}"
version = "{resolved_version}"
authors = ["{author}"]
edition = "2024"

[workspace]

[lib]
name = "mod_slot"

[dependencies]
"#
        );

        let mut manifest: cargo_toml::Manifest = toml::from_str(&basic_toml).map_err(|e| {
            let err = BuildError::Manifest(format!("Couldn't build base manifest: {}", e));
            tracing::error!(error = ?err, "Failed to parse base basic_toml");
            err
        })?;
        let mut pyro_dep = self.pyroduct_dep.clone();
        pyro_dep.detail_mut().features.push("module".to_string());
        manifest
            .dependencies
            .insert("pyroduct".to_string(), pyro_dep);
        for (dep_name, dep) in source.dependencies().dependencies.iter() {
            manifest.dependencies.insert(dep_name.clone(), dep.clone());
        }
        // Copy interfaces into the build slot with the builder's pyroduct dep
        let build_ifaces_dir = build_dir.join(".pyro_interfaces");
        let caps = source.dependencies().capabilities.clone();
        self.cache_manager
            .prepare_build_interfaces(&build_ifaces_dir, &self.pyroduct_dep, &caps)
            .await
            .map_err(|e| {
                let err =
                    BuildError::Manifest(format!("Failed to prepare build interfaces: {}", e));
                tracing::error!(error = ?err);
                err
            })?;

        for cap in caps.iter() {
            let path: String = build_ifaces_dir
                .join(&cap.author)
                .join(&cap.package)
                .join(&cap.version)
                .to_string_lossy()
                .into();
            let dep = Dependency::Detailed(Box::new(cargo_toml::DependencyDetail {
                path: Some(path),
                ..Default::default()
            }));
            manifest.dependencies.insert(cap.package.clone(), dep);
        }
        manifest.lib = ensure_cdylib(manifest.lib.take());

        let cargo_toml_content = toml::to_string_pretty(&manifest).map_err(|e| {
            let err = BuildError::Manifest(e.to_string());
            tracing::error!(error = ?err, "Failed to serialize slot Cargo.toml");
            err
        })?;
        tfs::write(build_dir.join("Cargo.toml"), &cargo_toml_content)
            .await
            .map_err(|e| {
                let err = BuildError::io("write Cargo.toml", e);
                tracing::error!(error = ?err, "Failed to write slot Cargo.toml");
                err
            })?;

        tracing::debug!("Running cargo compilation command in slot {}", slot.index);
        run_command(
            build_dir,
            &["build", "--release", "--target", "wasm32-unknown-unknown"],
            true,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Cargo compilation failed");
            BuildError::Command(e)
        })?;

        let wasm_path = self
            .target_dir
            .join("wasm32-unknown-unknown")
            .join("release")
            .join("mod_slot.wasm");

        let wasm = tfs::read(&wasm_path)
            .await
            .map_err(|e| {
                let err = BuildError::io("read compiled wasm", e);
                tracing::error!(error = ?err, "Failed to read compiled WASM artifact at {:?}", wasm_path);
                err
            })?;

        drop(slot);

        let mut dep_interfaces = Vec::new();
        for cap in source.dependencies().capabilities.iter() {
            if let Ok(spec_str) = self
                .cache_manager
                .capability_interface_spec(&cap.author, &cap.package, &cap.version)
                .await
            {
                if let Ok(spec) = serde_json::from_str::<pyro_spec::InterfaceSpec>(&spec_str) {
                    dep_interfaces.push(spec);
                }
            }
        }

        let func = generate_module_spec(&source.source, &dep_interfaces)
            .map_err(|s| {
                let err = BuildError::Documentation(format_syn_error("Cannot generate docstring", s));
                tracing::error!(error = ?err, "Failed to generate module spec from source docstrings");
                err
            })?
            .ok_or_else(|| {
                let err = BuildError::Documentation("Module main functions is missing".to_string());
                tracing::error!(error = ?err, "Module main function is missing");
                err
            })?;
        let spec = PlaybookSpec {
            ident: crate::artifacts::PlaybookIdent {
                author: source_ident.author.clone(),
                package: source_ident.package.clone(),
                version: resolved_version.clone(),
            },
            hash,
            func,
            capabilities: source.dependencies().capabilities.clone(),
            interconnect: source.manifest.interconnect.clone(),
        };

        let binary = PlaybookBinary {
            wasm,
            spec,
            configurations: source.configurations().clone(),
        };

        let mut updated_source = source.clone();
        updated_source.manifest.module = crate::cargo::CapabilityIdent {
            author: source_ident.author.clone(),
            package: source_ident.package.clone(),
            version: resolved_version.clone(),
        };

        // Save to cache
        tracing::debug!("Saving source and compiled binary to CacheManager");
        if let Err(e) = self
            .cache_manager
            .write_artifacts(&updated_source.into())
            .await
        {
            tracing::error!(error = ?e, "Failed to save playbook source to cache");
        }
        if let Err(e) = self
            .cache_manager
            .write_artifacts(&binary.clone().into())
            .await
        {
            tracing::error!(error = ?e, "Failed to save playbook binary to cache");
        }

        tracing::info!("Playbook compilation completed successfully");
        Ok(binary)
    }
}

pub struct BuildSlot {
    pub index: usize,
    pub dir: PathBuf,
    _lock_file: std::fs::File,
}

impl BuildSlot {
    #[tracing::instrument(skip(build_base))]
    fn try_acquire(build_base: &Path, index: usize) -> io::Result<Option<Self>> {
        use fs2::FileExt;

        tracing::debug!("Probing build slot lock file");
        let slot_dir = build_base.join(index.to_string());
        std::fs::create_dir_all(&slot_dir)?;

        let lock_path = slot_dir.join(".lock");
        let lock_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)?;

        if lock_file.try_lock_exclusive().is_ok() {
            tracing::debug!("Lock acquired successfully for build slot {}", index);
            Ok(Some(BuildSlot {
                index,
                dir: slot_dir,
                _lock_file: lock_file,
            }))
        } else {
            tracing::debug!("Build slot {} lock is already held", index);
            Ok(None)
        }
    }

    #[tracing::instrument(skip(build_base))]
    async fn acquire_any(build_base: &Path, slot_count: usize) -> Result<Self, BuildError> {
        tracing::debug!("Acquiring any available build slot...");
        loop {
            for i in 0..slot_count {
                match Self::try_acquire(build_base, i) {
                    Ok(Some(slot)) => {
                        tracing::debug!(slot = i, "Acquired build slot");
                        return Ok(slot);
                    }
                    Ok(None) => continue,
                    Err(e) => {
                        let err = BuildError::NoSlot(format!("Failed to probe slot {}: {}", i, e));
                        tracing::error!(error = ?err, "Failed to probe build slot lock file");
                        return Err(err);
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    }
}
