use std::borrow::Cow;
use std::fmt;

use crate::format::value::{
    schema::{PyroField, PyroType},
    time::Time,
};

use super::{ValueError, ValueResult};
use half::f16;
use rkyv::with::AsOwned;
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::Deserialize; // Standard Serde traits

mod display;
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

#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
#[serde(untagged)]
pub enum PrimitiveValueList<'a> {
    Bool(#[rkyv(with = AsOwned)] Cow<'a, [bool]>),
    U8(#[rkyv(with = AsOwned)] Cow<'a, [u8]>),
    U16(#[rkyv(with = AsOwned)] Cow<'a, [u16]>),
    U32(#[rkyv(with = AsOwned)] Cow<'a, [u32]>),
    U64(#[rkyv(with = AsOwned)] Cow<'a, [u64]>),
    I8(#[rkyv(with = AsOwned)] Cow<'a, [i8]>),
    I16(#[rkyv(with = AsOwned)] Cow<'a, [i16]>),
    I32(#[rkyv(with = AsOwned)] Cow<'a, [i32]>),
    I64(#[rkyv(with = AsOwned)] Cow<'a, [i64]>),
    F16(#[rkyv(with = AsOwned)] Cow<'a, [f16]>),
    F32(#[rkyv(with = AsOwned)] Cow<'a, [f32]>),
    F64(#[rkyv(with = AsOwned)] Cow<'a, [f64]>),
}

impl<'a> PrimitiveValueList<'a> {
    /// Creates a fully owned, static version of this list by cloning the data.
    pub fn to_static(&self) -> PrimitiveValueList<'static> {
        match self {
            PrimitiveValueList::Bool(v) => PrimitiveValueList::Bool(Cow::Owned(v.to_vec())),
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

    pub fn is_number(&self) -> bool {
        match self {
            PrimitiveValueList::U8(_) => true,
            PrimitiveValueList::U16(_) => true,
            PrimitiveValueList::U32(_) => true,
            PrimitiveValueList::U64(_) => true,
            PrimitiveValueList::I8(_) => true,
            PrimitiveValueList::I16(_) => true,
            PrimitiveValueList::I32(_) => true,
            PrimitiveValueList::I64(_) => true,
            _ => false,
        }
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
        match self {
            PrimitiveValueList::F16(_) => true,
            PrimitiveValueList::F32(_) => true,
            PrimitiveValueList::F64(_) => true,
            _ => false,
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
        }
    }
}

pub enum PrimitiveNumericIterator<'a> {
    Bool(std::slice::Iter<'a, bool>),
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
            PrimitiveNumericIterator::Bool(iter) => iter.next().map(|&x| x as i128),
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
            PrimitiveFloatIterator::F64(iter) => iter.next().map(|&x| x),
        }
    }
}

impl<'a> PartialEq for PrimitiveValueList<'a> {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() || self.is_number() != other.is_number() {
            return false;
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
    Bool(std::slice::Iter<'a, bool>),
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
}

impl<'a> Iterator for PrimitiveForceNumericIterator<'a> {
    type Item = i128;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            PrimitiveForceNumericIterator::Bool(iter) => iter.next().map(|&x| x as i128),
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
        }
    }
}

pub enum PrimitiveForceFloatIterator<'a> {
    Bool(std::slice::Iter<'a, bool>),
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
}

impl<'a> Iterator for PrimitiveForceFloatIterator<'a> {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            // Int/Bool to Float logic: Direct cast
            PrimitiveForceFloatIterator::Bool(iter) => iter.next().map(|&x| x as i8 as f64),
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
            PrimitiveForceFloatIterator::F64(iter) => iter.next().map(|&x| x),
        }
    }
}

// -----------------------------------------------------------------------------
// Arrow Row Item
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Archive, RkyvSerialize, RkyvDeserialize)]
pub struct RowItem<'a> {
    #[rkyv(with = AsOwned)]
    pub key: Cow<'a, str>,
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

impl<'de, 'a> serde::Deserialize<'de> for RowItem<'a> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct TupleVisitor<'a>(std::marker::PhantomData<&'a ()>);

        impl<'de, 'a> serde::de::Visitor<'de> for TupleVisitor<'a> {
            type Value = RowItem<'a>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a tuple of (String, PyroValue)")
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let key: Cow<'a, str> = seq
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

#[derive(Debug, Clone, Default, PartialEq, Archive, RkyvSerialize, RkyvDeserialize)]
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
            // Serialize entries directly using the Key (Cow<str>) and Value (PyroValue)
            map.serialize_entry(&item.key, &item.value)?;
        }
        map.end()
    }
}

// Manual implementation of Deserialize to support both Map (JSON Object) and Seq (Internal Tuple format)
impl<'de, 'a> Deserialize<'de> for PyroRow<'a> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct PyroRowVisitor<'a>(std::marker::PhantomData<&'a ()>);

        impl<'de, 'a> serde::de::Visitor<'de> for PyroRowVisitor<'a> {
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
                        key: Cow::Owned(key),
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
        if let Some(pos) = self.0.iter().position(|item| item.key == key.as_str()) {
            self.0[pos] = RowItem {
                key: Cow::Owned(key),
                value,
            };
        } else {
            self.0.push(RowItem {
                key: Cow::Owned(key),
                value,
            });
        }
    }

    pub fn insert_ref(&mut self, key: &'a str, value: PyroValue<'a>) {
        if let Some(pos) = self.0.iter().position(|item| item.key == key) {
            self.0[pos] = RowItem {
                key: Cow::Borrowed(key),
                value,
            };
        } else {
            self.0.push(RowItem {
                key: Cow::Borrowed(key),
                value,
            });
        }
    }

    pub fn extend(&mut self, other: Self) {
        for item in other.0 {
            if self.get(&item.key).is_some() {
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
                    key: Cow::Owned(item.key.into_owned()),
                    value: item.value.into_owned(),
                })
                .collect(),
        )
    }

    /// Creates a fully owned, static version of this row.
    pub fn to_static(&self) -> PyroRow<'static> {
        PyroRow(
            self.0
                .iter()
                .map(|item| RowItem {
                    // Clone the key string into an Owned Cow
                    key: Cow::Owned(item.key.to_string()),
                    // Recursively make the value static
                    value: item.value.to_static(),
                })
                .collect(),
        )
    }

    /// Projects the Row onto the given Schema (List of PyroFields).
    /// This handles recursive projection for nested Groups.
    pub fn project(&self, fields: &[PyroField]) -> ValueResult<PyroRow<'a>> {
        let mut new_vec = Vec::with_capacity(fields.len());

        for f in fields {
            let target_name = f.name();

            // Find the item in the current row
            let entry = self.0.iter().find(|item| item.key == target_name);

            if let Some(item) = entry {
                match (f.data_type(), &item.value) {
                    // Case 1: Schema expects Group, Value is Group -> Recurse
                    (PyroType::Group(inner_fields), PyroValue::Group(inner_row)) => {
                        new_vec.push(RowItem {
                            key: item.key.clone(),
                            value: PyroValue::Group(inner_row.project(&inner_fields)?),
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
                    key: Cow::Owned(target_name.to_string()),
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
            .map(|item| (item.key.into_owned(), item.value))
            .collect();
        vec.into_iter()
    }
}

// -----------------------------------------------------------------------------
// Arrow Value
// -----------------------------------------------------------------------------

#[derive(
    Debug,
    Clone,
    Default,
    serde::Serialize,
    serde::Deserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[serde(untagged)]
#[rkyv(serialize_bounds(
    __S: rkyv::ser::Writer + rkyv::ser::Allocator,
    __S::Error: rkyv::rancor::Source,
))]
#[rkyv(deserialize_bounds(__D::Error: rkyv::rancor::Source))]
#[rkyv(bytecheck(
    bounds(
        __C: rkyv::validation::ArchiveContext,
    )
))]
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
    PrimitiveList(PrimitiveValueList<'a>),
    Str(#[rkyv(with = AsOwned)] Cow<'a, str>),
    // FIX: Omit bounds on recursive fields
    Group(#[rkyv(omit_bounds)] PyroRow<'a>),
    // FIX: Omit bounds on recursive fields
    List(#[rkyv(omit_bounds)] Vec<PyroValue<'a>>),
    // FIX: Omit bounds on recursive fields (also contains PyroValue)
    MapInternal(#[rkyv(omit_bounds)] Vec<(PyroValue<'a>, PyroValue<'a>)>),
}

impl<'a> PyroValue<'a> {
    pub fn is_null(&self) -> bool {
        matches!(self, PyroValue::Null)
    }

    /// Creates a fully owned, static version of this value.
    ///
    /// This is useful when you have a `&PyroValue<'a>` and need to store it
    /// long-term as a `PyroValue<'static>`.
    pub fn to_static(&self) -> PyroValue<'static> {
        match self {
            PyroValue::Null => PyroValue::Null,
            PyroValue::Bool(v) => PyroValue::Bool(*v),
            PyroValue::I8(v) => PyroValue::I8(*v),
            PyroValue::I16(v) => PyroValue::I16(*v),
            PyroValue::I32(v) => PyroValue::I32(*v),
            PyroValue::I64(v) => PyroValue::I64(*v),
            PyroValue::U8(v) => PyroValue::U8(*v),
            PyroValue::U16(v) => PyroValue::U16(*v),
            PyroValue::U32(v) => PyroValue::U32(*v),
            PyroValue::U64(v) => PyroValue::U64(*v),
            PyroValue::F16(v) => PyroValue::F16(*v),
            PyroValue::F32(v) => PyroValue::F32(*v),
            PyroValue::F64(v) => PyroValue::F64(*v),
            PyroValue::Timestamp(nanos) => PyroValue::Timestamp(*nanos),
            // Strings: Convert &str (or borrowed Cow) to Owned Cow
            PyroValue::Str(v) => PyroValue::Str(Cow::Owned(v.to_string())),

            // Complex types: Recursion
            PyroValue::PrimitiveList(l) => PyroValue::PrimitiveList(l.to_static()),
            PyroValue::Group(row) => PyroValue::Group(row.to_static()),
            PyroValue::List(l) => PyroValue::List(l.iter().map(|v| v.to_static()).collect()),
            PyroValue::MapInternal(entries) => PyroValue::MapInternal(
                entries
                    .iter()
                    .map(|(k, v)| (k.to_static(), v.to_static()))
                    .collect(),
            ),
        }
    }

    pub fn into_owned(self) -> PyroValue<'static> {
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
            PyroValue::Str(s) => PyroValue::Str(Cow::Owned(s.into_owned())),
            PyroValue::Group(g) => PyroValue::Group(g.into_owned()),
            PyroValue::List(l) => PyroValue::List(l.into_iter().map(|v| v.into_owned()).collect()),
            PyroValue::MapInternal(l) => PyroValue::MapInternal(
                l.into_iter()
                    .map(|(k, v)| (k.into_owned(), v.into_owned()))
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
            (PyroValue::Timestamp(n1), PyroValue::Timestamp(n2)) => n1 == n2,
            // Recursive types rely on the Inner types' PartialEq.
            // Note: Since PyroRow/Vec contain PyroValues, they will recursively
            // call back into this `eq` implementation.
            (PyroValue::PrimitiveList(a), PyroValue::PrimitiveList(b)) => a == b,
            (PyroValue::Group(a), PyroValue::Group(b)) => a == b,
            (PyroValue::List(a), PyroValue::List(b)) => a == b,
            (PyroValue::MapInternal(a), PyroValue::MapInternal(b)) => a == b,

            _ => false,
        }
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use rkyv::rancor::Error;

    use super::*;

    #[test]
    fn test_macros_and_cow() {
        // Test primitive scalar from owned
        let v: PyroValue = 10i32.into();
        assert_eq!(v, PyroValue::I32(10));

        // Test primitive list from slice (Borrowed)
        let data = [1, 2, 3];
        let v: PyroValue = (&data[..]).into();
        if let PyroValue::PrimitiveList(PrimitiveValueList::I32(cow)) = &v {
            assert!(matches!(cow, Cow::Borrowed(_)));
        } else {
            panic!("Wrong type");
        }

        // Test primitive list from Vec (Owned)
        let data_vec = vec![1, 2, 3];
        let v: PyroValue = data_vec.into();
        if let PyroValue::PrimitiveList(PrimitiveValueList::I32(cow)) = &v {
            assert!(matches!(cow, Cow::Owned(_)));
        } else {
            panic!("Wrong type");
        }
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

        // Note: JSON deserialization will produce Owned Cows.
        // The original row has Borrowed Cows.
        // However, partialEq checks value equality, so this should pass.
        assert_eq!(row_reconstructed, row);
    }

    #[test]
    fn rkyv_round_trip() {
        let row = PyroRow::from([
            ("id", PyroValue::from(101i32)),
            ("data", PyroValue::from(&[10.0f64, 20.0, 30.5][..])),
            ("name", PyroValue::from("rkyv test")),
        ]);

        // Serialize
        let bytes = rkyv::to_bytes::<Error>(&row).expect("failed to serialize");

        // Deserialize (Zero Copy Access)
        let archived = rkyv::access::<ArchivedPyroRow, Error>(&bytes[..]).unwrap();

        // Verify contents
        // Note: Accessing archived fields depends on rkyv generated code structure.
        // PyroRow is a struct wrapping a Vec.
        let vec = &archived.0;
        assert_eq!(vec.len(), 3);
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
