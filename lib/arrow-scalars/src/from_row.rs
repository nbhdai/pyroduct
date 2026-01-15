use crate::{ArrowRow, ArrowValue, PrimitiveValueList, ToValue};
use half::f16;
use std::borrow::Cow;

pub trait FromValue<'a>: 'a + Sized {
    fn from_value(value: &ArrowValue<'a>) -> std::result::Result<Self, String>;
}

pub trait FromRow<'a>: 'a + Sized {
    fn from_row(row: &ArrowRow<'a>) -> std::result::Result<Self, String>;
}

impl<'a, T: FromRow<'a>> FromValue<'a> for T {
    fn from_value(value: &ArrowValue<'a>) -> Result<Self, String> {
        match value {
            ArrowValue::Group(row) => T::from_row(row),
            _ => Err(format!(
                "Expected a List of structs for Vec<T>, got a value in the list"
            )),
        }
    }
}

impl<'a, T: FromRow<'a>> FromValue<'a> for Vec<T> {
    fn from_value(value: &ArrowValue<'a>) -> Result<Self, String> {
        match value {
            ArrowValue::List(values) => values
                .iter()
                .map(|v| match v {
                    ArrowValue::Group(row) => T::from_row(row),
                    _ => Err(format!(
                        "Expected a List of structs for Vec<T>, got a value in the list"
                    )),
                })
                .collect(),
            _ => Err(format!(
                "Expected a List of structs for Vec<T>, got a value"
            )),
        }
    }
}

// =========================================================================
// 1. Strings (&'a str)
// =========================================================================
impl<'a> FromValue<'a> for &'a str {
    fn from_value(value: &ArrowValue<'a>) -> Result<Self, String> {
        match value {
            ArrowValue::Str(s) => {
                // SAFETY: The archived data has lifetime 'a (from ArchivedArrowValue<'a>),
                // but we're borrowing through '_. We transmute to extend the lifetime back to 'a
                // because we know the string data lives in the archived buffer with lifetime 'a.
                let s_str = s.as_ref();
                let extended: &'a str = unsafe { std::mem::transmute(s_str) };
                Ok(extended.as_ref())
            }
            _ => Err(format!("Expected Str, got {:?}", value)),
        }
    }
}

impl<'a> FromValue<'a> for Vec<&'a str> {
    fn from_value(value: &ArrowValue<'a>) -> Result<Self, String> {
        match value {
            ArrowValue::List(items) => items
                .iter()
                .map(|item| <&str as FromValue<'a>>::from_value(item))
                .collect(),
            _ => Err(format!("Expected List for Vec<&str>, got {:?}", value)),
        }
    }
}

// =========================================================================
// 2. Primitive Slices (&'a [T])
// =========================================================================
macro_rules! impl_slice_from_value {
    ($t:ty, $variant:ident) => {
        impl<'a> FromValue<'a> for &'a [$t] {
            fn from_value(value: &ArrowValue<'a>) -> Result<Self, String> {
                match value {
                    ArrowValue::PrimitiveList(PrimitiveValueList::$variant(cow)) => {
                        // SAFETY: The archived data has lifetime 'a (from ArchivedArrowValue<'a>),
                        // but we're borrowing through '_. We transmute to extend the lifetime back to 'a
                        // because we know the string data lives in the archived buffer with lifetime 'a.
                        let t_slice = cow.as_ref();
                        let extended: &'a [$t] = unsafe { std::mem::transmute(t_slice) };
                        Ok(extended.as_ref())
                    }
                    _ => Err(format!(
                        "Expected PrimitiveList({}), got {:?}",
                        stringify!($variant),
                        value
                    )),
                }
            }
        }

        impl<'a> ToValue for &'a [$t] {
            fn to_value(&self) -> ArrowValue<'a> {
                ArrowValue::PrimitiveList(PrimitiveValueList::$variant(Cow::Borrowed(self)))
            }
        }

        impl<'a> ToValue for &'a Vec<$t> {
            fn to_value(&self) -> ArrowValue<'a> {
                ArrowValue::PrimitiveList(PrimitiveValueList::$variant(Cow::Borrowed(self)))
            }
        }
    };
}

impl_slice_from_value!(bool, Bool);
impl_slice_from_value!(u8, U8);
impl_slice_from_value!(u16, U16);
impl_slice_from_value!(u32, U32);
impl_slice_from_value!(u64, U64);
impl_slice_from_value!(i8, I8);
impl_slice_from_value!(i16, I16);
impl_slice_from_value!(i32, I32);
impl_slice_from_value!(i64, I64);
impl_slice_from_value!(f16, F16);
impl_slice_from_value!(f32, F32);
impl_slice_from_value!(f64, F64);

// =========================================================================
// 3. Primitive Scalars (i32, f64, bool, etc.)
// =========================================================================
macro_rules! impl_primitive_from_value {
    ($t:ty, $variant:ident) => {
        impl<'a> FromValue<'a> for $t {
            fn from_value(value: &ArrowValue<'a>) -> Result<Self, String> {
                match value {
                    ArrowValue::$variant(v) => Ok(*v),
                    _ => Err(format!(
                        "Expected {}, got {:?}",
                        stringify!($variant),
                        value
                    )),
                }
            }
        }
    };
}

impl_primitive_from_value!(bool, Bool);
impl_primitive_from_value!(i8, I8);
impl_primitive_from_value!(i16, I16);
impl_primitive_from_value!(i32, I32);
impl_primitive_from_value!(i64, I64);
impl_primitive_from_value!(u8, U8);
impl_primitive_from_value!(u16, U16);
impl_primitive_from_value!(u32, U32);
impl_primitive_from_value!(u64, U64);
impl_primitive_from_value!(f16, F16);
impl_primitive_from_value!(f32, F32);
impl_primitive_from_value!(f64, F64);

// =========================================================================
// 4. Vec<T> for primitives
// =========================================================================
macro_rules! impl_vec_primitive_from_value {
    ($t:ty, $variant:ident) => {
        impl<'a> FromValue<'a> for Vec<$t> {
            fn from_value(value: &ArrowValue<'a>) -> Result<Self, String> {
                match value {
                    ArrowValue::PrimitiveList(PrimitiveValueList::$variant(cow)) => {
                        Ok(cow.to_vec())
                    }
                    ArrowValue::List(items) => items
                        .iter()
                        .map(|item| <$t as FromValue<'a>>::from_value(item))
                        .collect(),
                    _ => Err(format!(
                        "Expected PrimitiveList or List for Vec<{}>, got {:?}",
                        stringify!($t),
                        value
                    )),
                }
            }
        }
    };
}

impl_vec_primitive_from_value!(i8, I8);
impl_vec_primitive_from_value!(i16, I16);
impl_vec_primitive_from_value!(i32, I32);
impl_vec_primitive_from_value!(i64, I64);
impl_vec_primitive_from_value!(u8, U8);
impl_vec_primitive_from_value!(u16, U16);
impl_vec_primitive_from_value!(u32, U32);
impl_vec_primitive_from_value!(u64, U64);
impl_vec_primitive_from_value!(f16, F16);
impl_vec_primitive_from_value!(f32, F32);
impl_vec_primitive_from_value!(f64, F64);

// =========================================================================
// 5. Option<T>
// =========================================================================
impl<'a, T: FromValue<'a>> FromValue<'a> for Option<T> {
    fn from_value(value: &ArrowValue<'a>) -> Result<Self, String> {
        match value {
            ArrowValue::Null => Ok(None),
            _ => Ok(Some(T::from_value(value)?)),
        }
    }
}

// =========================================================================
// 6. String (owned)
// =========================================================================
impl<'a> FromValue<'a> for String {
    fn from_value(value: &ArrowValue<'a>) -> Result<Self, String> {
        match value {
            ArrowValue::Str(cow) => Ok(cow.to_string()),
            _ => Err(format!("Expected Str, got {:?}", value)),
        }
    }
}
