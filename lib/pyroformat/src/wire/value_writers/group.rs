//! Group writers.
//!
//! A group has no data of its own: its children are writers in their own
//! right, constructed with their own offsets from their inodes. A group only
//! routes each field's value to the right child and, when it is repeated,
//! records how many groups there were.
//!
//! # Root groups (`Group`, `OptionGroup`)
//!
//! A nesting-0 field outside any list. Children are [`RootWriter`]s and are
//! ingested once. `Group` writes nothing of its own (`GROUP_PAIR_BOOL_U16` is
//! just the two fields); `OptionGroup` additionally owns one bit in the fixed
//! section's validity bitmap, same as a nullable scalar's own bit.
//!
//! # Repeated groups (`NestedGroup`, `NestedOptionGroup`)
//!
//! `List<Group>` and deeper, or a group inside another group or a map.
//! Children are [`NestedWriter`]s and are `feed` once per group instance.
//! Cumulative levels record the shape exactly as the primitives do, one group
//! being the unit:
//!
//! * `NestedGroup` at the root with no levels (`List<Group>`) writes a bare
//!   one-word group count in its slot (`LIST_GROUP`, `write_count`). With
//!   levels it writes the `(start, words)` header and the sentinel-prefixed
//!   levels (`DOUBLE_LIST_GROUP_TWO_U8_FIELDS`, `write_topology`). A depth-0
//!   child (a field of a repeated group, a map's key or value) writes its
//!   own one-word instance count too (`LIST_MAP_GROUP_KEY_U8_VALUE`): its
//!   parent's slot can be a topology pointer rather than a count.
//! * `NestedOptionGroup` with no levels (`List<Option<Group>>`) writes the
//!   validity mask, one bit per group, and a `(start, groups)` header
//!   (`LIST_OPT_GROUP`, `EMPTY_LIST_GROUP_OPTION`, `write_group_mask`). With
//!   levels it writes the levels, then the mask, and a `(start, words)`
//!   header (`DOUBLE_LIST_GROUP_OPTION`, `write_topology_option`). An absent
//!   group takes a bit and a topology slot but feeds nothing to its
//!   children.
//!
//! Payload is written in alignment passes (`write_16` .. `write_1`), each pass
//! forwarded to every child in field order — the same shape `Map` uses for
//! its key/value children.

use bitvec::order::Lsb0;
use bitvec::vec::BitVec;
use pyro_schema::{Bow, StructureWord};
use tracing::trace;

use crate::errors::WireError;
use crate::schema::PyroType;
use crate::value::PyroValue;
use crate::wire::{
    IoType,
    value_writers::{
        NestedWriter, RootWriter,
        interface::{Padding, SplitSink, TopologyCache},
    },
};
use crate::{MAX_TIERS, PyroRow};

pub(super) static NULL: PyroValue<'static> = PyroValue::Null;

/// `t` wrapped in `lists` levels of `List<..>`, for error messages.
fn nested(mut t: PyroType, lists: usize) -> PyroType {
    for _ in 0..lists {
        t = PyroType::List(Bow::new(t));
    }
    t
}

/// The row behind a group value, or a type mismatch.
fn row_of<'b>(val: &'b PyroValue<'b>, expected: &PyroType) -> Result<&'b PyroRow<'b>, WireError> {
    match val {
        PyroValue::Group(row) => Ok(row),
        _ => Err(WireError::TypeMismatch {
            expected: expected.clone(),
            actual: val.datatype(),
        }),
    }
}

// =============================================================================
// Group — nesting 0, outside any list
// =============================================================================

#[derive(Debug, Clone)]
pub struct Group<'b, W: StructureWord> {
    datatype: PyroType,
    items: Vec<(String, RootWriter<'b, W>)>,
}

impl<'b, W: StructureWord> Group<'b, W> {
    /// `datatype` is the already-known `PyroType::Group(..)` for this field
    /// (from the schema); it is not derived from the children, since a fresh
    /// child writer does not yet hold a value to introspect.
    pub fn new(datatype: PyroType, items: Vec<(String, RootWriter<'b, W>)>) -> Self {
        trace!(fields = items.len(), "Creating Group");
        Self { datatype, items }
    }

    #[inline]
    pub fn items(&self) -> &[(String, RootWriter<'b, W>)] {
        &self.items
    }

    /// Route each field to its child. A `Null` group is an absent one.
    pub fn ingest(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
        let row = row_of(val, &self.datatype)?;
        for (name, w) in &mut self.items {
            w.ingest(row.get(name).unwrap_or(&NULL))?;
        }
        Ok(())
    }

    pub fn variable_length(&self) -> usize {
        self.items.iter().map(|(_, w)| w.variable_length()).sum()
    }

    pub fn write_structure(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        for (_, w) in &self.items {
            w.write_structure(buffer)?;
        }
        Ok(())
    }

    pub fn padding_structure(&self, padding: &mut Padding) {
        for (_, w) in &self.items {
            w.padding_structure(padding);
        }
    }

    pub fn padding_data(&self, pass: usize, padding: &mut Padding) {
        for (_, w) in &self.items {
            w.padding_data(pass, padding);
        }
    }

    pub fn write_16(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        for (_, w) in &self.items {
            w.write_16(buffer)?;
        }
        Ok(())
    }

    pub fn write_8(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        for (_, w) in &self.items {
            w.write_8(buffer)?;
        }
        Ok(())
    }

    pub fn write_4(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        for (_, w) in &self.items {
            w.write_4(buffer)?;
        }
        Ok(())
    }

    pub fn write_2(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        for (_, w) in &self.items {
            w.write_2(buffer)?;
        }
        Ok(())
    }

    pub fn write_1(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        for (_, w) in &self.items {
            w.write_1(buffer)?;
        }
        Ok(())
    }

    pub fn clear(self) -> Group<'static, W> {
        Group {
            datatype: self.datatype,
            items: self
                .items
                .into_iter()
                .map(|(n, w)| (n, w.clear()))
                .collect(),
        }
    }
}

impl<'b, W: StructureWord> IoType for Group<'b, W> {
    fn datatype(&self) -> PyroType {
        self.datatype.clone()
    }
}

// =============================================================================
// OptionGroup — nesting 0, outside any list, nullable as a whole
// =============================================================================

#[derive(Debug, Clone)]
pub struct OptionGroup<'b, W: StructureWord> {
    datatype: PyroType,
    /// The group's inode, which owns its bit in the fixed region's bitmap.
    n: usize,
    present: bool,
    items: Vec<(String, RootWriter<'b, W>)>,
}

impl<'b, W: StructureWord> OptionGroup<'b, W> {
    /// `datatype` is the already-known `PyroType::OptionGroup(..)` for this
    /// field, same reasoning as [`Group::new`].
    pub fn new(datatype: PyroType, n: usize, items: Vec<(String, RootWriter<'b, W>)>) -> Self {
        trace!(n, fields = items.len(), "Creating OptionGroup");
        Self {
            datatype,
            n,
            present: false,
            items,
        }
    }

    #[inline]
    pub fn is_null(&self) -> bool {
        !self.present
    }

    #[inline]
    pub fn items(&self) -> &[(String, RootWriter<'b, W>)] {
        &self.items
    }

    /// A `Null` clears the group: the bit stays clear and every child is
    /// absent (see [`RootWriter::ingest_absent`]).
    pub fn ingest(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
        if val.is_null() {
            return Ok(());
        }
        let row = row_of(val, &self.datatype)?;
        self.present = true;
        for (name, w) in &mut self.items {
            w.ingest(row.get(name).unwrap_or(&NULL))?;
        }
        Ok(())
    }

    pub fn variable_length(&self) -> usize {
        self.items.iter().map(|(_, w)| w.variable_length()).sum()
    }

    /// This group's own bit in the fixed region's bitmap when present (a
    /// null leaves it clear), then every child's structure.
    pub fn write_structure(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        if self.present {
            buffer.write_valid(self.n);
        }
        for (_, w) in &self.items {
            w.write_structure(buffer)?;
        }
        Ok(())
    }

    pub fn padding_structure(&self, padding: &mut Padding) {
        for (_, w) in &self.items {
            w.padding_structure(padding);
        }
    }

    pub fn padding_data(&self, pass: usize, padding: &mut Padding) {
        for (_, w) in &self.items {
            w.padding_data(pass, padding);
        }
    }

    pub fn write_16(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        for (_, w) in &self.items {
            w.write_16(buffer)?;
        }
        Ok(())
    }

    pub fn write_8(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        for (_, w) in &self.items {
            w.write_8(buffer)?;
        }
        Ok(())
    }

    pub fn write_4(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        for (_, w) in &self.items {
            w.write_4(buffer)?;
        }
        Ok(())
    }

    pub fn write_2(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        for (_, w) in &self.items {
            w.write_2(buffer)?;
        }
        Ok(())
    }

    pub fn write_1(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        for (_, w) in &self.items {
            w.write_1(buffer)?;
        }
        Ok(())
    }

    pub fn clear(self) -> OptionGroup<'static, W> {
        OptionGroup {
            datatype: self.datatype,
            n: self.n,
            present: false,
            items: self
                .items
                .into_iter()
                .map(|(n, w)| (n, w.clear()))
                .collect(),
        }
    }
}

impl<'b, W: StructureWord> IoType for OptionGroup<'b, W> {
    fn datatype(&self) -> PyroType {
        self.datatype.clone()
    }
}

// =============================================================================
// NestedGroup — repeated group, children fed once per instance
// =============================================================================

#[derive(Debug, Clone)]
pub struct NestedGroup<'b, W: StructureWord> {
    /// The group's inode, which owns its count or topology slot.
    n: usize,
    depth: usize,
    /// Groups seen.
    count: usize,
    topology: TopologyCache<W>,
    datatype: PyroType,
    items: Vec<(String, NestedWriter<'b, W>)>,
}

impl<'b, W: StructureWord> NestedGroup<'b, W> {
    /// `depth` = `nesting - 1 + group_nest`.
    pub fn new(n: usize, depth: usize, items: Vec<(String, NestedWriter<'b, W>)>) -> Self {
        assert!(
            depth <= MAX_TIERS,
            "nesting depth {depth} exceeds the value path's maximum of {MAX_TIERS}"
        );
        trace!(n, depth, fields = items.len(), "Creating NestedGroup");
        let datatype = PyroType::Group(items.iter().map(|(_, w)| w.datatype()).collect());
        Self {
            n,
            depth,
            count: 0,
            topology: TopologyCache::new(),
            datatype,
            items,
        }
    }

    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    #[inline]
    pub fn num_groups(&self) -> usize {
        self.count
    }

    #[inline]
    pub fn topology(&self) -> &[Vec<W>] {
        &self.topology.levels()[..self.depth]
    }

    #[inline]
    pub fn items(&self) -> &[(String, NestedWriter<'b, W>)] {
        &self.items
    }

    /// One group instance: feed every child its field. A `Null` group feeds
    /// every child a `Null`.
    fn group(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
        let row = if val.is_null() {
            None
        } else {
            Some(row_of(val, &self.datatype)?)
        };
        for (name, w) in &mut self.items {
            w.feed(row.and_then(|r| r.get(name)).unwrap_or(&NULL))?;
        }
        self.count += 1;
        Ok(())
    }

    /// The sample-level container: a `List` of groups, or `Null`. Returns
    /// how many groups it held.
    fn groups(&mut self, val: &'b PyroValue<'b>) -> Result<usize, WireError> {
        match val {
            PyroValue::List(items) => {
                for item in items {
                    self.group(item)?;
                }
                Ok(items.len())
            }
            PyroValue::Null => Ok(0),
            _ => Err(WireError::TypeMismatch {
                expected: nested(self.datatype.clone(), 1),
                actual: val.datatype(),
            }),
        }
    }

    fn walk(&mut self, val: &'b PyroValue<'b>, level: usize) -> Result<(), WireError> {
        if self.depth() == 0 {
            return self.group(val);
        }
        let n: usize = if level + 1 == self.depth() {
            self.groups(val)?
        } else {
            match val {
                PyroValue::List(items) => {
                    for item in items {
                        self.walk(item, level + 1)?;
                    }
                    items.len()
                }
                PyroValue::Null => 0,
                _ => {
                    return Err(WireError::TypeMismatch {
                        expected: nested(self.datatype.clone(), self.depth() - level),
                        actual: val.datatype(),
                    });
                }
            }
        };
        self.topology.bound(level, n);
        Ok(())
    }

    /// One group instance from an enclosing group or map; with levels, a
    /// list of them.
    pub fn feed(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
        trace!(depth = self.depth(), ?val, "feed");
        self.walk(val, 0)
    }

    /// The root row list.
    pub fn ingest(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
        trace!(depth = self.depth(), ?val, "ingest");
        match val {
            PyroValue::List(rows) => {
                for row in rows {
                    self.walk(row, 0)?;
                }
                Ok(())
            }
            PyroValue::Null => Ok(()),
            _ => Err(WireError::TypeMismatch {
                expected: nested(self.datatype.clone(), self.depth() + 1),
                actual: val.datatype(),
            }),
        }
    }

    pub fn variable_length(&self) -> usize {
        self.topology.variable_length()
            + self
                .items
                .iter()
                .map(|(_, w)| w.variable_length())
                .sum::<usize>()
    }

    /// Own header (see the module docs), then every child's structure.
    pub fn write_structure(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        if self.depth() == 0 {
            buffer.write_count(self.n, self.count);
        } else {
            buffer.write_topology(self.n, &self.topology);
        }
        for (_, w) in &self.items {
            w.write_structure(buffer)?;
        }
        Ok(())
    }

    pub fn padding_structure(&self, padding: &mut Padding) {
        if self.depth() > 0 {
            self.topology.padding_levels(padding);
        }
        for (_, w) in &self.items {
            w.padding_structure(padding);
        }
    }

    pub fn padding_data(&self, pass: usize, padding: &mut Padding) {
        for (_, w) in &self.items {
            w.padding_data(pass, padding);
        }
    }

    pub fn write_16(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        for (_, w) in &self.items {
            w.write_16(buffer)?;
        }
        Ok(())
    }

    pub fn write_8(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        for (_, w) in &self.items {
            w.write_8(buffer)?;
        }
        Ok(())
    }

    pub fn write_4(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        for (_, w) in &self.items {
            w.write_4(buffer)?;
        }
        Ok(())
    }

    pub fn write_2(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        for (_, w) in &self.items {
            w.write_2(buffer)?;
        }
        Ok(())
    }

    pub fn write_1(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        for (_, w) in &self.items {
            w.write_1(buffer)?;
        }
        Ok(())
    }

    pub fn clear(self) -> NestedGroup<'static, W> {
        let mut topology = self.topology;
        topology.clear();
        NestedGroup {
            n: self.n,
            depth: self.depth,
            count: 0,
            topology,
            datatype: self.datatype,
            items: self
                .items
                .into_iter()
                .map(|(n, w)| (n, w.clear()))
                .collect(),
        }
    }
}

impl<'b, W: StructureWord> IoType for NestedGroup<'b, W> {
    fn datatype(&self) -> PyroType {
        nested(self.datatype.clone(), self.depth())
    }
}

// =============================================================================
// NestedOptionGroup — repeated nullable group
// =============================================================================

#[derive(Debug, Clone)]
pub struct NestedOptionGroup<'b, W: StructureWord> {
    /// The group's inode, which owns its mask or topology slot.
    n: usize,
    depth: usize,
    topology: TopologyCache<W>,
    /// One bit per group, nulls included, packed LSB first into bytes.
    validity: BitVec<u8, Lsb0>,
    datatype: PyroType,
    items: Vec<(String, NestedWriter<'b, W>)>,
}

impl<'b, W: StructureWord> NestedOptionGroup<'b, W> {
    /// `depth` = `nesting - 1 + group_nest`.
    pub fn new(n: usize, depth: usize, items: Vec<(String, NestedWriter<'b, W>)>) -> Self {
        assert!(
            depth <= MAX_TIERS,
            "nesting depth {depth} exceeds the value path's maximum of {MAX_TIERS}"
        );
        trace!(n, depth, fields = items.len(), "Creating NestedOptionGroup");
        let datatype = PyroType::OptionGroup(items.iter().map(|(_, w)| w.datatype()).collect());
        Self {
            n,
            depth,
            topology: TopologyCache::new(),
            validity: BitVec::new(),
            datatype,
            items,
        }
    }

    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Groups seen, nulls included.
    #[inline]
    pub fn num_groups(&self) -> usize {
        self.validity.len()
    }

    #[inline]
    pub fn topology(&self) -> &[Vec<W>] {
        &self.topology.levels()[..self.depth]
    }

    #[inline]
    pub fn items(&self) -> &[(String, NestedWriter<'b, W>)] {
        &self.items
    }

    #[inline]
    fn mask_len(&self) -> usize {
        self.validity.len().div_ceil(8)
    }

    /// One group or a `Null`. Always exactly one validity bit; an absent
    /// group feeds nothing.
    fn group(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
        if val.is_null() {
            self.validity.push(false);
            return Ok(());
        }
        let row = row_of(val, &self.datatype)?;
        for (name, w) in &mut self.items {
            w.feed(row.get(name).unwrap_or(&NULL))?;
        }
        self.validity.push(true);
        Ok(())
    }

    fn groups(&mut self, val: &'b PyroValue<'b>) -> Result<usize, WireError> {
        match val {
            PyroValue::List(items) => {
                for item in items {
                    self.group(item)?;
                }
                Ok(items.len())
            }
            PyroValue::Null => Ok(0),
            _ => Err(WireError::TypeMismatch {
                expected: nested(self.datatype.clone(), 1),
                actual: val.datatype(),
            }),
        }
    }

    fn walk(&mut self, val: &'b PyroValue<'b>, level: usize) -> Result<(), WireError> {
        if self.depth() == 0 {
            return self.group(val);
        }
        let n: usize = if level + 1 == self.depth() {
            self.groups(val)?
        } else {
            match val {
                PyroValue::List(items) => {
                    for item in items {
                        self.walk(item, level + 1)?;
                    }
                    items.len()
                }
                PyroValue::Null => 0,
                _ => {
                    return Err(WireError::TypeMismatch {
                        expected: nested(self.datatype.clone(), self.depth() - level),
                        actual: val.datatype(),
                    });
                }
            }
        };
        self.topology.bound(level, n);
        Ok(())
    }

    pub fn feed(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
        trace!(depth = self.depth(), ?val, "feed");
        self.walk(val, 0)
    }

    pub fn ingest(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
        trace!(depth = self.depth(), ?val, "ingest");
        match val {
            PyroValue::List(rows) => {
                for row in rows {
                    self.walk(row, 0)?;
                }
                Ok(())
            }
            PyroValue::Null => Ok(()),
            _ => Err(WireError::TypeMismatch {
                expected: nested(self.datatype.clone(), self.depth() + 1),
                actual: val.datatype(),
            }),
        }
    }

    pub fn variable_length(&self) -> usize {
        self.topology.variable_length()
            + self.mask_len()
            + self
                .items
                .iter()
                .map(|(_, w)| w.variable_length())
                .sum::<usize>()
    }

    /// The mask, with header `(start, groups)`, when there are no levels;
    /// otherwise the levels then the mask, with header `(start, words)`.
    /// Then every child's structure.
    pub fn write_structure(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        if self.depth() == 0 {
            buffer.write_group_mask(self.n, &self.validity);
        } else {
            buffer.write_topology_option(self.n, &self.topology, &self.validity);
        }
        for (_, w) in &self.items {
            w.write_structure(buffer)?;
        }
        Ok(())
    }

    pub fn padding_structure(&self, padding: &mut Padding) {
        padding.align(W::SIZE);
        self.topology.padding_levels(padding);
        padding.skip(self.mask_len());
        for (_, w) in &self.items {
            w.padding_structure(padding);
        }
    }

    pub fn padding_data(&self, pass: usize, padding: &mut Padding) {
        for (_, w) in &self.items {
            w.padding_data(pass, padding);
        }
    }

    pub fn write_16(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        for (_, w) in &self.items {
            w.write_16(buffer)?;
        }
        Ok(())
    }

    pub fn write_8(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        for (_, w) in &self.items {
            w.write_8(buffer)?;
        }
        Ok(())
    }

    pub fn write_4(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        for (_, w) in &self.items {
            w.write_4(buffer)?;
        }
        Ok(())
    }

    pub fn write_2(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        for (_, w) in &self.items {
            w.write_2(buffer)?;
        }
        Ok(())
    }

    pub fn write_1(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        for (_, w) in &self.items {
            w.write_1(buffer)?;
        }
        Ok(())
    }

    pub fn clear(self) -> NestedOptionGroup<'static, W> {
        let mut topology = self.topology;
        topology.clear();
        let mut validity = self.validity;
        validity.clear();
        NestedOptionGroup {
            n: self.n,
            depth: self.depth,
            topology,
            validity,
            datatype: self.datatype,
            items: self
                .items
                .into_iter()
                .map(|(n, w)| (n, w.clear()))
                .collect(),
        }
    }
}

impl<'b, W: StructureWord> IoType for NestedOptionGroup<'b, W> {
    fn datatype(&self) -> PyroType {
        nested(self.datatype.clone(), self.depth())
    }
}
