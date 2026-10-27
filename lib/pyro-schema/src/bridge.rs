//! The only place `types/` (the `PyroType`/`PyroSchema` family) and `wire/`
//! (`Inode`/`WireKind`) meet — the `PyroType` ↔ `WireKind` conversion, the
//! `Layout` const-layout trait, `SchemaParts`, the metadata (names/docs)
//! string table, and `PyroSchema`'s own encode/decode. See
//! `docs/pyro-schema-crate-plan.md`.

use std::borrow::Cow;

mod const_type;
pub use const_type::{arena_root, inodes_to_type, type_node_count};

use crate::bow::Bow;
use crate::error::SchemaError;
use crate::types::{ChildField, FieldNames, PrimitiveDataType, PyroSchema, PyroType};
use crate::wire::{INODE_SIZE, Inode, InodeMeta, KindMeta, WireKind, layout_in_place};
use crate::word::Word;

/// Try to interpret a [`WireKind`] as a [`PrimitiveDataType`].
///
/// Free function, not an inherent method: `WireKind` lives in `wire/`,
/// `PrimitiveDataType` in `types/`, and neither module may depend on the
/// other — this function is the bridge between them.
pub const fn wire_kind_as_primitive_data_type(kind: WireKind) -> Option<PrimitiveDataType> {
    match kind {
        WireKind::Bool => Some(PrimitiveDataType::Bool),
        WireKind::I8 => Some(PrimitiveDataType::I8),
        WireKind::I16 => Some(PrimitiveDataType::I16),
        WireKind::I32 => Some(PrimitiveDataType::I32),
        WireKind::I64 => Some(PrimitiveDataType::I64),
        WireKind::U8 => Some(PrimitiveDataType::U8),
        WireKind::U16 => Some(PrimitiveDataType::U16),
        WireKind::U32 => Some(PrimitiveDataType::U32),
        WireKind::U64 => Some(PrimitiveDataType::U64),
        WireKind::F16 => Some(PrimitiveDataType::F16),
        WireKind::F32 => Some(PrimitiveDataType::F32),
        WireKind::F64 => Some(PrimitiveDataType::F64),
        WireKind::Timestamp => Some(PrimitiveDataType::Timestamp),
        _ => None,
    }
}

fn required_from_primitive(pdt: PrimitiveDataType) -> WireKind {
    match pdt {
        PrimitiveDataType::Bool => WireKind::Bool,
        PrimitiveDataType::I8 => WireKind::I8,
        PrimitiveDataType::I16 => WireKind::I16,
        PrimitiveDataType::I32 => WireKind::I32,
        PrimitiveDataType::I64 => WireKind::I64,
        PrimitiveDataType::U8 => WireKind::U8,
        PrimitiveDataType::U16 => WireKind::U16,
        PrimitiveDataType::U32 => WireKind::U32,
        PrimitiveDataType::U64 => WireKind::U64,
        PrimitiveDataType::F16 => WireKind::F16,
        PrimitiveDataType::F32 => WireKind::F32,
        PrimitiveDataType::F64 => WireKind::F64,
        PrimitiveDataType::Timestamp => WireKind::Timestamp,
    }
}

fn optional_from_primitive(pdt: PrimitiveDataType) -> WireKind {
    match pdt {
        PrimitiveDataType::Bool => WireKind::OptionBool,
        PrimitiveDataType::I8 => WireKind::OptionI8,
        PrimitiveDataType::I16 => WireKind::OptionI16,
        PrimitiveDataType::I32 => WireKind::OptionI32,
        PrimitiveDataType::I64 => WireKind::OptionI64,
        PrimitiveDataType::U8 => WireKind::OptionU8,
        PrimitiveDataType::U16 => WireKind::OptionU16,
        PrimitiveDataType::U32 => WireKind::OptionU32,
        PrimitiveDataType::U64 => WireKind::OptionU64,
        PrimitiveDataType::F16 => WireKind::OptionF16,
        PrimitiveDataType::F32 => WireKind::OptionF32,
        PrimitiveDataType::F64 => WireKind::OptionF64,
        PrimitiveDataType::Timestamp => WireKind::OptionTimestamp,
    }
}

/// Convert a [`PyroType`] into its wire-level [`WireKind`] discriminant.
///
/// A free function rather than `impl From<PyroType> for WireKind`, since
/// this crate's `types`/`wire` module split is the same discipline that
/// used to be an inter-crate boundary: neither module may reach into the
/// other, so the conversion lives here in `bridge.rs` instead.
pub fn pyro_type_to_wire_kind(value: &PyroType) -> WireKind {
    match value {
        PyroType::PrimitiveScalar(pdt) => required_from_primitive(*pdt),
        PyroType::OptionPrimitiveScalar(pdt) => optional_from_primitive(*pdt),
        PyroType::PrimitiveList(pdt) => required_from_primitive(*pdt),
        PyroType::PrimitiveFixedList(pdt, _) => required_from_primitive(*pdt),
        PyroType::OptionPrimitiveFixedList(pdt, _) => optional_from_primitive(*pdt),

        PyroType::Str => WireKind::Str,
        PyroType::Bytes => WireKind::Bytes,
        PyroType::List(inner) => pyro_type_to_wire_kind(inner.as_ref()),
        PyroType::Group(_) | PyroType::OptionGroup(_) => WireKind::Group,
        PyroType::Unknown => WireKind::Unknown,
        PyroType::Map { .. } => WireKind::Map,
    }
}

const HAS_DOCS: u32 = 1 << 31;

pub fn encode_metadata(
    schema_doc: Option<&str>,
    fields: &[&str],
    docs: Option<&[&str]>,
) -> Vec<u8> {
    let mut strings = Vec::new();
    let count_val;
    if let Some(d) = docs {
        count_val = (fields.len() as u32) | HAS_DOCS;
        strings.push(schema_doc.unwrap_or_default());
        for (f, doc) in fields.iter().zip(d.iter()) {
            strings.push(*f);
            strings.push(*doc);
        }
    } else {
        count_val = fields.len() as u32;
        for f in fields {
            strings.push(*f);
        }
    }

    let total_strings = strings.len();
    let header_size = 4 + 4 * (total_strings + 1);
    let data_size: usize = strings.iter().map(|s| s.len()).sum();
    let mut buf = Vec::with_capacity(header_size + data_size);

    buf.extend_from_slice(&(count_val as u32).to_le_bytes());

    let mut offset = 0u32;
    for s in &strings {
        buf.extend_from_slice(&offset.to_le_bytes());
        offset += s.len() as u32;
    }
    buf.extend_from_slice(&offset.to_le_bytes()); // sentinel

    for s in strings {
        buf.extend_from_slice(s.as_bytes());
    }
    buf
}

pub fn decode_metadata_header(fields_buf: &'_ [u8]) -> Result<(bool, u32), SchemaError> {
    if fields_buf.len() < 4 {
        return Err(SchemaError::InvalidUtf8);
    }
    let count_val =
        u32::from_le_bytes([fields_buf[0], fields_buf[1], fields_buf[2], fields_buf[3]]);
    let has_docs = (count_val & HAS_DOCS) != 0;
    let num_fields = count_val & !HAS_DOCS;
    Ok((has_docs, num_fields))
}

pub fn decode_metadata_string(
    fields_buf: &'_ [u8],
    string_idx: u32,
) -> Result<&'_ str, SchemaError> {
    if fields_buf.len() < 4 {
        return Err(SchemaError::FieldOutOfRange(string_idx as u16));
    }
    let count_val =
        u32::from_le_bytes([fields_buf[0], fields_buf[1], fields_buf[2], fields_buf[3]]);
    let has_docs = (count_val & HAS_DOCS) != 0;
    let num_fields = count_val & !HAS_DOCS;
    let total_strings = if has_docs {
        1 + 2 * num_fields
    } else {
        num_fields
    };

    if string_idx >= total_strings {
        return Err(SchemaError::FieldOutOfRange(string_idx as u16));
    }

    let off_base = 4;
    let i = string_idx as usize;
    let start = u32::from_le_bytes([
        fields_buf[off_base + i * 4],
        fields_buf[off_base + i * 4 + 1],
        fields_buf[off_base + i * 4 + 2],
        fields_buf[off_base + i * 4 + 3],
    ]) as usize;
    let end = u32::from_le_bytes([
        fields_buf[off_base + (i + 1) * 4],
        fields_buf[off_base + (i + 1) * 4 + 1],
        fields_buf[off_base + (i + 1) * 4 + 2],
        fields_buf[off_base + (i + 1) * 4 + 3],
    ]) as usize;

    let data_base = off_base + (total_strings as usize + 1) * 4;
    let bytes = &fields_buf[data_base + start..data_base + end];
    std::str::from_utf8(bytes).map_err(|_| SchemaError::InvalidUtf8)
}

#[derive(Debug, PartialEq, Eq)]
pub struct SchemaParts<'a> {
    pub inodes: Vec<Inode>,
    pub field: Vec<&'a str>,
    pub documentation: Vec<&'a str>,
    pub schema_documentation: Option<&'a str>,
    /// Message-wide settings (the structure [`Word`]) that [`Self::inodes`]
    /// needs to be read. Sent as the table's first slot; see [`Self::encode`].
    pub meta: InodeMeta,
}

impl<'a> SchemaParts<'a> {
    /// Number of immediate top-level pushes (`push_field`/`push_group`/
    /// `push_map`) this builder has received so far — exactly the `count`
    /// [`Self::push_group`] needs when folding a builder in as a nested
    /// `Group`'s children. Derived by walking `self.inodes` and counting
    /// how many root inodes it takes to consume the whole tree, using the
    /// same span logic as [`Self::parse_type`]/[`Self::to_schema`], rather
    /// than tracked incrementally.
    fn top_level_count(&self) -> Result<u16, &'static str> {
        let mut inode_iter = self.inodes.iter().copied();
        let mut count: u16 = 0;
        let mut consumed = 0;
        let total = self.inodes.len();
        while consumed < total {
            let inode = inode_iter.next().ok_or("Unexpected end of inodes")?;
            consumed += 1;
            let (_, n) = Self::parse_type(&mut inode_iter, inode)?;
            consumed += n;
            count += 1;
        }
        Ok(count)
    }
}

impl<'a> Default for SchemaParts<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> SchemaParts<'a> {
    pub fn new() -> Self {
        Self {
            inodes: Vec::new(),
            field: Vec::new(),
            documentation: Vec::new(),
            schema_documentation: None,
            meta: InodeMeta::DEFAULT,
        }
    }

    pub fn set_word(&mut self, word: Word) {
        self.meta.word = word;
    }

    /// Lay out `inodes` in place in `meta`'s word.
    fn layout(meta: InodeMeta, inodes: &mut [Inode]) -> usize {
        match meta.word {
            Word::U16 => layout_in_place::<u16>(inodes),
            Word::U32 => layout_in_place::<u32>(inodes),
        }
    }

    /// Set the schema-level documentation string.
    pub fn set_schema_doc(&mut self, doc: &'a str) {
        self.schema_documentation = if doc.is_empty() { None } else { Some(doc) };
    }

    /// Push a plain (non-Group, non-Map) field: its own name/doc, plus its
    /// inode tree. Use [`Self::push_group`]/[`Self::push_map`] for those two
    /// cases instead — [`Self::push_node`] itself refuses to handle them.
    pub fn push_field(
        &mut self,
        name: &'a str,
        doc: &'a str,
        ty: &PyroType,
    ) -> Result<(), &'static str> {
        if ty.group_children_types().is_some() {
            return Err("Cannot push group or map types directly");
        }
        self.field.push(name);
        self.documentation.push(doc);
        self.push_node(ty)
    }

    pub fn push_group(
        &mut self,
        name: &'a str,
        doc: &'a str,
        nullable: bool,
        nesting: usize,
        ty: SchemaParts<'a>,
    ) -> Result<(), &'static str> {
        let node = Inode {
            kind: if nullable {
                WireKind::OptionGroup
            } else {
                WireKind::Group
            },
            kind_meta: KindMeta::Group {
                nesting: nesting as u8,
                count: ty.top_level_count()?,
                topology_offset: 0,
                nullable_bit_index: 0,
                group: false,
                group_nest: false,
            },
        };
        self.field.push(name);
        self.documentation.push(doc);
        self.inodes.push(node);

        self.field.extend(ty.field);
        self.documentation.extend(ty.documentation);
        self.inodes.extend(ty.inodes);
        Ok(())
    }

    /// Push a `Map { key, value }` field: a `Map` root inode followed by the
    /// key's and value's own inode trees (no separate name/doc entries for
    /// the key/value themselves — [`Self::parse_type`]'s `Map` arm doesn't
    /// expect any). Moved out of [`Self::push_node`] so that function can
    /// reject bare `Map`/`Group` types instead of silently recursing into
    /// them.
    pub fn push_map(
        &mut self,
        name: &'a str,
        doc: &'a str,
        key_ty: &PyroType,
        value_ty: &PyroType,
    ) -> Result<(), &'static str> {
        self.field.push(name);
        self.documentation.push(doc);
        self.inodes.push(Inode {
            kind: WireKind::Map,
            kind_meta: KindMeta::Map {
                nesting: 0,
                topology_offset: 0,
                group: false,
                group_nest: false,
            },
        });
        self.push_node(key_ty)?;
        self.push_node(value_ty)
    }

    /// Build the inodes from all pushed fields and return them.
    /// This is useful for fuzzing where we need to separate schema
    /// construction from encoding.
    pub fn build_inodes(mut self) -> Vec<Inode> {
        Self::layout(self.meta, &mut self.inodes);
        self.inodes
    }

    fn push_node(&mut self, pyro_type: &PyroType) -> Result<(), &'static str> {
        match pyro_type {
            PyroType::Unknown => {
                self.inodes.push(Inode {
                    kind: WireKind::Unknown,
                    kind_meta: KindMeta::Primitive {
                        nesting: 0,
                        data_offset: 0,
                        topology_offset: 0,
                        fixed_size: 0,
                        nullable_bit_index: 0,
                        group: false,
                        group_nest: false,
                    },
                });
            }
            PyroType::PrimitiveScalar(pdt) => {
                let kind = required_from_primitive(*pdt);
                self.inodes.push(Inode {
                    kind,
                    kind_meta: KindMeta::Primitive {
                        nesting: 0,
                        data_offset: 0,
                        topology_offset: 0,
                        fixed_size: 0,
                        nullable_bit_index: 0,
                        group: false,
                        group_nest: false,
                    },
                });
            }
            PyroType::OptionPrimitiveScalar(pdt) => {
                let kind = optional_from_primitive(*pdt);
                self.inodes.push(Inode {
                    kind,
                    kind_meta: KindMeta::Primitive {
                        nesting: 0,
                        data_offset: 0,
                        topology_offset: 0,
                        fixed_size: 0,
                        nullable_bit_index: 0,
                        group: false,
                        group_nest: false,
                    },
                });
            }
            PyroType::Str => {
                self.inodes.push(Inode {
                    kind: WireKind::Str,
                    kind_meta: KindMeta::Primitive {
                        nesting: 0,
                        data_offset: 0,
                        topology_offset: 0,
                        fixed_size: 0,
                        nullable_bit_index: 0,
                        group: false,
                        group_nest: false,
                    },
                });
            }
            PyroType::Bytes => {
                self.inodes.push(Inode {
                    kind: WireKind::Bytes,
                    kind_meta: KindMeta::Primitive {
                        nesting: 0,
                        data_offset: 0,
                        topology_offset: 0,
                        fixed_size: 0,
                        nullable_bit_index: 0,
                        group: false,
                        group_nest: false,
                    },
                });
            }
            PyroType::PrimitiveList(pdt) => {
                let kind = required_from_primitive(*pdt);
                self.inodes.push(Inode {
                    kind,
                    kind_meta: KindMeta::Primitive {
                        nesting: 1,
                        data_offset: 0,
                        topology_offset: 0,
                        fixed_size: 0,
                        nullable_bit_index: 0,
                        group: false,
                        group_nest: false,
                    },
                });
            }
            PyroType::PrimitiveFixedList(_, 0) | PyroType::OptionPrimitiveFixedList(_, 0) => {
                // An inode's `fixed_size: 0` means "not a fixed array".
                return Err("A fixed array must have at least one element");
            }
            PyroType::PrimitiveFixedList(pdt, size) => {
                // `nesting: 0` — a single inline `[T; N]`, matching
                // `OptionPrimitiveFixedList` just below and the actual
                // runtime layout (`wire::writers::fixed`/`wire::readers::fixed`:
                // N elements written directly at `data_offset`, no pointer).
                let kind = required_from_primitive(*pdt);
                self.inodes.push(Inode {
                    kind,
                    kind_meta: KindMeta::Primitive {
                        nesting: 0,
                        data_offset: 0,
                        topology_offset: 0,
                        fixed_size: *size as u16,
                        nullable_bit_index: 0,
                        group: false,
                        group_nest: false,
                    },
                });
            }
            PyroType::OptionPrimitiveFixedList(pdt, size) => {
                let kind = optional_from_primitive(*pdt);
                self.inodes.push(Inode {
                    kind,
                    kind_meta: KindMeta::Primitive {
                        nesting: 0,
                        data_offset: 0,
                        topology_offset: 0,
                        fixed_size: *size as u16,
                        nullable_bit_index: 0,
                        group: false,
                        group_nest: false,
                    },
                });
            }
            PyroType::Group(types) | PyroType::OptionGroup(types) => {
                let nullable = matches!(pyro_type, PyroType::OptionGroup(_));
                let start_idx = self.inodes.len();
                let node = Inode {
                    kind: if nullable {
                        WireKind::OptionGroup
                    } else {
                        WireKind::Group
                    },
                    kind_meta: KindMeta::Group {
                        nesting: 0,
                        count: types.len() as u16,
                        topology_offset: 0,
                        nullable_bit_index: 0,
                        group: false,
                        group_nest: false,
                    },
                };
                self.inodes.push(node);
                for ty in types.iter() {
                    self.push_node(ty)?;
                }
                let _ = start_idx;
            }
            PyroType::List(inner_pyro_type) => {
                let mut nesting = 1;
                let mut ty = inner_pyro_type.as_ref();

                // Unroll nested lists to find the base type and total nesting depth
                while let PyroType::List(pyro_type) = ty {
                    ty = pyro_type.as_ref();
                    nesting += 1;
                }

                // Record how many inodes we currently have
                let start_idx = self.inodes.len();

                // Push the underlying type (which will push its root inode + any children)
                self.push_node(ty)?;

                // Now, go back and inject the nesting level into the root inode of this list
                if let Some(root_inode) = self.inodes.get_mut(start_idx) {
                    match &mut root_inode.kind_meta {
                        KindMeta::Primitive { nesting: n, .. } => {
                            *n += nesting;
                        }
                        KindMeta::Group { nesting: n, .. } | KindMeta::Map { nesting: n, .. } => {
                            *n = nesting;
                        }
                    }
                }
            }
            PyroType::Map { key, value } => {
                self.inodes.push(Inode {
                    kind: WireKind::Map,
                    kind_meta: KindMeta::Map {
                        nesting: 0,
                        topology_offset: 0,
                        group: false,
                        group_nest: false,
                    },
                });
                self.push_node(key)?;
                self.push_node(value)?;
            }
        }
        Ok(())
    }

    fn parse_type<I>(inode_iter: &mut I, inode: Inode) -> Result<(PyroType, usize), &'static str>
    where
        I: Iterator<Item = Inode>,
    {
        macro_rules! resolve_primitive {
            ($pdt:ident) => {
                if let KindMeta::Primitive {
                    nesting,
                    fixed_size,
                    ..
                } = inode.kind_meta
                {
                    let mut ty = if fixed_size > 0 {
                        PyroType::PrimitiveFixedList(PrimitiveDataType::$pdt, fixed_size as usize)
                    } else if nesting > 0 {
                        PyroType::PrimitiveList(PrimitiveDataType::$pdt)
                    } else {
                        PyroType::PrimitiveScalar(PrimitiveDataType::$pdt)
                    };

                    // Only a plain `PrimitiveList` carries the first list
                    // layer itself; a `PrimitiveFixedList` sits at `nesting: 0`
                    // (see `push_node`).
                    let loops = if fixed_size > 0 {
                        nesting
                    } else {
                        nesting.saturating_sub(1)
                    };

                    for _ in 0..loops {
                        ty = PyroType::List(Bow::new(ty));
                    }
                    ty
                } else {
                    unreachable!()
                }
            };
        }

        macro_rules! resolve_option_primitive {
            ($pdt:ident) => {
                if let KindMeta::Primitive {
                    nesting,
                    fixed_size,
                    ..
                } = inode.kind_meta
                {
                    let mut ty = if fixed_size > 0 {
                        PyroType::OptionPrimitiveFixedList(
                            PrimitiveDataType::$pdt,
                            fixed_size as usize,
                        )
                    } else {
                        PyroType::OptionPrimitiveScalar(PrimitiveDataType::$pdt)
                    };

                    // Neither option base carries a list layer (both sit at
                    // `nesting: 0`, see `push_node`), so every level is a
                    // `List`.
                    let loops = nesting;

                    for _ in 0..loops {
                        ty = PyroType::List(Bow::new(ty));
                    }
                    ty
                } else {
                    unreachable!()
                }
            };
        }

        let mut total_consumed = 0;
        let dt = match inode.kind {
            WireKind::Unknown => PyroType::Unknown,
            WireKind::Bool => resolve_primitive!(Bool),
            WireKind::I8 => resolve_primitive!(I8),
            WireKind::I16 => resolve_primitive!(I16),
            WireKind::I32 => resolve_primitive!(I32),
            WireKind::I64 => resolve_primitive!(I64),
            WireKind::U8 => resolve_primitive!(U8),
            WireKind::U16 => resolve_primitive!(U16),
            WireKind::U32 => resolve_primitive!(U32),
            WireKind::U64 => resolve_primitive!(U64),
            WireKind::F16 => resolve_primitive!(F16),
            WireKind::F32 => resolve_primitive!(F32),
            WireKind::F64 => resolve_primitive!(F64),
            WireKind::OptionBool => resolve_option_primitive!(Bool),
            WireKind::OptionI8 => resolve_option_primitive!(I8),
            WireKind::OptionI16 => resolve_option_primitive!(I16),
            WireKind::OptionI32 => resolve_option_primitive!(I32),
            WireKind::OptionI64 => resolve_option_primitive!(I64),
            WireKind::OptionU8 => resolve_option_primitive!(U8),
            WireKind::OptionU16 => resolve_option_primitive!(U16),
            WireKind::OptionU32 => resolve_option_primitive!(U32),
            WireKind::OptionU64 => resolve_option_primitive!(U64),
            WireKind::OptionF16 => resolve_option_primitive!(F16),
            WireKind::OptionF32 => resolve_option_primitive!(F32),
            WireKind::OptionF64 => resolve_option_primitive!(F64),
            WireKind::Timestamp => resolve_primitive!(Timestamp),
            WireKind::OptionTimestamp => resolve_option_primitive!(Timestamp),
            WireKind::Str => {
                if let KindMeta::Primitive { nesting, .. } = inode.kind_meta {
                    let mut ty = PyroType::Str;
                    for _ in 0..nesting {
                        ty = PyroType::List(Bow::new(ty));
                    }
                    ty
                } else {
                    unreachable!()
                }
            }
            WireKind::Bytes => {
                if let KindMeta::Primitive { nesting, .. } = inode.kind_meta {
                    let mut ty = PyroType::Bytes;
                    for _ in 0..nesting {
                        ty = PyroType::List(Bow::new(ty));
                    }
                    ty
                } else {
                    unreachable!()
                }
            }
            WireKind::Map => {
                // Key/value inodes carry their true declared nesting —
                // `push_node`'s `PyroType::Map` arm no longer bumps it,
                // since `layout_in_place` treats a `Map`'s own effective
                // nesting as `nesting + 1` internally instead.
                let (key_type, consumed_k) = {
                    let key_inode = inode_iter.next().ok_or("Missing map key inode")?;
                    Self::parse_type(inode_iter, key_inode)?
                };
                let (value_type, consumed_v) = {
                    let value_inode = inode_iter.next().ok_or("Missing map value inode")?;
                    Self::parse_type(inode_iter, value_inode)?
                };

                let nesting = if let KindMeta::Map { nesting, .. } = inode.kind_meta {
                    nesting
                } else {
                    0
                };

                let mut ty = PyroType::Map {
                    key: Bow::new(key_type),
                    value: Bow::new(value_type),
                };
                for _ in 0..nesting {
                    ty = PyroType::List(Bow::new(ty));
                }

                total_consumed += 2 + consumed_k + consumed_v;
                ty
            }
            t @ (WireKind::Group | WireKind::OptionGroup) => {
                let (count, nesting) =
                    if let KindMeta::Group { count, nesting, .. } = inode.kind_meta {
                        (count as usize, nesting)
                    } else {
                        unreachable!()
                    };
                let mut group_types = Vec::with_capacity(count);
                total_consumed += count;
                for _ in 0..count {
                    let child_inode = inode_iter.next().ok_or("Missing group child inode")?;
                    let (child_type, consumed) = Self::parse_type(inode_iter, child_inode)?;
                    total_consumed += consumed;
                    group_types.push(child_type);
                }
                let mut ty = if matches!(t, WireKind::OptionGroup) {
                    PyroType::OptionGroup(group_types.into())
                } else {
                    PyroType::Group(group_types.into())
                };
                for _ in 0..nesting {
                    ty = PyroType::List(Bow::new(ty));
                }
                ty
            }
        };

        Ok((dt, total_consumed))
    }
}

impl<'a> SchemaParts<'a> {
    /// Flatten a [`PyroSchema`] into its parts: walks `schema.fields()`,
    /// routing each field to [`Self::push_group`] (recursing into a fresh
    /// nested `SchemaParts` for its children) or [`Self::push_field`]
    /// depending on its type. `ChildField` is only used here to read the
    /// already-built `schema` we were handed, not as a general construction
    /// API.
    pub fn from_schema(schema: &'a PyroSchema<'a>) -> Result<Self, &'static str> {
        let mut parts = SchemaParts::new();
        parts.meta = schema.meta();
        parts.schema_documentation = if schema.doc().is_empty() {
            None
        } else {
            Some(schema.doc())
        };
        for field in schema.fields() {
            Self::push_child(&mut parts, field)?;
        }
        Self::layout(parts.meta, &mut parts.inodes);
        Ok(parts)
    }

    /// Push one [`ChildField`] (top-level or nested) onto `parts`, routing
    /// `Group`/`OptionGroup` (bare, or wrapped in any number of `List`s) to
    /// [`Self::push_group`], `Map` to [`Self::push_map`], and everything
    /// else (a `Map` under `List`s included) to [`Self::push_field`].
    fn push_child(parts: &mut Self, field: ChildField<'a>) -> Result<(), &'static str> {
        match field.datatype() {
            PyroType::Map { key, value } => {
                parts.push_map(field.name(), field.doc(), key.as_ref(), value.as_ref())?;
                Self::push_descendant_names(parts, &field);
                Ok(())
            }
            other => {
                if let Some((nesting, nullable, _)) = other.list_group_nesting() {
                    let mut nested = SchemaParts::new();
                    let children = if nesting == 0 {
                        field.group_children()
                    } else {
                        field.list_group_children()
                    };
                    for child in children.ok_or("expected group children")? {
                        Self::push_child(&mut nested, child)?;
                    }
                    parts.push_group(field.name(), field.doc(), nullable, nesting, nested)
                } else {
                    parts.push_field(field.name(), field.doc(), other)?;
                    Self::push_descendant_names(parts, &field);
                    Ok(())
                }
            }
        }
    }

    /// Push the names of every inode below `field` that `push_map`/
    /// `push_field` laid out through `push_node`'s name-less recursion: the
    /// fields of any `Group`/`OptionGroup` inside a `Map`'s key or value,
    /// through any `List`s and `Map`s above or below it.
    ///
    /// A map's key and value roots get no name of their own; every other
    /// inode below `field` gets one, in pre-order — the order `push_node`
    /// lays them out in. That is exactly `field`'s flattened
    /// [`child_names`](ChildField::child_name): a map's key and value add
    /// their own [`name_count`](PyroType::name_count) and no slot for
    /// themselves, a group one slot per child plus that child's count.
    fn push_descendant_names(parts: &mut Self, field: &ChildField<'a>) {
        for i in 0..field.child_names_len() {
            parts.field.push(field.child_name(i));
            parts.documentation.push(field.child_doc(i));
        }
    }

    /// Decode a flat inode forest into one [`PyroType`] per root.
    fn parse_top_level(inodes: &[Inode]) -> Result<Vec<PyroType>, &'static str> {
        let mut inode_iter = inodes.iter().copied();
        let mut top_types = Vec::new();

        let mut consumed = 0;
        let total = inodes.len();
        while consumed < total {
            let inode = inode_iter.next().ok_or("Unexpected end of inodes")?;
            consumed += 1;
            let (ty, n) = Self::parse_type(&mut inode_iter, inode)?;
            consumed += n;
            top_types.push(ty);
        }
        Ok(top_types)
    }

    /// Build a [`PyroSchema`] back out of these parts.
    pub fn to_schema(&self) -> Result<PyroSchema<'a>, &'static str> {
        let top_types = Self::parse_top_level(&self.inodes)?;

        let field_names = FieldNames::Owned(self.field.iter().map(|s| Cow::Borrowed(*s)).collect());
        let field_docs = FieldNames::Owned(
            self.documentation
                .iter()
                .map(|s| Cow::Borrowed(*s))
                .collect(),
        );

        let mut schema = PyroSchema::from_raw(field_names, top_types, self.meta);
        if let Some(doc) = self.schema_documentation {
            schema = schema.with_doc(doc);
        }
        Ok(schema.with_field_docs(field_docs))
    }

    /// Pack into the single-buffer wire format `PyroSchema::decode` reads.
    /// `include_doc` controls whether docstrings (schema-level and
    /// per-field) are packed in at all, mirroring
    /// [`PyroSchema::encode_with_doc`].
    ///
    /// All integers little-endian. The inode table comes first, headed by
    /// its [`InodeMeta`] slot, then the name/doc string table:
    ///
    /// ```text
    /// ┌──────────────────────┬─────────────────────────────────────────────┐
    /// │ u32  n               │ inode count, not counting the meta slot     │
    /// │ [u8; 12]             │ InodeMeta (version, word, reserved)         │
    /// │ [u8; 12] × n         │ Inode × n, pre-order                        │
    /// ├──────────────────────┼─────────────────────────────────────────────┤
    /// │ u32  len             │ byte length of the string table below       │
    /// │ u32  count | HAS_DOCS│ name count; bit 31 set if docs are packed   │
    /// │ u32 × (s + 1)        │ start offset of each string, plus sentinel  │
    /// │ [u8]                 │ UTF-8 string bytes, back to back            │
    /// └──────────────────────┴─────────────────────────────────────────────┘
    /// ```
    ///
    /// The strings (`s` of them) are, without docs, the `count` field names;
    /// with docs, the schema doc followed by `count` interleaved
    /// `name, doc` pairs (`s = 1 + 2·count`). See [`encode_metadata`].
    ///
    /// The `InodeMeta` slot plus the inodes, `12·(n + 1)` bytes, are all a
    /// reader needs and exactly what [`schema_hash`](crate::schema_hash)
    /// covers; the string table only names things.
    pub fn encode(&self, include_doc: bool) -> Vec<u8> {
        let fields_buf = encode_metadata(
            Some(self.schema_documentation.unwrap_or("")),
            &self.field,
            if include_doc {
                Some(&self.documentation[..])
            } else {
                None
            },
        );

        let mut out = Vec::new();

        out.extend_from_slice(&(self.inodes.len() as u32).to_le_bytes());
        out.extend_from_slice(&self.meta.encode());
        for inode in &self.inodes {
            out.extend_from_slice(&inode.encode());
        }

        out.extend_from_slice(&(fields_buf.len() as u32).to_le_bytes());
        out.extend_from_slice(&fields_buf);

        out
    }

    /// Unpack from the single-buffer wire format [`Self::encode`] produces.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, &'static str> {
        if bytes.len() < 4 {
            return Err("Buffer too small to read inode count");
        }

        let num_inodes = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        let mut offset = 4;

        if bytes.len() < offset + INODE_SIZE {
            return Err("Buffer too small for inode meta");
        }
        let meta = InodeMeta::decode(&bytes[offset..offset + INODE_SIZE])
            .map_err(|_| "Failed to decode inode meta")?;
        offset += INODE_SIZE;

        let mut inodes = Vec::with_capacity(num_inodes);
        for _ in 0..num_inodes {
            if bytes.len() < offset + INODE_SIZE {
                return Err("Buffer too small for expected inodes");
            }
            let inode = Inode::decode(&bytes[offset..offset + INODE_SIZE])
                .map_err(|_| "Failed to decode inode")?;
            inodes.push(inode);
            offset += INODE_SIZE;
        }

        if bytes.len() < offset + 4 {
            return Err("Buffer too small to read fields length");
        }

        let fields_len = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        offset += 4;

        if bytes.len() < offset + fields_len {
            return Err("Buffer too small for fields buffer segment");
        }

        let fields_buf = &bytes[offset..offset + fields_len];

        let (has_docs, num_names) =
            decode_metadata_header(fields_buf).map_err(|_| "Failed to decode metadata header")?;

        let mut names = Vec::with_capacity(num_names as usize);
        let mut docs = Vec::with_capacity(num_names as usize);
        let mut schema_doc = "";

        if has_docs {
            schema_doc =
                decode_metadata_string(fields_buf, 0).map_err(|_| "Failed to decode schema doc")?;
            for i in 0..num_names {
                let name = decode_metadata_string(fields_buf, 1 + 2 * i)
                    .map_err(|_| "Failed to decode field name")?;
                let doc = decode_metadata_string(fields_buf, 2 + 2 * i)
                    .map_err(|_| "Failed to decode field doc")?;
                names.push(name);
                docs.push(doc);
            }
        } else {
            for i in 0..num_names {
                let name = decode_metadata_string(fields_buf, i)
                    .map_err(|_| "Failed to decode field name")?;
                names.push(name);
                docs.push("");
            }
        }

        Ok(Self {
            inodes,
            field: names,
            documentation: docs,
            schema_documentation: if schema_doc.is_empty() {
                None
            } else {
                Some(schema_doc)
            },
            meta,
        })
    }
}

impl<'a> PyroSchema<'a> {
    pub fn encode(&self) -> Result<Vec<u8>, &'static str> {
        self.encode_with_doc(false)
    }

    pub fn encode_with_doc(&self, include_doc: bool) -> Result<Vec<u8>, &'static str> {
        Ok(SchemaParts::from_schema(self)?.encode(include_doc))
    }

    pub fn decode(bytes: &'a [u8]) -> Result<Self, &'static str> {
        SchemaParts::decode(bytes)?.to_schema()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Bow, Inode, KindMeta, SchemaParts, WireKind, decode_metadata_header,
        decode_metadata_string, encode_metadata, layout_in_place,
    };
    use crate::types::{PrimitiveDataType, PyroField, PyroSchema, PyroType};

    #[test]
    fn test_inode_builder() {
        let mut builder = Vec::new();
        builder.push(Inode {
            kind: WireKind::I8,
            kind_meta: KindMeta::Primitive {
                nesting: 0,
                data_offset: 0,
                topology_offset: 0,
                fixed_size: 0,
                nullable_bit_index: 0,
                group: false,
                group_nest: false,
            },
        });
        builder.push(Inode {
            kind: WireKind::Group,
            kind_meta: KindMeta::Group {
                count: 0,
                nesting: 0,
                topology_offset: 0,
                nullable_bit_index: 0,
                group: false,
                group_nest: false,
            },
        });

        layout_in_place::<u32>(&mut builder);
        let built = builder;
        assert_eq!(built.len(), 2);
    }

    #[test]
    fn test_metadata_encode_decode() {
        let fields = vec!["block_id", "tx_hash", "amount"];
        let encoded = encode_metadata(None, &fields, None);

        let (has_docs, count) = decode_metadata_header(&encoded).unwrap();
        assert!(!has_docs);
        assert_eq!(count, 3);
        assert_eq!(decode_metadata_string(&encoded, 0).unwrap(), "block_id");
        assert_eq!(decode_metadata_string(&encoded, 1).unwrap(), "tx_hash");
        assert_eq!(decode_metadata_string(&encoded, 2).unwrap(), "amount");

        let docs = vec!["doc1", "doc2", "doc3"];
        let encoded_with = encode_metadata(Some("schema doc"), &fields, Some(&docs));
        let (has_docs, count) = decode_metadata_header(&encoded_with).unwrap();
        assert!(has_docs);
        assert_eq!(count, 3);
        assert_eq!(
            decode_metadata_string(&encoded_with, 0).unwrap(),
            "schema doc"
        );
        assert_eq!(
            decode_metadata_string(&encoded_with, 1).unwrap(),
            "block_id"
        );
        assert_eq!(decode_metadata_string(&encoded_with, 2).unwrap(), "doc1");
    }

    #[test]
    fn test_schema_group_roundtrip() {
        let point_field = PyroField::group(
            "location",
            vec![
                PyroField::new("x", PyroType::PrimitiveScalar(PrimitiveDataType::F32)),
                PyroField::new("y", PyroType::PrimitiveScalar(PrimitiveDataType::F32)),
            ],
        );

        // 1. Define the initial Schema
        let schema = PyroSchema::new(vec![
            point_field,
            PyroField::new("tags", PyroType::PrimitiveList(PrimitiveDataType::U8)),
        ]);

        // 2. Encode to raw bytes
        let encoded = schema.encode().expect("Failed to encode schema");

        // 3. Decode bytes back into a zero-copy schema
        let decoded_schema = PyroSchema::decode(&encoded).expect("Failed to decode wire format");

        // Ensure perfect structural equivalence
        assert_eq!(decoded_schema, schema);
    }

    // Helper function to keep our 10 tests clean and focused
    fn assert_schema_roundtrip(schema: &PyroSchema) {
        let encoded = schema.encode().expect("Failed to encode schema");
        let decoded = PyroSchema::decode(&encoded).expect("Failed to decode schema");
        assert_eq!(schema, &decoded, "Schema roundtrip mismatch");
    }

    #[test]
    fn test_roundtrip_empty_schema() {
        let schema = PyroSchema::new(vec![]);
        assert_schema_roundtrip(&schema);
    }

    #[test]
    fn test_roundtrip_single_primitive() {
        let schema = PyroSchema::new(vec![PyroField::new(
            "is_active",
            PyroType::PrimitiveScalar(PrimitiveDataType::Bool),
        )]);
        assert_schema_roundtrip(&schema);
    }

    #[test]
    fn test_roundtrip_all_primitives() {
        let schema = PyroSchema::new(vec![
            PyroField::new("f1", PyroType::PrimitiveScalar(PrimitiveDataType::I8)),
            PyroField::new("f2", PyroType::PrimitiveScalar(PrimitiveDataType::U16)),
            PyroField::new("f3", PyroType::PrimitiveScalar(PrimitiveDataType::I32)),
            PyroField::new("f4", PyroType::PrimitiveScalar(PrimitiveDataType::F64)),
        ]);
        assert_schema_roundtrip(&schema);
    }

    #[test]
    fn test_roundtrip_all_primitives_nullable() {
        let schema = PyroSchema::new(vec![
            PyroField::new("f1", PyroType::OptionPrimitiveScalar(PrimitiveDataType::I8)),
            PyroField::new(
                "f2",
                PyroType::OptionPrimitiveScalar(PrimitiveDataType::U16),
            ),
            PyroField::new(
                "f3",
                PyroType::OptionPrimitiveScalar(PrimitiveDataType::I32),
            ),
            PyroField::new(
                "f4",
                PyroType::OptionPrimitiveScalar(PrimitiveDataType::F64),
            ),
        ]);
        assert_schema_roundtrip(&schema);
    }

    #[test]
    fn test_roundtrip_strings_and_timestamps() {
        let schema = PyroSchema::new(vec![
            PyroField::new(
                "created_at",
                PyroType::PrimitiveScalar(PrimitiveDataType::Timestamp),
            ),
            PyroField::new("username", PyroType::Str),
            PyroField::new(
                "updated_at",
                PyroType::PrimitiveScalar(PrimitiveDataType::Timestamp),
            ),
        ]);
        assert_schema_roundtrip(&schema);
    }

    #[test]
    fn test_roundtrip_primitive_lists() {
        let schema = PyroSchema::new(vec![
            PyroField::new("byte_data", PyroType::PrimitiveList(PrimitiveDataType::U8)),
            PyroField::new(
                "sensor_readings",
                PyroType::PrimitiveList(PrimitiveDataType::F32),
            ),
        ]);
        assert_schema_roundtrip(&schema);
    }

    #[test]
    fn test_roundtrip_primitive_fixed_lists() {
        let schema = PyroSchema::new(vec![
            PyroField::new(
                "byte_data",
                PyroType::PrimitiveFixedList(PrimitiveDataType::U8, 10),
            ),
            PyroField::new(
                "sensor_readings",
                PyroType::PrimitiveFixedList(PrimitiveDataType::F32, 10),
            ),
        ]);
        assert_schema_roundtrip(&schema);
    }

    #[test]
    fn test_roundtrip_simple_group() {
        let group_field = PyroField::group(
            "location",
            vec![
                PyroField::new("lat", PyroType::PrimitiveScalar(PrimitiveDataType::F64)),
                PyroField::new(
                    "lon",
                    PyroType::OptionPrimitiveScalar(PrimitiveDataType::F64),
                ),
            ],
        );

        let schema = PyroSchema::new(vec![group_field]);
        assert_schema_roundtrip(&schema);
    }

    #[test]
    fn test_roundtrip_nested_groups() {
        let inner_group = PyroField::group(
            "deep",
            vec![PyroField::new(
                "z",
                PyroType::OptionPrimitiveScalar(PrimitiveDataType::I32),
            )],
        );

        let outer_group = PyroField::group(
            "nested_struct",
            vec![
                PyroField::new("x", PyroType::PrimitiveScalar(PrimitiveDataType::I32)),
                PyroField::new("y", PyroType::PrimitiveScalar(PrimitiveDataType::I32)),
                inner_group,
            ],
        );

        let schema = PyroSchema::new(vec![outer_group]);
        assert_schema_roundtrip(&schema);
    }

    #[test]
    fn test_roundtrip_group_with_lists() {
        let group_field = PyroField::group(
            "user_permissions",
            vec![
                PyroField::new("name", PyroType::Str),
                PyroField::new("roles", PyroType::PrimitiveList(PrimitiveDataType::U8)),
            ],
        );

        let schema = PyroSchema::new(vec![group_field]);
        assert_schema_roundtrip(&schema);
    }

    #[test]
    fn test_roundtrip_sibling_groups() {
        let metadata_group = PyroField::group(
            "meta",
            vec![PyroField::new(
                "version",
                PyroType::PrimitiveScalar(PrimitiveDataType::U16),
            )],
        );

        let payload_group = PyroField::group(
            "body",
            vec![PyroField::new(
                "data",
                PyroType::PrimitiveList(PrimitiveDataType::U8),
            )],
        );

        let schema = PyroSchema::new(vec![metadata_group, payload_group]);
        assert_schema_roundtrip(&schema);
    }

    #[test]
    fn test_word_roundtrip() {
        use crate::word::Word;
        for word in [Word::U16, Word::U32] {
            let mut schema = PyroSchema::new(vec![
                PyroField::new("id", PyroType::PrimitiveScalar(PrimitiveDataType::U64)),
                PyroField::new("tags", PyroType::PrimitiveList(PrimitiveDataType::U32)),
            ]);
            schema.set_word(word);
            let bytes = schema.encode().expect("Encode failed");
            let parts = SchemaParts::decode(&bytes).expect("Decode failed");
            assert_eq!(parts.meta.word, word);
            assert_eq!(
                parts.inodes,
                SchemaParts::from_schema(&schema).unwrap().inodes
            );
            assert_eq!(PyroSchema::decode(&bytes).unwrap().word(), word);
        }
    }

    #[test]
    fn test_schema_documentation_roundtrip() {
        let mut schema = PyroSchema::new(vec![
            PyroField::new("f1", PyroType::Str).with_doc("Field 1 doc"),
            PyroField::new(
                "f2",
                PyroType::OptionPrimitiveScalar(PrimitiveDataType::I32),
            )
            .with_doc("Field 2 doc"),
        ]);
        schema = schema.with_doc("Schema level doc");

        // 1. Roundtrip with documentation
        let encoded_with = schema.encode_with_doc(true).expect("Encode failed");
        let decoded_with = PyroSchema::decode(&encoded_with).expect("Decode failed");
        assert_eq!(schema.doc(), decoded_with.doc());
        assert_eq!(schema.num_fields(), decoded_with.num_fields());
        assert_eq!(decoded_with.doc(), "Schema level doc");
        assert_eq!(decoded_with.field(0).doc(), "Field 1 doc");

        // 2. Roundtrip without documentation
        let encoded_without = schema.encode_with_doc(false).expect("Encode failed");
        let decoded_without = PyroSchema::decode(&encoded_without).expect("Decode failed");
        // fields should match in name/type/nullability but documentation should be empty
        assert_eq!(decoded_without.num_fields(), schema.num_fields());
        assert_eq!(decoded_without.field(0).name(), schema.field(0).name());
        assert_eq!(decoded_without.field(0).doc(), "");
        assert_eq!(decoded_without.doc(), "");
    }

    #[test]
    fn test_schema_parts_roundtrip() {
        let mut schema = PyroSchema::new(vec![
            PyroField::new("f1", PyroType::Str).with_doc("Field 1 doc"),
            PyroField::group(
                "profile",
                vec![
                    PyroField::new("street", PyroType::Str).with_doc("Street line"),
                    PyroField::new(
                        "age",
                        PyroType::OptionPrimitiveScalar(PrimitiveDataType::I32),
                    ),
                ],
            ),
        ]);
        schema = schema.with_doc("Schema level doc");

        let parts = SchemaParts::from_schema(&schema).expect("from_schema failed");
        let rebuilt = parts.to_schema().expect("to_schema failed");

        assert_eq!(rebuilt.doc(), "Schema level doc");
        assert_eq!(rebuilt.num_fields(), schema.num_fields());
        assert_eq!(rebuilt.field(0).name(), "f1");
        assert_eq!(rebuilt.field(0).doc(), "Field 1 doc");
        assert_eq!(rebuilt.field(1).name(), "profile");
        assert_eq!(rebuilt.field(1).child_name(0), "street");
        assert_eq!(rebuilt.field(1).child_doc(0), "Street line");
        assert_eq!(rebuilt.field(1).datatype(), schema.field(1).datatype());
    }

    #[test]
    fn test_nested_documentation_roundtrip() {
        let group = PyroField::group(
            "profile",
            vec![
                PyroField::new("street", PyroType::Str).with_doc("Street line"),
                PyroField::group(
                    "geo",
                    vec![
                        PyroField::new("lat", PyroType::PrimitiveScalar(PrimitiveDataType::F64))
                            .with_doc("Latitude"),
                        PyroField::new("lon", PyroType::PrimitiveScalar(PrimitiveDataType::F64)),
                    ],
                )
                .with_doc("Coordinates"),
            ],
        )
        .with_doc("The profile group");

        let schema = PyroSchema::new(vec![
            PyroField::new("id", PyroType::PrimitiveScalar(PrimitiveDataType::U64))
                .with_doc("Row id"),
            group,
        ])
        .with_doc("Top level");

        let encoded = schema.encode_with_doc(true).expect("encode");
        let decoded = PyroSchema::decode(&encoded).expect("decode");

        assert_eq!(decoded.doc(), "Top level");
        assert_eq!(decoded.names(), schema.names());
        assert_eq!(decoded.field_docs(), schema.field_docs());

        let profile = decoded.field(1);
        assert_eq!(profile.doc(), "The profile group");
        let mut kids = profile.group_children().expect("Group");
        assert_eq!(kids.next().unwrap().doc(), "Street line");
        let geo = kids.next().unwrap();
        assert_eq!(geo.doc(), "Coordinates");
        let mut geo_kids = geo.group_children().expect("Group");
        assert_eq!(geo_kids.next().unwrap().doc(), "Latitude");
        assert_eq!(geo_kids.next().unwrap().doc(), "");
    }

    /// `SchemaParts::from_schema` records each field's name and doc
    /// alongside its inodes, so `to_schema()` realizes the *documented*
    /// schema rather than an anonymous one — the same thing
    /// `PyroSchema::decode` gets from the wire metadata string table.
    #[test]
    fn test_schema_builder_push_realizes_names_and_docs() {
        let schema = PyroSchema::new(vec![
            PyroField::new("id", PyroType::PrimitiveScalar(PrimitiveDataType::U64))
                .with_doc("Row id"),
            PyroField::group(
                "addr",
                vec![
                    PyroField::new("street", PyroType::Str).with_doc("Street line"),
                    PyroField::new("zip", PyroType::PrimitiveScalar(PrimitiveDataType::U32)),
                ],
            )
            .with_doc("Where they live"),
        ])
        .with_doc("Top level");

        let parts = SchemaParts::from_schema(&schema).expect("from_schema");
        let realized = parts.to_schema().expect("to_schema");
        assert_eq!(realized.doc(), "Top level");
        assert_eq!(realized.names(), schema.names());
        assert_eq!(realized.field_docs(), schema.field_docs());
        assert_eq!(realized.types(), schema.types());
    }

    #[test]
    fn test_roundtrip_realistic_user_profile() {
        // A complex schema modeling a realistic application payload
        let address_group = PyroField::group(
            "address",
            vec![
                PyroField::new("street", PyroType::Str),
                PyroField::new(
                    "zip",
                    PyroType::OptionPrimitiveScalar(PrimitiveDataType::U32),
                ),
            ],
        );

        let schema = PyroSchema::new(vec![
            PyroField::new("id", PyroType::PrimitiveScalar(PrimitiveDataType::U64)),
            PyroField::new("username", PyroType::Str),
            PyroField::new(
                "is_verified",
                PyroType::PrimitiveScalar(PrimitiveDataType::Bool),
            ),
            PyroField::new(
                "created_timestamp",
                PyroType::PrimitiveScalar(PrimitiveDataType::Timestamp),
            ),
            address_group,
            PyroField::new(
                "account_flags",
                PyroType::PrimitiveList(PrimitiveDataType::U8),
            ),
        ]);

        assert_schema_roundtrip(&schema);
    }

    #[test]
    fn test_roundtrip_complex_maps_and_generic_lists() {
        let audit_log_type = PyroType::List(Bow::new(PyroType::Group(
            vec![
                PyroType::PrimitiveScalar(PrimitiveDataType::Timestamp),
                PyroType::Str,
            ]
            .into(),
        )));

        let schema = PyroSchema::new(vec![
            // 1. Generic List of Strings
            PyroField::new("aliases", PyroType::List(Bow::new(PyroType::Str))),
            // 2. Map of String -> Primitive Fixed List
            PyroField::new(
                "vector_map",
                PyroType::Map {
                    key: Bow::new(PyroType::Str),
                    value: Bow::new(PyroType::PrimitiveFixedList(PrimitiveDataType::F32, 128)),
                },
            ),
            // 3. Generic List of Groups - we MUST use with_children to provide the inner Group's field names
            PyroField::with_children(
                "audit_log",
                crate::FieldNames::Owned(vec!["timestamp".into(), "action".into()]),
                audit_log_type,
            ),
        ]);

        assert_schema_roundtrip(&schema);
    }

    /// `fixed_size: 0` is "not a fixed array", so a zero-length one has no
    /// inode: required or optional, at the root or under a list.
    #[test]
    fn test_zero_length_fixed_list_rejected() {
        for ty in [
            PyroType::PrimitiveFixedList(PrimitiveDataType::U8, 0),
            PyroType::OptionPrimitiveFixedList(PrimitiveDataType::U8, 0),
            PyroType::List(Bow::new(PyroType::PrimitiveFixedList(
                PrimitiveDataType::U8,
                0,
            ))),
            PyroType::List(Bow::new(PyroType::OptionPrimitiveFixedList(
                PrimitiveDataType::U8,
                0,
            ))),
        ] {
            let schema = PyroSchema::new(vec![PyroField::new("a", ty)]);
            assert!(SchemaParts::from_schema(&schema).is_err());
        }
    }

    /// The names `RowReader::from_parts` hands out: one per inode, in
    /// pre-order, except a `Map`'s key and value roots.
    fn named_inode_count(inodes: &[Inode]) -> usize {
        // (is a map, children left)
        let mut stack: Vec<(bool, usize)> = Vec::new();
        let mut named = 0;
        for inode in inodes {
            if !stack.last().is_some_and(|&(map, _)| map) {
                named += 1;
            }
            if let Some((_, left)) = stack.last_mut() {
                *left -= 1;
            }
            let children = match inode.kind {
                WireKind::Group | WireKind::OptionGroup => inode.group_count() as usize,
                WireKind::Map => 2,
                _ => 0,
            };
            if children > 0 {
                stack.push((inode.kind == WireKind::Map, children));
            }
            while let Some(&(_, 0)) = stack.last() {
                stack.pop();
            }
        }
        named
    }

    fn names(names: &[&'static str]) -> crate::FieldNames<'static> {
        crate::FieldNames::Owned(names.iter().map(|&n| n.into()).collect())
    }

    fn u8_ty() -> PyroType {
        PyroType::PrimitiveScalar(PrimitiveDataType::U8)
    }

    fn group(children: Vec<PyroType>) -> PyroType {
        PyroType::Group(children.into())
    }

    fn list(inner: PyroType) -> PyroType {
        PyroType::List(Bow::new(inner))
    }

    fn map(key: PyroType, value: PyroType) -> PyroType {
        PyroType::Map {
            key: Bow::new(key),
            value: Bow::new(value),
        }
    }

    /// `from_schema` names every group field reachable through a map, under
    /// any mix of `List`s, `Map`s and `Group`s, in the order the inodes are
    /// laid out.
    #[test]
    fn test_map_group_names_at_any_nesting() {
        let cases: Vec<(PyroField, Vec<&str>)> = vec![
            // A struct-keyed map, one list down.
            (
                PyroField::with_children(
                    "a",
                    names(&["a", "s"]),
                    list(map(group(vec![u8_ty(), PyroType::Str]), u8_ty())),
                ),
                vec!["a", "a", "s"],
            ),
            // A struct-valued map, two lists down.
            (
                PyroField::with_children(
                    "m",
                    names(&["x", "y"]),
                    list(list(map(u8_ty(), group(vec![u8_ty(), u8_ty()])))),
                ),
                vec!["m", "x", "y"],
            ),
            // A map as a map's value, with struct key and value.
            (
                PyroField::with_children(
                    "m",
                    names(&["k", "v"]),
                    map(u8_ty(), map(group(vec![u8_ty()]), group(vec![u8_ty()]))),
                ),
                vec!["m", "k", "v"],
            ),
            // A listed map as a map's key.
            (
                PyroField::with_children(
                    "m",
                    names(&["z"]),
                    map(list(map(u8_ty(), group(vec![u8_ty()]))), u8_ty()),
                ),
                vec!["m", "z"],
            ),
            // A listed map as a group's field, holding a listed group.
            (
                PyroField::group(
                    "g",
                    vec![PyroField::with_children(
                        "inner",
                        names(&["a", "b"]),
                        list(map(group(vec![u8_ty()]), list(group(vec![u8_ty()])))),
                    )],
                ),
                vec!["g", "inner", "a", "b"],
            ),
        ];

        for (field, expected) in cases {
            let schema = PyroSchema::new(vec![field]);
            let parts = SchemaParts::from_schema(&schema).unwrap();
            assert_eq!(parts.field, expected, "{:?}", schema);
            assert_eq!(parts.field.len(), parts.documentation.len());
            assert_eq!(parts.field.len(), named_inode_count(&parts.inodes));
        }
    }
}
