//! Leaf primitives and the nesting-0 scalar writer.
//!
//! [`ScalarWriter`] is the writer for a plain nesting-0 field outside any
//! group: the eleven numerics, `bool` and [`Time`] (and their nullable
//! forms) are written straight into the fixed region at the inode's data
//! offset, no header (`_U8 = [200]`, `_TIMESTAMP` = 16 LE bytes). A
//! nullable one also sets its bit in the fixed region's bitmap when
//! present; a null leaves both untouched. A bare `Str` or `Bytes` is
//! appended to the variable region, with a `(start, byte count)` slot at
//! the data offset.
//!
//! Scalars *inside* a Map or `List<Group>` go through `NestedValueWriter`
//! at depth 0 instead, since they need a payload run and a header.

use std::borrow::Cow;

use pyro_schema::{PrimitiveDataType, StructureWord};
use tracing::trace;

use crate::schema::PyroType;
use crate::value::Time;
use crate::wire::IoType;
use crate::wire::value_writers::interface::SplitSink;
use crate::{PyroValue, errors::WireError};

// =============================================================================
// ScalarWriter — a nesting-0 field outside any group
// =============================================================================

/// Holds at most one sample for inode `n` and writes it at its data offset.
/// `T` is the sample: a `Copy` primitive, an `Option` of one, or a borrowed
/// `&str` / `&[u8]`.
#[derive(Debug, Clone)]
pub struct ScalarWriter<T> {
    n: usize,
    data: Option<T>,
}

impl<T: Copy> ScalarWriter<T> {
    pub fn new(n: usize) -> Self {
        trace!(n, "Creating ScalarWriter<{}>", std::any::type_name::<T>());
        Self { n, data: None }
    }

    #[inline]
    pub fn get(&self) -> Option<T> {
        self.data
    }
}

/// `val` is not an `expected`.
fn mismatch(expected: PyroType, val: &PyroValue<'_>) -> WireError {
    WireError::TypeMismatch {
        expected,
        actual: val.datatype(),
    }
}

/// `ingest`, `clear`, `write` and `datatype` for one required primitive and
/// its nullable form: `$Variant` is its `PyroValue` variant, `$write` its
/// `SplitSink` write and `$pyro` its scalar type.
macro_rules! primitive_writer {
    ($($ty:ty, $Variant:ident, $write:ident, $pyro:expr, $option_pyro:expr;)*) => {
        $(
            impl ScalarWriter<$ty> {
                /// Store the one value. A required field has no `Null`.
                pub fn ingest(&mut self, val: &PyroValue<'_>) -> Result<(), WireError> {
                    match val {
                        PyroValue::$Variant(v) => {
                            self.data = Some(*v);
                            Ok(())
                        }
                        other => Err(mismatch($pyro, other)),
                    }
                }

                #[inline]
                pub fn clear(self) -> Self {
                    Self { n: self.n, data: None }
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
                    if let Some(v) = self.data {
                        buffer.$write(self.n, v);
                    }
                    Ok(())
                }
            }

            impl IoType for ScalarWriter<$ty> {
                fn datatype(&self) -> PyroType {
                    $pyro
                }
            }

            impl ScalarWriter<Option<$ty>> {
                /// Store the one value; a `Null` is stored as absent.
                pub fn ingest(&mut self, val: &PyroValue<'_>) -> Result<(), WireError> {
                    match val {
                        PyroValue::Null => self.data = Some(None),
                        PyroValue::$Variant(v) => self.data = Some(Some(*v)),
                        other => return Err(mismatch($option_pyro, other)),
                    }
                    Ok(())
                }

                #[inline]
                pub fn clear(self) -> Self {
                    Self { n: self.n, data: None }
                }

                /// Nothing is appended.
                #[inline]
                pub fn variable_length(&self) -> usize {
                    0
                }

                /// Write inline at the inode's data offset and mark it
                /// present; a null leaves the value and its bit zero.
                pub fn write<W: StructureWord>(
                    &self,
                    buffer: &mut SplitSink<'_, W>,
                ) -> Result<(), WireError> {
                    if let Some(Some(v)) = self.data {
                        buffer.write_valid(self.n);
                        buffer.$write(self.n, v);
                    }
                    Ok(())
                }
            }

            impl IoType for ScalarWriter<Option<$ty>> {
                fn datatype(&self) -> PyroType {
                    $option_pyro
                }
            }
        )*
    };
}

use PrimitiveDataType as P;
use PyroType::{OptionPrimitiveScalar as Opt, PrimitiveScalar as Req};

primitive_writer! {
    bool, Bool, write_bool, Req(P::Bool), Opt(P::Bool);
    u8, U8, write_u8, Req(P::U8), Opt(P::U8);
    i8, I8, write_i8, Req(P::I8), Opt(P::I8);
    u16, U16, write_u16, Req(P::U16), Opt(P::U16);
    i16, I16, write_i16, Req(P::I16), Opt(P::I16);
    u32, U32, write_u32, Req(P::U32), Opt(P::U32);
    i32, I32, write_i32, Req(P::I32), Opt(P::I32);
    u64, U64, write_u64, Req(P::U64), Opt(P::U64);
    i64, I64, write_i64, Req(P::I64), Opt(P::I64);
    half::f16, F16, write_f16, Req(P::F16), Opt(P::F16);
    f32, F32, write_f32, Req(P::F32), Opt(P::F32);
    f64, F64, write_f64, Req(P::F64), Opt(P::F64);
    Time, Timestamp, write_time, Req(P::Timestamp), Opt(P::Timestamp);
}

// =============================================================================
// Blobs — a bare `Str` / `Bytes`, borrowed from the value
// =============================================================================

/// `ingest`, `clear`, `write` and `datatype` for one blob type, borrowed
/// from its `PyroValue::$Variant`'s `Cow`.
macro_rules! blob_writer {
    ($($ty:ty, $Variant:ident, $write:ident, $pyro:expr, $len:expr;)*) => {
        $(
            impl<'v> ScalarWriter<&'v $ty> {
                /// Store the one value, borrowed from `val`. A required
                /// field has no `Null`.
                pub fn ingest(&mut self, val: &'v PyroValue<'_>) -> Result<(), WireError> {
                    match val {
                        PyroValue::$Variant(v) => {
                            self.data = Some(Cow::as_ref(v));
                            Ok(())
                        }
                        other => Err(mismatch($pyro, other)),
                    }
                }

                /// Drop the value and the borrow.
                #[inline]
                pub fn clear<'c>(self) -> ScalarWriter<&'c $ty> {
                    ScalarWriter { n: self.n, data: None }
                }

                /// The blob's bytes, appended as-is.
                #[inline]
                pub fn variable_length(&self) -> usize {
                    self.data.map_or(0, $len)
                }

                /// Append the bytes and point the slot at them.
                pub fn write<W: StructureWord>(
                    &self,
                    buffer: &mut SplitSink<'_, W>,
                ) -> Result<(), WireError> {
                    if let Some(v) = self.data {
                        buffer.$write(self.n, v);
                    }
                    Ok(())
                }
            }

            impl<'v> IoType for ScalarWriter<&'v $ty> {
                fn datatype(&self) -> PyroType {
                    $pyro
                }
            }
        )*
    };
}

blob_writer! {
    str, Str, write_str, PyroType::Str, str::len;
    [u8], Bytes, write_bytes, PyroType::Bytes, <[u8]>::len;
}

pub type BoolWriter = ScalarWriter<bool>;
pub type U8Writer = ScalarWriter<u8>;
pub type I8Writer = ScalarWriter<i8>;
pub type U16Writer = ScalarWriter<u16>;
pub type I16Writer = ScalarWriter<i16>;
pub type U32Writer = ScalarWriter<u32>;
pub type I32Writer = ScalarWriter<i32>;
pub type U64Writer = ScalarWriter<u64>;
pub type I64Writer = ScalarWriter<i64>;
pub type F16Writer = ScalarWriter<half::f16>;
pub type F32Writer = ScalarWriter<f32>;
pub type F64Writer = ScalarWriter<f64>;
pub type TimestampWriter = ScalarWriter<Time>;

pub type OptionBoolWriter = ScalarWriter<Option<bool>>;
pub type OptionU8Writer = ScalarWriter<Option<u8>>;
pub type OptionI8Writer = ScalarWriter<Option<i8>>;
pub type OptionU16Writer = ScalarWriter<Option<u16>>;
pub type OptionI16Writer = ScalarWriter<Option<i16>>;
pub type OptionU32Writer = ScalarWriter<Option<u32>>;
pub type OptionI32Writer = ScalarWriter<Option<i32>>;
pub type OptionU64Writer = ScalarWriter<Option<u64>>;
pub type OptionI64Writer = ScalarWriter<Option<i64>>;
pub type OptionF16Writer = ScalarWriter<Option<half::f16>>;
pub type OptionF32Writer = ScalarWriter<Option<f32>>;
pub type OptionF64Writer = ScalarWriter<Option<f64>>;
pub type OptionTimestampWriter = ScalarWriter<Option<Time>>;
