use std::borrow::Cow;
use std::convert::TryInto;

use half::f16;

use super::repair::ScalarRepairError;
use super::schema::{PrimitiveDataType, PyroSchema, PyroType};
use super::{PrimitiveValueList, PyroRow, PyroValue, ToRow};

// =============================================================================
// Sealed trait pattern
// =============================================================================

mod sealed {
    pub trait Sealed {}
}

pub trait IntoPyroValue<'a>: sealed::Sealed {
    /// The natural [`PyroType`] for this Rust type.
    const DATA_TYPE: PyroType<'static>;

    /// Identity conversion — produce a `PyroValue` matching this type's
    /// natural variant, borrowing from `&'a self` where possible.
    fn coerce(&'a self) -> PyroValue<'a>;

    /// Coerce this Rust value directly into a `PyroValue` of `target` type.
    ///
    /// The error lifetime `'b` is tied to the `target` type, not to `self`.
    /// This is correct because error variants embed the *target* `PyroType`
    /// for diagnostics, not the source value (which is `into_owned()` to `'static`).
    fn try_coerce_to<'b>(
        &'a self,
        target: &PyroType<'b>,
    ) -> Result<PyroValue<'a>, ScalarRepairError<'b>>;
}

// =============================================================================
// Internal: cast helpers
// =============================================================================

/// Checked integer cast: `$src as $tgt` via TryInto, wrapping in `PyroValue::$variant`.
macro_rules! try_int_cast {
    ($val:expr, $tgt:ty, $variant:ident, $dt:expr) => {
        match ($val).try_into() {
            Ok(v) => Ok(PyroValue::$variant(v)),
            Err(_) => Err(ScalarRepairError::OutOfBounds(
                format!("{} -> {}", $val, stringify!($tgt)),
                $dt,
            )),
        }
    };
}

/// Lossy float cast: `$val as $tgt`.
macro_rules! lossy_cast {
    ($val:expr, $tgt:ty, $variant:ident) => {
        Ok(PyroValue::$variant($val as $tgt))
    };
}

/// Dispatch from a concrete numeric value to a target PyroType.
///
/// `$target` is `&PyroType<'b>` — all error paths clone it into `ScalarRepairError<'b>`.
macro_rules! numeric_try_coerce {
    ($self_val:expr, $rust_type:ty, $self_variant:ident, $target:expr) => {
        #[allow(unreachable_patterns)]
        match $target {
            // --- Scalars ---
            PyroType::PrimitiveScalar(pt) => match pt {
                // Identity
                PrimitiveDataType::$self_variant => Ok(PyroValue::$self_variant(*$self_val)),

                // Float targets (lossy)
                PrimitiveDataType::F32 => lossy_cast!(*$self_val, f32, F32),
                PrimitiveDataType::F64 => lossy_cast!(*$self_val, f64, F64),

                // Bool: != 0
                PrimitiveDataType::Bool => Ok(PyroValue::Bool(*$self_val != 0 as $rust_type)),

                // Integer targets (checked)
                PrimitiveDataType::I8 => try_int_cast!(*$self_val, i8, I8, $target.clone()),
                PrimitiveDataType::I16 => try_int_cast!(*$self_val, i16, I16, $target.clone()),
                PrimitiveDataType::I32 => try_int_cast!(*$self_val, i32, I32, $target.clone()),
                PrimitiveDataType::I64 => try_int_cast!(*$self_val, i64, I64, $target.clone()),
                PrimitiveDataType::U8 => try_int_cast!(*$self_val, u8, U8, $target.clone()),
                PrimitiveDataType::U16 => try_int_cast!(*$self_val, u16, U16, $target.clone()),
                PrimitiveDataType::U32 => try_int_cast!(*$self_val, u32, U32, $target.clone()),
                PrimitiveDataType::U64 => try_int_cast!(*$self_val, u64, U64, $target.clone()),
                PrimitiveDataType::F16 => {
                    Err(ScalarRepairError::Unimplemented("f16 coerce".into()))
                }
            },

            // --- Stringify ---
            PyroType::Str => Ok(PyroValue::Str(Cow::Owned($self_val.to_string()))),

            // --- Null ---
            PyroType::Null => Ok(PyroValue::Null),

            // --- Fail ---
            _ => Err(ScalarRepairError::Cast($target.clone())),
        }
    };
}

/// Dispatch for float sources (int casts are lossy).
macro_rules! float_try_coerce {
    ($self_val:expr, $rust_type:ty, $self_variant:ident, $target:expr) => {
        #[allow(unreachable_patterns)]
        match $target {
            // --- Scalars ---
            PyroType::PrimitiveScalar(pt) => match pt {
                PrimitiveDataType::$self_variant => Ok(PyroValue::$self_variant(*$self_val)),

                // Float targets
                PrimitiveDataType::F32 => lossy_cast!(*$self_val, f32, F32),
                PrimitiveDataType::F64 => lossy_cast!(*$self_val, f64, F64),

                // Int targets (lossy for floats)
                PrimitiveDataType::I8 => lossy_cast!(*$self_val, i8, I8),
                PrimitiveDataType::I16 => lossy_cast!(*$self_val, i16, I16),
                PrimitiveDataType::I32 => lossy_cast!(*$self_val, i32, I32),
                PrimitiveDataType::I64 => lossy_cast!(*$self_val, i64, I64),
                PrimitiveDataType::U8 => lossy_cast!(*$self_val, u8, U8),
                PrimitiveDataType::U16 => lossy_cast!(*$self_val, u16, U16),
                PrimitiveDataType::U32 => lossy_cast!(*$self_val, u32, U32),
                PrimitiveDataType::U64 => lossy_cast!(*$self_val, u64, U64),

                // Bool
                PrimitiveDataType::Bool => Ok(PyroValue::Bool(*$self_val != 0.0 as $rust_type)),

                PrimitiveDataType::F16 => {
                    Err(ScalarRepairError::Unimplemented("f16 coerce".into()))
                }
            },

            // --- Stringify ---
            PyroType::Str => Ok(PyroValue::Str(Cow::Owned($self_val.to_string()))),

            // --- Null ---
            PyroType::Null => Ok(PyroValue::Null),

            _ => Err(ScalarRepairError::Cast($target.clone())),
        }
    };
}

// =============================================================================
// Integer primitives
// =============================================================================

macro_rules! impl_sealed_int {
    ($rust_type:ty, $variant:ident) => {
        impl sealed::Sealed for $rust_type {}
        impl<'a> IntoPyroValue<'a> for $rust_type {
            const DATA_TYPE: PyroType<'static> =
                PyroType::PrimitiveScalar(PrimitiveDataType::$variant);

            #[inline]
            fn coerce(&'a self) -> PyroValue<'a> {
                PyroValue::$variant(*self)
            }

            fn try_coerce_to<'b>(
                &'a self,
                target: &PyroType<'b>,
            ) -> Result<PyroValue<'a>, ScalarRepairError<'b>> {
                numeric_try_coerce!(self, $rust_type, $variant, target)
            }
        }
    };
}

impl_sealed_int!(i8, I8);
impl_sealed_int!(i16, I16);
impl_sealed_int!(i32, I32);
impl_sealed_int!(i64, I64);
impl_sealed_int!(u8, U8);
impl_sealed_int!(u16, U16);
impl_sealed_int!(u32, U32);
impl_sealed_int!(u64, U64);

// =============================================================================
// Float primitives
// =============================================================================

macro_rules! impl_sealed_float {
    ($rust_type:ty, $variant:ident) => {
        impl sealed::Sealed for $rust_type {}
        impl<'a> IntoPyroValue<'a> for $rust_type {
            const DATA_TYPE: PyroType<'static> =
                PyroType::PrimitiveScalar(PrimitiveDataType::$variant);

            #[inline]
            fn coerce(&'a self) -> PyroValue<'a> {
                PyroValue::$variant(*self)
            }

            fn try_coerce_to<'b>(
                &'a self,
                target: &PyroType<'b>,
            ) -> Result<PyroValue<'a>, ScalarRepairError<'b>> {
                float_try_coerce!(self, $rust_type, $variant, target)
            }
        }
    };
}

impl_sealed_float!(f32, F32);
impl_sealed_float!(f64, F64);

// =============================================================================
// Bool
// =============================================================================

impl sealed::Sealed for bool {}
impl<'a> IntoPyroValue<'a> for bool {
    const DATA_TYPE: PyroType<'static> = PyroType::PrimitiveScalar(PrimitiveDataType::Bool);

    #[inline]
    fn coerce(&'a self) -> PyroValue<'a> {
        PyroValue::Bool(*self)
    }

    fn try_coerce_to<'b>(
        &'a self,
        target: &PyroType<'b>,
    ) -> Result<PyroValue<'a>, ScalarRepairError<'b>> {
        let v = *self;
        match target {
            PyroType::PrimitiveScalar(pt) => match pt {
                PrimitiveDataType::Bool => Ok(PyroValue::Bool(v)),
                PrimitiveDataType::I8 => Ok(PyroValue::I8(v as i8)),
                PrimitiveDataType::I16 => Ok(PyroValue::I16(v as i16)),
                PrimitiveDataType::I32 => Ok(PyroValue::I32(v as i32)),
                PrimitiveDataType::I64 => Ok(PyroValue::I64(v as i64)),
                PrimitiveDataType::U8 => Ok(PyroValue::U8(v as u8)),
                PrimitiveDataType::U16 => Ok(PyroValue::U16(v as u16)),
                PrimitiveDataType::U32 => Ok(PyroValue::U32(v as u32)),
                PrimitiveDataType::U64 => Ok(PyroValue::U64(v as u64)),
                PrimitiveDataType::F32 => Ok(PyroValue::F32(if v { 1.0 } else { 0.0 })),
                PrimitiveDataType::F64 => Ok(PyroValue::F64(if v { 1.0 } else { 0.0 })),
                PrimitiveDataType::F16 => {
                    Err(ScalarRepairError::Unimplemented("f16 coerce".into()))
                }
            },
            PyroType::Str => Ok(PyroValue::Str(Cow::Owned(v.to_string()))),
            PyroType::Null => Ok(PyroValue::Null),
            _ => Err(ScalarRepairError::Cast(target.clone())),
        }
    }
}

// =============================================================================
// f16 — identity only
// =============================================================================

impl sealed::Sealed for f16 {}
impl<'a> IntoPyroValue<'a> for f16 {
    const DATA_TYPE: PyroType<'static> = PyroType::PrimitiveScalar(PrimitiveDataType::F16);

    #[inline]
    fn coerce(&'a self) -> PyroValue<'a> {
        PyroValue::F16(*self)
    }

    fn try_coerce_to<'b>(
        &'a self,
        target: &PyroType<'b>,
    ) -> Result<PyroValue<'a>, ScalarRepairError<'b>> {
        match target {
            PyroType::PrimitiveScalar(PrimitiveDataType::F16) => Ok(PyroValue::F16(*self)),
            PyroType::Null => Ok(PyroValue::Null),
            _ => Err(ScalarRepairError::Unimplemented("f16 coerce".into())),
        }
    }
}

// =============================================================================
// Strings
// =============================================================================

impl sealed::Sealed for String {}
impl<'a> IntoPyroValue<'a> for String {
    const DATA_TYPE: PyroType<'static> = PyroType::Str;

    #[inline]
    fn coerce(&'a self) -> PyroValue<'a> {
        PyroValue::Str(Cow::Borrowed(self.as_str()))
    }

    fn try_coerce_to<'b>(
        &'a self,
        target: &PyroType<'b>,
    ) -> Result<PyroValue<'a>, ScalarRepairError<'b>> {
        str_try_coerce_to(self.as_str(), target)
    }
}

impl sealed::Sealed for str {}
impl<'a> IntoPyroValue<'a> for str {
    const DATA_TYPE: PyroType<'static> = PyroType::Str;

    #[inline]
    fn coerce(&'a self) -> PyroValue<'a> {
        PyroValue::Str(Cow::Borrowed(self))
    }

    fn try_coerce_to<'b>(
        &'a self,
        target: &PyroType<'b>,
    ) -> Result<PyroValue<'a>, ScalarRepairError<'b>> {
        str_try_coerce_to(self, target)
    }
}

/// Shared string coercion logic.
///
/// `'a` is the borrow lifetime of the source string.
/// `'b` is the lifetime of the target `PyroType` (and thus the error).
fn str_try_coerce_to<'a, 'b>(
    s: &'a str,
    target: &PyroType<'b>,
) -> Result<PyroValue<'a>, ScalarRepairError<'b>> {
    match target {
        PyroType::Str => Ok(PyroValue::Str(Cow::Borrowed(s))),

        PyroType::PrimitiveScalar(pt) => match pt {
            PrimitiveDataType::Bool => match s.to_lowercase().as_str() {
                "true" | "1" | "yes" => Ok(PyroValue::Bool(true)),
                "false" | "0" | "no" => Ok(PyroValue::Bool(false)),
                _ => Err(ScalarRepairError::ParseError(
                    s.to_owned(),
                    PyroType::PrimitiveScalar(PrimitiveDataType::Bool),
                )),
            },
            PrimitiveDataType::I8 => s.parse::<i8>().map(PyroValue::I8).map_err(|_| {
                ScalarRepairError::ParseError(
                    s.to_owned(),
                    PyroType::PrimitiveScalar(PrimitiveDataType::I8),
                )
            }),
            PrimitiveDataType::I16 => s.parse::<i16>().map(PyroValue::I16).map_err(|_| {
                ScalarRepairError::ParseError(
                    s.to_owned(),
                    PyroType::PrimitiveScalar(PrimitiveDataType::I16),
                )
            }),
            PrimitiveDataType::I32 => s.parse::<i32>().map(PyroValue::I32).map_err(|_| {
                ScalarRepairError::ParseError(
                    s.to_owned(),
                    PyroType::PrimitiveScalar(PrimitiveDataType::I32),
                )
            }),
            PrimitiveDataType::I64 => s.parse::<i64>().map(PyroValue::I64).map_err(|_| {
                ScalarRepairError::ParseError(
                    s.to_owned(),
                    PyroType::PrimitiveScalar(PrimitiveDataType::I64),
                )
            }),
            PrimitiveDataType::U8 => s.parse::<u8>().map(PyroValue::U8).map_err(|_| {
                ScalarRepairError::ParseError(
                    s.to_owned(),
                    PyroType::PrimitiveScalar(PrimitiveDataType::U8),
                )
            }),
            PrimitiveDataType::U16 => s.parse::<u16>().map(PyroValue::U16).map_err(|_| {
                ScalarRepairError::ParseError(
                    s.to_owned(),
                    PyroType::PrimitiveScalar(PrimitiveDataType::U16),
                )
            }),
            PrimitiveDataType::U32 => s.parse::<u32>().map(PyroValue::U32).map_err(|_| {
                ScalarRepairError::ParseError(
                    s.to_owned(),
                    PyroType::PrimitiveScalar(PrimitiveDataType::U32),
                )
            }),
            PrimitiveDataType::U64 => s.parse::<u64>().map(PyroValue::U64).map_err(|_| {
                ScalarRepairError::ParseError(
                    s.to_owned(),
                    PyroType::PrimitiveScalar(PrimitiveDataType::U64),
                )
            }),
            PrimitiveDataType::F32 => s.parse::<f32>().map(PyroValue::F32).map_err(|_| {
                ScalarRepairError::ParseError(
                    s.to_owned(),
                    PyroType::PrimitiveScalar(PrimitiveDataType::F32),
                )
            }),
            PrimitiveDataType::F64 => s.parse::<f64>().map(PyroValue::F64).map_err(|_| {
                ScalarRepairError::ParseError(
                    s.to_owned(),
                    PyroType::PrimitiveScalar(PrimitiveDataType::F64),
                )
            }),
            PrimitiveDataType::F16 => Err(ScalarRepairError::Unimplemented("f16 parse".into())),
        },

        PyroType::Null => Ok(PyroValue::Null),

        _ => Err(ScalarRepairError::Cast(target.clone())),
    }
}

// =============================================================================
// Option<T>
// =============================================================================

impl<T: sealed::Sealed> sealed::Sealed for Option<T> {}
impl<'a, T: IntoPyroValue<'a>> IntoPyroValue<'a> for Option<T> {
    const DATA_TYPE: PyroType<'static> = T::DATA_TYPE;

    #[inline]
    fn coerce(&'a self) -> PyroValue<'a> {
        match self {
            Some(v) => v.coerce(),
            None => PyroValue::Null,
        }
    }

    fn try_coerce_to<'b>(
        &'a self,
        target: &PyroType<'b>,
    ) -> Result<PyroValue<'a>, ScalarRepairError<'b>> {
        match self {
            Some(v) => v.try_coerce_to(target),
            None => Ok(PyroValue::Null),
        }
    }
}

// =============================================================================
// Slices & Vecs of primitives
// =============================================================================

macro_rules! impl_sealed_slice {
    ($rust_type:ty, $list_variant:ident, $prim_dt:ident) => {
        // --- [T] ---
        impl sealed::Sealed for [$rust_type] {}
        impl<'a> IntoPyroValue<'a> for [$rust_type] {
            const DATA_TYPE: PyroType<'static> =
                PyroType::PrimitiveList(PrimitiveDataType::$prim_dt);

            #[inline]
            fn coerce(&'a self) -> PyroValue<'a> {
                PyroValue::PrimitiveList(PrimitiveValueList::$list_variant(Cow::Borrowed(self)))
            }

            fn try_coerce_to<'b>(
                &'a self,
                target: &PyroType<'b>,
            ) -> Result<PyroValue<'a>, ScalarRepairError<'b>> {
                prim_slice_try_coerce(
                    PrimitiveValueList::$list_variant(Cow::Borrowed(self)),
                    PrimitiveDataType::$prim_dt,
                    target,
                )
            }
        }

        // --- Vec<T> ---
        impl sealed::Sealed for Vec<$rust_type> {}
        impl<'a> IntoPyroValue<'a> for Vec<$rust_type> {
            const DATA_TYPE: PyroType<'static> =
                PyroType::PrimitiveList(PrimitiveDataType::$prim_dt);

            #[inline]
            fn coerce(&'a self) -> PyroValue<'a> {
                PyroValue::PrimitiveList(PrimitiveValueList::$list_variant(Cow::Borrowed(
                    self.as_slice(),
                )))
            }

            fn try_coerce_to<'b>(
                &'a self,
                target: &PyroType<'b>,
            ) -> Result<PyroValue<'a>, ScalarRepairError<'b>> {
                prim_slice_try_coerce(
                    PrimitiveValueList::$list_variant(Cow::Borrowed(self.as_slice())),
                    PrimitiveDataType::$prim_dt,
                    target,
                )
            }
        }

        // --- [T; N] ---
        impl<const N: usize> sealed::Sealed for [$rust_type; N] {}
        impl<'a, const N: usize> IntoPyroValue<'a> for [$rust_type; N] {
            const DATA_TYPE: PyroType<'static> =
                PyroType::PrimitiveList(PrimitiveDataType::$prim_dt);

            #[inline]
            fn coerce(&'a self) -> PyroValue<'a> {
                PyroValue::PrimitiveList(PrimitiveValueList::$list_variant(Cow::Borrowed(
                    self.as_slice(),
                )))
            }

            fn try_coerce_to<'b>(
                &'a self,
                target: &PyroType<'b>,
            ) -> Result<PyroValue<'a>, ScalarRepairError<'b>> {
                prim_slice_try_coerce(
                    PrimitiveValueList::$list_variant(Cow::Borrowed(self.as_slice())),
                    PrimitiveDataType::$prim_dt,
                    target,
                )
            }
        }
    };
}

/// Shared coercion logic for primitive slices/vecs/arrays.
///
/// `'a` is the borrow lifetime of the source data.
/// `'b` is the lifetime of the target `PyroType` (and thus the error).
fn prim_slice_try_coerce<'a, 'b>(
    list: PrimitiveValueList<'a>,
    src_prim: PrimitiveDataType,
    target: &PyroType<'b>,
) -> Result<PyroValue<'a>, ScalarRepairError<'b>> {
    match target {
        // Identity: PrimitiveList of same type
        PyroType::PrimitiveList(pt) if *pt == src_prim => Ok(PyroValue::PrimitiveList(list)),
        // Cross-type PrimitiveList
        PyroType::PrimitiveList(pt) => {
            let repaired = list.repair(*pt, None)?;
            Ok(PyroValue::PrimitiveList(repaired))
        }
        // PrimitiveFixedList: same element type, check size
        PyroType::PrimitiveFixedList(pt, size) if *pt == src_prim => {
            let actual = prim_list_len(&list);
            if actual != *size {
                return Err(ScalarRepairError::OutOfBounds(
                    format!("FixedSizeList len {} != {}", actual, size),
                    target.clone(),
                ));
            }
            Ok(PyroValue::PrimitiveList(list))
        }
        // Cross-type PrimitiveFixedList
        PyroType::PrimitiveFixedList(pt, size) => {
            let repaired = list.repair(*pt, Some(*size))?;
            Ok(PyroValue::PrimitiveList(repaired))
        }
        _ => Err(ScalarRepairError::Cast(target.clone())),
    }
}

fn prim_list_len(list: &PrimitiveValueList<'_>) -> usize {
    list.len()
}

impl_sealed_slice!(bool, Bool, Bool);
impl_sealed_slice!(i8, I8, I8);
impl_sealed_slice!(i16, I16, I16);
impl_sealed_slice!(i32, I32, I32);
impl_sealed_slice!(i64, I64, I64);
impl_sealed_slice!(u8, U8, U8);
impl_sealed_slice!(u16, U16, U16);
impl_sealed_slice!(u32, U32, U32);
impl_sealed_slice!(u64, U64, U64);
impl_sealed_slice!(f16, F16, F16);
impl_sealed_slice!(f32, F32, F32);
impl_sealed_slice!(f64, F64, F64);

// =============================================================================
// Vec<String>
// =============================================================================

// impl sealed::Sealed for Vec<String> {}
// impl<'a> IntoPyroValue<'a> for Vec<String> {
//     const DATA_TYPE: PyroType<'static> =
//         PyroType::List(Box::new(PyroType::Str), false);

//     fn coerce(&'a self) -> PyroValue<'a> {
//         PyroValue::List(
//             self.iter()
//                 .map(|s| PyroValue::Str(Cow::Borrowed(s.as_str())))
//                 .collect(),
//         )
//     }

//     fn try_coerce_to<'b>(
//         &'a self,
//         target: &PyroType<'b>,
//     ) -> Result<PyroValue<'a>, ScalarRepairError<'b>> {
//         match target {
//             // List(Str, _) — identity, borrow strings
//             PyroType::List(inner, _) if inner.as_ref() == &PyroType::Str => Ok(self.coerce()),
//             // List(other, _) — parse each string to the inner type
//             PyroType::List(inner, _) => {
//                 let items: Result<Vec<PyroValue<'a>>, ScalarRepairError<'b>> = self
//                     .iter()
//                     .map(|s| str_try_coerce_to(s.as_str(), inner))
//                     .collect();
//                 Ok(PyroValue::List(items?))
//             }
//             _ => Err(ScalarRepairError::Cast(
//                 target.clone(),
//             )),
//         }
//     }
// }

// =============================================================================
// CoerceToSchema
// =============================================================================

pub trait CoerceToSchema {
    fn try_coerce_to<'s, 'b>(
        &'s self,
        schema: &PyroSchema<'b>,
    ) -> Result<PyroRow<'s>, ScalarRepairError<'b>>;

    fn try_coerce_to_owned<'b>(
        &self,
        schema: &PyroSchema<'b>,
    ) -> Result<PyroRow<'static>, ScalarRepairError<'b>> {
        self.try_coerce_to(schema).map(|row| row.into_owned())
    }
}

impl<T: ToRow> CoerceToSchema for T {
    fn try_coerce_to<'s, 'b>(
        &'s self,
        schema: &PyroSchema<'b>,
    ) -> Result<PyroRow<'s>, ScalarRepairError<'b>> {
        let row = self.to_row();
        row.project_repair(schema.clone().fields())
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::format::value::PyroField;

    use super::*;

    // ================================================================
    // coerce() — identity, zero-copy
    // ================================================================

    #[test]
    fn test_coerce_i32() {
        assert_eq!(42i32.coerce(), PyroValue::I32(42));
    }

    #[test]
    fn test_coerce_f64() {
        assert_eq!(3.14f64.coerce(), PyroValue::F64(3.14));
    }

    #[test]
    fn test_coerce_bool() {
        assert_eq!(true.coerce(), PyroValue::Bool(true));
    }

    #[test]
    fn test_coerce_static_str_borrows() {
        let val = "hello".coerce();
        assert_eq!(val, PyroValue::Str(Cow::Borrowed("hello")));
        match &val {
            PyroValue::Str(Cow::Borrowed(_)) => {}
            _ => panic!("expected Cow::Borrowed"),
        }
    }

    #[test]
    fn test_coerce_string_borrows() {
        let s = String::from("world");
        let val = s.coerce();
        match &val {
            PyroValue::Str(Cow::Borrowed(b)) => assert_eq!(*b, "world"),
            _ => panic!("expected Cow::Borrowed"),
        }
    }

    #[test]
    fn test_coerce_option_some() {
        let x: Option<i32> = Some(42);
        assert_eq!(x.coerce(), PyroValue::I32(42));
    }

    #[test]
    fn test_coerce_option_none() {
        let x: Option<i32> = None;
        assert_eq!(x.coerce(), PyroValue::Null);
    }

    #[test]
    fn test_coerce_slice_borrows() {
        let data: &[i32] = &[1, 2, 3];
        match data.coerce() {
            PyroValue::PrimitiveList(PrimitiveValueList::I32(Cow::Borrowed(s))) => {
                assert_eq!(s, &[1, 2, 3]);
            }
            _ => panic!("expected borrowed PrimitiveList"),
        }
    }

    #[test]
    fn test_coerce_vec_borrows() {
        let data = vec![1.0f64, 2.0, 3.0];
        match data.coerce() {
            PyroValue::PrimitiveList(PrimitiveValueList::F64(Cow::Borrowed(_))) => {}
            _ => panic!("expected borrowed PrimitiveList"),
        }
    }

    #[test]
    fn test_coerce_fixed_array() {
        let data: [u8; 4] = [0xDE, 0xAD, 0xBE, 0xEF];
        match data.coerce() {
            PyroValue::PrimitiveList(PrimitiveValueList::U8(Cow::Borrowed(s))) => {
                assert_eq!(s, &[0xDE, 0xAD, 0xBE, 0xEF]);
            }
            _ => panic!("expected borrowed PrimitiveList"),
        }
    }

    // #[test]
    // fn test_coerce_vec_string_borrows_inner() {
    //     let data = vec!["a".to_string(), "b".to_string()];
    //     match &data.coerce() {
    //         PyroValue::List(items) => {
    //             assert_eq!(items.len(), 2);
    //             match &items[0] {
    //                 PyroValue::Str(Cow::Borrowed(_)) => {}
    //                 _ => panic!("expected inner Cow::Borrowed"),
    //             }
    //         }
    //         _ => panic!("expected List"),
    //     }
    // }

    // ================================================================
    // try_coerce_to() — cross-type coercion
    // ================================================================

    // --- Numeric → Numeric ---

    #[test]
    fn test_i32_to_f64() {
        let x: i32 = 42;
        assert_eq!(
            x.try_coerce_to(&PyroType::PrimitiveScalar(PrimitiveDataType::F64))
                .unwrap(),
            PyroValue::F64(42.0)
        );
    }

    #[test]
    fn test_i32_to_i64() {
        let x: i32 = 42;
        assert_eq!(
            x.try_coerce_to(&PyroType::PrimitiveScalar(PrimitiveDataType::I64))
                .unwrap(),
            PyroValue::I64(42)
        );
    }

    #[test]
    fn test_f64_to_i32_lossy() {
        let x: f64 = 3.14;
        assert_eq!(
            x.try_coerce_to(&PyroType::PrimitiveScalar(PrimitiveDataType::I32))
                .unwrap(),
            PyroValue::I32(3)
        );
    }

    #[test]
    fn test_u8_to_i64() {
        let x: u8 = 255;
        assert_eq!(
            x.try_coerce_to(&PyroType::PrimitiveScalar(PrimitiveDataType::I64))
                .unwrap(),
            PyroValue::I64(255)
        );
    }

    #[test]
    fn test_i64_overflow_to_i8() {
        let x: i64 = 9999;
        assert!(
            x.try_coerce_to(&PyroType::PrimitiveScalar(PrimitiveDataType::I8))
                .is_err()
        );
    }

    // --- Numeric → Str ---

    #[test]
    fn test_i32_to_str() {
        let x: i32 = 42;
        assert_eq!(
            x.try_coerce_to(&PyroType::Str).unwrap(),
            PyroValue::Str(Cow::Owned("42".into()))
        );
    }

    // --- Str → Numeric ---

    #[test]
    fn test_str_to_i32() {
        assert_eq!(
            "42".try_coerce_to(&PyroType::PrimitiveScalar(PrimitiveDataType::I32))
                .unwrap(),
            PyroValue::I32(42)
        );
    }

    #[test]
    fn test_str_to_bool() {
        assert_eq!(
            "true"
                .try_coerce_to(&PyroType::PrimitiveScalar(PrimitiveDataType::Bool))
                .unwrap(),
            PyroValue::Bool(true)
        );
        assert_eq!(
            "0".try_coerce_to(&PyroType::PrimitiveScalar(PrimitiveDataType::Bool))
                .unwrap(),
            PyroValue::Bool(false)
        );
    }

    #[test]
    fn test_str_parse_failure() {
        assert!(
            "not_a_number"
                .try_coerce_to(&PyroType::PrimitiveScalar(PrimitiveDataType::I32))
                .is_err()
        );
    }

    #[test]
    fn test_str_identity_borrows() {
        let s = String::from("hello");
        match s.try_coerce_to(&PyroType::Str).unwrap() {
            PyroValue::Str(Cow::Borrowed(b)) => assert_eq!(b, "hello"),
            _ => panic!("expected Cow::Borrowed on identity"),
        }
    }

    // --- Option passthrough ---

    #[test]
    fn test_option_some_coerce_to() {
        let x: Option<i32> = Some(42);
        assert_eq!(
            x.try_coerce_to(&PyroType::PrimitiveScalar(PrimitiveDataType::F64))
                .unwrap(),
            PyroValue::F64(42.0)
        );
    }

    #[test]
    fn test_option_none_coerce_to() {
        let x: Option<i32> = None;
        assert_eq!(
            x.try_coerce_to(&PyroType::PrimitiveScalar(PrimitiveDataType::F64))
                .unwrap(),
            PyroValue::Null
        );
    }

    // --- Slice coercion ---

    #[test]
    fn test_vec_i32_to_primitive_list_f64() {
        let data = vec![1i32, 2, 3];
        match data
            .try_coerce_to(&PyroType::PrimitiveList(PrimitiveDataType::F64))
            .unwrap()
        {
            PyroValue::PrimitiveList(PrimitiveValueList::F64(cow)) => {
                assert_eq!(cow.as_ref(), &[1.0, 2.0, 3.0]);
            }
            other => panic!("expected PrimitiveList::F64, got {:?}", other),
        }
    }

    #[test]
    fn test_vec_i32_identity_borrows() {
        let data = vec![1i32, 2, 3];
        match data
            .try_coerce_to(&PyroType::PrimitiveList(PrimitiveDataType::I32))
            .unwrap()
        {
            PyroValue::PrimitiveList(PrimitiveValueList::I32(Cow::Borrowed(_))) => {}
            _ => panic!("expected borrowed identity"),
        }
    }

    #[test]
    fn test_fixed_array_to_fixed_list() {
        let data: [f32; 3] = [1.0, 2.0, 3.0];
        let target = PyroType::PrimitiveFixedList(PrimitiveDataType::F32, 3);
        match data.try_coerce_to(&target).unwrap() {
            PyroValue::PrimitiveList(PrimitiveValueList::F32(Cow::Borrowed(_))) => {}
            _ => panic!("expected borrowed identity"),
        }
    }

    #[test]
    fn test_fixed_array_wrong_size() {
        let data: [f32; 3] = [1.0, 2.0, 3.0];
        let target = PyroType::PrimitiveFixedList(PrimitiveDataType::F32, 5);
        assert!(data.try_coerce_to(&target).is_err());
    }

    // --- Vec<String> coercion ---

    // #[test]
    // fn test_vec_string_to_list_i32() {
    //     let data = vec!["1".to_string(), "2".to_string(), "3".to_string()];
    //     let target = PyroType::List(
    //         Box::new(PyroType::PrimitiveScalar(PrimitiveDataType::I32)),
    //         false,
    //     );
    //     match data.try_coerce_to(&target).unwrap() {
    //         PyroValue::List(items) => {
    //             assert_eq!(
    //                 items,
    //                 vec![
    //                     PyroValue::I32(1),
    //                     PyroValue::I32(2),
    //                     PyroValue::I32(3)
    //                 ]
    //             );
    //         }
    //         _ => panic!("expected List"),
    //     }
    // }

    // --- Null target ---

    #[test]
    fn test_any_to_null() {
        assert_eq!(
            42i32.try_coerce_to(&PyroType::Null).unwrap(),
            PyroValue::Null
        );
        assert_eq!(
            "hello".try_coerce_to(&PyroType::Null).unwrap(),
            PyroValue::Null
        );
    }

    // ================================================================
    // CoerceToSchema
    // ================================================================

    struct TestSensor {
        id: String,
        value: f32,
        label: String,
    }

    impl ToRow for TestSensor {
        fn to_row(&self) -> PyroRow<'_> {
            PyroRow::from([
                ("id", PyroValue::Str(Cow::Borrowed(&self.id))),
                ("value", PyroValue::F32(self.value)),
                ("label", PyroValue::Str(Cow::Borrowed(&self.label))),
            ])
        }
    }

    #[test]
    fn test_struct_coerce_to_schema_type_repair() {
        let schema = PyroSchema::new(vec![
            PyroField::new("id", PyroType::Str, false),
            PyroField::new(
                "value",
                PyroType::PrimitiveScalar(PrimitiveDataType::F64),
                false,
            ),
        ]);

        let sensor = TestSensor {
            id: "s1".into(),
            value: 3.14,
            label: "temp".into(),
        };

        let row = sensor.try_coerce_to(&schema).unwrap();
        assert_eq!(row.len(), 2);
        assert_eq!(row.get("id"), Some(&PyroValue::Str(Cow::Borrowed("s1"))));
        match row.get("value") {
            Some(PyroValue::F64(v)) => assert!((*v - 3.14f32 as f64).abs() < 1e-6),
            other => panic!("expected F64, got {:?}", other),
        }
        assert_eq!(row.get("label"), None);
    }

    #[test]
    fn test_struct_coerce_missing_nullable() {
        let schema = PyroSchema::new(vec![
            PyroField::new("id", PyroType::Str, false),
            PyroField::new(
                "extra",
                PyroType::PrimitiveScalar(PrimitiveDataType::I32),
                true,
            ),
        ]);

        let sensor = TestSensor {
            id: "s1".into(),
            value: 3.14,
            label: "temp".into(),
        };
        let row = sensor.try_coerce_to(&schema).unwrap();
        assert_eq!(row.len(), 2);
        assert_eq!(row.get("extra"), Some(&PyroValue::Null));
    }
}
