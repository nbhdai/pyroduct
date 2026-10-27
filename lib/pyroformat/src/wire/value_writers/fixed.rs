//! Fixed-size array scalars: the `[T; k]` / `Option<[T; k]>` twins of
//! [`super::primitive::ScalarWriter`].
//!
//! [`FixedWriter`] is the writer for a plain nesting-0 `[T; k]` field outside
//! any group: the array is written straight into the fixed region at the
//! inode's data offset, no header, `k * size_of::<T>()` bytes (`k` bits,
//! packed LSB first, for `bool`). A nullable one also sets its bit in the
//! fixed region's bitmap when present; a null leaves both untouched.
//!
//! An array arrives as a `PrimitiveList` of exactly `k` elements, which is
//! borrowed, or as a `List` of exactly `k` scalars, which is gathered. A
//! wrong length or a `Null` *element* is a type mismatch: the option, if
//! any, is on the array, never on its elements.
//!
//! Arrays *inside* a Map or `List<Group>` go through `NestedFixedWriter` at
//! depth 0 instead, since they need a payload run and a header.

use std::borrow::Cow;

use bitvec::{order::Lsb0, slice::BitSlice, vec::BitVec};
use pyro_schema::{PrimitiveDataType, StructureWord};
use tracing::trace;

use crate::schema::PyroType;
use crate::value::{PrimitiveValueList, Time};
use crate::wire::IoType;
use crate::wire::value_writers::interface::{FixedLeaf, SplitSink};
use crate::{PyroValue, errors::WireError};

// =============================================================================
// FixedWriter — a nesting-0 `[T; k]` field outside any group
// =============================================================================

/// Holds at most one array of `k` elements for inode `n` and writes it at
/// its data offset. `T` is the array: a [`FixedLeaf`] for a numeric, a
/// `Cow<BitSlice>` for `bool`, or an `Option` of either.
#[derive(Debug, Clone)]
pub struct FixedWriter<T> {
    n: usize,
    /// Elements per array.
    k: usize,
    data: Option<T>,
}

impl<T> FixedWriter<T> {
    /// Forget the array in place; see [`ScalarWriter::reset`].
    ///
    /// [`ScalarWriter::reset`]: super::primitive::ScalarWriter::reset
    #[inline]
    pub fn reset(&mut self) {
        self.data = None;
    }

    /// `k` is the inode's `fixed_size`.
    pub fn new(k: usize, n: usize) -> Self {
        debug_assert!(k > 0, "a fixed array has at least one element");
        trace!(k, n, "Creating FixedWriter<{}>", std::any::type_name::<T>());
        Self { n, k, data: None }
    }

    /// Elements per array.
    #[inline]
    pub fn k(&self) -> usize {
        self.k
    }
}

/// `val` is not an `expected`.
fn mismatch(expected: PyroType, val: &PyroValue<'_>) -> WireError {
    WireError::TypeMismatch {
        expected,
        actual: val.datatype(),
    }
}

/// Arrays of `actual` elements where `expected` (an array of `k`) was
/// wanted.
fn wrong_len(expected: PyroType, pdt: PrimitiveDataType, actual: usize) -> WireError {
    WireError::TypeMismatch {
        expected,
        actual: PyroType::PrimitiveFixedList(pdt, actual),
    }
}

/// `get`, `clear`, `variable_length` and `write` for one array type `$ty`
/// (its required form) and `Option<$ty>` (its nullable form), each a
/// borrow for `'v`; `$ty_c` is the same type borrowed for `'c`, which
/// `clear` moves to. `$slice` turns a stored array into what `$write`
/// takes.
macro_rules! fixed_common {
    ($ty:ty, $ty_c:ty, $target:ty, $write:ident, |$a:ident| $slice:expr) => {
        impl<'v> FixedWriter<$ty> {
            #[inline]
            pub fn get(&self) -> Option<&$target> {
                self.data.as_ref().map(|$a| $slice)
            }

            /// Drop the array and the borrow; `n` and `k` are kept.
            #[inline]
            pub fn clear<'c>(self) -> FixedWriter<$ty_c> {
                FixedWriter {
                    n: self.n,
                    k: self.k,
                    data: None,
                }
            }

            /// Nothing is appended.
            #[inline]
            pub fn variable_length(&self) -> usize {
                0
            }

            /// Write inline at the inode's data offset; nothing is
            /// appended.
            pub fn write<W: StructureWord>(
                &self,
                buffer: &mut SplitSink<'_, W>,
            ) -> Result<(), WireError> {
                if let Some(array) = self.get() {
                    buffer.$write(self.n, array);
                }
                Ok(())
            }
        }

        impl<'v> FixedWriter<Option<$ty>> {
            /// The array, `Some(None)` for a null.
            #[inline]
            pub fn get(&self) -> Option<Option<&$target>> {
                self.data
                    .as_ref()
                    .map(|array| array.as_ref().map(|$a| $slice))
            }

            /// Drop the array and the borrow; `n` and `k` are kept.
            #[inline]
            pub fn clear<'c>(self) -> FixedWriter<Option<$ty_c>> {
                FixedWriter {
                    n: self.n,
                    k: self.k,
                    data: None,
                }
            }

            /// Nothing is appended.
            #[inline]
            pub fn variable_length(&self) -> usize {
                0
            }

            /// Write inline at the inode's data offset and mark it present;
            /// a null leaves the array and its bit zero.
            pub fn write<W: StructureWord>(
                &self,
                buffer: &mut SplitSink<'_, W>,
            ) -> Result<(), WireError> {
                if let Some(Some(array)) = self.get() {
                    buffer.write_valid(self.n);
                    buffer.$write(self.n, array);
                }
                Ok(())
            }
        }
    };
}

// =============================================================================
// Numerics — a `FixedLeaf`: borrowed from a `PrimitiveList`, or gathered
// =============================================================================

macro_rules! numeric_fixed_writer {
    ($($ty:ty, $Variant:ident, $write:ident;)*) => {
        $(
            fixed_common!(FixedLeaf<'v, $ty>, FixedLeaf<'c, $ty>, [$ty], $write, |array| match array {
                FixedLeaf::Slice(s) => *s,
                FixedLeaf::Entry(v) => v.as_slice(),
            });

            impl<'v> FixedWriter<FixedLeaf<'v, $ty>> {
                /// One array of exactly `k`, borrowed from `val` or gathered.
                fn array(&self, val: &'v PyroValue<'_>) -> Result<FixedLeaf<'v, $ty>, WireError> {
                    let expected = PyroType::PrimitiveFixedList(PrimitiveDataType::$Variant, self.k);
                    match val {
                        PyroValue::PrimitiveList(PrimitiveValueList::$Variant(v)) => {
                            let s: &'v [$ty] = v.as_ref();
                            if s.len() != self.k {
                                return Err(wrong_len(expected, PrimitiveDataType::$Variant, s.len()));
                            }
                            Ok(FixedLeaf::Slice(s))
                        }
                        PyroValue::List(items) => {
                            if items.len() != self.k {
                                return Err(wrong_len(expected, PrimitiveDataType::$Variant, items.len()));
                            }
                            let mut array = Vec::with_capacity(self.k);
                            for item in items {
                                match item {
                                    PyroValue::$Variant(v) => array.push(*v),
                                    other => return Err(mismatch(expected, other)),
                                }
                            }
                            Ok(FixedLeaf::Entry(array))
                        }
                        other => Err(mismatch(expected, other)),
                    }
                }

                /// Store the one array. A required field has no `Null`.
                pub fn ingest(&mut self, val: &'v PyroValue<'_>) -> Result<(), WireError> {
                    self.data = Some(self.array(val)?);
                    Ok(())
                }
            }

            impl<'v> IoType for FixedWriter<FixedLeaf<'v, $ty>> {
                fn datatype(&self) -> PyroType {
                    PyroType::PrimitiveFixedList(PrimitiveDataType::$Variant, self.k)
                }
            }

            impl<'v> FixedWriter<Option<FixedLeaf<'v, $ty>>> {
                /// Store the one array; a `Null` is stored as absent.
                pub fn ingest(&mut self, val: &'v PyroValue<'_>) -> Result<(), WireError> {
                    if val.is_null() {
                        self.data = Some(None);
                        return Ok(());
                    }
                    // The array's own checks, on a writer of the required form.
                    let required = FixedWriter::<FixedLeaf<'v, $ty>>::new(self.k, self.n);
                    let array = required.array(val).map_err(|_| mismatch(self.datatype(), val))?;
                    self.data = Some(Some(array));
                    Ok(())
                }
            }

            impl<'v> IoType for FixedWriter<Option<FixedLeaf<'v, $ty>>> {
                fn datatype(&self) -> PyroType {
                    PyroType::OptionPrimitiveFixedList(PrimitiveDataType::$Variant, self.k)
                }
            }
        )*
    };
}

numeric_fixed_writer! {
    u8, U8, write_fixed_u8;
    i8, I8, write_fixed_i8;
    u16, U16, write_fixed_u16;
    i16, I16, write_fixed_i16;
    u32, U32, write_fixed_u32;
    i32, I32, write_fixed_i32;
    u64, U64, write_fixed_u64;
    i64, I64, write_fixed_i64;
    half::f16, F16, write_fixed_f16;
    f32, F32, write_fixed_f32;
    f64, F64, write_fixed_f64;
    Time, Timestamp, write_fixed_time;
}

// =============================================================================
// Bool — `k` bits: borrowed from a `PrimitiveList`, or packed from scalars
// =============================================================================

/// A bool array: its `k` bits, borrowed or packed.
pub type Bits<'v> = Cow<'v, BitSlice<u8, Lsb0>>;

fixed_common!(Bits<'v>, Bits<'c>, BitSlice<u8, Lsb0>, write_fixed_bool, |array| &**array);

impl<'v> FixedWriter<Bits<'v>> {
    /// One array of exactly `k` bits, borrowed from `val` or packed.
    fn array(&self, val: &'v PyroValue<'_>) -> Result<Bits<'v>, WireError> {
        let expected = PyroType::PrimitiveFixedList(PrimitiveDataType::Bool, self.k);
        match val {
            PyroValue::PrimitiveList(PrimitiveValueList::Bool(bits)) => {
                if bits.len() != self.k {
                    return Err(wrong_len(expected, PrimitiveDataType::Bool, bits.len()));
                }
                Ok(Cow::Borrowed(&**bits))
            }
            PyroValue::List(items) => {
                if items.len() != self.k {
                    return Err(wrong_len(expected, PrimitiveDataType::Bool, items.len()));
                }
                let mut bits = BitVec::with_capacity(self.k);
                for item in items {
                    match item {
                        PyroValue::Bool(b) => bits.push(*b),
                        other => return Err(mismatch(expected, other)),
                    }
                }
                Ok(Cow::Owned(bits))
            }
            other => Err(mismatch(expected, other)),
        }
    }

    /// Store the one array. A required field has no `Null`.
    pub fn ingest(&mut self, val: &'v PyroValue<'_>) -> Result<(), WireError> {
        self.data = Some(self.array(val)?);
        Ok(())
    }
}

impl<'v> IoType for FixedWriter<Bits<'v>> {
    fn datatype(&self) -> PyroType {
        PyroType::PrimitiveFixedList(PrimitiveDataType::Bool, self.k)
    }
}

impl<'v> FixedWriter<Option<Bits<'v>>> {
    /// Store the one array; a `Null` is stored as absent.
    pub fn ingest(&mut self, val: &'v PyroValue<'_>) -> Result<(), WireError> {
        if val.is_null() {
            self.data = Some(None);
            return Ok(());
        }
        // The array's own checks, on a writer of the required form.
        let required = FixedWriter::<Bits<'v>>::new(self.k, self.n);
        let array = required
            .array(val)
            .map_err(|_| mismatch(self.datatype(), val))?;
        self.data = Some(Some(array));
        Ok(())
    }
}

impl<'v> IoType for FixedWriter<Option<Bits<'v>>> {
    fn datatype(&self) -> PyroType {
        PyroType::OptionPrimitiveFixedList(PrimitiveDataType::Bool, self.k)
    }
}

pub type BoolFixedWriter<'v> = FixedWriter<Bits<'v>>;
pub type U8FixedWriter<'v> = FixedWriter<FixedLeaf<'v, u8>>;
pub type I8FixedWriter<'v> = FixedWriter<FixedLeaf<'v, i8>>;
pub type U16FixedWriter<'v> = FixedWriter<FixedLeaf<'v, u16>>;
pub type I16FixedWriter<'v> = FixedWriter<FixedLeaf<'v, i16>>;
pub type U32FixedWriter<'v> = FixedWriter<FixedLeaf<'v, u32>>;
pub type I32FixedWriter<'v> = FixedWriter<FixedLeaf<'v, i32>>;
pub type U64FixedWriter<'v> = FixedWriter<FixedLeaf<'v, u64>>;
pub type I64FixedWriter<'v> = FixedWriter<FixedLeaf<'v, i64>>;
pub type F16FixedWriter<'v> = FixedWriter<FixedLeaf<'v, half::f16>>;
pub type F32FixedWriter<'v> = FixedWriter<FixedLeaf<'v, f32>>;
pub type F64FixedWriter<'v> = FixedWriter<FixedLeaf<'v, f64>>;
pub type TimestampFixedWriter<'v> = FixedWriter<FixedLeaf<'v, Time>>;

pub type BoolFixedOptionWriter<'v> = FixedWriter<Option<Bits<'v>>>;
pub type U8FixedOptionWriter<'v> = FixedWriter<Option<FixedLeaf<'v, u8>>>;
pub type I8FixedOptionWriter<'v> = FixedWriter<Option<FixedLeaf<'v, i8>>>;
pub type U16FixedOptionWriter<'v> = FixedWriter<Option<FixedLeaf<'v, u16>>>;
pub type I16FixedOptionWriter<'v> = FixedWriter<Option<FixedLeaf<'v, i16>>>;
pub type U32FixedOptionWriter<'v> = FixedWriter<Option<FixedLeaf<'v, u32>>>;
pub type I32FixedOptionWriter<'v> = FixedWriter<Option<FixedLeaf<'v, i32>>>;
pub type U64FixedOptionWriter<'v> = FixedWriter<Option<FixedLeaf<'v, u64>>>;
pub type I64FixedOptionWriter<'v> = FixedWriter<Option<FixedLeaf<'v, i64>>>;
pub type F16FixedOptionWriter<'v> = FixedWriter<Option<FixedLeaf<'v, half::f16>>>;
pub type F32FixedOptionWriter<'v> = FixedWriter<Option<FixedLeaf<'v, f32>>>;
pub type F64FixedOptionWriter<'v> = FixedWriter<Option<FixedLeaf<'v, f64>>>;
pub type TimestampFixedOptionWriter<'v> = FixedWriter<Option<FixedLeaf<'v, Time>>>;
