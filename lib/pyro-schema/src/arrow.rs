// =============================================================================
// Arrow ↔ Pyro schema mapping
// =============================================================================

use std::borrow::Cow;
use std::sync::Arc;

use arrow_schema::{DataType, Field, Schema, TimeUnit};

use crate::bow::Bow;
use crate::types::{ChildField, FieldNames, PrimitiveDataType, PyroField, PyroSchema, PyroType};

// =============================================================================
// PyroType → arrow::datatypes::DataType
// =============================================================================

impl PyroType {
    /// Convert this [`PyroType`] to the corresponding Arrow [`DataType`].
    ///
    /// For `Group`/`OptionGroup`, child names are not available on `PyroType`
    /// alone, so struct fields get placeholder names "0", "1", etc.
    /// Use [`PyroField::to_arrow`] or [`ChildField::to_arrow`] for named fields.
    pub fn to_arrow(&self) -> DataType {
        match self {
            PyroType::Unknown => DataType::Null,
            PyroType::PrimitiveScalar(p) | PyroType::OptionPrimitiveScalar(p) => p.to_arrow(),
            PyroType::Str => DataType::Utf8,
            PyroType::Bytes => DataType::Binary,
            PyroType::PrimitiveList(pdt) => {
                DataType::List(Arc::new(Field::new("item", pdt.to_arrow(), true)))
            }

            PyroType::PrimitiveFixedList(pdt, size)
            | PyroType::OptionPrimitiveFixedList(pdt, size) => DataType::FixedSizeList(
                Arc::new(Field::new("item", pdt.to_arrow(), true)),
                *size as i32,
            ),

            PyroType::List(inner) => DataType::List(Arc::new(Field::new(
                "item",
                inner.to_arrow(),
                inner.is_nullable(),
            ))),

            PyroType::Group(children) | PyroType::OptionGroup(children) => {
                let arrow_fields: Vec<Arc<Field>> = children
                    .iter()
                    .enumerate()
                    .map(|(i, ct)| {
                        Arc::new(Field::new(i.to_string(), ct.to_arrow(), ct.is_nullable()))
                    })
                    .collect();
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

    /// Try to convert an Arrow [`DataType`] to a [`PyroType`].
    ///
    /// Returns the **non-nullable** variant. Callers should check
    /// `Field::is_nullable()` and call `to_nullable()` if needed.
    pub fn from_arrow(dt: &DataType) -> Option<Self> {
        Some(match dt {
            DataType::Null => PyroType::Unknown,
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

            DataType::Binary | DataType::LargeBinary => {
                PyroType::PrimitiveList(PrimitiveDataType::U8)
            }
            DataType::FixedSizeBinary(size) => {
                PyroType::PrimitiveFixedList(PrimitiveDataType::U8, *size as usize)
            }

            DataType::Date32
            | DataType::Date64
            | DataType::Time32(_)
            | DataType::Time64(_)
            | DataType::Timestamp(_, _) => PyroType::PrimitiveScalar(PrimitiveDataType::Timestamp),
            DataType::Duration(_) => {
                tracing::error!("Unimplemented Duration");
                return None;
            }

            DataType::List(inner) | DataType::LargeList(inner) => {
                if let Some(pdt) = PrimitiveDataType::from_arrow(inner.data_type()) {
                    PyroType::PrimitiveList(pdt)
                } else {
                    let inner_dt = PyroType::from_arrow(inner.data_type())?;
                    PyroType::List(Bow::new(inner_dt))
                }
            }

            DataType::FixedSizeList(inner, size) => {
                if let Some(pdt) = PrimitiveDataType::from_arrow(inner.data_type()) {
                    PyroType::PrimitiveFixedList(pdt, *size as usize)
                } else {
                    let inner_dt = PyroType::from_arrow(inner.data_type())?;
                    PyroType::List(Bow::new(inner_dt))
                }
            }

            DataType::Struct(fields) => {
                let child_types: Option<Vec<PyroType>> = fields
                    .iter()
                    .map(|f| {
                        let base = PyroType::from_arrow(f.data_type())?;
                        Some(if f.is_nullable() {
                            base.to_nullable()
                        } else {
                            base
                        })
                    })
                    .collect();
                PyroType::Group(child_types?.into())
            }

            DataType::Map(entries_field, _) => {
                if let DataType::Struct(kv_fields) = entries_field.data_type() {
                    if kv_fields.len() == 2 {
                        let key_dt = PyroType::from_arrow(kv_fields[0].data_type())?;
                        let val_dt = PyroType::from_arrow(kv_fields[1].data_type())?;
                        PyroType::Map {
                            key: Bow::new(key_dt),
                            value: Bow::new(val_dt),
                        }
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            }

            DataType::Dictionary(_, value_type) => PyroType::from_arrow(value_type)?,

            _ => return None,
        })
    }
}

// =============================================================================
// PrimitiveDataType ↔ Arrow
// =============================================================================

impl PrimitiveDataType {
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
            PrimitiveDataType::Timestamp => DataType::Timestamp(TimeUnit::Nanosecond, None),
        }
    }

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
            DataType::Date32
            | DataType::Date64
            | DataType::Time32(_)
            | DataType::Time64(_)
            | DataType::Timestamp(_, _) => PrimitiveDataType::Timestamp,
            _ => return None,
        })
    }
}

// =============================================================================
// PyroField ↔ arrow::datatypes::Field
// =============================================================================

impl<'a> PyroField<'a> {
    pub fn to_arrow(&self) -> Field {
        let dt = type_to_arrow_with_names(self.datatype(), self.child_names());
        Field::new(self.name(), dt, self.datatype.is_nullable())
    }

    pub fn from_arrow(field: &Field) -> Option<PyroField<'static>> {
        let (dt, child_names) = type_from_arrow_with_names(field.data_type())?;
        let dt = if field.is_nullable() {
            dt.to_nullable()
        } else {
            dt
        };
        if child_names.is_empty() {
            Some(PyroField::new(field.name().to_string(), dt))
        } else {
            Some(PyroField::with_children(
                field.name().to_string(),
                child_names,
                dt,
            ))
        }
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
// ChildField → arrow::datatypes::Field
// =============================================================================

impl<'f> ChildField<'f> {
    pub fn to_arrow(&self) -> Field {
        let dt = child_field_type_to_arrow(self);
        Field::new(self.name(), dt, self.datatype.is_nullable())
    }
}

// =============================================================================
// PyroSchema ↔ arrow::datatypes::Schema
// =============================================================================

impl<'a> PyroSchema<'a> {
    pub fn to_arrow(&self) -> Schema {
        let fields: Vec<Field> = self.fields().map(|cf| cf.to_arrow()).collect();
        Schema::new(fields)
    }

    pub fn to_arrow_arc(&self) -> Arc<Schema> {
        Arc::new(self.to_arrow())
    }

    pub fn from_arrow(schema: &Schema) -> Option<PyroSchema<'static>> {
        let fields: Option<Vec<PyroField<'static>>> = schema
            .fields()
            .iter()
            .map(|f| PyroField::from_arrow(f.as_ref()))
            .collect();
        Some(PyroSchema::new(fields?))
    }
}

impl<'a> From<&PyroSchema<'a>> for Schema {
    fn from(s: &PyroSchema<'a>) -> Self {
        s.to_arrow()
    }
}

impl<'a> From<PyroSchema<'a>> for Schema {
    fn from(s: PyroSchema<'a>) -> Self {
        s.to_arrow()
    }
}

// =============================================================================
// Internal: convert types WITH names
// =============================================================================

fn type_to_arrow_with_names(ty: &PyroType, names: &FieldNames<'_>) -> DataType {
    match ty {
        PyroType::Group(children) | PyroType::OptionGroup(children) => {
            let mut offset = 0;
            let arrow_fields: Vec<Arc<Field>> = children
                .iter()
                .map(|ct| {
                    let name = names.get(offset);
                    let nc = ct.name_count();
                    let child_dt = if nc > 0 {
                        type_to_arrow_with_names_at(ct, names, offset + 1)
                    } else {
                        ct.to_arrow()
                    };
                    offset += 1 + nc;
                    Arc::new(Field::new(name, child_dt, ct.is_nullable()))
                })
                .collect();
            DataType::Struct(arrow_fields.into())
        }

        PyroType::List(inner) => {
            let inner_dt = if inner.name_count() > 0 {
                type_to_arrow_with_names(inner, names)
            } else {
                inner.to_arrow()
            };
            DataType::List(Arc::new(Field::new("item", inner_dt, inner.is_nullable())))
        }

        PyroType::Map { key, value } => {
            let key_nc = key.name_count();
            let key_dt = if key_nc > 0 {
                type_to_arrow_with_names_at(key, names, 0)
            } else {
                key.to_arrow()
            };
            let val_dt = if value.name_count() > 0 {
                type_to_arrow_with_names_at(value, names, key_nc)
            } else {
                value.to_arrow()
            };
            DataType::Map(
                Arc::new(Field::new(
                    "entries",
                    DataType::Struct(
                        vec![
                            Arc::new(Field::new("key", key_dt, false)),
                            Arc::new(Field::new("value", val_dt, true)),
                        ]
                        .into(),
                    ),
                    false,
                )),
                false,
            )
        }

        _ => ty.to_arrow(),
    }
}

fn type_to_arrow_with_names_at(ty: &PyroType, names: &FieldNames<'_>, start: usize) -> DataType {
    match ty {
        PyroType::Group(children) | PyroType::OptionGroup(children) => {
            let mut offset = start;
            let arrow_fields: Vec<Arc<Field>> = children
                .iter()
                .map(|ct| {
                    let name = names.get(offset);
                    let nc = ct.name_count();
                    let child_dt = if nc > 0 {
                        type_to_arrow_with_names_at(ct, names, offset + 1)
                    } else {
                        ct.to_arrow()
                    };
                    offset += 1 + nc;
                    Arc::new(Field::new(name, child_dt, ct.is_nullable()))
                })
                .collect();
            DataType::Struct(arrow_fields.into())
        }

        PyroType::List(inner) => {
            let inner_dt = if inner.name_count() > 0 {
                type_to_arrow_with_names_at(inner, names, start)
            } else {
                inner.to_arrow()
            };
            DataType::List(Arc::new(Field::new("item", inner_dt, inner.is_nullable())))
        }

        PyroType::Map { key, value } => {
            let key_nc = key.name_count();
            let key_dt = if key_nc > 0 {
                type_to_arrow_with_names_at(key, names, start)
            } else {
                key.to_arrow()
            };
            let val_dt = if value.name_count() > 0 {
                type_to_arrow_with_names_at(value, names, start + key_nc)
            } else {
                value.to_arrow()
            };
            DataType::Map(
                Arc::new(Field::new(
                    "entries",
                    DataType::Struct(
                        vec![
                            Arc::new(Field::new("key", key_dt, false)),
                            Arc::new(Field::new("value", val_dt, true)),
                        ]
                        .into(),
                    ),
                    false,
                )),
                false,
            )
        }

        _ => ty.to_arrow(),
    }
}

fn child_field_type_to_arrow(cf: &ChildField<'_>) -> DataType {
    let ty = cf.datatype();
    if ty.name_count() == 0 {
        return ty.to_arrow();
    }

    match ty {
        PyroType::Group(_) | PyroType::OptionGroup(_) => {
            let child_iter = cf.group_children().unwrap();
            let arrow_fields: Vec<Arc<Field>> =
                child_iter.map(|child| Arc::new(child.to_arrow())).collect();
            DataType::Struct(arrow_fields.into())
        }

        PyroType::List(inner) => {
            if let Some(inner_children) = cf.list_group_children() {
                let arrow_fields: Vec<Arc<Field>> = inner_children
                    .map(|child| Arc::new(child.to_arrow()))
                    .collect();
                let inner_dt = DataType::Struct(arrow_fields.into());
                DataType::List(Arc::new(Field::new("item", inner_dt, inner.is_nullable())))
            } else {
                DataType::List(Arc::new(Field::new(
                    "item",
                    inner.to_arrow(),
                    inner.is_nullable(),
                )))
            }
        }

        _ => ty.to_arrow(),
    }
}

// =============================================================================
// Internal: Arrow → (PyroType, FieldNames)
// =============================================================================

fn type_from_arrow_with_names(dt: &DataType) -> Option<(PyroType, FieldNames<'static>)> {
    Some(match dt {
        DataType::Struct(fields) => {
            let mut names: Vec<Cow<'static, str>> = Vec::new();
            let mut child_types = Vec::with_capacity(fields.len());

            for f in fields.iter() {
                names.push(Cow::Owned(f.name().to_string()));
                let (ct, sub_names) = type_from_arrow_with_names(f.data_type())?;
                let ct = if f.is_nullable() {
                    ct.to_nullable()
                } else {
                    ct
                };
                match sub_names {
                    FieldNames::Owned(v) => names.extend(v),
                    FieldNames::Borrowed(s) => names.extend_from_slice(s),
                    FieldNames::Static(s) => {
                        names.extend(s.iter().map(|&r| Cow::<'static, str>::Owned(r.to_owned())));
                    }
                    FieldNames::Empty(l) => names.extend(vec!["".into(); l]),
                }
                child_types.push(ct);
            }

            (
                PyroType::Group(child_types.into()),
                FieldNames::Owned(names),
            )
        }

        DataType::List(inner) | DataType::LargeList(inner) => {
            if let Some(pdt) = PrimitiveDataType::from_arrow(inner.data_type()) {
                (PyroType::PrimitiveList(pdt), FieldNames::empty(0))
            } else {
                let (inner_dt, inner_names) = type_from_arrow_with_names(inner.data_type())?;
                (PyroType::List(Bow::new(inner_dt)), inner_names)
            }
        }

        DataType::FixedSizeList(inner, size) => {
            if let Some(pdt) = PrimitiveDataType::from_arrow(inner.data_type()) {
                (
                    PyroType::PrimitiveFixedList(pdt, *size as usize),
                    FieldNames::empty(0),
                )
            } else {
                let (inner_dt, inner_names) = type_from_arrow_with_names(inner.data_type())?;
                (PyroType::List(Bow::new(inner_dt)), inner_names)
            }
        }

        DataType::Map(entries_field, _) => {
            if let DataType::Struct(kv_fields) = entries_field.data_type() {
                if kv_fields.len() == 2 {
                    let (key_dt, key_names) = type_from_arrow_with_names(kv_fields[0].data_type())?;
                    let (val_dt, val_names) = type_from_arrow_with_names(kv_fields[1].data_type())?;

                    let mut names = key_names.into_owned_vec();
                    names.extend(val_names.into_owned_vec());
                    let field_names = if names.is_empty() {
                        FieldNames::empty(0)
                    } else {
                        FieldNames::Owned(names)
                    };

                    (
                        PyroType::Map {
                            key: Bow::new(key_dt),
                            value: Bow::new(val_dt),
                        },
                        field_names,
                    )
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }

        DataType::Dictionary(_, value_type) => type_from_arrow_with_names(value_type)?,

        other => (PyroType::from_arrow(other)?, FieldNames::empty(0)),
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_roundtrip() {
        let types = vec![
            PyroType::Unknown,
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
            PyroType::PrimitiveScalar(PrimitiveDataType::Timestamp),
        ];

        for dt in types {
            let arrow_dt = dt.to_arrow();
            let roundtripped = PyroType::from_arrow(&arrow_dt).unwrap();
            assert_eq!(dt, roundtripped, "roundtrip failed for {dt:?}");
        }
    }

    #[test]
    fn test_nullable_field_roundtrip() {
        let field = PyroField::new(
            "age".to_string(),
            PyroType::OptionPrimitiveScalar(PrimitiveDataType::I32),
        );

        let arrow_field = field.to_arrow();
        assert!(arrow_field.is_nullable());
        assert_eq!(arrow_field.data_type(), &DataType::Int32);

        let back = PyroField::from_arrow(&arrow_field).unwrap();
        assert_eq!(back.name(), "age");
        assert!(back.datatype.is_nullable());
        assert_eq!(
            back.datatype(),
            &PyroType::OptionPrimitiveScalar(PrimitiveDataType::I32)
        );
    }

    #[test]
    fn test_non_nullable_field_roundtrip() {
        let field = PyroField::new(
            "id".to_string(),
            PyroType::PrimitiveScalar(PrimitiveDataType::I64),
        );

        let arrow_field = field.to_arrow();
        assert!(!arrow_field.is_nullable());

        let back = PyroField::from_arrow(&arrow_field).unwrap();
        assert!(!back.datatype.is_nullable());
        assert_eq!(
            back.datatype(),
            &PyroType::PrimitiveScalar(PrimitiveDataType::I64)
        );
    }

    #[test]
    fn test_nullable_timestamp_roundtrip() {
        let field = PyroField::new(
            "ts".to_string(),
            PyroType::OptionPrimitiveScalar(PrimitiveDataType::Timestamp),
        );

        let arrow_field = field.to_arrow();
        assert!(arrow_field.is_nullable());

        let back = PyroField::from_arrow(&arrow_field).unwrap();
        assert_eq!(
            back.datatype(),
            &PyroType::OptionPrimitiveScalar(PrimitiveDataType::Timestamp)
        );
    }

    #[test]
    fn test_group_with_nullable_children_roundtrip() {
        let field = PyroField::group(
            "person".to_string(),
            vec![
                PyroField::new("name".to_string(), PyroType::Str),
                PyroField::new(
                    "age".to_string(),
                    PyroType::OptionPrimitiveScalar(PrimitiveDataType::I32),
                ),
            ],
        );

        let arrow_field = field.to_arrow();
        if let DataType::Struct(fields) = arrow_field.data_type() {
            assert!(!fields[0].is_nullable()); // Str is not Option*
            assert!(fields[1].is_nullable()); // OptionPrimitiveScalar
        } else {
            panic!("Expected Struct");
        }

        let back = PyroField::from_arrow(&arrow_field).unwrap();
        let children: Vec<_> = back.group_children().unwrap().collect();
        assert!(!children[0].datatype().is_nullable());
        assert!(children[1].datatype().is_nullable());
    }

    #[test]
    fn test_nullable_group_roundtrip() {
        let mut field = PyroField::group(
            "meta".to_string(),
            vec![PyroField::new("key".to_string(), PyroType::Str)],
        );
        field.datatype = field.datatype.to_nullable();

        let arrow_field = field.to_arrow();
        assert!(arrow_field.is_nullable());

        let back = PyroField::from_arrow(&arrow_field).unwrap();
        assert!(back.datatype.is_nullable());
        assert!(matches!(back.datatype(), PyroType::OptionGroup(_)));
    }

    #[test]
    fn test_primitive_list_roundtrip() {
        for pdt in [
            PrimitiveDataType::Bool,
            PrimitiveDataType::U8,
            PrimitiveDataType::I32,
            PrimitiveDataType::F64,
        ] {
            let dt = PyroType::PrimitiveList(pdt);
            let arrow_dt = dt.to_arrow();
            let back = PyroType::from_arrow(&arrow_dt).unwrap();
            assert_eq!(dt, back);
        }
    }

    #[test]
    fn test_fixed_list_roundtrip() {
        let dt = PyroType::PrimitiveFixedList(PrimitiveDataType::F32, 128);
        let arrow_dt = dt.to_arrow();
        let back = PyroType::from_arrow(&arrow_dt).unwrap();
        assert_eq!(back, dt);

        let dt_opt = PyroType::OptionPrimitiveFixedList(PrimitiveDataType::F32, 128);
        let arrow_dt_opt = dt_opt.to_arrow();
        // Since from_arrow returns non-nullable, we roundtrip back to non-nullable first,
        // and then test that after converting to_nullable it matches.
        let back_opt = PyroType::from_arrow(&arrow_dt_opt).unwrap().to_nullable();
        assert_eq!(back_opt, dt_opt);
    }

    #[test]
    fn test_list_roundtrip() {
        let dt = PyroType::List(Bow::new(PyroType::Str));
        let arrow_dt = dt.to_arrow();
        let back = PyroType::from_arrow(&arrow_dt).unwrap();
        assert_eq!(back, dt);
    }

    #[test]
    fn test_map_roundtrip() {
        let dt = PyroType::Map {
            key: Bow::new(PyroType::Str),
            value: Bow::new(PyroType::PrimitiveScalar(PrimitiveDataType::I64)),
        };
        let arrow_dt = dt.to_arrow();
        let back = PyroType::from_arrow(&arrow_dt).unwrap();
        assert_eq!(back, dt);
    }

    #[test]
    fn test_list_of_group_roundtrip() {
        let field = PyroField::with_children(
            "items".to_string(),
            FieldNames::Owned(vec![Cow::Borrowed("x"), Cow::Borrowed("y")]),
            PyroType::List(Bow::new(PyroType::Group(
                vec![
                    PyroType::Str,
                    PyroType::PrimitiveScalar(PrimitiveDataType::I32),
                ]
                .into(),
            ))),
        );

        let arrow_field = field.to_arrow();
        if let DataType::List(inner) = arrow_field.data_type() {
            if let DataType::Struct(fields) = inner.data_type() {
                assert_eq!(fields[0].name(), "x");
                assert_eq!(fields[1].name(), "y");
            } else {
                panic!("Expected inner Struct");
            }
        } else {
            panic!("Expected List");
        }

        let back = PyroField::from_arrow(&arrow_field).unwrap();
        let children: Vec<_> = back.list_group_children().unwrap().collect();
        assert_eq!(children[0].name(), "x");
        assert_eq!(children[1].name(), "y");
    }

    #[test]
    fn test_schema_roundtrip() {
        let schema = PyroSchema::new(vec![
            PyroField::new(
                "id".to_string(),
                PyroType::PrimitiveScalar(PrimitiveDataType::I64),
            ),
            PyroField::new("name".to_string(), PyroType::Str),
            PyroField::new(
                "score".to_string(),
                PyroType::OptionPrimitiveScalar(PrimitiveDataType::F64),
            ),
        ]);

        let arrow_schema = schema.to_arrow();
        assert_eq!(arrow_schema.fields().len(), 3);
        assert!(!arrow_schema.field(0).is_nullable());
        assert!(arrow_schema.field(2).is_nullable());

        let back = PyroSchema::from_arrow(&arrow_schema).unwrap();
        assert_eq!(back.num_fields(), 3);
        assert!(!back.field(0).datatype().is_nullable());
        assert!(back.field(2).datatype().is_nullable());
    }

    #[test]
    fn test_unsupported_returns_none() {
        assert!(PyroType::from_arrow(&DataType::Decimal128(10, 2)).is_none());
    }

    #[test]
    fn test_dictionary_unwrap() {
        let dt = DataType::Dictionary(Box::new(DataType::Int32), Box::new(DataType::Utf8));
        assert_eq!(PyroType::from_arrow(&dt).unwrap(), PyroType::Str);
    }
}
