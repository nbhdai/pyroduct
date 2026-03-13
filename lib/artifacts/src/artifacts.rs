use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use std::io::{self, Read};
use std::path::Path;
use tar::{Builder, Header};
use tokio::fs;

pub enum CapBinary {
    Pe(Vec<u8>),
    MachO(Vec<u8>),
    Elf(Vec<u8>),
}

pub enum Artifacts {
    Module {
        wasm: Vec<u8>,
        manifest: String,
        cargo_toml: String,
        cargo_lock: String,
        src_lib_rs: String,
    },
    Capability {
        libs: Vec<CapBinary>,
        manifest: String,
        cargo_toml: String,
        cargo_lock: String,
        src_lib_rs: String,
        interface_json: String,
        config_json: Option<String>,
    },
    Interface {
        manifest: String,
        cargo_toml: String,
        src_lib_rs: String,
        interface_json: String,
        config_json: Option<String>,
    },
    AnonModule {
        wasm: Vec<u8>,
        doc: String,
    },
}

impl Artifacts {
    /// Writes the artifact to the specified directory structure.
    pub async fn write_to_directory(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let path = path.as_ref();
        fs::create_dir_all(path).await?;

        match self {
            Artifacts::Module {
                manifest,
                wasm,
                cargo_toml,
                cargo_lock,
                src_lib_rs,
            } => {
                fs::write(path.join("mod.wasm"), wasm).await?;
                fs::write(path.join("Module.toml"), manifest).await?;
                fs::write(path.join("Cargo.toml"), cargo_toml).await?;
                fs::write(path.join("Cargo.lock"), cargo_lock).await?;

                let src_dir = path.join("src");
                fs::create_dir_all(&src_dir).await?;
                fs::write(src_dir.join("lib.rs"), src_lib_rs).await?;
            }
            Artifacts::Capability {
                libs,
                manifest,
                cargo_toml,
                cargo_lock,
                src_lib_rs,
                interface_json,
                config_json,
            } => {
                for lib in libs {
                    match lib {
                        CapBinary::Pe(bytes) => fs::write(path.join("lib.dll"), bytes).await?,
                        CapBinary::MachO(bytes) => fs::write(path.join("lib.dylib"), bytes).await?,
                        CapBinary::Elf(bytes) => fs::write(path.join("lib.so"), bytes).await?,
                    }
                }
                fs::write(path.join("Capability.toml"), manifest).await?;
                fs::write(path.join("Cargo.toml"), cargo_toml).await?;
                fs::write(path.join("Cargo.lock"), cargo_lock).await?;
                fs::write(path.join("interface.json"), interface_json).await?;

                if let Some(config) = config_json {
                    fs::write(path.join("config.json"), config).await?;
                }

                let src_dir = path.join("src");
                fs::create_dir_all(&src_dir).await?;
                fs::write(src_dir.join("lib.rs"), src_lib_rs).await?;
            }
            Artifacts::Interface {
                manifest,
                cargo_toml,
                src_lib_rs,
                interface_json,
                config_json,
            } => {
                fs::write(path.join("Capability.toml"), manifest).await?;
                fs::write(path.join("Cargo.toml"), cargo_toml).await?;
                fs::write(path.join("interface.json"), interface_json).await?;

                if let Some(config) = config_json {
                    fs::write(path.join("config.json"), config).await?;
                }

                let src_dir = path.join("src");
                fs::create_dir_all(&src_dir).await?;
                fs::write(src_dir.join("lib.rs"), src_lib_rs).await?;
            }
            Artifacts::AnonModule { wasm, doc } => {
                fs::write(path.join("mod.wasm"), wasm).await?;
                fs::write(path.join("interface.json"), doc).await?;
            }
        }

        Ok(())
    }

    /// Packs the Artifacts enum directly into a gzip-compressed tarball.
    pub fn to_tarball(&self) -> Result<Vec<u8>, io::Error> {
        let encoder = GzEncoder::new(Vec::new(), Compression::default());
        let mut tar = Builder::new(encoder);

        let mut append_file = |name: &str, data: &[u8]| -> Result<(), io::Error> {
            let mut header = Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append_data(&mut header, name, data)
        };

        match self {
            Artifacts::Module {
                manifest,
                wasm,
                cargo_toml,
                cargo_lock,
                src_lib_rs,
            } => {
                append_file("mod.wasm", wasm)?;
                append_file("Module.toml", manifest.as_bytes())?;
                append_file("Cargo.toml", cargo_toml.as_bytes())?;
                append_file("Cargo.lock", cargo_lock.as_bytes())?;
                append_file("src/lib.rs", src_lib_rs.as_bytes())?;
            }
            Artifacts::Capability {
                libs,
                manifest,
                cargo_toml,
                cargo_lock,
                src_lib_rs,
                interface_json,
                config_json,
            } => {
                for lib in libs {
                    match lib {
                        CapBinary::Pe(bytes) => append_file("lib.dll", bytes)?,
                        CapBinary::MachO(bytes) => append_file("lib.dylib", bytes)?,
                        CapBinary::Elf(bytes) => append_file("lib.so", bytes)?,
                    }
                }
                append_file("Capability.toml", manifest.as_bytes())?;
                append_file("Cargo.toml", cargo_toml.as_bytes())?;
                append_file("Cargo.lock", cargo_lock.as_bytes())?;
                append_file("src/lib.rs", src_lib_rs.as_bytes())?;
                append_file("interface.json", interface_json.as_bytes())?;
                if let Some(config) = config_json {
                    append_file("config.json", config.as_bytes())?;
                }
            }
            Artifacts::Interface {
                manifest,
                cargo_toml,
                src_lib_rs,
                interface_json,
                config_json,
            } => {
                append_file("Capability.toml", manifest.as_bytes())?;
                append_file("Cargo.toml", cargo_toml.as_bytes())?;
                append_file("src/lib.rs", src_lib_rs.as_bytes())?;
                append_file("interface.json", interface_json.as_bytes())?;
                if let Some(config) = config_json {
                    append_file("config.json", config.as_bytes())?;
                }
            }
            Artifacts::AnonModule { wasm, doc } => {
                append_file("mod.wasm", wasm)?;
                append_file("interface.json", doc.as_bytes())?;
            }
        }

        let encoder = tar.into_inner()?;
        encoder.finish()
    }

    /// Extracts a Module artifact from a tarball. Errors if required files are missing.
    pub fn module_from_tarball(bytes: &[u8]) -> Result<Self, io::Error> {
        let tar = GzDecoder::new(bytes);
        let mut archive = tar::Archive::new(tar);

        let mut manifest = None;
        let mut wasm = None;
        let mut cargo_toml = None;
        let mut cargo_lock = None;
        let mut src_lib_rs = None;

        for file in archive.entries()? {
            let mut file = file?;
            let path = file.path()?.to_path_buf();
            let mut content = Vec::new();
            file.read_to_end(&mut content)?;

            match path.to_string_lossy().as_ref() {
                "mod.wasm" => wasm = Some(content),
                "Module.toml" => manifest = String::from_utf8(content).ok(),
                "Cargo.toml" => cargo_toml = String::from_utf8(content).ok(),
                "Cargo.lock" => cargo_lock = String::from_utf8(content).ok(),
                "src/lib.rs" => src_lib_rs = String::from_utf8(content).ok(),
                _ => {}
            }
        }

        Ok(Artifacts::Module {
            manifest: manifest.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "Missing or invalid Module.toml")
            })?,
            wasm: wasm
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Missing mod.wasm"))?,
            cargo_toml: cargo_toml.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "Missing or invalid Cargo.toml")
            })?,
            cargo_lock: cargo_lock.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "Missing or invalid Cargo.lock")
            })?,
            src_lib_rs: src_lib_rs.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "Missing or invalid src/lib.rs")
            })?,
        })
    }

    /// Extracts a Capability artifact from a tarball. Errors if required files or libraries are missing.
    pub fn capability_from_tarball(bytes: &[u8]) -> Result<Self, io::Error> {
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
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Missing capability library",
            ));
        }

        Ok(Artifacts::Capability {
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

    /// Extracts an Interface artifact from a tarball. Errors if required files are missing.
    pub fn interface_from_tarball(bytes: &[u8]) -> Result<Self, io::Error> {
        let tar = GzDecoder::new(bytes);
        let mut archive = tar::Archive::new(tar);

        let mut manifest = None;
        let mut cargo_toml = None;
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
                "Cargo.toml" => cargo_toml = String::from_utf8(content).ok(),
                "src/lib.rs" => src_lib_rs = String::from_utf8(content).ok(),
                "interface.json" => interface_json = String::from_utf8(content).ok(),
                "config.json" => config_json = String::from_utf8(content).ok(),
                _ => {}
            }
        }

        Ok(Artifacts::Interface {
            manifest: manifest.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "Missing Capability.toml")
            })?,
            cargo_toml: cargo_toml
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing Cargo.toml"))?,
            src_lib_rs: src_lib_rs
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing src/lib.rs"))?,
            interface_json: interface_json.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "Missing interface.json")
            })?,
            config_json,
        })
    }

    /// Extracts an AnonModule artifact from a tarball. Errors if the WASM file is missing.
    pub fn anon_module_from_tarball(bytes: &[u8]) -> Result<Self, io::Error> {
        let tar = GzDecoder::new(bytes);
        let mut archive = tar::Archive::new(tar);

        let mut wasm = None;
        let mut doc = None;

        for file in archive.entries()? {
            let mut file = file?;
            let path = file.path()?.to_path_buf();
            let mut content = Vec::new();
            file.read_to_end(&mut content)?;

            match path.to_string_lossy().as_ref() {
                "mod.wasm" => wasm = Some(content),
                "interface.json" => doc = String::from_utf8(content).ok(),
                _ => {}
            }
        }

        Ok(Artifacts::AnonModule {
            wasm: wasm
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Missing mod.wasm"))?,
            doc: doc.unwrap_or_default(),
        })
    }
}
