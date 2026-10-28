//! The function/interface spec — interfaces, classes, request/reply
//! signatures. May use `crate::types` and `crate::bridge`, never
//! `crate::wire` directly.
//!
//! Every type here round-trips through JSON. Schemas are written in their
//! [`human`](crate::human) form, and names always deserialize owned, so a
//! spec read from a borrowed string can still be `'static`.

mod serde_impl;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{FieldName, PyroSchema};

/// How a module is called.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleKind {
    /// One input, one output.
    #[default]
    Normal,
    /// Also given every prior output of the session: `(prior, input)`.
    Session,
    /// Also given every prior input and output: `(prior_input, prior_output,
    /// input)`.
    SessionDiff,
}

/// Documentation for the main function of a module
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleFunc<'a> {
    #[serde(with = "serde_impl::name")]
    pub name: FieldName<'a>,
    #[serde(default, with = "serde_impl::opt_name")]
    pub description: Option<FieldName<'a>>,
    #[serde(with = "serde_impl::schema")]
    pub input: PyroSchema<'a>,
    #[serde(with = "serde_impl::schema")]
    pub output: PyroSchema<'a>,
    #[serde(default)]
    pub kind: ModuleKind,
}

/// The root specification object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterfaceSpec<'a> {
    #[serde(with = "serde_impl::name")]
    pub capability: FieldName<'a>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_impl::opt_name"
    )]
    pub description: Option<FieldName<'a>>,
    pub classes: Vec<ClassSpec<'a>>,
    /// Every struct the interface declares, by name, so a module built
    /// against it can describe the types it takes from it.
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        with = "serde_impl::schema_map"
    )]
    pub structs: BTreeMap<FieldName<'a>, PyroSchema<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassSpec<'a> {
    #[serde(with = "serde_impl::name")]
    pub name: FieldName<'a>,
    #[serde(default, with = "serde_impl::opt_name")]
    pub description: Option<FieldName<'a>>,
    pub methods: Vec<CapabilityFunc<'a>>,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_impl::opt_schema"
    )]
    pub client: Option<PyroSchema<'a>>,
    #[serde(default, with = "serde_impl::opt_schema")]
    pub config: Option<PyroSchema<'a>>,
}

/// Documentation for a capability function
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityFunc<'a> {
    #[serde(with = "serde_impl::name")]
    pub name: FieldName<'a>,
    #[serde(default, with = "serde_impl::opt_name")]
    pub description: Option<FieldName<'a>>,
    #[serde(with = "serde_impl::schema")]
    pub input: PyroSchema<'a>,
    /// The `Ok` value as a single field, `output`, so a struct keeps its
    /// field names; empty for `()`.
    #[serde(with = "serde_impl::schema")]
    pub output: PyroSchema<'a>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FieldNames, PrimitiveDataType, PyroField, PyroType, Word};

    fn spec() -> InterfaceSpec<'static> {
        let point = PyroType::Group(
            vec![
                PyroType::PrimitiveScalar(PrimitiveDataType::F32),
                PyroType::PrimitiveScalar(PrimitiveDataType::F32),
            ]
            .into(),
        );
        let path = PyroField::with_children_and_docs(
            "path",
            FieldNames::from(vec!["x".to_string(), "y".to_string()]),
            FieldNames::from(vec!["Across".to_string(), String::new()]),
            PyroType::List(crate::Bow::new(point)),
        )
        .with_doc("Where to go");
        let mut config = PyroSchema::new(vec![PyroField::new(
            "speed",
            PyroType::PrimitiveScalar(PrimitiveDataType::U16),
        )])
        .with_doc("How to go");
        config.set_word(Word::U16);

        InterfaceSpec {
            capability: "nav".into(),
            description: Some("Navigation".into()),
            classes: vec![ClassSpec {
                name: "Driver".into(),
                description: None,
                methods: vec![CapabilityFunc {
                    name: "drive".into(),
                    description: Some("Drive a path".into()),
                    input: PyroSchema::new(vec![path]),
                    output: PyroSchema::new(vec![]),
                }],
                client: None,
                config: Some(config.clone()),
            }],
            structs: BTreeMap::from([("Config".into(), config)]),
        }
    }

    #[test]
    fn interface_round_trips_through_json() {
        let spec = spec();
        let json = serde_json::to_string_pretty(&spec).unwrap();
        // Read from a short-lived string, as a spec from disk is.
        let back: InterfaceSpec<'static> = serde_json::from_str(&json.clone()).unwrap();
        assert_eq!(back, spec);
    }

    #[test]
    fn module_func_kind_defaults_to_normal() {
        let json = r#"{
            "name": "main",
            "input": {"doc": "", "fields": []},
            "output": {"doc": "", "fields": []}
        }"#;
        let func: ModuleFunc<'static> = serde_json::from_str(json).unwrap();
        assert_eq!(func.kind, ModuleKind::Normal);
        assert_eq!(func.description, None);
    }
}
