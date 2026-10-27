//! Nullable fixed-size array leaf writer: the `Option<[T; k]>` twin of
//! [`super::nested_fixed::NestedFixedWriter`].
//!
//! # Wire shape
//!
//! * Topology: as [`NestedFixedWriter`](super::nested_fixed::NestedFixedWriter);
//!   the deepest level counts **arrays, nulls included**.
//! * Data header at `data_offset` is **three words**: `(start, valid, total)`.
//! * Payload: the valid arrays packed back to back (`valid * k * size`
//!   bytes, `valid * k` bits for `bool`), then the validity mask, one bit per
//!   array (nulls included), LSB first.
//!
//! At depth 0 a bare `Null` is one missing array with its bit clear; at the
//! sample level a `Null` *container* contributes no arrays at all.
//!
//! An array arrives as a `PrimitiveList` of exactly `k` elements (borrowed as
//! one leaf) or a `List` of exactly `k` scalars (gathered into one entry). A
//! wrong length or a `Null` *element* is a type mismatch: the option is on
//! the array, never on its elements.

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

/// `pdt` arrays of `actual` elements where `expected` were wanted.
fn wrong_len(pdt: PrimitiveDataType, expected: usize, actual: usize) -> WireError {
    WireError::TypeMismatch {
        expected: PyroType::OptionPrimitiveFixedList(pdt, expected),
        actual: PyroType::PrimitiveFixedList(pdt, actual),
    }
}

// =============================================================================
// NestedOptionFixedWriter
// =============================================================================

/// `A` accumulates the present arrays: the borrowed runs and gathered
/// entries for a numeric (`Vec<FixedLeaf<'b, T>>`), the packed bits for
/// `bool` ([`Bits`]). `validity` has one bit per array, nulls included.
#[derive(Debug, Clone)]
pub struct NestedOptionFixedWriter<'b, W: StructureWord, T, A = Vec<FixedLeaf<'b, T>>> {
    n: usize,
    /// Elements per array.
    k: usize,
    depth: usize,
    /// Valid arrays (the packed payload count).
    valid: usize,
    topology: TopologyCache<W>,
    leaves: A,
    /// One bit per array, nulls included. `validity.len()` is the total.
    validity: Bits,
    phantom: PhantomData<&'b T>,
}

impl<'b, W: StructureWord, T, A: Default> NestedOptionFixedWriter<'b, W, T, A> {
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
            "Creating NestedOptionFixedWriter<{}>",
            std::any::type_name::<T>()
        );
        Self {
            n,
            k,
            depth,
            valid: 0,
            topology: TopologyCache::new(),
            leaves: A::default(),
            validity: Bits::new(),
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

    /// Every array, nulls included.
    #[inline]
    pub fn num_arrays(&self) -> usize {
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
// Numerics — a `Vec<FixedLeaf>` of borrowed runs and gathered arrays
// =============================================================================

macro_rules! numeric_option_fixed_writer {
    ($($ty:ty, $Variant:ident, $write_option_slice:ident;)*) => {
        $(
            impl<'b, W: StructureWord> NestedOptionFixedWriter<'b, W, $ty> {
                fn array_type(&self) -> PyroType {
                    PyroType::OptionPrimitiveFixedList(PrimitiveDataType::$Variant, self.k)
                }

                /// One array or a `Null`. Always exactly one validity bit.
                fn array(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
                    let leaf = match val {
                        PyroValue::Null => {
                            self.validity.push(false);
                            return Ok(());
                        }
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
                    self.valid += 1;
                    self.validity.push(true);
                    Ok(())
                }

                /// The sample-level container: a `List` of arrays (nulls
                /// allowed), a flat hole-free `j * k` run split into `j`
                /// borrowed arrays, or `Null`. Returns how many arrays it
                /// held.
                fn samples(&mut self, val: &'b PyroValue<'b>) -> Result<usize, WireError> {
                    match val {
                        PyroValue::List(items) => {
                            for item in items {
                                self.array(item)?;
                            }
                            Ok(items.len())
                        }
                        PyroValue::PrimitiveList(PrimitiveValueList::$Variant(v)) => {
                            let s: &'b [$ty] = v.as_ref();
                            if s.len() % self.k != 0 {
                                return Err(wrong_len(PrimitiveDataType::$Variant, self.k, s.len()));
                            }
                            self.leaves.extend(s.chunks_exact(self.k).map(FixedLeaf::Slice));
                            let j = s.len() / self.k;
                            self.valid += j;
                            self.validity.resize(self.validity.len() + j, true);
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
                        return self.array(val);
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
                pub fn clear<'c>(mut self) -> NestedOptionFixedWriter<'c, W, $ty> {
                    self.topology.clear();
                    self.leaves.clear();
                    self.validity.clear();
                    // Safety: `leaves` is empty; only the allocation is
                    // reused.
                    let leaves = unsafe {
                        std::mem::transmute::<Vec<FixedLeaf<'b, $ty>>, Vec<FixedLeaf<'c, $ty>>>(
                            self.leaves,
                        )
                    };
                    NestedOptionFixedWriter {
                        n: self.n,
                        k: self.k,
                        depth: self.depth,
                        valid: 0,
                        topology: self.topology,
                        leaves,
                        validity: self.validity,
                        phantom: PhantomData,
                    }
                }
            }

            impl<'b, W: StructureWord> FeedValue<'b> for NestedOptionFixedWriter<'b, W, $ty> {
                fn feed(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
                    trace!(k = self.k, depth = self.depth, ?val, "feed");
                    self.walk(val, 0)
                }
            }

            impl<'b, W: StructureWord> IngestValue<'b> for NestedOptionFixedWriter<'b, W, $ty> {
                fn ingest(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
                    trace!(k = self.k, depth = self.depth, ?val, "ingest");
                    match val {
                        PyroValue::List(rows) => {
                            for row in rows {
                                self.walk(row, 0)?;
                            }
                            Ok(())
                        }
                        // A flat, hole-free row list of arrays arrives as one
                        // run.
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

            impl<'b, W: StructureWord> WriteLeaf<W> for NestedOptionFixedWriter<'b, W, $ty> {
                fn variable_length(&self) -> usize {
                    self.topology.variable_length()
                        + self.valid * self.k * size_of::<$ty>()
                        + mask_len(self.validity.len())
                }

                fn write_structure(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
                    if self.depth > 0 {
                        buffer.write_topology(self.n, &self.topology);
                    }
                    Ok(())
                }

                /// Valid arrays, then the mask; three-word header.
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
                    padding.skip(
                        self.valid * self.k * size_of::<$ty>() + mask_len(self.validity.len()),
                    );
                }
            }

            impl<'b, W: StructureWord> IoType for NestedOptionFixedWriter<'b, W, $ty> {
                fn datatype(&self) -> PyroType {
                    nested(self.array_type(), self.depth)
                }
            }
        )*
    };
}

numeric_option_fixed_writer! {
    u8, U8, write_fixed_u8_option_slice;
    i8, I8, write_fixed_i8_option_slice;
    u16, U16, write_fixed_u16_option_slice;
    i16, I16, write_fixed_i16_option_slice;
    u32, U32, write_fixed_u32_option_slice;
    i32, I32, write_fixed_i32_option_slice;
    u64, U64, write_fixed_u64_option_slice;
    i64, I64, write_fixed_i64_option_slice;
    half::f16, F16, write_fixed_f16_option_slice;
    f32, F32, write_fixed_f32_option_slice;
    f64, F64, write_fixed_f64_option_slice;
    Time, Timestamp, write_fixed_time_option_slice;
}

// =============================================================================
// Bool — every present array's `k` bits packed into a `Bits`
// =============================================================================

impl<'b, W: StructureWord> NestedOptionFixedWriter<'b, W, bool, Bits> {
    fn array_type(&self) -> PyroType {
        PyroType::OptionPrimitiveFixedList(PrimitiveDataType::Bool, self.k)
    }

    /// One array or a `Null`. Always exactly one validity bit.
    fn array(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
        match val {
            PyroValue::Null => {
                self.validity.push(false);
                return Ok(());
            }
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
        self.valid += 1;
        self.validity.push(true);
        Ok(())
    }

    /// The sample-level container: a `List` of arrays (nulls allowed), a
    /// flat hole-free `j * k` bit run, or `Null`. Returns how many arrays it
    /// held.
    fn samples(&mut self, val: &'b PyroValue<'b>) -> Result<usize, WireError> {
        match val {
            PyroValue::List(items) => {
                for item in items {
                    self.array(item)?;
                }
                Ok(items.len())
            }
            PyroValue::PrimitiveList(PrimitiveValueList::Bool(bits)) => {
                if bits.len() % self.k != 0 {
                    return Err(wrong_len(PrimitiveDataType::Bool, self.k, bits.len()));
                }
                self.leaves.extend_from_bitslice(bits);
                let j = bits.len() / self.k;
                self.valid += j;
                self.validity.resize(self.validity.len() + j, true);
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
            return self.array(val);
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
    pub fn clear<'c>(mut self) -> NestedOptionFixedWriter<'c, W, bool, Bits> {
        self.topology.clear();
        self.leaves.clear();
        self.validity.clear();
        NestedOptionFixedWriter {
            n: self.n,
            k: self.k,
            depth: self.depth,
            valid: 0,
            topology: self.topology,
            leaves: self.leaves,
            validity: self.validity,
            phantom: PhantomData,
        }
    }
}

impl<'b, W: StructureWord> FeedValue<'b> for NestedOptionFixedWriter<'b, W, bool, Bits> {
    fn feed(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
        trace!(k = self.k, depth = self.depth, ?val, "feed");
        self.walk(val, 0)
    }
}

impl<'b, W: StructureWord> IngestValue<'b> for NestedOptionFixedWriter<'b, W, bool, Bits> {
    fn ingest(&mut self, val: &'b PyroValue<'b>) -> Result<(), WireError> {
        trace!(k = self.k, depth = self.depth, ?val, "ingest");
        match val {
            PyroValue::List(rows) => {
                for row in rows {
                    self.walk(row, 0)?;
                }
                Ok(())
            }
            // A flat, hole-free row list of arrays arrives as one bit run.
            PyroValue::PrimitiveList(_) if self.depth == 0 => self.samples(val).map(|_| ()),
            PyroValue::Null => Ok(()),
            _ => Err(WireError::TypeMismatch {
                expected: nested(self.array_type(), self.depth + 1),
                actual: val.datatype(),
            }),
        }
    }
}

impl<'b, W: StructureWord> WriteLeaf<W> for NestedOptionFixedWriter<'b, W, bool, Bits> {
    fn variable_length(&self) -> usize {
        self.topology.variable_length()
            + mask_len(self.valid * self.k)
            + mask_len(self.validity.len())
    }

    fn write_structure(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        if self.depth > 0 {
            buffer.write_topology(self.n, &self.topology);
        }
        Ok(())
    }

    /// Valid arrays' bits, then the mask; three-word header.
    fn write_data(&self, buffer: &mut SplitSink<'_, W>) -> Result<(), WireError> {
        buffer.write_fixed_bool_option_slice(self.n, self.k, &self.leaves, &self.validity);
        Ok(())
    }

    fn padding_structure(&self, padding: &mut Padding) {
        if self.depth > 0 {
            self.topology.padding_levels(padding);
        }
    }

    fn padding_data(&self, padding: &mut Padding) {
        padding.skip(mask_len(self.valid * self.k) + mask_len(self.validity.len()));
    }
}

impl<'b, W: StructureWord> IoType for NestedOptionFixedWriter<'b, W, bool, Bits> {
    fn datatype(&self) -> PyroType {
        nested(self.array_type(), self.depth)
    }
}

pub type BoolOptionFixedWriter<'b, W> = NestedOptionFixedWriter<'b, W, bool, Bits>;
pub type U8OptionFixedWriter<'b, W> = NestedOptionFixedWriter<'b, W, u8>;
pub type I8OptionFixedWriter<'b, W> = NestedOptionFixedWriter<'b, W, i8>;
pub type U16OptionFixedWriter<'b, W> = NestedOptionFixedWriter<'b, W, u16>;
pub type I16OptionFixedWriter<'b, W> = NestedOptionFixedWriter<'b, W, i16>;
pub type U32OptionFixedWriter<'b, W> = NestedOptionFixedWriter<'b, W, u32>;
pub type I32OptionFixedWriter<'b, W> = NestedOptionFixedWriter<'b, W, i32>;
pub type U64OptionFixedWriter<'b, W> = NestedOptionFixedWriter<'b, W, u64>;
pub type I64OptionFixedWriter<'b, W> = NestedOptionFixedWriter<'b, W, i64>;
pub type F16OptionFixedWriter<'b, W> = NestedOptionFixedWriter<'b, W, half::f16>;
pub type F32OptionFixedWriter<'b, W> = NestedOptionFixedWriter<'b, W, f32>;
pub type F64OptionFixedWriter<'b, W> = NestedOptionFixedWriter<'b, W, f64>;
pub type TimestampOptionFixedWriter<'b, W> = NestedOptionFixedWriter<'b, W, Time>;
