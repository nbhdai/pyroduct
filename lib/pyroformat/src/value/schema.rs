// =============================================================================
// PyroType / PyroSchema inference from PyroValue
// =============================================================================

use std::borrow::Cow;
use std::collections::{BTreeMap, HashSet};

use super::{PrimitiveValueList, PyroRow, PyroValue};
use crate::schema::{
    FieldNames, PrimitiveDataType, PyroField, PyroSchema, PyroType, coerce_pyro_types,
};
use pyro_schema::Bow;
use thiserror::Error;

// =============================================================================
// PyroType inference from PyroValue
// =============================================================================

impl<'a> PyroValue<'a> {
    /// Infer the [`PyroType`] from a single [`PyroValue`].
    ///
    /// For `Group` values, only the type tree is returned here.
    /// Use [`PyroValue::infer_field`] or [`PyroRow::schema`] to get
    /// a full `PyroField` with names.
    pub fn datatype(&self) -> PyroType {
        match self {
            PyroValue::Null => PyroType::Unknown,
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
            PyroValue::Bytes(_) => PyroType::Bytes,
            PyroValue::Timestamp(_) => PyroType::PrimitiveScalar(PrimitiveDataType::Timestamp),

            PyroValue::PrimitiveList(pl) => PyroType::PrimitiveList(pl.datatype()),

            PyroValue::List(items) => {
                let mut inner = PyroType::Unknown;
                let mut contains_null = false;

                for item in items {
                    let item_dt = item.datatype();
                    if item.is_null() {
                        contains_null = true;
                    }

                    if inner == PyroType::Unknown {
                        inner = item_dt;
                    } else if inner != item_dt && !item.is_null() {
                        if let Some(coerced) = coerce_pyro_types(&inner, &item_dt) {
                            inner = coerced;
                        }
                    }
                }

                if contains_null {
                    inner = inner.to_nullable();
                }

                PyroType::List(Bow::new(inner))
            }

            PyroValue::Group(row) => {
                let child_types: Vec<PyroType> =
                    row.0.iter().map(|item| item.value.datatype()).collect();
                PyroType::Group(child_types.into())
            }

            PyroValue::Map(pairs) => {
                let mut key_dt = PyroType::Unknown;
                let mut val_dt = PyroType::Unknown;

                for (k, v) in pairs {
                    let k_type = k.datatype();
                    let v_type = v.datatype();

                    if key_dt == PyroType::Unknown {
                        key_dt = k_type;
                    } else if let Some(coerced) = coerce_pyro_types(&key_dt, &k_type) {
                        key_dt = coerced;
                    }

                    if val_dt == PyroType::Unknown {
                        val_dt = v_type;
                    } else if let Some(coerced) = coerce_pyro_types(&val_dt, &v_type) {
                        val_dt = coerced;
                    }
                }

                if key_dt == PyroType::Unknown {
                    key_dt = PyroType::Str;
                }

                PyroType::Map {
                    key: Bow::new(key_dt),
                    value: Bow::new(val_dt),
                }
            }
        }
    }

    /// Infer a `PyroField` from this value, given a field name.
    /// Extracts both type and child names (for Groups).
    pub fn infer_field<'b>(&self, name: impl Into<Cow<'b, str>>) -> PyroField<'b> {
        let name_cow = name.into();
        match self {
            PyroValue::Group(row) => {
                let children: Vec<PyroField<'b>> = row
                    .0
                    .iter()
                    .map(|item| {
                        item.value
                            .infer_field(Cow::Owned(item.key.as_ref().to_owned()))
                    })
                    .collect();
                PyroField::group(name_cow.into_owned(), children)
            }
            PyroValue::List(items) => {
                // Check if inner type is a Group — if so, we need child names
                let dt = self.datatype();
                if let PyroType::List(ref inner) = dt {
                    if let PyroType::Group(_) = inner.as_ref() {
                        // Find the first non-null Group item to extract names
                        let child_names = extract_list_group_names(items);
                        if !child_names.is_empty() {
                            return PyroField::with_children(
                                name_cow.into_owned(),
                                FieldNames::Owned(child_names),
                                dt,
                            );
                        }
                    }
                }
                PyroField::new(name_cow.into_owned(), dt)
            }
            _ => {
                let dt = self.datatype();
                let nc = dt.name_count();
                if nc == 0 {
                    PyroField::new(name_cow.into_owned(), dt)
                } else {
                    // Shouldn't happen for non-Group/non-List-of-Group, but handle gracefully
                    let child_names = FieldNames::Owned(vec![Cow::Owned("?".to_string()); nc]);
                    PyroField::with_children(name_cow.into_owned(), child_names, dt)
                }
            }
        }
    }
}

/// Extract flattened child names from the first non-null Group in a list of values.
fn extract_list_group_names<'a>(items: &[PyroValue<'a>]) -> Vec<Cow<'static, str>> {
    for item in items {
        if let PyroValue::Group(row) = item {
            return extract_group_names_recursive(row);
        }
    }
    vec![]
}

/// Recursively extract flattened pre-order names from a PyroRow.
fn extract_group_names_recursive<'a>(row: &PyroRow<'a>) -> Vec<Cow<'static, str>> {
    let mut names = Vec::new();
    for item in &row.0 {
        names.push(Cow::Owned(item.key.as_ref().to_owned()));
        if let PyroValue::Group(inner_row) = &item.value {
            names.extend(extract_group_names_recursive(inner_row));
        }
    }
    names
}

impl<'a> PrimitiveValueList<'a> {
    pub fn datatype(&self) -> PrimitiveDataType {
        match self {
            PrimitiveValueList::Bool(..) => PrimitiveDataType::Bool,
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
            PrimitiveValueList::Timestamp(_) => PrimitiveDataType::Timestamp,
        }
    }
}

// =============================================================================
// PyroSchema inference (from rows)
// =============================================================================

impl<'a> PyroRow<'a> {
    /// Build a schema from a single row.
    pub fn schema(&self) -> Result<PyroSchema<'static>, ValueSchemaInferenceError> {
        if self.is_empty() {
            return Err(ValueSchemaInferenceError::EmptyRow);
        }
        let fields: Vec<PyroField<'static>> = self
            .0
            .iter()
            .map(|item| {
                item.value
                    .infer_field(Cow::Owned(item.key.as_ref().to_owned()))
            })
            .collect();

        Ok(PyroSchema::new(fields))
    }

    /// Infer a schema from multiple rows, with coercion and nullability tracking.
    pub fn infer_schema(
        rows: &[PyroRow<'_>],
    ) -> Result<PyroSchema<'static>, ValueSchemaInferenceError> {
        let mut field_map: BTreeMap<String, FieldAccumulator> = BTreeMap::new();

        for (i, row) in rows.iter().enumerate() {
            let mut row_keys: HashSet<&str> = HashSet::new();

            for item in &row.0 {
                let key = item.key.as_ref();
                row_keys.insert(key);
                let is_new = !field_map.contains_key(key);

                let accum = field_map
                    .entry(key.to_owned())
                    .or_insert_with(FieldAccumulator::new);

                if is_new && i > 0 {
                    accum.nullable = true;
                }

                accum.observe(key, &item.value)?;
            }

            // Fields absent from this row become nullable
            for (key, accum) in field_map.iter_mut() {
                if !row_keys.contains(key.as_str()) {
                    accum.nullable = true;
                }
            }
        }

        let fields: Vec<PyroField<'static>> = field_map
            .into_iter()
            .map(|(name, accum)| accum.build(name))
            .collect::<Result<_, _>>()?;

        Ok(PyroSchema::new(fields))
    }
}

// =============================================================================
// Errors
// =============================================================================

#[derive(Debug, Error, PartialEq, Clone)]
pub enum ValueSchemaInferenceError {
    #[error("Incompatible types for field '{0}': {1:?} vs {2:?}")]
    IncompatibleTypes(String, PyroType, PyroType),
    #[error("Row is empty, cannot infer schema")]
    EmptyRow,
}

// =============================================================================
// Internal: accumulator
// =============================================================================

struct FieldAccumulator {
    datatype: Option<PyroType>,
    /// Tracks child names for Group types (flattened pre-order).
    child_names: Option<Vec<Cow<'static, str>>>,
    nullable: bool,
}

impl FieldAccumulator {
    fn new() -> Self {
        Self {
            datatype: None,
            child_names: None,
            nullable: false,
        }
    }

    fn observe(
        &mut self,
        field_name: &str,
        value: &PyroValue<'_>,
    ) -> Result<(), ValueSchemaInferenceError> {
        if matches!(value, PyroValue::Null) {
            self.nullable = true;
            return Ok(());
        }

        let observed = value.datatype();

        // Extract child names from Group values if we don't have them yet
        if self.child_names.is_none() {
            if let PyroValue::Group(row) = value {
                self.child_names = Some(extract_group_names_recursive(row));
            } else if let PyroValue::List(items) = value {
                if let PyroType::List(ref inner) = observed {
                    if matches!(inner.as_ref(), PyroType::Group(_)) {
                        let names = extract_list_group_names(items);
                        if !names.is_empty() {
                            self.child_names = Some(names);
                        }
                    }
                }
            }
        }

        match &self.datatype {
            None => {
                self.datatype = Some(observed);
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
                self.datatype = Some(coerced);
            }
        }
        Ok(())
    }

    fn build(self, name: String) -> Result<PyroField<'static>, ValueSchemaInferenceError> {
        let dt = self.datatype.unwrap_or(PyroType::Unknown);
        let dt = if self.nullable { dt.to_nullable() } else { dt };

        if let Some(child_names) = self.child_names {
            if dt.name_count() == child_names.len() {
                Ok(PyroField::with_children(
                    name,
                    FieldNames::Owned(child_names),
                    dt,
                ))
            } else {
                // Names don't match (e.g. coercion changed group structure).
                // Fall back — this is an edge case.
                if dt.name_count() == 0 {
                    Ok(PyroField::new(name, dt))
                } else {
                    // Pad or truncate — shouldn't normally happen
                    let mut padded = child_names;
                    padded.resize(dt.name_count(), Cow::Owned("?".to_string()));
                    Ok(PyroField::with_children(
                        name,
                        FieldNames::Owned(padded),
                        dt,
                    ))
                }
            }
        } else if dt.name_count() == 0 {
            Ok(PyroField::new(name, dt))
        } else {
            // Type has groups but we never saw a non-null value with names
            let placeholder = vec![Cow::Owned("?".to_string()); dt.name_count()];
            Ok(PyroField::with_children(
                name,
                FieldNames::Owned(placeholder),
                dt,
            ))
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use pyro_schema::Bow;

    use super::*;

    // ---- Inference from single values ----

    #[test]
    fn test_datatype_from_value_scalars() {
        assert_eq!(PyroValue::Null.datatype(), PyroType::Unknown);
        assert_eq!(
            PyroValue::I32(42).datatype(),
            PyroType::PrimitiveScalar(PrimitiveDataType::I32)
        );
        assert_eq!(
            PyroValue::F64(1.0).datatype(),
            PyroType::PrimitiveScalar(PrimitiveDataType::F64)
        );
        assert_eq!(PyroValue::from("hello").datatype(), PyroType::Str);
    }

    #[test]
    fn test_datatype_from_value_list_nullable() {
        let list = PyroValue::List(vec![PyroValue::I32(1), PyroValue::Null, PyroValue::I32(3)]);
        assert_eq!(
            list.datatype(),
            PyroType::List(Bow::new(PyroType::OptionPrimitiveScalar(
                PrimitiveDataType::I32
            )),)
        );
    }

    #[test]
    fn test_datatype_from_value_list_not_nullable() {
        let list = PyroValue::List(vec![PyroValue::I32(1), PyroValue::I32(2)]);
        assert_eq!(
            list.datatype(),
            PyroType::List(Bow::new(PyroType::PrimitiveScalar(PrimitiveDataType::I32)),)
        );
    }

    #[test]
    fn test_datatype_from_value_group() {
        let row = PyroRow::from([
            ("name", PyroValue::from("Alice")),
            ("age", PyroValue::I32(30)),
        ]);
        let dt = PyroValue::Group(row).datatype();
        assert_eq!(
            dt,
            PyroType::Group(
                vec![
                    PyroType::Str,
                    PyroType::PrimitiveScalar(PrimitiveDataType::I32),
                ]
                .into()
            )
        );
    }

    #[test]
    fn test_datatype_from_value_map_coercion() {
        let map_val = PyroValue::Map(crate::Map::from(vec![
            (PyroValue::from("a"), PyroValue::I32(10)),
            (PyroValue::from("b"), PyroValue::F64(20.5)),
        ]));
        let dt = map_val.datatype();

        if let PyroType::Map { key, value } = dt {
            assert_eq!(*key, PyroType::Str);
            assert_eq!(*value, PyroType::PrimitiveScalar(PrimitiveDataType::F64));
        } else {
            panic!("Expected Map type");
        }
    }

    // ---- infer_field ----

    #[test]
    fn test_infer_field_leaf() {
        let field = PyroValue::I32(42).infer_field("x");
        assert_eq!(field.name(), "x");
        assert_eq!(
            field.datatype(),
            &PyroType::PrimitiveScalar(PrimitiveDataType::I32)
        );
        assert!(field.child_names().is_empty());
    }

    #[test]
    fn test_infer_field_group() {
        let row = PyroRow::from([
            ("name", PyroValue::from("Alice")),
            ("age", PyroValue::I32(30)),
        ]);
        let field = PyroValue::Group(row).infer_field("person");
        assert_eq!(field.name(), "person");
        assert_eq!(field.child_names().len(), 2);

        let children: Vec<_> = field.group_children().unwrap().collect();
        assert_eq!(children[0].name(), "name");
        assert_eq!(children[0].datatype(), &PyroType::Str);
        assert_eq!(children[1].name(), "age");
        assert_eq!(
            children[1].datatype(),
            &PyroType::PrimitiveScalar(PrimitiveDataType::I32)
        );
    }

    #[test]
    fn test_infer_field_list_of_group() {
        let items = vec![
            PyroValue::Group(PyroRow::from([
                ("x", PyroValue::I32(1)),
                ("y", PyroValue::I32(2)),
            ])),
            PyroValue::Group(PyroRow::from([
                ("x", PyroValue::I32(3)),
                ("y", PyroValue::I32(4)),
            ])),
        ];
        let field = PyroValue::List(items).infer_field("points");
        assert_eq!(field.name(), "points");
        assert_eq!(field.child_names().len(), 2);

        let children: Vec<_> = field.list_group_children().unwrap().collect();
        assert_eq!(children[0].name(), "x");
        assert_eq!(children[1].name(), "y");
    }

    // ---- Schema inference ----

    #[test]
    fn test_trusted_schema() {
        let row = PyroRow::from([("a", PyroValue::I32(10)), ("b", PyroValue::from("hi"))]);
        let schema = row.schema().unwrap();
        assert_eq!(schema.num_fields(), 2);
        assert_eq!(schema.field(0).name(), "a");
        assert_eq!(
            schema.field(0).datatype(),
            &PyroType::PrimitiveScalar(PrimitiveDataType::I32)
        );
        assert_eq!(schema.field(1).name(), "b");
        assert_eq!(schema.field(1).datatype(), &PyroType::Str);
    }

    #[test]
    fn test_infer_coercion() {
        let rows = vec![
            PyroRow::from([("mix", PyroValue::I32(10))]),
            PyroRow::from([("mix", PyroValue::F64(20.5))]),
        ];
        let schema = PyroRow::infer_schema(&rows).unwrap();
        assert_eq!(
            schema.field(0).datatype(),
            &PyroType::PrimitiveScalar(PrimitiveDataType::F64)
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
            schema.field(0).datatype(),
            &PyroType::OptionPrimitiveScalar(PrimitiveDataType::I32)
        );
        assert!(schema.field(0).datatype().is_nullable());
    }

    #[test]
    fn test_infer_sparse() {
        let rows = vec![
            PyroRow::from([("a", PyroValue::I32(10))]),
            PyroRow::from([("b", PyroValue::F64(1.0))]),
        ];
        let schema = PyroRow::infer_schema(&rows).unwrap();
        assert_eq!(schema.num_fields(), 2);
        // Both should be nullable since each is missing from one row
        assert!(
            schema
                .field_with_name("a")
                .unwrap()
                .datatype()
                .is_nullable()
        );
        assert!(
            schema
                .field_with_name("b")
                .unwrap()
                .datatype()
                .is_nullable()
        );
    }

    #[test]
    fn test_infer_int_widening() {
        let rows = vec![
            PyroRow::from([("num", PyroValue::I8(1))]),
            PyroRow::from([("num", PyroValue::I32(1000))]),
        ];
        let schema = PyroRow::infer_schema(&rows).unwrap();
        assert_eq!(
            schema.field(0).datatype(),
            &PyroType::PrimitiveScalar(PrimitiveDataType::I32)
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
            schema.field(0).datatype(),
            &PyroType::PrimitiveList(PrimitiveDataType::I32)
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

    #[test]
    fn test_infer_group_field() {
        let rows = vec![
            PyroRow::from([(
                "meta",
                PyroValue::Group(PyroRow::from([
                    ("key", PyroValue::from("a")),
                    ("val", PyroValue::I32(1)),
                ])),
            )]),
            PyroRow::from([(
                "meta",
                PyroValue::Group(PyroRow::from([
                    ("key", PyroValue::from("b")),
                    ("val", PyroValue::I32(2)),
                ])),
            )]),
        ];
        let schema = PyroRow::infer_schema(&rows).unwrap();
        let meta = schema.field(0);
        assert_eq!(meta.name(), "meta");

        let children: Vec<_> = meta.group_children().unwrap().collect();
        assert_eq!(children[0].name(), "key");
        assert_eq!(children[1].name(), "val");
    }

    #[test]
    fn test_infer_nullable_group() {
        let rows = vec![
            PyroRow::from([(
                "meta",
                PyroValue::Group(PyroRow::from([("key", PyroValue::from("a"))])),
            )]),
            PyroRow::from([("meta", PyroValue::Null)]),
        ];
        let schema = PyroRow::infer_schema(&rows).unwrap();
        let meta = schema.field(0);
        assert!(meta.datatype().is_nullable());
        assert!(matches!(meta.datatype(), PyroType::OptionGroup(_)));
    }

    // ---- Coercion edge cases ----

    #[test]
    fn test_coerce_primitive_fixed_list_same_size() {
        let a = PyroType::PrimitiveFixedList(PrimitiveDataType::I8, 4);
        let b = PyroType::PrimitiveFixedList(PrimitiveDataType::I32, 4);
        assert_eq!(
            coerce_pyro_types(&a, &b),
            Some(PyroType::PrimitiveFixedList(PrimitiveDataType::I32, 4))
        );
    }

    #[test]
    fn test_coerce_primitive_fixed_list_different_size() {
        let a = PyroType::PrimitiveFixedList(PrimitiveDataType::I32, 4);
        let b = PyroType::PrimitiveFixedList(PrimitiveDataType::I32, 8);
        assert_eq!(
            coerce_pyro_types(&a, &b),
            Some(PyroType::PrimitiveList(PrimitiveDataType::I32))
        );
    }

    #[test]
    fn test_coerce_fixed_with_variable_list() {
        let a = PyroType::PrimitiveFixedList(PrimitiveDataType::I32, 4);
        let b = PyroType::PrimitiveList(PrimitiveDataType::I32);
        assert_eq!(
            coerce_pyro_types(&a, &b),
            Some(PyroType::PrimitiveList(PrimitiveDataType::I32))
        );
    }

    #[test]
    fn test_coerce_lists_merge_nullability() {
        let a = PyroType::List(Bow::new(PyroType::PrimitiveScalar(PrimitiveDataType::I32)));
        let b = PyroType::List(Bow::new(PyroType::PrimitiveScalar(PrimitiveDataType::I32)));
        assert_eq!(
            coerce_pyro_types(&a, &b),
            Some(PyroType::List(Bow::new(PyroType::PrimitiveScalar(
                PrimitiveDataType::I32
            )),))
        );
    }
}
