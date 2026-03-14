//! Advanced debugging tools for artifacts

use std::{borrow::Cow, collections::HashMap};

use crate::artifacts::{Module, Capability};
use object::{Object, ObjectSection, ObjectSymbol, SymbolKind, BinaryFormat};

pub fn wat(module: &Module) -> Result<String, String> {
    wasmprinter::print_bytes(&module.wasm).map_err(|e| format!("Failed to convert WASM to WAT: {}", e))
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

/// Scans a dynamic library and uses DWARF debug info to reconstruct signatures
pub fn symbols(capability: &Capability) -> Vec<Result<CapSymbols, String>> {
    let mut results = Vec::new();

    for (index, bin) in capability.libs.iter().enumerate() {
        let data: &[u8] = &**bin;

        let file = match object::File::parse(data) {
            Ok(f) => f,
            Err(e) => {
                results.push(Err(format!("Failed to parse binary at index {}: {}", index, e)));
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

        let dwarf = dwarf_sections.borrow(|section| {
            gimli::EndianSlice::new(&*section, endian)
        });

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
                
                let name_attr = entry.attr_value(gimli::DW_AT_linkage_name)
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

                    if let Some(gimli::AttributeValue::UnitRef(offset)) = entry.attr_value(gimli::DW_AT_type) {
                        // Pass `dwarf` down into the type resolver
                        return_type = resolve_dwarf_type(dwarf, &unit, offset).unwrap_or_else(|| "Unknown".to_string());
                    }

                    let mut children = unit.entries_at_offset(entry.offset)?;
                    children.next_dfs()?; 

                    while let Some(child) = children.next_dfs()? {
                        if child.depth <= 0 { break; } 

                        if child.tag == gimli::DW_TAG_formal_parameter {
                            if let Some(gimli::AttributeValue::UnitRef(offset)) = child.attr_value(gimli::DW_AT_type) {
                                // Pass `dwarf` down into the type resolver
                                args.push(resolve_dwarf_type(dwarf, &unit, offset).unwrap_or_else(|| "Unknown".to_string()));
                            }
                        }
                    }

                    sym.signature = Some(format!("fn {}({}) -> {}", name, args.join(", "), return_type));
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
        if let Some(gimli::AttributeValue::UnitRef(inner_offset)) = entry.attr_value(gimli::DW_AT_type) {
            return Some(format!("*{}", resolve_dwarf_type(dwarf, unit, inner_offset).unwrap_or_else(|| "void".into())));
        }
        return Some("*void".to_string());
    }

    // 3. For unnamed modifiers (const, volatile) or typedefs, follow the DW_AT_type chain down.
    if let Some(gimli::AttributeValue::UnitRef(inner_offset)) = entry.attr_value(gimli::DW_AT_type) {
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