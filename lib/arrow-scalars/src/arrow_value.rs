use std::borrow::Cow;
use std::fmt;
use std::iter::FromIterator;

use crate::{ArrowScalarError, Result};
use arrow_schema::{DataType, Field};
use half::f16;
use rkyv::with::AsOwned;
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::Deserialize; // Standard Serde traits

// -----------------------------------------------------------------------------
// Type Aliases
// -----------------------------------------------------------------------------

pub type ArrowRowOwned = ArrowRow<'static>;
pub type ArrowValueOwned = ArrowValue<'static>;

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
pub(crate) struct ArrowItem<'a> {
    #[rkyv(with = AsOwned)]
    pub key: Cow<'a, str>,
    pub value: ArrowValue<'a>,
}

// Implement Serde manually to act like a Tuple (Key, Value)
impl<'a> serde::Serialize for ArrowItem<'a> {
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

impl<'de, 'a> serde::Deserialize<'de> for ArrowItem<'a> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct TupleVisitor<'a>(std::marker::PhantomData<&'a ()>);

        impl<'de, 'a> serde::de::Visitor<'de> for TupleVisitor<'a> {
            type Value = ArrowItem<'a>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a tuple of (String, ArrowValue)")
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let key: Cow<'a, str> = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                let value: ArrowValue<'a> = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
                Ok(ArrowItem { key, value })
            }
        }

        deserializer.deserialize_tuple(2, TupleVisitor(std::marker::PhantomData))
    }
}

// -----------------------------------------------------------------------------
// Arrow Row
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Archive, RkyvSerialize, RkyvDeserialize)]
pub struct ArrowRow<'a>(pub(crate) Vec<ArrowItem<'a>>);

impl<'a> serde::Serialize for ArrowRow<'a> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        // Start a map serialization
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for item in &self.0 {
            // Serialize entries directly using the Key (Cow<str>) and Value (ArrowValue)
            map.serialize_entry(&item.key, &item.value)?;
        }
        map.end()
    }
}

// Manual implementation of Deserialize to support both Map (JSON Object) and Seq (Internal Tuple format)
impl<'de, 'a> Deserialize<'de> for ArrowRow<'a> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ArrowRowVisitor<'a>(std::marker::PhantomData<&'a ()>);

        impl<'de, 'a> serde::de::Visitor<'de> for ArrowRowVisitor<'a> {
            type Value = ArrowRow<'a>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map or sequence of (key, value) tuples")
            }

            // Handle JSON Arrays (Internal Format: [[k,v], [k,v]])
            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut items = Vec::new();
                while let Some(item) = seq.next_element::<ArrowItem<'a>>()? {
                    items.push(item);
                }
                Ok(ArrowRow(items))
            }

            // Handle JSON Objects (Standard Format: {"k": v})
            fn visit_map<M>(self, mut map: M) -> std::result::Result<Self::Value, M::Error>
            where
                M: serde::de::MapAccess<'de>,
            {
                let mut items = Vec::with_capacity(map.size_hint().unwrap_or(0));
                while let Some((key, value)) = map.next_entry::<String, ArrowValue<'a>>()? {
                    items.push(ArrowItem {
                        key: Cow::Owned(key),
                        value,
                    });
                }
                Ok(ArrowRow(items))
            }
        }

        deserializer.deserialize_any(ArrowRowVisitor(std::marker::PhantomData))
    }
}

impl<'a> ArrowRow<'a> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn empty() -> Self {
        Self(Vec::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    pub fn get(&self, key: &str) -> Option<&ArrowValue<'a>> {
        self.0
            .iter()
            .find(|item| item.key == key)
            .map(|item| &item.value)
    }

    pub fn insert(&mut self, key: String, value: ArrowValue<'a>) {
        if let Some(pos) = self.0.iter().position(|item| item.key == key.as_str()) {
            self.0[pos] = ArrowItem {
                key: Cow::Owned(key),
                value,
            };
        } else {
            self.0.push(ArrowItem {
                key: Cow::Owned(key),
                value,
            });
        }
    }

    pub fn insert_ref(&mut self, key: &'a str, value: ArrowValue<'a>) {
        if let Some(pos) = self.0.iter().position(|item| item.key == key) {
            self.0[pos] = ArrowItem {
                key: Cow::Borrowed(key),
                value,
            };
        } else {
            self.0.push(ArrowItem {
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

    pub fn into_owned(self) -> ArrowRow<'static> {
        ArrowRow(
            self.0
                .into_iter()
                .map(|item| ArrowItem {
                    key: Cow::Owned(item.key.into_owned()),
                    value: item.value.into_owned(),
                })
                .collect(),
        )
    }

    pub fn to_ref(&self) -> ArrowRow<'_> {
        ArrowRow(
            self.0
                .iter()
                .map(|item| ArrowItem {
                    key: Cow::Borrowed(item.key.as_ref()),
                    value: item.value.to_ref(),
                })
                .collect(),
        )
    }

    pub fn project<F: AsRef<Field>>(&self, fields: &[F]) -> Result<ArrowRow<'a>> {
        let mut new_vec = Vec::with_capacity(fields.len());

        for f in fields {
            let target_name = f.as_ref().name().as_str();
            let entry = self.0.iter().find(|item| item.key == target_name);

            if let Some(item) = entry {
                let key_cow = item.key.clone();

                match (f.as_ref().data_type(), &item.value) {
                    (DataType::Struct(inner_fields), ArrowValue::Group(inner_row)) => {
                        new_vec.push(ArrowItem {
                            key: key_cow,
                            value: ArrowValue::Group(inner_row.project(inner_fields)?),
                        });
                    }
                    (DataType::Struct(_), _) => {
                        return Err(ArrowScalarError::InvalidScalar(ArrowValue::Null));
                    }
                    (_, ArrowValue::Group(_)) => {
                        return Err(ArrowScalarError::InvalidScalar(ArrowValue::Null));
                    }
                    _ => {
                        new_vec.push(ArrowItem {
                            key: key_cow,
                            value: item.value.clone(),
                        });
                    }
                }
            } else {
                new_vec.push(ArrowItem {
                    key: Cow::Owned(target_name.to_string()),
                    value: ArrowValue::Null,
                });
            }
        }
        Ok(ArrowRow(new_vec))
    }

    pub fn get_deep<S: AsRef<str>>(&self, path: &[S]) -> Option<&ArrowValue<'a>> {
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
                        ArrowValue::Group(next_row) => {
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

impl<'a> ArrowRow<'a> {
    pub fn iter(&self) -> impl Iterator<Item = (&str, &ArrowValue<'a>)> {
        self.0.iter().map(|item| (item.key.as_ref(), &item.value))
    }

    pub fn values(&self) -> impl Iterator<Item = &ArrowValue<'a>> {
        self.0.iter().map(|item| &item.value)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'a> IntoIterator for ArrowRow<'a> {
    type Item = (String, ArrowValue<'a>);
    type IntoIter = std::vec::IntoIter<(String, ArrowValue<'a>)>;

    fn into_iter(self) -> Self::IntoIter {
        // Must convert ArrowItems to tuples to maintain expected API
        let vec: Vec<(String, ArrowValue<'a>)> = self
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
pub enum ArrowValue<'a> {
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
    IntervalDayTime {
        days: i32,
        milliseconds: i32,
    },
    PrimitiveList(PrimitiveValueList<'a>),
    Str(#[rkyv(with = AsOwned)] Cow<'a, str>),
    // FIX: Omit bounds on recursive fields
    Group(#[rkyv(omit_bounds)] ArrowRow<'a>),
    // FIX: Omit bounds on recursive fields
    List(#[rkyv(omit_bounds)] Vec<ArrowValue<'a>>),
    // FIX: Omit bounds on recursive fields (also contains ArrowValue)
    MapInternal(#[rkyv(omit_bounds)] Vec<(ArrowValue<'a>, ArrowValue<'a>)>),
}

impl<'a> ArrowValue<'a> {
    pub fn is_null(&self) -> bool {
        matches!(self, ArrowValue::Null)
    }

    pub fn into_owned(self) -> ArrowValue<'static> {
        match self {
            ArrowValue::Null => ArrowValue::Null,
            ArrowValue::Bool(v) => ArrowValue::Bool(v),
            ArrowValue::I8(v) => ArrowValue::I8(v),
            ArrowValue::I16(v) => ArrowValue::I16(v),
            ArrowValue::I32(v) => ArrowValue::I32(v),
            ArrowValue::I64(v) => ArrowValue::I64(v),
            ArrowValue::U8(v) => ArrowValue::U8(v),
            ArrowValue::U16(v) => ArrowValue::U16(v),
            ArrowValue::U32(v) => ArrowValue::U32(v),
            ArrowValue::U64(v) => ArrowValue::U64(v),
            ArrowValue::F16(v) => ArrowValue::F16(v),
            ArrowValue::F32(v) => ArrowValue::F32(v),
            ArrowValue::F64(v) => ArrowValue::F64(v),
            ArrowValue::IntervalDayTime { days, milliseconds } => {
                ArrowValue::IntervalDayTime { days, milliseconds }
            }
            ArrowValue::PrimitiveList(l) => ArrowValue::PrimitiveList(l.into_owned()),
            ArrowValue::Str(s) => ArrowValue::Str(Cow::Owned(s.into_owned())),
            ArrowValue::Group(g) => ArrowValue::Group(g.into_owned()),
            ArrowValue::List(l) => {
                ArrowValue::List(l.into_iter().map(|v| v.into_owned()).collect())
            }
            ArrowValue::MapInternal(l) => ArrowValue::MapInternal(
                l.into_iter()
                    .map(|(k, v)| (k.into_owned(), v.into_owned()))
                    .collect(),
            ),
        }
    }

    pub fn to_ref(&self) -> ArrowValue<'_> {
        match self {
            ArrowValue::Null => ArrowValue::Null,
            ArrowValue::Bool(v) => ArrowValue::Bool(*v),
            ArrowValue::I8(v) => ArrowValue::I8(*v),
            ArrowValue::I16(v) => ArrowValue::I16(*v),
            ArrowValue::I32(v) => ArrowValue::I32(*v),
            ArrowValue::I64(v) => ArrowValue::I64(*v),
            ArrowValue::U8(v) => ArrowValue::U8(*v),
            ArrowValue::U16(v) => ArrowValue::U16(*v),
            ArrowValue::U32(v) => ArrowValue::U32(*v),
            ArrowValue::U64(v) => ArrowValue::U64(*v),
            ArrowValue::F16(v) => ArrowValue::F16(*v),
            ArrowValue::F32(v) => ArrowValue::F32(*v),
            ArrowValue::F64(v) => ArrowValue::F64(*v),
            ArrowValue::IntervalDayTime { days, milliseconds } => ArrowValue::IntervalDayTime {
                days: *days,
                milliseconds: *milliseconds,
            },
            ArrowValue::PrimitiveList(l) => ArrowValue::PrimitiveList(l.to_ref()),
            ArrowValue::Str(s) => ArrowValue::Str(Cow::Borrowed(s.as_ref())),
            ArrowValue::Group(g) => ArrowValue::Group(g.to_ref()),
            ArrowValue::List(l) => ArrowValue::List(l.iter().map(|v| v.to_ref()).collect()),
            ArrowValue::MapInternal(l) => {
                ArrowValue::MapInternal(l.iter().map(|(k, v)| (k.to_ref(), v.to_ref())).collect())
            }
        }
    }
}

impl<'a> PartialEq for ArrowValue<'a> {
    fn eq(&self, other: &Self) -> bool {
        // Helper to normalize integers to i128
        // Note: u64::MAX fits within i128, so this covers all ArrowValue int types safely.
        let as_int = |v: &ArrowValue| -> Option<i128> {
            match v {
                ArrowValue::I8(n) => Some(*n as i128),
                ArrowValue::I16(n) => Some(*n as i128),
                ArrowValue::I32(n) => Some(*n as i128),
                ArrowValue::I64(n) => Some(*n as i128),
                ArrowValue::U8(n) => Some(*n as i128),
                ArrowValue::U16(n) => Some(*n as i128),
                ArrowValue::U32(n) => Some(*n as i128),
                ArrowValue::U64(n) => Some(*n as i128),
                _ => None,
            }
        };

        // Helper to normalize floats to f64
        let as_float = |v: &ArrowValue| -> Option<f64> {
            match v {
                ArrowValue::F16(n) => Some(n.to_f64()),
                ArrowValue::F32(n) => Some(*n as f64),
                ArrowValue::F64(n) => Some(*n),
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
            (ArrowValue::Null, ArrowValue::Null) => true,
            (ArrowValue::Bool(a), ArrowValue::Bool(b)) => a == b,
            (ArrowValue::Str(a), ArrowValue::Str(b)) => a == b,
            (
                ArrowValue::IntervalDayTime {
                    days: d1,
                    milliseconds: m1,
                },
                ArrowValue::IntervalDayTime {
                    days: d2,
                    milliseconds: m2,
                },
            ) => d1 == d2 && m1 == m2,

            // Recursive types rely on the Inner types' PartialEq.
            // Note: Since ArrowRow/Vec contain ArrowValues, they will recursively
            // call back into this `eq` implementation.
            (ArrowValue::PrimitiveList(a), ArrowValue::PrimitiveList(b)) => a == b,
            (ArrowValue::Group(a), ArrowValue::Group(b)) => a == b,
            (ArrowValue::List(a), ArrowValue::List(b)) => a == b,
            (ArrowValue::MapInternal(a), ArrowValue::MapInternal(b)) => a == b,

            _ => false,
        }
    }
}

// -----------------------------------------------------------------------------
// Macros for Conversions
// -----------------------------------------------------------------------------

macro_rules! val_from_primitive {
    ($primitive_type:ty, $values_type:ident) => {
        // Owned value
        impl<'a> From<$primitive_type> for ArrowValue<'a> {
            fn from(v: $primitive_type) -> Self {
                ArrowValue::$values_type(v)
            }
        }

        // Option<Value>
        impl<'a> From<Option<$primitive_type>> for ArrowValue<'a> {
            fn from(v: Option<$primitive_type>) -> Self {
                if let Some(v) = v {
                    ArrowValue::$values_type(v)
                } else {
                    ArrowValue::Null
                }
            }
        }

        impl<'a> From<Option<&'a $primitive_type>> for ArrowValue<'a> {
            fn from(v: Option<&'a $primitive_type>) -> Self {
                if let Some(v) = v {
                    ArrowValue::$values_type(*v)
                } else {
                    ArrowValue::Null
                }
            }
        }

        impl<'a> From<&'a Option<$primitive_type>> for ArrowValue<'a> {
            fn from(v: &'a Option<$primitive_type>) -> Self {
                if let Some(v) = v {
                    ArrowValue::$values_type(*v)
                } else {
                    ArrowValue::Null
                }
            }
        }

        // Reference to value (implicitly copied)
        impl<'a> From<&'a $primitive_type> for ArrowValue<'a> {
            fn from(v: &'a $primitive_type) -> Self {
                ArrowValue::$values_type(*v)
            }
        }
    };
}

macro_rules! val_from_primitive_list {
    ($primitive_type:ty, $values_type:ident) => {
        // 1. Borrowed slice -> Borrowed Cow
        impl<'a> From<&'a [$primitive_type]> for ArrowValue<'a> {
            fn from(v: &'a [$primitive_type]) -> Self {
                ArrowValue::PrimitiveList(PrimitiveValueList::$values_type(Cow::Borrowed(v)))
            }
        }

        impl<'a> From<&'a Vec<$primitive_type>> for ArrowValue<'a> {
            fn from(v: &'a Vec<$primitive_type>) -> Self {
                ArrowValue::PrimitiveList(PrimitiveValueList::$values_type(Cow::Borrowed(v)))
            }
        }

        impl<'a> From<&&'a [$primitive_type]> for ArrowValue<'a> {
            fn from(v: &&'a [$primitive_type]) -> Self {
                ArrowValue::PrimitiveList(PrimitiveValueList::$values_type(Cow::Borrowed(v)))
            }
        }

        // 2. Owned Vec -> Owned Cow
        impl From<Vec<$primitive_type>> for ArrowValue<'static> {
            fn from(v: Vec<$primitive_type>) -> Self {
                ArrowValue::PrimitiveList(PrimitiveValueList::$values_type(Cow::Owned(v)))
            }
        }

        // 3. Fixed size array ref -> Borrowed Cow
        impl<'a, const N: usize> From<&'a [$primitive_type; N]> for ArrowValue<'a> {
            fn from(v: &'a [$primitive_type; N]) -> Self {
                ArrowValue::PrimitiveList(PrimitiveValueList::$values_type(Cow::Borrowed(
                    v.as_slice(),
                )))
            }
        }

        // 4. Fixed size array owned -> Owned Cow
        impl<const N: usize> From<[$primitive_type; N]> for ArrowValue<'static> {
            fn from(v: [$primitive_type; N]) -> Self {
                ArrowValue::PrimitiveList(PrimitiveValueList::$values_type(Cow::Owned(v.to_vec())))
            }
        }

        // 5. Option Vec -> List of values (Not PrimitiveList optimization)
        impl<'a> From<Vec<Option<$primitive_type>>> for ArrowValue<'a> {
            fn from(v: Vec<Option<$primitive_type>>) -> Self {
                ArrowValue::List(v.into_iter().map(|i| i.into()).collect())
            }
        }
    };
}

val_from_primitive!(bool, Bool);
val_from_primitive!(i8, I8);
val_from_primitive!(i16, I16);
val_from_primitive!(i32, I32);
val_from_primitive!(i64, I64);
val_from_primitive!(u8, U8);
val_from_primitive!(u16, U16);
val_from_primitive!(u32, U32);
val_from_primitive!(u64, U64);
val_from_primitive!(f16, F16);
val_from_primitive!(f32, F32);
val_from_primitive!(f64, F64);

val_from_primitive_list!(bool, Bool);
val_from_primitive_list!(i8, I8);
val_from_primitive_list!(i16, I16);
val_from_primitive_list!(i32, I32);
val_from_primitive_list!(i64, I64);
val_from_primitive_list!(u8, U8);
val_from_primitive_list!(u16, U16);
val_from_primitive_list!(u32, U32);
val_from_primitive_list!(u64, U64);
val_from_primitive_list!(f16, F16);
val_from_primitive_list!(f32, F32);
val_from_primitive_list!(f64, F64);

// -----------------------------------------------------------------------------
// String Conversions
// -----------------------------------------------------------------------------

impl From<String> for ArrowValue<'static> {
    fn from(v: String) -> Self {
        ArrowValue::Str(Cow::Owned(v))
    }
}

impl From<Option<String>> for ArrowValue<'static> {
    fn from(v: Option<String>) -> Self {
        match v {
            Some(v) => ArrowValue::Str(Cow::Owned(v)),
            None => ArrowValue::Null,
        }
    }
}

impl<'a> From<&'a String> for ArrowValue<'a> {
    fn from(v: &'a String) -> Self {
        ArrowValue::Str(Cow::Borrowed(v))
    }
}

impl<'a> From<&'a Option<String>> for ArrowValue<'a> {
    fn from(v: &'a Option<String>) -> Self {
        match v {
            Some(v) => ArrowValue::Str(Cow::Borrowed(v)),
            None => ArrowValue::Null,
        }
    }
}

impl<'a> From<Option<&'a String>> for ArrowValue<'a> {
    fn from(v: Option<&'a String>) -> Self {
        match v {
            Some(v) => ArrowValue::Str(Cow::Borrowed(v)),
            None => ArrowValue::Null,
        }
    }
}

impl<'a> From<&'a str> for ArrowValue<'a> {
    fn from(v: &'a str) -> Self {
        ArrowValue::Str(Cow::Borrowed(v))
    }
}

impl<'a> From<&'a Option<&'a str>> for ArrowValue<'a> {
    fn from(v: &'a Option<&'a str>) -> Self {
        match v {
            Some(v) => ArrowValue::Str(Cow::Borrowed(v)),
            None => ArrowValue::Null,
        }
    }
}

impl<'a> From<Option<&'a str>> for ArrowValue<'a> {
    fn from(v: Option<&'a str>) -> Self {
        match v {
            Some(v) => ArrowValue::Str(Cow::Borrowed(v)),
            None => ArrowValue::Null,
        }
    }
}

impl<'a> From<&&'a str> for ArrowValue<'a> {
    fn from(v: &&'a str) -> Self {
        ArrowValue::Str(Cow::Borrowed(v))
    }
}

// -----------------------------------------------------------------------------
// Row Conversions
// -----------------------------------------------------------------------------

impl<'a> From<ArrowRow<'a>> for ArrowValue<'a> {
    fn from(v: ArrowRow<'a>) -> Self {
        ArrowValue::Group(v)
    }
}

impl<'a> FromIterator<(String, ArrowValue<'a>)> for ArrowRow<'a> {
    fn from_iter<T: IntoIterator<Item = (String, ArrowValue<'a>)>>(iter: T) -> Self {
        let mut row = ArrowRow::new();
        for (k, v) in iter {
            row.insert(k, v);
        }
        row
    }
}

impl<'a> FromIterator<(&'a str, ArrowValue<'a>)> for ArrowRow<'a> {
    fn from_iter<T: IntoIterator<Item = (&'a str, ArrowValue<'a>)>>(iter: T) -> Self {
        let mut row = ArrowRow::new();
        for (k, v) in iter {
            row.insert_ref(k, v);
        }
        row
    }
}

impl<'a, const N: usize> From<[(&'a str, ArrowValue<'a>); N]> for ArrowRow<'a> {
    fn from(values: [(&'a str, ArrowValue<'a>); N]) -> Self {
        let mut row = ArrowRow::with_capacity(N);
        for (k, v) in values {
            row.insert_ref(k, v);
        }
        row
    }
}

impl<'a, const N: usize> From<[(&'a str, ArrowValue<'a>); N]> for ArrowValue<'a> {
    fn from(values: [(&'a str, ArrowValue<'a>); N]) -> Self {
        let mut row = ArrowRow::with_capacity(N);
        for (k, v) in values {
            row.insert_ref(k, v);
        }
        ArrowValue::Group(row)
    }
}

impl<const N: usize> From<[(String, ArrowValueOwned); N]> for ArrowRowOwned {
    fn from(values: [(String, ArrowValueOwned); N]) -> Self {
        let mut row = ArrowRow::with_capacity(N);
        for (k, v) in values {
            row.insert(k, v);
        }
        row
    }
}

impl<'a, const N: usize> From<[ArrowValue<'a>; N]> for ArrowValue<'a> {
    fn from(values: [ArrowValue<'a>; N]) -> Self {
        ArrowValue::List(values.to_vec())
    }
}

impl<'a> From<Vec<ArrowValue<'a>>> for ArrowValue<'a> {
    fn from(values: Vec<ArrowValue<'a>>) -> Self {
        ArrowValue::List(values)
    }
}

#[cfg(target_endian = "little")]
impl<'a, 'b> From<&'b ArchivedArrowValue<'a>> for ArrowValue<'a> {
    fn from(archived: &'b ArchivedArrowValue<'a>) -> Self {
        match archived {
            ArchivedArrowValue::Null => ArrowValue::Null,

            // Primitives - direct copy
            ArchivedArrowValue::Bool(b) => ArrowValue::Bool(*b),
            ArchivedArrowValue::I8(v) => ArrowValue::I8(*v),
            ArchivedArrowValue::I16(v) => ArrowValue::I16(v.to_native()),
            ArchivedArrowValue::I32(v) => ArrowValue::I32(v.to_native()),
            ArchivedArrowValue::I64(v) => ArrowValue::I64(v.to_native()),
            ArchivedArrowValue::U8(v) => ArrowValue::U8(*v),
            ArchivedArrowValue::U16(v) => ArrowValue::U16(v.to_native()),
            ArchivedArrowValue::U32(v) => ArrowValue::U32(v.to_native()),
            ArchivedArrowValue::U64(v) => ArrowValue::U64(v.to_native()),
            ArchivedArrowValue::F16(v) => {
                // SAFETY: On little-endian systems, &ArchivedF16 has the same memory layout as &f16
                ArrowValue::F16(unsafe { *(v as *const _ as *const f16) })
            }
            ArchivedArrowValue::F32(v) => ArrowValue::F32(v.to_native()),
            ArchivedArrowValue::F64(v) => ArrowValue::F64(v.to_native()),

            ArchivedArrowValue::IntervalDayTime { days, milliseconds } => {
                ArrowValue::IntervalDayTime {
                    days: days.to_native(),
                    milliseconds: milliseconds.to_native(),
                }
            }

            // String - zero-copy borrow from archived data
            // SAFETY: The archived data has lifetime 'a (from ArchivedArrowValue<'a>),
            // but we're borrowing through 'b. We transmute to extend the lifetime back to 'a
            // because we know the string data lives in the archived buffer with lifetime 'a.
            ArchivedArrowValue::Str(s) => {
                let s_str = s.as_str();
                let extended: &'a str = unsafe { std::mem::transmute(s_str) };
                ArrowValue::Str(Cow::Borrowed(extended))
            }

            // PrimitiveList - zero-copy borrow with unsafe transmutation
            // SAFETY: On little-endian systems, rkyv's archived primitive types have the same
            // memory layout as native Rust primitives. We verify this is only compiled on
            // little-endian architectures with the cfg gate above.
            // Additionally, we transmute the slice lifetime from 'b to 'a because the underlying
            // data lives in the archived buffer with lifetime 'a.
            ArchivedArrowValue::PrimitiveList(pl) => {
                let borrowed_list = match pl {
                    ArchivedPrimitiveValueList::Bool(v) => {
                        let slice = v.as_slice();
                        let extended: &'a [u8] = unsafe { std::mem::transmute(slice) };
                        PrimitiveValueList::U8(Cow::Borrowed(extended))
                    }
                    // U8 and I8 are always safe - no endianness concerns
                    ArchivedPrimitiveValueList::U8(v) => {
                        let slice = v.as_slice();
                        let extended: &'a [u8] = unsafe { std::mem::transmute(slice) };
                        PrimitiveValueList::U8(Cow::Borrowed(extended))
                    }
                    ArchivedPrimitiveValueList::I8(v) => {
                        let slice = v.as_slice();
                        let extended: &'a [i8] = unsafe { std::mem::transmute(slice) };
                        PrimitiveValueList::I8(Cow::Borrowed(extended))
                    }

                    // Multi-byte types require unsafe transmutation for both type and lifetime
                    ArchivedPrimitiveValueList::U16(v) => {
                        let slice = unsafe {
                            let ptr = v.as_ptr() as *const u16;
                            let len = v.len();
                            std::slice::from_raw_parts(ptr, len)
                        };
                        let extended: &'a [u16] = unsafe { std::mem::transmute(slice) };
                        PrimitiveValueList::U16(Cow::Borrowed(extended))
                    }
                    ArchivedPrimitiveValueList::U32(v) => {
                        let slice = unsafe {
                            let ptr = v.as_ptr() as *const u32;
                            let len = v.len();
                            std::slice::from_raw_parts(ptr, len)
                        };
                        let extended: &'a [u32] = unsafe { std::mem::transmute(slice) };
                        PrimitiveValueList::U32(Cow::Borrowed(extended))
                    }
                    ArchivedPrimitiveValueList::U64(v) => {
                        let slice = unsafe {
                            let ptr = v.as_ptr() as *const u64;
                            let len = v.len();
                            std::slice::from_raw_parts(ptr, len)
                        };
                        let extended: &'a [u64] = unsafe { std::mem::transmute(slice) };
                        PrimitiveValueList::U64(Cow::Borrowed(extended))
                    }
                    ArchivedPrimitiveValueList::I16(v) => {
                        let slice = unsafe {
                            let ptr = v.as_ptr() as *const i16;
                            let len = v.len();
                            std::slice::from_raw_parts(ptr, len)
                        };
                        let extended: &'a [i16] = unsafe { std::mem::transmute(slice) };
                        PrimitiveValueList::I16(Cow::Borrowed(extended))
                    }
                    ArchivedPrimitiveValueList::I32(v) => {
                        let slice = unsafe {
                            let ptr = v.as_ptr() as *const i32;
                            let len = v.len();
                            std::slice::from_raw_parts(ptr, len)
                        };
                        let extended: &'a [i32] = unsafe { std::mem::transmute(slice) };
                        PrimitiveValueList::I32(Cow::Borrowed(extended))
                    }
                    ArchivedPrimitiveValueList::I64(v) => {
                        let slice = unsafe {
                            let ptr = v.as_ptr() as *const i64;
                            let len = v.len();
                            std::slice::from_raw_parts(ptr, len)
                        };
                        let extended: &'a [i64] = unsafe { std::mem::transmute(slice) };
                        PrimitiveValueList::I64(Cow::Borrowed(extended))
                    }
                    ArchivedPrimitiveValueList::F16(v) => {
                        let slice = unsafe {
                            let ptr = v.as_ptr() as *const f16;
                            let len = v.len();
                            std::slice::from_raw_parts(ptr, len)
                        };
                        let extended: &'a [f16] = unsafe { std::mem::transmute(slice) };
                        PrimitiveValueList::F16(Cow::Borrowed(extended))
                    }
                    ArchivedPrimitiveValueList::F32(v) => {
                        let slice = unsafe {
                            let ptr = v.as_ptr() as *const f32;
                            let len = v.len();
                            std::slice::from_raw_parts(ptr, len)
                        };
                        let extended: &'a [f32] = unsafe { std::mem::transmute(slice) };
                        PrimitiveValueList::F32(Cow::Borrowed(extended))
                    }
                    ArchivedPrimitiveValueList::F64(v) => {
                        let slice = unsafe {
                            let ptr = v.as_ptr() as *const f64;
                            let len = v.len();
                            std::slice::from_raw_parts(ptr, len)
                        };
                        let extended: &'a [f64] = unsafe { std::mem::transmute(slice) };
                        PrimitiveValueList::F64(Cow::Borrowed(extended))
                    }
                };
                ArrowValue::PrimitiveList(borrowed_list)
            }

            // Group - recursively convert each item
            ArchivedArrowValue::Group(archived_row) => {
                ArrowValue::Group(ArrowRow::from(archived_row))
            }

            // List - recursively convert each element
            ArchivedArrowValue::List(archived_list) => {
                let items: Vec<ArrowValue<'a>> = archived_list
                    .iter()
                    .map(|archived_val| ArrowValue::from(archived_val))
                    .collect();
                ArrowValue::List(items)
            }

            // MapInternal - recursively convert key-value pairs
            ArchivedArrowValue::MapInternal(archived_map) => {
                let items: Vec<(ArrowValue<'a>, ArrowValue<'a>)> = archived_map
                    .iter()
                    .map(|t| (ArrowValue::from(&t.0), ArrowValue::from(&t.1)))
                    .collect();
                ArrowValue::MapInternal(items)
            }
        }
    }
}

#[cfg(target_endian = "little")]
impl<'a> From<ArchivedArrowValue<'a>> for ArrowValue<'a> {
    fn from(archived: ArchivedArrowValue<'a>) -> Self {
        ArrowValue::from(&archived)
    }
}

#[cfg(target_endian = "little")]
impl<'a, 'b> From<&'b ArchivedArrowRow<'a>> for ArrowRow<'a> {
    fn from(value: &'b ArchivedArrowRow<'a>) -> Self {
        let items: Vec<ArrowItem<'a>> = value
            .0
            .iter()
            .map(|archived_item| {
                // String - zero-copy borrow from archived data
                // SAFETY: The archived data has lifetime 'a (from ArchivedArrowValue<'a>),
                // but we're borrowing through 'b. We transmute to extend the lifetime back to 'a
                // because we know the string data lives in the archived buffer with lifetime 'a.
                let key_str = archived_item.key.as_str();
                let extended_key: &'a str = unsafe { std::mem::transmute(key_str) };
                ArrowItem {
                    key: Cow::Borrowed(extended_key),
                    value: ArrowValue::from(&archived_item.value),
                }
            })
            .collect();
        ArrowRow(items)
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
        let v: ArrowValue = 10i32.into();
        assert_eq!(v, ArrowValue::I32(10));

        // Test primitive list from slice (Borrowed)
        let data = [1, 2, 3];
        let v: ArrowValue = (&data[..]).into();
        if let ArrowValue::PrimitiveList(PrimitiveValueList::I32(cow)) = &v {
            assert!(matches!(cow, Cow::Borrowed(_)));
        } else {
            panic!("Wrong type");
        }

        // Test primitive list from Vec (Owned)
        let data_vec = vec![1, 2, 3];
        let v: ArrowValue = data_vec.into();
        if let ArrowValue::PrimitiveList(PrimitiveValueList::I32(cow)) = &v {
            assert!(matches!(cow, Cow::Owned(_)));
        } else {
            panic!("Wrong type");
        }
    }

    #[test]
    fn json_round_trip() {
        // Construct using the helper macros / From traits
        let row = ArrowRow::from([
            ("name", ArrowValue::from("John Doe")),
            ("age", ArrowValue::from(70000)),
            ("child_ages", ArrowValue::from(&[0u8, 1, 4][..])), // Primitive list
            (
                "address",
                ArrowValue::from(ArrowRow::from([
                    ("street", ArrowValue::from("10 Downing Street")),
                    ("city", ArrowValue::from("London")),
                ])),
            ),
            (
                "phones",
                ArrowValue::List(vec![
                    ArrowValue::from("+44 1234567"),
                    ArrowValue::from("+44 2345678"),
                ]),
            ),
        ]);

        let row_string = serde_json::to_string(&row).unwrap();
        println!("Row_string: {}", row_string);
        let row_reconstructed: ArrowRow = serde_json::from_str(&row_string).unwrap();

        // Note: JSON deserialization will produce Owned Cows.
        // The original row has Borrowed Cows.
        // However, partialEq checks value equality, so this should pass.
        assert_eq!(row_reconstructed, row);
    }

    #[test]
    fn rkyv_round_trip() {
        let row = ArrowRow::from([
            ("id", ArrowValue::from(101i32)),
            ("data", ArrowValue::from(&[10.0f64, 20.0, 30.5][..])),
            ("name", ArrowValue::from("rkyv test")),
        ]);

        // Serialize
        let bytes = rkyv::to_bytes::<Error>(&row).expect("failed to serialize");

        // Deserialize (Zero Copy Access)
        let archived = rkyv::access::<ArchivedArrowRow, Error>(&bytes[..]).unwrap();

        // Verify contents
        // Note: Accessing archived fields depends on rkyv generated code structure.
        // ArrowRow is a struct wrapping a Vec.
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
        let row = ArrowRow::from([
            ("foo", ArrowValue::from("bar")),
            ("sin", ArrowValue::from("cos")),
            ("count", ArrowValue::from(123)),
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
        let inner_row = ArrowRow::from([("inner_key", ArrowValue::from("inner_val"))]);

        let outer_row = ArrowRow::from([
            ("outer_key", ArrowValue::from("outer_val")),
            ("group", ArrowValue::from(inner_row)),
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
        let row: ArrowRow = serde_json::from_str(json_input).expect("Failed to deserialize map");

        // Assert
        assert_eq!(row.get("name"), Some(&ArrowValue::from("Alice")));
        assert_eq!(row.get("age"), Some(&ArrowValue::from(30)));
    }

    #[test]
    fn test_deserialize_legacy_array_format() {
        // Arrange: The old "Sequence of Tuples" format: [[key, val], [key, val]]
        // Your custom Deserialize impl has `visit_seq`, so this should still work.
        let json_input = r#"[ ["key1", "value1"], ["key2", 99] ]"#;

        // Act
        let row: ArrowRow =
            serde_json::from_str(json_input).expect("Failed to deserialize legacy sequence");

        // Assert
        assert_eq!(row.get("key1"), Some(&ArrowValue::from("value1")));
        assert_eq!(row.get("key2"), Some(&ArrowValue::from(99)));
    }

    #[test]
    fn test_round_trip() {
        // Arrange
        let original = ArrowRow::from([("a", ArrowValue::from(1)), ("b", ArrowValue::from(2))]);

        // Act
        let serialized = serde_json::to_string(&original).unwrap();
        // Ensure it looks like a map
        assert!(serialized.starts_with('{'));
        assert!(serialized.ends_with('}'));

        let deserialized: ArrowRow = serde_json::from_str(&serialized).unwrap();

        // Assert
        assert_eq!(original, deserialized);
    }
}
