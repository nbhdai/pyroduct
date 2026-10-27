//! `Str` / `Bytes` leaf writers: the blob twins of
//! [`super::nested_slices::NestedValueWriter`].
//!
//! [`StrWriter`] and [`BytesWriter`] borrow every blob's bytes (`str`s via
//! `as_bytes()`) as one [`Leaf::Slice`] each. A blob is always a whole
//! sample-level container, so there is no single-sample case: the payload
//! is their concatenation, written as one `u8` run with a
//! `(start, byte count)` slot.
//!
//! The deepest topology level is the per-blob **byte** boundary (`str` lengths
//! are always taken via `as_bytes()`), so `depth >= 1` always:
//!
//! ```text
//! depth = nesting + (group_nest ? 1 : 0)
//! ```
//!
//! * `Str`   key in a Map (nesting 0)                    → depth 1: `[MAX,3,6]`.
//! * `Str`   root `List<Str>` (nesting 1)                → depth 1: `[MAX,5,10]`.
//! * `Bytes` key `List<Bytes>` in a Map (nesting 1)      → depth 2: `[MAX,1]`, `[MAX,2]`.

use pyro_schema::{Bow, StructureWord};
use tracing::trace;

use crate::MAX_TIERS;
use crate::errors::WireError;
use crate::schema::PyroType;
use crate::value::{PrimitiveValueList, PyroValue};
use crate::wire::IoType;
use crate::wire::value_writers::interface::{Leaf, Padding, SplitSink, TopologyCache};
use crate::wire::value_writers::nested_slices::{FeedValue, IngestValue, WriteLeaf};

fn nested(mut t: PyroType, lists: usize) -> PyroType {
    for _ in 0..lists {
        t = PyroType::List(Bow::new(t));
    }
    t
}

// =============================================================================
// StrWriter
// =============================================================================

#[derive(Debug, Clone)]
pub struct StrWriter<'b, W: StructureWord> {
    n: usize,
    depth: usize,
    /// Total payload **bytes**.
    total: usize,
    topology: TopologyCache<W>,
    /// Every non-empty string's bytes, in order.
    parts: Vec<Leaf<'b, u8>>,
}

impl<'b, W: StructureWord> StrWriter<'b, W> {
    /// `depth == 0` is the bare root/flat-group field; `n` is the inode the
    /// topology and payload slots belong to.
    pub fn new(depth: usize, n: usize) -> Self {
        assert!(
            depth <= MAX_TIERS,
            "nesting depth {depth} exceeds the value path's maximum of {MAX_TIERS}"
        );
        trace!(depth, "Creating StrWriter");
        Self {
            n,
            depth,
            total: 0,
            topology: TopologyCache::new(),
            parts: Vec::new(),
        }
    }

    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    #[inline]
    pub fn num_bytes(&self) -> usize {
        self.total
    }

    #[inline]
    pub fn topology(&self) -> &[Vec<W>] {
        &self.topology.levels()[..self.depth]
    }

    /// The blob step: one `Str` (or Null as the empty string). Returns its
    /// byte length.
    #[inline]
    fn blob(&mut self, val: &'b PyroValue<'b>) -> Result<usize, WireError> {
        let s: &'b str = match val {
            PyroValue::Str(s) => s.as_ref(),
            PyroValue::Null => "",
            _ => {
                return Err(WireError::TypeMismatch {
                    expected: PyroType::Str,
                    actual: val.datatype(),
                });
            }
        };
        let n = s.len();
        if n > 0 {
            self.parts.push(Leaf::Slice(s.as_bytes()));
            self.total += n;
        }
        Ok(n)
    }

    fn walk(&mut self, val: &'b PyroValue<'b>, level: usize) -> Result<(), WireError> {
        if self.depth == 0 {
            self.blob(val)?;
            return Ok(());
        }

        let n: usize = if level + 1 == self.depth {
            let n = self.blob(val)?;
            n
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
                        expected: nested(PyroType::Str, self.depth - 1 - level),
                        actual: val.datatype(),
                    });
                }
            }
        };
        self.topology.bound(level, n);
        Ok(())
    }

    /// Move the (now empty) allocations to a new borrow lifetime.
    pub fn clear<'c>(mut self) -> StrWriter<'c, W> {
        self.topology.clear();
        self.parts.clear();
        // Safety: `parts` is empty; only the allocation is reused.
        let parts =
            unsafe { std::mem::transmute::<Vec<Leaf<'b, u8>>, Vec<Leaf<'c, u8>>>(self.parts) };
        StrWriter {
            n: self.n,
            depth: self.depth,
            total: 0,
            topology: self.topology,
            parts,
        }
    }
}

impl<'b, W: StructureWord> FeedValue<'b> for StrWriter<'b, W> {
    fn feed(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
        trace!(depth = self.depth, ?val, "feed");
        self.walk(val, 0)
    }
}

impl<'b, W: StructureWord> IngestValue<'b> for StrWriter<'b, W> {
    fn ingest(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
        trace!(depth = self.depth, ?val, "ingest");
        if self.depth == 0 {
            // The bare root/flat-group field: `val` is the blob itself, not
            // a row list.
            return self.walk(val, 0);
        }
        match val {
            PyroValue::List(rows) => {
                for row in rows {
                    self.walk(row, 0)?;
                }
                Ok(())
            }
            PyroValue::Null => Ok(()),
            _ => Err(WireError::TypeMismatch {
                expected: nested(PyroType::Str, self.depth),
                actual: val.datatype(),
            }),
        }
    }
}

impl<'b, W: StructureWord> WriteLeaf<W> for StrWriter<'b, W> {
    fn variable_length(&self) -> usize {
        self.topology.variable_length() + self.total
    }

    fn write_structure(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        // Depth 0 is the bare blob: no topology array of its own, so there is
        // nothing to write at its topology slot (which may not even be a
        // distinct, reserved slot — see `RootWriter`/`NestedWriter::new`).
        if self.depth > 0 {
            buffer.write_topology(self.n, &self.topology);
        }
        Ok(())
    }

    /// Every string's bytes back to back; header `(start, byte count)`.
    fn write_data(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        buffer.write_u8_slice(self.n, &self.parts);
        Ok(())
    }

    fn padding_structure(&self, padding: &mut Padding) {
        if self.depth > 0 {
            self.topology.padding_levels(padding);
        }
    }

    fn padding_data(&self, padding: &mut Padding) {
        padding.skip(self.total);
    }
}

impl<'b, W: StructureWord> IoType for StrWriter<'b, W> {
    fn datatype(&self) -> PyroType {
        if self.depth == 0 {
            PyroType::Str
        } else {
            nested(PyroType::Str, self.depth - 1)
        }
    }
}

// =============================================================================
// BytesWriter
// =============================================================================

#[derive(Debug, Clone)]
pub struct BytesWriter<'b, W: StructureWord> {
    n: usize,
    depth: usize,
    /// Total payload bytes.
    total: usize,
    topology: TopologyCache<W>,
    /// Every non-empty blob, in order.
    parts: Vec<Leaf<'b, u8>>,
}

impl<'b, W: StructureWord> BytesWriter<'b, W> {
    /// `depth == 0` is the bare root/flat-group field; `n` is the inode the
    /// topology and payload slots belong to.
    pub fn new(depth: usize, n: usize) -> Self {
        assert!(
            depth <= MAX_TIERS,
            "nesting depth {depth} exceeds the value path's maximum of {MAX_TIERS}"
        );
        trace!(depth, "Creating BytesWriter");
        Self {
            n,
            depth,
            total: 0,
            topology: TopologyCache::new(),
            parts: Vec::new(),
        }
    }

    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    #[inline]
    pub fn num_bytes(&self) -> usize {
        self.total
    }

    #[inline]
    pub fn topology(&self) -> &[Vec<W>] {
        &self.topology.levels()[..self.depth]
    }

    /// The blob step: `Bytes`, a `PrimitiveList::U8` (same bytes, other
    /// spelling), or Null as empty. Returns its length.
    #[inline]
    fn blob(&mut self, val: &'b PyroValue<'b>) -> Result<usize, WireError> {
        let b: &'b [u8] = match val {
            PyroValue::Bytes(b) => b.as_ref(),
            PyroValue::PrimitiveList(PrimitiveValueList::U8(b)) => b.as_ref(),
            PyroValue::Null => &[],
            _ => {
                return Err(WireError::TypeMismatch {
                    expected: PyroType::Bytes,
                    actual: val.datatype(),
                });
            }
        };
        let n = b.len();
        if n > 0 {
            self.parts.push(Leaf::Slice(b));
            self.total += n;
        }
        Ok(n)
    }

    fn walk(&mut self, val: &'b PyroValue<'b>, level: usize) -> Result<(), WireError> {
        if self.depth == 0 {
            self.blob(val)?;
            return Ok(());
        }

        let n: usize = if level + 1 == self.depth {
            let n = self.blob(val)?;
            n
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
                        expected: nested(PyroType::Bytes, self.depth - 1 - level),
                        actual: val.datatype(),
                    });
                }
            }
        };
        self.topology.bound(level, n);
        Ok(())
    }

    /// Move the (now empty) allocations to a new borrow lifetime.
    pub fn clear<'c>(mut self) -> BytesWriter<'c, W> {
        self.topology.clear();
        self.parts.clear();
        // Safety: `parts` is empty; only the allocation is reused.
        let parts =
            unsafe { std::mem::transmute::<Vec<Leaf<'b, u8>>, Vec<Leaf<'c, u8>>>(self.parts) };
        BytesWriter {
            n: self.n,
            depth: self.depth,
            total: 0,
            topology: self.topology,
            parts,
        }
    }
}

impl<'b, W: StructureWord> FeedValue<'b> for BytesWriter<'b, W> {
    fn feed(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
        trace!(depth = self.depth, ?val, "feed");
        self.walk(val, 0)
    }
}

impl<'b, W: StructureWord> IngestValue<'b> for BytesWriter<'b, W> {
    fn ingest(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
        trace!(depth = self.depth, ?val, "ingest");
        if self.depth == 0 {
            return self.walk(val, 0);
        }
        match val {
            PyroValue::List(rows) => {
                for row in rows {
                    self.walk(row, 0)?;
                }
                Ok(())
            }
            PyroValue::Null => Ok(()),
            _ => Err(WireError::TypeMismatch {
                expected: nested(PyroType::Bytes, self.depth),
                actual: val.datatype(),
            }),
        }
    }
}

impl<'b, W: StructureWord> WriteLeaf<W> for BytesWriter<'b, W> {
    fn variable_length(&self) -> usize {
        self.topology.variable_length() + self.total
    }

    fn write_structure(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        if self.depth > 0 {
            buffer.write_topology(self.n, &self.topology);
        }
        Ok(())
    }

    /// Every blob back to back; header `(start, byte count)`.
    fn write_data(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        buffer.write_u8_slice(self.n, &self.parts);
        Ok(())
    }

    fn padding_structure(&self, padding: &mut Padding) {
        if self.depth > 0 {
            self.topology.padding_levels(padding);
        }
    }

    fn padding_data(&self, padding: &mut Padding) {
        padding.skip(self.total);
    }
}

impl<'b, W: StructureWord> IoType for BytesWriter<'b, W> {
    fn datatype(&self) -> PyroType {
        if self.depth == 0 {
            PyroType::Bytes
        } else {
            nested(PyroType::Bytes, self.depth - 1)
        }
    }
}
