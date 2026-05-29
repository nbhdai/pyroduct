use cargo_toml::Dependency;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use pyro_spec::{InterfaceSpec, ModuleFunc};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::future::Future;
use std::io::{self, Read, Write};
use std::ops::Deref;
use std::path::Path;
use tar::{Builder, Header};
use tokio::fs;

use crate::cargo::{CapabilityIdent, CapabilityManifest, ConfiguredCapability, ResolvedCapability};

pub enum CapBinary {
    Pe(Vec<u8>),
    MachO(Vec<u8>),
    Elf(Vec<u8>),
}

impl Deref for CapBinary {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        match self {
            CapBinary::Pe(items) => items,
            CapBinary::MachO(items) => items,
            CapBinary::Elf(items) => items,
        }
    }
}

pub struct CapabilityBinary {
    pub ident: CapabilityIdent,
    pub libs: Vec<CapBinary>,
    pub interface: InterfaceSpec<'static>,
}

pub struct CapabilitySource {
    pub manifest: CapabilityManifest,
    pub cargo_toml: String,
    pub cargo_lock: String,
    pub src_lib_rs: String,
}

pub struct Interface {
    pub manifest: CapabilityManifest,
    pub src_lib_rs: String,
    pub interface: InterfaceSpec<'static>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct ModuleDependencies {
    pub dependencies: BTreeMap<String, Dependency>,
    pub capabilities: Vec<ResolvedCapability>,
}

/// Identity for a named module (from Module.toml [module] section).
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct PlaybookIdent {
    pub author: String,
    pub name: String,
    pub version: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct PlaybookSource {
    /// Named module identity (author/name/version), if this is a named module.
    #[serde(default)]
    pub ident: Option<PlaybookIdent>,
    pub dependencies: ModuleDependencies,
    pub source: String,
    #[serde(default)]
    pub configurations: Vec<ConfiguredCapability>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct PlaybookSpec {
    pub hash: String,
    pub func: ModuleFunc<'static>,
    pub capabilities: Vec<ResolvedCapability>,
    #[serde(default)]
    pub ident: Option<PlaybookIdent>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct PlaybookBinary {
    /// Named module identity (author/name/version), if this is a named module.
    #[serde(default)]
    pub ident: Option<PlaybookIdent>,
    pub wasm: Vec<u8>,
    pub spec: PlaybookSpec,
    #[serde(default)]
    pub configurations: Vec<ConfiguredCapability>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub enum Playbook {
    Source(PlaybookSource),
    Binary(PlaybookBinary),
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub struct CapabilityConfig {
    /// Per-class capability configuration. Keys are class names.
    pub classes: HashMap<String, Option<serde_json::Value>>,
}

impl std::hash::Hash for CapabilityConfig {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let mut entries: Vec<_> = self.classes.iter().collect();
        entries.sort_by_key(|(k, _)| *k);
        for (k, v) in entries {
            k.hash(state);
            if let Some(val) = v {
                if let Ok(json_str) = serde_json::to_string(val) {
                    json_str.hash(state);
                }
            } else {
                0.hash(state);
            }
        }
    }
}

impl PlaybookSource {
    /// Computes a deterministic hash of the module's source and dependencies.
    pub fn hash(&self) -> String {
        Self::compute_hash(
            &self.source,
            &self.dependencies.dependencies,
            &self.dependencies.capabilities,
        )
    }

    pub fn compute_hash(
        code: &str,
        dependencies: &std::collections::BTreeMap<String, cargo_toml::Dependency>,
        capabilities: &[crate::cargo::ResolvedCapability],
    ) -> String {
        let mut hasher = Sha256::new();

        hasher.update(code.as_bytes());

        if let Ok(deps_json) = serde_json::to_string(dependencies) {
            hasher.update(deps_json.as_bytes());
        }

        let mut sorted_caps = capabilities.to_vec();
        sorted_caps.sort_by(|a, b| {
            a.package
                .cmp(&b.package)
                .then_with(|| a.author.cmp(&b.author))
                .then_with(|| a.version.cmp(&b.version))
        });

        if let Ok(caps_json) = serde_json::to_string(&sorted_caps) {
            hasher.update(caps_json.as_bytes());
        }

        format!("{:x}", hasher.finalize())
    }
}

impl PlaybookBinary {
    pub fn hash(&self) -> String {
        self.spec.hash.clone()
    }
}

impl Playbook {
    pub fn hash(&self) -> String {
        match self {
            Playbook::Source(m) => m.hash(),
            Playbook::Binary(m) => m.hash(),
        }
    }
}

pub enum Artifacts {
    CapabilityBinary(CapabilityBinary),
    CapabilitySource(CapabilitySource),
    Interface(Interface),
    Playbook(Playbook),
}

impl From<CapabilityBinary> for Artifacts {
    fn from(value: CapabilityBinary) -> Self {
        Artifacts::CapabilityBinary(value)
    }
}

impl From<CapabilitySource> for Artifacts {
    fn from(value: CapabilitySource) -> Self {
        Artifacts::CapabilitySource(value)
    }
}

impl From<Interface> for Artifacts {
    fn from(value: Interface) -> Self {
        Artifacts::Interface(value)
    }
}

impl From<Playbook> for Artifacts {
    fn from(value: Playbook) -> Self {
        Artifacts::Playbook(value)
    }
}

impl From<PlaybookBinary> for Artifacts {
    fn from(value: PlaybookBinary) -> Self {
        Artifacts::Playbook(Playbook::Binary(value))
    }
}

impl From<PlaybookSource> for Artifacts {
    fn from(value: PlaybookSource) -> Self {
        Artifacts::Playbook(Playbook::Source(value))
    }
}

/// The common trait for all artifact types.
/// Requires Sized so we can return Self for the constructors.
pub trait Artifact: Sized {
    fn write_to_directory(&self, path: &Path) -> impl Future<Output = io::Result<()>> + Send;
    fn to_tarball(&self) -> Result<Vec<u8>, io::Error>;

    fn from_tarball(bytes: &[u8]) -> Result<Self, io::Error>;
    fn from_dir(path: &Path) -> impl Future<Output = Result<Self, io::Error>> + Send;
}

// --- Helper for appending files to a tarball ---
pub(crate) fn append_file<W: Write>(
    tar: &mut Builder<W>,
    name: &str,
    data: &[u8],
) -> Result<(), io::Error> {
    let mut header = Header::new_gnu();
    header.set_size(data.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(&mut header, name, data)
}

// ==========================================
// Trait Implementations for Concrete Structs
// ==========================================

impl Artifact for CapabilityBinary {
    #[tracing::instrument(skip(self, path), fields(path = %path.display(), ident = ?self.ident))]
    async fn write_to_directory(&self, path: &Path) -> io::Result<()> {
        tracing::debug!("Writing CapabilityBinary to directory");
        fs::create_dir_all(path).await?;
        for lib in &self.libs {
            match lib {
                CapBinary::Pe(bytes) => fs::write(path.join("lib.dll"), bytes).await?,
                CapBinary::MachO(bytes) => fs::write(path.join("lib.dylib"), bytes).await?,
                CapBinary::Elf(bytes) => fs::write(path.join("lib.so"), bytes).await?,
            }
        }
        fs::write(
            path.join("ident.json"),
            serde_json::to_string(&self.ident).map_err(|e| {
                let err = io::Error::new(io::ErrorKind::InvalidData, e);
                tracing::error!(error = ?err, "Failed to serialize ident");
                err
            })?,
        )
        .await?;
        let interface_json = serde_json::to_string_pretty(&self.interface).map_err(|e| {
            let err = io::Error::new(io::ErrorKind::InvalidData, e);
            tracing::error!(error = ?err, "Failed to serialize interface");
            err
        })?;
        fs::write(path.join("interface.json"), interface_json).await?;
        Ok(())
    }

    fn to_tarball(&self) -> Result<Vec<u8>, io::Error> {
        let encoder = GzEncoder::new(Vec::new(), Compression::default());
        let mut tar = Builder::new(encoder);

        for lib in &self.libs {
            match lib {
                CapBinary::Pe(bytes) => append_file(&mut tar, "lib.dll", bytes)?,
                CapBinary::MachO(bytes) => append_file(&mut tar, "lib.dylib", bytes)?,
                CapBinary::Elf(bytes) => append_file(&mut tar, "lib.so", bytes)?,
            }
        }
        append_file(
            &mut tar,
            "ident.json",
            serde_json::to_string(&self.ident)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
                .as_bytes(),
        )?;
        append_file(
            &mut tar,
            "interface.json",
            serde_json::to_string_pretty(&self.interface)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
                .as_bytes(),
        )?;

        tar.into_inner()?.finish()
    }

    fn from_tarball(bytes: &[u8]) -> Result<Self, io::Error> {
        let tar = GzDecoder::new(bytes);
        let mut archive = tar::Archive::new(tar);

        let mut libs = Vec::new();
        let mut ident = None;
        let mut interface = None;

        for file in archive.entries()? {
            let mut file = file?;
            let path = file.path()?.to_path_buf();
            let mut content = Vec::new();
            file.read_to_end(&mut content)?;

            match path.to_string_lossy().as_ref() {
                "lib.dll" => libs.push(CapBinary::Pe(content)),
                "lib.dylib" => libs.push(CapBinary::MachO(content)),
                "lib.so" => libs.push(CapBinary::Elf(content)),
                "ident.json" => {
                    ident = serde_json::from_slice(&content).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Unable to deserialize manifest: {}", e),
                        )
                    })?;
                }
                "interface.json" => {
                    interface = serde_json::from_slice(&content).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Unable to deserialize interface: {}", e),
                        )
                    })?;
                }
                _ => {}
            }
        }

        if libs.is_empty() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "Missing library"));
        }

        Ok(CapabilityBinary {
            ident: ident
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing ident.json"))?,
            libs,
            interface: interface.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "Missing interface.json")
            })?,
        })
    }

    #[tracing::instrument(skip(path), fields(path = %path.display()))]
    async fn from_dir(path: &Path) -> Result<Self, io::Error> {
        tracing::debug!("Loading CapabilityBinary from directory");
        let mut libs = Vec::new();
        if let Ok(bytes) = fs::read(path.join("lib.dll")).await {
            libs.push(CapBinary::Pe(bytes));
        }
        if let Ok(bytes) = fs::read(path.join("lib.dylib")).await {
            libs.push(CapBinary::MachO(bytes));
        }
        if let Ok(bytes) = fs::read(path.join("lib.so")).await {
            libs.push(CapBinary::Elf(bytes));
        }

        if libs.is_empty() {
            let err = io::Error::new(io::ErrorKind::NotFound, "Missing capability library");
            tracing::error!(error = ?err, "Failed to load capability library");
            return Err(err);
        }

        let ident_string = fs::read(path.join("ident.json")).await?;
        let ident = serde_json::from_slice(&ident_string).map_err(|e| {
            let err = io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to deserialize ident: {}", e),
            );
            tracing::error!(error = ?err, "Failed to deserialize ident.json");
            err
        })?;

        let interface_string = fs::read(path.join("interface.json")).await?;
        let interface = serde_json::from_slice(&interface_string).map_err(|e| {
            let err = io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to deserialize interface: {}", e),
            );
            tracing::error!(error = ?err, "Failed to deserialize interface.json");
            err
        })?;

        Ok(CapabilityBinary {
            libs,
            ident,
            interface,
        })
    }
}

impl Artifact for CapabilitySource {
    #[tracing::instrument(skip(self, path), fields(path = %path.display(), manifest = ?self.manifest.capability))]
    async fn write_to_directory(&self, path: &Path) -> io::Result<()> {
        tracing::debug!("Writing CapabilitySource to directory");
        fs::create_dir_all(path).await?;
        let manifest = toml::to_string_pretty(&self.manifest).map_err(|e| {
            let err = io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to serialize manifest: {}", e),
            );
            tracing::error!(error = ?err, "Failed to serialize Capability.toml");
            err
        })?;
        fs::write(path.join("Capability.toml"), &manifest).await?;
        fs::write(path.join("Cargo.toml"), &self.cargo_toml).await?;
        fs::write(path.join("Cargo.lock"), &self.cargo_lock).await?;

        let src_dir = path.join("src");
        fs::create_dir_all(&src_dir).await?;
        fs::write(src_dir.join("lib.rs"), &self.src_lib_rs).await?;
        Ok(())
    }

    fn to_tarball(&self) -> Result<Vec<u8>, io::Error> {
        let encoder = GzEncoder::new(Vec::new(), Compression::default());
        let mut tar = Builder::new(encoder);

        let manifest = toml::to_string_pretty(&self.manifest).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to serialize manifest: {}", e),
            )
        })?;
        append_file(&mut tar, "Capability.toml", manifest.as_bytes())?;
        append_file(&mut tar, "Cargo.toml", self.cargo_toml.as_bytes())?;
        append_file(&mut tar, "Cargo.lock", self.cargo_lock.as_bytes())?;
        append_file(&mut tar, "src/lib.rs", self.src_lib_rs.as_bytes())?;

        tar.into_inner()?.finish()
    }

    fn from_tarball(bytes: &[u8]) -> Result<Self, io::Error> {
        let tar = GzDecoder::new(bytes);
        let mut archive = tar::Archive::new(tar);

        let mut manifest = None;
        let mut cargo_toml = None;
        let mut cargo_lock = None;
        let mut src_lib_rs = None;

        for file in archive.entries()? {
            let mut file = file?;
            let path = file.path()?.to_path_buf();
            let mut content = Vec::new();
            file.read_to_end(&mut content)?;

            match path.to_string_lossy().as_ref() {
                "Capability.toml" => {
                    manifest = toml::from_slice(&content).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Unable to deserialize manifest: {}", e),
                        )
                    })?;
                }
                "Cargo.toml" => cargo_toml = String::from_utf8(content).ok(),
                "Cargo.lock" => cargo_lock = String::from_utf8(content).ok(),
                "src/lib.rs" => src_lib_rs = String::from_utf8(content).ok(),
                _ => {}
            }
        }

        Ok(CapabilitySource {
            manifest: manifest.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "Missing Capability.toml")
            })?,
            cargo_toml: cargo_toml
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing Cargo.toml"))?,
            cargo_lock: cargo_lock
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing Cargo.lock"))?,
            src_lib_rs: src_lib_rs
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing src/lib.rs"))?,
        })
    }

    #[tracing::instrument(skip(path), fields(path = %path.display()))]
    async fn from_dir(path: &Path) -> Result<Self, io::Error> {
        tracing::debug!("Loading CapabilitySource from directory");
        let manifest_string = fs::read(path.join("Capability.toml")).await?;
        let manifest = toml::from_slice(&manifest_string).map_err(|e| {
            let err = io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to deserialize manifest: {}", e),
            );
            tracing::error!(error = ?err, "Failed to deserialize Capability.toml");
            err
        })?;

        Ok(CapabilitySource {
            manifest,
            cargo_toml: fs::read_to_string(path.join("Cargo.toml")).await?,
            cargo_lock: fs::read_to_string(path.join("Cargo.lock")).await?,
            src_lib_rs: fs::read_to_string(path.join("src").join("lib.rs")).await?,
        })
    }
}

impl Artifact for Interface {
    #[tracing::instrument(skip(self, path), fields(path = %path.display(), manifest = ?self.manifest.capability))]
    async fn write_to_directory(&self, path: &Path) -> io::Result<()> {
        tracing::debug!("Writing Interface to directory");
        let manifest = toml::to_string_pretty(&self.manifest).map_err(|e| {
            let err = io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to serialize manifest: {}", e),
            );
            tracing::error!(error = ?err, "Failed to serialize Capability.toml");
            err
        })?;
        fs::create_dir_all(path).await?;
        fs::write(path.join("Capability.toml"), &manifest).await?;
        let interface_str = serde_json::to_string_pretty(&self.interface).map_err(|e| {
            let err = io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to serialize interface: {}", e),
            );
            tracing::error!(error = ?err, "Failed to serialize interface.json");
            err
        })?;
        fs::write(path.join("interface.json"), &interface_str).await?;

        let src_dir = path.join("src");
        fs::create_dir_all(&src_dir).await?;
        fs::write(src_dir.join("lib.rs"), &self.src_lib_rs).await?;
        Ok(())
    }

    fn to_tarball(&self) -> Result<Vec<u8>, io::Error> {
        let encoder = GzEncoder::new(Vec::new(), Compression::default());
        let mut tar = Builder::new(encoder);
        let manifest = toml::to_string_pretty(&self.manifest).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to serialize manifest: {}", e),
            )
        })?;
        let interface = serde_json::to_string_pretty(&self.interface).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to serialize interface: {}", e),
            )
        })?;

        append_file(&mut tar, "Capability.toml", manifest.as_bytes())?;
        append_file(&mut tar, "interface.json", interface.as_bytes())?;
        append_file(&mut tar, "src/lib.rs", self.src_lib_rs.as_bytes())?;

        tar.into_inner()?.finish()
    }

    fn from_tarball(bytes: &[u8]) -> Result<Self, io::Error> {
        let tar = GzDecoder::new(bytes);
        let mut archive = tar::Archive::new(tar);

        let mut manifest = None;
        let mut src_lib_rs = None;
        let mut interface = None;

        for file in archive.entries()? {
            let mut file = file?;
            let path = file.path()?.to_path_buf();
            let mut content = Vec::new();
            file.read_to_end(&mut content)?;

            match path.to_string_lossy().as_ref() {
                "Capability.toml" => {
                    manifest = toml::from_slice(&content).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Unable to deserialize manifest: {}", e),
                        )
                    })?;
                }
                "interface.json" => {
                    interface = serde_json::from_slice(&content).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Unable to deserialize interface: {}", e),
                        )
                    })?;
                }
                "src/lib.rs" => src_lib_rs = String::from_utf8(content).ok(),
                _ => {}
            }
        }

        Ok(Interface {
            manifest: manifest.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "Missing Capability.toml")
            })?,
            src_lib_rs: src_lib_rs
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing src/lib.rs"))?,
            interface: interface.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "Missing interface.json")
            })?,
        })
    }

    #[tracing::instrument(skip(path), fields(path = %path.display()))]
    async fn from_dir(path: &Path) -> Result<Self, io::Error> {
        tracing::debug!("Loading Interface from directory");
        let manifest_string = fs::read(path.join("Capability.toml")).await?;
        let manifest = toml::from_slice(&manifest_string).map_err(|e| {
            let err = io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to deserialize manifest: {}", e),
            );
            tracing::error!(error = ?err, "Failed to deserialize Capability.toml");
            err
        })?;

        let interface_string = fs::read(path.join("interface.json")).await?;
        let interface = toml::from_slice(&interface_string).map_err(|e| {
            let err = io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to deserialize interface: {}", e),
            );
            tracing::error!(error = ?err, "Failed to deserialize interface.json");
            err
        })?;

        Ok(Interface {
            manifest,
            src_lib_rs: fs::read_to_string(path.join("src").join("lib.rs")).await?,
            interface,
        })
    }
}

impl Artifact for PlaybookSource {
    #[tracing::instrument(skip(self, path), fields(path = %path.display(), ident = ?self.ident))]
    async fn write_to_directory(&self, path: &Path) -> io::Result<()> {
        tracing::debug!("Writing PlaybookSource to directory");
        fs::create_dir_all(path).await?;
        let dependencies = serde_json::to_string_pretty(&self.dependencies).map_err(|e| {
            let err = io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to serialize dependencies: {}", e),
            );
            tracing::error!(error = ?err, "Failed to serialize dependencies.json");
            err
        })?;
        let configurations = serde_json::to_string_pretty(&self.configurations).map_err(|e| {
            let err = io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to serialize configurations: {}", e),
            );
            tracing::error!(error = ?err, "Failed to serialize configurations.json");
            err
        })?;
        fs::write(path.join("source.rs"), &self.source).await?;
        fs::write(path.join("dependencies.json"), &dependencies).await?;
        fs::write(path.join("configurations.json"), &configurations).await?;
        Ok(())
    }

    fn to_tarball(&self) -> Result<Vec<u8>, io::Error> {
        let encoder = GzEncoder::new(Vec::new(), Compression::default());
        let mut tar = Builder::new(encoder);
        let dependencies = serde_json::to_string_pretty(&self.dependencies).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to serialize dependencies: {}", e),
            )
        })?;
        let configurations = serde_json::to_string_pretty(&self.configurations).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to serialize configurations: {}", e),
            )
        })?;
        append_file(&mut tar, "source.rs", self.source.as_bytes())?;
        append_file(&mut tar, "dependencies.json", dependencies.as_bytes())?;
        append_file(&mut tar, "configurations.json", configurations.as_bytes())?;
        tar.into_inner()?.finish()
    }

    fn from_tarball(bytes: &[u8]) -> Result<Self, io::Error> {
        let tar = GzDecoder::new(bytes);
        let mut archive = tar::Archive::new(tar);
        let mut source = None;
        let mut dependencies = None;
        let mut configurations = Vec::new();

        for file in archive.entries()? {
            let mut file = file?;
            let path = file.path()?.to_path_buf();
            let mut content = Vec::new();
            file.read_to_end(&mut content)?;

            match path.to_string_lossy().as_ref() {
                "source.rs" => source = String::from_utf8(content).ok(),
                "dependencies.json" => {
                    dependencies = serde_json::from_slice(&content).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Unable to deserialize dependencies: {}", e),
                        )
                    })?;
                }
                "configurations.json" => {
                    configurations = serde_json::from_slice(&content).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Unable to deserialize configurations: {}", e),
                        )
                    })?;
                }
                _ => {}
            }
        }

        Ok(PlaybookSource {
            ident: None,
            source: source
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Missing source.rs"))?,
            dependencies: dependencies.ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "Missing dependencies.json")
            })?,
            configurations,
        })
    }

    #[tracing::instrument(skip(path), fields(path = %path.display()))]
    async fn from_dir(path: &Path) -> Result<Self, io::Error> {
        tracing::debug!("Loading PlaybookSource from directory");
        let dependencies_string = fs::read(path.join("dependencies.json")).await?;
        let dependencies = serde_json::from_slice(&dependencies_string).map_err(|e| {
            let err = io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to deserialize dependencies: {}", e),
            );
            tracing::error!(error = ?err, "Failed to deserialize dependencies.json");
            err
        })?;
        let configurations = if fs::try_exists(path.join("configurations.json"))
            .await
            .unwrap_or(false)
        {
            let configurations_string = fs::read(path.join("configurations.json")).await?;
            serde_json::from_slice(&configurations_string).map_err(|e| {
                let err = io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Unable to deserialize configurations: {}", e),
                );
                tracing::error!(error = ?err, "Failed to deserialize configurations.json");
                err
            })?
        } else {
            Vec::new()
        };
        Ok(PlaybookSource {
            ident: None,
            source: fs::read_to_string(path.join("source.rs")).await?,
            dependencies,
            configurations,
        })
    }
}

impl Artifact for PlaybookBinary {
    #[tracing::instrument(skip(self, path), fields(path = %path.display(), ident = ?self.ident))]
    async fn write_to_directory(&self, path: &Path) -> io::Result<()> {
        tracing::debug!("Writing PlaybookBinary to directory");
        fs::create_dir_all(path).await?;
        fs::write(path.join("mod.wasm"), &self.wasm).await?;
        let spec = serde_json::to_string_pretty(&self.spec).map_err(|e| {
            let err = io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to serialize spec: {}", e),
            );
            tracing::error!(error = ?err, "Failed to serialize spec.json");
            err
        })?;
        fs::write(path.join("spec.json"), spec).await?;
        let configurations = serde_json::to_string_pretty(&self.configurations).map_err(|e| {
            let err = io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to serialize configurations: {}", e),
            );
            tracing::error!(error = ?err, "Failed to serialize configurations.json");
            err
        })?;
        fs::write(path.join("configurations.json"), &configurations).await?;
        Ok(())
    }

    fn to_tarball(&self) -> Result<Vec<u8>, io::Error> {
        let encoder = GzEncoder::new(Vec::new(), Compression::default());
        let mut tar = Builder::new(encoder);
        append_file(&mut tar, "mod.wasm", &self.wasm)?;
        let spec = serde_json::to_string_pretty(&self.spec).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to serialize spec: {}", e),
            )
        })?;
        append_file(&mut tar, "spec.json", spec.as_bytes())?;
        let configurations = serde_json::to_string_pretty(&self.configurations).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to serialize configurations: {}", e),
            )
        })?;
        append_file(&mut tar, "configurations.json", configurations.as_bytes())?;
        tar.into_inner()?.finish()
    }

    fn from_tarball(bytes: &[u8]) -> Result<Self, io::Error> {
        let tar = GzDecoder::new(bytes);
        let mut archive = tar::Archive::new(tar);
        let mut wasm = None;
        let mut spec = None;
        let mut configurations = Vec::new();

        for file in archive.entries()? {
            let mut file = file?;
            let path = file.path()?.to_path_buf();
            let mut content = Vec::new();
            file.read_to_end(&mut content)?;

            match path.to_string_lossy().as_ref() {
                "mod.wasm" => wasm = Some(content),
                "spec.json" => {
                    spec = serde_json::from_slice(&content).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Unable to deserialize spec: {}", e),
                        )
                    })?;
                }
                "configurations.json" => {
                    configurations = serde_json::from_slice(&content).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Unable to deserialize configurations: {}", e),
                        )
                    })?;
                }
                _ => {}
            }
        }

        Ok(PlaybookBinary {
            ident: None,
            wasm: wasm
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Missing mod.wasm"))?,
            spec: spec
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Missing spec.json"))?,
            configurations,
        })
    }

    #[tracing::instrument(skip(path), fields(path = %path.display()))]
    async fn from_dir(path: &Path) -> Result<Self, io::Error> {
        tracing::debug!("Loading PlaybookBinary from directory");
        let spec_string = fs::read(path.join("spec.json")).await?;
        let spec = serde_json::from_slice(&spec_string).map_err(|e| {
            let err = io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to deserialize spec: {}", e),
            );
            tracing::error!(error = ?err, "Failed to deserialize spec.json");
            err
        })?;
        let configurations = if fs::try_exists(path.join("configurations.json"))
            .await
            .unwrap_or(false)
        {
            let configurations_string = fs::read(path.join("configurations.json")).await?;
            serde_json::from_slice(&configurations_string).map_err(|e| {
                let err = io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Unable to deserialize configurations: {}", e),
                );
                tracing::error!(error = ?err, "Failed to deserialize configurations.json");
                err
            })?
        } else {
            Vec::new()
        };
        Ok(PlaybookBinary {
            ident: None,
            wasm: fs::read(path.join("mod.wasm")).await?,
            spec,
            configurations,
        })
    }
}

impl Artifact for Playbook {
    #[tracing::instrument(skip(self, path), fields(path = %path.display()))]
    async fn write_to_directory(&self, path: &Path) -> io::Result<()> {
        tracing::debug!("Writing Playbook to directory");
        match self {
            Playbook::Source(module_source) => module_source.write_to_directory(path).await,
            Playbook::Binary(module_binary) => module_binary.write_to_directory(path).await,
        }
    }

    fn to_tarball(&self) -> Result<Vec<u8>, io::Error> {
        match self {
            Playbook::Source(module_source) => module_source.to_tarball(),
            Playbook::Binary(module_binary) => module_binary.to_tarball(),
        }
    }

    fn from_tarball(bytes: &[u8]) -> Result<Self, io::Error> {
        // Peek at the filenames inside the tarball to determine if it's source or binary.
        let tar = GzDecoder::new(bytes);
        let mut archive = tar::Archive::new(tar);

        let mut has_source_rs = false;
        let mut has_wasm = false;

        for file in archive.entries()? {
            let file = file?;
            let path_str = file.path()?.to_string_lossy().into_owned();

            match path_str.as_ref() {
                "source.rs" => has_source_rs = true,
                "mod.wasm" => has_wasm = true,
                _ => {}
            }
        }

        if has_source_rs {
            Ok(Playbook::Source(PlaybookSource::from_tarball(bytes)?))
        } else if has_wasm {
            Ok(Playbook::Binary(PlaybookBinary::from_tarball(bytes)?))
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unknown module format in tarball: missing 'source.rs' or 'mod.wasm'",
            ))
        }
    }

    #[tracing::instrument(skip(path), fields(path = %path.display()))]
    async fn from_dir(path: &Path) -> Result<Self, io::Error> {
        tracing::debug!("Loading Playbook from directory");
        if fs::try_exists(path.join("source.rs"))
            .await
            .unwrap_or(false)
        {
            Ok(Playbook::Source(PlaybookSource::from_dir(path).await?))
        } else if fs::try_exists(path.join("mod.wasm")).await.unwrap_or(false) {
            Ok(Playbook::Binary(PlaybookBinary::from_dir(path).await?))
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unknown module format in directory: missing 'source.rs' or 'mod.wasm'",
            ))
        }
    }
}

impl Artifact for Artifacts {
    #[tracing::instrument(skip(self, path), fields(path = %path.display()))]
    async fn write_to_directory(&self, path: &Path) -> io::Result<()> {
        tracing::debug!("Writing Artifacts to directory");
        match self {
            Artifacts::CapabilityBinary(c) => c.write_to_directory(path).await,
            Artifacts::CapabilitySource(c) => c.write_to_directory(path).await,
            Artifacts::Interface(i) => i.write_to_directory(path).await,
            Artifacts::Playbook(m) => m.write_to_directory(path).await,
        }
    }

    fn to_tarball(&self) -> Result<Vec<u8>, io::Error> {
        match self {
            Artifacts::CapabilityBinary(c) => c.to_tarball(),
            Artifacts::CapabilitySource(c) => c.to_tarball(),
            Artifacts::Interface(i) => i.to_tarball(),
            Artifacts::Playbook(m) => m.to_tarball(),
        }
    }

    fn from_tarball(bytes: &[u8]) -> Result<Self, io::Error> {
        // Peek at the filenames inside the tarball to determine what artifact this is.
        let tar = GzDecoder::new(bytes);
        let mut archive = tar::Archive::new(tar);

        let mut has_source_rs = false;
        let mut has_wasm = false;
        let mut has_cap_toml = false;
        let mut has_lib = false;

        for file in archive.entries()? {
            let file = file?;
            let path_str = file.path()?.to_string_lossy().into_owned();

            match path_str.as_ref() {
                "source.rs" => has_source_rs = true,
                "mod.wasm" => has_wasm = true,
                "Capability.toml" => has_cap_toml = true,
                "lib.dll" | "lib.dylib" | "lib.so" => has_lib = true,
                _ => {}
            }
        }

        if has_source_rs || has_wasm {
            // Let the Playbook implementation handle the Source vs Binary split
            Ok(Artifacts::Playbook(Playbook::from_tarball(bytes)?))
        } else if has_cap_toml {
            if has_lib {
                // This shouldn't happen based on the split, but for backward compatibility or weird cases
                Ok(Artifacts::CapabilitySource(CapabilitySource::from_tarball(
                    bytes,
                )?))
            } else {
                Ok(Artifacts::Interface(Interface::from_tarball(bytes)?))
            }
        } else if has_lib {
            Ok(Artifacts::CapabilityBinary(CapabilityBinary::from_tarball(
                bytes,
            )?))
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unknown artifact format in tarball",
            ))
        }
    }

    #[tracing::instrument(skip(path), fields(path = %path.display()))]
    async fn from_dir(path: &Path) -> Result<Self, io::Error> {
        tracing::debug!("Loading Artifacts from directory");
        let has_source_rs = fs::try_exists(path.join("source.rs"))
            .await
            .unwrap_or(false);
        let has_wasm = fs::try_exists(path.join("mod.wasm")).await.unwrap_or(false);

        if has_source_rs || has_wasm {
            // Let the Playbook implementation handle the Source vs Binary split
            Ok(Artifacts::Playbook(Playbook::from_dir(path).await?))
        } else if fs::try_exists(path.join("Capability.toml"))
            .await
            .unwrap_or(false)
        {
            let has_dll = fs::try_exists(path.join("lib.dll")).await.unwrap_or(false);
            let has_dylib = fs::try_exists(path.join("lib.dylib"))
                .await
                .unwrap_or(false);
            let has_so = fs::try_exists(path.join("lib.so")).await.unwrap_or(false);

            if has_dll || has_dylib || has_so {
                // This shouldn't happen based on the split, but for backward compatibility
                Ok(Artifacts::CapabilitySource(
                    CapabilitySource::from_dir(path).await?,
                ))
            } else {
                Ok(Artifacts::Interface(Interface::from_dir(path).await?))
            }
        } else if fs::try_exists(path.join("lib.dll")).await.unwrap_or(false)
            || fs::try_exists(path.join("lib.dylib"))
                .await
                .unwrap_or(false)
            || fs::try_exists(path.join("lib.so")).await.unwrap_or(false)
        {
            Ok(Artifacts::CapabilityBinary(
                CapabilityBinary::from_dir(path).await?,
            ))
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unknown artifact format in directory",
            ))
        }
    }
}
