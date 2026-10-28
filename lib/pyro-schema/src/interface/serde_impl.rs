//! `#[serde(with)]` helpers for the interface types.
//!
//! [`FieldName`]'s own `Deserialize` borrows from the input, which would
//! tie a spec to the string it was read from; these always own. A
//! [`PyroSchema`] is written as its [`human`] form plus the word it is
//! laid out in, when that isn't the default.

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{FieldName, PyroSchema, Word, human};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum WordRepr {
    U16,
    U32,
}

#[derive(Serialize, Deserialize)]
struct SchemaRepr {
    #[serde(flatten)]
    schema: human::PyroSchema,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    word: Option<WordRepr>,
}

impl From<&PyroSchema<'_>> for SchemaRepr {
    fn from(schema: &PyroSchema<'_>) -> Self {
        let default = PyroSchema::new(Vec::new()).word();
        SchemaRepr {
            schema: schema.into(),
            word: (schema.word() != default).then_some(match schema.word() {
                Word::U16 => WordRepr::U16,
                Word::U32 => WordRepr::U32,
            }),
        }
    }
}

impl SchemaRepr {
    fn into_schema(self) -> PyroSchema<'static> {
        let mut schema = self.schema.to_machine();
        if let Some(word) = self.word {
            schema.set_word(match word {
                WordRepr::U16 => Word::U16,
                WordRepr::U32 => Word::U32,
            });
        }
        schema
    }
}

pub mod name {
    use super::*;

    pub fn serialize<S: Serializer>(name: &FieldName<'_>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(name.as_ref())
    }

    pub fn deserialize<'de, 'a, D: Deserializer<'de>>(d: D) -> Result<FieldName<'a>, D::Error> {
        String::deserialize(d).map(FieldName::Owned)
    }
}

pub mod opt_name {
    use super::*;

    pub fn serialize<S: Serializer>(name: &Option<FieldName<'_>>, s: S) -> Result<S::Ok, S::Error> {
        name.as_ref().map(|n| n.as_ref()).serialize(s)
    }

    pub fn deserialize<'de, 'a, D: Deserializer<'de>>(
        d: D,
    ) -> Result<Option<FieldName<'a>>, D::Error> {
        Option::<String>::deserialize(d).map(|n| n.map(FieldName::Owned))
    }
}

pub mod schema {
    use super::*;

    pub fn serialize<S: Serializer>(schema: &PyroSchema<'_>, s: S) -> Result<S::Ok, S::Error> {
        SchemaRepr::from(schema).serialize(s)
    }

    pub fn deserialize<'de, 'a, D: Deserializer<'de>>(d: D) -> Result<PyroSchema<'a>, D::Error> {
        SchemaRepr::deserialize(d).map(SchemaRepr::into_schema)
    }
}

pub mod opt_schema {
    use super::*;

    pub fn serialize<S: Serializer>(
        schema: &Option<PyroSchema<'_>>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        schema.as_ref().map(SchemaRepr::from).serialize(s)
    }

    pub fn deserialize<'de, 'a, D: Deserializer<'de>>(
        d: D,
    ) -> Result<Option<PyroSchema<'a>>, D::Error> {
        Option::<SchemaRepr>::deserialize(d).map(|s| s.map(SchemaRepr::into_schema))
    }
}

pub mod schema_map {
    use super::*;

    pub fn serialize<S: Serializer>(
        map: &BTreeMap<FieldName<'_>, PyroSchema<'_>>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        s.collect_map(map.iter().map(|(k, v)| (k.as_ref(), SchemaRepr::from(v))))
    }

    pub fn deserialize<'de, 'a, D: Deserializer<'de>>(
        d: D,
    ) -> Result<BTreeMap<FieldName<'a>, PyroSchema<'a>>, D::Error> {
        let map = BTreeMap::<String, SchemaRepr>::deserialize(d)?;
        Ok(map
            .into_iter()
            .map(|(k, v)| (FieldName::Owned(k), v.into_schema()))
            .collect())
    }
}
