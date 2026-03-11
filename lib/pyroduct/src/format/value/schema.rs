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
use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{PrimitiveValueList, PyroRow, PyroValue};

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
            fields: self
                .fields
                .into_iter()
                .map(|f| f.clone().into_owned())
                .collect(),
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

// =============================================================================
// PyroType inference from PyroValue
// =============================================================================

impl<'a> PyroType<'a> {
    /// Infer the [`PyroType`] from a single [`PyroValue`].
    ///
    /// NOTE: This returns a `PyroType<'a>` where possible, but if field names
    /// need to be extracted from `Group` keys, they are cloned to Owned strings
    /// to avoid lifetime safety issues.
    pub fn from_value(value: &PyroValue<'a>) -> Self {
        match value {
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
                let pdt = PrimitiveDataType::from_primitive_list(pl);
                PyroType::PrimitiveList(pdt)
            }

            PyroValue::List(items) => {
                let mut has_null = false;
                let mut inner = PyroType::Null;

                for item in items {
                    if matches!(item, PyroValue::Null) {
                        has_null = true;
                    } else {
                        let item_dt = PyroType::from_value(item);
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
                        PyroField::new(Cow::Owned(k.to_string()), PyroType::from_value(v), true)
                    })
                    .collect();
                PyroType::Group(Cow::Owned(fields))
            }

            PyroValue::MapInternal(pairs) => {
                let mut key_dt = PyroType::Null;
                let mut val_dt = PyroType::Null;

                for (k, v) in pairs {
                    let k_type = PyroType::from_value(k);
                    let v_type = PyroType::from_value(v);

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

impl PrimitiveDataType {
    /// Extract the [`PrimitiveDataType`] from a [`PrimitiveValueList`].
    pub fn from_primitive_list(pl: &PrimitiveValueList<'_>) -> Self {
        match pl {
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

impl<'a> PyroSchema<'a> {
    /// Build a trusted schema from a single row (no coercion, nullable = true).
    pub fn trusted(row: &PyroRow<'a>) -> Result<Self, ValueSchemaInferenceError<'a>> {
        if row.is_empty() {
            return Err(ValueSchemaInferenceError::EmptyRow);
        }
        let fields = row
            .iter()
            .map(|(k, v)| {
                // Ensure safe lifetime handling by owning the string
                PyroField::new(Cow::Owned(k.to_string()), PyroType::from_value(v), true)
            })
            .collect();

        Ok(Self {
            documentation: None,
            fields,
        })
    }

    /// Infer a schema from multiple rows, with coercion and nullability tracking.
    pub fn infer(rows: &[PyroRow<'a>]) -> Result<Self, ValueSchemaInferenceError<'a>> {
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

        Ok(Self {
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

        let observed = PyroType::from_value(value);

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

/// Coerce two [`PyroType`]s to a common supertype. Returns `None` if incompatible.
fn coerce_pyro_types<'a>(a: &PyroType<'a>, b: &PyroType<'a>) -> Option<PyroType<'a>> {
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
            coerce_pyro_types(&inner_a, &inner_b).map(|c| List(Box::new(c), merged_null))
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
        assert_eq!(PyroType::from_value(&PyroValue::Null), PyroType::Null);
        assert_eq!(
            PyroType::from_value(&PyroValue::I32(42)),
            PyroType::PrimitiveScalar(PrimitiveDataType::I32)
        );
        assert_eq!(
            PyroType::from_value(&PyroValue::F64(1.0)),
            PyroType::PrimitiveScalar(PrimitiveDataType::F64)
        );
        assert_eq!(
            PyroType::from_value(&PyroValue::from("hello")),
            PyroType::Str
        );
    }

    #[test]
    fn test_data_type_from_value_list_nullable() {
        let list = PyroValue::List(vec![PyroValue::I32(1), PyroValue::Null, PyroValue::I32(3)]);
        assert_eq!(
            PyroType::from_value(&list),
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
            PyroType::from_value(&list),
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
        let schema = PyroType::from_value(&map_val);

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
        let schema = PyroSchema::trusted(&row).unwrap();
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
        let schema = PyroSchema::infer(&rows).unwrap();
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
        let schema = PyroSchema::infer(&rows).unwrap();
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
        let schema = PyroSchema::infer(&rows).unwrap();
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
        let schema = PyroSchema::infer(&rows).unwrap();
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
        let schema = PyroSchema::infer(&rows).unwrap();
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
        let err = PyroSchema::infer(&rows).unwrap_err();
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
