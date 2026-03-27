use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::Hash;
use std::mem::ManuallyDrop;

use chrono::{DateTime, Utc};
use half::f16;

use crate::format::value::{Time, Typeable};
use pyro_spec::PrimitiveDataType;

use super::{PrimitiveValueList, PyroRow, PyroValue};

// =============================================================================
// Macros
// =============================================================================

macro_rules! impl_try_from_primitive {
    ($target:ty, $variant:ident) => {
        impl<'a> TryFrom<PyroValue<'a>> for $target {
            type Error = PyroValue<'a>;

            fn try_from(value: PyroValue<'a>) -> Result<Self, Self::Error> {
                if let PyroValue::$variant(v) = value {
                    Ok(v)
                } else {
                    Err(value)
                }
            }
        }
    };
}

// NOTE: impl_try_from_vec_primitive! removed to avoid conflict with generic Vec<T> impl.
// The logic is now handled inside the generic impl using Typeable optimization.

/// For `&'a [T]` where T is a primitive that has a corresponding `PrimitiveValueList` variant.
/// Only succeeds if the data is borrowed (Cow::Borrowed).
macro_rules! impl_try_from_slice_primitive {
    ($target:ty, $variant:ident) => {
        impl<'a> TryFrom<PyroValue<'a>> for &'a [$target] {
            type Error = PyroValue<'a>;

            fn try_from(value: PyroValue<'a>) -> Result<Self, Self::Error> {
                match value {
                    PyroValue::PrimitiveList(PrimitiveValueList::$variant(Cow::Borrowed(s))) => {
                        Ok(s)
                    }
                    other => Err(other),
                }
            }
        }
    };
}

/// For `Cow<'a, [T]>` where T is a primitive with a `PrimitiveValueList` variant.
macro_rules! impl_try_from_cow_slice_primitive {
    ($target:ty, $variant:ident) => {
        impl<'a> TryFrom<PyroValue<'a>> for Cow<'a, [$target]> {
            type Error = PyroValue<'a>;

            fn try_from(value: PyroValue<'a>) -> Result<Self, Self::Error> {
                match value {
                    PyroValue::PrimitiveList(PrimitiveValueList::$variant(cow)) => Ok(cow),
                    other => Err(other),
                }
            }
        }
    };
}

// =============================================================================
// Null
// =============================================================================

impl<'a> TryFrom<PyroValue<'a>> for () {
    type Error = PyroValue<'a>;

    fn try_from(value: PyroValue<'a>) -> Result<Self, Self::Error> {
        if let PyroValue::Null = value {
            Ok(())
        } else {
            Err(value)
        }
    }
}

// =============================================================================
// Primitives
// =============================================================================

impl<'a> TryFrom<PyroValue<'a>> for usize {
    type Error = PyroValue<'a>;

    fn try_from(value: PyroValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PyroValue::U64(v) => Ok(v as usize),
            PyroValue::U32(v) => Ok(v as usize),
            other => Err(other),
        }
    }
}

impl<'a> TryFrom<PyroValue<'a>> for isize {
    type Error = PyroValue<'a>;

    fn try_from(value: PyroValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PyroValue::I64(v) => Ok(v as isize),
            PyroValue::I32(v) => Ok(v as isize),
            other => Err(other),
        }
    }
}

impl_try_from_primitive!(bool, Bool);
impl_try_from_primitive!(i8, I8);
impl_try_from_primitive!(i16, I16);
impl_try_from_primitive!(i32, I32);
impl_try_from_primitive!(i64, I64);
impl_try_from_primitive!(u8, U8);
impl_try_from_primitive!(u16, U16);
impl_try_from_primitive!(u32, U32);
impl_try_from_primitive!(u64, U64);
impl_try_from_primitive!(f16, F16);
impl_try_from_primitive!(f32, F32);
impl_try_from_primitive!(f64, F64);

// =============================================================================
// Strings
// =============================================================================

impl<'a> TryFrom<PyroValue<'a>> for String {
    type Error = PyroValue<'a>;

    fn try_from(value: PyroValue<'a>) -> Result<Self, Self::Error> {
        if let PyroValue::Str(cow) = value {
            Ok(cow.into_owned())
        } else {
            Err(value)
        }
    }
}

impl<'a> TryFrom<PyroValue<'a>> for Cow<'a, str> {
    type Error = PyroValue<'a>;

    fn try_from(value: PyroValue<'a>) -> Result<Self, Self::Error> {
        if let PyroValue::Str(cow) = value {
            Ok(cow)
        } else {
            Err(value)
        }
    }
}

impl<'a> TryFrom<PyroValue<'a>> for &'a str {
    type Error = PyroValue<'a>;

    fn try_from(value: PyroValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PyroValue::Str(Cow::Borrowed(s)) => Ok(s),
            _ => Err(value),
        }
    }
}

// =============================================================================
// Timestamp / DateTime
// =============================================================================

impl<'a> TryFrom<PyroValue<'a>> for Time {
    type Error = PyroValue<'a>;

    fn try_from(value: PyroValue<'a>) -> Result<Self, Self::Error> {
        if let PyroValue::Timestamp(time) = value {
            Ok(time)
        } else {
            Err(value)
        }
    }
}

impl<'a> TryFrom<PyroValue<'a>> for DateTime<Utc> {
    type Error = PyroValue<'a>;

    fn try_from(value: PyroValue<'a>) -> Result<Self, Self::Error> {
        if let PyroValue::Timestamp(time) = value {
            Ok(time.to_datetime())
        } else {
            Err(value)
        }
    }
}

// =============================================================================
// Option<T>
// =============================================================================

impl<'a, T> TryFrom<PyroValue<'a>> for Option<T>
where
    T: TryFrom<PyroValue<'a>, Error = PyroValue<'a>>,
{
    type Error = PyroValue<'a>;

    fn try_from(value: PyroValue<'a>) -> Result<Self, Self::Error> {
        match value {
            PyroValue::Null => Ok(None),
            other => T::try_from(other).map(Some),
        }
    }
}

// =============================================================================
// PrimitiveValueList (raw)
// =============================================================================

impl<'a> TryFrom<PyroValue<'a>> for PrimitiveValueList<'a> {
    type Error = PyroValue<'a>;

    fn try_from(value: PyroValue<'a>) -> Result<Self, Self::Error> {
        if let PyroValue::PrimitiveList(pl) = value {
            Ok(pl)
        } else {
            Err(value)
        }
    }
}

// =============================================================================
// Primitive Slices: &'a [T]
// =============================================================================

impl_try_from_slice_primitive!(bool, Bool);
impl_try_from_slice_primitive!(i8, I8);
impl_try_from_slice_primitive!(i16, I16);
impl_try_from_slice_primitive!(i32, I32);
impl_try_from_slice_primitive!(i64, I64);
impl_try_from_slice_primitive!(u8, U8);
impl_try_from_slice_primitive!(u16, U16);
impl_try_from_slice_primitive!(u32, U32);
impl_try_from_slice_primitive!(u64, U64);
impl_try_from_slice_primitive!(f16, F16);
impl_try_from_slice_primitive!(f32, F32);
impl_try_from_slice_primitive!(f64, F64);

// =============================================================================
// Cow<'a, [T]> for primitives
// =============================================================================

impl_try_from_cow_slice_primitive!(bool, Bool);
impl_try_from_cow_slice_primitive!(i8, I8);
impl_try_from_cow_slice_primitive!(i16, I16);
impl_try_from_cow_slice_primitive!(i32, I32);
impl_try_from_cow_slice_primitive!(i64, I64);
impl_try_from_cow_slice_primitive!(u8, U8);
impl_try_from_cow_slice_primitive!(u16, U16);
impl_try_from_cow_slice_primitive!(u32, U32);
impl_try_from_cow_slice_primitive!(u64, U64);
impl_try_from_cow_slice_primitive!(f16, F16);
impl_try_from_cow_slice_primitive!(f32, F32);
impl_try_from_cow_slice_primitive!(f64, F64);

// =============================================================================
// PyroRow — from Group
// =============================================================================

impl<'a> TryFrom<PyroValue<'a>> for PyroRow<'a> {
    type Error = PyroValue<'a>;

    fn try_from(value: PyroValue<'a>) -> Result<Self, Self::Error> {
        if let PyroValue::Group(row) = value {
            Ok(row)
        } else {
            Err(value)
        }
    }
}

impl<'a> TryFrom<PyroValue<'a>> for Vec<PyroRow<'a>> {
    type Error = PyroValue<'a>;

    fn try_from(value: PyroValue<'a>) -> Result<Self, Self::Error> {
        if let PyroValue::List(entries) = value {
            match entries
                .iter()
                .map(|v| v.clone().try_into())
                .collect::<Result<Vec<PyroRow<'a>>, _>>()
            {
                Ok(v) => Ok(v),
                Err(_) => Err(PyroValue::List(entries)),
            }
        } else {
            Err(value)
        }
    }
}

// =============================================================================
// Vec<(PyroValue, PyroValue)> — raw MapInternal extraction
// =============================================================================

impl<'a> TryFrom<PyroValue<'a>> for Vec<(PyroValue<'a>, PyroValue<'a>)> {
    type Error = PyroValue<'a>;

    fn try_from(value: PyroValue<'a>) -> Result<Self, Self::Error> {
        if let PyroValue::MapInternal(entries) = value {
            Ok(entries)
        } else {
            Err(value)
        }
    }
}

// =============================================================================
// HashMap<K, V>
// =============================================================================

impl<'a, K, V> TryFrom<PyroValue<'a>> for HashMap<K, V>
where
    K: TryFrom<PyroValue<'a>, Error = PyroValue<'a>> + Eq + Hash,
    V: TryFrom<PyroValue<'a>, Error = PyroValue<'a>>,
{
    type Error = PyroValue<'a>;

    fn try_from(value: PyroValue<'a>) -> Result<Self, Self::Error> {
        if let PyroValue::MapInternal(entries) = value {
            let mut map = HashMap::with_capacity(entries.len());
            for (k, v) in entries {
                let key = K::try_from(k)
                    .map_err(|bad_k| PyroValue::MapInternal(vec![(bad_k, PyroValue::Null)]))?;
                let val = V::try_from(v)
                    .map_err(|bad_v| PyroValue::MapInternal(vec![(PyroValue::Null, bad_v)]))?;
                map.insert(key, val);
            }
            Ok(map)
        } else {
            Err(value)
        }
    }
}

// =============================================================================
// Vec<T> — Generic Implementation with Typeable Optimization
// =============================================================================

impl<'a, T> TryFrom<PyroValue<'a>> for Vec<T>
where
    T: TryFrom<PyroValue<'a>, Error = PyroValue<'a>> + Typeable,
{
    type Error = PyroValue<'a>;

    fn try_from(value: PyroValue<'a>) -> Result<Self, Self::Error> {
        // 1. FAST PATH: Optimized primitive array extraction.
        // If T is a primitive type (not nullable) and value is a PrimitiveList,
        // we can extract the vector directly without iteration.
        //
        // SAFETY: We rely on `Typeable::primitive_data_type()` correctly identifying T.
        // If `pdt` matches the variant, `cow` contains `Vec<Prim>`.
        // Since `T` corresponds to `Prim` (via Typeable contract), we can cast `Vec<Prim>` to `Vec<T>`.
        if !T::is_nullable() {
            if let (Some(pdt), PyroValue::PrimitiveList(pl)) = (T::primitive_data_type(), &value) {
                // Macro to generate match arms: checks if runtime variant matches metadata, then casts.
                macro_rules! try_cast_primitive {
                    ($variant:ident, $prim_type:ty) => {
                        if let (PrimitiveDataType::$variant, PrimitiveValueList::$variant(cow)) =
                            (pdt, pl)
                        {
                            let vec_prim: Vec<$prim_type> = cow.clone().into_owned();

                            // Transform Vec<$prim_type> -> Vec<T>
                            // T is semantically identical to $prim_type here.
                            let mut v = ManuallyDrop::new(vec_prim);
                            let (ptr, len, cap) = (v.as_mut_ptr(), v.len(), v.capacity());

                            // Safe because we verified T's identity via Typeable
                            let vec_t = unsafe { Vec::from_raw_parts(ptr as *mut T, len, cap) };
                            return Ok(vec_t);
                        }
                    };
                }

                try_cast_primitive!(Bool, bool);
                try_cast_primitive!(I8, i8);
                try_cast_primitive!(I16, i16);
                try_cast_primitive!(I32, i32);
                try_cast_primitive!(I64, i64);
                try_cast_primitive!(U8, u8);
                try_cast_primitive!(U16, u16);
                try_cast_primitive!(U32, u32);
                try_cast_primitive!(U64, u64);
                try_cast_primitive!(F16, f16);
                try_cast_primitive!(F32, f32);
                try_cast_primitive!(F64, f64);
            }
        }

        // 2. SLOW PATH: Generic List iteration.
        // Fallback for non-primitives (Vec<String>, Vec<Row>) or mismatched types.
        if let PyroValue::List(items) = value {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                // PyroValue::List holds owned values, so we convert them to T.
                // Note: items are passed by value because `PyroValue` (enum) is moved into `try_from`.
                // However, items inside `List` are `PyroValue<'a>`.
                // `T::try_from` consumes the item.
                match T::try_from(item) {
                    Ok(v) => out.push(v),
                    Err(bad) => return Err(PyroValue::List(vec![bad])), // Return error context roughly
                }
            }
            Ok(out)
        } else {
            // Note: We do NOT handle converting PrimitiveList -> Vec<NonPrimitive> (e.g. Vec<String> from I32 list).
            // This maintains strictness parity with the original macros.
            Err(value)
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Primitives ----

    #[test]
    fn test_try_from_i32() {
        let v = PyroValue::I32(42);
        assert_eq!(i32::try_from(v), Ok(42));
    }

    #[test]
    fn test_try_from_vec_i32_fast_path() {
        // Should use the optimized path via Typeable
        let v = PyroValue::PrimitiveList(PrimitiveValueList::I32(Cow::Owned(vec![1, 2, 3])));
        let res: Vec<i32> = Vec::try_from(v).unwrap();
        assert_eq!(res, vec![1, 2, 3]);
    }

    #[test]
    fn test_try_from_vec_bool_fast_path() {
        // Previously this caused the conflict
        let v = PyroValue::PrimitiveList(PrimitiveValueList::Bool(Cow::Owned(vec![true, false])));
        let res: Vec<bool> = Vec::try_from(v).unwrap();
        assert_eq!(res, vec![true, false]);
    }

    #[test]
    fn test_try_from_vec_via_list_fallback() {
        let v = PyroValue::List(vec![PyroValue::U8(10), PyroValue::U8(20)]);
        let res: Vec<u8> = Vec::try_from(v).unwrap();
        assert_eq!(res, vec![10, 20]);
    }

    #[test]
    fn test_try_from_vec_string() {
        let v = PyroValue::List(vec![
            PyroValue::Str(Cow::Borrowed("a")),
            PyroValue::Str(Cow::Owned("b".to_string())),
        ]);
        assert_eq!(
            Vec::<String>::try_from(v),
            Ok(vec!["a".to_string(), "b".to_string()])
        );
    }

    #[test]
    fn test_try_from_vec_pyro_row() {
        let mut row = PyroRow::new();
        row.insert("k".to_string(), PyroValue::Bool(true));
        let v = PyroValue::List(vec![PyroValue::Group(row.clone())]);

        // This relies on PyroRow implementing Typeable (dummy impl)
        let rows = Vec::<PyroRow>::try_from(v).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0], row);
    }

    #[test]
    fn test_try_from_vec_option_i32() {
        // Vec<Option<i32>> is Nullable, so it should SKIP the fast path.
        // It should match PyroValue::List.
        let v = PyroValue::List(vec![PyroValue::I32(1), PyroValue::Null]);
        let res: Vec<Option<i32>> = Vec::try_from(v).unwrap();
        assert_eq!(res, vec![Some(1), None]);
    }

    #[test]
    fn test_try_from_vec_option_i32_from_primitive_list_fails() {
        // Vec<Option<i32>> assumes nullability. PrimitiveList cannot represent nulls.
        // The optimization is skipped. The fallback expects List, gets PrimitiveList.
        // Should return Err.
        let v = PyroValue::PrimitiveList(PrimitiveValueList::I32(Cow::Owned(vec![1])));
        let res = Vec::<Option<i32>>::try_from(v);
        assert!(res.is_err());
    }
}
