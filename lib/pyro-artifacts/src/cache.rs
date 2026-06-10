use crate::artifacts::{
    Artifact, Artifacts, Playbook, PlaybookBinary, PlaybookIdent, PlaybookSource,
};

#[cfg(feature = "compiler")]
use crate::build::AnonPlaybook;
use crate::cargo::CapabilityIdent;

use cargo_toml::Dependency;
use std::path::{Path, PathBuf};
use std::{collections::HashMap, io};
use tokio::fs;

#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("{context}: {error}")]
    Io {
        context: String,
        #[source]
        error: std::io::Error,
    },
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct PyroductConfig {
    pub author: String,
    pub target: Option<PathBuf>,
    pub pyroduct: Option<Dependency>,
    pub build_slots: Option<usize>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RemoteAddress {
    Tcp(String),
    Unix(std::path::PathBuf),
}

/// A loaded playbook where all the libraries are on disk and the binary is loaded
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct LoadedPlaybook {
    pub binary: PlaybookBinary,
    pub remote: HashMap<CapabilityIdent, RemoteAddress>,
    #[serde(default)]
    pub paths: HashMap<CapabilityIdent, PathBuf>,

    pub log_dir: std::path::PathBuf,
    pub input_dir: std::path::PathBuf,
    pub output_dir: std::path::PathBuf,

    /// Number of worker shards for concurrent processing.
    #[serde(default = "default_num_workers")]
    pub num_workers: usize,
}

fn default_num_workers() -> usize {
    4
}

// The lock file is automatically unlocked when `_lock_file` is dropped (fs2 behavior).

pub struct CacheManager {
    pub root: PathBuf,
    pub pyroduct: Option<Dependency>,
    pub author: String,
}

impl CacheManager {
    #[tracing::instrument(skip(root), fields(root = %root.display()))]
    pub async fn new(
        root: &Path,
        pyroduct: Option<Dependency>,
        author: String,
    ) -> Result<Self, CacheError> {
        tracing::debug!("Creating CacheManager instance");
        if !root.exists() {
            fs::create_dir_all(&root).await.map_err(|e| {
                let err = CacheError::Io {
                    context: "Failed to create cache root".to_string(),
                    error: e,
                };
                tracing::error!(error = ?err, "Cache root directory creation failed");
                err
            })?;
        }

        let pyroduct = if let Some(mut dep) = pyroduct {
            crate::cache::resolve_dependency_path(&mut dep, root);
            Some(dep.clone())
        } else {
            None
        };

        let manager = Self {
            root: root.to_path_buf(),
            pyroduct,
            author,
        };

        Ok(manager)
    }

    #[tracing::instrument]
    pub async fn from_env() -> Result<Self, CacheError> {
        tracing::debug!("Loading CacheManager from environment");
        let root = std::env::var("PYRODUCT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                // Check the standard systemd service cache location (Linux)
                let system_cache = PathBuf::from("/var/lib/pyro-daemon/cache");
                if system_cache.join("config.toml").exists() {
                    return system_cache;
                }

                let home = std::env::var("HOME")
                    .or_else(|_| std::env::var("USERPROFILE"))
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| PathBuf::from("."));

                // Check the user-local ~/.pyroduct location (set up by install.sh)
                let user_cache = home.join(".pyroduct");
                if user_cache.join("config.toml").exists() {
                    return user_cache;
                }

                // Check the legacy macOS Application Support location
                let macos_cache = home.join("Library/Application Support/pyro-daemon/cache");
                if macos_cache.join("config.toml").exists() {
                    return macos_cache;
                }

                // Fallback to user-local directory
                home.join(".pyroduct")
            });
        let config_path = root.join("config.toml");
        let content = fs::read_to_string(&config_path)
            .await
            .map_err(|error| {
                let err = CacheError::Io {
                    context: "Failed to read the configuration".to_string(),
                    error,
                };
                tracing::error!(error = ?err, "Failed to read CacheManager config file at {:?}", config_path);
                err
            })?;
        let config = toml::from_str::<PyroductConfig>(&content).map_err(|error| {
            let err = CacheError::Io {
                context: "Failed to parse the configuration".to_string(),
                error: io::Error::new(io::ErrorKind::InvalidData, error),
            };
            tracing::error!(error = ?err, "Failed to parse CacheManager config toml");
            err
        })?;

        Self::new(&root, config.pyroduct, config.author).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn init(&self) -> Result<(), CacheError> {
        tracing::debug!("Initializing CacheManager directories");
        fs::create_dir_all(self.capabilities_base_dir())
            .await
            .map_err(|error| {
                let err = CacheError::Io {
                    context: format!(
                        "Failed to create capabilities cache dir in {:?}",
                        self.capabilities_base_dir()
                    ),
                    error,
                };
                tracing::error!(error = ?err, "Failed to initialize capabilities dir");
                err
            })?;

        fs::create_dir_all(self.interfaces_base_dir())
            .await
            .map_err(|error| {
                let err = CacheError::Io {
                    context: "Failed to create interfaces cache dir".to_string(),
                    error,
                };
                tracing::error!(error = ?err, "Failed to initialize interfaces dir");
                err
            })?;

        let module_dir = self.root.join("modules");
        fs::create_dir_all(&module_dir).await.map_err(|error| {
            let err = CacheError::Io {
                context: "Failed to create modules cache dir".to_string(),
                error,
            };
            tracing::error!(error = ?err, "Failed to initialize modules dir");
            err
        })?;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn purge(&self) -> Result<(), CacheError> {
        tracing::debug!("Purging CacheManager directories");
        let dirs = [
            self.capabilities_base_dir(),
            self.interfaces_base_dir(),
            self.root.join("modules"),
        ];
        for dir in dirs {
            if dir.exists() {
                fs::remove_dir_all(&dir).await.map_err(|e| CacheError::Io {
                    context: format!("Failed to remove cache dir {}", dir.display()),
                    error: e,
                })?;
            }
        }
        self.init().await?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn purge_capabilities(&self) -> Result<(), CacheError> {
        tracing::debug!("Purging Capabilities from CacheManager");
        let dirs = [
            self.capabilities_base_dir(),
            self.interfaces_base_dir(),
        ];
        for dir in dirs {
            if dir.exists() {
                fs::remove_dir_all(&dir).await.map_err(|e| CacheError::Io {
                    context: format!("Failed to remove capabilities cache dir {}", dir.display()),
                    error: e,
                })?;
            }
        }
        self.init().await?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn purge_modules(&self) -> Result<(), CacheError> {
        tracing::debug!("Purging Modules from CacheManager");
        let dir = self.root.join("modules");
        if dir.exists() {
            fs::remove_dir_all(&dir).await.map_err(|e| CacheError::Io {
                context: format!("Failed to remove modules cache dir {}", dir.display()),
                error: e,
            })?;
        }
        self.init().await?;
        Ok(())
    }


    pub async fn list_available_capabilities(
        &self,
    ) -> Result<Vec<(String, String, String)>, CacheError> {
        let base = self.capabilities_base_dir();
        if !base.exists() {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();
        let mut authors = fs::read_dir(&base).await.map_err(|e| CacheError::Io {
            context: "Failed to read capabilities base dir".to_string(),
            error: e,
        })?;

        while let Some(author_entry) = authors.next_entry().await.map_err(|e| CacheError::Io {
            context: "Failed to read author entry".to_string(),
            error: e,
        })? {
            let author_path = author_entry.path();
            if !author_path.is_dir() {
                continue;
            }
            let author_name = author_entry.file_name().to_string_lossy().to_string();

            let mut names = fs::read_dir(&author_path)
                .await
                .map_err(|e| CacheError::Io {
                    context: format!("Failed to read author dir: {}", author_path.display()),
                    error: e,
                })?;

            while let Some(name_entry) = names.next_entry().await.map_err(|e| CacheError::Io {
                context: "Failed to read name entry".to_string(),
                error: e,
            })? {
                let name_path = name_entry.path();
                if !name_path.is_dir() {
                    continue;
                }
                let cap_name = name_entry.file_name().to_string_lossy().to_string();

                let mut versions = fs::read_dir(&name_path).await.map_err(|e| CacheError::Io {
                    context: format!("Failed to read name dir: {}", name_path.display()),
                    error: e,
                })?;

                while let Some(version_entry) =
                    versions.next_entry().await.map_err(|e| CacheError::Io {
                        context: "Failed to read version entry".to_string(),
                        error: e,
                    })?
                {
                    let version_path = version_entry.path();
                    if !version_path.is_dir() {
                        continue;
                    }
                    let version = version_entry.file_name().to_string_lossy().to_string();

                    if version_path.join("interface.json").exists() {
                        results.push((author_name.clone(), cap_name.clone(), version));
                    }
                }
            }
        }

        Ok(results)
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

    pub fn interface_dir(&self, author: &str, name: &str, version: &str) -> PathBuf {
        self.interfaces_base_dir()
            .join(author)
            .join(name)
            .join(version)
    }

    // --- Named module directory helpers ---

    pub fn modules_base_dir(&self) -> PathBuf {
        self.root.join("modules")
    }

    pub fn module_dir(&self, author: &str, name: &str, version: &str) -> PathBuf {
        self.modules_base_dir()
            .join(author)
            .join(name)
            .join(version)
    }

    pub async fn list_available_modules(
        &self,
    ) -> Result<Vec<(String, String, String)>, CacheError> {
        let base = self.modules_base_dir();
        if !base.exists() {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();
        let mut authors = fs::read_dir(&base).await.map_err(|e| CacheError::Io {
            context: "Failed to read modules base dir".to_string(),
            error: e,
        })?;

        while let Some(author_entry) = authors.next_entry().await.map_err(|e| CacheError::Io {
            context: "Failed to read author entry".to_string(),
            error: e,
        })? {
            let author_path = author_entry.path();
            if !author_path.is_dir() {
                continue;
            }
            let author_name = author_entry.file_name().to_string_lossy().to_string();

            let mut names = fs::read_dir(&author_path)
                .await
                .map_err(|e| CacheError::Io {
                    context: format!("Failed to read author dir: {}", author_path.display()),
                    error: e,
                })?;

            while let Some(name_entry) = names.next_entry().await.map_err(|e| CacheError::Io {
                context: "Failed to read name entry".to_string(),
                error: e,
            })? {
                let name_path = name_entry.path();
                if !name_path.is_dir() {
                    continue;
                }
                let mod_name = name_entry.file_name().to_string_lossy().to_string();

                let mut versions = fs::read_dir(&name_path).await.map_err(|e| CacheError::Io {
                    context: format!("Failed to read name dir: {}", name_path.display()),
                    error: e,
                })?;

                while let Some(version_entry) =
                    versions.next_entry().await.map_err(|e| CacheError::Io {
                        context: "Failed to read version entry".to_string(),
                        error: e,
                    })?
                {
                    let version_path = version_entry.path();
                    if !version_path.is_dir() {
                        continue;
                    }
                    let version = version_entry.file_name().to_string_lossy().to_string();

                    if version_path.join("spec.json").exists() {
                        results.push((author_name.clone(), mod_name.clone(), version));
                    }
                }
            }
        }

        Ok(results)
    }

    pub fn interfaces_base_dir(&self) -> PathBuf {
        self.root.join("interfaces")
    }

    pub async fn capability_interface_spec(
        &self,
        author: &str,
        name: &str,
        version: &str,
    ) -> Result<String, CacheError> {
        let path = self
            .capabilities_dir(author, name, version)
            .join("interface.json");
        fs::read_to_string(&path)
            .await
            .map_err(|error| CacheError::Io {
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
            Err(CacheError::NotFound(format!(
                "Missing {} binary for this system",
                path.display()
            )))
        } else {
            Ok(path)
        }
    }

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
                .map_err(|error| CacheError::Io {
                    context: format!("Failed to read config.json from {}", path.display()),
                    error,
                })?;
            Ok(Some(content))
        } else {
            Ok(None)
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn remove_module(
        &self,
        author: &str,
        name: &str,
        version: &str,
    ) -> Result<(), CacheError> {
        tracing::debug!("Removing module from cache");
        let path = self.module_dir(author, name, version);
        if path.exists() {
            tokio::fs::remove_dir_all(&path).await.map_err(|error| {
                let err = CacheError::Io {
                    context: "Unable to remove module".to_string(),
                    error,
                };
                tracing::error!(error = ?err, "Failed to remove playbook at {:?}", path);
                err
            })?;
        }
        Ok(())
    }

    // --- Named module lookup ---

    #[tracing::instrument(skip(self))]
    pub async fn get_named_binary(
        &self,
        author: &str,
        package: &str,
        version: &str,
    ) -> Result<PlaybookBinary, CacheError> {
        tracing::debug!("Retrieving named playbook binary");
        let path = self.module_dir(author, package, version);
        if path.exists() {
            let binary = PlaybookBinary::from_dir(&path).await.map_err(|error| {
                let err = CacheError::Io {
                    context: "Unable to load named module binary".to_string(),
                    error,
                };
                tracing::error!(error = ?err, "Failed to load named module binary from {:?}", path);
                err
            })?;
            Ok(binary)
        } else {
            let err = CacheError::NotFound(format!(
                "Missing named module binary for {}/{}/{}",
                author, package, version
            ));
            tracing::debug!("Named module binary not found at {:?}", path);
            Err(err)
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_named_source(
        &self,
        author: &str,
        package: &str,
        version: &str,
    ) -> Result<PlaybookSource, CacheError> {
        tracing::debug!("Retrieving named playbook source");
        let path = self.module_dir(author, package, version);
        if path.exists() {
            let mut source = PlaybookSource::from_dir(&path).await.map_err(|error| {
                let err = CacheError::Io {
                    context: "Unable to load named module source".to_string(),
                    error,
                };
                tracing::error!(error = ?err, "Failed to load named module source from {:?}", path);
                err
            })?;
            source.manifest.module = crate::cargo::CapabilityIdent {
                author: author.to_string(),
                package: package.to_string(),
                version: version.to_string(),
            };
            Ok(source)
        } else {
            let err = CacheError::NotFound(format!(
                "Missing named module source for {}/{}/{}",
                author, package, version
            ));
            tracing::debug!("Named module source not found at {:?}", path);
            Err(err)
        }
    }

    #[tracing::instrument(skip(self, artifacts))]
    pub async fn write_artifacts(&self, artifacts: &Artifacts) -> Result<(), CacheError> {
        tracing::debug!("Writing artifacts to CacheManager");
        let res = async {
            match &artifacts {
                Artifacts::CapabilityBinary(capability) => {
                    let path = self.capabilities_dir(
                        &capability.ident.author,
                        &capability.ident.package,
                        &capability.ident.version,
                    );
                    capability
                        .write_to_directory(&path)
                        .await
                        .map_err(|e| CacheError::Io {
                            context: format!("Failed to write artifacts to {}", path.display()),
                            error: e,
                        })
                }
                Artifacts::CapabilitySource(capability) => {
                    let path = self.capabilities_dir(
                        &capability.manifest.capability.author,
                        &capability.manifest.capability.package,
                        &capability.manifest.capability.version,
                    );
                    capability
                        .write_to_directory(&path)
                        .await
                        .map_err(|e| CacheError::Io {
                            context: format!("Failed to write artifacts to {}", path.display()),
                            error: e,
                        })
                }
                Artifacts::Interface(interface) => {
                    let path = self.interface_dir(
                        &interface.manifest.capability.author,
                        &interface.manifest.capability.package,
                        &interface.manifest.capability.version,
                    );
                    fs::create_dir_all(&path)
                        .await
                        .map_err(|e| CacheError::Io {
                            context: format!("Failed to create  {}", path.display()),
                            error: e,
                        })?;
                    let mut manifest = interface.manifest.clone();
                    if let Some(pyroduct) = &self.pyroduct {
                        manifest.pyroduct = pyroduct.clone();
                    }
                    let cargo_path = path.join("Cargo.toml");
                    let cargo = manifest.clone().to_interface_manifest();
                    let cargo = toml::to_string_pretty(&cargo).map_err(|e| CacheError::Io {
                        context: format!(
                            "Failed to serialize Cargo.toml to {}",
                            cargo_path.display()
                        ),
                        error: io::Error::new(io::ErrorKind::InvalidData, e),
                    })?;
                    fs::write(&cargo_path, cargo)
                        .await
                        .map_err(|e| CacheError::Io {
                            context: format!(
                                "Failed to write Cargo.toml to {}",
                                cargo_path.display()
                            ),
                            error: e,
                        })?;
                    interface
                        .write_to_directory(&path)
                        .await
                        .map_err(|e| CacheError::Io {
                            context: format!("Failed to write artifacts to {}", path.display()),
                            error: e,
                        })
                }
                Artifacts::Playbook(Playbook::Binary(binary)) => {
                    let ident = &binary.spec.ident;
                    let path = self.module_dir(&ident.author, &ident.package, &ident.version);
                    fs::create_dir_all(&path)
                        .await
                        .map_err(|e| CacheError::Io {
                            context: format!("Failed to create module dir {}", path.display()),
                            error: e,
                        })?;
                    binary
                        .write_to_directory(&path)
                        .await
                        .map_err(|e| CacheError::Io {
                            context: format!("Failed to write artifacts to {}", path.display()),
                            error: e,
                        })
                }
                Artifacts::Playbook(Playbook::Source(source)) => {
                    let ident = source.ident();
                    let path = self.module_dir(&ident.author, &ident.package, &ident.version);
                    fs::create_dir_all(&path)
                        .await
                        .map_err(|e| CacheError::Io {
                            context: format!("Failed to create module dir {}", path.display()),
                            error: e,
                        })?;
                    source
                        .write_to_directory(&path)
                        .await
                        .map_err(|e| CacheError::Io {
                            context: format!("Failed to write artifacts to {}", path.display()),
                            error: e,
                        })
                }
            }
        }
        .await;

        if let Err(ref e) = res {
            tracing::error!(error = ?e, "Failed to write artifacts to cache");
        } else {
            tracing::debug!("Successfully wrote artifacts to cache");
        }
        res
    }

    #[tracing::instrument(skip(self, remotes, log_dir, input_dir, output_dir), fields(author = playbook.author, name = playbook.package, version = playbook.version))]
    pub async fn load_playbook(
        &self,
        playbook: PlaybookIdent,
        remotes: HashMap<CapabilityIdent, RemoteAddress>,
        log_dir: impl AsRef<Path>,
        input_dir: impl AsRef<Path>,
        output_dir: impl AsRef<Path>,
        num_workers: usize,
    ) -> Result<LoadedPlaybook, CacheError> {
        tracing::debug!("Loading playbook");
        let res = async {
            let binary = self.get_named_binary(&playbook.author, &playbook.package, &playbook.version).await?;
            let mut paths = HashMap::new();
            let mut remote = HashMap::new();

            tracing::debug!(capabilities = ?binary.spec.capabilities, "Loading playbook capabilities");
            tracing::debug!(
                config_keys = ?binary
                    .configurations
                    .iter()
                    .map(|c| &c.package)
                    .collect::<Vec<_>>(),
                "Loaded playbook configuration keys"
            );

            for cap in &binary.spec.capabilities {
                if let Some(addr) = remotes.get(cap) {
                    remote.insert(cap.clone(), addr.clone());
                } else if binary
                    .configurations
                    .iter()
                    .any(|c| c.package == cap.package)
                {
                    let path = self
                        .capability_binary_path(&cap.author, &cap.package, &cap.version)
                        .await?;
                    paths.insert(cap.clone(), path);
                } else {
                    return Err(CacheError::NotFound(format!("Capability {} not found", cap.package)));
                }
            }

            Ok(LoadedPlaybook {
                binary,
                remote,
                paths,
                log_dir: log_dir.as_ref().to_path_buf(),
                input_dir: input_dir.as_ref().to_path_buf(),
                output_dir: output_dir.as_ref().to_path_buf(),
                num_workers,
            })
        }.await;

        if let Err(ref e) = res {
            tracing::error!(error = ?e, "Failed to load playbook");
        } else {
            tracing::debug!("Successfully loaded playbook");
        }
        res
    }

    #[cfg(feature = "compiler")]
    pub fn convert_anon_playbook(&self, playbook: AnonPlaybook) -> PlaybookSource {
        let author = self.author.clone();
        let mut resolved_capabilities = Vec::new();
        for cap in &playbook.configurations {
            resolved_capabilities.push(CapabilityIdent {
                author: cap.author.clone(),
                package: cap.package.clone(),
                version: cap.version.clone(),
            });
        }
        PlaybookSource::new(
            crate::artifacts::PlaybookIdent {
                author,
                package: playbook.package,
                version: "0.1.0".to_string(),
            },
            crate::artifacts::ModuleDependencies {
                dependencies: playbook.dependencies,
                capabilities: resolved_capabilities,
            },
            playbook.configurations,
            playbook.source,
            playbook.interconnect,
        )
    }
}

pub(crate) fn resolve_dependency_path(dep: &mut Dependency, base: &std::path::Path) {
    if let Dependency::Detailed(detail) = dep
        && let Some(ref mut p) = detail.path
    {
        let path = std::path::Path::new(p.as_str());
        if path.is_relative() {
            let absolute = base.join(path);
            *p = absolute
                .canonicalize()
                .unwrap_or(absolute)
                .to_string_lossy()
                .into_owned();
        }
    }
}
