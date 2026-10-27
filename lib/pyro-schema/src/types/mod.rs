#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt;

use crate::bow::Bow;

mod coerce;
mod field;
mod name;
mod schema;

#[cfg(feature = "human")]
pub mod human;

pub use coerce::coerce_pyro_types;
pub use field::{ChildField, ChildIter, PyroField};
pub use name::{FieldName, FieldNames};
pub use schema::PyroSchema;

// =============================================================================
// PyroType  (lifetime-free)
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PyroType {
    Unknown,
    PrimitiveScalar(PrimitiveDataType),
    Str,
    Bytes,

    PrimitiveList(PrimitiveDataType),
    PrimitiveFixedList(PrimitiveDataType, usize),
    /// `(element_type, element_nullable)`
    List(Bow<'static, PyroType>),
    /// A struct/row with N child types. Names stored externally.
    Group(Cow<'static, [PyroType]>),
    Map {
        key: Bow<'static, PyroType>,
        value: Bow<'static, PyroType>,
    },

    OptionPrimitiveScalar(PrimitiveDataType),
    OptionPrimitiveFixedList(PrimitiveDataType, usize),
    OptionGroup(Cow<'static, [PyroType]>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PrimitiveDataType {
    Bool,
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    F16,
    F32,
    F64,
    /// A `Time`: 16 bytes on the wire.
    Timestamp,
}

impl PyroType {
    /// Number of name slots this type requires in the external names array.
    ///
    /// Counts names for *all* Groups reachable from this type,
    /// including through List and Map.
    pub fn name_count(&self) -> usize {
        match self {
            PyroType::Unknown
            | PyroType::PrimitiveScalar(_)
            | PyroType::Str
            | PyroType::Bytes
            | PyroType::PrimitiveList(_)
            | PyroType::PrimitiveFixedList(_, _)
            | PyroType::OptionPrimitiveScalar(_)
            | PyroType::OptionPrimitiveFixedList(_, _) => 0,

            PyroType::Group(children) | PyroType::OptionGroup(children) => {
                children.len() + children.iter().map(|c| c.name_count()).sum::<usize>()
            }

            PyroType::List(inner) => inner.name_count(),

            PyroType::Map { key, value } => key.name_count() + value.name_count(),
        }
    }

    /// Whether this type represents a nullable value.
    pub fn is_nullable(&self) -> bool {
        matches!(
            self,
            PyroType::Unknown
                | PyroType::OptionPrimitiveScalar(_)
                | PyroType::OptionPrimitiveFixedList(_, _)
                | PyroType::OptionGroup(_)
        )
    }

    /// Return the non-nullable equivalent of this type.
    /// For already non-nullable types, returns a clone.
    pub fn to_required(&self) -> PyroType {
        match self {
            PyroType::OptionPrimitiveScalar(p) => PyroType::PrimitiveScalar(*p),
            PyroType::OptionPrimitiveFixedList(p, size) => PyroType::PrimitiveFixedList(*p, *size),
            PyroType::OptionGroup(children) => PyroType::Group(children.clone()),
            other => other.clone(),
        }
    }

    /// Return the nullable equivalent of this type.
    /// For already nullable types, returns a clone.
    pub fn to_nullable(&self) -> PyroType {
        match self {
            PyroType::PrimitiveScalar(p) => PyroType::OptionPrimitiveScalar(*p),
            PyroType::PrimitiveFixedList(p, size) => PyroType::OptionPrimitiveFixedList(*p, *size),
            PyroType::Group(children) => PyroType::OptionGroup(children.clone()),
            PyroType::OptionPrimitiveScalar(_)
            | PyroType::OptionPrimitiveFixedList(_, _)
            | PyroType::OptionGroup(_)
            | PyroType::Unknown => self.clone(),
            // Str, Lists, Map are reference types — inherently nullable in most
            // runtimes. Return as-is.
            other => other.clone(),
        }
    }

    /// Return the inner children slice for Group or OptionGroup.
    pub fn group_children_types(&self) -> Option<&[PyroType]> {
        match self {
            PyroType::Group(c) | PyroType::OptionGroup(c) => Some(c),
            _ => None,
        }
    }

    /// [`Self::group_children_types`], but seeing through any number of
    /// `List` layers first — `List<Group<..>>` names its fields exactly as a
    /// bare `Group` does, and [`Self::name_count`] already counts them that
    /// way.
    pub fn list_group_children_types(&self) -> Option<&[PyroType]> {
        let mut ty = self;
        while let PyroType::List(inner) = ty {
            ty = inner.as_ref();
        }
        ty.group_children_types()
    }

    /// Unwrap any number of `List` layers (including zero) and, if the base
    /// type is `Group`/`OptionGroup`, return `(list_nesting, nullable,
    /// child_types)`. Returns `None` if the base type isn't a Group.
    pub fn list_group_nesting(&self) -> Option<(usize, bool, &[PyroType])> {
        let mut nesting = 0;
        let mut ty = self;
        while let PyroType::List(inner) = ty {
            nesting += 1;
            ty = inner.as_ref();
        }
        match ty {
            PyroType::Group(c) => Some((nesting, false, c)),
            PyroType::OptionGroup(c) => Some((nesting, true, c)),
            _ => None,
        }
    }
}

// =============================================================================
// Display
// =============================================================================

impl fmt::Display for PrimitiveDataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl fmt::Display for PyroType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PyroType::Unknown => write!(f, "Null"),
            PyroType::PrimitiveScalar(t) => write!(f, "{t}"),
            PyroType::Str => write!(f, "Str"),
            PyroType::Bytes => write!(f, "Bytes"),
            PyroType::PrimitiveList(t) => write!(f, "[{t}]"),
            PyroType::PrimitiveFixedList(t, n) => write!(f, "[{t}; {n}]"),
            PyroType::List(inner) => write!(f, "[{inner}]"),
            PyroType::Group(children) => write!(f, "Group({} fields)", children.len()),
            PyroType::Map { key, value } => write!(f, "Map<{key}, {value}>"),

            PyroType::OptionPrimitiveScalar(t) => write!(f, "Option{t}"),
            PyroType::OptionPrimitiveFixedList(t, n) => write!(f, "Option[{t}; {n}]"),
            PyroType::OptionGroup(children) => write!(f, "OptionGroup({} fields)", children.len()),
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::*;

    const CONST_TYPE: PyroType = PyroType::Group(Cow::Borrowed(&[
        PyroType::Str,
        PyroType::List(Bow::Borrowed(&PyroType::PrimitiveScalar(
            PrimitiveDataType::U8,
        ))),
        PyroType::Map {
            key: Bow::Borrowed(&PyroType::Str),
            value: Bow::Borrowed(&PyroType::PrimitiveScalar(PrimitiveDataType::Timestamp)),
        },
    ]));

    #[test]
    fn const_pyro_type_equals_owned() {
        let owned = PyroType::Group(
            vec![
                PyroType::Str,
                PyroType::List(Bow::new(PyroType::PrimitiveScalar(PrimitiveDataType::U8))),
                PyroType::Map {
                    key: Bow::new(PyroType::Str),
                    value: Bow::new(PyroType::PrimitiveScalar(PrimitiveDataType::Timestamp)),
                },
            ]
            .into(),
        );
        assert_eq!(CONST_TYPE, owned);
        assert_eq!(CONST_TYPE.name_count(), owned.name_count());
    }

    #[test]
    fn test_leaf_field() {
        let f = PyroField::new("age", PyroType::PrimitiveScalar(PrimitiveDataType::I32));
        assert_eq!(f.name(), "age");
        assert!(f.child_names().is_empty());
        assert!(f.group_children().is_none());
    }

    #[test]
    fn test_group_field() {
        let f = PyroField::group(
            "person",
            vec![
                PyroField::new("name", PyroType::Str),
                PyroField::new("age", PyroType::PrimitiveScalar(PrimitiveDataType::I32)),
            ],
        );

        assert_eq!(f.name(), "person");
        assert_eq!(f.child_names().len(), 2);
        assert_eq!(f.num_children(), 2);

        let children: Vec<_> = f.group_children().unwrap().collect();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].name(), "name");
        assert_eq!(children[0].datatype(), &PyroType::Str);
        assert_eq!(children[1].name(), "age");

        assert_eq!(f.group_child(0).name(), "name");
        assert_eq!(f.group_child(1).name(), "age");
    }

    #[test]
    fn test_nested_group() {
        let address = PyroField::group(
            "address",
            vec![
                PyroField::new("street", PyroType::Str),
                PyroField::new("city", PyroType::Str),
            ],
        );

        let person = PyroField::group(
            "person",
            vec![
                PyroField::new("name", PyroType::Str),
                address,
                PyroField::new("score", PyroType::PrimitiveScalar(PrimitiveDataType::F64)),
            ],
        );

        assert_eq!(person.child_names().len(), 5);

        let addr = person.group_child(1);
        assert_eq!(addr.name(), "address");

        let sub: Vec<_> = addr.group_children().unwrap().collect();
        assert_eq!(sub.len(), 2);
        assert_eq!(sub[0].name(), "street");
        assert_eq!(sub[1].name(), "city");

        // Random access into nested
        assert_eq!(addr.group_child(0).name(), "street");
        assert_eq!(addr.group_child(1).name(), "city");
    }

    #[test]
    fn test_list_of_group() {
        let inner = PyroType::Group(
            vec![
                PyroType::Str,
                PyroType::PrimitiveScalar(PrimitiveDataType::I32),
            ]
            .into(),
        );
        let list_type = PyroType::List(Bow::new(inner));
        assert_eq!(list_type.name_count(), 2);

        let f = PyroField::with_children(
            "items",
            FieldNames::Owned(vec![Cow::Borrowed("x"), Cow::Borrowed("y")]),
            list_type,
        );

        assert_eq!(f.name(), "items");
        let children: Vec<_> = f.list_group_children().unwrap().collect();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].name(), "x");
        assert_eq!(children[1].name(), "y");
    }

    #[test]
    fn test_list_of_nested_group() {
        let coords = PyroType::Group(
            vec![
                PyroType::PrimitiveScalar(PrimitiveDataType::F64),
                PyroType::PrimitiveScalar(PrimitiveDataType::F64),
            ]
            .into(),
        );
        let outer = PyroType::Group(vec![PyroType::Str, coords].into());
        let list_type = PyroType::List(Bow::new(outer));
        assert_eq!(list_type.name_count(), 4);

        let f = PyroField::with_children(
            "points",
            FieldNames::Owned(vec![
                Cow::Borrowed("label"),
                Cow::Borrowed("coords"),
                Cow::Borrowed("lat"),
                Cow::Borrowed("lon"),
            ]),
            list_type,
        );

        let outer_children: Vec<_> = f.list_group_children().unwrap().collect();
        assert_eq!(outer_children[0].name(), "label");
        assert_eq!(outer_children[1].name(), "coords");

        let coord_children: Vec<_> = outer_children[1].group_children().unwrap().collect();
        assert_eq!(coord_children[0].name(), "lat");
        assert_eq!(coord_children[1].name(), "lon");
    }

    #[test]
    fn test_static_schema() {
        static NAMES: &[&str] = &["x", "y", "z"];

        let schema = PyroSchema::from_raw(
            FieldNames::Static(NAMES),
            vec![PyroType::Str, PyroType::Str, PyroType::Str],
            crate::wire::InodeMeta::DEFAULT,
        );

        assert_eq!(schema.num_fields(), 3);
        assert_eq!(schema.field(0).name(), "x");
        assert_eq!(schema.field(2).name(), "z");

        let names: Vec<&str> = schema.fields().map(|f| f.name()).collect();
        assert_eq!(names, vec!["x", "y", "z"]);
    }

    #[test]
    fn test_schema_basic() {
        let schema = PyroSchema::new(vec![
            PyroField::new("id", PyroType::PrimitiveScalar(PrimitiveDataType::I64)),
            PyroField::new("value", PyroType::Str),
        ]);

        assert_eq!(schema.num_fields(), 2);
        assert_eq!(schema.field(0).name(), "id");
        assert_eq!(schema.field(1).name(), "value");
        assert_eq!(schema.index_of("value"), Some(1));
        assert_eq!(schema.index_of("missing"), None);
    }

    #[test]
    fn test_schema_with_group() {
        let schema = PyroSchema::new(vec![
            PyroField::new("id", PyroType::PrimitiveScalar(PrimitiveDataType::I64)),
            PyroField::group(
                "meta",
                vec![
                    PyroField::new("key", PyroType::Str),
                    PyroField::new("val", PyroType::Str),
                ],
            ),
            PyroField::new("score", PyroType::PrimitiveScalar(PrimitiveDataType::F64)),
        ]);

        assert_eq!(schema.names().len(), 5);
        assert_eq!(schema.num_fields(), 3);

        let meta = schema.field(1);
        assert_eq!(meta.name(), "meta");
        let children: Vec<_> = meta.group_children().unwrap().collect();
        assert_eq!(children[0].name(), "key");
        assert_eq!(children[1].name(), "val");

        assert_eq!(schema.field(2).name(), "score");
    }

    #[test]
    fn test_schema_iterate() {
        let schema = PyroSchema::new(vec![
            PyroField::new("a", PyroType::Str),
            PyroField::new("b", PyroType::Str),
            PyroField::new("c", PyroType::Str),
        ]);

        let names: Vec<&str> = schema.fields().map(|f| f.name()).collect();
        assert_eq!(names, vec!["a", "b", "c"]);
        assert_eq!(schema.fields().len(), 3);
    }

    #[test]
    fn test_name_count() {
        assert_eq!(PyroType::Unknown.name_count(), 0);
        assert_eq!(PyroType::Str.name_count(), 0);

        let g = PyroType::Group(vec![PyroType::Str, PyroType::Str].into());
        assert_eq!(g.name_count(), 2);

        let l = PyroType::List(Bow::new(PyroType::Str));
        assert_eq!(l.name_count(), 0);

        let lg = PyroType::List(Bow::new(g.clone()));
        assert_eq!(lg.name_count(), 2);

        let m = PyroType::Map {
            key: Bow::new(PyroType::Str),
            value: Bow::new(g),
        };
        assert_eq!(m.name_count(), 2);
    }

    #[test]
    fn test_to_owned_field() {
        let f = PyroField::group(
            "root",
            vec![
                PyroField::new("a", PyroType::Str),
                PyroField::new("b", PyroType::Str),
            ],
        );

        let child = f.group_child(0);
        let owned: PyroField<'static> = child.to_owned_field();
        assert_eq!(owned.name(), "a");
        assert_eq!(owned.datatype(), &PyroType::Str);
    }

    #[test]
    fn test_map_of_groups() {
        let key_group = PyroType::Group(
            vec![
                PyroType::Str,
                PyroType::PrimitiveScalar(PrimitiveDataType::I32),
            ]
            .into(),
        );
        let val_group =
            PyroType::Group(vec![PyroType::PrimitiveScalar(PrimitiveDataType::F64)].into());
        let map_type = PyroType::Map {
            key: Bow::new(key_group),
            value: Bow::new(val_group),
        };

        // Name list for map components: [k_name1, k_name2, v_name1]
        // Actually: Map { key: Group([Str, I32]), value: Group([F64]) }
        // key.name_count() = 2 (for Str, I32)
        // value.name_count() = 1 (for F64)
        let f = PyroField::with_children(
            "m",
            FieldNames::Owned(vec![
                Cow::Borrowed("kx"),
                Cow::Borrowed("ky"),
                Cow::Borrowed("vz"),
            ]),
            map_type,
        );

        let k_children: Vec<_> = f.map_key_group_children().unwrap().collect();
        assert_eq!(k_children.len(), 2);
        assert_eq!(k_children[0].name(), "kx");
        assert_eq!(k_children[1].name(), "ky");

        let v_children: Vec<_> = f.map_value_group_children().unwrap().collect();
        assert_eq!(v_children.len(), 1);
        assert_eq!(v_children[0].name(), "vz");
    }

    #[test]
    #[cfg(feature = "human")]
    fn test_human_map_of_groups() {
        use super::human::{
            PyroField as HumanField, PyroSchema as HumanSchema, PyroType as HumanType,
        };

        let human_schema = HumanSchema::new(vec![HumanField::new(
            "complex_map",
            HumanType::Map {
                key: Box::new(HumanType::Group {
                    fields: vec![HumanField::new("k1", HumanType::Str)],
                }),
                value: Box::new(HumanType::Group {
                    fields: vec![HumanField::new("v1", HumanType::Str)],
                }),
            },
        )]);

        let machine_schema = human_schema.to_machine();
        let back: HumanSchema = HumanSchema::from(&machine_schema);

        assert_eq!(human_schema, back);
    }
}
