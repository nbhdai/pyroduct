use anyhow::{Context, Result};
use cargo_toml::Dependency;
use fs_err as fs;
use std::path::PathBuf;

pub mod compile;
pub mod ship;

pub struct CacheManager {
    root: PathBuf,
}

impl CacheManager {
    pub fn new() -> Result<Self> {
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
        manager.init()?;
        Ok(manager)
    }

    pub fn config(&self) -> PyroductConfig {
        let path = self.root.join("config.toml");
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(config) = toml::from_str(&content) {
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
    pub fn init(&self) -> Result<()> {
        fs::create_dir_all(self.capabilities_base_dir())
            .context("Failed to create capabilities cache dir")?;
        fs::create_dir_all(self.interfaces_dir())
            .context("Failed to create interfaces cache dir")?;
        let module_dir = self.root.join("modules");
        fs::create_dir_all(&module_dir).context("Failed to create modules cache dir")?;

        let build_dir = self.root.join("build");
        fs::create_dir_all(build_dir).context("Failed to create build dir")?;
        let cargo_dir = self.root.join(".cargo");
        fs::create_dir_all(&cargo_dir).context("Failed to create .cargo dir")?;
        let config = self.config();
        if let Some(target) = config.target {
            fs::write(
                cargo_dir.join("config.toml"),
                format!("[build]\ntarget-dir = \"{}\"", target.display()),
            )?;
        } else {
            fs::write(
                cargo_dir.join("config.toml"),
                "[build]\ntarget-dir = \"target\"",
            )?;
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

    pub fn interfaces_dir(&self) -> PathBuf {
        self.root.join("interfaces")
    }

    pub fn add_anon_module(&self, hash: &str, wasm: &[u8]) -> Result<()> {
        let module_dir = self.root.join("modules");
        let module_path = module_dir.join(format!("{}.wasm", hash));
        fs::write(module_path, wasm).context("Failed to write module")?;
        Ok(())
    }

    pub fn module_dir(&self, author: &str, name: &str, version: &str) -> Result<PathBuf> {
        let dir = self
            .root
            .join("modules")
            .join(author)
            .join(name)
            .join(version);
        fs::create_dir_all(&dir).context("Failed to create module dir")?;
        Ok(dir)
    }
}

#[derive(serde::Deserialize)]
pub struct PyroductConfig {
    pub author: Option<String>,
    pub target: Option<PathBuf>,
    pub pyroduct: Option<Dependency>,
}
