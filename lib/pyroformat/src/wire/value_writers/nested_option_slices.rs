//! Nullable primitive leaf writers: the `Option<T>` twins of
//! [`super::nested_slices::NestedValueWriter`].
//!
//! # Wire shape (see `src/wire/tests/primitive_slice_option.rs`,
//! `primitive_option_nesting.rs`)
//!
//! * Topology: identical to the non-option writers. Boundaries count **every
//!   sample, nulls included**, so `[[3.14, null, 1.5]]` is `[MAX, 3]`.
//! * Data header at `data_offset` is **three words**: `(start, valid, total)`.
//! * Payload: the valid values packed contiguously, then the validity mask,
//!   one bit per sample (nulls included), LSB first.
//!
//! `depth` = `nesting - 1 + group_nest`, as for the plain primitives. At depth
//! 0 a bare `Null` is one sample with its bit clear; at the sample level a
//! `Null` *container* contributes no samples at all.
//!
//! The numerics borrow `PrimitiveList` runs (all valid) and keep single
//! scalars inline, so a `List` of scalar `PyroValue`s costs one `Leaf` per
//! non-null item and no copy. `Option<Time>` is 16-byte samples.
//! `Option<bool>` packs its present values into a [`Bits`] as they arrive.

use std::marker::PhantomData;

use pyro_schema::{PrimitiveDataType, StructureWord};
use tracing::trace;

use crate::{
    MAX_TIERS,
    errors::WireError,
    schema::{Bow, PyroType},
    value::{PrimitiveValueList, PyroValue, Time},
    wire::{
        IoType,
        value_writers::{
            interface::{Leaf, Padding, SplitSink, TopologyCache},
            nested_slices::{Bits, FeedValue, IngestValue, WriteLeaf},
        },
    },
};

/// Mask bytes for `total` bits. Not padded: whatever is written next pads
/// itself to its own alignment.
#[inline]
fn mask_len(total: usize) -> usize {
    total.div_ceil(8)
}

/// `t` under `lists` `List`s.
fn nested(mut t: PyroType, lists: usize) -> PyroType {
    for _ in 0..lists {
        t = PyroType::List(Bow::new(t));
    }
    t
}

// =============================================================================
// NestedOptionSliceWriter
// =============================================================================

/// `A` accumulates the present values: the borrowed runs and single
/// entries for a numeric (`Vec<Leaf<'b, T>>`), the packed bits for `bool`
/// ([`Bits`]). `validity` has one bit per sample, nulls included.
#[derive(Debug, Clone)]
pub struct NestedOptionSliceWriter<'b, W: StructureWord, T, A = Vec<Leaf<'b, T>>> {
    n: usize,
    depth: usize,
    /// Valid samples (the packed payload count).
    valid: usize,
    topology: TopologyCache<W>,
    leaves: A,
    /// One bit per sample, nulls included. `validity.len()` is the total.
    validity: Bits,
    phantom: PhantomData<&'b T>,
}

impl<'b, W: StructureWord, T, A: Default> NestedOptionSliceWriter<'b, W, T, A> {
    /// `n` is the inode the topology and payload slots belong to.
    pub fn new(depth: usize, n: usize) -> Self {
        assert!(
            depth <= MAX_TIERS,
            "nesting depth {depth} exceeds the value path's maximum of {MAX_TIERS}"
        );
        trace!(
            depth,
            "Creating NestedOptionSliceWriter<{}>",
            std::any::type_name::<T>()
        );
        Self {
            n,
            depth,
            valid: 0,
            topology: TopologyCache::new(),
            leaves: A::default(),
            validity: Bits::new(),
            phantom: PhantomData,
        }
    }

    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Every sample, nulls included.
    #[inline]
    pub fn num_samples(&self) -> usize {
        self.validity.len()
    }

    #[inline]
    pub fn num_valid(&self) -> usize {
        self.valid
    }

    #[inline]
    pub fn topology(&self) -> &[Vec<W>] {
        &self.topology.levels()[..self.depth]
    }
}

// =============================================================================
// Numerics — a `Vec<Leaf>` of borrowed runs and single entries
// =============================================================================

macro_rules! numeric_option_writer {
    ($($ty:ty, $Variant:ident, $write_option_slice:ident;)*) => {
        $(
            impl<'b, W: StructureWord> NestedOptionSliceWriter<'b, W, $ty> {
                fn scalar_type() -> PyroType {
                    PyroType::OptionPrimitiveScalar(PrimitiveDataType::$Variant)
                }

                /// One sample: a value or a `Null`. Always exactly one
                /// validity bit.
                #[inline]
                fn scalar(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
                    match val {
                        PyroValue::Null => self.validity.push(false),
                        PyroValue::$Variant(v) => {
                            self.leaves.push(Leaf::Entry(*v));
                            self.valid += 1;
                            self.validity.push(true);
                        }
                        other => {
                            return Err(WireError::TypeMismatch {
                                expected: Self::scalar_type(),
                                actual: other.datatype(),
                            });
                        }
                    }
                    Ok(())
                }

                /// The sample-level container. Returns how many samples it
                /// held.
                fn samples(&mut self, val: &'b PyroValue<'b>) -> Result<usize, WireError> {
                    match val {
                        // A bulk run has no holes: borrow it, mark every bit
                        // valid.
                        PyroValue::PrimitiveList(PrimitiveValueList::$Variant(v)) => {
                            let s: &'b [$ty] = v.as_ref();
                            if !s.is_empty() {
                                self.leaves.push(Leaf::Slice(s));
                            }
                            self.valid += s.len();
                            self.validity.resize(self.validity.len() + s.len(), true);
                            Ok(s.len())
                        }
                        PyroValue::List(items) => {
                            for item in items {
                                self.scalar(item)?;
                            }
                            Ok(items.len())
                        }
                        PyroValue::Null => Ok(0),
                        _ => Err(WireError::TypeMismatch {
                            expected: nested(Self::scalar_type(), 1),
                            actual: val.datatype(),
                        }),
                    }
                }

                fn walk(&mut self, val: &'b PyroValue<'b>, level: usize) -> Result<(), WireError> {
                    if self.depth == 0 {
                        return self.scalar(val);
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
                                    expected: nested(Self::scalar_type(), self.depth - level),
                                    actual: val.datatype(),
                                });
                            }
                        }
                    };
                    self.topology.bound(level, n);
                    Ok(())
                }

                /// Move the (now empty) allocations to a new borrow lifetime.
                pub fn clear<'c>(mut self) -> NestedOptionSliceWriter<'c, W, $ty> {
                    self.topology.clear();
                    self.leaves.clear();
                    self.validity.clear();
                    // Safety: `leaves` is empty; only the allocation is
                    // reused.
                    let leaves = unsafe {
                        std::mem::transmute::<Vec<Leaf<'b, $ty>>, Vec<Leaf<'c, $ty>>>(self.leaves)
                    };
                    NestedOptionSliceWriter {
                        n: self.n,
                        depth: self.depth,
                        valid: 0,
                        topology: self.topology,
                        leaves,
                        validity: self.validity,
                        phantom: PhantomData,
                    }
                }
            }

            impl<'b, W: StructureWord> FeedValue<'b> for NestedOptionSliceWriter<'b, W, $ty> {
                fn feed(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
                    trace!(depth = self.depth, ?val, "feed");
                    self.walk(val, 0)
                }
            }

            impl<'b, W: StructureWord> IngestValue<'b> for NestedOptionSliceWriter<'b, W, $ty> {
                fn ingest(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
                    trace!(depth = self.depth, ?val, "ingest");
                    match val {
                        PyroValue::List(rows) => {
                            for row in rows {
                                self.walk(row, 0)?;
                            }
                            Ok(())
                        }
                        // A flat, hole-free row list arrives as one bulk
                        // slice.
                        PyroValue::PrimitiveList(_) if self.depth == 0 => {
                            self.samples(val).map(|_| ())
                        }
                        PyroValue::Null => Ok(()),
                        _ => Err(WireError::TypeMismatch {
                            expected: nested(Self::scalar_type(), self.depth + 1),
                            actual: val.datatype(),
                        }),
                    }
                }
            }

            impl<'b, W: StructureWord> WriteLeaf<W> for NestedOptionSliceWriter<'b, W, $ty> {
                fn variable_length(&self) -> usize {
                    self.topology.variable_length()
                        + self.valid * size_of::<$ty>()
                        + mask_len(self.validity.len())
                }

                fn write_structure(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
                    if self.depth > 0 {
                        buffer.write_topology(self.n, &self.topology);
                    }
                    Ok(())
                }

                /// Values, then the mask; three-word header.
                fn write_data(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
                    buffer.$write_option_slice(self.n, &self.leaves, &self.validity);
                    Ok(())
                }

                fn padding_structure(&self, padding: &mut Padding) {
                    if self.depth > 0 {
                        self.topology.padding_levels(padding);
                    }
                }

                fn padding_data(&self, padding: &mut Padding) {
                    padding.align(size_of::<$ty>());
                    padding.skip(self.valid * size_of::<$ty>() + mask_len(self.validity.len()));
                }
            }

            impl<'b, W: StructureWord> IoType for NestedOptionSliceWriter<'b, W, $ty> {
                fn datatype(&self) -> PyroType {
                    nested(Self::scalar_type(), self.depth.max(1))
                }
            }
        )*
    };
}

numeric_option_writer! {
    u8, U8, write_u8_option_slice;
    i8, I8, write_i8_option_slice;
    u16, U16, write_u16_option_slice;
    i16, I16, write_i16_option_slice;
    u32, U32, write_u32_option_slice;
    i32, I32, write_i32_option_slice;
    u64, U64, write_u64_option_slice;
    i64, I64, write_i64_option_slice;
    half::f16, F16, write_f16_option_slice;
    f32, F32, write_f32_option_slice;
    f64, F64, write_f64_option_slice;
    Time, Timestamp, write_time_option_slice;
}

// =============================================================================
// Bool — the present values are packed into a `Bits` as they arrive
// =============================================================================

impl<'b, W: StructureWord> NestedOptionSliceWriter<'b, W, bool, Bits> {
    fn scalar_type() -> PyroType {
        PyroType::OptionPrimitiveScalar(PrimitiveDataType::Bool)
    }

    /// One sample: a value or a `Null`. Always exactly one validity bit.
    #[inline]
    fn scalar(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
        match val {
            PyroValue::Null => self.validity.push(false),
            PyroValue::Bool(b) => {
                self.leaves.push(*b);
                self.valid += 1;
                self.validity.push(true);
            }
            other => {
                return Err(WireError::TypeMismatch {
                    expected: Self::scalar_type(),
                    actual: other.datatype(),
                });
            }
        }
        Ok(())
    }

    /// The sample-level container. Returns how many samples it held.
    fn samples(&mut self, val: &'b PyroValue<'b>) -> Result<usize, WireError> {
        match val {
            // A bulk run has no holes: append its bits, mark every bit valid.
            PyroValue::PrimitiveList(PrimitiveValueList::Bool(bits)) => {
                self.leaves.extend_from_bitslice(bits);
                self.valid += bits.len();
                self.validity.resize(self.validity.len() + bits.len(), true);
                Ok(bits.len())
            }
            PyroValue::List(items) => {
                for item in items {
                    self.scalar(item)?;
                }
                Ok(items.len())
            }
            PyroValue::Null => Ok(0),
            _ => Err(WireError::TypeMismatch {
                expected: nested(Self::scalar_type(), 1),
                actual: val.datatype(),
            }),
        }
    }

    /// As the numerics' `walk`.
    fn walk(&mut self, val: &'b PyroValue<'b>, level: usize) -> Result<(), WireError> {
        if self.depth == 0 {
            return self.scalar(val);
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
                        expected: nested(Self::scalar_type(), self.depth - level),
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
    pub fn clear<'c>(mut self) -> NestedOptionSliceWriter<'c, W, bool, Bits> {
        self.topology.clear();
        self.leaves.clear();
        self.validity.clear();
        NestedOptionSliceWriter {
            n: self.n,
            depth: self.depth,
            valid: 0,
            topology: self.topology,
            leaves: self.leaves,
            validity: self.validity,
            phantom: PhantomData,
        }
    }
}

impl<'b, W: StructureWord> FeedValue<'b> for NestedOptionSliceWriter<'b, W, bool, Bits> {
    fn feed(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
        trace!(depth = self.depth, ?val, "feed");
        self.walk(val, 0)
    }
}

impl<'b, W: StructureWord> IngestValue<'b> for NestedOptionSliceWriter<'b, W, bool, Bits> {
    fn ingest(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
        trace!(depth = self.depth, ?val, "ingest");
        match val {
            PyroValue::List(rows) => {
                for row in rows {
                    self.walk(row, 0)?;
                }
                Ok(())
            }
            // A flat, hole-free row list arrives as one bulk bit run.
            PyroValue::PrimitiveList(_) if self.depth == 0 => self.samples(val).map(|_| ()),
            PyroValue::Null => Ok(()),
            _ => Err(WireError::TypeMismatch {
                expected: nested(Self::scalar_type(), self.depth + 1),
                actual: val.datatype(),
            }),
        }
    }
}

impl<'b, W: StructureWord> WriteLeaf<W> for NestedOptionSliceWriter<'b, W, bool, Bits> {
    fn variable_length(&self) -> usize {
        self.topology.variable_length() + mask_len(self.valid) + mask_len(self.validity.len())
    }

    fn write_structure(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        if self.depth > 0 {
            buffer.write_topology(self.n, &self.topology);
        }
        Ok(())
    }

    /// Present values' bits, then the mask; three-word header.
    fn write_data(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        buffer.write_bool_option_slice(self.n, &self.leaves, &self.validity);
        Ok(())
    }

    fn padding_structure(&self, padding: &mut Padding) {
        if self.depth > 0 {
            self.topology.padding_levels(padding);
        }
    }

    fn padding_data(&self, padding: &mut Padding) {
        padding.skip(mask_len(self.valid) + mask_len(self.validity.len()));
    }
}

impl<'b, W: StructureWord> IoType for NestedOptionSliceWriter<'b, W, bool, Bits> {
    fn datatype(&self) -> PyroType {
        nested(Self::scalar_type(), self.depth.max(1))
    }
}

pub type BoolOptionSliceWriter<'b, W> = NestedOptionSliceWriter<'b, W, bool, Bits>;
pub type U8OptionSliceWriter<'b, W> = NestedOptionSliceWriter<'b, W, u8>;
pub type I8OptionSliceWriter<'b, W> = NestedOptionSliceWriter<'b, W, i8>;
pub type U16OptionSliceWriter<'b, W> = NestedOptionSliceWriter<'b, W, u16>;
pub type I16OptionSliceWriter<'b, W> = NestedOptionSliceWriter<'b, W, i16>;
pub type U32OptionSliceWriter<'b, W> = NestedOptionSliceWriter<'b, W, u32>;
pub type I32OptionSliceWriter<'b, W> = NestedOptionSliceWriter<'b, W, i32>;
pub type U64OptionSliceWriter<'b, W> = NestedOptionSliceWriter<'b, W, u64>;
pub type I64OptionSliceWriter<'b, W> = NestedOptionSliceWriter<'b, W, i64>;
pub type F16OptionSliceWriter<'b, W> = NestedOptionSliceWriter<'b, W, half::f16>;
pub type F32OptionSliceWriter<'b, W> = NestedOptionSliceWriter<'b, W, f32>;
pub type F64OptionSliceWriter<'b, W> = NestedOptionSliceWriter<'b, W, f64>;
pub type TimestampOptionSliceWriter<'b, W> = NestedOptionSliceWriter<'b, W, Time>;
