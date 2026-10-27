use pyro_schema::{
    FieldName, Inode, InodeMeta, PyroSchema, SchemaParts, StructureWord, WireKind, Word,
    inode_data_len,
};

use crate::wire::IoType;
use crate::{PyroRow, PyroValue, errors::WireError, schema::PyroType, value::RowItem};

mod interface;
pub mod scalars;

pub mod group;
pub mod map;

use group::{GroupOptionReader, GroupReader};
pub use interface::SplitSource;
use map::MapReader;
use scalars::Leaf;

/// Decodes a single field from a wire buffer into a [`PyroValue`].
pub trait ValueReader {
    /// Decode this field from `buffer`, returning a [`PyroValue`].
    ///
    /// For nullable readers, an absent value yields `PyroValue::Null`. For
    /// required readers, every bit pattern is a valid value.
    fn read_value<'a, W: StructureWord>(
        &self,
        buffer: &SplitSource<'a, W>,
    ) -> Result<PyroValue<'a>, WireError>;
}

pub trait ValueNestedReader {
    /// Read all the values for a column: one per row of the repeated group
    /// (or entry of the map) this field sits in.
    fn read_values<'a, W: StructureWord>(
        &self,
        buffer: &SplitSource<'a, W>,
    ) -> Result<Vec<PyroValue<'a>>, WireError>;
}

/// One field's reader, wherever it sits: at the root, in a flat group, as
/// a repeated group's column or as a map's keys or values. Which of
/// [`ValueReader::read_value`] (one value) and
/// [`ValueNestedReader::read_values`] (a column) it is read with is up to
/// its parent; every reader's own `depth` covers both.
#[derive(Debug, Clone)]
pub enum Reader {
    Leaf(Leaf),
    Group(GroupReader),
    OptionGroup(GroupOptionReader),
    Map(Box<MapReader>),
}

impl Reader {
    pub fn datatype(&self) -> PyroType {
        match self {
            Reader::Leaf(r) => IoType::datatype(r),
            Reader::Group(r) => r.datatype(),
            Reader::OptionGroup(r) => r.datatype(),
            Reader::Map(r) => r.datatype(),
        }
    }
}

impl ValueReader for Reader {
    fn read_value<'a, W: StructureWord>(
        &self,
        buffer: &SplitSource<'a, W>,
    ) -> Result<PyroValue<'a>, WireError> {
        match self {
            Reader::Leaf(r) => r.read_value(buffer),
            Reader::Group(r) => r.read_value(buffer),
            Reader::OptionGroup(r) => r.read_value(buffer),
            Reader::Map(r) => r.read_value(buffer),
        }
    }
}

impl ValueNestedReader for Reader {
    fn read_values<'a, W: StructureWord>(
        &self,
        buffer: &SplitSource<'a, W>,
    ) -> Result<Vec<PyroValue<'a>>, WireError> {
        match self {
            Reader::Leaf(r) => r.read_values(buffer),
            Reader::Group(r) => r.read_values(buffer),
            Reader::OptionGroup(r) => r.read_values(buffer),
            Reader::Map(r) => r.read_values(buffer),
        }
    }
}

/// The leaf reader for inode `n`: its kind, its fixed-size array length
/// (`0` for none) and its `nesting`, the `Vec`s around the value.
fn leaf(n: usize, inode: &Inode) -> Result<Leaf, WireError> {
    let depth = inode.nesting() as usize;
    let k = inode.kind_meta.fixed_size() as usize;
    macro_rules! leaf_kinds {
        ($($Kind:ident),* $(,)?) => {
            paste::paste! {
                match (inode.kind, k) {
                    (WireKind::Str, 0) => Leaf::Str(n, depth),
                    (WireKind::Bytes, 0) => Leaf::Bytes(n, depth),
                    $(
                        (WireKind::$Kind, 0) => Leaf::$Kind(n, depth),
                        (WireKind::$Kind, k) => Leaf::[<$Kind Fixed>](k, n, depth),
                        (WireKind::[<Option $Kind>], 0) => Leaf::[<Option $Kind>](n, depth),
                        (WireKind::[<Option $Kind>], k) => Leaf::[<$Kind FixedOption>](k, n, depth),
                    )*
                    _ => return Err(WireError::InvalidInodes),
                }
            }
        };
    }
    Ok(leaf_kinds!(
        Bool, I8, I16, I32, I64, U8, U16, U32, U64, F16, F32, F64, Timestamp
    ))
}

// =============================================================================
// RowReader — the schema-driven orchestrator
// =============================================================================

/// The read-side twin of [`crate::wire::value_writers::ValueWriter`]: turns a
/// [`PyroSchema`] into a tree of [`Reader`]s once, then decodes any number of
/// wire buffers written against that same schema.
///
/// Named `RowReader` rather than `ValueReader` because the latter is the
/// per-field trait in this module. The readers don't depend on the
/// message's structure word; it is only picked, from [`InodeMeta::word`],
/// when a buffer is read.
#[derive(Debug, Clone)]
pub struct RowReader {
    inode_meta: InodeMeta,
    inodes: Vec<Inode>,
    fixed_len: usize,
    pyro_type: PyroType,
    items: Vec<(String, Reader)>,
}

impl RowReader {
    /// Flatten the schema to inodes, walk them into a parent/child tree
    /// (pre-order), then build readers bottom-up (post-order), trusting each
    /// inode's own `nesting()`/`group_nest()` bits for depth.
    pub fn new(schema: &PyroSchema) -> Result<Self, WireError> {
        let parts = SchemaParts::from_schema(schema).map_err(|_| WireError::InvalidInodes)?;
        Self::from_parts(parts.meta, parts.inodes, &parts.field)
    }

    /// The `(meta, inodes, field_names)` entry point — for callers that
    /// already hold a decoded inode set (e.g. a message header) rather than a
    /// `PyroSchema`. The inodes alone don't say how to read a message:
    /// `meta` carries the structure word they were laid out in.
    pub fn from_parts(
        inode_meta: InodeMeta,
        inodes: Vec<Inode>,
        field_names: &[&str],
    ) -> Result<Self, WireError> {
        if inodes.is_empty() {
            return Err(WireError::InvalidInodes);
        }

        // ------------------------------------------------------------------
        // PASS 1: parent/child tree + field names (pre-order, non-recursive)
        // ------------------------------------------------------------------
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); inodes.len()];
        let mut names: Vec<Option<String>> = vec![None; inodes.len()];
        // (parent_idx, remaining children to consume)
        let mut stack: Vec<(usize, usize)> = Vec::new();
        let mut roots: Vec<usize> = Vec::new();
        // `field_names` is 1:1 with `inodes` in this same pre-order walk,
        // except it skips `Map` children entirely — see
        // `SchemaParts::push_field`/`push_group`/`push_map`.
        let mut field_cursor = 0usize;

        for (i, inode) in inodes.iter().enumerate() {
            let parent = stack.last().map(|&(parent, _)| parent);
            if parent.is_none_or(|parent| inodes[parent].kind != WireKind::Map) {
                names[i] = field_names.get(field_cursor).map(|s| s.to_string());
                field_cursor += 1;
            }
            match stack.last_mut() {
                Some((parent, remaining)) => {
                    children[*parent].push(i);
                    *remaining -= 1;
                }
                None => roots.push(i),
            }

            let num_children = match inode.kind {
                WireKind::Group | WireKind::OptionGroup => inode.group_count() as usize,
                WireKind::Map => 2,
                _ => 0,
            };
            if num_children > 0 {
                stack.push((i, num_children));
            }
            while let Some(&(_, 0)) = stack.last() {
                stack.pop();
            }
        }

        // ------------------------------------------------------------------
        // PASS 2: build readers bottom-up (post-order)
        // ------------------------------------------------------------------
        let mut readers: Vec<Option<Reader>> = (0..inodes.len()).map(|_| None).collect();
        // Each reader is taken exactly once, by its parent (or as a root).
        fn take(readers: &mut [Option<Reader>], c: usize) -> Result<Reader, WireError> {
            readers[c].take().ok_or(WireError::InvalidInodes)
        }

        for i in (0..inodes.len()).rev() {
            let inode = &inodes[i];
            // A group's or map's own level counts too: `nesting +
            // group_nest`, with a flat group (`nesting == 0`, not
            // group-nested) at `0`.
            let depth = inode.nesting() as usize + inode.group_nest() as usize;
            let reader = match inode.kind {
                WireKind::Unknown => return Err(WireError::InvalidInodes),
                WireKind::Group | WireKind::OptionGroup => {
                    let mut elements = Vec::with_capacity(children[i].len());
                    for &c in &children[i] {
                        let name = FieldName::Owned(names[c].clone().unwrap_or_default());
                        elements.push((name, take(&mut readers, c)?));
                    }
                    if inode.kind == WireKind::OptionGroup {
                        Reader::OptionGroup(GroupOptionReader::new(i, depth, elements))
                    } else {
                        Reader::Group(GroupReader::new(i, depth, elements))
                    }
                }
                WireKind::Map => {
                    let [key, value] = children[i][..] else {
                        return Err(WireError::InvalidInodes);
                    };
                    let (key, value) = (take(&mut readers, key)?, take(&mut readers, value)?);
                    Reader::Map(Box::new(MapReader::new(i, depth, key, value)))
                }
                // Every other kind is a leaf.
                _ => Reader::Leaf(leaf(i, inode)?),
            };
            readers[i] = Some(reader);
        }

        let items = roots
            .iter()
            .map(|&r| Ok((names[r].clone().unwrap_or_default(), take(&mut readers, r)?)))
            .collect::<Result<Vec<_>, WireError>>()?;
        let pyro_type = PyroType::Group(items.iter().map(|(_, r)| r.datatype()).collect());
        let fixed_len = match inode_meta.word {
            Word::U16 => inode_data_len::<u16>(&inodes),
            Word::U32 => inode_data_len::<u32>(&inodes),
        };
        Ok(Self {
            inode_meta,
            inodes,
            fixed_len,
            pyro_type,
            items,
        })
    }

    /// The flattened inode set this reader was built from — offsets come
    /// from here and are never re-derived.
    pub fn inodes(&self) -> &[Inode] {
        &self.inodes
    }

    /// The message-wide settings (the structure word) that go with
    /// [`Self::inodes`].
    pub fn inode_meta(&self) -> InodeMeta {
        self.inode_meta
    }

    /// The row type this reader decodes to.
    pub fn datatype(&self) -> &PyroType {
        &self.pyro_type
    }

    /// The top-level field names, in wire order.
    pub fn field_names(&self) -> impl Iterator<Item = &str> {
        self.items.iter().map(|(n, _)| n.as_str())
    }

    /// Decode one whole row.
    ///
    /// Mirrors `ValueWriter::ingest`'s single-vs-multi split: a schema with
    /// exactly one top-level field yields that field's value directly,
    /// anything else yields a [`PyroValue::Group`] keyed by field name.
    pub fn read_value<'a>(&'a self, view: &'a [u8]) -> Result<PyroValue<'a>, WireError> {
        match self.items.len() {
            1 => match self.inode_meta.word {
                Word::U16 => self.items[0].1.read_value(&self.source::<u16>(view)?),
                Word::U32 => self.items[0].1.read_value(&self.source::<u32>(view)?),
            },
            _ => Ok(PyroValue::Group(self.read_row(view)?)),
        }
    }

    /// Decode one whole row, always as a name-keyed [`PyroRow`].
    pub fn read_row<'a>(&'a self, view: &'a [u8]) -> Result<PyroRow<'a>, WireError> {
        match self.inode_meta.word {
            Word::U16 => self.row(&self.source::<u16>(view)?),
            Word::U32 => self.row(&self.source::<u32>(view)?),
        }
    }

    fn source<'a, W: StructureWord>(
        &'a self,
        view: &'a [u8],
    ) -> Result<SplitSource<'a, W>, WireError> {
        SplitSource::new(view, self.fixed_len, &self.inodes)
    }

    fn row<'a, W: StructureWord>(
        &self,
        buffer: &SplitSource<'a, W>,
    ) -> Result<PyroRow<'a>, WireError> {
        let items = self
            .items
            .iter()
            .map(|(name, reader)| {
                Ok(RowItem {
                    key: FieldName::Owned(name.clone()),
                    value: reader.read_value(buffer)?,
                })
            })
            .collect::<Result<Vec<_>, WireError>>()?;
        Ok(PyroRow(items))
    }
}
