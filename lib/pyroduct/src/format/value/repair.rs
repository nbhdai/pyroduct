use std::borrow::Cow;
use std::convert::TryInto;

use spec::{PrimitiveDataType, PyroField, PyroType};
use thiserror::Error;

use super::{PrimitiveValueList, PyroRow, PyroValue, RowItem};

#[derive(Error, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ScalarRepairError<'a> {
    #[error("Can't cast to {0}")]
    Cast(PyroType<'a>),
    #[error("Value {0:?} out of bounds for {1}")]
    OutOfBounds(String, PyroType<'a>),
    #[error("Expected field {0} missing")]
    Missing(String),
    #[error("Invalid Scalar")]
    InvalidScalar(PyroValue<'a>),
    #[error("Failed to parse string {0} as {1}")]
    ParseError(String, PyroType<'a>),
    #[error("Unimplemented repair for {0}")]
    Unimplemented(String),
}

/// Macro for safe integer conversions using TryFrom.
macro_rules! try_cast_numeric {
    ($val:expr, $target_type:ty, $variant:ident, $data_type:expr) => {{
        match $val.try_into() {
            Ok(cast_val) => Ok(PyroValue::$variant(cast_val)),
            Err(_) => Err(ScalarRepairError::OutOfBounds(
                format!("{} -> {}", $val, stringify!($target_type)),
                $data_type,
            )),
        }
    }};
}

/// Macro for floating point conversions using `as`.
macro_rules! lossy_cast_numeric {
    ($val:expr, $target_type:ty, $variant:ident, $data_type:expr) => {{
        let cast_val = $val as $target_type;
        Ok(PyroValue::$variant(cast_val))
    }};
}

/// Repair implementation for Integers.
macro_rules! impl_int_repair {
    ($self:ident, $target_type:ty, $variant:ident, $p_type:ident) => {
        #[allow(unreachable_patterns)]
        match $self {
            // 1. Identity Case: Zero-copy return
            PyroValue::$variant(_) => return Ok($self.into_owned()),

            PyroValue::Null => return Ok(PyroValue::Null),

            // 2. Parsing
            PyroValue::Str(s) => {
                return s
                    .parse::<$target_type>()
                    .map(PyroValue::$variant)
                    .map_err(|_| {
                        ScalarRepairError::ParseError(
                            s.into_owned(),
                            PyroType::PrimitiveScalar(PrimitiveDataType::$p_type),
                        )
                    });
            }

            // 3. Numeric Casting (Creates new Owned values)
            PyroValue::I8(v) => try_cast_numeric!(
                v,
                $target_type,
                $variant,
                PyroType::PrimitiveScalar(PrimitiveDataType::$p_type)
            ),
            PyroValue::I16(v) => try_cast_numeric!(
                v,
                $target_type,
                $variant,
                PyroType::PrimitiveScalar(PrimitiveDataType::$p_type)
            ),
            PyroValue::I32(v) => try_cast_numeric!(
                v,
                $target_type,
                $variant,
                PyroType::PrimitiveScalar(PrimitiveDataType::$p_type)
            ),
            PyroValue::I64(v) => try_cast_numeric!(
                v,
                $target_type,
                $variant,
                PyroType::PrimitiveScalar(PrimitiveDataType::$p_type)
            ),
            PyroValue::U8(v) => try_cast_numeric!(
                v,
                $target_type,
                $variant,
                PyroType::PrimitiveScalar(PrimitiveDataType::$p_type)
            ),
            PyroValue::U16(v) => try_cast_numeric!(
                v,
                $target_type,
                $variant,
                PyroType::PrimitiveScalar(PrimitiveDataType::$p_type)
            ),
            PyroValue::U32(v) => try_cast_numeric!(
                v,
                $target_type,
                $variant,
                PyroType::PrimitiveScalar(PrimitiveDataType::$p_type)
            ),
            PyroValue::U64(v) => try_cast_numeric!(
                v,
                $target_type,
                $variant,
                PyroType::PrimitiveScalar(PrimitiveDataType::$p_type)
            ),

            // Float -> Int (Lossy)
            PyroValue::F32(v) => lossy_cast_numeric!(
                v,
                $target_type,
                $variant,
                PyroType::PrimitiveScalar(PrimitiveDataType::$p_type)
            ),
            PyroValue::F64(v) => lossy_cast_numeric!(
                v,
                $target_type,
                $variant,
                PyroType::PrimitiveScalar(PrimitiveDataType::$p_type)
            ),

            PyroValue::Bool(v) => Ok(PyroValue::$variant(if v {
                1 as $target_type
            } else {
                0 as $target_type
            })),

            _ => {
                return Err(ScalarRepairError::Cast(PyroType::PrimitiveScalar(
                    PrimitiveDataType::$p_type,
                )))
            }
        }
    };
}

/// Repair implementation for Floats.
macro_rules! impl_float_repair {
    ($self:ident, $target_type:ty, $variant:ident, $p_type:ident) => {
        #[allow(unreachable_patterns)]
        match $self {
            // 1. Identity Case: Zero-copy return
            PyroValue::$variant(_) => return Ok($self.into_owned()),

            PyroValue::Null => return Ok(PyroValue::Null),

            PyroValue::Str(s) => {
                return s
                    .parse::<$target_type>()
                    .map(PyroValue::$variant)
                    .map_err(|_| {
                        ScalarRepairError::ParseError(
                            s.into_owned(),
                            PyroType::PrimitiveScalar(PrimitiveDataType::$p_type),
                        )
                    });
            }

            // 3. Numeric Casting (Always lossy for float targets)
            PyroValue::I8(v) => lossy_cast_numeric!(
                v,
                $target_type,
                $variant,
                PyroType::PrimitiveScalar(PrimitiveDataType::$p_type)
            ),
            PyroValue::I16(v) => lossy_cast_numeric!(
                v,
                $target_type,
                $variant,
                PyroType::PrimitiveScalar(PrimitiveDataType::$p_type)
            ),
            PyroValue::I32(v) => lossy_cast_numeric!(
                v,
                $target_type,
                $variant,
                PyroType::PrimitiveScalar(PrimitiveDataType::$p_type)
            ),
            PyroValue::I64(v) => lossy_cast_numeric!(
                v,
                $target_type,
                $variant,
                PyroType::PrimitiveScalar(PrimitiveDataType::$p_type)
            ),
            PyroValue::U8(v) => lossy_cast_numeric!(
                v,
                $target_type,
                $variant,
                PyroType::PrimitiveScalar(PrimitiveDataType::$p_type)
            ),
            PyroValue::U16(v) => lossy_cast_numeric!(
                v,
                $target_type,
                $variant,
                PyroType::PrimitiveScalar(PrimitiveDataType::$p_type)
            ),
            PyroValue::U32(v) => lossy_cast_numeric!(
                v,
                $target_type,
                $variant,
                PyroType::PrimitiveScalar(PrimitiveDataType::$p_type)
            ),
            PyroValue::U64(v) => lossy_cast_numeric!(
                v,
                $target_type,
                $variant,
                PyroType::PrimitiveScalar(PrimitiveDataType::$p_type)
            ),
            PyroValue::F32(v) => lossy_cast_numeric!(
                v,
                $target_type,
                $variant,
                PyroType::PrimitiveScalar(PrimitiveDataType::$p_type)
            ),
            PyroValue::F64(v) => lossy_cast_numeric!(
                v,
                $target_type,
                $variant,
                PyroType::PrimitiveScalar(PrimitiveDataType::$p_type)
            ),

            PyroValue::Bool(v) => Ok(PyroValue::$variant(if v {
                1.0 as $target_type
            } else {
                0.0 as $target_type
            })),

            _ => {
                return Err(ScalarRepairError::Cast(PyroType::PrimitiveScalar(
                    PrimitiveDataType::$p_type,
                )))
            }
        }
    };
}

impl<'a> PyroValue<'a> {
    #[inline]
    pub fn repair_to_bool<'b>(
        self,
    ) -> std::result::Result<PyroValue<'static>, ScalarRepairError<'b>> {
        match self {
            PyroValue::Bool(_) => Ok(self.into_owned()), // Identity
            PyroValue::Null => Ok(PyroValue::Null),
            PyroValue::Str(s) => match s.to_lowercase().as_str() {
                "true" | "1" | "yes" => Ok(PyroValue::Bool(true)),
                "false" | "0" | "no" => Ok(PyroValue::Bool(false)),
                _ => Err(ScalarRepairError::ParseError(
                    s.into_owned(),
                    PyroType::PrimitiveScalar(PrimitiveDataType::Bool),
                )),
            },
            PyroValue::I8(v) => Ok(PyroValue::Bool(v != 0)),
            PyroValue::I32(v) => Ok(PyroValue::Bool(v != 0)),
            PyroValue::I64(v) => Ok(PyroValue::Bool(v != 0)),
            _v => Err(ScalarRepairError::Cast(PyroType::PrimitiveScalar(
                PrimitiveDataType::Bool,
            ))),
        }
    }

    pub fn repair_to_i8<'b>(
        self,
    ) -> std::result::Result<PyroValue<'static>, ScalarRepairError<'b>> {
        impl_int_repair!(self, i8, I8, I8)
    }
    pub fn repair_to_i16<'b>(
        self,
    ) -> std::result::Result<PyroValue<'static>, ScalarRepairError<'b>> {
        impl_int_repair!(self, i16, I16, I16)
    }
    pub fn repair_to_i32<'b>(
        self,
    ) -> std::result::Result<PyroValue<'static>, ScalarRepairError<'b>> {
        impl_int_repair!(self, i32, I32, I32)
    }
    pub fn repair_to_i64<'b>(
        self,
    ) -> std::result::Result<PyroValue<'static>, ScalarRepairError<'b>> {
        impl_int_repair!(self, i64, I64, I64)
    }
    pub fn repair_to_u8<'b>(
        self,
    ) -> std::result::Result<PyroValue<'static>, ScalarRepairError<'b>> {
        impl_int_repair!(self, u8, U8, U8)
    }
    pub fn repair_to_u16<'b>(
        self,
    ) -> std::result::Result<PyroValue<'static>, ScalarRepairError<'b>> {
        impl_int_repair!(self, u16, U16, U16)
    }
    pub fn repair_to_u32<'b>(
        self,
    ) -> std::result::Result<PyroValue<'static>, ScalarRepairError<'b>> {
        impl_int_repair!(self, u32, U32, U32)
    }
    pub fn repair_to_u64<'b>(
        self,
    ) -> std::result::Result<PyroValue<'static>, ScalarRepairError<'b>> {
        impl_int_repair!(self, u64, U64, U64)
    }
    pub fn repair_to_f32<'b>(
        self,
    ) -> std::result::Result<PyroValue<'static>, ScalarRepairError<'b>> {
        impl_float_repair!(self, f32, F32, F32)
    }
    pub fn repair_to_f64<'b>(
        self,
    ) -> std::result::Result<PyroValue<'static>, ScalarRepairError<'b>> {
        impl_float_repair!(self, f64, F64, F64)
    }

    #[inline]
    pub fn repair_to_utf8<'b>(self) -> std::result::Result<PyroValue<'a>, ScalarRepairError<'b>> {
        match self {
            PyroValue::Str(_) => Ok(self), // Identity: Keep Borrowed Cow
            PyroValue::Null => Ok(PyroValue::Null),

            // Must allocate new string
            PyroValue::Bool(v) => Ok(PyroValue::Str(Cow::Owned(v.to_string()))),
            PyroValue::I8(v) => Ok(PyroValue::Str(Cow::Owned(v.to_string()))),
            PyroValue::I16(v) => Ok(PyroValue::Str(Cow::Owned(v.to_string()))),
            PyroValue::I32(v) => Ok(PyroValue::Str(Cow::Owned(v.to_string()))),
            PyroValue::I64(v) => Ok(PyroValue::Str(Cow::Owned(v.to_string()))),
            PyroValue::U8(v) => Ok(PyroValue::Str(Cow::Owned(v.to_string()))),
            PyroValue::U16(v) => Ok(PyroValue::Str(Cow::Owned(v.to_string()))),
            PyroValue::U32(v) => Ok(PyroValue::Str(Cow::Owned(v.to_string()))),
            PyroValue::U64(v) => Ok(PyroValue::Str(Cow::Owned(v.to_string()))),
            PyroValue::F32(v) => Ok(PyroValue::Str(Cow::Owned(v.to_string()))),
            PyroValue::F64(v) => Ok(PyroValue::Str(Cow::Owned(v.to_string()))),
            _v => Err(ScalarRepairError::Cast(PyroType::Str)),
        }
    }

    #[inline]
    pub fn repair_primitive_list<'b>(
        self,
        inner_type: PrimitiveDataType,
        fixed_len: Option<usize>,
    ) -> std::result::Result<PyroValue<'a>, ScalarRepairError<'b>> {
        // 1. Pass through PrimitiveList if types match
        if let PyroValue::PrimitiveList(pl) = self {
            let repaired_pl = pl.repair(inner_type, fixed_len)?;
            return Ok(PyroValue::PrimitiveList(repaired_pl));
        }

        // 2. Generic List logic (try to pack into primitive)
        if let PyroValue::List(list) = self {
            if let Some(size) = fixed_len {
                if list.len() != size {
                    return Err(ScalarRepairError::OutOfBounds(
                        format!("FixedSizeList len {} != {}", list.len(), size),
                        PyroType::PrimitiveFixedList(inner_type, size),
                    ));
                }
            }

            // Pack into PrimitiveList (might own new data)
            return pack_primitive_list(list, inner_type);
        }

        // 3. Scalar broadcast
        match fixed_len {
            Some(1) | None => {
                // Determine the pyro type for the scalar repair
                let scalar_type = PyroType::PrimitiveScalar(inner_type);
                let val = self.repair(&scalar_type)?;

                // Pack single value
                pack_primitive_list(vec![val], inner_type)
            }
            _ => Err(ScalarRepairError::Cast(PyroType::PrimitiveList(inner_type))),
        }
    }

    pub fn repair<'b>(
        self,
        intended_type: &PyroType<'b>,
    ) -> std::result::Result<PyroValue<'a>, ScalarRepairError<'b>> {
        if matches!(self, PyroValue::Null) {
            return Ok(PyroValue::Null);
        }

        match intended_type {
            PyroType::Null => Ok(PyroValue::Null),

            // --- Primitives ---
            PyroType::PrimitiveScalar(pt) => match pt {
                PrimitiveDataType::Bool => self.repair_to_bool(),
                PrimitiveDataType::I8 => self.repair_to_i8(),
                PrimitiveDataType::I16 => self.repair_to_i16(),
                PrimitiveDataType::I32 => self.repair_to_i32(),
                PrimitiveDataType::I64 => self.repair_to_i64(),
                PrimitiveDataType::U8 => self.repair_to_u8(),
                PrimitiveDataType::U16 => self.repair_to_u16(),
                PrimitiveDataType::U32 => self.repair_to_u32(),
                PrimitiveDataType::U64 => self.repair_to_u64(),
                PrimitiveDataType::F16 => {
                    Err(ScalarRepairError::Unimplemented("f16 repair".into()))
                }
                PrimitiveDataType::F32 => self.repair_to_f32(),
                PrimitiveDataType::F64 => self.repair_to_f64(),
            },

            PyroType::Str => self.repair_to_utf8(),

            PyroType::Timestamp => {
                // Identity check
                if let PyroValue::Timestamp { .. } = self {
                    Ok(self)
                } else {
                    Err(ScalarRepairError::Cast(PyroType::Timestamp))
                }
            }

            // --- Optimized Primitive Lists ---
            PyroType::PrimitiveList(pt) => self.repair_primitive_list(*pt, None),
            PyroType::PrimitiveFixedList(pt, len) => self.repair_primitive_list(*pt, Some(*len)),

            // --- Generic Lists ---
            PyroType::List(inner_type, _) => {
                // inner_type is Cow<PyroType>
                let inner_ref = inner_type.as_ref();

                // If it's already a List, recurse
                if let PyroValue::List(items) = self {
                    let repaired_items: Result<Vec<PyroValue<'a>>, ScalarRepairError<'b>> =
                        items.into_iter().map(|v| v.repair(inner_ref)).collect();
                    Ok(PyroValue::List(repaired_items?))
                } else {
                    // Scalar broadcast to List
                    let val = self.repair(inner_ref)?;
                    Ok(PyroValue::List(vec![val]))
                }
            }

            // --- Structs / Groups ---
            PyroType::Group(fields) => match self {
                PyroValue::Group(row) => Ok(PyroValue::Group(row.project_repair(fields)?)),
                _ => Err(ScalarRepairError::Cast(PyroType::Group(fields.clone()))),
            },

            // --- Maps ---
            PyroType::Map { key, value } => {
                let key_type = key.as_ref();
                let value_type = value.as_ref();

                if let PyroValue::MapInternal(pairs) = self {
                    let repaired_pairs: Result<Vec<_>, ScalarRepairError<'b>> = pairs
                        .into_iter()
                        .map(|(k, v)| {
                            let rk = k.repair(key_type)?;
                            let rv = v.repair(value_type)?;
                            Ok((rk, rv))
                        })
                        .collect();
                    Ok(PyroValue::MapInternal(repaired_pairs?))
                } else {
                    Err(ScalarRepairError::Cast(PyroType::Map {
                        key: key.clone(),
                        value: value.clone(),
                    }))
                }
            }
        }
    }
}

impl<'a> PyroRow<'a> {
    /// Projects the row to the schema AND repairs types.
    /// Returns PyroRow<'a>, preserving borrowed Cows if no repair was needed.
    pub fn project_repair<'b>(
        mut self,
        fields: &[PyroField<'b>],
    ) -> std::result::Result<PyroRow<'a>, ScalarRepairError<'b>> {
        let mut new_row_vec = Vec::with_capacity(fields.len());

        for f in fields {
            let target_name = f.name();

            // Locate the field in the current row
            if let Some(pos) = self.0.iter().position(|item| item.key == target_name) {
                // Efficient swap remove to take ownership of value
                let item = self.0.swap_remove(pos);

                let repaired_val = match (f.data_type(), item.value) {
                    // Special case: Nested Group recursion
                    (PyroType::Group(inner_fields), PyroValue::Group(inner_row)) => {
                        PyroValue::Group(inner_row.project_repair(inner_fields)?)
                    }
                    // Standard case: Repair value to target type
                    (dtype, val) => val.repair(dtype)?,
                };

                new_row_vec.push(RowItem {
                    key: item.key,
                    value: repaired_val,
                });
            } else {
                if f.is_nullable() {
                    new_row_vec.push(RowItem {
                        key: Cow::Owned(target_name.to_string()),
                        value: PyroValue::Null,
                    });
                } else {
                    return Err(ScalarRepairError::Missing(target_name.to_string()));
                }
            }
        }

        Ok(PyroRow(new_row_vec))
    }
}

impl<'a> PrimitiveValueList<'a> {
    pub fn repair<'b>(
        self,
        target_type: PrimitiveDataType,
        size: Option<usize>,
    ) -> std::result::Result<PrimitiveValueList<'a>, ScalarRepairError<'b>> {
        macro_rules! cast_slice {
            ($cow:expr, $target:ty, $variant:ident) => {{
                let src = $cow.as_ref();
                let mut dst: Vec<$target> = Vec::with_capacity(src.len());
                for &val in src {
                    dst.push(val as $target);
                }
                if let Some(sz) = size {
                    if dst.len() != sz {
                        return Err(ScalarRepairError::OutOfBounds(
                            "FixedSizeList mismatch".into(),
                            PyroType::Null,
                        ));
                    }
                }
                Ok(PrimitiveValueList::$variant(Cow::Owned(dst)))
            }};
        }

        // Check identity first! If types match, return self (preserves Borrowed Cow).
        use PrimitiveDataType as P;
        match (&self, target_type) {
            (PrimitiveValueList::I32(_), P::I32) => return Ok(self),
            (PrimitiveValueList::I64(_), P::I64) => return Ok(self),
            (PrimitiveValueList::F32(_), P::F32) => return Ok(self),
            (PrimitiveValueList::F64(_), P::F64) => return Ok(self),
            (PrimitiveValueList::U8(_), P::U8) => return Ok(self),
            (PrimitiveValueList::U16(_), P::U16) => return Ok(self),
            (PrimitiveValueList::U32(_), P::U32) => return Ok(self),
            (PrimitiveValueList::U64(_), P::U64) => return Ok(self),
            (PrimitiveValueList::I8(_), P::I8) => return Ok(self),
            (PrimitiveValueList::I16(_), P::I16) => return Ok(self),
            (PrimitiveValueList::Bool(_), P::Bool) => return Ok(self),
            _ => {} // Continue to casting
        }

        // Helper macro to expand casting arms for numeric types
        macro_rules! match_cast {
            ($self_val:expr, $target_dt:expr) => {
                match ($self_val, $target_dt) {
                    // To Int32
                    (PrimitiveValueList::I64(c), P::I32) => cast_slice!(c, i32, I32),
                    (PrimitiveValueList::F64(c), P::I32) => cast_slice!(c, i32, I32),
                    (PrimitiveValueList::F32(c), P::I32) => cast_slice!(c, i32, I32),
                    (PrimitiveValueList::U8(c), P::I32) => cast_slice!(c, i32, I32),
                    (PrimitiveValueList::U16(c), P::I32) => cast_slice!(c, i32, I32),
                    (PrimitiveValueList::U32(c), P::I32) => cast_slice!(c, i32, I32),
                    (PrimitiveValueList::U64(c), P::I32) => cast_slice!(c, i32, I32),
                    (PrimitiveValueList::I8(c), P::I32) => cast_slice!(c, i32, I32),
                    (PrimitiveValueList::I16(c), P::I32) => cast_slice!(c, i32, I32),

                    // To Int64
                    (PrimitiveValueList::I32(c), P::I64) => cast_slice!(c, i64, I64),
                    (PrimitiveValueList::F64(c), P::I64) => cast_slice!(c, i64, I64),
                    (PrimitiveValueList::F32(c), P::I64) => cast_slice!(c, i64, I64),
                    (PrimitiveValueList::U8(c), P::I64) => cast_slice!(c, i64, I64),
                    (PrimitiveValueList::U16(c), P::I64) => cast_slice!(c, i64, I64),
                    (PrimitiveValueList::U32(c), P::I64) => cast_slice!(c, i64, I64),
                    (PrimitiveValueList::U64(c), P::I64) => cast_slice!(c, i64, I64),
                    (PrimitiveValueList::I8(c), P::I64) => cast_slice!(c, i64, I64),
                    (PrimitiveValueList::I16(c), P::I64) => cast_slice!(c, i64, I64),

                    // To Float32
                    (PrimitiveValueList::F64(c), P::F32) => cast_slice!(c, f32, F32),
                    (PrimitiveValueList::I64(c), P::F32) => cast_slice!(c, f32, F32),
                    (PrimitiveValueList::I32(c), P::F32) => cast_slice!(c, f32, F32),

                    // To Float64
                    (PrimitiveValueList::F32(c), P::F64) => cast_slice!(c, f64, F64),
                    (PrimitiveValueList::I64(c), P::F64) => cast_slice!(c, f64, F64),
                    (PrimitiveValueList::I32(c), P::F64) => cast_slice!(c, f64, F64),

                    // Catch-all fallthrough
                    (_v, t) => Err(ScalarRepairError::Cast(PyroType::PrimitiveList(t.clone()))),
                }
            };
        }

        match_cast!(self, target_type)
    }
}

// Pack logic needs to return 'a, effectively always Owned for now as it builds new Vecs
fn pack_primitive_list<'a, 'b>(
    list: Vec<PyroValue<'a>>,
    target_type: PrimitiveDataType,
) -> Result<PyroValue<'a>, ScalarRepairError<'b>> {
    use PrimitiveDataType as P;
    match target_type {
        P::I32 => {
            let mut vec = Vec::with_capacity(list.len());
            for v in list {
                if let PyroValue::I32(i) = v.repair_to_i32()? {
                    vec.push(i);
                }
            }
            Ok(vec.into())
        }
        P::I64 => {
            let mut vec = Vec::with_capacity(list.len());
            for v in list {
                if let PyroValue::I64(i) = v.repair_to_i64()? {
                    vec.push(i);
                }
            }
            Ok(vec.into())
        }
        P::F64 => {
            let mut vec = Vec::with_capacity(list.len());
            for v in list {
                if let PyroValue::F64(i) = v.repair_to_f64()? {
                    vec.push(i);
                }
            }
            Ok(vec.into())
        }
        P::F32 => {
            let mut vec = Vec::with_capacity(list.len());
            for v in list {
                if let PyroValue::F32(i) = v.repair_to_f32()? {
                    vec.push(i);
                }
            }
            Ok(vec.into())
        }
        P::Bool => {
            let mut vec = Vec::with_capacity(list.len());
            for v in list {
                if let PyroValue::Bool(i) = v.repair_to_bool()? {
                    vec.push(i);
                }
            }
            Ok(vec.into())
        }
        P::U8 => {
            let mut vec = Vec::with_capacity(list.len());
            for v in list {
                if let PyroValue::U8(i) = v.repair_to_u8()? {
                    vec.push(i);
                }
            }
            Ok(vec.into())
        }
        // ... (Implement other types as needed, U16/U32 etc) ...
        _ => {
            // Fallback for unimplemented packing: Generic List
            // Note: This strictly violates the schema type (PrimitiveList),
            // but is a safe fallback if the packing logic isn't exhausted.
            let pyro_type = PyroType::PrimitiveScalar(target_type);
            let repaired_items: Result<Vec<PyroValue<'a>>, ScalarRepairError<'b>> =
                list.into_iter().map(|v| v.repair(&pyro_type)).collect();
            Ok(PyroValue::List(repaired_items?))
        }
    }
}
