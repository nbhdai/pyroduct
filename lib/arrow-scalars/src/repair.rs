use std::borrow::Cow;
use std::convert::TryInto;
use std::sync::Arc;

use arrow_schema::{DataType, Field};
use thiserror::Error;

use crate::{ArrowItem, ArrowRow, ArrowValue, PrimitiveValueList};

#[derive(Error, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ScalarRepairError {
    #[error("Can't cast {0:?} to {1}")]
    Cast(ArrowValue<'static>, DataType),
    #[error("Value {0:?} out of bounds for {1}")]
    OutOfBounds(String, DataType),
    #[error("Expected field {0} missing")]
    Missing(String),
    #[error("Invalid Scalar")]
    InvalidScalar(ArrowValue<'static>),
    #[error("Failed to parse string {0} as {1}")]
    ParseError(String, DataType),
    #[error("Unimplemented repair for {0}")]
    Unimplemented(String),
}

/// Macro for safe integer conversions using TryFrom.
macro_rules! try_cast_numeric {
    ($val:expr, $target_type:ty, $variant:ident, $data_type:expr) => {{
        match $val.try_into() {
            Ok(cast_val) => Ok(ArrowValue::$variant(cast_val)),
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
        Ok(ArrowValue::$variant(cast_val))
    }};
}

/// Repair implementation for Integers.
/// OPTIMIZATION: If the variant matches, return self immediately.
macro_rules! impl_int_repair {
    ($self:ident, $target_type:ty, $variant:ident, $data_type:expr) => {
        #[allow(unreachable_patterns)]
        match $self {
            // 1. Identity Case: Zero-copy return
            ArrowValue::$variant(_) => return Ok($self),

            ArrowValue::Null => return Ok(ArrowValue::Null),

            // 2. Parsing
            ArrowValue::Str(s) => {
                return s
                    .parse::<$target_type>()
                    .map(ArrowValue::$variant)
                    .map_err(|_| ScalarRepairError::ParseError(s.into_owned(), $data_type));
            }

            // 3. Numeric Casting (Creates new Owned values)
            ArrowValue::I8(v) => try_cast_numeric!(v, $target_type, $variant, $data_type),
            ArrowValue::I16(v) => try_cast_numeric!(v, $target_type, $variant, $data_type),
            ArrowValue::I32(v) => try_cast_numeric!(v, $target_type, $variant, $data_type),
            ArrowValue::I64(v) => try_cast_numeric!(v, $target_type, $variant, $data_type),
            ArrowValue::U8(v) => try_cast_numeric!(v, $target_type, $variant, $data_type),
            ArrowValue::U16(v) => try_cast_numeric!(v, $target_type, $variant, $data_type),
            ArrowValue::U32(v) => try_cast_numeric!(v, $target_type, $variant, $data_type),
            ArrowValue::U64(v) => try_cast_numeric!(v, $target_type, $variant, $data_type),

            // Float -> Int (Lossy)
            ArrowValue::F32(v) => lossy_cast_numeric!(v, $target_type, $variant, $data_type),
            ArrowValue::F64(v) => lossy_cast_numeric!(v, $target_type, $variant, $data_type),

            ArrowValue::Bool(v) => Ok(ArrowValue::$variant(if v {
                1 as $target_type
            } else {
                0 as $target_type
            })),

            _ => return Err(ScalarRepairError::Cast($self.into_owned(), $data_type)),
        }
    };
}

/// Repair implementation for Floats.
/// OPTIMIZATION: If the variant matches, return self immediately.
macro_rules! impl_float_repair {
    ($self:ident, $target_type:ty, $variant:ident, $data_type:expr) => {
        #[allow(unreachable_patterns)]
        match $self {
            // 1. Identity Case: Zero-copy return
            ArrowValue::$variant(_) => return Ok($self),

            ArrowValue::Null => return Ok(ArrowValue::Null),

            ArrowValue::Str(s) => {
                return s
                    .parse::<$target_type>()
                    .map(ArrowValue::$variant)
                    .map_err(|_| ScalarRepairError::ParseError(s.into_owned(), $data_type));
            }

            // 3. Numeric Casting (Always lossy for float targets)
            ArrowValue::I8(v) => lossy_cast_numeric!(v, $target_type, $variant, $data_type),
            ArrowValue::I16(v) => lossy_cast_numeric!(v, $target_type, $variant, $data_type),
            ArrowValue::I32(v) => lossy_cast_numeric!(v, $target_type, $variant, $data_type),
            ArrowValue::I64(v) => lossy_cast_numeric!(v, $target_type, $variant, $data_type),
            ArrowValue::U8(v) => lossy_cast_numeric!(v, $target_type, $variant, $data_type),
            ArrowValue::U16(v) => lossy_cast_numeric!(v, $target_type, $variant, $data_type),
            ArrowValue::U32(v) => lossy_cast_numeric!(v, $target_type, $variant, $data_type),
            ArrowValue::U64(v) => lossy_cast_numeric!(v, $target_type, $variant, $data_type),
            ArrowValue::F32(v) => lossy_cast_numeric!(v, $target_type, $variant, $data_type),
            ArrowValue::F64(v) => lossy_cast_numeric!(v, $target_type, $variant, $data_type),

            ArrowValue::Bool(v) => Ok(ArrowValue::$variant(if v {
                1.0 as $target_type
            } else {
                0.0 as $target_type
            })),

            _ => return Err(ScalarRepairError::Cast($self.into_owned(), $data_type)),
        }
    };
}

impl<'a> ArrowValue<'a> {
    #[inline]
    pub fn repair_to_bool(self) -> std::result::Result<ArrowValue<'a>, ScalarRepairError> {
        match self {
            ArrowValue::Bool(_) => Ok(self), // Identity
            ArrowValue::Null => Ok(ArrowValue::Null),
            ArrowValue::Str(s) => match s.to_lowercase().as_str() {
                "true" | "1" | "yes" => Ok(ArrowValue::Bool(true)),
                "false" | "0" | "no" => Ok(ArrowValue::Bool(false)),
                _ => Err(ScalarRepairError::ParseError(
                    s.into_owned(),
                    DataType::Boolean,
                )),
            },
            ArrowValue::I8(v) => Ok(ArrowValue::Bool(v != 0)),
            ArrowValue::I32(v) => Ok(ArrowValue::Bool(v != 0)),
            ArrowValue::I64(v) => Ok(ArrowValue::Bool(v != 0)),
            v => Err(ScalarRepairError::Cast(v.into_owned(), DataType::Boolean)),
        }
    }

    pub fn repair_to_i8(self) -> std::result::Result<ArrowValue<'a>, ScalarRepairError> {
        impl_int_repair!(self, i8, I8, DataType::Int8)
    }
    pub fn repair_to_i16(self) -> std::result::Result<ArrowValue<'a>, ScalarRepairError> {
        impl_int_repair!(self, i16, I16, DataType::Int16)
    }
    pub fn repair_to_i32(self) -> std::result::Result<ArrowValue<'a>, ScalarRepairError> {
        impl_int_repair!(self, i32, I32, DataType::Int32)
    }
    pub fn repair_to_i64(self) -> std::result::Result<ArrowValue<'a>, ScalarRepairError> {
        impl_int_repair!(self, i64, I64, DataType::Int64)
    }
    pub fn repair_to_u8(self) -> std::result::Result<ArrowValue<'a>, ScalarRepairError> {
        impl_int_repair!(self, u8, U8, DataType::UInt8)
    }
    pub fn repair_to_u16(self) -> std::result::Result<ArrowValue<'a>, ScalarRepairError> {
        impl_int_repair!(self, u16, U16, DataType::UInt16)
    }
    pub fn repair_to_u32(self) -> std::result::Result<ArrowValue<'a>, ScalarRepairError> {
        impl_int_repair!(self, u32, U32, DataType::UInt32)
    }
    pub fn repair_to_u64(self) -> std::result::Result<ArrowValue<'a>, ScalarRepairError> {
        impl_int_repair!(self, u64, U64, DataType::UInt64)
    }
    pub fn repair_to_f32(self) -> std::result::Result<ArrowValue<'a>, ScalarRepairError> {
        impl_float_repair!(self, f32, F32, DataType::Float32)
    }
    pub fn repair_to_f64(self) -> std::result::Result<ArrowValue<'a>, ScalarRepairError> {
        impl_float_repair!(self, f64, F64, DataType::Float64)
    }

    #[inline]
    pub fn repair_to_utf8(self) -> std::result::Result<ArrowValue<'a>, ScalarRepairError> {
        match self {
            ArrowValue::Str(_) => Ok(self), // Identity: Keep Borrowed Cow
            ArrowValue::Null => Ok(ArrowValue::Null),

            // Must allocate new string
            ArrowValue::Bool(v) => Ok(ArrowValue::Str(Cow::Owned(v.to_string()))),
            ArrowValue::I8(v) => Ok(ArrowValue::Str(Cow::Owned(v.to_string()))),
            ArrowValue::I64(v) => Ok(ArrowValue::Str(Cow::Owned(v.to_string()))),
            ArrowValue::F64(v) => Ok(ArrowValue::Str(Cow::Owned(v.to_string()))),
            v => Err(ScalarRepairError::Cast(v.into_owned(), DataType::Utf8)),
        }
    }

    #[inline]
    pub fn repair_to_list(
        self,
        size: Option<i32>,
        field: &Arc<Field>,
    ) -> std::result::Result<ArrowValue<'a>, ScalarRepairError> {
        // 1. Pass through PrimitiveList if types match
        if let ArrowValue::PrimitiveList(pl) = self {
            let repaired_pl = pl.repair(size, field)?;
            return Ok(ArrowValue::PrimitiveList(repaired_pl));
        }

        // 2. Generic List logic
        if let ArrowValue::List(list) = self {
            if let Some(size) = size {
                if list.len() != size as usize {
                    return Err(ScalarRepairError::OutOfBounds(
                        format!("FixedSizeList len {} != {}", list.len(), size),
                        DataType::FixedSizeList(field.clone(), size),
                    ));
                }
            }

            if field.data_type().is_primitive() {
                // Pack into PrimitiveList (might own new data)
                return pack_primitive_list(list, field.data_type());
            }

            let repaired_items: Result<Vec<ArrowValue<'a>>, ScalarRepairError> = list
                .into_iter()
                .map(|v| v.repair(field.data_type()))
                .collect();

            return Ok(ArrowValue::List(repaired_items?));
        }

        // 3. Scalar broadcast
        match size {
            Some(1) | None => {
                let val = self.repair(field.data_type())?;
                if field.data_type().is_primitive() {
                    return pack_primitive_list(vec![val], field.data_type());
                }
                Ok(ArrowValue::List(vec![val]))
            }
            _ => Err(ScalarRepairError::Cast(
                self.into_owned(),
                DataType::List(field.clone()),
            )),
        }
    }

    pub fn repair(
        self,
        intended_type: &DataType,
    ) -> std::result::Result<ArrowValue<'a>, ScalarRepairError> {
        if matches!(self, ArrowValue::Null) {
            return Ok(ArrowValue::Null);
        }

        match intended_type {
            DataType::Null => Ok(ArrowValue::Null),
            DataType::Boolean => self.repair_to_bool(),
            DataType::UInt8 => self.repair_to_u8(),
            DataType::UInt16 => self.repair_to_u16(),
            DataType::UInt32 => self.repair_to_u32(),
            DataType::UInt64 => self.repair_to_u64(),
            DataType::Int8 => self.repair_to_i8(),
            DataType::Int16 => self.repair_to_i16(),
            DataType::Int32 => self.repair_to_i32(),
            DataType::Int64 => self.repair_to_i64(),
            DataType::Float16 => Err(ScalarRepairError::Unimplemented("f16 repair".into())),
            DataType::Float32 => self.repair_to_f32(),
            DataType::Float64 => self.repair_to_f64(),

            DataType::Utf8 | DataType::LargeUtf8 => self.repair_to_utf8(),

            DataType::List(field) | DataType::LargeList(field) => self.repair_to_list(None, field),
            DataType::FixedSizeList(field, size) => self.repair_to_list(Some(*size), field),

            DataType::Struct(fields) => match (self, fields.len()) {
                (ArrowValue::Group(row), _) => {
                    // Project Repair recursively, preserving lifetimes where possible
                    Ok(ArrowValue::Group(row.project_repair(fields)?))
                }
                (data, 1) => {
                    let field = fields.last().unwrap();
                    let value = data.repair(field.data_type())?;
                    let mut row = ArrowRow::new();
                    row.insert(field.name().to_string(), value);
                    Ok(ArrowValue::Group(row))
                }
                (data, _) => Err(ScalarRepairError::Cast(
                    data.into_owned(),
                    DataType::Struct(fields.clone()),
                )),
            },
            DataType::Date32 => self.repair_to_i32(),
            DataType::Date64 => self.repair_to_i64(),
            DataType::Time32(_) => self.repair_to_i32(),
            DataType::Time64(_) => self.repair_to_i64(),
            DataType::Timestamp(_, _) => self.repair_to_i64(),
            _ => Err(ScalarRepairError::Unimplemented(format!(
                "{:?}",
                intended_type
            ))),
        }
    }
}

impl<'a> ArrowRow<'a> {
    /// Projects the row to the schema AND repairs types.
    /// Returns ArrowRow<'a>, preserving borrowed Cows if no repair was needed.
    pub fn project_repair<F: AsRef<Field>>(
        mut self,
        fields: &[F],
    ) -> std::result::Result<ArrowRow<'a>, crate::ScalarRepairError> {
        let mut new_row_vec = Vec::with_capacity(fields.len());

        for f in fields {
            let target_name = f.as_ref().name().as_str();

            if let Some(pos) = self.0.iter().position(|item| item.key == target_name) {
                // Efficient swap remove
                let item = self.0.swap_remove(pos);

                let repaired_val = match (f.as_ref().data_type(), item.value) {
                    (DataType::Struct(inner_fields), ArrowValue::Group(inner_row)) => {
                        ArrowValue::Group(inner_row.project_repair(inner_fields)?)
                    }
                    (DataType::Struct(_), val) => val.repair(f.as_ref().data_type())?,
                    (dtype, val) => val.repair(dtype)?,
                };

                new_row_vec.push(ArrowItem {
                    key: item.key,
                    value: repaired_val,
                });
            } else {
                if f.as_ref().is_nullable() {
                    new_row_vec.push(ArrowItem {
                        key: Cow::Owned(target_name.to_string()),
                        value: ArrowValue::Null,
                    });
                } else {
                    return Err(crate::ScalarRepairError::Missing(target_name.to_string()));
                }
            }
        }

        Ok(ArrowRow(new_row_vec))
    }
}

impl<'a> PrimitiveValueList<'a> {
    pub fn repair(
        self,
        size: Option<i32>,
        field: &Arc<Field>,
    ) -> std::result::Result<PrimitiveValueList<'a>, ScalarRepairError> {
        macro_rules! cast_slice {
            ($cow:expr, $target:ty, $variant:ident) => {{
                let src = $cow.as_ref();
                let mut dst: Vec<$target> = Vec::with_capacity(src.len());
                for &val in src {
                    dst.push(val as $target);
                }
                if let Some(sz) = size {
                    if dst.len() != sz as usize {
                        return Err(ScalarRepairError::OutOfBounds(
                            "FixedSizeList mismatch".into(),
                            DataType::Null,
                        ));
                    }
                }
                Ok(PrimitiveValueList::$variant(Cow::Owned(dst)))
            }};
        }

        // Check identity first! If types match, return self (preserves Borrowed Cow).
        let target_type = field.data_type();
        match (&self, target_type) {
            (PrimitiveValueList::I32(_), DataType::Int32) => return Ok(self),
            (PrimitiveValueList::I64(_), DataType::Int64) => return Ok(self),
            (PrimitiveValueList::F32(_), DataType::Float32) => return Ok(self),
            (PrimitiveValueList::F64(_), DataType::Float64) => return Ok(self),
            (PrimitiveValueList::U8(_), DataType::UInt8) => return Ok(self),
            (PrimitiveValueList::U16(_), DataType::UInt16) => return Ok(self),
            (PrimitiveValueList::U32(_), DataType::UInt32) => return Ok(self),
            (PrimitiveValueList::U64(_), DataType::UInt64) => return Ok(self),
            (PrimitiveValueList::I8(_), DataType::Int8) => return Ok(self),
            (PrimitiveValueList::I16(_), DataType::Int16) => return Ok(self),
            _ => {} // Continue to casting
        }

        // Helper macro to expand casting arms for numeric types
        macro_rules! match_cast {
            ($self_val:expr, $target_dt:expr) => {
                match ($self_val, $target_dt) {
                    // To Int32
                    (PrimitiveValueList::I64(c), DataType::Int32) => cast_slice!(c, i32, I32),
                    (PrimitiveValueList::F64(c), DataType::Int32) => cast_slice!(c, i32, I32),
                    (PrimitiveValueList::F32(c), DataType::Int32) => cast_slice!(c, i32, I32),
                    (PrimitiveValueList::U8(c), DataType::Int32) => cast_slice!(c, i32, I32),
                    (PrimitiveValueList::U16(c), DataType::Int32) => cast_slice!(c, i32, I32),
                    (PrimitiveValueList::U32(c), DataType::Int32) => cast_slice!(c, i32, I32),
                    (PrimitiveValueList::U64(c), DataType::Int32) => cast_slice!(c, i32, I32),
                    (PrimitiveValueList::I8(c), DataType::Int32) => cast_slice!(c, i32, I32),
                    (PrimitiveValueList::I16(c), DataType::Int32) => cast_slice!(c, i32, I32),

                    // To Int64
                    (PrimitiveValueList::I32(c), DataType::Int64) => cast_slice!(c, i64, I64),
                    (PrimitiveValueList::F64(c), DataType::Int64) => cast_slice!(c, i64, I64),
                    (PrimitiveValueList::F32(c), DataType::Int64) => cast_slice!(c, i64, I64),
                    (PrimitiveValueList::U8(c), DataType::Int64) => cast_slice!(c, i64, I64),
                    (PrimitiveValueList::U16(c), DataType::Int64) => cast_slice!(c, i64, I64),
                    (PrimitiveValueList::U32(c), DataType::Int64) => cast_slice!(c, i64, I64),
                    (PrimitiveValueList::U64(c), DataType::Int64) => cast_slice!(c, i64, I64),
                    (PrimitiveValueList::I8(c), DataType::Int64) => cast_slice!(c, i64, I64),
                    (PrimitiveValueList::I16(c), DataType::Int64) => cast_slice!(c, i64, I64),

                    // To Float32
                    (PrimitiveValueList::F64(c), DataType::Float32) => cast_slice!(c, f32, F32),
                    (PrimitiveValueList::I64(c), DataType::Float32) => cast_slice!(c, f32, F32),
                    (PrimitiveValueList::I32(c), DataType::Float32) => cast_slice!(c, f32, F32),

                    // To Float64
                    (PrimitiveValueList::F32(c), DataType::Float64) => cast_slice!(c, f64, F64),
                    (PrimitiveValueList::I64(c), DataType::Float64) => cast_slice!(c, f64, F64),
                    (PrimitiveValueList::I32(c), DataType::Float64) => cast_slice!(c, f64, F64),

                    // Catch-all fallthrough
                    (v, t) => Err(ScalarRepairError::Cast(
                        ArrowValue::PrimitiveList(v.into_owned()),
                        t.clone(),
                    )),
                }
            };
        }

        match_cast!(self, target_type)
    }
}

// Pack logic needs to return 'a, effectively always Owned for now as it builds new Vecs
fn pack_primitive_list<'a>(
    list: Vec<ArrowValue<'a>>,
    target_type: &DataType,
) -> Result<ArrowValue<'a>, ScalarRepairError> {
    match target_type {
        DataType::Int32 => {
            let mut vec = Vec::with_capacity(list.len());
            for v in list {
                if let ArrowValue::I32(i) = v.repair_to_i32()? {
                    vec.push(i);
                }
            }
            Ok(vec.into()) // Returns ArrowValue::PrimitiveList(Cow::Owned(vec))
        }
        // ... (other types same pattern) ...
        _ => {
            let repaired_items: Result<Vec<ArrowValue<'a>>, ScalarRepairError> =
                list.into_iter().map(|v| v.repair(target_type)).collect();
            Ok(ArrowValue::List(repaired_items?))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_zero_copy_identity_repair() {
        let data = vec![1, 2, 3];
        let val = ArrowValue::from(&data[..]); // Borrowed PrimitiveList

        // Check it is borrowed
        if let ArrowValue::PrimitiveList(PrimitiveValueList::I32(Cow::Borrowed(_))) = val {
            // good
        } else {
            panic!("Should start borrowed");
        }

        // Repair to SAME type
        let list_type = DataType::List(Arc::new(Field::new("item", DataType::Int32, true)));
        let repaired = val.repair(&list_type).unwrap();

        // Should still be Borrowed
        if let ArrowValue::PrimitiveList(PrimitiveValueList::I32(Cow::Borrowed(_))) = repaired {
            println!("Success: Zero copy maintained!");
        } else {
            panic!("Repair caused unnecessary allocation!");
        }
    }

    #[test]
    fn test_string_to_int_parsing() {
        let val = ArrowValue::from("123");
        let repaired = val.repair(&DataType::Int32).unwrap();
        assert_eq!(repaired, ArrowValue::I32(123));
    }

    #[test]
    fn test_string_to_float_parsing() {
        let val = ArrowValue::from("123.45");
        let repaired = val.repair(&DataType::Float64).unwrap();
        assert_eq!(repaired, ArrowValue::F64(123.45));
    }

    #[test]
    fn test_json_numeric_casting() {
        // Serde usually makes this f64 or i64
        let json_val = json!(10.5);
        let val: ArrowValue = serde_json::from_value(json_val).unwrap();

        // Repair to Int (truncate)
        let repaired = val.repair(&DataType::Int32).unwrap();
        assert_eq!(repaired, ArrowValue::I32(10));
    }

    #[test]
    fn test_json_list_packing() {
        // JSON array of numbers -> Vec<ArrowValue>
        // NOTE: Serde + ArrowValue untagged enums might parse [1,2,3,4] as PrimitiveValueList::U8
        // because they fit in bytes. The repair logic must cast U8 -> Int32.
        let json_val = json!([1, 2, 3, 4]);
        let val: ArrowValue = serde_json::from_value(json_val).unwrap();

        // Schema expects Primitive List of Int32
        let list_type = DataType::List(Arc::new(Field::new("item", DataType::Int32, true)));

        let repaired = val.repair(&list_type).unwrap();

        match repaired {
            ArrowValue::PrimitiveList(PrimitiveValueList::I32(cow)) => {
                assert_eq!(cow.as_ref(), &[1, 2, 3, 4]);
            }
            _ => panic!("Did not pack into PrimitiveValueList::I32"),
        }
    }

    #[test]
    fn test_struct_projection_and_missing() {
        let json_val = json!({
            "keep_me": "100",
            "ignore_me": "garbage"
        });
        // Now that ArrowRow implements visit_map, this works:
        let row: ArrowRow = serde_json::from_value(json_val).unwrap();

        let fields = vec![
            Arc::new(Field::new("keep_me", DataType::Int32, false)),
            Arc::new(Field::new("missing_nullable", DataType::Utf8, true)),
        ];

        let repaired_row = row.project_repair(&fields).unwrap();

        assert_eq!(repaired_row.get("keep_me"), Some(&ArrowValue::I32(100)));
        assert_eq!(
            repaired_row.get("missing_nullable"),
            Some(&ArrowValue::Null)
        );
        assert!(repaired_row.get("ignore_me").is_none());
    }

    #[test]
    fn test_u8_deserialization_repair() {
        // User specific issue: Deserializing JSON might result in generic numbers
        // but we want specific types (u8 vs u64)

        let val = ArrowValue::from(255i32); // Simulate generic int
        let repaired = val.repair(&DataType::UInt8).unwrap();
        assert_eq!(repaired, ArrowValue::U8(255));

        // 256 is out of bounds for u8.
        // Since we use `try_into` in `impl_int_repair` for strict checking, this should Err.
        let val_overflow = ArrowValue::from(256i32);
        assert!(val_overflow.repair(&DataType::UInt8).is_err());
    }
}
