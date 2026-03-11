// =============================================================================
// Arrow ↔ Pyro schema mapping
// =============================================================================
//
// Converts between Pyro-native schema types (`PyroType`, `PyroField`,
// `PyroSchema`) and Arrow schema types (`DataType`, `Field`, `Schema`).

use std::{borrow::Cow, sync::Arc};

use arrow::datatypes::{DataType, Field, Schema, TimeUnit};

use crate::format::value::schema::{PrimitiveDataType, PyroField, PyroSchema, PyroType};

// =============================================================================
// PyroType → arrow::datatypes::DataType
// =============================================================================

impl<'a> PyroType<'a> {
    /// Convert this [`PyroType`] to the corresponding Arrow [`DataType`].
    pub fn to_arrow(&self) -> DataType {
        match self {
            PyroType::Null => DataType::Null,
            PyroType::PrimitiveScalar(PrimitiveDataType::Bool) => DataType::Boolean,
            PyroType::PrimitiveScalar(PrimitiveDataType::I8) => DataType::Int8,
            PyroType::PrimitiveScalar(PrimitiveDataType::I16) => DataType::Int16,
            PyroType::PrimitiveScalar(PrimitiveDataType::I32) => DataType::Int32,
            PyroType::PrimitiveScalar(PrimitiveDataType::I64) => DataType::Int64,
            PyroType::PrimitiveScalar(PrimitiveDataType::U8) => DataType::UInt8,
            PyroType::PrimitiveScalar(PrimitiveDataType::U16) => DataType::UInt16,
            PyroType::PrimitiveScalar(PrimitiveDataType::U32) => DataType::UInt32,
            PyroType::PrimitiveScalar(PrimitiveDataType::U64) => DataType::UInt64,
            PyroType::PrimitiveScalar(PrimitiveDataType::F16) => DataType::Float16,
            PyroType::PrimitiveScalar(PrimitiveDataType::F32) => DataType::Float32,
            PyroType::PrimitiveScalar(PrimitiveDataType::F64) => DataType::Float64,
            PyroType::Str => DataType::Utf8,
            PyroType::Timestamp => DataType::Timestamp(TimeUnit::Nanosecond, None),

            PyroType::PrimitiveList(pdt) => {
                let inner = pdt.to_arrow();
                DataType::List(Arc::new(Field::new("item", inner, true)))
            }

            PyroType::PrimitiveFixedList(pdt, size) => {
                let inner = pdt.to_arrow();
                DataType::FixedSizeList(Arc::new(Field::new("item", inner, true)), *size as i32)
            }

            PyroType::List(inner, nullable) => {
                DataType::List(Arc::new(Field::new("item", inner.to_arrow(), *nullable)))
            }

            PyroType::Group(fields) => {
                let arrow_fields: Vec<Arc<Field>> =
                    fields.iter().map(|f| Arc::new(f.to_arrow())).collect();
                DataType::Struct(arrow_fields.into())
            }

            PyroType::Map { key, value } => DataType::Map(
                Arc::new(Field::new(
                    "entries",
                    DataType::Struct(
                        vec![
                            Arc::new(Field::new("key", key.to_arrow(), false)),
                            Arc::new(Field::new("value", value.to_arrow(), true)),
                        ]
                        .into(),
                    ),
                    false,
                )),
                false,
            ),
        }
    }
}
impl PyroType<'static> {
    /// Try to convert an Arrow [`DataType`] to a [`PyroType`].
    ///
    /// Returns `None` for Arrow types that have no PyroValue equivalent
    /// (e.g. Decimal128, RunEndEncoded, Timestamp with timezone, etc.).
    pub fn from_arrow(dt: &DataType) -> Option<Self> {
        Some(match dt {
            DataType::Null => PyroType::<'static>::Null,
            DataType::Boolean => PyroType::PrimitiveScalar(PrimitiveDataType::Bool),
            DataType::Int8 => PyroType::PrimitiveScalar(PrimitiveDataType::I8),
            DataType::Int16 => PyroType::PrimitiveScalar(PrimitiveDataType::I16),
            DataType::Int32 => PyroType::PrimitiveScalar(PrimitiveDataType::I32),
            DataType::Int64 => PyroType::PrimitiveScalar(PrimitiveDataType::I64),
            DataType::UInt8 => PyroType::PrimitiveScalar(PrimitiveDataType::U8),
            DataType::UInt16 => PyroType::PrimitiveScalar(PrimitiveDataType::U16),
            DataType::UInt32 => PyroType::PrimitiveScalar(PrimitiveDataType::U32),
            DataType::UInt64 => PyroType::PrimitiveScalar(PrimitiveDataType::U64),
            DataType::Float16 => PyroType::PrimitiveScalar(PrimitiveDataType::F16),
            DataType::Float32 => PyroType::PrimitiveScalar(PrimitiveDataType::F32),
            DataType::Float64 => PyroType::PrimitiveScalar(PrimitiveDataType::F64),
            DataType::Utf8 | DataType::LargeUtf8 => PyroType::Str,
            DataType::Interval(_) => {
                tracing::error!("Unimplemented Interval");
                return None;
            }

            // Binary types map to PrimitiveList(U8)
            DataType::Binary | DataType::LargeBinary => {
                PyroType::PrimitiveList(PrimitiveDataType::U8)
            }
            DataType::FixedSizeBinary(size) => {
                PyroType::PrimitiveFixedList(PrimitiveDataType::U8, *size as usize)
            }

            // Date/Time types that PyroValue stores as I32/I64
            DataType::Date32
            | DataType::Date64
            | DataType::Time32(_)
            | DataType::Time64(_)
            | DataType::Timestamp(_, _) => PyroType::Timestamp,
            DataType::Duration(_) => {
                tracing::error!("Unimplemented Duration");
                return None;
            }

            // Variable-size lists
            DataType::List(inner) | DataType::LargeList(inner) => {
                if let Some(pdt) = PrimitiveDataType::from_arrow(inner.data_type()) {
                    PyroType::PrimitiveList(pdt)
                } else {
                    let inner_dt = PyroType::from_arrow(inner.data_type())?;
                    PyroType::List(Box::new(inner_dt), inner.is_nullable())
                }
            }

            // Fixed-size lists
            DataType::FixedSizeList(inner, size) => {
                if let Some(pdt) = PrimitiveDataType::from_arrow(inner.data_type()) {
                    PyroType::PrimitiveFixedList(pdt, *size as usize)
                } else {
                    let inner_dt = PyroType::from_arrow(inner.data_type())?;
                    PyroType::List(Box::new(inner_dt), inner.is_nullable())
                }
            }

            // Struct → Group
            DataType::Struct(fields) => {
                let pyro_fields: Option<Cow<'static, [PyroField<'static>]>> =
                    fields.iter().map(|f| PyroField::from_arrow(f)).collect();
                PyroType::Group(pyro_fields?)
            }

            // Map
            DataType::Map(entries_field, _) => {
                if let DataType::Struct(kv_fields) = entries_field.data_type() {
                    if kv_fields.len() == 2 {
                        let key_dt = PyroType::from_arrow(kv_fields[0].data_type())?;
                        let val_dt = PyroType::from_arrow(kv_fields[1].data_type())?;
                        PyroType::Map {
                            key: Box::new(key_dt),
                            value: Box::new(val_dt),
                        }
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            }

            // Dictionary → unwrap to value type
            DataType::Dictionary(_, value_type) => PyroType::from_arrow(value_type)?,

            // Unsupported Arrow types
            _ => return None,
        })
    }
}

// =============================================================================
// PrimitiveDataType → arrow::datatypes::DataType
// =============================================================================

impl PrimitiveDataType {
    /// Convert to the corresponding Arrow scalar [`DataType`].
    pub fn to_arrow(&self) -> DataType {
        match self {
            PrimitiveDataType::Bool => DataType::Boolean,
            PrimitiveDataType::U8 => DataType::UInt8,
            PrimitiveDataType::U16 => DataType::UInt16,
            PrimitiveDataType::U32 => DataType::UInt32,
            PrimitiveDataType::U64 => DataType::UInt64,
            PrimitiveDataType::I8 => DataType::Int8,
            PrimitiveDataType::I16 => DataType::Int16,
            PrimitiveDataType::I32 => DataType::Int32,
            PrimitiveDataType::I64 => DataType::Int64,
            PrimitiveDataType::F16 => DataType::Float16,
            PrimitiveDataType::F32 => DataType::Float32,
            PrimitiveDataType::F64 => DataType::Float64,
        }
    }

    /// Try to map an Arrow [`DataType`] to a [`PrimitiveDataType`].
    /// Only succeeds for the primitive scalar types that `PrimitiveValueList` supports.
    pub fn from_arrow(dt: &DataType) -> Option<Self> {
        Some(match dt {
            DataType::Boolean => PrimitiveDataType::Bool,
            DataType::UInt8 => PrimitiveDataType::U8,
            DataType::UInt16 => PrimitiveDataType::U16,
            DataType::UInt32 => PrimitiveDataType::U32,
            DataType::UInt64 => PrimitiveDataType::U64,
            DataType::Int8 => PrimitiveDataType::I8,
            DataType::Int16 => PrimitiveDataType::I16,
            DataType::Int32 => PrimitiveDataType::I32,
            DataType::Int64 => PrimitiveDataType::I64,
            DataType::Float16 => PrimitiveDataType::F16,
            DataType::Float32 => PrimitiveDataType::F32,
            DataType::Float64 => PrimitiveDataType::F64,
            _ => return None,
        })
    }
}

// =============================================================================
// PyroField ↔ arrow::datatypes::Field
// =============================================================================

impl<'a> PyroField<'a> {
    /// Convert to an Arrow [`Field`].
    pub fn to_arrow(&self) -> Field {
        Field::new(self.name(), self.data_type().to_arrow(), self.is_nullable())
    }
}

impl PyroField<'static> {
    /// Try to convert an Arrow [`Field`] to a [`PyroField`].
    pub fn from_arrow(field: &Field) -> Option<Self> {
        let dt = PyroType::from_arrow(field.data_type())?;
        Some(PyroField::new(
            field.name().to_string(),
            dt,
            field.is_nullable(),
        ))
    }
}

impl<'a> From<&PyroField<'a>> for Field {
    fn from(f: &PyroField<'a>) -> Self {
        f.to_arrow()
    }
}

impl<'a> From<PyroField<'a>> for Field {
    fn from(f: PyroField<'a>) -> Self {
        f.to_arrow()
    }
}

// =============================================================================
// PyroSchema ↔ arrow::datatypes::Schema
// =============================================================================

impl<'a> PyroSchema<'a> {
    /// Convert to an Arrow [`Schema`].
    pub fn to_arrow(&self) -> Schema {
        let fields: Vec<Field> = self.fields().iter().map(|f| f.to_arrow()).collect();
        Schema::new(fields)
    }

    /// Convert to a reference-counted Arrow [`Schema`].
    pub fn to_arrow_arc(&self) -> Arc<Schema> {
        Arc::new(self.to_arrow())
    }
}

impl PyroSchema<'static> {
    /// Try to convert an Arrow [`Schema`] to a [`PyroSchema`].
    ///
    /// Returns `None` if any field contains an Arrow type that cannot be
    /// represented as a [`PyroType`].
    pub fn from_arrow(schema: &Schema) -> Option<Self> {
        let fields: Option<Vec<PyroField>> = schema
            .fields()
            .iter()
            .map(|f| PyroField::from_arrow(f.as_ref()))
            .collect();
        Some(PyroSchema::new(fields?))
    }
}

impl<'a> From<&PyroSchema<'a>> for Schema {
    fn from(s: &PyroSchema) -> Self {
        s.to_arrow()
    }
}

impl<'a> From<PyroSchema<'a>> for Schema {
    fn from(s: PyroSchema) -> Self {
        s.to_arrow()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_roundtrip() {
        let types: Vec<PyroType<'static>> = vec![
            PyroType::Null,
            PyroType::PrimitiveScalar(PrimitiveDataType::Bool),
            PyroType::PrimitiveScalar(PrimitiveDataType::I8),
            PyroType::PrimitiveScalar(PrimitiveDataType::I16),
            PyroType::PrimitiveScalar(PrimitiveDataType::I32),
            PyroType::PrimitiveScalar(PrimitiveDataType::I64),
            PyroType::PrimitiveScalar(PrimitiveDataType::U8),
            PyroType::PrimitiveScalar(PrimitiveDataType::U16),
            PyroType::PrimitiveScalar(PrimitiveDataType::U32),
            PyroType::PrimitiveScalar(PrimitiveDataType::U64),
            PyroType::PrimitiveScalar(PrimitiveDataType::F16),
            PyroType::PrimitiveScalar(PrimitiveDataType::F32),
            PyroType::PrimitiveScalar(PrimitiveDataType::F64),
            PyroType::Str,
            PyroType::Timestamp,
        ];

        for dt in types {
            let arrow_dt = dt.to_arrow();
            let roundtripped = PyroType::from_arrow(&arrow_dt).unwrap();
            assert_eq!(dt, roundtripped, "roundtrip failed for {dt:?}");
        }
    }

    #[test]
    fn test_primitive_list_roundtrip() {
        let prim_types = vec![
            PrimitiveDataType::Bool,
            PrimitiveDataType::U8,
            PrimitiveDataType::U16,
            PrimitiveDataType::U32,
            PrimitiveDataType::U64,
            PrimitiveDataType::I8,
            PrimitiveDataType::I16,
            PrimitiveDataType::I32,
            PrimitiveDataType::I64,
            PrimitiveDataType::F16,
            PrimitiveDataType::F32,
            PrimitiveDataType::F64,
        ];

        for pdt in prim_types {
            let dt = PyroType::PrimitiveList(pdt);
            let arrow_dt = dt.to_arrow();
            let roundtripped = PyroType::from_arrow(&arrow_dt).unwrap();
            assert_eq!(
                dt, roundtripped,
                "PrimitiveList roundtrip failed for {pdt:?}"
            );
        }
    }

    #[test]
    fn test_primitive_fixed_list_roundtrip() {
        let dt = PyroType::PrimitiveFixedList(PrimitiveDataType::F32, 128);
        let arrow_dt = dt.to_arrow();

        if let DataType::FixedSizeList(inner, size) = &arrow_dt {
            assert_eq!(inner.data_type(), &DataType::Float32);
            assert_eq!(*size, 128);
        } else {
            panic!("Expected Arrow FixedSizeList, got {arrow_dt:?}");
        }

        let back = PyroType::from_arrow(&arrow_dt).unwrap();
        assert_eq!(back, dt);
    }

    #[test]
    fn test_fixed_size_binary_to_fixed_list() {
        let arrow_dt = DataType::FixedSizeBinary(16);
        let pyro_dt = PyroType::from_arrow(&arrow_dt).unwrap();
        assert_eq!(
            pyro_dt,
            PyroType::PrimitiveFixedList(PrimitiveDataType::U8, 16)
        );
    }

    #[test]
    fn test_list_with_nullable_roundtrip() {
        let dt = PyroType::List(Box::new(PyroType::Str), true);
        let arrow_dt = dt.to_arrow();

        if let DataType::List(inner) = &arrow_dt {
            assert_eq!(inner.data_type(), &DataType::Utf8);
            assert!(inner.is_nullable());
        } else {
            panic!("Expected Arrow List, got {arrow_dt:?}");
        }

        let back = PyroType::from_arrow(&arrow_dt).unwrap();
        assert_eq!(back, dt);
    }

    #[test]
    fn test_list_non_nullable_roundtrip() {
        let dt = PyroType::List(Box::new(PyroType::Str), false);
        let arrow_dt = dt.to_arrow();

        if let DataType::List(inner) = &arrow_dt {
            assert!(!inner.is_nullable());
        } else {
            panic!("Expected Arrow List");
        }

        let back = PyroType::from_arrow(&arrow_dt).unwrap();
        assert_eq!(back, dt);
    }

    #[test]
    fn test_list_non_nullable_roundtrip_primitive() {
        let dt = PyroType::List(
            Box::new(PyroType::PrimitiveScalar(PrimitiveDataType::I8)),
            false,
        );
        let arrow_dt = dt.to_arrow();

        if let DataType::List(inner) = &arrow_dt {
            assert!(!inner.is_nullable());
        } else {
            panic!("Expected Arrow List");
        }

        let back = PyroType::from_arrow(&arrow_dt).unwrap();
        // We get back what this will be retrieved as
        assert_eq!(back, PyroType::PrimitiveList(PrimitiveDataType::I8));
    }

    #[test]
    fn test_group_roundtrip() {
        let dt = PyroType::Group(Cow::Owned(vec![
            PyroField::new("name", PyroType::Str, false),
            PyroField::new(
                "age",
                PyroType::PrimitiveScalar(PrimitiveDataType::I32),
                true,
            ),
        ]));

        let arrow_dt = dt.to_arrow();
        if let DataType::Struct(fields) = &arrow_dt {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name(), "name");
            assert_eq!(fields[0].data_type(), &DataType::Utf8);
            assert!(!fields[0].is_nullable());
            assert_eq!(fields[1].name(), "age");
            assert_eq!(fields[1].data_type(), &DataType::Int32);
            assert!(fields[1].is_nullable());
        } else {
            panic!("Expected Arrow Struct, got {arrow_dt:?}");
        }

        let back = PyroType::from_arrow(&arrow_dt).unwrap();
        assert_eq!(back, dt);
    }

    #[test]
    fn test_map_roundtrip() {
        let dt = PyroType::Map {
            key: Box::new(PyroType::Str),
            value: Box::new(PyroType::PrimitiveScalar(PrimitiveDataType::I64)),
        };

        let arrow_dt = dt.to_arrow();
        assert!(matches!(arrow_dt, DataType::Map(_, false)));

        let back = PyroType::from_arrow(&arrow_dt).unwrap();
        assert_eq!(back, dt);
    }

    #[test]
    fn test_schema_roundtrip() {
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
            PyroField::new(
                "embedding",
                PyroType::PrimitiveFixedList(PrimitiveDataType::F32, 128),
                false,
            ),
            PyroField::new("tags", PyroType::List(Box::new(PyroType::Str), true), true),
        ]);

        let arrow_schema = schema.to_arrow();
        assert_eq!(arrow_schema.fields().len(), 5);
        assert_eq!(arrow_schema.field(0).name(), "id");
        assert_eq!(arrow_schema.field(0).data_type(), &DataType::Int64);
        assert!(!arrow_schema.field(0).is_nullable());

        // FixedSizeList check
        if let DataType::FixedSizeList(inner, size) = arrow_schema.field(3).data_type() {
            assert_eq!(inner.data_type(), &DataType::Float32);
            assert_eq!(*size, 128);
        } else {
            panic!("Expected FixedSizeList for embedding");
        }

        let back = PyroSchema::from_arrow(&arrow_schema).unwrap();
        assert_eq!(schema, back);
    }

    #[test]
    fn test_schema_to_arrow_arc() {
        let schema = PyroSchema::new(vec![PyroField::new(
            "x",
            PyroType::PrimitiveScalar(PrimitiveDataType::F64),
            false,
        )]);
        let arc: Arc<Schema> = schema.to_arrow_arc();
        assert_eq!(arc.fields().len(), 1);
    }

    #[test]
    fn test_unsupported_arrow_type_returns_none() {
        let dt = DataType::Decimal128(10, 2);
        assert!(PyroType::from_arrow(&dt).is_none());
    }

    #[test]
    fn test_dictionary_unwrap() {
        let dt = DataType::Dictionary(Box::new(DataType::Int32), Box::new(DataType::Utf8));
        let pyro_dt = PyroType::from_arrow(&dt).unwrap();
        assert_eq!(pyro_dt, PyroType::Str);
    }
}
