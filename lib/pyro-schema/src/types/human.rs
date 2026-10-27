// =============================================================================
// Human-readable Pyro schema types
// =============================================================================
//
// Fully owned, serde-friendly versions of the machine-oriented types.
// Group carries `Vec<PyroField>` inline, making these natural for
// JSON/YAML serialization and human editing.
//
// Convert to/from the machine types via `From` / `Into` impls.

use crate::Bow;
use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::types::{self as machine, ChildField, FieldNames, PrimitiveDataType};

// =============================================================================
// PyroType (human)
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PyroType {
    Unknown,
    PrimitiveScalar {
        primitive: PrimitiveDataType,
    },
    Str,
    Bytes,
    PrimitiveFixedList {
        element: PrimitiveDataType,
        size: usize,
    },
    List {
        element: Box<PyroType>,
    },
    Group {
        fields: Vec<PyroField>,
    },
    Map {
        key: Box<PyroType>,
        value: Box<PyroType>,
    },

    OptionPrimitiveScalar {
        primitive: PrimitiveDataType,
    },
    OptionPrimitiveFixedList {
        element: PrimitiveDataType,
        size: usize,
    },
    OptionGroup {
        fields: Vec<PyroField>,
    },
}

// =============================================================================
// PyroField (human)
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PyroField {
    pub name: String,
    pub doc: String,
    pub data_type: PyroType,
}

impl PyroField {
    pub fn new(name: impl Into<String>, data_type: PyroType) -> Self {
        Self {
            name: name.into(),
            doc: String::new(),
            data_type,
        }
    }

    pub fn with_doc(mut self, doc: impl Into<String>) -> Self {
        self.doc = doc.into();
        self
    }
}

// =============================================================================
// PyroSchema (human)
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PyroSchema {
    pub doc: String,
    pub fields: Vec<PyroField>,
}

impl PyroSchema {
    pub fn new(fields: Vec<PyroField>) -> Self {
        Self {
            doc: String::new(),
            fields,
        }
    }

    pub fn with_doc(mut self, doc: impl Into<String>) -> Self {
        self.doc = doc.into();
        self
    }
}

// =============================================================================
// human → machine conversions
// =============================================================================

impl PyroType {
    /// Convert to the machine `PyroType`.
    /// Returns `(type, child_names, child_docs)` since Group names/docs must be stored externally.
    pub fn to_machine(
        &self,
    ) -> (
        machine::PyroType,
        Vec<Cow<'static, str>>,
        Vec<Cow<'static, str>>,
    ) {
        match self {
            PyroType::Unknown => (machine::PyroType::Unknown, vec![], vec![]),
            PyroType::PrimitiveScalar { primitive } => (
                machine::PyroType::PrimitiveScalar(*primitive),
                vec![],
                vec![],
            ),
            PyroType::Str => (machine::PyroType::Str, vec![], vec![]),
            PyroType::Bytes => (machine::PyroType::Bytes, vec![], vec![]),
            PyroType::PrimitiveFixedList { element, size } => (
                machine::PyroType::PrimitiveFixedList(*element, *size),
                vec![],
                vec![],
            ),
            PyroType::List { element } => {
                let (inner, names, docs) = element.to_machine();
                (machine::PyroType::List(Bow::new(inner)), names, docs)
            }
            PyroType::Group { fields } => {
                let (child_types, names, docs) = fields_to_machine(fields);
                (machine::PyroType::Group(child_types.into()), names, docs)
            }
            PyroType::Map { key, value } => {
                let (k, mut key_names, mut key_docs) = key.to_machine();
                let (v, val_names, val_docs) = value.to_machine();
                key_names.extend(val_names);
                key_docs.extend(val_docs);
                (
                    machine::PyroType::Map {
                        key: Bow::new(k),
                        value: Bow::new(v),
                    },
                    key_names,
                    key_docs,
                )
            }
            PyroType::OptionPrimitiveScalar { primitive } => (
                machine::PyroType::OptionPrimitiveScalar(*primitive),
                vec![],
                vec![],
            ),
            PyroType::OptionPrimitiveFixedList { element, size } => (
                machine::PyroType::OptionPrimitiveFixedList(*element, *size),
                vec![],
                vec![],
            ),
            PyroType::OptionGroup { fields } => {
                let (child_types, names, docs) = fields_to_machine(fields);
                (
                    machine::PyroType::OptionGroup(child_types.into()),
                    names,
                    docs,
                )
            }
        }
    }
}

/// Helper: convert a slice of human fields to (child_types, flattened_names, flattened_docs).
fn fields_to_machine(
    fields: &[PyroField],
) -> (
    Vec<machine::PyroType>,
    Vec<Cow<'static, str>>,
    Vec<Cow<'static, str>>,
) {
    let mut names = Vec::new();
    let mut docs = Vec::new();
    let mut child_types = Vec::new();
    for f in fields {
        names.push(Cow::Owned(f.name.clone()));
        docs.push(Cow::Owned(f.doc.clone()));
        let (ct, sub_names, sub_docs) = f.data_type.to_machine();
        names.extend(sub_names);
        docs.extend(sub_docs);
        child_types.push(ct);
    }
    (child_types, names, docs)
}

impl PyroField {
    pub fn to_machine(&self) -> machine::PyroField<'static> {
        let (dt, names, docs) = self.data_type.to_machine();
        if names.is_empty() {
            machine::PyroField::new(self.name.clone(), dt).with_doc(self.doc.clone())
        } else {
            machine::PyroField::with_children_and_docs(
                self.name.clone(),
                FieldNames::Owned(names),
                FieldNames::Owned(docs),
                dt,
            )
            .with_doc(self.doc.clone())
        }
    }
}

impl PyroSchema {
    pub fn to_machine(&self) -> machine::PyroSchema<'static> {
        let fields: Vec<machine::PyroField<'static>> =
            self.fields.iter().map(|f| f.to_machine()).collect();
        machine::PyroSchema::new(fields).with_doc(self.doc.clone())
    }
}

// =============================================================================
// machine → human conversions
// =============================================================================

/// Convert a machine `PyroType` to human, reading Group/OptionGroup names
/// from `names` starting at `offset`. Returns `(human_type, names_consumed)`.
fn machine_type_to_human(
    ty: &machine::PyroType,
    names: &FieldNames<'_>,
    docs: &FieldNames<'_>,
    offset: usize,
) -> (PyroType, usize) {
    match ty {
        machine::PyroType::Unknown => (PyroType::Unknown, 0),
        machine::PyroType::PrimitiveScalar(p) => (PyroType::PrimitiveScalar { primitive: *p }, 0),
        machine::PyroType::Str => (PyroType::Str, 0),
        machine::PyroType::Bytes => (PyroType::Bytes, 0),
        // A list of primitives has one human spelling: a `List` of them.
        machine::PyroType::PrimitiveList(p) => (
            PyroType::List {
                element: Box::new(PyroType::PrimitiveScalar { primitive: *p }),
            },
            0,
        ),
        machine::PyroType::PrimitiveFixedList(p, s) => (
            PyroType::PrimitiveFixedList {
                element: *p,
                size: *s,
            },
            0,
        ),
        machine::PyroType::OptionPrimitiveScalar(p) => {
            (PyroType::OptionPrimitiveScalar { primitive: *p }, 0)
        }
        machine::PyroType::OptionPrimitiveFixedList(p, s) => (
            PyroType::OptionPrimitiveFixedList {
                element: *p,
                size: *s,
            },
            0,
        ),

        machine::PyroType::List(inner) => {
            let (inner_human, consumed) = machine_type_to_human(inner, names, docs, offset);
            (
                PyroType::List {
                    element: Box::new(inner_human),
                },
                consumed,
            )
        }

        machine::PyroType::Group(children) => {
            let (fields, consumed) = machine_group_children_to_human(children, names, docs, offset);
            (PyroType::Group { fields }, consumed)
        }

        machine::PyroType::OptionGroup(children) => {
            let (fields, consumed) = machine_group_children_to_human(children, names, docs, offset);
            (PyroType::OptionGroup { fields }, consumed)
        }

        machine::PyroType::Map { key, value } => {
            let (k, kc) = machine_type_to_human(key, names, docs, offset);
            let (v, vc) = machine_type_to_human(value, names, docs, offset + kc);
            (
                PyroType::Map {
                    key: Box::new(k),
                    value: Box::new(v),
                },
                kc + vc,
            )
        }
    }
}

/// Helper: convert a group's children (types + names at offset) to human fields.
fn machine_group_children_to_human(
    children: &[machine::PyroType],
    names: &FieldNames<'_>,
    docs: &FieldNames<'_>,
    offset: usize,
) -> (Vec<PyroField>, usize) {
    let mut fields = Vec::with_capacity(children.len());
    let mut pos = offset;
    for ct in children {
        let name = names.get(pos).to_owned();
        let doc = docs.get(pos).to_owned();
        pos += 1;
        let (child_human, consumed) = machine_type_to_human(ct, names, docs, pos);
        pos += consumed;
        fields.push(PyroField {
            name,
            doc,
            data_type: child_human,
        });
    }
    (fields, pos - offset)
}

/// Convert a machine `ChildField` to a human `PyroField`: its type read
/// against the shared flattened names, from the field's own offset.
fn child_field_to_human(cf: &ChildField<'_>) -> PyroField {
    let (data_type, consumed) = machine_type_to_human(
        cf.datatype,
        cf.child_names,
        cf.child_docs,
        cf.child_names_offset,
    );
    debug_assert_eq!(consumed, cf.child_names_len);
    PyroField {
        name: cf.name().to_owned(),
        doc: cf.doc().to_owned(),
        data_type,
    }
}

// =============================================================================
// From impls
// =============================================================================

impl<'a> From<&machine::PyroField<'a>> for PyroField {
    fn from(mf: &machine::PyroField<'a>) -> Self {
        let (dt, _) = machine_type_to_human(mf.datatype(), mf.child_names(), mf.child_docs(), 0);
        PyroField {
            name: mf.name().to_owned(),
            doc: mf.doc().to_owned(),
            data_type: dt,
        }
    }
}

impl<'a> From<machine::PyroField<'a>> for PyroField {
    fn from(mf: machine::PyroField<'a>) -> Self {
        PyroField::from(&mf)
    }
}

impl<'f> From<&ChildField<'f>> for PyroField {
    fn from(cf: &ChildField<'f>) -> Self {
        child_field_to_human(cf)
    }
}

impl<'a> From<&machine::PyroSchema<'a>> for PyroSchema {
    fn from(ms: &machine::PyroSchema<'a>) -> Self {
        let fields: Vec<PyroField> = ms.fields().map(|cf| child_field_to_human(&cf)).collect();
        PyroSchema {
            doc: ms.doc().to_owned(),
            fields,
        }
    }
}

impl<'a> From<machine::PyroSchema<'a>> for PyroSchema {
    fn from(ms: machine::PyroSchema<'a>) -> Self {
        PyroSchema::from(&ms)
    }
}

impl From<&PyroField> for machine::PyroField<'static> {
    fn from(hf: &PyroField) -> Self {
        hf.to_machine()
    }
}

impl From<PyroField> for machine::PyroField<'static> {
    fn from(hf: PyroField) -> Self {
        hf.to_machine()
    }
}

impl From<&PyroSchema> for machine::PyroSchema<'static> {
    fn from(hs: &PyroSchema) -> Self {
        hs.to_machine()
    }
}

impl From<PyroSchema> for machine::PyroSchema<'static> {
    fn from(hs: PyroSchema) -> Self {
        hs.to_machine()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // JSON roundtrips
    // -------------------------------------------------------------------------

    #[test]
    fn test_schema_json_roundtrip() {
        let schema = PyroSchema::new(vec![
            PyroField::new(
                "id",
                PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::I64,
                },
            ),
            PyroField::new("name", PyroType::Str),
            PyroField::new(
                "scores",
                PyroType::List {
                    element: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::F32,
                    }),
                },
            ),
            PyroField::new(
                "tags",
                PyroType::List {
                    element: Box::new(PyroType::Str),
                },
            ),
            PyroField::new(
                "embedding",
                PyroType::PrimitiveFixedList {
                    element: PrimitiveDataType::F32,
                    size: 128,
                },
            ),
        ]);

        let json = serde_json::to_string_pretty(&schema).unwrap();
        let deserialized: PyroSchema = serde_json::from_str(&json).unwrap();
        assert_eq!(schema, deserialized);
    }

    #[test]
    fn test_datatype_json_roundtrip() {
        let types = vec![
            PyroType::Unknown,
            PyroType::PrimitiveScalar {
                primitive: PrimitiveDataType::Bool,
            },
            PyroType::PrimitiveScalar {
                primitive: PrimitiveDataType::I32,
            },
            PyroType::PrimitiveScalar {
                primitive: PrimitiveDataType::F64,
            },
            PyroType::Str,
            PyroType::PrimitiveScalar {
                primitive: PrimitiveDataType::Timestamp,
            },
            PyroType::List {
                element: Box::new(PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                }),
            },
            PyroType::PrimitiveFixedList {
                element: PrimitiveDataType::F32,
                size: 16,
            },
            PyroType::List {
                element: Box::new(PyroType::Str),
            },
            PyroType::Group {
                fields: vec![PyroField::new(
                    "x",
                    PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::I32,
                    },
                )],
            },
            PyroType::Map {
                key: Box::new(PyroType::Str),
                value: Box::new(PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::I64,
                }),
            },
            PyroType::OptionPrimitiveScalar {
                primitive: PrimitiveDataType::F64,
            },
            PyroType::OptionPrimitiveFixedList {
                element: PrimitiveDataType::F32,
                size: 16,
            },
            PyroType::OptionPrimitiveScalar {
                primitive: PrimitiveDataType::Timestamp,
            },
            PyroType::OptionGroup {
                fields: vec![PyroField::new("a", PyroType::Str).with_doc("A string field")],
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
            PyroType::OptionPrimitiveScalar {
                primitive: PrimitiveDataType::U32,
            },
        )
        .with_doc("The age field");
        let json = serde_json::to_string(&field).unwrap();
        let back: PyroField = serde_json::from_str(&json).unwrap();
        assert_eq!(field, back);
        assert_eq!(back.doc, "The age field");
    }

    #[test]
    fn test_nested_group_json_roundtrip() {
        let schema = PyroSchema::new(vec![PyroField::new(
            "person",
            PyroType::Group {
                fields: vec![
                    PyroField::new("name", PyroType::Str),
                    PyroField::new(
                        "address",
                        PyroType::OptionGroup {
                            fields: vec![
                                PyroField::new("street", PyroType::Str),
                                PyroField::new("city", PyroType::Str),
                            ],
                        },
                    ),
                ],
            },
        )]);

        let json = serde_json::to_string_pretty(&schema).unwrap();
        let back: PyroSchema = serde_json::from_str(&json).unwrap();
        assert_eq!(schema, back);
    }

    #[test]
    fn test_list_of_group_json_roundtrip() {
        let dt = PyroType::List {
            element: Box::new(PyroType::Group {
                fields: vec![
                    PyroField::new(
                        "x",
                        PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::F64,
                        },
                    ),
                    PyroField::new(
                        "y",
                        PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::F64,
                        },
                    ),
                ],
            }),
        };

        let json = serde_json::to_string(&dt).unwrap();
        let back: PyroType = serde_json::from_str(&json).unwrap();
        assert_eq!(dt, back);
    }

    // -------------------------------------------------------------------------
    // human ↔ machine roundtrips
    // -------------------------------------------------------------------------

    #[test]
    fn test_human_to_machine_leaf() {
        let human = PyroField::new(
            "age",
            PyroType::OptionPrimitiveScalar {
                primitive: PrimitiveDataType::I32,
            },
        );
        let mf = human.to_machine();
        assert_eq!(mf.name(), "age");
        assert!(mf.datatype().is_nullable());
        assert_eq!(
            mf.datatype(),
            &machine::PyroType::OptionPrimitiveScalar(PrimitiveDataType::I32)
        );
    }

    #[test]
    fn test_human_to_machine_group() {
        let human = PyroField::new(
            "person",
            PyroType::Group {
                fields: vec![
                    PyroField::new("name", PyroType::Str),
                    PyroField::new(
                        "age",
                        PyroType::OptionPrimitiveScalar {
                            primitive: PrimitiveDataType::I32,
                        },
                    ),
                ],
            },
        );

        let mf = human.to_machine();
        assert_eq!(mf.name(), "person");
        assert!(!mf.datatype().is_nullable());
        assert_eq!(mf.num_children(), 2);

        let children: Vec<_> = mf.group_children().unwrap().collect();
        assert_eq!(children[0].name(), "name");
        assert!(!children[0].datatype().is_nullable());
        assert_eq!(children[1].name(), "age");
        assert!(children[1].datatype().is_nullable());
    }

    #[test]
    fn test_human_to_machine_option_group() {
        let human = PyroField::new(
            "meta",
            PyroType::OptionGroup {
                fields: vec![PyroField::new("key", PyroType::Str)],
            },
        );

        let mf = human.to_machine();
        assert!(mf.datatype().is_nullable());
        assert!(matches!(mf.datatype(), machine::PyroType::OptionGroup(_)));
        assert_eq!(mf.num_children(), 1);
    }

    #[test]
    fn test_human_to_machine_list_of_group() {
        let human = PyroField::new(
            "items",
            PyroType::List {
                element: Box::new(PyroType::Group {
                    fields: vec![
                        PyroField::new("x", PyroType::Str),
                        PyroField::new(
                            "y",
                            PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::I32,
                            },
                        ),
                    ],
                }),
            },
        );

        let mf = human.to_machine();
        assert_eq!(mf.child_names().len(), 2);

        let children: Vec<_> = mf.list_group_children().unwrap().collect();
        assert_eq!(children[0].name(), "x");
        assert_eq!(children[1].name(), "y");
    }

    #[test]
    fn test_machine_to_human_roundtrip() {
        let human_schema = PyroSchema::new(vec![
            PyroField::new(
                "id",
                PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::I64,
                },
            )
            .with_doc("Unique ID"),
            PyroField::new(
                "data",
                PyroType::OptionGroup {
                    fields: vec![
                        PyroField::new("key", PyroType::Str).with_doc("The key"),
                        PyroField::new(
                            "nested",
                            PyroType::Group {
                                fields: vec![
                                    PyroField::new("a", PyroType::Str).with_doc("A"),
                                    PyroField::new(
                                        "b",
                                        PyroType::OptionPrimitiveScalar {
                                            primitive: PrimitiveDataType::F64,
                                        },
                                    )
                                    .with_doc("B"),
                                ],
                            },
                        )
                        .with_doc("Nested group"),
                    ],
                },
            )
            .with_doc("Optional data"),
        ])
        .with_doc("A test schema");

        let machine_schema = human_schema.to_machine();
        let back: PyroSchema = PyroSchema::from(&machine_schema);

        assert_eq!(back.doc, "A test schema");
        assert_eq!(back.fields.len(), 2);
        assert_eq!(back.fields[0].name, "id");
        assert_eq!(back.fields[0].doc, "Unique ID");
        assert_eq!(back.fields[1].name, "data");
        assert_eq!(back.fields[1].doc, "Optional data");

        if let PyroType::OptionGroup { fields } = &back.fields[1].data_type {
            assert_eq!(fields[0].name, "key");
            assert_eq!(fields[0].doc, "The key");
            assert_eq!(fields[1].name, "nested");
            assert_eq!(fields[1].doc, "Nested group");
            if let PyroType::Group { fields: inner } = &fields[1].data_type {
                assert_eq!(inner[0].name, "a");
                assert_eq!(inner[0].doc, "A");
                assert_eq!(inner[1].name, "b");
                assert_eq!(inner[1].doc, "B");
                assert!(matches!(
                    inner[1].data_type,
                    PyroType::OptionPrimitiveScalar {
                        primitive: PrimitiveDataType::F64
                    }
                ));
            } else {
                panic!("Expected nested Group");
            }
        } else {
            panic!("Expected OptionGroup");
        }
    }

    #[test]
    fn test_machine_to_human_list_of_group_roundtrip() {
        let human = PyroField::new(
            "points",
            PyroType::List {
                element: Box::new(PyroType::Group {
                    fields: vec![
                        PyroField::new(
                            "lat",
                            PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::F64,
                            },
                        ),
                        PyroField::new(
                            "lon",
                            PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::F64,
                            },
                        ),
                    ],
                }),
            },
        );

        let mf = human.to_machine();
        let back: PyroField = PyroField::from(&mf);

        assert_eq!(back.name, "points");
        if let PyroType::List { element } = &back.data_type {
            if let PyroType::Group { fields } = element.as_ref() {
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].name, "lat");
                assert_eq!(fields[1].name, "lon");
            } else {
                panic!("Expected inner Group");
            }
        } else {
            panic!("Expected List");
        }
    }
}
