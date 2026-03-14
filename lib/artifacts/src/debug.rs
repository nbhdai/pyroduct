//! Advanced debugging tools for artifacts

use std::{borrow::Cow, collections::HashMap};

use crate::artifacts::append_file;
use crate::artifacts::{Artifact, Capability, Module};
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use object::{BinaryFormat, Object, ObjectSection, ObjectSymbol, SymbolKind};
use std::io::{self, Read};
use std::path::Path;
use tar::Builder;
use tokio::fs;

pub fn wat(module: &Module) -> Result<String, String> {
    wasmprinter::print_bytes(&module.wasm)
        .map_err(|e| format!("Failed to convert WASM to WAT: {}", e))
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CapSymbol {
    pub name: String,
    pub address: u64,
    pub signature: Option<String>,
}

/// Wraps the symbols with the detected file format of their parent binary
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CapSymbols {
    Elf(Vec<CapSymbol>),
    MachO(Vec<CapSymbol>),
    Pe(Vec<CapSymbol>),
    Unknown(Vec<CapSymbol>),
}

pub struct CapabilityDebug {
    pub symbols: Vec<Result<CapSymbols, String>>,
    pub cap_rs: Option<String>,
}

impl Artifact for CapabilityDebug {
    async fn write_to_directory(&self, path: &Path) -> std::io::Result<()> {
        fs::create_dir_all(path).await?;
        for sym in &self.symbols {
            let (name, content) = match sym {
                Ok(CapSymbols::Elf(sym)) => ("elf.json", sym),
                Ok(CapSymbols::MachO(sym)) => ("macho.json", sym),
                Ok(CapSymbols::Pe(sym)) => ("pe.json", sym),
                Ok(CapSymbols::Unknown(sym)) => ("Unknown.json", sym),
                Err(error) => {
                    tracing::error!(error, "Unable to get one set of symbols");
                    continue;
                }
            };
            match serde_json::to_string_pretty(&content) {
                Ok(content) => fs::write(path.join(name), content).await?,
                Err(error) => {
                    tracing::error!(?error, "Unable to serialize symbols");
                    continue;
                }
            }
        }

        if let Some(code) = &self.cap_rs {
            fs::write(path.join("cap.rs"), code).await?;
        }

        Ok(())
    }

    fn to_tarball(&self) -> Result<Vec<u8>, io::Error> {
        let encoder = GzEncoder::new(Vec::new(), Compression::default());
        let mut tar = Builder::new(encoder);

        for sym in &self.symbols {
            let (name, content) = match sym {
                Ok(CapSymbols::Elf(sym)) => ("elf.json", sym),
                Ok(CapSymbols::MachO(sym)) => ("macho.json", sym),
                Ok(CapSymbols::Pe(sym)) => ("pe.json", sym),
                Ok(CapSymbols::Unknown(sym)) => ("Unknown.json", sym),
                Err(_) => continue,
            };
            let content = serde_json::to_vec_pretty(&content).map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, format!("JSON error: {}", e))
            })?;
            append_file(&mut tar, name, &content)?;
        }

        if let Some(code) = &self.cap_rs {
            append_file(&mut tar, "cap.rs", code.as_bytes())?;
        }

        tar.into_inner()?.finish()
    }

    fn from_tarball(bytes: &[u8]) -> Result<Self, io::Error> {
        let tar = GzDecoder::new(bytes);
        let mut archive = tar::Archive::new(tar);

        let mut symbols = Vec::new();
        let mut cap_rs = None;

        for file in archive.entries()? {
            let mut file = file?;
            let path = file.path()?.to_path_buf();
            let mut content = Vec::new();
            file.read_to_end(&mut content)?;

            let filename = path.to_string_lossy();
            if filename.ends_with(".json") {
                let sym: Vec<CapSymbol> = serde_json::from_slice(&content)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                let cap_symbols = if filename == "elf.json" {
                    CapSymbols::Elf(sym)
                } else if filename == "macho.json" {
                    CapSymbols::MachO(sym)
                } else if filename == "pe.json" {
                    CapSymbols::Pe(sym)
                } else {
                    CapSymbols::Unknown(sym)
                };
                symbols.push(Ok(cap_symbols));
            } else if filename == "cap.rs" {
                cap_rs = String::from_utf8(content).ok();
            }
        }

        Ok(CapabilityDebug { symbols, cap_rs })
    }

    async fn from_dir(path: &Path) -> Result<Self, io::Error> {
        let mut symbols = Vec::new();
        let mut cap_rs = None;

        let names = ["elf.json", "macho.json", "pe.json", "Unknown.json"];
        for name in names {
            let full_path = path.join(name);
            if let Ok(content) = fs::read(&full_path).await {
                let sym: Vec<CapSymbol> = serde_json::from_slice(&content).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Failed to parse {}: {}", name, e),
                    )
                })?;
                let cap_symbols = match name {
                    "elf.json" => CapSymbols::Elf(sym),
                    "macho.json" => CapSymbols::MachO(sym),
                    "pe.json" => CapSymbols::Pe(sym),
                    _ => CapSymbols::Unknown(sym),
                };
                symbols.push(Ok(cap_symbols));
            }
        }

        if let Ok(code) = fs::read_to_string(path.join("cap.rs")).await {
            cap_rs = Some(code);
        }

        Ok(CapabilityDebug { symbols, cap_rs })
    }
}

pub struct ModuleDebug {
    pub wat: Option<String>,
    pub cap_rs: Option<String>,
}

impl Artifact for ModuleDebug {
    async fn write_to_directory(&self, path: &Path) -> std::io::Result<()> {
        fs::create_dir_all(path).await?;
        if let Some(wat) = &self.wat {
            fs::write(path.join("mod.wat"), wat).await?;
        }
        if let Some(code) = &self.cap_rs {
            fs::write(path.join("cap.rs"), code).await?;
        }
        Ok(())
    }

    fn to_tarball(&self) -> Result<Vec<u8>, io::Error> {
        let encoder = GzEncoder::new(Vec::new(), Compression::default());
        let mut tar = Builder::new(encoder);

        if let Some(wat) = &self.wat {
            append_file(&mut tar, "mod.wat", wat.as_bytes())?;
        }
        if let Some(code) = &self.cap_rs {
            append_file(&mut tar, "cap.rs", code.as_bytes())?;
        }

        tar.into_inner()?.finish()
    }

    fn from_tarball(bytes: &[u8]) -> Result<Self, io::Error> {
        let tar = GzDecoder::new(bytes);
        let mut archive = tar::Archive::new(tar);

        let mut wat = None;
        let mut cap_rs = None;

        for file in archive.entries()? {
            let mut file = file?;
            let path = file.path()?.to_path_buf();
            let mut content = Vec::new();
            file.read_to_end(&mut content)?;

            match path.to_string_lossy().as_ref() {
                "mod.wat" => wat = String::from_utf8(content).ok(),
                "cap.rs" => cap_rs = String::from_utf8(content).ok(),
                _ => {}
            }
        }

        Ok(ModuleDebug { wat, cap_rs })
    }

    async fn from_dir(path: &Path) -> Result<Self, io::Error> {
        let mut wat = None;
        let mut cap_rs = None;

        if let Ok(w) = fs::read_to_string(path.join("mod.wat")).await {
            wat = Some(w);
        }
        if let Ok(c) = fs::read_to_string(path.join("cap.rs")).await {
            cap_rs = Some(c);
        }

        Ok(ModuleDebug { wat, cap_rs })
    }
}

/// Scans a dynamic library and uses DWARF debug info to reconstruct signatures
pub fn symbols(capability: &Capability) -> Vec<Result<CapSymbols, String>> {
    let mut results = Vec::new();

    for (index, bin) in capability.libs.iter().enumerate() {
        let data: &[u8] = &**bin;

        let file = match object::File::parse(data) {
            Ok(f) => f,
            Err(e) => {
                results.push(Err(format!(
                    "Failed to parse binary at index {}: {}",
                    index, e
                )));
                continue;
            }
        };

        let mut symbols = HashMap::new();
        for symbol in file.symbols() {
            if symbol.kind() == SymbolKind::Text && symbol.is_global() && !symbol.is_undefined() {
                let name = symbol.name().unwrap_or("<unknown>");

                // Target specifically exports that start with "p_"
                if name.trim_start_matches('_').starts_with("p_") {
                    symbols.insert(
                        name.to_string(),
                        CapSymbol {
                            name: name.to_string(),
                            address: symbol.address(),
                            signature: None,
                        },
                    );
                }
            }
        }

        if symbols.is_empty() {
            continue;
        }

        let endian = if file.is_little_endian() {
            gimli::RunTimeEndian::Little
        } else {
            gimli::RunTimeEndian::Big
        };

        let load_section = |id: gimli::SectionId| -> Result<Cow<[u8]>, gimli::Error> {
            match file.section_by_name(id.name()) {
                Some(ref section) => Ok(section
                    .uncompressed_data()
                    .unwrap_or(Cow::Borrowed(&[][..]))),
                None => Ok(Cow::Borrowed(&[][..])),
            }
        };

        let wrap_symbols = |syms: HashMap<String, CapSymbol>| -> CapSymbols {
            let vec_syms = syms.into_values().collect();
            match file.format() {
                BinaryFormat::Elf => CapSymbols::Elf(vec_syms),
                BinaryFormat::MachO => CapSymbols::MachO(vec_syms),
                BinaryFormat::Pe => CapSymbols::Pe(vec_syms),
                _ => CapSymbols::Unknown(vec_syms),
            }
        };

        let dwarf_sections = match gimli::DwarfSections::load(&load_section) {
            Ok(s) => s,
            Err(_) => {
                results.push(Ok(wrap_symbols(symbols)));
                continue;
            }
        };

        let dwarf = dwarf_sections.borrow(|section| gimli::EndianSlice::new(&*section, endian));

        if let Err(e) = enrich_signatures_with_dwarf(&dwarf, endian, &mut symbols) {
            results.push(Err(format!("DWARF parsing error: {}", e)));
        }

        results.push(Ok(wrap_symbols(symbols)));
    }

    results
}

fn enrich_signatures_with_dwarf(
    dwarf: &gimli::Dwarf<gimli::EndianSlice<gimli::RunTimeEndian>>,
    _endian: gimli::RunTimeEndian,
    symbols: &mut HashMap<String, CapSymbol>,
) -> Result<(), gimli::Error> {
    let mut iter = dwarf.units();

    while let Some(header) = iter.next()? {
        let unit = dwarf.unit(header)?;
        let mut entries = unit.entries();

        while let Some(entry) = entries.next_dfs()? {
            if entry.tag == gimli::DW_TAG_subprogram {
                let name_attr = entry
                    .attr_value(gimli::DW_AT_linkage_name)
                    .or(entry.attr_value(gimli::DW_AT_name));

                let name = if let Some(attr) = name_attr {
                    // Use `attr_string` to natively handle both string offsets and inline strings
                    if let Ok(s) = dwarf.attr_string(&unit, attr) {
                        s.to_string_lossy().into_owned()
                    } else {
                        continue;
                    }
                } else {
                    continue;
                };

                if let Some(sym) = symbols.get_mut(&name) {
                    let mut args = Vec::new();
                    let mut return_type = String::from("()");

                    if let Some(gimli::AttributeValue::UnitRef(offset)) =
                        entry.attr_value(gimli::DW_AT_type)
                    {
                        // Pass `dwarf` down into the type resolver
                        return_type = resolve_dwarf_type(dwarf, &unit, offset)
                            .unwrap_or_else(|| "Unknown".to_string());
                    }

                    let mut children = unit.entries_at_offset(entry.offset)?;
                    children.next_dfs()?;

                    while let Some(child) = children.next_dfs()? {
                        if child.depth <= 0 {
                            break;
                        }

                        if child.tag == gimli::DW_TAG_formal_parameter {
                            if let Some(gimli::AttributeValue::UnitRef(offset)) =
                                child.attr_value(gimli::DW_AT_type)
                            {
                                // Pass `dwarf` down into the type resolver
                                args.push(
                                    resolve_dwarf_type(dwarf, &unit, offset)
                                        .unwrap_or_else(|| "Unknown".to_string()),
                                );
                            }
                        }
                    }

                    sym.signature = Some(format!(
                        "fn {}({}) -> {}",
                        name,
                        args.join(", "),
                        return_type
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Recursively resolves DWARF types (handling pointers, modifiers, and base types)
fn resolve_dwarf_type(
    dwarf: &gimli::Dwarf<gimli::EndianSlice<gimli::RunTimeEndian>>,
    unit: &gimli::Unit<gimli::EndianSlice<gimli::RunTimeEndian>, usize>,
    offset: gimli::UnitOffset,
) -> Option<String> {
    let mut entries = unit.entries_at_offset(offset).ok()?;
    let entry = entries.next_dfs().ok()??;

    // 1. If this DWARF node has a name directly (e.g., base type, struct, typedef), return it.
    if let Some(attr) = entry.attr_value(gimli::DW_AT_name) {
        if let Ok(s) = dwarf.attr_string(unit, attr) {
            return Some(s.to_string_lossy().into_owned());
        }
    }

    // 2. Format pointer types natively
    if entry.tag == gimli::DW_TAG_pointer_type {
        if let Some(gimli::AttributeValue::UnitRef(inner_offset)) =
            entry.attr_value(gimli::DW_AT_type)
        {
            return Some(format!(
                "*{}",
                resolve_dwarf_type(dwarf, unit, inner_offset).unwrap_or_else(|| "void".into())
            ));
        }
        return Some("*void".to_string());
    }

    // 3. For unnamed modifiers (const, volatile) or typedefs, follow the DW_AT_type chain down.
    if let Some(gimli::AttributeValue::UnitRef(inner_offset)) = entry.attr_value(gimli::DW_AT_type)
    {
        return resolve_dwarf_type(dwarf, unit, inner_offset);
    }

    // 4. Fallbacks for unnamed composite types
    match entry.tag {
        gimli::DW_TAG_structure_type => Some("<unnamed struct>".to_string()),
        gimli::DW_TAG_union_type => Some("<unnamed union>".to_string()),
        gimli::DW_TAG_enumeration_type => Some("<unnamed enum>".to_string()),
        gimli::DW_TAG_subroutine_type => Some("<function pointer>".to_string()),
        _ => None,
    }
}
