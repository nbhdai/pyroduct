use std::borrow::Cow;
use std::iter::FromIterator;

use crate::ToRow;
use crate::value::Time;

use super::RowItem;
use super::{PrimitiveValueList, PyroRow, PyroRowOwned, PyroValue, PyroValueOwned};
use chrono::{DateTime, Utc};
// Needed for internal logic if used, or just public structs
use half::f16;

#[cfg(target_endian = "little")]
use super::{ArchivedPrimitiveValueList, ArchivedPyroRow, ArchivedPyroValue};

impl<'a> From<&'a PyroValue<'_>> for PyroValue<'a> {
    fn from(value: &'a PyroValue<'_>) -> Self {
        match value {
            PyroValue::Null => PyroValue::Null,
            PyroValue::Bool(b) => PyroValue::Bool(*b),
            PyroValue::I8(n) => PyroValue::I8(*n),
            PyroValue::I16(n) => PyroValue::I16(*n),
            PyroValue::I32(n) => PyroValue::I32(*n),
            PyroValue::I64(n) => PyroValue::I64(*n),
            PyroValue::U8(n) => PyroValue::U8(*n),
            PyroValue::U16(n) => PyroValue::U16(*n),
            PyroValue::U32(n) => PyroValue::U32(*n),
            PyroValue::U64(n) => PyroValue::U64(*n),
            PyroValue::F16(n) => PyroValue::F16(*n),
            PyroValue::F32(n) => PyroValue::F32(*n),
            PyroValue::F64(n) => PyroValue::F64(*n),
            PyroValue::Timestamp(nanos) => PyroValue::Timestamp(*nanos),
            PyroValue::PrimitiveList(l) => PyroValue::PrimitiveList(l.to_ref()),
            PyroValue::Str(cow) => PyroValue::Str(Cow::Borrowed(cow.as_ref())),
            PyroValue::Group(row) => PyroValue::Group(row.clone()),
            PyroValue::List(l) => PyroValue::List(l.iter().map(|v| v.into()).collect()),
            PyroValue::MapInternal(items) => {
                PyroValue::MapInternal(items.iter().map(|(k, v)| (k.into(), v.into())).collect())
            }
        }
    }
}

impl From<DateTime<Utc>> for PyroValue<'_> {
    fn from(dt: DateTime<Utc>) -> Self {
        let secs = dt.timestamp() as i128;
        let nanos = dt.timestamp_subsec_nanos() as i128;
        PyroValue::Timestamp(Time(secs * 1_000_000_000 + nanos))
    }
}

impl From<&DateTime<Utc>> for PyroValue<'_> {
    fn from(dt: &DateTime<Utc>) -> Self {
        let secs = dt.timestamp() as i128;
        let nanos = dt.timestamp_subsec_nanos() as i128;
        PyroValue::Timestamp(Time(secs * 1_000_000_000 + nanos))
    }
}

impl From<Time> for PyroValue<'_> {
    fn from(dt: Time) -> Self {
        PyroValue::Timestamp(dt)
    }
}

impl From<&Time> for PyroValue<'_> {
    fn from(dt: &Time) -> Self {
        PyroValue::Timestamp(*dt)
    }
}

// -----------------------------------------------------------------------------
// Macros
// -----------------------------------------------------------------------------

/// Implements From<&'a T> for PyroValue<'a> by cloning and calling into().
macro_rules! impl_ref_into_pyro {
    ($type:ty) => {
        impl<'a> From<&'a $type> for PyroValue<'a> {
            fn from(v: &'a $type) -> Self {
                v.clone().into()
            }
        }
    };
}

macro_rules! val_from_primitive {
    ($primitive_type:ty, $values_type:ident) => {
        // Owned value
        impl<'a> From<$primitive_type> for PyroValue<'a> {
            fn from(v: $primitive_type) -> Self {
                PyroValue::$values_type(v)
            }
        }

        // Apply reference implementations via the new macro
        impl_ref_into_pyro!($primitive_type);
        impl_ref_into_pyro!(Option<$primitive_type>);
    };
}

macro_rules! val_from_primitive_list {
    ($primitive_type:ty, $values_type:ident) => {
        // 1. Borrowed slice -> Borrowed Cow
        impl<'a> From<&'a [$primitive_type]> for PyroValue<'a> {
            fn from(v: &'a [$primitive_type]) -> Self {
                PyroValue::PrimitiveList(PrimitiveValueList::$values_type(Cow::Borrowed(v)))
            }
        }

        impl<'a> From<&'a Vec<$primitive_type>> for PyroValue<'a> {
            fn from(v: &'a Vec<$primitive_type>) -> Self {
                PyroValue::PrimitiveList(PrimitiveValueList::$values_type(Cow::Borrowed(v)))
            }
        }

        impl<'a> From<&&'a [$primitive_type]> for PyroValue<'a> {
            fn from(v: &&'a [$primitive_type]) -> Self {
                PyroValue::PrimitiveList(PrimitiveValueList::$values_type(Cow::Borrowed(v)))
            }
        }

        // 2. Owned Vec -> Owned Cow
        impl From<Vec<$primitive_type>> for PyroValue<'static> {
            fn from(v: Vec<$primitive_type>) -> Self {
                PyroValue::PrimitiveList(PrimitiveValueList::$values_type(Cow::Owned(v)))
            }
        }

        // 3. Fixed size array ref -> Borrowed Cow
        impl<'a, const N: usize> From<&'a [$primitive_type; N]> for PyroValue<'a> {
            fn from(v: &'a [$primitive_type; N]) -> Self {
                PyroValue::PrimitiveList(PrimitiveValueList::$values_type(Cow::Borrowed(
                    v.as_slice(),
                )))
            }
        }

        // 4. Fixed size array owned -> Owned Cow
        impl<const N: usize> From<[$primitive_type; N]> for PyroValue<'static> {
            fn from(v: [$primitive_type; N]) -> Self {
                PyroValue::PrimitiveList(PrimitiveValueList::$values_type(Cow::Owned(v.to_vec())))
            }
        }

        // 5. Option Vec -> List of values (Not PrimitiveList optimization)
        impl<'a> From<Vec<Option<$primitive_type>>> for PyroValue<'a> {
            fn from(v: Vec<Option<$primitive_type>>) -> Self {
                PyroValue::List(v.into_iter().map(|i| i.into()).collect())
            }
        }
    };
}

// -----------------------------------------------------------------------------
// Primitive Implementations
// -----------------------------------------------------------------------------

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

impl From<String> for PyroValue<'static> {
    fn from(v: String) -> Self {
        PyroValue::Str(Cow::Owned(v))
    }
}

// Note: We retain specific optimized implementations for &str and Cow to ensure zero-copy behavior
// where possible, rather than using the generic impl_ref_into_pyro which forces a Clone/Owned.

impl<'a> From<&'a String> for PyroValue<'a> {
    fn from(v: &'a String) -> Self {
        PyroValue::Str(Cow::Borrowed(v))
    }
}

impl<'a> From<&'a Vec<String>> for PyroValue<'a> {
    fn from(v: &'a Vec<String>) -> Self {
        let values = v
            .into_iter()
            .map(|v| PyroValue::Str(Cow::Borrowed(v.as_str())))
            .collect();
        PyroValue::List(values)
    }
}

impl<'a> From<&'a Option<String>> for PyroValue<'a> {
    fn from(v: &'a Option<String>) -> Self {
        match v {
            Some(v) => PyroValue::Str(Cow::Borrowed(v)),
            None => PyroValue::Null,
        }
    }
}

impl<'a> From<&'a str> for PyroValue<'a> {
    fn from(v: &'a str) -> Self {
        PyroValue::Str(Cow::Borrowed(v))
    }
}

impl<'a> From<&'a Vec<&str>> for PyroValue<'a> {
    fn from(v: &'a Vec<&str>) -> Self {
        let values = v
            .into_iter()
            .map(|v| PyroValue::Str(Cow::Borrowed(v)))
            .collect();
        PyroValue::List(values)
    }
}

impl<'a> From<&'a Option<&'a str>> for PyroValue<'a> {
    fn from(v: &'a Option<&'a str>) -> Self {
        match v {
            Some(v) => PyroValue::Str(Cow::Borrowed(v)),
            None => PyroValue::Null,
        }
    }
}

impl<'a> From<&&'a str> for PyroValue<'a> {
    fn from(v: &&'a str) -> Self {
        PyroValue::Str(Cow::Borrowed(v))
    }
}

// -----------------------------------------------------------------------------
// Row & List Conversions
// -----------------------------------------------------------------------------

impl<'a> From<PyroRow<'a>> for PyroValue<'a> {
    fn from(v: PyroRow<'a>) -> Self {
        PyroValue::Group(v)
    }
}

// Apply macro to structural types
impl_ref_into_pyro!(PyroRow<'a>);
impl_ref_into_pyro!(Vec<PyroValue<'a>>);

impl<'a> FromIterator<(String, PyroValue<'a>)> for PyroRow<'a> {
    fn from_iter<T: IntoIterator<Item = (String, PyroValue<'a>)>>(iter: T) -> Self {
        let mut row = PyroRow::new();
        for (k, v) in iter {
            row.insert(k, v);
        }
        row
    }
}

impl<'a> FromIterator<(&'a str, PyroValue<'a>)> for PyroRow<'a> {
    fn from_iter<T: IntoIterator<Item = (&'a str, PyroValue<'a>)>>(iter: T) -> Self {
        let mut row = PyroRow::new();
        for (k, v) in iter {
            row.insert_ref(k, v);
        }
        row
    }
}

impl<'a, const N: usize> From<[(&'a str, PyroValue<'a>); N]> for PyroRow<'a> {
    fn from(values: [(&'a str, PyroValue<'a>); N]) -> Self {
        let mut row = PyroRow::with_capacity(N);
        for (k, v) in values {
            row.insert_ref(k, v);
        }
        row
    }
}

impl<'a, const N: usize> From<[(&'a str, PyroValue<'a>); N]> for PyroValue<'a> {
    fn from(values: [(&'a str, PyroValue<'a>); N]) -> Self {
        let mut row = PyroRow::with_capacity(N);
        for (k, v) in values {
            row.insert_ref(k, v);
        }
        PyroValue::Group(row)
    }
}

impl<const N: usize> From<[(String, PyroValueOwned); N]> for PyroRowOwned {
    fn from(values: [(String, PyroValueOwned); N]) -> Self {
        let mut row = PyroRow::with_capacity(N);
        for (k, v) in values {
            row.insert(k, v);
        }
        row
    }
}

impl<'a, const N: usize> From<[PyroValue<'a>; N]> for PyroValue<'a> {
    fn from(values: [PyroValue<'a>; N]) -> Self {
        PyroValue::List(values.to_vec())
    }
}

impl<'a> From<Vec<PyroValue<'a>>> for PyroValue<'a> {
    fn from(values: Vec<PyroValue<'a>>) -> Self {
        PyroValue::List(values)
    }
}

// -----------------------------------------------------------------------------
// Generic conversions
// -----------------------------------------------------------------------------

impl<'a, T: ToRow + 'a> From<&'a T> for PyroValue<'a> {
    fn from(value: &'a T) -> Self {
        PyroValue::Group(value.to_row())
    }
}

impl<'a, T: ToRow + 'a> From<Vec<T>> for PyroValue<'a> {
    fn from(values: Vec<T>) -> Self {
        let values = values
            .into_iter()
            .map(|v| {
                // Safe because T lives for at least 'a
                unsafe { std::mem::transmute(PyroValue::Group(v.to_row())) }
            })
            .collect();
        PyroValue::List(values)
    }
}

impl<'a, T: ToRow + 'a> From<&'a [T]> for PyroValue<'a> {
    fn from(values: &'a [T]) -> Self {
        let values = values
            .into_iter()
            .map(|v| {
                // Safe because T lives for at least 'a
                unsafe { std::mem::transmute(PyroValue::Group(v.to_row())) }
            })
            .collect();
        PyroValue::List(values)
    }
}

impl<'a, T: Into<PyroValue<'a>>> From<Option<T>> for PyroValue<'a> {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(v) => v.into(),
            None => PyroValue::Null,
        }
    }
}

// -----------------------------------------------------------------------------
// Rkyv Conversions
// -----------------------------------------------------------------------------

#[cfg(target_endian = "little")]
impl<'a, 'b> From<&'b ArchivedPyroValue<'a>> for PyroValue<'a> {
    fn from(archived: &'b ArchivedPyroValue<'a>) -> Self {
        match archived {
            ArchivedPyroValue::Null => PyroValue::Null,

            // Primitives - direct copy
            ArchivedPyroValue::Bool(b) => PyroValue::Bool(*b),
            ArchivedPyroValue::I8(v) => PyroValue::I8(*v),
            ArchivedPyroValue::I16(v) => PyroValue::I16(v.to_native()),
            ArchivedPyroValue::I32(v) => PyroValue::I32(v.to_native()),
            ArchivedPyroValue::I64(v) => PyroValue::I64(v.to_native()),
            ArchivedPyroValue::U8(v) => PyroValue::U8(*v),
            ArchivedPyroValue::U16(v) => PyroValue::U16(v.to_native()),
            ArchivedPyroValue::U32(v) => PyroValue::U32(v.to_native()),
            ArchivedPyroValue::U64(v) => PyroValue::U64(v.to_native()),
            ArchivedPyroValue::F16(v) => {
                // SAFETY: On little-endian systems, &ArchivedF16 has the same memory layout as &f16
                PyroValue::F16(unsafe { *(v as *const _ as *const f16) })
            }
            ArchivedPyroValue::F32(v) => PyroValue::F32(v.to_native()),
            ArchivedPyroValue::F64(v) => PyroValue::F64(v.to_native()),

            ArchivedPyroValue::Timestamp(nanos) => PyroValue::Timestamp(Time(nanos.0.to_native())),

            // String - zero-copy borrow from archived data
            // SAFETY: The archived data has lifetime 'a (from ArchivedPyroValue<'a>),
            // but we're borrowing through 'b. We transmute to extend the lifetime back to 'a
            // because we know the string data lives in the archived buffer with lifetime 'a.
            ArchivedPyroValue::Str(s) => {
                let s_str = s.as_str();
                let extended: &'a str = unsafe { std::mem::transmute(s_str) };
                PyroValue::Str(Cow::Borrowed(extended))
            }

            // PrimitiveList - zero-copy borrow with unsafe transmutation
            // SAFETY: On little-endian systems, rkyv's archived primitive types have the same
            // memory layout as native Rust primitives.
            ArchivedPyroValue::PrimitiveList(pl) => {
                let borrowed_list = match pl {
                    ArchivedPrimitiveValueList::Bool(v) => {
                        let slice = v.as_slice();
                        let extended: &'a [u8] = unsafe { std::mem::transmute(slice) };
                        PrimitiveValueList::U8(Cow::Borrowed(extended))
                    }
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
                PyroValue::PrimitiveList(borrowed_list)
            }

            ArchivedPyroValue::Group(archived_row) => PyroValue::Group(PyroRow::from(archived_row)),

            ArchivedPyroValue::List(archived_list) => {
                let items: Vec<PyroValue<'a>> = archived_list
                    .iter()
                    .map(|archived_val| PyroValue::from(archived_val))
                    .collect();
                PyroValue::List(items)
            }

            ArchivedPyroValue::MapInternal(archived_map) => {
                let items: Vec<(PyroValue<'a>, PyroValue<'a>)> = archived_map
                    .iter()
                    .map(|t| (PyroValue::from(&t.0), PyroValue::from(&t.1)))
                    .collect();
                PyroValue::MapInternal(items)
            }
        }
    }
}

#[cfg(target_endian = "little")]
impl<'a> From<ArchivedPyroValue<'a>> for PyroValue<'a> {
    fn from(archived: ArchivedPyroValue<'a>) -> Self {
        PyroValue::from(&archived)
    }
}

#[cfg(target_endian = "little")]
impl<'a, 'b> From<&'b ArchivedPyroRow<'a>> for PyroRow<'a> {
    fn from(value: &'b ArchivedPyroRow<'a>) -> Self {
        let items: Vec<RowItem<'a>> = value
            .0
            .iter()
            .map(|archived_item| {
                let key_str = archived_item.key.as_str();
                let extended_key: &'a str = unsafe { std::mem::transmute(key_str) };
                RowItem {
                    key: Cow::Borrowed(extended_key),
                    value: PyroValue::from(&archived_item.value),
                }
            })
            .collect();
        PyroRow(items)
    }
}
