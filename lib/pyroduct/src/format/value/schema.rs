// =============================================================================
// Pyro-native type system
// =============================================================================
//
// A lightweight schema representation optimized for PyroValue.
// Unlike Arrow's DataType (which has ~40 variants for timestamps, decimals,
// run-end-encoded, etc.), PyroType mirrors *exactly* the variants that
// PyroValue can represent, making match arms exhaustive and tiny.
//
// Conversion to/from `arrow::datatypes::DataType` lives in `value::arrow::schema`.

use std::borrow::Cow;
use std::collections::{BTreeMap, HashSet};

use super::{PrimitiveValueList, PyroRow, PyroValue};
use pyro_spec::{PrimitiveDataType, PyroField, PyroSchema, PyroType, coerce_pyro_types};
use thiserror::Error;

// =============================================================================
// PyroType inference from PyroValue
// =============================================================================

impl<'a> PyroValue<'a> {
    /// Infer the [`PyroType`] from a single [`PyroValue`].
    ///
    /// NOTE: This returns a `PyroType<'a>` where possible, but if field names
    /// need to be extracted from `Group` keys, they are cloned to Owned strings
    /// to avoid lifetime safety issues.
    pub fn data_type(&self) -> PyroType<'a> {
        match self {
            PyroValue::Null => PyroType::Null,
            PyroValue::Bool(_) => PyroType::PrimitiveScalar(PrimitiveDataType::Bool),
            PyroValue::I8(_) => PyroType::PrimitiveScalar(PrimitiveDataType::I8),
            PyroValue::I16(_) => PyroType::PrimitiveScalar(PrimitiveDataType::I16),
            PyroValue::I32(_) => PyroType::PrimitiveScalar(PrimitiveDataType::I32),
            PyroValue::I64(_) => PyroType::PrimitiveScalar(PrimitiveDataType::I64),
            PyroValue::U8(_) => PyroType::PrimitiveScalar(PrimitiveDataType::U8),
            PyroValue::U16(_) => PyroType::PrimitiveScalar(PrimitiveDataType::U16),
            PyroValue::U32(_) => PyroType::PrimitiveScalar(PrimitiveDataType::U32),
            PyroValue::U64(_) => PyroType::PrimitiveScalar(PrimitiveDataType::U64),
            PyroValue::F16(_) => PyroType::PrimitiveScalar(PrimitiveDataType::F16),
            PyroValue::F32(_) => PyroType::PrimitiveScalar(PrimitiveDataType::F32),
            PyroValue::F64(_) => PyroType::PrimitiveScalar(PrimitiveDataType::F64),
            PyroValue::Str(_) => PyroType::Str,
            PyroValue::Timestamp { .. } => PyroType::Timestamp,

            PyroValue::PrimitiveList(pl) => {
                let pdt = pl.data_type();
                PyroType::PrimitiveList(pdt)
            }

            PyroValue::List(items) => {
                let mut has_null = false;
                let mut inner = PyroType::Null;

                for item in items {
                    if matches!(item, PyroValue::Null) {
                        has_null = true;
                    } else {
                        let item_dt = item.data_type();
                        if inner == PyroType::Null {
                            inner = item_dt;
                        } else if inner != item_dt {
                            // Try coercion
                            if let Some(coerced) = coerce_pyro_types(&inner, &item_dt) {
                                inner = coerced;
                            }
                        }
                    }
                }
                PyroType::List(Box::new(inner), has_null)
            }

            PyroValue::Group(row) => {
                let fields: Vec<PyroField<'a>> = row
                    .iter()
                    .map(|(k, v)| {
                        // We strictly own the field name here to avoid unsafe lifetime juggling.
                        // Since schema metadata is small, this clone is acceptable.
                        PyroField::new(Cow::Owned(k.to_string()), v.data_type(), true)
                    })
                    .collect();
                PyroType::Group(Cow::Owned(fields))
            }

            PyroValue::MapInternal(pairs) => {
                let mut key_dt = PyroType::Null;
                let mut val_dt = PyroType::Null;

                for (k, v) in pairs {
                    let k_type = k.data_type();
                    let v_type = v.data_type();

                    if key_dt == PyroType::Null {
                        key_dt = k_type;
                    } else if let Some(coerced) = coerce_pyro_types(&key_dt, &k_type) {
                        key_dt = coerced;
                    }

                    if val_dt == PyroType::Null {
                        val_dt = v_type;
                    } else if let Some(coerced) = coerce_pyro_types(&val_dt, &v_type) {
                        val_dt = coerced;
                    }
                }

                if key_dt == PyroType::Null {
                    key_dt = PyroType::Str; // Default key type
                }

                PyroType::Map {
                    key: Box::new(key_dt),
                    value: Box::new(val_dt),
                }
            }
        }
    }
}

impl<'a> PrimitiveValueList<'a> {
    /// Extract the [`PrimitiveDataType`] from a [`PrimitiveValueList`].
    pub fn data_type(&self) -> PrimitiveDataType {
        match self {
            PrimitiveValueList::Bool(_) => PrimitiveDataType::Bool,
            PrimitiveValueList::U8(_) => PrimitiveDataType::U8,
            PrimitiveValueList::U16(_) => PrimitiveDataType::U16,
            PrimitiveValueList::U32(_) => PrimitiveDataType::U32,
            PrimitiveValueList::U64(_) => PrimitiveDataType::U64,
            PrimitiveValueList::I8(_) => PrimitiveDataType::I8,
            PrimitiveValueList::I16(_) => PrimitiveDataType::I16,
            PrimitiveValueList::I32(_) => PrimitiveDataType::I32,
            PrimitiveValueList::I64(_) => PrimitiveDataType::I64,
            PrimitiveValueList::F16(_) => PrimitiveDataType::F16,
            PrimitiveValueList::F32(_) => PrimitiveDataType::F32,
            PrimitiveValueList::F64(_) => PrimitiveDataType::F64,
        }
    }
}

// =============================================================================
// PyroSchema inference (from rows)
// =============================================================================

impl<'a> PyroRow<'a> {
    /// Build a trusted schema from a single row (no coercion, nullable = true).
    pub fn schema(&self) -> Result<PyroSchema<'a>, ValueSchemaInferenceError<'a>> {
        if self.is_empty() {
            return Err(ValueSchemaInferenceError::EmptyRow);
        }
        let fields = self
            .iter()
            .map(|(k, v)| {
                // Ensure safe lifetime handling by owning the string
                PyroField::new(Cow::Owned(k.to_string()), v.data_type(), true)
            })
            .collect();

        Ok(PyroSchema {
            documentation: None,
            fields,
        })
    }

    /// Infer a schema from multiple rows, with coercion and nullability tracking.
    pub fn infer_schema(
        rows: &[PyroRow<'a>],
    ) -> Result<PyroSchema<'a>, ValueSchemaInferenceError<'a>> {
        // We use String (owned) for keys during accumulation to simplify lifetime management
        let mut field_map: BTreeMap<String, PyroFieldAccumulator<'a>> = BTreeMap::new();

        for (i, row) in rows.iter().enumerate() {
            let mut row_keys: HashSet<&str> = HashSet::new();

            for (key, value) in row.iter() {
                row_keys.insert(key);
                let is_new = !field_map.contains_key(key);

                let accum = field_map
                    .entry(key.to_string())
                    .or_insert_with(PyroFieldAccumulator::new);

                if is_new && i > 0 {
                    accum.nullable = true;
                }

                accum.observe(key, value)?;
            }

            for (key, accum) in field_map.iter_mut() {
                if !row_keys.contains(key.as_str()) {
                    accum.nullable = true;
                }
            }
        }

        let mut fields: Vec<PyroField<'a>> = Vec::with_capacity(field_map.len());
        for (name, accum) in field_map {
            fields.push(accum.build(name)?);
        }

        Ok(PyroSchema {
            documentation: None,
            fields: Cow::Owned(fields),
        })
    }
}

// =============================================================================
// Errors
// =============================================================================

#[derive(Debug, Error, PartialEq, Clone)]
pub enum ValueSchemaInferenceError<'a> {
    #[error("Incompatible types for field '{0}': {1:?} vs {2:?}")]
    IncompatibleTypes(String, PyroType<'a>, PyroType<'a>),
    #[error("Row is empty, cannot infer schema")]
    EmptyRow,
}

// =============================================================================
// Internal: accumulator + coercion
// =============================================================================

struct PyroFieldAccumulator<'a> {
    data_type: Option<PyroType<'a>>,
    nullable: bool,
}

impl<'a> PyroFieldAccumulator<'a> {
    fn new() -> Self {
        Self {
            data_type: None,
            nullable: false,
        }
    }

    fn observe(
        &mut self,
        field_name: &str,
        value: &PyroValue<'a>,
    ) -> Result<(), ValueSchemaInferenceError<'a>> {
        if matches!(value, PyroValue::Null) {
            self.nullable = true;
            return Ok(());
        }

        let observed = value.data_type();

        match &self.data_type {
            None => {
                self.data_type = Some(observed);
            }
            Some(current) if current == &observed => {}
            Some(current) => {
                let coerced = coerce_pyro_types(current, &observed).ok_or_else(|| {
                    ValueSchemaInferenceError::IncompatibleTypes(
                        field_name.to_string(),
                        current.clone(),
                        observed.clone(),
                    )
                })?;
                self.data_type = Some(coerced);
            }
        }
        Ok(())
    }

    fn build(self, name: String) -> Result<PyroField<'a>, ValueSchemaInferenceError<'a>> {
        let dt = self.data_type.unwrap_or(PyroType::Null);
        Ok(PyroField::new(Cow::Owned(name), dt, self.nullable))
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::PyroRow;
    use super::*;

    // ---- Inference from single values ----

    #[test]
    fn test_data_type_from_value_scalars() {
        assert_eq!(PyroValue::Null.data_type(), PyroType::Null);
        assert_eq!(
            PyroValue::I32(42).data_type(),
            PyroType::PrimitiveScalar(PrimitiveDataType::I32)
        );
        assert_eq!(
            PyroValue::F64(1.0).data_type(),
            PyroType::PrimitiveScalar(PrimitiveDataType::F64)
        );
        assert_eq!(PyroValue::from("hello").data_type(), PyroType::Str);
    }

    #[test]
    fn test_data_type_from_value_list_nullable() {
        let list = PyroValue::List(vec![PyroValue::I32(1), PyroValue::Null, PyroValue::I32(3)]);
        assert_eq!(
            list.data_type(),
            PyroType::List(
                Box::new(PyroType::PrimitiveScalar(PrimitiveDataType::I32)),
                true
            )
        );
    }

    #[test]
    fn test_data_type_from_value_list_not_nullable() {
        let list = PyroValue::List(vec![PyroValue::I32(1), PyroValue::I32(2)]);
        assert_eq!(
            list.data_type(),
            PyroType::List(
                Box::new(PyroType::PrimitiveScalar(PrimitiveDataType::I32)),
                false
            )
        );
    }

    #[test]
    fn test_data_type_from_value_map_coercion() {
        // Map with I32 and Float values should widen to F64
        let map_val = PyroValue::MapInternal(vec![
            (PyroValue::from("a"), PyroValue::I32(10)),
            (PyroValue::from("b"), PyroValue::F64(20.5)),
        ]);
        let schema = map_val.data_type();

        if let PyroType::Map { key, value } = schema {
            assert_eq!(*key, PyroType::Str);
            assert_eq!(*value, PyroType::PrimitiveScalar(PrimitiveDataType::F64));
        } else {
            panic!("Expected Map type");
        }
    }

    // ---- Schema inference ----

    #[test]
    fn test_trusted_schema() {
        let row = PyroRow::from([("a", PyroValue::I32(10)), ("b", PyroValue::from("hi"))]);
        let schema = row.schema().unwrap();
        assert_eq!(schema.num_fields(), 2);
        assert_eq!(schema.field(0).name(), "a");
        assert_eq!(
            *schema.field(0).data_type(),
            PyroType::PrimitiveScalar(PrimitiveDataType::I32)
        );
        assert_eq!(schema.field(1).name(), "b");
        assert_eq!(*schema.field(1).data_type(), PyroType::Str);
    }

    #[test]
    fn test_infer_coercion() {
        let rows = vec![
            PyroRow::from([("mix", PyroValue::I32(10))]),
            PyroRow::from([("mix", PyroValue::F64(20.5))]),
        ];
        let schema = PyroRow::infer_schema(&rows).unwrap();
        assert_eq!(
            *schema.field(0).data_type(),
            PyroType::PrimitiveScalar(PrimitiveDataType::F64)
        );
    }

    #[test]
    fn test_infer_nullable() {
        let rows = vec![
            PyroRow::from([("a", PyroValue::I32(10))]),
            PyroRow::from([("a", PyroValue::Null)]),
        ];
        let schema = PyroRow::infer_schema(&rows).unwrap();
        assert_eq!(
            *schema.field(0).data_type(),
            PyroType::PrimitiveScalar(PrimitiveDataType::I32)
        );
        assert!(schema.field(0).is_nullable());
    }

    #[test]
    fn test_infer_sparse() {
        let rows = vec![
            PyroRow::from([("a", PyroValue::I32(10))]),
            PyroRow::from([("b", PyroValue::F64(1.0))]),
        ];
        let schema = PyroRow::infer_schema(&rows).unwrap();
        assert_eq!(schema.num_fields(), 2);
        assert!(schema.field_with_name("a").unwrap().is_nullable());
        assert!(schema.field_with_name("b").unwrap().is_nullable());
    }

    #[test]
    fn test_infer_int_widening() {
        let rows = vec![
            PyroRow::from([("num", PyroValue::I8(1))]),
            PyroRow::from([("num", PyroValue::I32(1000))]),
        ];
        let schema = PyroRow::infer_schema(&rows).unwrap();
        assert_eq!(
            *schema.field(0).data_type(),
            PyroType::PrimitiveScalar(PrimitiveDataType::I32)
        );
    }

    #[test]
    fn test_infer_primitive_list() {
        let rows = vec![
            PyroRow::from([("l", PyroValue::from(&[1i32, 2][..]))]),
            PyroRow::from([("l", PyroValue::from(&[3i32][..]))]),
        ];
        let schema = PyroRow::infer_schema(&rows).unwrap();
        assert_eq!(
            *schema.field(0).data_type(),
            PyroType::PrimitiveList(PrimitiveDataType::I32)
        );
    }

    #[test]
    fn test_infer_incompatible() {
        let rows = vec![
            PyroRow::from([("x", PyroValue::I32(10))]),
            PyroRow::from([("x", PyroValue::from("text"))]),
        ];
        let err = PyroRow::infer_schema(&rows).unwrap_err();
        assert!(matches!(
            err,
            ValueSchemaInferenceError::IncompatibleTypes(..)
        ));
    }

    // ---- Coercion edge cases ----

    #[test]
    fn test_coerce_primitive_fixed_list_same_size() {
        let a = PyroType::PrimitiveFixedList(PrimitiveDataType::I8, 4);
        let b = PyroType::PrimitiveFixedList(PrimitiveDataType::I32, 4);
        let result = coerce_pyro_types(&a, &b);
        assert_eq!(
            result,
            Some(PyroType::PrimitiveFixedList(PrimitiveDataType::I32, 4))
        );
    }

    #[test]
    fn test_coerce_primitive_fixed_list_different_size() {
        let a = PyroType::PrimitiveFixedList(PrimitiveDataType::I32, 4);
        let b = PyroType::PrimitiveFixedList(PrimitiveDataType::I32, 8);
        let result = coerce_pyro_types(&a, &b);
        assert_eq!(
            result,
            Some(PyroType::PrimitiveList(PrimitiveDataType::I32))
        );
    }

    #[test]
    fn test_coerce_fixed_with_variable_list() {
        let a = PyroType::PrimitiveFixedList(PrimitiveDataType::I32, 4);
        let b = PyroType::PrimitiveList(PrimitiveDataType::I32);
        let result = coerce_pyro_types(&a, &b);
        assert_eq!(
            result,
            Some(PyroType::PrimitiveList(PrimitiveDataType::I32))
        );
    }

    #[test]
    fn test_coerce_lists_merge_nullability() {
        let a = PyroType::List(
            Box::new(PyroType::PrimitiveScalar(PrimitiveDataType::I32)),
            false,
        );
        let b = PyroType::List(
            Box::new(PyroType::PrimitiveScalar(PrimitiveDataType::I32)),
            true,
        );
        let result = coerce_pyro_types(&a, &b);
        assert_eq!(
            result,
            Some(PyroType::List(
                Box::new(PyroType::PrimitiveScalar(PrimitiveDataType::I32)),
                true
            ))
        );
    }

    // ---- JSON serialization ----

    #[test]
    fn test_schema_json_roundtrip() {
        let schema = PyroSchema::new(vec![
            PyroField::new(
                "id",
                PyroType::PrimitiveScalar(PrimitiveDataType::I64),
                false,
            ),
            PyroField::new("name", PyroType::Str, true),
            PyroField::new(
                "scores",
                PyroType::PrimitiveList(PrimitiveDataType::F32),
                true,
            ),
            PyroField::new("tags", PyroType::List(Box::new(PyroType::Str), false), true),
            PyroField::new(
                "embedding",
                PyroType::PrimitiveFixedList(PrimitiveDataType::F32, 128),
                false,
            ),
        ]);

        let json = serde_json::to_string_pretty(&schema).unwrap();
        let deserialized: PyroSchema = serde_json::from_str(&json).unwrap();
        assert_eq!(schema, deserialized);
    }

    #[test]
    fn test_data_type_json_roundtrip() {
        let types = vec![
            PyroType::Null,
            PyroType::PrimitiveScalar(PrimitiveDataType::Bool),
            PyroType::PrimitiveScalar(PrimitiveDataType::I32),
            PyroType::PrimitiveScalar(PrimitiveDataType::F64),
            PyroType::Str,
            PyroType::Timestamp,
            PyroType::PrimitiveList(PrimitiveDataType::U8),
            PyroType::PrimitiveFixedList(PrimitiveDataType::F32, 16),
            PyroType::List(Box::new(PyroType::Str), true),
            PyroType::Group(Cow::Owned(vec![PyroField::new(
                "x",
                PyroType::PrimitiveScalar(PrimitiveDataType::I32),
                false,
            )])),
            PyroType::Map {
                key: Box::new(PyroType::Str),
                value: Box::new(PyroType::PrimitiveScalar(PrimitiveDataType::I64)),
            },
        ];

        for dt in types {
            let json = serde_json::to_string(&dt).unwrap();
            let back: PyroType = serde_json::from_str(&json).unwrap();
            assert_eq!(dt, back, "JSON roundtrip failed for {dt:?}");
        }
    }

    #[test]
    fn test_field_json_roundtrip() {
        let field = PyroField::new(
            "age",
            PyroType::PrimitiveScalar(PrimitiveDataType::U32),
            true,
        );
        let json = serde_json::to_string(&field).unwrap();
        let back: PyroField = serde_json::from_str(&json).unwrap();
        assert_eq!(field, back);
    }
}
