
use std::path::{Path, PathBuf};
use std::io;
use tokio::fs as tfs;
use cargo_toml::Dependency;
use std::sync::Arc;
use crate::cache::{CacheManager, PyroductConfig, CacheError};
use crate::artifacts::{ModuleSource, ModuleBinary, ModuleSpec};
use crate::command::{run_command, format_syn_error, CommandError};
use pyro_macro::module::generate_module_spec;
use crate::cargo::ensure_cdylib;

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
    pub async fn new(root: &Path, mut config: PyroductConfig, cache_manager: Arc<CacheManager>) -> Result<Self, CacheError> {
        tfs::create_dir_all(root).await.map_err(|e| CacheError {
            context: "Failed to create build root".to_string(),
            error: e,
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
        tracing::info!(?root, "Setup Build directory");

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

    pub async fn from_env(cache_manager: Arc<CacheManager>) -> Result<Self, CacheError> {
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
        let content = tfs::read_to_string(&config_path)
            .await
            .map_err(|error| CacheError {
                context: format!("Failed to read the configuration"),
                error,
            })?;
        let config = toml::from_str::<PyroductConfig>(&content).map_err(|error| CacheError {
            context: format!("Failed to parse the configuration"),
            error: io::Error::new(io::ErrorKind::InvalidData, error),
        })?;

        Self::new(&root, config, cache_manager).await
    }

    fn build_base_dir(&self) -> &Path {
        &self.root
    }

    async fn init(&self) -> Result<(), CacheError> {
        // Create all build slot directories
        let build_base = self.build_base_dir();
        for i in 0..self.build_slots {
            let slot_dir = build_base.join(i.to_string());
            tfs::create_dir_all(&slot_dir)
                .await
                .map_err(|error| CacheError {
                    context: format!("Failed to create build slot dir {}", i),
                    error,
                })?;
        }

        let cargo_dir = self.root.join(".cargo");
        tfs::create_dir_all(&cargo_dir)
            .await
            .map_err(|error| CacheError {
                context: "Failed to create .cargo dir".to_string(),
                error,
            })?;

        tfs::write(
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

    #[cfg(feature = "compiler")]
    pub async fn compile(&self, source: &ModuleSource) -> Result<ModuleBinary, BuildError> {
        let hash = source.hash();
        
        // Check if binary is already in cache
        if let Ok(binary) = self.cache_manager.get_binary(&hash).await {
            return Ok(binary);
        }

        // Acquire a file-locked build slot
        let slot = BuildSlot::acquire_any(&self.build_base_dir(), self.build_slots).await?;
        tracing::info!(slot = slot.index, hash = %hash, "Compiling in build slot");

        let build_dir = &slot.dir;
        let src_dir = build_dir.join("src");
        tfs::create_dir_all(&src_dir)
            .await
            .map_err(|e| BuildError::io("create src dir", e))?;
        tfs::write(src_dir.join("lib.rs"), &source.source)
            .await
            .map_err(|e| BuildError::io("write lib.rs", e))?;

        let crate_name = format!("mod_slot{}", slot.index);
        let basic_toml = format!(
            r#"
[package]
name = "{crate_name}"
version = "0.1.0"
author = "anon"
edition = "2024"

[workspace]

[lib]
name = "mod_slot"

[dependencies]
"#
        );

        let mut manifest: cargo_toml::Manifest = toml::from_str(&basic_toml)
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
            let path = self.cache_manager.interface_dir(&cap.author, &cap.package, &cap.version)
                .to_string_lossy()
                .into();
            let dep = Dependency::Detailed(Box::new(cargo_toml::DependencyDetail {
                path: Some(path),
                ..Default::default()
            }));
            manifest.dependencies.insert(cap.package.clone(), dep);
        }
        manifest.lib = ensure_cdylib(manifest.lib.take());

        // Patch pyroduct to redirect all dependencies to the builder's local path
        let mut crates_io_patch = std::collections::BTreeMap::new();
        crates_io_patch.insert("pyroduct".to_string(), self.pyroduct_dep.clone());
        manifest.patch.insert("crates-io".to_string(), crates_io_patch);

        let cargo_toml_content =
            toml::to_string_pretty(&manifest).map_err(|e| BuildError::Manifest(e.to_string()))?;
        tfs::write(build_dir.join("Cargo.toml"), &cargo_toml_content)
            .await
            .map_err(|e| BuildError::io("write Cargo.toml", e))?;

        run_command(
            build_dir,
            &["build", "--release", "--target", "wasm32-unknown-unknown"],
            true,
        )
        .await?;

        let wasm_path = self
            .target_dir
            .join("wasm32-unknown-unknown")
            .join("release")
            .join("mod_slot.wasm");

        let wasm = tfs::read(wasm_path)
            .await
            .map_err(|e| BuildError::io("read compiled wasm", e))?;

        drop(slot);

        let func = generate_module_spec(&source.source)
            .map_err(|s| {
                BuildError::Documentation(format_syn_error("Cannot generate docstring", s))
            })?
            .ok_or(BuildError::Documentation(
                "Module main functions is missing".to_string(),
            ))?;
        let spec = ModuleSpec {
            hash,
            func,
            capabilities: source.dependencies.capabilities.clone(),
        };

        let binary = ModuleBinary { wasm, spec };

        // Save to cache
        let _ = self.cache_manager.write_artifacts(&source.clone().into()).await;
        let _ = self.cache_manager.write_artifacts(&binary.clone().into()).await;

        Ok(binary)
    }
}

pub struct BuildSlot {
    pub index: usize,
    pub dir: PathBuf,
    _lock_file: std::fs::File,
}

impl BuildSlot {
    fn try_acquire(build_base: &Path, index: usize) -> io::Result<Option<Self>> {
        use fs2::FileExt;

        let slot_dir = build_base.join(index.to_string());
        std::fs::create_dir_all(&slot_dir)?;

        let lock_path = slot_dir.join(".lock");
        let lock_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)?;

        if lock_file.try_lock_exclusive().is_ok() {
            Ok(Some(BuildSlot {
                index,
                dir: slot_dir,
                _lock_file: lock_file,
            }))
        } else {
            Ok(None)
        }
    }

    async fn acquire_any(build_base: &Path, slot_count: usize) -> Result<Self, BuildError> {
        loop {
            for i in 0..slot_count {
                match Self::try_acquire(build_base, i) {
                    Ok(Some(slot)) => {
                        tracing::info!(slot = i, "Acquired build slot");
                        return Ok(slot);
                    }
                    Ok(None) => continue,
                    Err(e) => {
                        return Err(BuildError::NoSlot(format!(
                            "Failed to probe slot {}: {}",
                            i, e
                        )));
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    }
}
