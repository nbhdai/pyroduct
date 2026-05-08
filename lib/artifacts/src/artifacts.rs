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

use crate::cargo::{CapabilityIdent, CapabilityManifest, ResolvedCapability};

pub enum CapBinary {
    Pe(Vec<u8>),
    MachO(Vec<u8>),
    Elf(Vec<u8>),
}

impl Deref for CapBinary {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        match self {
            CapBinary::Pe(items) => &*items,
            CapBinary::MachO(items) => &*items,
            CapBinary::Elf(items) => &*items,
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

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct ModuleSource {
    pub dependencies: ModuleDependencies,
    pub source: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct ModuleSpec {
    pub func: ModuleFunc<'static>,
    pub capabilities: Vec<ResolvedCapability>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct ModuleBinary {
    pub hash: String,
    pub wasm: Vec<u8>,
    pub spec: ModuleSpec,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub enum Module {
    Source(ModuleSource),
    Binary(ModuleBinary),
}

/// A single wasm module in the pipeline intent.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct Playbook {
    pub hash: String,
    /// Per-class capability configuration. Keys are class names.
    #[serde(default)]
    pub configurations: HashMap<String, Option<serde_json::Value>>,
}

impl ModuleSource {
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

impl ModuleBinary {
    pub fn hash(&self) -> String {
        self.hash.clone()
    }
}

impl Module {
    pub fn hash(&self) -> String {
        match self {
            Module::Source(m) => m.hash(),
            Module::Binary(m) => m.hash(),
        }
    }
}

pub enum Artifacts {
    CapabilityBinary(CapabilityBinary),
    CapabilitySource(CapabilitySource),
    Interface(Interface),
    Module(Module),
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

impl From<Module> for Artifacts {
    fn from(value: Module) -> Self {
        Artifacts::Module(value)
    }
}

impl From<ModuleBinary> for Artifacts {
    fn from(value: ModuleBinary) -> Self {
        Artifacts::Module(Module::Binary(value))
    }
}

impl From<ModuleSource> for Artifacts {
    fn from(value: ModuleSource) -> Self {
        Artifacts::Module(Module::Source(value))
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
    async fn write_to_directory(&self, path: &Path) -> io::Result<()> {
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
            serde_json::to_string(&self.ident)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
        )
        .await?;
        let interface_json = serde_json::to_string_pretty(&self.interface)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
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

    async fn from_dir(path: &Path) -> Result<Self, io::Error> {
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
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Missing capability library",
            ));
        }

        let ident_string = fs::read(path.join("ident.json")).await?;
        let ident = serde_json::from_slice(&ident_string).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to deserialize ident: {}", e),
            )
        })?;

        let interface_string = fs::read(path.join("interface.json")).await?;
        let interface = serde_json::from_slice(&interface_string).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to deserialize interface: {}", e),
            )
        })?;

        Ok(CapabilityBinary { libs, ident, interface })
    }
}

impl Artifact for CapabilitySource {
    async fn write_to_directory(&self, path: &Path) -> io::Result<()> {
        fs::create_dir_all(path).await?;
        let manifest = toml::to_string_pretty(&self.manifest).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to serialize manifest: {}", e),
            )
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

    async fn from_dir(path: &Path) -> Result<Self, io::Error> {
        let manifest_string = fs::read(path.join("Capability.toml")).await?;
        let manifest = toml::from_slice(&manifest_string).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to deserialize manifest: {}", e),
            )
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
    async fn write_to_directory(&self, path: &Path) -> io::Result<()> {
        let manifest = toml::to_string_pretty(&self.manifest).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to serialize manifest: {}", e),
            )
        })?;
        fs::create_dir_all(path).await?;
        fs::write(path.join("Capability.toml"), &manifest).await?;
        let interface_str = serde_json::to_string_pretty(&self.interface).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to serialize manifest: {}", e),
            )
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

    async fn from_dir(path: &Path) -> Result<Self, io::Error> {
        let manifest_string = fs::read(path.join("Capability.toml")).await?;
        let manifest = toml::from_slice(&manifest_string).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to deserialize manifest: {}", e),
            )
        })?;

        let interface_string = fs::read(path.join("interface.json")).await?;
        let interface = toml::from_slice(&interface_string).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to deserialize interface: {}", e),
            )
        })?;

        Ok(Interface {
            manifest,
            src_lib_rs: fs::read_to_string(path.join("src").join("lib.rs")).await?,
            interface,
        })
    }
}

impl Artifact for ModuleSource {
    async fn write_to_directory(&self, path: &Path) -> io::Result<()> {
        fs::create_dir_all(path).await?;
        let dependencies = serde_json::to_string_pretty(&self.dependencies).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to serialize dependencies: {}", e),
            )
        })?;
        fs::write(path.join("source.rs"), &self.source).await?;
        fs::write(path.join("dependencies.json"), &dependencies).await?;
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
        append_file(&mut tar, "source.rs", self.source.as_bytes())?;
        append_file(&mut tar, "dependencies.json", dependencies.as_bytes())?;
        tar.into_inner()?.finish()
    }

    fn from_tarball(bytes: &[u8]) -> Result<Self, io::Error> {
        let tar = GzDecoder::new(bytes);
        let mut archive = tar::Archive::new(tar);
        let mut source = None;
        let mut dependencies = None;

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
                _ => {}
            }
        }

        Ok(ModuleSource {
            source: source
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Missing source.rs"))?,
            dependencies: dependencies.ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "Missing dependencies.json")
            })?,
        })
    }

    async fn from_dir(path: &Path) -> Result<Self, io::Error> {
        let dependencies_string = fs::read(path.join("dependencies.json")).await?;
        let dependencies = serde_json::from_slice(&dependencies_string).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to deserialize dependencies: {}", e),
            )
        })?;
        Ok(ModuleSource {
            source: fs::read_to_string(path.join("source.rs")).await?,
            dependencies,
        })
    }
}

impl Artifact for ModuleBinary {
    async fn write_to_directory(&self, path: &Path) -> io::Result<()> {
        fs::create_dir_all(path).await?;
        fs::write(path.join("hash.txt"), self.hash.as_bytes()).await?;
        fs::write(path.join("mod.wasm"), &self.wasm).await?;
        let spec = serde_json::to_string_pretty(&self.spec).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to serialize spec: {}", e),
            )
        })?;
        fs::write(path.join("spec.json"), spec).await?;
        Ok(())
    }

    fn to_tarball(&self) -> Result<Vec<u8>, io::Error> {
        let encoder = GzEncoder::new(Vec::new(), Compression::default());
        let mut tar = Builder::new(encoder);
        append_file(&mut tar, "hash.txt", self.hash.as_bytes())?;
        append_file(&mut tar, "mod.wasm", &self.wasm)?;
        let spec = serde_json::to_string_pretty(&self.spec).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to serialize spec: {}", e),
            )
        })?;
        append_file(&mut tar, "spec.json", spec.as_bytes())?;
        tar.into_inner()?.finish()
    }

    fn from_tarball(bytes: &[u8]) -> Result<Self, io::Error> {
        let tar = GzDecoder::new(bytes);
        let mut archive = tar::Archive::new(tar);
        let mut wasm = None;
        let mut spec = None;
        let mut hash = None;

        for file in archive.entries()? {
            let mut file = file?;
            let path = file.path()?.to_path_buf();
            let mut content = Vec::new();
            file.read_to_end(&mut content)?;

            match path.to_string_lossy().as_ref() {
                "hash.txt" => {
                    hash = Some(String::from_utf8(content).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Unable to deserialize hash: {}", e),
                        )
                    })?)
                }
                "mod.wasm" => wasm = Some(content),
                "spec.json" => {
                    spec = serde_json::from_slice(&content).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Unable to deserialize spec: {}", e),
                        )
                    })?;
                }
                _ => {}
            }
        }

        Ok(ModuleBinary {
            hash: hash
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Missing hash.txt"))?,
            wasm: wasm
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Missing mod.wasm"))?,
            spec: spec
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Missing spec.json"))?,
        })
    }

    async fn from_dir(path: &Path) -> Result<Self, io::Error> {
        let spec_string = fs::read(path.join("spec.json")).await?;
        let spec = serde_json::from_slice(&spec_string).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to deserialize spec: {}", e),
            )
        })?;
        Ok(ModuleBinary {
            hash: fs::read_to_string(path.join("hash.txt")).await?,
            wasm: fs::read(path.join("mod.wasm")).await?,
            spec,
        })
    }
}

impl Artifact for Module {
    async fn write_to_directory(&self, path: &Path) -> io::Result<()> {
        match self {
            Module::Source(module_source) => module_source.write_to_directory(path).await,
            Module::Binary(module_binary) => module_binary.write_to_directory(path).await,
        }
    }

    fn to_tarball(&self) -> Result<Vec<u8>, io::Error> {
        match self {
            Module::Source(module_source) => module_source.to_tarball(),
            Module::Binary(module_binary) => module_binary.to_tarball(),
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
            Ok(Module::Source(ModuleSource::from_tarball(bytes)?))
        } else if has_wasm {
            Ok(Module::Binary(ModuleBinary::from_tarball(bytes)?))
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unknown module format in tarball: missing 'source.rs' or 'mod.wasm'",
            ))
        }
    }

    async fn from_dir(path: &Path) -> Result<Self, io::Error> {
        if fs::try_exists(path.join("source.rs"))
            .await
            .unwrap_or(false)
        {
            Ok(Module::Source(ModuleSource::from_dir(path).await?))
        } else if fs::try_exists(path.join("mod.wasm")).await.unwrap_or(false) {
            Ok(Module::Binary(ModuleBinary::from_dir(path).await?))
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unknown module format in directory: missing 'source.rs' or 'mod.wasm'",
            ))
        }
    }
}

impl Artifact for Artifacts {
    async fn write_to_directory(&self, path: &Path) -> io::Result<()> {
        match self {
            Artifacts::CapabilityBinary(c) => c.write_to_directory(path).await,
            Artifacts::CapabilitySource(c) => c.write_to_directory(path).await,
            Artifacts::Interface(i) => i.write_to_directory(path).await,
            Artifacts::Module(m) => m.write_to_directory(path).await,
        }
    }

    fn to_tarball(&self) -> Result<Vec<u8>, io::Error> {
        match self {
            Artifacts::CapabilityBinary(c) => c.to_tarball(),
            Artifacts::CapabilitySource(c) => c.to_tarball(),
            Artifacts::Interface(i) => i.to_tarball(),
            Artifacts::Module(m) => m.to_tarball(),
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
            // Let the Module implementation handle the Source vs Binary split
            Ok(Artifacts::Module(Module::from_tarball(bytes)?))
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

    async fn from_dir(path: &Path) -> Result<Self, io::Error> {
        let has_source_rs = fs::try_exists(path.join("source.rs"))
            .await
            .unwrap_or(false);
        let has_wasm = fs::try_exists(path.join("mod.wasm")).await.unwrap_or(false);

        if has_source_rs || has_wasm {
            // Let the Module implementation handle the Source vs Binary split
            Ok(Artifacts::Module(Module::from_dir(path).await?))
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
