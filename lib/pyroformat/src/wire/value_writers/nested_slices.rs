use std::marker::PhantomData;

use bitvec::{order::Lsb0, vec::BitVec};
use pyro_schema::{PrimitiveDataType, StructureWord};
use tracing::trace;

use crate::{
    MAX_TIERS,
    errors::WireError,
    schema::{Bow, PyroType},
    value::{PrimitiveValueList, PyroValue, Time},
    wire::{
        IoType,
        value_writers::interface::{Leaf, Padding, SplitSink, TopologyCache},
    },
};

// =============================================================================
// The traits — value twins of f64_impl's FeedTopology / Ingest / WriteFinalLeaf
// =============================================================================

/// One entry at a time, as an enclosing Map or `List<Group<..>>` iterates.
/// Twin of `f64_impl::FeedTopology`.
pub trait FeedValue<'b>: IngestValue<'b> {
    fn feed(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError>;
}

/// The whole root value (the row list) in a single call. One reset per level,
/// then every row's boundaries cumulatively. Twin of `f64_impl::Ingest`.
pub trait IngestValue<'b>: Sized {
    fn ingest(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError>;
}

/// Emit the collected topology and payload and back-patch the fixed headers.
pub trait WriteLeaf<W: StructureWord> {
    /// Bytes this writer appends beyond the fixed section (topology + payload).
    fn variable_length(&self) -> usize;
    /// Emit the topology and patch the header at the writer's `topology_offset`.
    fn write_structure(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError>;
    /// Emit the payload and patch the header at the writer's `data_offset`.
    fn write_data(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError>;
    /// [`Self::write_structure`]'s appends, dry.
    fn padding_structure(&self, padding: &mut Padding);
    /// [`Self::write_data`]'s appends, dry.
    fn padding_data(&self, padding: &mut Padding);
}

// =============================================================================
// NestedValueWriter
// =============================================================================

/// Runtime-depth twin of `NestedSliceWriter<'a, N, W>`: the value path only
/// learns `depth` from the inode, so the levels are a `Vec` rather than an
/// array.
///
/// `A` accumulates the samples: the borrowed runs and single entries for a
/// numeric (`Vec<Leaf<'b, T>>`), the packed bits for `bool` (`BitVec`).
#[derive(Debug, Clone)]
pub struct NestedValueWriter<'b, W: StructureWord, T, A = Vec<Leaf<'b, T>>> {
    n: usize,
    total: usize,
    depth: usize,
    topology: TopologyCache<W>,
    leaves: A,
    phantom: PhantomData<&'b T>,
}

impl<'b, W: StructureWord, T, A: Default> NestedValueWriter<'b, W, T, A> {
    /// `depth` per the module docs:
    /// `nesting - 1 + group_nest as usize + (Bytes|Str) as usize`.
    /// `n` is the inode the topology and payload slots belong to.
    pub fn new(depth: usize, n: usize) -> Self {
        assert!(
            depth <= MAX_TIERS,
            "nesting depth {depth} exceeds the value path's maximum of {MAX_TIERS}"
        );
        trace!(
            depth,
            "Creating NestedValueWriter<{}>",
            std::any::type_name::<T>()
        );
        Self {
            n,
            total: 0,
            depth,
            topology: TopologyCache::new(),
            leaves: A::default(),
            phantom: PhantomData,
        }
    }

    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    #[inline]
    pub fn num_samples(&self) -> usize {
        self.total
    }
}

/// `t` under `levels` `List`s.
fn lists(mut t: PyroType, levels: usize) -> PyroType {
    for _ in 0..levels {
        t = PyroType::List(Bow::new(t));
    }
    t
}

// =============================================================================
// Numerics — a `Vec<Leaf>` of borrowed runs and single entries
// =============================================================================

macro_rules! numeric_writer {
    ($($ty:ty, $Variant:ident, $write_slice:ident;)*) => {
        $(
            impl<'b, W: StructureWord> NestedValueWriter<'b, W, $ty> {
                /// The sample-level container's type under `levels` `List`s,
                /// for errors and [`IoType::datatype`].
                fn expected_type(&self, levels: usize) -> PyroType {
                    lists(PyroType::PrimitiveList(PrimitiveDataType::$Variant), levels)
                }

                fn scalar(val: &PyroValue<'_>) -> Result<$ty, WireError> {
                    match val {
                        PyroValue::$Variant(v) => Ok(*v),
                        other => Err(WireError::TypeMismatch {
                            expected: PyroType::PrimitiveScalar(PrimitiveDataType::$Variant),
                            actual: other.datatype(),
                        }),
                    }
                }

                /// The sample-level container: a bulk `PrimitiveList` is one
                /// borrowed `Slice`, a `List` of scalars one `Entry` per
                /// non-null item, a `Null` no samples. Returns the sample
                /// count.
                fn samples(&mut self, val: &'b PyroValue<'b>) -> Result<usize, WireError> {
                    match val {
                        PyroValue::PrimitiveList(PrimitiveValueList::$Variant(v)) => {
                            let s: &'b [$ty] = v.as_ref();
                            if !s.is_empty() {
                                self.leaves.push(Leaf::Slice(s));
                            }
                            Ok(s.len())
                        }
                        PyroValue::List(items) => {
                            let mut n = 0;
                            for item in items.iter().filter(|item| !item.is_null()) {
                                self.leaves.push(Leaf::Entry(Self::scalar(item)?));
                                n += 1;
                            }
                            Ok(n)
                        }
                        PyroValue::Null => Ok(0),
                        _ => Err(WireError::TypeMismatch {
                            expected: self.expected_type(0),
                            actual: val.datatype(),
                        }),
                    }
                }

                /// Record `val` at topology `level` (0 == outermost owned
                /// level).
                ///
                /// * `depth == 0`: `val` is a single scalar.
                /// * `level + 1 == depth`: `val` is the sample-level
                ///   container; its sample count advances the deepest
                ///   boundary.
                /// * otherwise: `val` is a `List` whose items live one level
                ///   deeper; its item count advances this level's boundary.
                fn walk(&mut self, val: &'b PyroValue<'b>, level: usize) -> Result<(), WireError> {
                    if self.depth == 0 {
                        // A null scalar contributes nothing; nullability is
                        // the Option writer's business, not ours.
                        if !val.is_null() {
                            self.leaves.push(Leaf::Entry(Self::scalar(val)?));
                            self.total += 1;
                        }
                        return Ok(());
                    }
                    let n = if level + 1 == self.depth {
                        let n = self.samples(val)?;
                        self.total += n;
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
                                    expected: self.expected_type(self.depth - 1 - level),
                                    actual: val.datatype(),
                                });
                            }
                        }
                    };
                    self.topology.bound(level, n);
                    Ok(())
                }

                /// Move the (now empty) allocations to a new borrow lifetime.
                pub fn clear<'c>(mut self) -> NestedValueWriter<'c, W, $ty> {
                    self.topology.clear();
                    self.leaves.clear();
                    // Safety: the vector is empty, so no `&'b` reference
                    // survives the lifetime change; only the allocation is
                    // reused.
                    let leaves = unsafe {
                        std::mem::transmute::<Vec<Leaf<'b, $ty>>, Vec<Leaf<'c, $ty>>>(self.leaves)
                    };
                    NestedValueWriter {
                        n: self.n,
                        total: 0,
                        depth: self.depth,
                        topology: self.topology,
                        leaves,
                        phantom: PhantomData,
                    }
                }
            }

            impl<'b, W: StructureWord> FeedValue<'b> for NestedValueWriter<'b, W, $ty> {
                fn feed(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
                    trace!(depth = self.depth, ?val, "feed");
                    self.walk(val, 0)
                }
            }

            impl<'b, W: StructureWord> IngestValue<'b> for NestedValueWriter<'b, W, $ty> {
                fn ingest(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
                    trace!(depth = self.depth, ?val, "ingest");
                    match val {
                        PyroValue::List(rows) => {
                            for row in rows {
                                self.walk(row, 0)?;
                            }
                            Ok(())
                        }
                        // A flat primitive row list arrives as one bulk slice.
                        PyroValue::PrimitiveList(_) if self.depth == 0 => {
                            let n = self.samples(val)?;
                            self.total += n;
                            Ok(())
                        }
                        PyroValue::Null => Ok(()),
                        _ => Err(WireError::TypeMismatch {
                            expected: self.expected_type(self.depth),
                            actual: val.datatype(),
                        }),
                    }
                }
            }

            impl<'b, W: StructureWord> WriteLeaf<W> for NestedValueWriter<'b, W, $ty> {
                fn variable_length(&self) -> usize {
                    self.topology.variable_length() + self.total * size_of::<$ty>()
                }

                fn write_structure(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
                    if self.depth > 0 {
                        buffer.write_topology(self.n, &self.topology);
                    }
                    Ok(())
                }

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
                    padding.skip(self.total * size_of::<$ty>());
                }
            }

            impl<'b, W: StructureWord> IoType for NestedValueWriter<'b, W, $ty> {
                fn datatype(&self) -> PyroType {
                    self.expected_type(self.depth.saturating_sub(1))
                }
            }
        )*
    };
}

numeric_writer! {
    u8, U8, write_u8_slice;
    i8, I8, write_i8_slice;
    u16, U16, write_u16_slice;
    i16, I16, write_i16_slice;
    u32, U32, write_u32_slice;
    i32, I32, write_i32_slice;
    u64, U64, write_u64_slice;
    i64, I64, write_i64_slice;
    half::f16, F16, write_f16_slice;
    f32, F32, write_f32_slice;
    f64, F64, write_f64_slice;
    Time, Timestamp, write_time_slice;
}

// =============================================================================
// Bool — the samples are packed into a `BitVec` as they arrive
// =============================================================================

/// The bool writer's accumulator: every sample's bit, in order.
pub type Bits = BitVec<u8, Lsb0>;

impl<'b, W: StructureWord> NestedValueWriter<'b, W, bool, Bits> {
    /// The sample-level container's type under `levels` `List`s, for errors
    /// and [`IoType::datatype`].
    fn expected_type(&self, levels: usize) -> PyroType {
        lists(PyroType::PrimitiveList(PrimitiveDataType::Bool), levels)
    }

    fn scalar(val: &PyroValue<'_>) -> Result<bool, WireError> {
        match val {
            PyroValue::Bool(b) => Ok(*b),
            other => Err(WireError::TypeMismatch {
                expected: PyroType::PrimitiveScalar(PrimitiveDataType::Bool),
                actual: other.datatype(),
            }),
        }
    }

    /// The sample-level container: a bulk `PrimitiveList`'s bits are
    /// appended whole, a `List` of scalars one bit per non-null item, a
    /// `Null` no samples. Returns the sample count.
    fn samples(&mut self, val: &'b PyroValue<'b>) -> Result<usize, WireError> {
        match val {
            PyroValue::PrimitiveList(PrimitiveValueList::Bool(bits)) => {
                self.leaves.extend_from_bitslice(bits);
                Ok(bits.len())
            }
            PyroValue::List(items) => {
                let mut n = 0;
                for item in items.iter().filter(|item| !item.is_null()) {
                    self.leaves.push(Self::scalar(item)?);
                    n += 1;
                }
                Ok(n)
            }
            PyroValue::Null => Ok(0),
            _ => Err(WireError::TypeMismatch {
                expected: self.expected_type(0),
                actual: val.datatype(),
            }),
        }
    }

    /// As the numerics' `walk`.
    fn walk(&mut self, val: &'b PyroValue<'b>, level: usize) -> Result<(), WireError> {
        if self.depth == 0 {
            if !val.is_null() {
                self.leaves.push(Self::scalar(val)?);
                self.total += 1;
            }
            return Ok(());
        }
        let n = if level + 1 == self.depth {
            let n = self.samples(val)?;
            self.total += n;
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
                        expected: self.expected_type(self.depth - 1 - level),
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
    pub fn clear<'c>(mut self) -> NestedValueWriter<'c, W, bool, Bits> {
        self.topology.clear();
        self.leaves.clear();
        NestedValueWriter {
            n: self.n,
            total: 0,
            depth: self.depth,
            topology: self.topology,
            leaves: self.leaves,
            phantom: PhantomData,
        }
    }
}

impl<'b, W: StructureWord> FeedValue<'b> for NestedValueWriter<'b, W, bool, Bits> {
    fn feed(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
        trace!(depth = self.depth, ?val, "feed");
        self.walk(val, 0)
    }
}

impl<'b, W: StructureWord> IngestValue<'b> for NestedValueWriter<'b, W, bool, Bits> {
    fn ingest(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
        trace!(depth = self.depth, ?val, "ingest");
        match val {
            PyroValue::List(rows) => {
                for row in rows {
                    self.walk(row, 0)?;
                }
                Ok(())
            }
            // A flat bool row list arrives as one bulk bit run.
            PyroValue::PrimitiveList(_) if self.depth == 0 => {
                let n = self.samples(val)?;
                self.total += n;
                Ok(())
            }
            PyroValue::Null => Ok(()),
            _ => Err(WireError::TypeMismatch {
                expected: self.expected_type(self.depth),
                actual: val.datatype(),
            }),
        }
    }
}

impl<'b, W: StructureWord> WriteLeaf<W> for NestedValueWriter<'b, W, bool, Bits> {
    fn variable_length(&self) -> usize {
        self.topology.variable_length() + self.total.div_ceil(8)
    }

    fn write_structure(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        if self.depth > 0 {
            buffer.write_topology(self.n, &self.topology);
        }
        Ok(())
    }

    fn write_data(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        buffer.write_bool_slice(self.n, &self.leaves);
        Ok(())
    }

    fn padding_structure(&self, padding: &mut Padding) {
        if self.depth > 0 {
            self.topology.padding_levels(padding);
        }
    }

    fn padding_data(&self, padding: &mut Padding) {
        padding.skip(self.total.div_ceil(8));
    }
}

impl<'b, W: StructureWord> IoType for NestedValueWriter<'b, W, bool, Bits> {
    fn datatype(&self) -> PyroType {
        self.expected_type(self.depth.saturating_sub(1))
    }
}

// Convenience aliases mirroring the names `value_writers.rs` currently re-exports.
pub type BoolSliceWriter<'b, W> = NestedValueWriter<'b, W, bool, Bits>;
pub type U8SliceWriter<'b, W> = NestedValueWriter<'b, W, u8>;
pub type I8SliceWriter<'b, W> = NestedValueWriter<'b, W, i8>;
pub type U16SliceWriter<'b, W> = NestedValueWriter<'b, W, u16>;
pub type I16SliceWriter<'b, W> = NestedValueWriter<'b, W, i16>;
pub type U32SliceWriter<'b, W> = NestedValueWriter<'b, W, u32>;
pub type I32SliceWriter<'b, W> = NestedValueWriter<'b, W, i32>;
pub type U64SliceWriter<'b, W> = NestedValueWriter<'b, W, u64>;
pub type I64SliceWriter<'b, W> = NestedValueWriter<'b, W, i64>;
pub type F16SliceWriter<'b, W> = NestedValueWriter<'b, W, half::f16>;
pub type F32SliceWriter<'b, W> = NestedValueWriter<'b, W, f32>;
pub type F64SliceWriter<'b, W> = NestedValueWriter<'b, W, f64>;
pub type TimestampSliceWriter<'b, W> = NestedValueWriter<'b, W, Time>;
