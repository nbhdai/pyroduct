use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use arrow_schema::{DataType, Field, Schema};
use thiserror::Error;

use crate::{ArrowRow, ArrowValue, PrimitiveValueList};

// -----------------------------------------------------------------------------
// Errors
// -----------------------------------------------------------------------------

#[derive(Debug, Error, PartialEq, Clone)]
pub enum SchemaInferenceError {
    #[error("Incompatible types for field '{0}': {1:?} vs {2:?}")]
    IncompatibleTypes(String, DataType, DataType),
    #[error("Ambiguous type for field '{0}': only Null values observed")]
    AmbiguousNullType(String),
    #[error("Row is empty, cannot infer schema")]
    EmptyRow,
}

// -----------------------------------------------------------------------------
// Public API
// -----------------------------------------------------------------------------

/// Generates a "trusted" schema from a single ArrowRow.
///
/// This assumes the row is the definitive source of truth. No coercion is performed.
/// - Fields present in the row become fields in the schema.
/// - Values are mapped directly to their logical DataType.
/// - Null values result in `DataType::Null`.
pub fn trusted_schema(row: &ArrowRow<'_>) -> Result<Schema, SchemaInferenceError> {
    if row.is_empty() {
        return Err(SchemaInferenceError::EmptyRow);
    }

    let fields: Vec<Field> = row
        .iter()
        .map(|(key, value)| {
            let dt = get_logical_type(value);
            // In a trusted schema from a single row, we usually assume nullable=true 
            // to be safe for future rows, unless strictly specified otherwise.
            Field::new(key, dt, true)
        })
        .collect();

    Ok(Schema::new(fields))
}

/// Infers an Arrow Schema from a slice of ArrowRows.
///
/// This process involves:
/// 1. Identifying all unique field names across all rows.
/// 2. Inferring the data type for each field by observing all values for that key.
/// 3. Coercing mixed types (e.g., Int32 and Float64 -> Float64) to a common compatible type.
/// 4. Marking fields as nullable if they are missing from some rows or contain Null values.
pub fn infer_schema(rows: &[ArrowRow<'_>]) -> Result<Arc<Schema>, SchemaInferenceError> {
    let mut field_map: BTreeMap<String, FieldAccumulator> = BTreeMap::new();

    for (i, row) in rows.iter().enumerate() {
        let mut row_keys: HashSet<&str> = HashSet::new();

        for (key, value) in row.iter() {
            row_keys.insert(key);
            let is_new_field = !field_map.contains_key(key);
            
            let accum = field_map
                .entry(key.to_string())
                .or_insert_with(FieldAccumulator::new);
            
            if is_new_field && i > 0 {
                accum.nullable = true;
            }

            accum.observe_value(key, value)?;
        }

        for (key, accum) in field_map.iter_mut() {
            if !row_keys.contains(key.as_str()) {
                accum.nullable = true;
            }
        }
    }

    let mut fields = Vec::with_capacity(field_map.len());
    for (name, accum) in field_map {
        fields.push(accum.build_field(&name)?);
    }

    Ok(Arc::new(Schema::new(fields)))
}

// -----------------------------------------------------------------------------
// Internal Logic
// -----------------------------------------------------------------------------

struct FieldAccumulator {
    data_type: Option<DataType>,
    nullable: bool,
}

impl FieldAccumulator {
    fn new() -> Self {
        Self {
            data_type: None,
            nullable: false,
        }
    }

    fn observe_value(&mut self, field_name: &str, value: &ArrowValue<'_>) -> Result<(), SchemaInferenceError> {
        if matches!(value, ArrowValue::Null) {
            self.nullable = true;
            return Ok(());
        }

        let observed_type = get_logical_type(value);

        match &self.data_type {
            None => {
                self.data_type = Some(observed_type);
            }
            Some(current_type) => {
                if current_type != &observed_type {
                    // Try to coerce to a common type
                    let coerced = coerce_types(current_type, &observed_type).ok_or_else(|| {
                        SchemaInferenceError::IncompatibleTypes(
                            field_name.to_string(),
                            current_type.clone(),
                            observed_type.clone(),
                        )
                    })?;
                    self.data_type = Some(coerced);
                }
            }
        }
        Ok(())
    }

    fn build_field(self, name: &str) -> Result<Field, SchemaInferenceError> {
        // If we only saw nulls, we can't safely infer a type other than Null.
        // Some systems default to Utf8 here, but Arrow expects explicit types.
        let dt = self.data_type.unwrap_or(DataType::Null);
        
        Ok(Field::new(name, dt, self.nullable))
    }
}

/// Extract the logical DataType from an ArrowValue.
/// For Containers (List, Group), this is recursive.
fn get_logical_type(value: &ArrowValue<'_>) -> DataType {
    match value {
        ArrowValue::Null => DataType::Null,
        ArrowValue::Bool(_) => DataType::Boolean,
        ArrowValue::I8(_) => DataType::Int8,
        ArrowValue::I16(_) => DataType::Int16,
        ArrowValue::I32(_) => DataType::Int32,
        ArrowValue::I64(_) => DataType::Int64,
        ArrowValue::U8(_) => DataType::UInt8,
        ArrowValue::U16(_) => DataType::UInt16,
        ArrowValue::U32(_) => DataType::UInt32,
        ArrowValue::U64(_) => DataType::UInt64,
        ArrowValue::F16(_) => DataType::Float16,
        ArrowValue::F32(_) => DataType::Float32,
        ArrowValue::F64(_) => DataType::Float64,
        ArrowValue::Str(_) => DataType::Utf8, // Default to Utf8
        ArrowValue::IntervalDayTime { .. } => DataType::Interval(arrow_schema::IntervalUnit::DayTime),
        
        ArrowValue::PrimitiveList(pl) => {
             match pl {
                 PrimitiveValueList::Bool(_) => DataType::List(Arc::new(Field::new("item", DataType::Boolean, true))),
                 PrimitiveValueList::U8(_) => DataType::List(Arc::new(Field::new("item", DataType::UInt8, true))),
                 PrimitiveValueList::U16(_) => DataType::List(Arc::new(Field::new("item", DataType::UInt16, true))),
                 PrimitiveValueList::U32(_) => DataType::List(Arc::new(Field::new("item", DataType::UInt32, true))),
                 PrimitiveValueList::U64(_) => DataType::List(Arc::new(Field::new("item", DataType::UInt64, true))),
                 PrimitiveValueList::I8(_) => DataType::List(Arc::new(Field::new("item", DataType::Int8, true))),
                 PrimitiveValueList::I16(_) => DataType::List(Arc::new(Field::new("item", DataType::Int16, true))),
                 PrimitiveValueList::I32(_) => DataType::List(Arc::new(Field::new("item", DataType::Int32, true))),
                 PrimitiveValueList::I64(_) => DataType::List(Arc::new(Field::new("item", DataType::Int64, true))),
                 PrimitiveValueList::F16(_) => DataType::List(Arc::new(Field::new("item", DataType::Float16, true))),
                 PrimitiveValueList::F32(_) => DataType::List(Arc::new(Field::new("item", DataType::Float32, true))),
                 PrimitiveValueList::F64(_) => DataType::List(Arc::new(Field::new("item", DataType::Float64, true))),
             }
        }
        
        ArrowValue::List(items) => {
            // Best effort inference for generic lists:
            // Scan items to find a common supertype.
            let mut common_type = DataType::Null;
            
            for item in items {
                let item_type = get_logical_type(item);
                if common_type == DataType::Null {
                    common_type = item_type;
                } else if item_type != DataType::Null {
                    if let Some(coerced) = coerce_types(&common_type, &item_type) {
                        common_type = coerced;
                    } 
                    // If coercion fails inside get_logical_type, we might default to Null 
                    // or keep the previous type. For now, we assume best effort.
                }
            }
            DataType::List(Arc::new(Field::new("item", common_type, true)))
        }

        ArrowValue::Group(row) => {
            let fields: Vec<Field> = row.iter().map(|(k, v)| {
                Field::new(k, get_logical_type(v), true)
            }).collect();
            DataType::Struct(fields.into())
        }

        ArrowValue::MapInternal(_) => DataType::Map(
            Arc::new(Field::new(
                "entries",
                DataType::Struct(vec![
                    Arc::new(Field::new("key", DataType::Utf8, false)), 
                    Arc::new(Field::new("value", DataType::Null, true))
                ].into()),
                false
            )),
            false
        ),
    }
}

/// Coerces two data types to a compatible common type (Widening).
/// Returns None if incompatible.
fn coerce_types(t1: &DataType, t2: &DataType) -> Option<DataType> {
    if t1 == t2 {
        return Some(t1.clone());
    }

    match (t1, t2) {
        // --- Numerics ---
        (DataType::Null, other) | (other, DataType::Null) => Some(other.clone()),
        
        // Signed Integers
        (DataType::Int8, DataType::Int16) | (DataType::Int16, DataType::Int8) => Some(DataType::Int16),
        (DataType::Int8, DataType::Int32) | (DataType::Int32, DataType::Int8) => Some(DataType::Int32),
        (DataType::Int8, DataType::Int64) | (DataType::Int64, DataType::Int8) => Some(DataType::Int64),
        
        (DataType::Int16, DataType::Int32) | (DataType::Int32, DataType::Int16) => Some(DataType::Int32),
        (DataType::Int16, DataType::Int64) | (DataType::Int64, DataType::Int16) => Some(DataType::Int64),
        
        (DataType::Int32, DataType::Int64) | (DataType::Int64, DataType::Int32) => Some(DataType::Int64),

        // Unsigned Integers
        (DataType::UInt8, DataType::UInt16) | (DataType::UInt16, DataType::UInt8) => Some(DataType::UInt16),
        (DataType::UInt8, DataType::UInt32) | (DataType::UInt32, DataType::UInt8) => Some(DataType::UInt32),
        (DataType::UInt8, DataType::UInt64) | (DataType::UInt64, DataType::UInt8) => Some(DataType::UInt64),
        
        // Mixed Signed/Unsigned - Promote to Signed larger
        (DataType::UInt32, DataType::Int32) | (DataType::Int32, DataType::UInt32) => Some(DataType::Int64),
        (DataType::UInt64, DataType::Int64) | (DataType::Int64, DataType::UInt64) => Some(DataType::Int64), 
        
        // Floats
        (DataType::Float16, DataType::Float32) | (DataType::Float32, DataType::Float16) => Some(DataType::Float32),
        (DataType::Float32, DataType::Float64) | (DataType::Float64, DataType::Float32) => Some(DataType::Float64),
        
        // Int/Float mixing -> Float
        (DataType::Int8, DataType::Float64) | (DataType::Float64, DataType::Int8) => Some(DataType::Float64),
        (DataType::Int16, DataType::Float64) | (DataType::Float64, DataType::Int16) => Some(DataType::Float64),
        (DataType::Int32, DataType::Float64) | (DataType::Float64, DataType::Int32) => Some(DataType::Float64),
        (DataType::Int64, DataType::Float64) | (DataType::Float64, DataType::Int64) => Some(DataType::Float64),
        
        // Strings
        (DataType::Utf8, DataType::LargeUtf8) | (DataType::LargeUtf8, DataType::Utf8) => Some(DataType::LargeUtf8),

        // Lists
        (DataType::List(f1), DataType::List(f2)) => {
            let inner = coerce_types(f1.data_type(), f2.data_type())?;
            Some(DataType::List(Arc::new(Field::new("item", inner, true))))
        }
        
        // Structs - Merge fields
        (DataType::Struct(f1), DataType::Struct(f2)) => {
            let mut merged_map = BTreeMap::new();
            
            let mut add_fields = |fields: &[Arc<Field>]| {
                for f in fields {
                    merged_map.entry(f.name().clone()).or_insert_with(Vec::new).push(f.clone());
                }
            };

            add_fields(f1);
            add_fields(f2);

            let merged_fields: Vec<Arc<Field>> = merged_map.into_iter().map(|(name, candidates)| {
                // Coerce all candidates for this field
                let mut iter = candidates.into_iter();
                let first = iter.next().unwrap();
                let final_type = iter.try_fold(first.data_type().clone(), |acc, f| {
                    coerce_types(&acc, f.data_type())
                })?;
                
                Some(Arc::new(Field::new(name, final_type, true)))
            }).collect::<Option<Vec<_>>>()?; // If any field failed, fail struct merge

            Some(DataType::Struct(merged_fields.into()))
        }

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ArrowRow;

    #[test]
    fn test_simple_inference() {
        let rows = vec![
            ArrowRow::from([("a", ArrowValue::I32(10))]),
            ArrowRow::from([("a", ArrowValue::I32(20))]),
        ];

        let schema = infer_schema(&rows).unwrap();
        assert_eq!(schema.fields().len(), 1);
        assert_eq!(schema.field(0).name(), "a");
        assert_eq!(schema.field(0).data_type(), &DataType::Int32);
        assert_eq!(schema.field(0).is_nullable(), false);
    }

    #[test]
    fn test_nullable_inference() {
        let rows = vec![
            ArrowRow::from([("a", ArrowValue::I32(10))]),
            ArrowRow::from([("a", ArrowValue::Null)]), // Explicit Null
        ];

        let schema = infer_schema(&rows).unwrap();
        assert_eq!(schema.field(0).data_type(), &DataType::Int32);
        assert!(schema.field(0).is_nullable());
    }

    #[test]
    fn test_sparse_rows_inference() {
        let rows = vec![
            ArrowRow::from([("a", ArrowValue::I32(10))]),
            ArrowRow::from([("b", ArrowValue::F64(1.0))]), // 'a' missing here
        ];

        let schema = infer_schema(&rows).unwrap();
        assert_eq!(schema.fields().len(), 2);
        
        let field_a = schema.field_with_name("a").unwrap();
        assert_eq!(field_a.data_type(), &DataType::Int32);
        assert!(field_a.is_nullable());

        let field_b = schema.field_with_name("b").unwrap();
        assert_eq!(field_b.data_type(), &DataType::Float64);
        assert!(field_b.is_nullable());
    }

    #[test]
    fn test_numeric_coercion() {
        // Int32 + Float64 -> Float64
        let rows = vec![
            ArrowRow::from([("mix", ArrowValue::I32(10))]),
            ArrowRow::from([("mix", ArrowValue::F64(20.5))]),
        ];

        let schema = infer_schema(&rows).unwrap();
        assert_eq!(schema.field(0).data_type(), &DataType::Float64);
    }

    #[test]
    fn test_int_widening() {
        // Int8 + Int32 -> Int32
        let rows = vec![
            ArrowRow::from([("num", ArrowValue::I8(1))]),
            ArrowRow::from([("num", ArrowValue::I32(1000))],),
        ];
        let schema = infer_schema(&rows).unwrap();
        assert_eq!(schema.field(0).data_type(), &DataType::Int32);
    }

    #[test]
    fn test_list_inference() {
        // List<I32>
        let rows = vec![
            ArrowRow::from([("l", ArrowValue::from(&[1i32, 2][..]))]),
            ArrowRow::from([("l", ArrowValue::from(&[3i32][..]))]),
        ];

        let schema = infer_schema(&rows).unwrap();
        if let DataType::List(inner) = schema.field(0).data_type() {
            assert_eq!(inner.data_type(), &DataType::Int32);
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_list_coercion() {
        // List<I32> + List<F64> -> List<F64>
        // Note: The second row uses manual List(Vec) construction to mix types cleanly in test
        let rows = vec![
            ArrowRow::from([("l", ArrowValue::from(&[1i32][..]))]),
            ArrowRow::from([("l", ArrowValue::List(vec![ArrowValue::F64(2.0)]))]),
        ];

        let schema = infer_schema(&rows).unwrap();
        if let DataType::List(inner) = schema.field(0).data_type() {
            assert_eq!(inner.data_type(), &DataType::Float64);
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_struct_merge() {
        // Row 1: { nest: { a: 1 } }
        // Row 2: { nest: { b: 2.0 } }
        // Result: { nest: { a: Int32(null), b: Float64(null) } }
        let rows = vec![
            ArrowRow::from([(
                "nest",
                ArrowValue::from(ArrowRow::from([("a", ArrowValue::I32(1))])),
            )]),
            ArrowRow::from([(
                "nest",
                ArrowValue::from(ArrowRow::from([("b", ArrowValue::F64(2.0))])),
            )]),
        ];

        let schema = infer_schema(&rows).unwrap();
        if let DataType::Struct(fields) = schema.field(0).data_type() {
            assert_eq!(fields.len(), 2);
            // BTreeMap ensures order a, b
            assert_eq!(fields[0].name(), "a");
            assert_eq!(fields[1].name(), "b");
        } else {
            panic!("Expected Struct");
        }
    }
}