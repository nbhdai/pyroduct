use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt;
pub mod coerce;
mod repair;
mod schema;
mod time;

use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;
use bitvec::vec::BitVec;
pub use repair::ScalarRepairError;
pub use schema::ValueSchemaInferenceError;
pub use time::Time;

use crate::Map;
use crate::errors::{ValueError, ValueResult};
use crate::schema::{ChildIter, FieldName, PyroType};

use half::f16;
use serde::Deserialize; // Standard Serde traits

mod display;
mod gets;
mod pretty_print;
mod try_to_v;
mod v_from;

pub use pretty_print::{RowDisplayMode, RowPrinter};

// -----------------------------------------------------------------------------
// Type Aliases
// -----------------------------------------------------------------------------

pub type PyroRowOwned = PyroRow<'static>;
pub type PyroValueOwned = PyroValue<'static>;

// -----------------------------------------------------------------------------
// Primitive Value List
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum PrimitiveValueList<'a> {
    Bool(Cow<'a, BitSlice<u8>>),
    U8(Cow<'a, [u8]>),
    U16(Cow<'a, [u16]>),
    U32(Cow<'a, [u32]>),
    U64(Cow<'a, [u64]>),
    I8(Cow<'a, [i8]>),
    I16(Cow<'a, [i16]>),
    I32(Cow<'a, [i32]>),
    I64(Cow<'a, [i64]>),
    F16(Cow<'a, [f16]>),
    F32(Cow<'a, [f32]>),
    F64(Cow<'a, [f64]>),
    Timestamp(Cow<'a, [Time]>),
}

/// Element types that map onto a [`PrimitiveValueList`] variant.
///
/// Numeric slices are borrowed as-is; `bool` is bit-packed, so it always
/// produces an owned list.
pub(crate) trait PrimitiveListElem: Sized + Clone {
    fn borrowed_list(s: &[Self]) -> PrimitiveValueList<'_>;
    fn owned_list(v: Vec<Self>) -> PrimitiveValueList<'static>;
}

impl PrimitiveListElem for bool {
    fn borrowed_list(s: &[Self]) -> PrimitiveValueList<'_> {
        let bits: BitVec<u8> = s.into_iter().collect();
        PrimitiveValueList::Bool(Cow::Owned(bits))
    }
    fn owned_list(v: Vec<Self>) -> PrimitiveValueList<'static> {
        let bits: BitVec<u8> = v.into_iter().collect();
        PrimitiveValueList::Bool(Cow::Owned(bits))
    }
}

macro_rules! impl_primitive_list_elem {
    ($($t:ty => $variant:ident),* $(,)?) => {
        $(
            impl PrimitiveListElem for $t {
                fn borrowed_list(s: &[Self]) -> PrimitiveValueList<'_> {
                    PrimitiveValueList::$variant(Cow::Borrowed(s))
                }
                fn owned_list(v: Vec<Self>) -> PrimitiveValueList<'static> {
                    PrimitiveValueList::$variant(Cow::Owned(v))
                }
            }
        )*
    };
}

impl_primitive_list_elem!(
    u8 => U8, u16 => U16, u32 => U32, u64 => U64,
    i8 => I8, i16 => I16, i32 => I32, i64 => I64,
    f16 => F16, f32 => F32, f64 => F64,
    Time => Timestamp,
);

impl<'a> PrimitiveValueList<'a> {
    /// Borrows `bits` as a `Bool` list of `bits.len()` values.
    pub fn from_bits(bits: &'a BitSlice<u8>) -> Self {
        PrimitiveValueList::Bool(Cow::Borrowed(bits))
    }

    /// The live bits of a `Bool` list (the first `count` bits), or `None`.
    pub fn as_bits(&self) -> Option<&BitSlice<u8>> {
        match self {
            PrimitiveValueList::Bool(v) => Some(&v),
            _ => None,
        }
    }
}

impl<'a> PrimitiveValueList<'a> {
    /// Creates a fully owned, static version of this list by cloning the data.
    pub fn to_static(&self) -> PrimitiveValueList<'static> {
        match self {
            PrimitiveValueList::Bool(v) => PrimitiveValueList::Bool(Cow::Owned(v.to_bitvec())),
            PrimitiveValueList::U8(v) => PrimitiveValueList::U8(Cow::Owned(v.to_vec())),
            PrimitiveValueList::U16(v) => PrimitiveValueList::U16(Cow::Owned(v.to_vec())),
            PrimitiveValueList::U32(v) => PrimitiveValueList::U32(Cow::Owned(v.to_vec())),
            PrimitiveValueList::U64(v) => PrimitiveValueList::U64(Cow::Owned(v.to_vec())),
            PrimitiveValueList::I8(v) => PrimitiveValueList::I8(Cow::Owned(v.to_vec())),
            PrimitiveValueList::I16(v) => PrimitiveValueList::I16(Cow::Owned(v.to_vec())),
            PrimitiveValueList::I32(v) => PrimitiveValueList::I32(Cow::Owned(v.to_vec())),
            PrimitiveValueList::I64(v) => PrimitiveValueList::I64(Cow::Owned(v.to_vec())),
            PrimitiveValueList::F16(v) => PrimitiveValueList::F16(Cow::Owned(v.to_vec())),
            PrimitiveValueList::F32(v) => PrimitiveValueList::F32(Cow::Owned(v.to_vec())),
            PrimitiveValueList::F64(v) => PrimitiveValueList::F64(Cow::Owned(v.to_vec())),
            PrimitiveValueList::Timestamp(v) => {
                PrimitiveValueList::Timestamp(Cow::Owned(v.to_vec()))
            }
        }
    }

    pub fn into_owned(self) -> PrimitiveValueList<'static> {
        match self {
            PrimitiveValueList::Bool(v) => PrimitiveValueList::Bool(Cow::Owned(v.into_owned())),
            PrimitiveValueList::U8(v) => PrimitiveValueList::U8(Cow::Owned(v.into_owned())),
            PrimitiveValueList::U16(v) => PrimitiveValueList::U16(Cow::Owned(v.into_owned())),
            PrimitiveValueList::U32(v) => PrimitiveValueList::U32(Cow::Owned(v.into_owned())),
            PrimitiveValueList::U64(v) => PrimitiveValueList::U64(Cow::Owned(v.into_owned())),
            PrimitiveValueList::I8(v) => PrimitiveValueList::I8(Cow::Owned(v.into_owned())),
            PrimitiveValueList::I16(v) => PrimitiveValueList::I16(Cow::Owned(v.into_owned())),
            PrimitiveValueList::I32(v) => PrimitiveValueList::I32(Cow::Owned(v.into_owned())),
            PrimitiveValueList::I64(v) => PrimitiveValueList::I64(Cow::Owned(v.into_owned())),
            PrimitiveValueList::F16(v) => PrimitiveValueList::F16(Cow::Owned(v.into_owned())),
            PrimitiveValueList::F32(v) => PrimitiveValueList::F32(Cow::Owned(v.into_owned())),
            PrimitiveValueList::F64(v) => PrimitiveValueList::F64(Cow::Owned(v.into_owned())),
            PrimitiveValueList::Timestamp(v) => {
                PrimitiveValueList::Timestamp(Cow::Owned(v.into_owned()))
            }
        }
    }

    pub fn to_ref(&self) -> PrimitiveValueList<'_> {
        match self {
            PrimitiveValueList::Bool(v) => PrimitiveValueList::Bool(Cow::Borrowed(v.as_ref())),
            PrimitiveValueList::U8(v) => PrimitiveValueList::U8(Cow::Borrowed(v.as_ref())),
            PrimitiveValueList::U16(v) => PrimitiveValueList::U16(Cow::Borrowed(v.as_ref())),
            PrimitiveValueList::U32(v) => PrimitiveValueList::U32(Cow::Borrowed(v.as_ref())),
            PrimitiveValueList::U64(v) => PrimitiveValueList::U64(Cow::Borrowed(v.as_ref())),
            PrimitiveValueList::I8(v) => PrimitiveValueList::I8(Cow::Borrowed(v.as_ref())),
            PrimitiveValueList::I16(v) => PrimitiveValueList::I16(Cow::Borrowed(v.as_ref())),
            PrimitiveValueList::I32(v) => PrimitiveValueList::I32(Cow::Borrowed(v.as_ref())),
            PrimitiveValueList::I64(v) => PrimitiveValueList::I64(Cow::Borrowed(v.as_ref())),
            PrimitiveValueList::F16(v) => PrimitiveValueList::F16(Cow::Borrowed(v.as_ref())),
            PrimitiveValueList::F32(v) => PrimitiveValueList::F32(Cow::Borrowed(v.as_ref())),
            PrimitiveValueList::F64(v) => PrimitiveValueList::F64(Cow::Borrowed(v.as_ref())),
            PrimitiveValueList::Timestamp(v) => {
                PrimitiveValueList::Timestamp(Cow::Borrowed(v.as_ref()))
            }
        }
    }

    pub fn len(&self) -> usize {
        match self {
            PrimitiveValueList::Bool(v) => v.len(),
            PrimitiveValueList::U8(v) => v.len(),
            PrimitiveValueList::U16(v) => v.len(),
            PrimitiveValueList::U32(v) => v.len(),
            PrimitiveValueList::U64(v) => v.len(),
            PrimitiveValueList::I8(v) => v.len(),
            PrimitiveValueList::I16(v) => v.len(),
            PrimitiveValueList::I32(v) => v.len(),
            PrimitiveValueList::I64(v) => v.len(),
            PrimitiveValueList::F16(v) => v.len(),
            PrimitiveValueList::F32(v) => v.len(),
            PrimitiveValueList::F64(v) => v.len(),
            PrimitiveValueList::Timestamp(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            PrimitiveValueList::Bool(v) => v.is_empty(),
            PrimitiveValueList::U8(v) => v.is_empty(),
            PrimitiveValueList::U16(v) => v.is_empty(),
            PrimitiveValueList::U32(v) => v.is_empty(),
            PrimitiveValueList::U64(v) => v.is_empty(),
            PrimitiveValueList::I8(v) => v.is_empty(),
            PrimitiveValueList::I16(v) => v.is_empty(),
            PrimitiveValueList::I32(v) => v.is_empty(),
            PrimitiveValueList::I64(v) => v.is_empty(),
            PrimitiveValueList::F16(v) => v.is_empty(),
            PrimitiveValueList::F32(v) => v.is_empty(),
            PrimitiveValueList::F64(v) => v.is_empty(),
            PrimitiveValueList::Timestamp(v) => v.is_empty(),
        }
    }

    pub fn iter_numbers(&self) -> Option<PrimitiveNumericIterator<'_>> {
        match self {
            PrimitiveValueList::U8(v) => Some(PrimitiveNumericIterator::U8(v.iter())),
            PrimitiveValueList::U16(v) => Some(PrimitiveNumericIterator::U16(v.iter())),
            PrimitiveValueList::U32(v) => Some(PrimitiveNumericIterator::U32(v.iter())),
            PrimitiveValueList::U64(v) => Some(PrimitiveNumericIterator::U64(v.iter())),
            PrimitiveValueList::I8(v) => Some(PrimitiveNumericIterator::I8(v.iter())),
            PrimitiveValueList::I16(v) => Some(PrimitiveNumericIterator::I16(v.iter())),
            PrimitiveValueList::I32(v) => Some(PrimitiveNumericIterator::I32(v.iter())),
            PrimitiveValueList::I64(v) => Some(PrimitiveNumericIterator::I64(v.iter())),
            _ => None,
        }
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, PrimitiveValueList::Bool(..))
    }

    pub fn is_number(&self) -> bool {
        matches!(
            self,
            PrimitiveValueList::U8(_)
                | PrimitiveValueList::U16(_)
                | PrimitiveValueList::U32(_)
                | PrimitiveValueList::U64(_)
                | PrimitiveValueList::I8(_)
                | PrimitiveValueList::I16(_)
                | PrimitiveValueList::I32(_)
                | PrimitiveValueList::I64(_)
        )
    }

    pub fn iter_floats(&self) -> Option<PrimitiveFloatIterator<'_>> {
        match self {
            PrimitiveValueList::F16(v) => Some(PrimitiveFloatIterator::F16(v.iter())),
            PrimitiveValueList::F32(v) => Some(PrimitiveFloatIterator::F32(v.iter())),
            PrimitiveValueList::F64(v) => Some(PrimitiveFloatIterator::F64(v.iter())),
            _ => None,
        }
    }

    pub fn is_float(&self) -> bool {
        matches!(
            self,
            PrimitiveValueList::F16(_) | PrimitiveValueList::F32(_) | PrimitiveValueList::F64(_)
        )
    }

    /// Returns an iterator that yields all values as `i128`.
    /// Floats are rounded to the nearest integer.
    pub fn iter(&self) -> PrimitiveIter<'_> {
        match self {
            PrimitiveValueList::Bool(v) => PrimitiveIter::Bool(v.iter()),
            PrimitiveValueList::U8(v) => PrimitiveIter::U8(v.iter()),
            PrimitiveValueList::U16(v) => PrimitiveIter::U16(v.iter()),
            PrimitiveValueList::U32(v) => PrimitiveIter::U32(v.iter()),
            PrimitiveValueList::U64(v) => PrimitiveIter::U64(v.iter()),
            PrimitiveValueList::I8(v) => PrimitiveIter::I8(v.iter()),
            PrimitiveValueList::I16(v) => PrimitiveIter::I16(v.iter()),
            PrimitiveValueList::I32(v) => PrimitiveIter::I32(v.iter()),
            PrimitiveValueList::I64(v) => PrimitiveIter::I64(v.iter()),
            PrimitiveValueList::F16(v) => PrimitiveIter::F16(v.iter()),
            PrimitiveValueList::F32(v) => PrimitiveIter::F32(v.iter()),
            PrimitiveValueList::F64(v) => PrimitiveIter::F64(v.iter()),
            PrimitiveValueList::Timestamp(v) => PrimitiveIter::Timestamp(v.iter()),
        }
    }

    /// Returns an iterator that yields all values as `i128`.
    /// Floats are rounded to the nearest integer.
    pub fn force_iter_numbers(&self) -> PrimitiveForceNumericIterator<'_> {
        match self {
            PrimitiveValueList::Bool(v) => PrimitiveForceNumericIterator::Bool(v.iter()),
            PrimitiveValueList::U8(v) => PrimitiveForceNumericIterator::U8(v.iter()),
            PrimitiveValueList::U16(v) => PrimitiveForceNumericIterator::U16(v.iter()),
            PrimitiveValueList::U32(v) => PrimitiveForceNumericIterator::U32(v.iter()),
            PrimitiveValueList::U64(v) => PrimitiveForceNumericIterator::U64(v.iter()),
            PrimitiveValueList::I8(v) => PrimitiveForceNumericIterator::I8(v.iter()),
            PrimitiveValueList::I16(v) => PrimitiveForceNumericIterator::I16(v.iter()),
            PrimitiveValueList::I32(v) => PrimitiveForceNumericIterator::I32(v.iter()),
            PrimitiveValueList::I64(v) => PrimitiveForceNumericIterator::I64(v.iter()),
            PrimitiveValueList::F16(v) => PrimitiveForceNumericIterator::F16(v.iter()),
            PrimitiveValueList::F32(v) => PrimitiveForceNumericIterator::F32(v.iter()),
            PrimitiveValueList::F64(v) => PrimitiveForceNumericIterator::F64(v.iter()),
            PrimitiveValueList::Timestamp(v) => PrimitiveForceNumericIterator::Timestamp(v.iter()),
        }
    }

    /// Returns an iterator that yields all values as `f64`.
    /// Integers and Bools are cast to `f64`.
    pub fn force_iter_floats(&self) -> PrimitiveForceFloatIterator<'_> {
        match self {
            PrimitiveValueList::Bool(v) => PrimitiveForceFloatIterator::Bool(v.iter()),
            PrimitiveValueList::U8(v) => PrimitiveForceFloatIterator::U8(v.iter()),
            PrimitiveValueList::U16(v) => PrimitiveForceFloatIterator::U16(v.iter()),
            PrimitiveValueList::U32(v) => PrimitiveForceFloatIterator::U32(v.iter()),
            PrimitiveValueList::U64(v) => PrimitiveForceFloatIterator::U64(v.iter()),
            PrimitiveValueList::I8(v) => PrimitiveForceFloatIterator::I8(v.iter()),
            PrimitiveValueList::I16(v) => PrimitiveForceFloatIterator::I16(v.iter()),
            PrimitiveValueList::I32(v) => PrimitiveForceFloatIterator::I32(v.iter()),
            PrimitiveValueList::I64(v) => PrimitiveForceFloatIterator::I64(v.iter()),
            PrimitiveValueList::F16(v) => PrimitiveForceFloatIterator::F16(v.iter()),
            PrimitiveValueList::F32(v) => PrimitiveForceFloatIterator::F32(v.iter()),
            PrimitiveValueList::F64(v) => PrimitiveForceFloatIterator::F64(v.iter()),
            PrimitiveValueList::Timestamp(v) => PrimitiveForceFloatIterator::Timestamp(v.iter()),
        }
    }
}

pub enum PrimitiveIter<'a> {
    Bool(bitvec::slice::Iter<'a, u8, Lsb0>),
    U8(std::slice::Iter<'a, u8>),
    U16(std::slice::Iter<'a, u16>),
    U32(std::slice::Iter<'a, u32>),
    U64(std::slice::Iter<'a, u64>),
    I8(std::slice::Iter<'a, i8>),
    I16(std::slice::Iter<'a, i16>),
    I32(std::slice::Iter<'a, i32>),
    I64(std::slice::Iter<'a, i64>),
    F16(std::slice::Iter<'a, f16>),
    F32(std::slice::Iter<'a, f32>),
    F64(std::slice::Iter<'a, f64>),
    Timestamp(std::slice::Iter<'a, Time>),
}

impl<'a> Iterator for PrimitiveIter<'a> {
    type Item = PyroValue<'static>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            PrimitiveIter::Bool(iter) => iter.next().map(|x| PyroValue::Bool(*x)),
            PrimitiveIter::U8(iter) => iter.next().map(|&x| PyroValue::U8(x)),
            PrimitiveIter::U16(iter) => iter.next().map(|&x| PyroValue::U16(x)),
            PrimitiveIter::U32(iter) => iter.next().map(|&x| PyroValue::U32(x)),
            PrimitiveIter::U64(iter) => iter.next().map(|&x| PyroValue::U64(x)),
            PrimitiveIter::I8(iter) => iter.next().map(|&x| PyroValue::I8(x)),
            PrimitiveIter::I16(iter) => iter.next().map(|&x| PyroValue::I16(x)),
            PrimitiveIter::I32(iter) => iter.next().map(|&x| PyroValue::I32(x)),
            PrimitiveIter::I64(iter) => iter.next().map(|&x| PyroValue::I64(x)),
            PrimitiveIter::F16(iter) => iter.next().map(|&x| PyroValue::F16(x)),
            PrimitiveIter::F32(iter) => iter.next().map(|&x| PyroValue::F32(x)),
            PrimitiveIter::F64(iter) => iter.next().map(|&x| PyroValue::F64(x)),
            PrimitiveIter::Timestamp(iter) => iter.next().map(|&t| PyroValue::Timestamp(t)),
        }
    }
}

pub enum PrimitiveNumericIterator<'a> {
    Bool(bitvec::slice::Iter<'a, u8, Lsb0>),
    U8(std::slice::Iter<'a, u8>),
    U16(std::slice::Iter<'a, u16>),
    U32(std::slice::Iter<'a, u32>),
    U64(std::slice::Iter<'a, u64>),
    I8(std::slice::Iter<'a, i8>),
    I16(std::slice::Iter<'a, i16>),
    I32(std::slice::Iter<'a, i32>),
    I64(std::slice::Iter<'a, i64>),
}

impl<'a> Iterator for PrimitiveNumericIterator<'a> {
    type Item = i128;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            PrimitiveNumericIterator::Bool(iter) => iter.next().map(|x| *x as i128),
            PrimitiveNumericIterator::U8(iter) => iter.next().map(|&x| x as i128),
            PrimitiveNumericIterator::U16(iter) => iter.next().map(|&x| x as i128),
            PrimitiveNumericIterator::U32(iter) => iter.next().map(|&x| x as i128),
            PrimitiveNumericIterator::U64(iter) => iter.next().map(|&x| x as i128),
            PrimitiveNumericIterator::I8(iter) => iter.next().map(|&x| x as i128),
            PrimitiveNumericIterator::I16(iter) => iter.next().map(|&x| x as i128),
            PrimitiveNumericIterator::I32(iter) => iter.next().map(|&x| x as i128),
            PrimitiveNumericIterator::I64(iter) => iter.next().map(|&x| x as i128),
        }
    }
}

pub enum PrimitiveFloatIterator<'a> {
    F16(std::slice::Iter<'a, f16>),
    F32(std::slice::Iter<'a, f32>),
    F64(std::slice::Iter<'a, f64>),
}

impl<'a> Iterator for PrimitiveFloatIterator<'a> {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            PrimitiveFloatIterator::F16(iter) => iter.next().map(|x| x.to_f64()),
            PrimitiveFloatIterator::F32(iter) => iter.next().map(|&x| x as f64),
            PrimitiveFloatIterator::F64(iter) => iter.next().copied(),
        }
    }
}

impl<'a> PartialEq for PrimitiveValueList<'a> {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len()
            || self.is_number() != other.is_number()
            || self.is_bool() != other.is_bool()
        {
            return false;
        }

        if self.is_bool() {
            return match (self, other) {
                (PrimitiveValueList::Bool(a), PrimitiveValueList::Bool(b)) => a == b,
                _ => false,
            };
        }

        match (self, other) {
            (PrimitiveValueList::Timestamp(a), PrimitiveValueList::Timestamp(b)) => return a == b,
            (PrimitiveValueList::Timestamp(_), _) | (_, PrimitiveValueList::Timestamp(_)) => {
                return false;
            }
            _ => {}
        }

        // 1. Compare as Integers (if both are integer types)
        if let (Some(iter_a), Some(iter_b)) = (self.iter_numbers(), other.iter_numbers()) {
            return iter_a.zip(iter_b).all(|(a, b)| a == b);
        }

        // 2. Compare as Floats (if both are float types)
        if let (Some(iter_a), Some(iter_b)) = (self.iter_floats(), other.iter_floats()) {
            return iter_a.zip(iter_b).all(|(a, b)| a == b);
        }

        // 3. Fallback: If types are mixed (e.g., Int vs Float), they are not equal.
        false
    }
}

pub enum PrimitiveForceNumericIterator<'a> {
    Bool(bitvec::slice::Iter<'a, u8, Lsb0>),
    U8(std::slice::Iter<'a, u8>),
    U16(std::slice::Iter<'a, u16>),
    U32(std::slice::Iter<'a, u32>),
    U64(std::slice::Iter<'a, u64>),
    I8(std::slice::Iter<'a, i8>),
    I16(std::slice::Iter<'a, i16>),
    I32(std::slice::Iter<'a, i32>),
    I64(std::slice::Iter<'a, i64>),
    F16(std::slice::Iter<'a, f16>),
    F32(std::slice::Iter<'a, f32>),
    F64(std::slice::Iter<'a, f64>),
    Timestamp(std::slice::Iter<'a, Time>),
}

impl<'a> Iterator for PrimitiveForceNumericIterator<'a> {
    type Item = i128;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            PrimitiveForceNumericIterator::Bool(iter) => iter.next().map(|x| *x as i128),
            PrimitiveForceNumericIterator::U8(iter) => iter.next().map(|&x| x as i128),
            PrimitiveForceNumericIterator::U16(iter) => iter.next().map(|&x| x as i128),
            PrimitiveForceNumericIterator::U32(iter) => iter.next().map(|&x| x as i128),
            PrimitiveForceNumericIterator::U64(iter) => iter.next().map(|&x| x as i128),
            PrimitiveForceNumericIterator::I8(iter) => iter.next().map(|&x| x as i128),
            PrimitiveForceNumericIterator::I16(iter) => iter.next().map(|&x| x as i128),
            PrimitiveForceNumericIterator::I32(iter) => iter.next().map(|&x| x as i128),
            PrimitiveForceNumericIterator::I64(iter) => iter.next().map(|&x| x as i128),

            PrimitiveForceNumericIterator::F16(iter) => {
                iter.next().map(|x| x.to_f64().round() as i128)
            }
            PrimitiveForceNumericIterator::F32(iter) => iter.next().map(|&x| x.round() as i128),
            PrimitiveForceNumericIterator::F64(iter) => iter.next().map(|&x| x.round() as i128),
            PrimitiveForceNumericIterator::Timestamp(iter) => iter.next().map(|t| t.0),
        }
    }
}

pub enum PrimitiveForceFloatIterator<'a> {
    Bool(bitvec::slice::Iter<'a, u8, Lsb0>),
    U8(std::slice::Iter<'a, u8>),
    U16(std::slice::Iter<'a, u16>),
    U32(std::slice::Iter<'a, u32>),
    U64(std::slice::Iter<'a, u64>),
    I8(std::slice::Iter<'a, i8>),
    I16(std::slice::Iter<'a, i16>),
    I32(std::slice::Iter<'a, i32>),
    I64(std::slice::Iter<'a, i64>),
    F16(std::slice::Iter<'a, f16>),
    F32(std::slice::Iter<'a, f32>),
    F64(std::slice::Iter<'a, f64>),
    Timestamp(std::slice::Iter<'a, Time>),
}

impl<'a> Iterator for PrimitiveForceFloatIterator<'a> {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            // Int/Bool to Float logic: Direct cast
            PrimitiveForceFloatIterator::Bool(iter) => iter.next().map(|x| *x as i8 as f64),
            PrimitiveForceFloatIterator::U8(iter) => iter.next().map(|&x| x as f64),
            PrimitiveForceFloatIterator::U16(iter) => iter.next().map(|&x| x as f64),
            PrimitiveForceFloatIterator::U32(iter) => iter.next().map(|&x| x as f64),
            PrimitiveForceFloatIterator::U64(iter) => iter.next().map(|&x| x as f64),
            PrimitiveForceFloatIterator::I8(iter) => iter.next().map(|&x| x as f64),
            PrimitiveForceFloatIterator::I16(iter) => iter.next().map(|&x| x as f64),
            PrimitiveForceFloatIterator::I32(iter) => iter.next().map(|&x| x as f64),
            PrimitiveForceFloatIterator::I64(iter) => iter.next().map(|&x| x as f64),
            PrimitiveForceFloatIterator::F16(iter) => iter.next().map(|x| x.to_f64()),
            PrimitiveForceFloatIterator::F32(iter) => iter.next().map(|&x| x as f64),
            PrimitiveForceFloatIterator::F64(iter) => iter.next().copied(),
            PrimitiveForceFloatIterator::Timestamp(iter) => iter.next().map(|t| t.0 as f64),
        }
    }
}

// -----------------------------------------------------------------------------
// Arrow Row Item
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct RowItem<'a> {
    pub key: FieldName<'a>,
    pub value: PyroValue<'a>,
}

// Implement Serde manually to act like a Tuple (Key, Value)
impl<'a> serde::Serialize for RowItem<'a> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeTuple;
        let mut tup = serializer.serialize_tuple(2)?;
        tup.serialize_element(&self.key)?;
        tup.serialize_element(&self.value)?;
        tup.end()
    }
}

impl<'a, 'de: 'a> serde::Deserialize<'de> for RowItem<'a> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct TupleVisitor<'a>(std::marker::PhantomData<&'a ()>);

        impl<'a, 'de: 'a> serde::de::Visitor<'de> for TupleVisitor<'a> {
            type Value = RowItem<'a>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a tuple of (String, PyroValue)")
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let key: FieldName<'a> = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                let value: PyroValue<'a> = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
                Ok(RowItem { key, value })
            }
        }

        deserializer.deserialize_tuple(2, TupleVisitor(std::marker::PhantomData))
    }
}

// -----------------------------------------------------------------------------
// Arrow Row
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PyroRow<'a>(pub Vec<RowItem<'a>>);

impl<'a> serde::Serialize for PyroRow<'a> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        // Start a map serialization
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for item in &self.0 {
            // Serialize entries directly using the Key (Bow<str>) and Value (PyroValue)
            map.serialize_entry(&item.key, &item.value)?;
        }
        map.end()
    }
}

// Manual implementation of Deserialize to support both Map (JSON Object) and Seq (Internal Tuple format)
impl<'a, 'de: 'a> Deserialize<'de> for PyroRow<'a> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct PyroRowVisitor<'a>(std::marker::PhantomData<&'a ()>);

        impl<'a, 'de: 'a> serde::de::Visitor<'de> for PyroRowVisitor<'a> {
            type Value = PyroRow<'a>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map or sequence of (key, value) tuples")
            }

            // Handle JSON Arrays (Internal Format: [[k,v], [k,v]])
            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut items = Vec::new();
                while let Some(item) = seq.next_element::<RowItem<'a>>()? {
                    items.push(item);
                }
                Ok(PyroRow(items))
            }

            // Handle JSON Objects (Standard Format: {"k": v})
            fn visit_map<M>(self, mut map: M) -> std::result::Result<Self::Value, M::Error>
            where
                M: serde::de::MapAccess<'de>,
            {
                let mut items = Vec::with_capacity(map.size_hint().unwrap_or(0));
                while let Some((key, value)) = map.next_entry::<String, PyroValue<'a>>()? {
                    items.push(RowItem {
                        key: key.into(),
                        value,
                    });
                }
                Ok(PyroRow(items))
            }
        }

        deserializer.deserialize_any(PyroRowVisitor(std::marker::PhantomData))
    }
}

impl<'a> PyroRow<'a> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn empty() -> Self {
        Self(Vec::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    pub fn get(&self, key: &str) -> Option<&PyroValue<'a>> {
        self.0
            .iter()
            .find(|item| item.key == key)
            .map(|item| &item.value)
    }

    pub fn insert(&mut self, key: String, value: PyroValue<'a>) {
        if let Some(pos) = self
            .0
            .iter()
            .position(|item| item.key.as_ref() == key.as_str())
        {
            self.0[pos] = RowItem {
                key: key.into(),
                value,
            };
        } else {
            self.0.push(RowItem {
                key: key.into(),
                value,
            });
        }
    }

    pub fn insert_ref(&mut self, key: &'a str, value: PyroValue<'a>) {
        if let Some(pos) = self.0.iter().position(|item| item.key.as_ref() == key) {
            self.0[pos] = RowItem {
                key: key.into(),
                value,
            };
        } else {
            self.0.push(RowItem {
                key: key.into(),
                value,
            });
        }
    }

    pub fn extend(&mut self, other: Self) {
        for item in other.0 {
            if self.get(item.key.as_ref()).is_some() {
                panic!("Cannot extend with overlap");
            }
            self.0.push(item);
        }
    }

    pub fn into_owned(self) -> PyroRow<'static> {
        PyroRow(
            self.0
                .into_iter()
                .map(|item| RowItem {
                    key: item.key.to_static(),
                    value: item.value.to_static(),
                })
                .collect(),
        )
    }

    /// Projects the Row onto the given Schema (List of PyroFields).
    /// This handles recursive projection for nested Groups.
    pub fn project<'b: 'a>(&self, schema: ChildIter<'b>) -> ValueResult<'_, PyroRow<'a>> {
        let mut new_vec = Vec::with_capacity(schema.len());

        for f in schema {
            let target_name = f.name();

            // Find the item in the current row
            let entry = self.0.iter().find(|item| item.key == target_name);

            if let Some(item) = entry {
                match (f.datatype(), &item.value) {
                    // Case 1: Schema expects Group, Value is Group -> Recurse
                    (PyroType::Group(_), PyroValue::Group(inner_row)) => {
                        new_vec.push(RowItem {
                            key: item.key.clone(),
                            value: PyroValue::Group(
                                inner_row.project(f.group_children().unwrap())?,
                            ),
                        });
                    }

                    // Case 2: Schema expects Group, Value is NOT Group -> Error
                    (PyroType::Group(_), val) => {
                        // Assuming ValueError::InvalidScalar can take the value that was found
                        return Err(ValueError::invalid_large(val));
                    }

                    // Case 3: Value is Group, but Schema expects Primitive/Scalar -> Error
                    (_, PyroValue::Group(val)) => {
                        // We cannot project a scalar out of a whole Group
                        return Err(ValueError::invalid_large(val));
                    }

                    // Case 4: Standard value match (Primitive/List/Map) -> Copy/Clone
                    _ => {
                        new_vec.push(RowItem {
                            key: item.key.clone(),
                            value: item.value.clone(),
                        });
                    }
                }
            } else {
                new_vec.push(RowItem {
                    key: target_name.into(),
                    value: PyroValue::Null,
                });
            }
        }
        Ok(PyroRow(new_vec))
    }

    pub fn get_deep<S: AsRef<str>>(&self, path: &[S]) -> Option<&PyroValue<'a>> {
        if path.is_empty() {
            return None;
        }

        let mut current_row = self;
        let mut iter = path.iter();

        // We handle the iteration manually to distinguish the "middle" (Groups)
        // from the "end" (Target Value).
        while let Some(key) = iter.next() {
            let key_str = key.as_ref();

            // Look up the value
            match current_row.get(key_str) {
                Some(val) => {
                    // If this is the last item in the path, return the value found
                    if iter.len() == 0 {
                        return Some(val);
                    }

                    // If not the last item, we expect a Group to continue traversing
                    match val {
                        PyroValue::Group(next_row) => {
                            current_row = next_row;
                        }
                        _ => return None, // Path blocked: expected Group, found scalar/list
                    }
                }
                None => return None, // Key not found
            }
        }

        None
    }
}

impl<'a> PyroRow<'a> {
    pub fn iter(&self) -> impl Iterator<Item = (&str, &PyroValue<'a>)> {
        self.0.iter().map(|item| (item.key.as_ref(), &item.value))
    }

    pub fn values(&self) -> impl Iterator<Item = &PyroValue<'a>> {
        self.0.iter().map(|item| &item.value)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'a> IntoIterator for PyroRow<'a> {
    type Item = (String, PyroValue<'a>);
    type IntoIter = std::vec::IntoIter<(String, PyroValue<'a>)>;

    fn into_iter(self) -> Self::IntoIter {
        // Must convert RowItems to tuples to maintain expected API
        let vec: Vec<(String, PyroValue<'a>)> = self
            .0
            .into_iter()
            .map(|item| (item.key.as_ref().to_string(), item.value))
            .collect();
        vec.into_iter()
    }
}

// -----------------------------------------------------------------------------
// Arrow Value
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum PyroValue<'a> {
    #[default]
    Null,
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F16(f16),
    F32(f32),
    F64(f64),
    Timestamp(Time),
    #[serde(borrow)]
    PrimitiveList(PrimitiveValueList<'a>),
    #[serde(borrow)]
    Str(Cow<'a, str>),
    Bytes(Cow<'a, [u8]>),
    // #[serde(borrow)]
    // Bytes(Cow<'a, [u8]>),
    #[serde(borrow)]
    Group(PyroRow<'a>),
    #[serde(borrow)]
    List(Vec<PyroValue<'a>>),
    #[serde(borrow)]
    Map(Map<PyroValue<'a>, PyroValue<'a>>),
}

impl<'a> PyroValue<'a> {
    pub fn is_null(&self) -> bool {
        matches!(self, PyroValue::Null)
    }

    pub fn to_static(self) -> PyroValue<'static> {
        match self {
            PyroValue::Null => PyroValue::Null,
            PyroValue::Bool(v) => PyroValue::Bool(v),
            PyroValue::I8(v) => PyroValue::I8(v),
            PyroValue::I16(v) => PyroValue::I16(v),
            PyroValue::I32(v) => PyroValue::I32(v),
            PyroValue::I64(v) => PyroValue::I64(v),
            PyroValue::U8(v) => PyroValue::U8(v),
            PyroValue::U16(v) => PyroValue::U16(v),
            PyroValue::U32(v) => PyroValue::U32(v),
            PyroValue::U64(v) => PyroValue::U64(v),
            PyroValue::F16(v) => PyroValue::F16(v),
            PyroValue::F32(v) => PyroValue::F32(v),
            PyroValue::F64(v) => PyroValue::F64(v),
            PyroValue::Timestamp(nanos) => PyroValue::Timestamp(nanos),
            PyroValue::PrimitiveList(l) => PyroValue::PrimitiveList(l.into_owned()),
            PyroValue::Str(s) => PyroValue::Str(Cow::Owned(s.to_string())),
            PyroValue::Bytes(s) => PyroValue::Bytes(Cow::Owned(s.to_vec())),
            PyroValue::Group(g) => PyroValue::Group(g.into_owned()),
            PyroValue::List(l) => PyroValue::List(l.into_iter().map(|v| v.to_static()).collect()),
            PyroValue::Map(l) => PyroValue::Map(
                l.into_iter()
                    .map(|(k, v)| (k.to_static(), v.to_static()))
                    .collect(),
            ),
        }
    }
}

impl<'a> PartialEq for PyroValue<'a> {
    fn eq(&self, other: &Self) -> bool {
        // Helper to normalize integers to i128
        // Note: u64::MAX fits within i128, so this covers all PyroValue int types safely.
        let as_int = |v: &PyroValue| -> Option<i128> {
            match v {
                PyroValue::I8(n) => Some(*n as i128),
                PyroValue::I16(n) => Some(*n as i128),
                PyroValue::I32(n) => Some(*n as i128),
                PyroValue::I64(n) => Some(*n as i128),
                PyroValue::U8(n) => Some(*n as i128),
                PyroValue::U16(n) => Some(*n as i128),
                PyroValue::U32(n) => Some(*n as i128),
                PyroValue::U64(n) => Some(*n as i128),
                _ => None,
            }
        };

        // Helper to normalize floats to f64
        let as_float = |v: &PyroValue| -> Option<f64> {
            match v {
                PyroValue::F16(n) => Some(n.to_f64()),
                PyroValue::F32(n) => Some(*n as f64),
                PyroValue::F64(n) => Some(*n),
                _ => None,
            }
        };

        // 1. Try Integer Equality
        if let (Some(a), Some(b)) = (as_int(self), as_int(other)) {
            return a == b;
        }

        // 2. Try Float Equality
        if let (Some(a), Some(b)) = (as_float(self), as_float(other)) {
            return a == b;
        }

        // 3. Fallback to Strict Variant Equality
        match (self, other) {
            (PyroValue::Null, PyroValue::Null) => true,
            (PyroValue::Bool(a), PyroValue::Bool(b)) => a == b,
            (PyroValue::Str(a), PyroValue::Str(b)) => a == b,
            (PyroValue::Bytes(a), PyroValue::Bytes(b)) => a == b,
            (PyroValue::Timestamp(n1), PyroValue::Timestamp(n2)) => n1 == n2,
            // Recursive types rely on the Inner types' PartialEq.
            // Note: Since PyroRow/Vec contain PyroValues, they will recursively
            // call back into this `eq` implementation.
            (PyroValue::PrimitiveList(a), PyroValue::PrimitiveList(b)) => a == b,
            (PyroValue::Group(a), PyroValue::Group(b)) => a == b,
            (PyroValue::List(a), PyroValue::List(b)) => a == b,
            (PyroValue::Map(a), PyroValue::Map(b)) => a == b,

            _ => false,
        }
    }
}

/// `Eq` by fiat: `==` is an equivalence on everything but NaN (see [`Ord`]).
impl<'a> Eq for PyroValue<'a> {}

impl<'a> PartialOrd for PyroValue<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A total order that agrees with `==`, so a `PyroValue` can key a [`Map`].
///
/// Values order by class first — `Null` < `Bool` < integers < floats <
/// `Timestamp` < `Str` < `Bytes` < `PrimitiveList` < `List` < `Group` <
/// `Map` — and by value within a class. The classes are the ones `==` uses:
/// every integer width and signedness compares as one `i128`, every float
/// width as one `f64`, a `Timestamp` at microsecond granularity, and
/// compound values element by element.
///
/// A `Group` orders by its values alone, field by field from the left like
/// a tuple: the first field decides, then the second, and so on. Field
/// names don't take part. Groups of one schema share their field order, so
/// this is their declaration order.
///
/// Floats that are `==` (including `-0.0` and `0.0`) are `Equal`; any other
/// pair falls back to [`f64::total_cmp`]. That makes NaN one exception:
/// `NaN != NaN`, but a NaN compares `Equal` to a NaN with the same bits.
/// The other is two `Group`s whose values match but whose field names
/// differ: `Equal` here, `!=` under `==`.
impl<'a> Ord for PyroValue<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        let class = |v: &PyroValue| -> u8 {
            match v {
                PyroValue::Null => 0,
                PyroValue::Bool(_) => 1,
                PyroValue::I8(_)
                | PyroValue::I16(_)
                | PyroValue::I32(_)
                | PyroValue::I64(_)
                | PyroValue::U8(_)
                | PyroValue::U16(_)
                | PyroValue::U32(_)
                | PyroValue::U64(_) => 2,
                PyroValue::F16(_) | PyroValue::F32(_) | PyroValue::F64(_) => 3,
                PyroValue::Timestamp(_) => 4,
                PyroValue::Str(_) => 5,
                PyroValue::Bytes(_) => 6,
                PyroValue::PrimitiveList(_) => 7,
                PyroValue::List(_) => 8,
                PyroValue::Group(_) => 9,
                PyroValue::Map(_) => 10,
            }
        };
        let as_int = |v: &PyroValue| -> i128 {
            match v {
                PyroValue::I8(n) => *n as i128,
                PyroValue::I16(n) => *n as i128,
                PyroValue::I32(n) => *n as i128,
                PyroValue::I64(n) => *n as i128,
                PyroValue::U8(n) => *n as i128,
                PyroValue::U16(n) => *n as i128,
                PyroValue::U32(n) => *n as i128,
                PyroValue::U64(n) => *n as i128,
                _ => unreachable!(),
            }
        };
        let as_float = |v: &PyroValue| -> f64 {
            match v {
                PyroValue::F16(n) => n.to_f64(),
                PyroValue::F32(n) => *n as f64,
                PyroValue::F64(n) => *n,
                _ => unreachable!(),
            }
        };

        class(self)
            .cmp(&class(other))
            .then_with(|| match (self, other) {
                (PyroValue::Null, PyroValue::Null) => Ordering::Equal,
                (PyroValue::Bool(a), PyroValue::Bool(b)) => a.cmp(b),
                (PyroValue::Timestamp(a), PyroValue::Timestamp(b)) => {
                    a.0.div_euclid(1_000).cmp(&b.0.div_euclid(1_000))
                }
                (PyroValue::Str(a), PyroValue::Str(b)) => a.cmp(b),
                (PyroValue::Bytes(a), PyroValue::Bytes(b)) => a.cmp(b),
                (PyroValue::PrimitiveList(a), PyroValue::PrimitiveList(b)) => a.cmp(b),
                (PyroValue::List(a), PyroValue::List(b)) => a.cmp(b),
                (PyroValue::Group(a), PyroValue::Group(b)) => {
                    a.0.iter()
                        .map(|item| &item.value)
                        .cmp(b.0.iter().map(|item| &item.value))
                }
                (PyroValue::Map(a), PyroValue::Map(b)) => a.cmp(b),
                _ if class(self) == 2 => as_int(self).cmp(&as_int(other)),
                _ => float_cmp(as_float(self), as_float(other)),
            })
    }
}

/// `==` floats are `Equal` (so `-0.0 == 0.0` holds); anything else by
/// `total_cmp`.
fn float_cmp(a: f64, b: f64) -> Ordering {
    if a == b {
        Ordering::Equal
    } else {
        a.total_cmp(&b)
    }
}

/// Agrees with `PrimitiveValueList`'s `==`: bool lists < integer lists <
/// float lists, then element by element (integers as `i128`, floats as
/// `f64`), a shorter list first when one is a prefix of the other.
impl<'a> Eq for PrimitiveValueList<'a> {}

impl<'a> PartialOrd for PrimitiveValueList<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Ord for PrimitiveValueList<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        let class = |l: &PrimitiveValueList| -> u8 {
            if l.is_bool() {
                0
            } else if l.is_number() {
                1
            } else {
                2
            }
        };
        class(self).cmp(&class(other)).then_with(|| {
            if let (PrimitiveValueList::Bool(a), PrimitiveValueList::Bool(b)) = (self, other) {
                return a.iter().by_vals().cmp(b.iter().by_vals());
            }
            if let (Some(a), Some(b)) = (self.iter_numbers(), other.iter_numbers()) {
                return a.cmp(b);
            }
            if let (Some(a), Some(b)) = (self.iter_floats(), other.iter_floats()) {
                return a
                    .zip(b)
                    .map(|(a, b)| float_cmp(a, b))
                    .find(|o| o.is_ne())
                    .unwrap_or_else(|| self.len().cmp(&other.len()));
            }
            Ordering::Equal
        })
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_keys_sort_and_merge_like_eq() {
        let map = Map::from(vec![
            (PyroValue::Str("b".into()), PyroValue::I32(2)),
            (PyroValue::U8(1), PyroValue::I32(1)),
            (PyroValue::Str("a".into()), PyroValue::I32(3)),
            // `I64(1) == U8(1)`, so this replaces the `U8(1)` entry.
            (PyroValue::I64(1), PyroValue::I32(4)),
            (PyroValue::F32(-0.5), PyroValue::Null),
            (PyroValue::Null, PyroValue::Null),
        ]);
        let keys: Vec<_> = map.keys().cloned().collect();
        assert_eq!(
            keys,
            [
                PyroValue::Null,
                PyroValue::I64(1),
                PyroValue::F32(-0.5),
                PyroValue::Str("a".into()),
                PyroValue::Str("b".into()),
            ]
        );
        assert_eq!(map.get(&PyroValue::U32(1)), Some(&PyroValue::I32(4)));
        assert_eq!(
            PyroValue::F64(0.0).cmp(&PyroValue::F32(-0.0)),
            Ordering::Equal
        );
        assert!(PyroValue::I8(-1) < PyroValue::U64(0));
    }

    #[test]
    fn json_round_trip() {
        // Construct using the helper macros / From traits
        let row = PyroRow::from([
            ("name", PyroValue::from("John Doe")),
            ("age", PyroValue::from(70000)),
            ("child_ages", PyroValue::from(&[0u8, 1, 4][..])), // Primitive list
            (
                "address",
                PyroValue::from(PyroRow::from([
                    ("street", PyroValue::from("10 Downing Street")),
                    ("city", PyroValue::from("London")),
                ])),
            ),
            (
                "phones",
                PyroValue::List(vec![
                    PyroValue::from("+44 1234567"),
                    PyroValue::from("+44 2345678"),
                ]),
            ),
        ]);

        let row_string = serde_json::to_string(&row).unwrap();
        println!("Row_string: {}", row_string);
        let row_reconstructed: PyroRow = serde_json::from_str(&row_string).unwrap();

        // Note: JSON deserialization will produce Owned Bows.
        // The original row has Borrowed Bows.
        // However, partialEq checks value equality, so this should pass.
        assert_eq!(row_reconstructed, row);
    }
}

#[cfg(test)]
mod serialization_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_arrow_row_serializes_to_json_object() {
        // Arrange
        let row = PyroRow::from([
            ("foo", PyroValue::from("bar")),
            ("sin", PyroValue::from("cos")),
            ("count", PyroValue::from(123)),
        ]);

        // Act
        let json_string = serde_json::to_string(&row).expect("Failed to serialize");

        // Assert
        // We expect a Map structure: {"foo":"bar", "sin":"cos", "count":123}
        // Note: We parse it back to a Value to ignore key ordering differences in assertion
        let actual_json: serde_json::Value = serde_json::from_str(&json_string).unwrap();
        let expected_json = json!({
            "foo": "bar",
            "sin": "cos",
            "count": 123
        });

        assert_eq!(
            actual_json, expected_json,
            "Should serialize as a JSON Object, not an Array"
        );
    }

    #[test]
    fn test_nested_arrow_row_serialization() {
        // Arrange: A row containing another row
        let inner_row = PyroRow::from([("inner_key", PyroValue::from("inner_val"))]);

        let outer_row = PyroRow::from([
            ("outer_key", PyroValue::from("outer_val")),
            ("group", PyroValue::from(inner_row)),
        ]);

        // Act
        let json_string = serde_json::to_string(&outer_row).unwrap();

        // Assert
        let expected = json!({
            "outer_key": "outer_val",
            "group": {
                "inner_key": "inner_val"
            }
        });
        let actual: serde_json::Value = serde_json::from_str(&json_string).unwrap();

        assert_eq!(
            actual, expected,
            "Nested rows should serialize as nested JSON objects"
        );
    }

    #[test]
    fn test_deserialize_json_map() {
        // Arrange: Standard JSON object format
        let json_input = r#"{ "name": "Alice", "age": 30 }"#;

        // Act
        let row: PyroRow = serde_json::from_str(json_input).expect("Failed to deserialize map");

        // Assert
        assert_eq!(row.get("name"), Some(&PyroValue::from("Alice")));
        assert_eq!(row.get("age"), Some(&PyroValue::from(30)));
    }

    #[test]
    fn test_deserialize_legacy_array_format() {
        // Arrange: The old "Sequence of Tuples" format: [[key, val], [key, val]]
        // Your custom Deserialize impl has `visit_seq`, so this should still work.
        let json_input = r#"[ ["key1", "value1"], ["key2", 99] ]"#;

        // Act
        let row: PyroRow =
            serde_json::from_str(json_input).expect("Failed to deserialize legacy sequence");

        // Assert
        assert_eq!(row.get("key1"), Some(&PyroValue::from("value1")));
        assert_eq!(row.get("key2"), Some(&PyroValue::from(99)));
    }

    #[test]
    fn test_round_trip() {
        // Arrange
        let original = PyroRow::from([("a", PyroValue::from(1)), ("b", PyroValue::from(2))]);

        // Act
        let serialized = serde_json::to_string(&original).unwrap();
        // Ensure it looks like a map
        assert!(serialized.starts_with('{'));
        assert!(serialized.ends_with('}'));

        let deserialized: PyroRow = serde_json::from_str(&serialized).unwrap();

        // Assert
        assert_eq!(original, deserialized);
    }
}
