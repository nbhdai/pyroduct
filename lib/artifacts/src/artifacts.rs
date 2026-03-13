use cargo_toml::Dependency;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use pyroduct::format::value::ModuleFunc;
use std::collections::BTreeMap;
use std::future::Future;
use std::io::{self, Read, Write};
use std::path::Path;
use tar::{Builder, Header};
use tokio::fs;

use crate::cargo::CapabilityManifest;
use crate::environment::ResolvedCapability;

pub enum CapBinary {
    Pe(Vec<u8>),
    MachO(Vec<u8>),
    Elf(Vec<u8>),
}

pub struct Capability {
    pub libs: Vec<CapBinary>,
    pub manifest: String,
    pub cargo_toml: String,
    pub cargo_lock: String,
    pub src_lib_rs: String,
    pub interface_json: String,
    pub config_json: Option<String>,
}

pub struct Interface {
    pub manifest: CapabilityManifest,
    pub src_lib_rs: String,
    pub interface_json: String,
    pub config_json: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ModuleDependencies {
    pub dependencies: BTreeMap<String, Dependency>,
    pub capabilities: Vec<ResolvedCapability>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct AnonModule {
    pub dependencies: ModuleDependencies,
    pub source: String,
    pub wasm: Vec<u8>,
    pub spec: ModuleFunc<'static>,
}

pub enum Artifacts {
    Capability(Capability),
    Interface(Interface),
    AnonModule(AnonModule),
}

impl From<Capability> for Artifacts {
    fn from(value: Capability) -> Self {
        Artifacts::Capability(value)
    }
}

impl From<Interface> for Artifacts {
    fn from(value: Interface) -> Self {
        Artifacts::Interface(value)
    }
}

impl From<AnonModule> for Artifacts {
    fn from(value: AnonModule) -> Self {
        Artifacts::AnonModule(value)
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
fn append_file<W: Write>(tar: &mut Builder<W>, name: &str, data: &[u8]) -> Result<(), io::Error> {
    let mut header = Header::new_gnu();
    header.set_size(data.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(&mut header, name, data)
}

// ==========================================
// Trait Implementations for Concrete Structs
// ==========================================

impl Artifact for Capability {
    async fn write_to_directory(&self, path: &Path) -> io::Result<()> {
        fs::create_dir_all(path).await?;
        for lib in &self.libs {
            match lib {
                CapBinary::Pe(bytes) => fs::write(path.join("lib.dll"), bytes).await?,
                CapBinary::MachO(bytes) => fs::write(path.join("lib.dylib"), bytes).await?,
                CapBinary::Elf(bytes) => fs::write(path.join("lib.so"), bytes).await?,
            }
        }
        fs::write(path.join("Capability.toml"), &self.manifest).await?;
        fs::write(path.join("Cargo.toml"), &self.cargo_toml).await?;
        fs::write(path.join("Cargo.lock"), &self.cargo_lock).await?;
        fs::write(path.join("interface.json"), &self.interface_json).await?;

        if let Some(config) = &self.config_json {
            fs::write(path.join("config.json"), config).await?;
        }

        let src_dir = path.join("src");
        fs::create_dir_all(&src_dir).await?;
        fs::write(src_dir.join("lib.rs"), &self.src_lib_rs).await?;
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
        append_file(&mut tar, "Capability.toml", self.manifest.as_bytes())?;
        append_file(&mut tar, "Cargo.toml", self.cargo_toml.as_bytes())?;
        append_file(&mut tar, "Cargo.lock", self.cargo_lock.as_bytes())?;
        append_file(&mut tar, "src/lib.rs", self.src_lib_rs.as_bytes())?;
        append_file(&mut tar, "interface.json", self.interface_json.as_bytes())?;
        if let Some(config) = &self.config_json {
            append_file(&mut tar, "config.json", config.as_bytes())?;
        }

        tar.into_inner()?.finish()
    }

    fn from_tarball(bytes: &[u8]) -> Result<Self, io::Error> {
        let tar = GzDecoder::new(bytes);
        let mut archive = tar::Archive::new(tar);

        let mut manifest = None;
        let mut libs = Vec::new();
        let mut cargo_toml = None;
        let mut cargo_lock = None;
        let mut src_lib_rs = None;
        let mut interface_json = None;
        let mut config_json = None;

        for file in archive.entries()? {
            let mut file = file?;
            let path = file.path()?.to_path_buf();
            let mut content = Vec::new();
            file.read_to_end(&mut content)?;

            match path.to_string_lossy().as_ref() {
                "Capability.toml" => manifest = String::from_utf8(content).ok(),
                "lib.dll" => libs.push(CapBinary::Pe(content)),
                "lib.dylib" => libs.push(CapBinary::MachO(content)),
                "lib.so" => libs.push(CapBinary::Elf(content)),
                "Cargo.toml" => cargo_toml = String::from_utf8(content).ok(),
                "Cargo.lock" => cargo_lock = String::from_utf8(content).ok(),
                "src/lib.rs" => src_lib_rs = String::from_utf8(content).ok(),
                "interface.json" => interface_json = String::from_utf8(content).ok(),
                "config.json" => config_json = String::from_utf8(content).ok(),
                _ => {}
            }
        }

        if libs.is_empty() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "Missing library"));
        }

        Ok(Capability {
            manifest: manifest.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "Missing Capability.toml")
            })?,
            libs,
            cargo_toml: cargo_toml
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing Cargo.toml"))?,
            cargo_lock: cargo_lock
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing Cargo.lock"))?,
            src_lib_rs: src_lib_rs
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing src/lib.rs"))?,
            interface_json: interface_json.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "Missing interface.json")
            })?,
            config_json,
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

        Ok(Capability {
            libs,
            manifest: fs::read_to_string(path.join("Capability.toml")).await?,
            cargo_toml: fs::read_to_string(path.join("Cargo.toml")).await?,
            cargo_lock: fs::read_to_string(path.join("Cargo.lock")).await?,
            src_lib_rs: fs::read_to_string(path.join("src").join("lib.rs")).await?,
            interface_json: fs::read_to_string(path.join("interface.json")).await?,
            config_json: fs::read_to_string(path.join("config.json")).await.ok(),
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
        fs::write(path.join("interface.json"), &self.interface_json).await?;

        if let Some(config) = &self.config_json {
            fs::write(path.join("config.json"), config).await?;
        }

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
        append_file(&mut tar, "src/lib.rs", self.src_lib_rs.as_bytes())?;
        append_file(&mut tar, "interface.json", self.interface_json.as_bytes())?;
        if let Some(config) = &self.config_json {
            append_file(&mut tar, "config.json", config.as_bytes())?;
        }

        tar.into_inner()?.finish()
    }

    fn from_tarball(bytes: &[u8]) -> Result<Self, io::Error> {
        let tar = GzDecoder::new(bytes);
        let mut archive = tar::Archive::new(tar);

        let mut manifest = None;
        let mut src_lib_rs = None;
        let mut interface_json = None;
        let mut config_json = None;

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
                "src/lib.rs" => src_lib_rs = String::from_utf8(content).ok(),
                "interface.json" => interface_json = String::from_utf8(content).ok(),
                "config.json" => config_json = String::from_utf8(content).ok(),
                _ => {}
            }
        }

        Ok(Interface {
            manifest: manifest.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "Missing Capability.toml")
            })?,
            src_lib_rs: src_lib_rs
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing src/lib.rs"))?,
            interface_json: interface_json.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "Missing interface.json")
            })?,
            config_json,
        })
    }

    async fn from_dir(path: &Path) -> Result<Self, io::Error> {
        let manifest_string = fs::read(path.join("Capability.toml")).await?;
        let manifest = serde_json::from_slice(&manifest_string).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to deserialize manifest: {}", e),
            )
        })?;
        Ok(Interface {
            manifest: manifest,
            src_lib_rs: fs::read_to_string(path.join("src").join("lib.rs")).await?,
            interface_json: fs::read_to_string(path.join("interface.json")).await?,
            config_json: fs::read_to_string(path.join("config.json")).await.ok(),
        })
    }
}

impl Artifact for AnonModule {
    async fn write_to_directory(&self, path: &Path) -> io::Result<()> {
        let spec = serde_json::to_string_pretty(&self.spec).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to serialize spec: {}", e),
            )
        })?;
        fs::create_dir_all(path).await?;

        let dependencies = serde_json::to_string_pretty(&self.dependencies).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to serialize spec: {}", e),
            )
        })?;
        fs::create_dir_all(path).await?;

        fs::write(path.join("source.rs"), &self.source).await?;
        fs::write(path.join("mod.wasm"), &self.wasm).await?;
        fs::write(path.join("spec.json"), &spec).await?;
        fs::write(path.join("dependencies.json"), &dependencies).await?;
        Ok(())
    }

    fn to_tarball(&self) -> Result<Vec<u8>, io::Error> {
        let encoder = GzEncoder::new(Vec::new(), Compression::default());
        let mut tar = Builder::new(encoder);
        let spec = serde_json::to_string_pretty(&self.spec).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to serialize spec: {}", e),
            )
        })?;
        let dependencies = serde_json::to_string_pretty(&self.dependencies).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to serialize spec: {}", e),
            )
        })?;
        append_file(&mut tar, "source.rs", self.source.as_bytes())?;
        append_file(&mut tar, "mod.wasm", &self.wasm)?;
        append_file(&mut tar, "spec.json", spec.as_bytes())?;
        append_file(&mut tar, "dependencies.json", dependencies.as_bytes())?;

        tar.into_inner()?.finish()
    }

    fn from_tarball(bytes: &[u8]) -> Result<Self, io::Error> {
        let tar = GzDecoder::new(bytes);
        let mut archive = tar::Archive::new(tar);

        let mut source = None;
        let mut wasm = None;
        let mut spec = None;
        let mut dependencies = None;

        for file in archive.entries()? {
            let mut file = file?;
            let path = file.path()?.to_path_buf();
            let mut content = Vec::new();
            file.read_to_end(&mut content)?;

            match path.to_string_lossy().as_ref() {
                "source.rs" => source = String::from_utf8(content).ok(),
                "mod.wasm" => wasm = Some(content),
                "spec.json" => {
                    spec = serde_json::from_slice(&content).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Unable to deserialize spec: {}", e),
                        )
                    })?;
                }
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

        Ok(AnonModule {
            source: source
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Missing source.rs"))?,
            wasm: wasm
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Missing mod.wasm"))?,
            spec: spec
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Missing spec.json"))?,
            dependencies: dependencies.ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "Missing dependencies.json")
            })?,
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
        let dependencies_string = fs::read(path.join("dependencies.json")).await?;
        let dependencies = serde_json::from_slice(&dependencies_string).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unable to deserialize dependencies: {}", e),
            )
        })?;
        Ok(AnonModule {
            source: fs::read_to_string(path.join("source.rs")).await?,
            wasm: fs::read(path.join("mod.wasm")).await?,
            spec,
            dependencies,
        })
    }
}

// ==========================================
// Trait Implementation for the Enum (Switch)
// ==========================================

impl Artifact for Artifacts {
    async fn write_to_directory(&self, path: &Path) -> io::Result<()> {
        match self {
            Artifacts::Capability(c) => c.write_to_directory(path).await,
            Artifacts::Interface(i) => i.write_to_directory(path).await,
            Artifacts::AnonModule(a) => a.write_to_directory(path).await,
        }
    }

    fn to_tarball(&self) -> Result<Vec<u8>, io::Error> {
        match self {
            Artifacts::Capability(c) => c.to_tarball(),
            Artifacts::Interface(i) => i.to_tarball(),
            Artifacts::AnonModule(a) => a.to_tarball(),
        }
    }

    fn from_tarball(bytes: &[u8]) -> Result<Self, io::Error> {
        // Peek at the filenames inside the tarball to determine what artifact this is.
        let tar = GzDecoder::new(bytes);
        let mut archive = tar::Archive::new(tar);

        let mut is_anon = false;
        let mut is_cap_or_interface = false;
        let mut has_lib = false;

        for file in archive.entries()? {
            let file = file?;
            let path_str = file.path()?.to_string_lossy().into_owned();

            match path_str.as_ref() {
                "source.rs" => is_anon = true,
                "Capability.toml" => is_cap_or_interface = true,
                "lib.dll" | "lib.dylib" | "lib.so" => has_lib = true,
                _ => {}
            }
        }

        if is_anon {
            Ok(Artifacts::AnonModule(AnonModule::from_tarball(bytes)?))
        } else if is_cap_or_interface {
            if has_lib {
                Ok(Artifacts::Capability(Capability::from_tarball(bytes)?))
            } else {
                Ok(Artifacts::Interface(Interface::from_tarball(bytes)?))
            }
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unknown artifact format in tarball",
            ))
        }
    }

    async fn from_dir(path: &Path) -> Result<Self, io::Error> {
        if fs::try_exists(path.join("source.rs"))
            .await
            .unwrap_or(false)
        {
            Ok(Artifacts::AnonModule(AnonModule::from_dir(path).await?))
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
                Ok(Artifacts::Capability(Capability::from_dir(path).await?))
            } else {
                Ok(Artifacts::Interface(Interface::from_dir(path).await?))
            }
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unknown artifact format in directory",
            ))
        }
    }
}
