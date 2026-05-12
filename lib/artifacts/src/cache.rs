use crate::{artifacts::{
    Artifact, Artifacts, Module, ModuleBinary, ModuleSource,
    Playbook,
}};

use cargo_toml::Dependency;
use std::path::{Path, PathBuf};
use std::{collections::HashMap, io};
use tokio::fs;

#[derive(Debug, thiserror::Error)]
#[error("{context}: {error}")]
pub struct CacheError {
    pub context: String,
    #[source]
    pub error: std::io::Error,
}


#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct PyroductConfig {
    pub author: Option<String>,
    pub target: Option<PathBuf>,
    pub pyroduct: Option<Dependency>,
    pub build_slots: Option<usize>,
}

/// A loaded playbook where all the libraries are on disk and the binary is loaded
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct LoadedPlaybook {
    pub binary: ModuleBinary,
    /// Per-class capability configuration. Keys are class names.
    #[serde(default)]
    pub configurations: HashMap<String, Option<serde_json::Value>>,
    #[serde(default)]
    pub paths: HashMap<String, PathBuf>,
}

// The lock file is automatically unlocked when `_lock_file` is dropped (fs2 behavior).

pub struct CacheManager {
    pub root: PathBuf,
}

impl CacheManager {
    pub async fn new(root: &Path) -> Result<Self, CacheError> {
        fs::create_dir_all(&root).await.map_err(|e| CacheError {
            context: "Failed to create cache root".to_string(),
            error: e,
        })?;
        let manager = Self {
            root: root.to_path_buf(),
        };

        manager.init().await?;
        Ok(manager)
    }

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

        Self::new(&root).await
    }



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
        let mut authors = fs::read_dir(&base).await.map_err(|e| CacheError {
            context: "Failed to read capabilities base dir".to_string(),
            error: e,
        })?;

        while let Some(author_entry) = authors.next_entry().await.map_err(|e| CacheError {
            context: "Failed to read author entry".to_string(),
            error: e,
        })? {
            let author_path = author_entry.path();
            if !author_path.is_dir() {
                continue;
            }
            let author_name = author_entry.file_name().to_string_lossy().to_string();

            let mut names = fs::read_dir(&author_path).await.map_err(|e| CacheError {
                context: format!("Failed to read author dir: {}", author_path.display()),
                error: e,
            })?;

            while let Some(name_entry) = names.next_entry().await.map_err(|e| CacheError {
                context: "Failed to read name entry".to_string(),
                error: e,
            })? {
                let name_path = name_entry.path();
                if !name_path.is_dir() {
                    continue;
                }
                let cap_name = name_entry.file_name().to_string_lossy().to_string();

                let mut versions = fs::read_dir(&name_path).await.map_err(|e| CacheError {
                    context: format!("Failed to read name dir: {}", name_path.display()),
                    error: e,
                })?;

                while let Some(version_entry) =
                    versions.next_entry().await.map_err(|e| CacheError {
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


    pub async fn write_artifacts(&self, artifacts: &Artifacts) -> Result<(), CacheError> {
        match &artifacts {
            Artifacts::CapabilityBinary(capability) => {
                let path = self.capabilities_dir(
                    &capability.ident.author,
                    &capability.ident.name,
                    &capability.ident.version,
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
                let manifest = interface.manifest.clone();
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
                let path = self.root.join("anon").join(&binary.spec.hash);
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

    pub async fn load_playbook(&self, playbook: Playbook) -> Result<LoadedPlaybook, CacheError> {
        let binary = self.get_binary(&playbook.hash).await?;
        let mut paths = HashMap::new();

        for cap in &binary.spec.capabilities {
            let path = self
                .capability_binary_path(&cap.author, &cap.package, &cap.version)
                .await?;
            paths.insert(cap.package.clone(), path);
        }

        Ok(LoadedPlaybook {
            binary,
            configurations: playbook.configurations,
            paths,
        })
    }
}

pub(crate) fn resolve_dependency_path(dep: &mut Dependency, base: &std::path::Path) {
    if let Dependency::Detailed(detail) = dep {
        if let Some(ref mut p) = detail.path {
            let path = std::path::Path::new(p.as_str());
            if path.is_relative() {
                let absolute = base.join(&path);
                *p = absolute
                    .canonicalize()
                    .unwrap_or(absolute)
                    .to_string_lossy()
                    .into_owned();
            }
        }
    }
}
