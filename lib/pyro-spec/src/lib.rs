//! Pyro-native type system and schema representation.
//!
//! This crate provides a lightweight schema representation optimized for
//! `PyroValue`. Unlike Arrow's `DataType` (which has ~40 variants for timestamps,
//! decimals, run-end-encoded, etc.), `PyroType` mirrors *exactly* the variants that
//! `PyroValue` can represent, making match arms exhaustive and tiny.
//!
//! Core types:
//! - [`PyroType`] — the main type enum, mirroring `PyroValue` discriminants 1:1
//! - [`PyroField`] — a named, nullable column descriptor (equivalent to Arrow `Field`)
//! - [`PyroSchema`] — an ordered collection of `PyroField`s (equivalent to Arrow `Schema`)
//! - [`coerce_pyro_types`] — type coercion to find common supertypes
//!
//! Conversion to/from `arrow::datatypes::DataType` lives in the `arrow` module
//! (behind the `arrow` feature flag).

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

#[cfg(feature = "arrow")]
mod arrow;

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// The kind of module execution model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ModuleKind {
    #[default]
    Normal,
    Session,
    SessionDiff,
}

/// Documentation for the main function of a module
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModuleFunc<'a> {
    pub name: Cow<'a, str>,
    pub description: Option<Cow<'a, str>>,
    pub input: PyroSchema<'a>,
    pub output: PyroSchema<'a>,
    #[serde(default)]
    pub kind: ModuleKind,
}

/// The root specification object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterfaceSpec<'a> {
    pub capability: Cow<'a, str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Cow<'a, str>>,

    pub classes: Vec<ClassSpec<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassSpec<'a> {
    pub name: Cow<'a, str>,
    pub description: Option<Cow<'a, str>>,
    pub methods: Vec<CapabilityFunc<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client: Option<PyroSchema<'a>>,
    pub config: Option<PyroSchema<'a>>,
}

/// Documentation for a capability function
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CapabilityFunc<'a> {
    pub name: Cow<'a, str>,
    pub description: Option<Cow<'a, str>>,
    pub input: PyroSchema<'a>,
    pub output: PyroType<'a>,
}

// =============================================================================
// PyroType
// =============================================================================

/// A data type enum that mirrors the variants of [`PyroValue`] exactly.
///
/// This is intentionally much smaller than `arrow::datatypes::DataType`.
/// Every variant here has a 1:1 correspondence with a `PyroValue` discriminant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PyroType<'a> {
    /// No value / unknown type (corresponds to `PyroValue::Null`).
    Null,
    /// Scalar primitive (Bool, Int, Float).
    PrimitiveScalar(PrimitiveDataType),
    /// UTF-8 string (corresponds to `PyroValue::Str`).
    Str,
    /// Day + millisecond interval (corresponds to `PyroValue::Timestamp`).
    Timestamp,
    /// Homogeneous list of a single primitive type (corresponds to `PyroValue::PrimitiveList`).
    PrimitiveList(PrimitiveDataType),
    /// Fixed-size homogeneous list of a single primitive type.
    PrimitiveFixedList(PrimitiveDataType, usize),
    /// Heterogeneous list of arbitrary pyro values (corresponds to `PyroValue::List`).
    ///
    /// Fields: `(element_type, element_nullable)`.
    List(Box<PyroType<'a>>, bool),
    /// Named struct / row (corresponds to `PyroValue::Group`).
    Group(Cow<'a, [PyroField<'a>]>),
    /// Key-value map (corresponds to `PyroValue::MapInternal`).
    Map {
        key: Box<PyroType<'a>>,
        value: Box<PyroType<'a>>,
    },
}

impl<'a> fmt::Display for PyroType<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Primitives
            PyroType::Null => write!(f, "Null"),
            PyroType::PrimitiveScalar(t) => write!(f, "{}", t),
            PyroType::Str => write!(f, "Str"),
            PyroType::Timestamp => write!(f, "Timestamp"),

            // Complex Types
            PyroType::PrimitiveList(inner_type) => {
                write!(f, "[{}]", inner_type)
            }
            PyroType::PrimitiveFixedList(inner_type, len) => {
                write!(f, "[{}; {}]", inner_type, len)
            }
            PyroType::List(inner_type, _nullable) => {
                write!(f, "[{}]", inner_type)
            }
            PyroType::Group(fields) => {
                write!(f, "{{ ")?;
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", field.name, field.data_type)?;
                }
                write!(f, " }}")
            }
            PyroType::Map { key, value } => {
                write!(f, "Map<{}, {}>", key, value)
            }
        }
    }
}

/// The primitive element type inside a `PrimitiveValueList`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
}

impl fmt::Display for PrimitiveDataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrimitiveDataType::Bool => write!(f, "Bool"),
            PrimitiveDataType::U8 => write!(f, "U8"),
            PrimitiveDataType::U16 => write!(f, "U16"),
            PrimitiveDataType::U32 => write!(f, "U32"),
            PrimitiveDataType::U64 => write!(f, "U64"),
            PrimitiveDataType::I8 => write!(f, "I8"),
            PrimitiveDataType::I16 => write!(f, "I16"),
            PrimitiveDataType::I32 => write!(f, "I32"),
            PrimitiveDataType::I64 => write!(f, "I64"),
            PrimitiveDataType::F16 => write!(f, "F16"),
            PrimitiveDataType::F32 => write!(f, "F32"),
            PrimitiveDataType::F64 => write!(f, "F64"),
        }
    }
}

impl<'a> PyroType<'a> {
    pub fn into_owned(self) -> PyroType<'static> {
        match self {
            PyroType::Null => PyroType::Null,
            PyroType::PrimitiveScalar(p) => PyroType::PrimitiveScalar(p),
            PyroType::Str => PyroType::Str,
            PyroType::Timestamp => PyroType::Timestamp,
            PyroType::PrimitiveList(p) => PyroType::PrimitiveList(p),
            PyroType::PrimitiveFixedList(p, l) => PyroType::PrimitiveFixedList(p, l),
            PyroType::List(inner, n) => PyroType::List(Box::new(inner.into_owned()), n),
            PyroType::Group(fields) => {
                let owned_fields: Vec<PyroField<'static>> =
                    fields.iter().map(|f| f.clone().into_owned()).collect();
                PyroType::Group(Cow::Owned(owned_fields))
            }
            PyroType::Map { key, value } => PyroType::Map {
                key: Box::new(key.into_owned()),
                value: Box::new(value.into_owned()),
            },
        }
    }
}

// =============================================================================
// PyroField
// =============================================================================

/// A named, nullable column descriptor — the Pyro equivalent of `arrow::datatypes::Field`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PyroField<'a> {
    pub name: Cow<'a, str>,
    pub documentation: Option<Cow<'a, str>>,
    pub data_type: PyroType<'a>,
    pub nullable: bool,
}

impl<'a> PyroField<'a> {
    /// Create a new field.
    /// Accepts `&'static str`, `String`, or `Cow<'a, str>`.
    pub fn new(name: impl Into<Cow<'a, str>>, data_type: PyroType<'a>, nullable: bool) -> Self {
        Self {
            name: name.into(),
            documentation: None,
            data_type,
            nullable,
        }
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    pub fn data_type(&self) -> &PyroType<'a> {
        &self.data_type
    }

    #[inline]
    pub fn is_nullable(&self) -> bool {
        self.nullable
    }

    pub fn with_nullable(mut self, nullable: bool) -> Self {
        self.nullable = nullable;
        self
    }

    /// Convert to an owned version (PyroField<'static>) by cloning data.
    pub fn into_owned(self) -> PyroField<'static> {
        PyroField {
            name: Cow::Owned(self.name.into_owned()),
            documentation: self.documentation.map(|d| Cow::Owned(d.into_owned())),
            data_type: self.data_type.into_owned(),
            nullable: self.nullable,
        }
    }

    pub fn add_docstring(mut self, doc: impl Into<Cow<'a, str>>) -> Self {
        self.documentation = Some(doc.into());
        self
    }
}

impl<'a> fmt::Display for PyroField<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {:?}{}",
            self.name,
            self.data_type,
            if self.nullable { " (nullable)" } else { "" }
        )
    }
}

// =============================================================================
// PyroSchema
// =============================================================================

/// An ordered collection of [`PyroField`]s — the Pyro equivalent of `arrow::datatypes::Schema`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PyroSchema<'a> {
    pub documentation: Option<Cow<'a, str>>,
    pub fields: Cow<'a, [PyroField<'a>]>,
}

impl<'a> PyroSchema<'a> {
    pub fn new(fields: Vec<PyroField<'a>>) -> Self {
        Self {
            documentation: None,
            fields: Cow::Owned(fields),
        }
    }

    pub fn empty() -> Self {
        Self {
            documentation: None,
            fields: Cow::Owned(Vec::new()),
        }
    }

    #[inline]
    pub fn fields(&self) -> &[PyroField<'a>] {
        &self.fields
    }

    #[inline]
    pub fn num_fields(&self) -> usize {
        self.fields.len()
    }

    /// Look up a field by name (linear scan).
    pub fn field_with_name(&self, name: &str) -> Option<&PyroField<'a>> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// Get a field by index.
    pub fn field(&self, index: usize) -> &PyroField<'a> {
        &self.fields[index]
    }

    /// Returns column index for the given name, if present.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|f| f.name == name)
    }

    /// Convert to an fully owned schema (useful for inference results).
    pub fn into_owned(self) -> PyroSchema<'static> {
        PyroSchema {
            documentation: None,
            fields: self.fields.iter().map(|f| f.clone().into_owned()).collect(),
        }
    }

    pub fn add_docstring(mut self, doc: impl Into<Cow<'a, str>>) -> Self {
        self.documentation = Some(doc.into());
        self
    }
}

impl<'a> fmt::Display for PyroSchema<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "PyroSchema {{")?;
        for field in self.fields.iter() {
            writeln!(f, "  {field},")?;
        }
        write!(f, "}}")
    }
}

impl<'a> From<Vec<PyroField<'a>>> for PyroSchema<'a> {
    fn from(fields: Vec<PyroField<'a>>) -> Self {
        Self::new(fields)
    }
}

/// Coerce two [`PyroType`]s to a common supertype. Returns `None` if incompatible.
pub fn coerce_pyro_types<'a>(a: &PyroType<'a>, b: &PyroType<'a>) -> Option<PyroType<'a>> {
    if a == b {
        return Some(a.clone());
    }

    use PyroType::*;

    match (a, b) {
        // Null widens to anything
        (Null, other) | (other, Null) => Some(other.clone()),

        // --- Primitive Scalar coercion ---
        (PrimitiveScalar(pa), PrimitiveScalar(pb)) => {
            coerce_primitive_types(*pa, *pb).map(PrimitiveScalar)
        }

        // --- List coercion (merge nullability) ---
        (List(inner_a, null_a), List(inner_b, null_b)) => {
            let merged_null = *null_a || *null_b;
            coerce_pyro_types(inner_a, inner_b).map(|c| List(Box::new(c), merged_null))
        }

        // --- PrimitiveList coercion ---
        (PrimitiveList(pa), PrimitiveList(pb)) => {
            coerce_primitive_types(*pa, *pb).map(PrimitiveList)
        }

        // --- PrimitiveFixedList coercion ---
        // Same size + coercible element type → PrimitiveFixedList
        // Different size → promote to PrimitiveList
        (PrimitiveFixedList(pa, sa), PrimitiveFixedList(pb, sb)) => {
            let coerced_elem = coerce_primitive_types(*pa, *pb)?;
            if sa == sb {
                Some(PrimitiveFixedList(coerced_elem, *sa))
            } else {
                Some(PrimitiveList(coerced_elem))
            }
        }

        // PrimitiveFixedList + PrimitiveList → PrimitiveList
        (PrimitiveFixedList(pa, _), PrimitiveList(pb))
        | (PrimitiveList(pa), PrimitiveFixedList(pb, _)) => {
            coerce_primitive_types(*pa, *pb).map(PrimitiveList)
        }

        // --- Group (struct) coercion: merge fields ---
        (Group(fields_a), Group(fields_b)) => {
            let mut merged_map: BTreeMap<String, PyroField> = BTreeMap::new();

            for f in fields_a.iter().chain(fields_b.iter()) {
                match merged_map.get(f.name()) {
                    None => {
                        // Field only in one side so far — mark nullable since the other side lacks it
                        merged_map.insert(
                            f.name().to_string(),
                            PyroField::new(
                                Cow::Owned(f.name().to_string()),
                                f.data_type().clone(),
                                true,
                            ),
                        );
                    }
                    Some(existing) => {
                        let coerced = coerce_pyro_types(existing.data_type(), f.data_type())?;
                        let nullable = existing.is_nullable() || f.is_nullable();
                        merged_map.insert(
                            f.name().to_string(),
                            PyroField::new(Cow::Owned(f.name().to_string()), coerced, nullable),
                        );
                    }
                }
            }

            Some(Group(Cow::Owned(merged_map.into_values().collect())))
        }

        // --- Map Coercion ---
        (Map { key: ka, value: va }, Map { key: kb, value: vb }) => {
            let coerced_key = coerce_pyro_types(ka, kb)?;
            let coerced_val = coerce_pyro_types(va, vb)?;
            Some(Map {
                key: Box::new(coerced_key),
                value: Box::new(coerced_val),
            })
        }

        _ => None,
    }
}

fn coerce_primitive_types(a: PrimitiveDataType, b: PrimitiveDataType) -> Option<PrimitiveDataType> {
    if a == b {
        return Some(a);
    }

    use PrimitiveDataType as P;

    match (a, b) {
        (P::I8, P::I16) | (P::I16, P::I8) => Some(P::I16),
        (P::I8, P::I32) | (P::I32, P::I8) => Some(P::I32),
        (P::I8, P::I64) | (P::I64, P::I8) => Some(P::I64),
        (P::I16, P::I32) | (P::I32, P::I16) => Some(P::I32),
        (P::I16, P::I64) | (P::I64, P::I16) => Some(P::I64),
        (P::I32, P::I64) | (P::I64, P::I32) => Some(P::I64),

        (P::U8, P::U16) | (P::U16, P::U8) => Some(P::U16),
        (P::U8, P::U32) | (P::U32, P::U8) => Some(P::U32),
        (P::U8, P::U64) | (P::U64, P::U8) => Some(P::U64),
        (P::U16, P::U32) | (P::U32, P::U16) => Some(P::U32),
        (P::U16, P::U64) | (P::U64, P::U16) => Some(P::U64),
        (P::U32, P::U64) | (P::U64, P::U32) => Some(P::U64),

        (P::F16, P::F32) | (P::F32, P::F16) => Some(P::F32),
        (P::F32, P::F64) | (P::F64, P::F32) => Some(P::F64),
        (P::F16, P::F64) | (P::F64, P::F16) => Some(P::F64),

        // --- Int to Float promotion ---
        (P::I8 | P::I16 | P::I32 | P::I64, P::F64) | (P::F64, P::I8 | P::I16 | P::I32 | P::I64) => {
            Some(P::F64)
        }
        (P::U8 | P::U16 | P::U32 | P::U64, P::F64) | (P::F64, P::U8 | P::U16 | P::U32 | P::U64) => {
            Some(P::F64)
        }

        _ => None,
    }
}
