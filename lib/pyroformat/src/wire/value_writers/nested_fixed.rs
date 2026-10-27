//! Fixed-size array leaf writer: the `[T; k]` twin of
//! [`super::nested_slices::NestedValueWriter`].
//!
//! # Wire shape (see `src/wire/tests/group_nesting.rs::
//! LIST_GROUP_FIXED_U8_ARRAY_AND_LIST_FIXED_U16_ARRAY`)
//!
//! * Topology: identical to the plain primitives, except the deepest level
//!   counts **arrays**, not elements.
//! * Data header at `data_offset` is `(start, array_count)`.
//! * Payload: every array packed back to back, `array_count * k * size`
//!   bytes (`array_count * k` bits for `bool`).
//!
//! `depth` = `nesting - 1 + group_nest`. A `[u8; 16]` field inside a repeated
//! group is depth 0 (one array per group entry, no topology); `List<[u16; 8]>`
//! inside that group is depth 1.
//!
//! An array arrives either as a `PrimitiveList` of exactly `k` elements,
//! which is borrowed as one leaf, or as a `List` of exactly `k` scalars, which
//! is gathered into one inline entry. Anything else, including a wrong length
//! or a `Null` element, is a type mismatch: fixed arrays have no holes.

use std::marker::PhantomData;

use pyro_schema::StructureWord;
use tracing::trace;

use crate::{
    MAX_TIERS,
    errors::WireError,
    schema::{Bow, PrimitiveDataType, PyroType},
    value::{PrimitiveValueList, PyroValue, Time},
    wire::{
        IoType,
        value_writers::{
            interface::{FixedLeaf, Padding, SplitSink, TopologyCache},
            nested_slices::{Bits, FeedValue, IngestValue, WriteLeaf},
        },
    },
};

/// `t` under `lists` `List`s.
fn nested(mut t: PyroType, lists: usize) -> PyroType {
    for _ in 0..lists {
        t = PyroType::List(Bow::new(t));
    }
    t
}

/// `pdt` arrays of `actual` elements where `expected` were wanted.
fn wrong_len(pdt: PrimitiveDataType, expected: usize, actual: usize) -> WireError {
    WireError::TypeMismatch {
        expected: PyroType::PrimitiveFixedList(pdt, expected),
        actual: PyroType::PrimitiveFixedList(pdt, actual),
    }
}

// =============================================================================
// NestedFixedWriter
// =============================================================================

/// `A` accumulates the arrays: the borrowed runs and gathered entries for a
/// numeric (`Vec<FixedLeaf<'b, T>>`), the packed bits for `bool`
/// ([`Bits`]).
#[derive(Debug, Clone)]
pub struct NestedFixedWriter<'b, W: StructureWord, T, A = Vec<FixedLeaf<'b, T>>> {
    n: usize,
    /// Elements per array.
    k: usize,
    depth: usize,
    /// Arrays collected.
    arrays: usize,
    topology: TopologyCache<W>,
    leaves: A,
    phantom: PhantomData<&'b T>,
}

impl<'b, W: StructureWord, T, A: Default> NestedFixedWriter<'b, W, T, A> {
    /// `k` is the inode's `fixed_size`; `n` is the inode the topology and
    /// payload slots belong to.
    pub fn new(k: usize, depth: usize, n: usize) -> Self {
        debug_assert!(k > 0, "a fixed array has at least one element");
        assert!(
            depth <= MAX_TIERS,
            "nesting depth {depth} exceeds the value path's maximum of {MAX_TIERS}"
        );
        trace!(
            k,
            depth,
            "Creating NestedFixedWriter<{}>",
            std::any::type_name::<T>()
        );
        Self {
            n,
            k,
            depth,
            arrays: 0,
            topology: TopologyCache::new(),
            leaves: A::default(),
            phantom: PhantomData,
        }
    }

    /// Elements per array.
    #[inline]
    pub fn k(&self) -> usize {
        self.k
    }

    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    #[inline]
    pub fn num_arrays(&self) -> usize {
        self.arrays
    }

    #[inline]
    pub fn topology(&self) -> &[Vec<W>] {
        &self.topology.levels()[..self.depth]
    }
}

// =============================================================================
// Numerics — a `Vec<FixedLeaf>` of borrowed runs and gathered arrays
// =============================================================================

macro_rules! numeric_fixed_writer {
    ($($ty:ty, $Variant:ident, $write_slice:ident;)*) => {
        $(
            impl<'b, W: StructureWord> NestedFixedWriter<'b, W, $ty> {
                fn array_type(&self) -> PyroType {
                    PyroType::PrimitiveFixedList(PrimitiveDataType::$Variant, self.k)
                }

                /// One array: a `PrimitiveList` of exactly `k` (borrowed) or a
                /// `List` of exactly `k` scalars (gathered). Returns 1, or 0
                /// for `Null` (a missing array).
                fn array(&mut self, val: &'b PyroValue<'b>) -> Result<usize, WireError> {
                    let leaf = match val {
                        PyroValue::Null => return Ok(0),
                        PyroValue::PrimitiveList(PrimitiveValueList::$Variant(v)) => {
                            let s: &'b [$ty] = v.as_ref();
                            if s.len() != self.k {
                                return Err(wrong_len(PrimitiveDataType::$Variant, self.k, s.len()));
                            }
                            FixedLeaf::Slice(s)
                        }
                        PyroValue::List(items) => {
                            if items.len() != self.k {
                                return Err(wrong_len(PrimitiveDataType::$Variant, self.k, items.len()));
                            }
                            let mut array = Vec::with_capacity(self.k);
                            for item in items {
                                match item {
                                    PyroValue::$Variant(v) => array.push(*v),
                                    _ => {
                                        return Err(WireError::TypeMismatch {
                                            expected: self.array_type(),
                                            actual: item.datatype(),
                                        });
                                    }
                                }
                            }
                            FixedLeaf::Entry(array)
                        }
                        _ => {
                            return Err(WireError::TypeMismatch {
                                expected: self.array_type(),
                                actual: val.datatype(),
                            });
                        }
                    };
                    self.leaves.push(leaf);
                    self.arrays += 1;
                    Ok(1)
                }

                /// The sample-level container: a `List` of arrays, a flat
                /// `j * k` run split into `j` borrowed arrays, or `Null`.
                /// Returns how many arrays it held.
                fn samples(&mut self, val: &'b PyroValue<'b>) -> Result<usize, WireError> {
                    match val {
                        PyroValue::List(items) => {
                            let mut j = 0;
                            for item in items {
                                j += self.array(item)?;
                            }
                            Ok(j)
                        }
                        PyroValue::PrimitiveList(PrimitiveValueList::$Variant(v)) => {
                            let s: &'b [$ty] = v.as_ref();
                            if s.len() % self.k != 0 {
                                return Err(wrong_len(PrimitiveDataType::$Variant, self.k, s.len()));
                            }
                            self.leaves.extend(s.chunks_exact(self.k).map(FixedLeaf::Slice));
                            let j = s.len() / self.k;
                            self.arrays += j;
                            Ok(j)
                        }
                        PyroValue::Null => Ok(0),
                        _ => Err(WireError::TypeMismatch {
                            expected: nested(self.array_type(), 1),
                            actual: val.datatype(),
                        }),
                    }
                }

                fn walk(&mut self, val: &'b PyroValue<'b>, level: usize) -> Result<(), WireError> {
                    if self.depth == 0 {
                        self.array(val)?;
                        return Ok(());
                    }
                    let n = if level + 1 == self.depth {
                        self.samples(val)?
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
                                    expected: nested(self.array_type(), self.depth - level),
                                    actual: val.datatype(),
                                });
                            }
                        }
                    };
                    self.topology.bound(level, n);
                    Ok(())
                }

                /// Move the (now empty) allocations to a new borrow lifetime.
                pub fn clear<'c>(mut self) -> NestedFixedWriter<'c, W, $ty> {
                    self.topology.clear();
                    self.leaves.clear();
                    // Safety: `leaves` is empty; only the allocation is
                    // reused.
                    let leaves = unsafe {
                        std::mem::transmute::<Vec<FixedLeaf<'b, $ty>>, Vec<FixedLeaf<'c, $ty>>>(
                            self.leaves,
                        )
                    };
                    NestedFixedWriter {
                        n: self.n,
                        k: self.k,
                        depth: self.depth,
                        arrays: 0,
                        topology: self.topology,
                        leaves,
                        phantom: PhantomData,
                    }
                }
            }

            impl<'b, W: StructureWord> FeedValue<'b> for NestedFixedWriter<'b, W, $ty> {
                fn feed(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
                    trace!(k = self.k, depth = self.depth, ?val, "feed");
                    self.walk(val, 0)
                }
            }

            impl<'b, W: StructureWord> IngestValue<'b> for NestedFixedWriter<'b, W, $ty> {
                fn ingest(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
                    trace!(k = self.k, depth = self.depth, ?val, "ingest");
                    match val {
                        PyroValue::List(rows) => {
                            for row in rows {
                                self.walk(row, 0)?;
                            }
                            Ok(())
                        }
                        // A flat row list of arrays arrives as one run of
                        // `j * k` elements.
                        PyroValue::PrimitiveList(_) if self.depth == 0 => {
                            self.samples(val).map(|_| ())
                        }
                        PyroValue::Null => Ok(()),
                        _ => Err(WireError::TypeMismatch {
                            expected: nested(self.array_type(), self.depth + 1),
                            actual: val.datatype(),
                        }),
                    }
                }
            }

            impl<'b, W: StructureWord> WriteLeaf<W> for NestedFixedWriter<'b, W, $ty> {
                fn variable_length(&self) -> usize {
                    self.topology.variable_length() + self.arrays * self.k * size_of::<$ty>()
                }

                fn write_structure(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
                    if self.depth > 0 {
                        buffer.write_topology(self.n, &self.topology);
                    }
                    Ok(())
                }

                /// Arrays back to back; header `(start, array_count)`.
                fn write_data(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
                    buffer.$write_slice(self.n, &self.leaves);
                    Ok(())
                }

                fn padding_structure(&self, padding: &mut Padding) {
                    if self.depth > 0 {
                        self.topology.padding_levels(padding);
                    }
                }

                fn padding_data(&self, padding: &mut Padding) {
                    padding.align(size_of::<$ty>());
                    padding.skip(self.arrays * self.k * size_of::<$ty>());
                }
            }

            impl<'b, W: StructureWord> IoType for NestedFixedWriter<'b, W, $ty> {
                fn datatype(&self) -> PyroType {
                    nested(self.array_type(), self.depth)
                }
            }
        )*
    };
}

numeric_fixed_writer! {
    u8, U8, write_fixed_u8_slice;
    i8, I8, write_fixed_i8_slice;
    u16, U16, write_fixed_u16_slice;
    i16, I16, write_fixed_i16_slice;
    u32, U32, write_fixed_u32_slice;
    i32, I32, write_fixed_i32_slice;
    u64, U64, write_fixed_u64_slice;
    i64, I64, write_fixed_i64_slice;
    half::f16, F16, write_fixed_f16_slice;
    f32, F32, write_fixed_f32_slice;
    f64, F64, write_fixed_f64_slice;
    Time, Timestamp, write_fixed_time_slice;
}

// =============================================================================
// Bool — every array's `k` bits packed into a `Bits` as they arrive
// =============================================================================

impl<'b, W: StructureWord> NestedFixedWriter<'b, W, bool, Bits> {
    fn array_type(&self) -> PyroType {
        PyroType::PrimitiveFixedList(PrimitiveDataType::Bool, self.k)
    }

    /// One array: a `PrimitiveList` of exactly `k` bits or a `List` of
    /// exactly `k` scalars. Returns 1, or 0 for `Null` (a missing array).
    fn array(&mut self, val: &'b PyroValue<'b>) -> Result<usize, WireError> {
        match val {
            PyroValue::Null => return Ok(0),
            PyroValue::PrimitiveList(PrimitiveValueList::Bool(bits)) => {
                if bits.len() != self.k {
                    return Err(wrong_len(PrimitiveDataType::Bool, self.k, bits.len()));
                }
                self.leaves.extend_from_bitslice(bits);
            }
            PyroValue::List(items) => {
                if items.len() != self.k {
                    return Err(wrong_len(PrimitiveDataType::Bool, self.k, items.len()));
                }
                // Check the whole array before taking any of it.
                if let Some(item) = items
                    .iter()
                    .find(|item| !matches!(item, PyroValue::Bool(_)))
                {
                    return Err(WireError::TypeMismatch {
                        expected: self.array_type(),
                        actual: item.datatype(),
                    });
                }
                self.leaves.extend(
                    items
                        .iter()
                        .map(|item| matches!(item, PyroValue::Bool(true))),
                );
            }
            _ => {
                return Err(WireError::TypeMismatch {
                    expected: self.array_type(),
                    actual: val.datatype(),
                });
            }
        }
        self.arrays += 1;
        Ok(1)
    }

    /// The sample-level container: a `List` of arrays, a flat `j * k` bit
    /// run, or `Null`. Returns how many arrays it held.
    fn samples(&mut self, val: &'b PyroValue<'b>) -> Result<usize, WireError> {
        match val {
            PyroValue::List(items) => {
                let mut j = 0;
                for item in items {
                    j += self.array(item)?;
                }
                Ok(j)
            }
            PyroValue::PrimitiveList(PrimitiveValueList::Bool(bits)) => {
                if bits.len() % self.k != 0 {
                    return Err(wrong_len(PrimitiveDataType::Bool, self.k, bits.len()));
                }
                self.leaves.extend_from_bitslice(bits);
                let j = bits.len() / self.k;
                self.arrays += j;
                Ok(j)
            }
            PyroValue::Null => Ok(0),
            _ => Err(WireError::TypeMismatch {
                expected: nested(self.array_type(), 1),
                actual: val.datatype(),
            }),
        }
    }

    /// As the numerics' `walk`.
    fn walk(&mut self, val: &'b PyroValue<'b>, level: usize) -> Result<(), WireError> {
        if self.depth == 0 {
            self.array(val)?;
            return Ok(());
        }
        let n = if level + 1 == self.depth {
            self.samples(val)?
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
                        expected: nested(self.array_type(), self.depth - level),
                        actual: val.datatype(),
                    });
                }
            }
        };
        self.topology.bound(level, n);
        Ok(())
    }

    /// Clear the bits and topology, keeping their allocations. Nothing is
    /// borrowed, so any lifetime will do.
    pub fn clear<'c>(mut self) -> NestedFixedWriter<'c, W, bool, Bits> {
        self.topology.clear();
        self.leaves.clear();
        NestedFixedWriter {
            n: self.n,
            k: self.k,
            depth: self.depth,
            arrays: 0,
            topology: self.topology,
            leaves: self.leaves,
            phantom: PhantomData,
        }
    }
}

impl<'b, W: StructureWord> FeedValue<'b> for NestedFixedWriter<'b, W, bool, Bits> {
    fn feed(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
        trace!(k = self.k, depth = self.depth, ?val, "feed");
        self.walk(val, 0)
    }
}

impl<'b, W: StructureWord> IngestValue<'b> for NestedFixedWriter<'b, W, bool, Bits> {
    fn ingest(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
        trace!(k = self.k, depth = self.depth, ?val, "ingest");
        match val {
            PyroValue::List(rows) => {
                for row in rows {
                    self.walk(row, 0)?;
                }
                Ok(())
            }
            // A flat row list of arrays arrives as one run of `j * k` bits.
            PyroValue::PrimitiveList(_) if self.depth == 0 => self.samples(val).map(|_| ()),
            PyroValue::Null => Ok(()),
            _ => Err(WireError::TypeMismatch {
                expected: nested(self.array_type(), self.depth + 1),
                actual: val.datatype(),
            }),
        }
    }
}

impl<'b, W: StructureWord> WriteLeaf<W> for NestedFixedWriter<'b, W, bool, Bits> {
    fn variable_length(&self) -> usize {
        self.topology.variable_length() + (self.arrays * self.k).div_ceil(8)
    }

    fn write_structure(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        if self.depth > 0 {
            buffer.write_topology(self.n, &self.topology);
        }
        Ok(())
    }

    /// Every array's bits back to back; header `(start, array_count)`.
    fn write_data(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        buffer.write_fixed_bool_slice(self.n, self.k, &self.leaves);
        Ok(())
    }

    fn padding_structure(&self, padding: &mut Padding) {
        if self.depth > 0 {
            self.topology.padding_levels(padding);
        }
    }

    fn padding_data(&self, padding: &mut Padding) {
        padding.skip((self.arrays * self.k).div_ceil(8));
    }
}

impl<'b, W: StructureWord> IoType for NestedFixedWriter<'b, W, bool, Bits> {
    fn datatype(&self) -> PyroType {
        nested(self.array_type(), self.depth)
    }
}

pub type BoolFixedWriter<'b, W> = NestedFixedWriter<'b, W, bool, Bits>;
pub type U8FixedWriter<'b, W> = NestedFixedWriter<'b, W, u8>;
pub type I8FixedWriter<'b, W> = NestedFixedWriter<'b, W, i8>;
pub type U16FixedWriter<'b, W> = NestedFixedWriter<'b, W, u16>;
pub type I16FixedWriter<'b, W> = NestedFixedWriter<'b, W, i16>;
pub type U32FixedWriter<'b, W> = NestedFixedWriter<'b, W, u32>;
pub type I32FixedWriter<'b, W> = NestedFixedWriter<'b, W, i32>;
pub type U64FixedWriter<'b, W> = NestedFixedWriter<'b, W, u64>;
pub type I64FixedWriter<'b, W> = NestedFixedWriter<'b, W, i64>;
pub type F16FixedWriter<'b, W> = NestedFixedWriter<'b, W, half::f16>;
pub type F32FixedWriter<'b, W> = NestedFixedWriter<'b, W, f32>;
pub type F64FixedWriter<'b, W> = NestedFixedWriter<'b, W, f64>;
pub type TimestampFixedWriter<'b, W> = NestedFixedWriter<'b, W, Time>;
