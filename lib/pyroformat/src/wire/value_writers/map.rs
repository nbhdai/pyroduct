//! Map writer.
//!
//! A map is a virtual `List<Group<key, value>>`: two children, fed in
//! lockstep, one pair per entry. The children are [`NestedWriter`]s built
//! with their own offsets; the map only records how many entries each map
//! had, an entry being the unit:
//!
//! * With no levels (a root `Map` at nesting 0) that is the bare one-word
//!   entry count in its slot (`MAP_STR_U32_THREE_ENTRIES` starts
//!   `3, 0, 0, 0`; `write_count`).
//! * With levels (`List<Map>`, or a map inside a group) it is the
//!   `(start, words)` header and sentinel-prefixed cumulative entry counts
//!   (`write_topology`): `LIST_MAP_U8_U8` is two maps of 2 and 1 entries →
//!   `[MAX, 2, 3]`.
//!
//! `depth` = `nesting + group_nest`: a map is like a blob, its entries are
//! the intrinsic level, so a map inside a group already has one level.
//!
//! Children are written key then value; each leaf pads itself to its own
//! alignment.

use pyro_schema::{Bow, StructureWord};
use tracing::trace;

use crate::MAX_TIERS;
use crate::errors::WireError;
use crate::schema::PyroType;
use crate::value::PyroValue;
use crate::wire::IoType;
use crate::wire::value_writers::NestedWriter;
use crate::wire::value_writers::interface::{Padding, SplitSink, TopologyCache};

#[derive(Debug, Clone)]
pub struct Map<'b, W: StructureWord> {
    /// The map's inode, which owns its entry count or topology slot.
    n: usize,
    depth: usize,
    /// Entries seen across every map fed.
    count: usize,
    topology: TopologyCache<W>,
    pub(crate) datatype: PyroType,
    keys: Box<NestedWriter<'b, W>>,
    values: Box<NestedWriter<'b, W>>,
}

impl<'b, W: StructureWord> Map<'b, W> {
    /// `depth` = `nesting + group_nest`.
    pub fn new(
        n: usize,
        depth: usize,
        keys: NestedWriter<'b, W>,
        values: NestedWriter<'b, W>,
    ) -> Self {
        assert!(
            depth <= MAX_TIERS,
            "nesting depth {depth} exceeds the value path's maximum of {MAX_TIERS}"
        );
        trace!(n, depth, "Creating Map");
        Self {
            n,
            depth,
            count: 0,
            datatype: PyroType::Map {
                key: keys.datatype().into(),
                value: values.datatype().into(),
            },
            topology: TopologyCache::new(),
            keys: Box::new(keys),
            values: Box::new(values),
        }
    }

    #[inline]
    pub fn datatype(&self) -> &PyroType {
        &self.datatype
    }

    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    #[inline]
    pub fn num_entries(&self) -> usize {
        self.count
    }

    #[inline]
    pub fn topology(&self) -> &[Vec<W>] {
        &self.topology.levels()[..self.depth]
    }

    #[inline]
    pub fn keys(&self) -> &NestedWriter<'b, W> {
        &self.keys
    }

    #[inline]
    pub fn values(&self) -> &NestedWriter<'b, W> {
        &self.values
    }

    /// One map: feed every pair. `Null` is an empty map. Returns the entry count.
    fn map(&mut self, val: &'b PyroValue<'b>) -> Result<usize, WireError> {
        match val {
            PyroValue::Map(entries) => {
                for (k, v) in entries {
                    self.keys.feed(k)?;
                    self.values.feed(v)?;
                }
                self.count += entries.len();
                Ok(entries.len())
            }
            PyroValue::Null => Ok(0),
            _ => Err(WireError::TypeMismatch {
                expected: self.datatype.clone(),
                actual: val.datatype(),
            }),
        }
    }

    fn walk(&mut self, val: &'b PyroValue<'b>, level: usize) -> Result<(), WireError> {
        if self.depth() == 0 {
            self.map(val)?;
            return Ok(());
        }
        let n: usize = if level + 1 == self.depth() {
            self.map(val)?
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
                        expected: self.nested(self.depth() - 1 - level),
                        actual: val.datatype(),
                    });
                }
            }
        };
        self.topology.bound(level, n);
        Ok(())
    }

    fn nested(&self, lists: usize) -> PyroType {
        let mut t = self.datatype.clone();
        for _ in 0..lists {
            t = PyroType::List(Bow::new(t));
        }
        t
    }

    pub fn clear<'c>(self) -> Map<'static, W> {
        let mut topology = self.topology;
        topology.clear();
        Map {
            n: self.n,
            depth: self.depth,
            count: 0,
            topology,
            datatype: self.datatype,
            keys: (*self.keys).clear().into(),
            values: (*self.values).clear().into(),
        }
    }

    /// One map from an enclosing group; with levels, a list of them.
    pub fn feed(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
        trace!(depth = self.depth(), ?val, "feed");
        self.walk(val, 0)
    }

    /// The root value: the map itself at depth 0, or the row list of maps.
    pub fn ingest(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
        trace!(depth = self.depth(), ?val, "ingest");
        match val {
            PyroValue::Map(_) if self.depth() == 0 => self.map(val).map(|_| ()),
            PyroValue::List(rows) => {
                for row in rows {
                    self.walk(row, 0)?;
                }
                Ok(())
            }
            PyroValue::Null => Ok(()),
            _ => Err(WireError::TypeMismatch {
                expected: self.nested(self.depth()),
                actual: val.datatype(),
            }),
        }
    }

    pub fn variable_length(&self) -> usize {
        self.topology.variable_length()
            + self.keys.variable_length()
            + self.values.variable_length()
    }

    /// Bare entry count at depth 0, `(start, words)` plus levels otherwise;
    /// then the children's structure.
    pub fn write_structure(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        if self.depth() == 0 {
            buffer.write_count(self.n, self.count);
        } else {
            buffer.write_topology(self.n, &self.topology);
        }
        self.keys.write_structure(buffer)?;
        self.values.write_structure(buffer)
    }

    pub fn padding_structure(&self, padding: &mut Padding) {
        if self.depth() > 0 {
            self.topology.padding_levels(padding);
        }
        self.keys.padding_structure(padding);
        self.values.padding_structure(padding);
    }

    pub fn padding_data(&self, pass: usize, padding: &mut Padding) {
        self.keys.padding_data(pass, padding);
        self.values.padding_data(pass, padding);
    }

    /// Every data pass goes to the keys, then the values.
    pub fn write_16(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        self.keys.write_16(buffer)?;
        self.values.write_16(buffer)?;
        Ok(())
    }

    pub fn write_8(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        self.keys.write_8(buffer)?;
        self.values.write_8(buffer)?;
        Ok(())
    }

    pub fn write_4(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        self.keys.write_4(buffer)?;
        self.values.write_4(buffer)?;
        Ok(())
    }

    pub fn write_2(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        self.keys.write_2(buffer)?;
        self.values.write_2(buffer)?;
        Ok(())
    }

    pub fn write_1(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        self.keys.write_1(buffer)?;
        self.values.write_1(buffer)?;
        Ok(())
    }
}

impl<'b, W: StructureWord> IoType for Map<'b, W> {
    fn datatype(&self) -> PyroType {
        self.nested(self.depth().saturating_sub(1))
    }
}
