//! Stub for the function/interface spec — interfaces, classes, request/reply
//! signatures. Moved here verbatim (`docs/pyro-schema-crate-plan.md`, Step
//! 6); the function spec itself is a follow-up design, not part of this
//! move. May use `crate::types` and `crate::bridge`, never `crate::wire`
//! directly.
//!
//! Serde derives were dropped from the original draft: `PyroSchema`/
//! `PyroField`/`FieldName` don't implement `Serialize`/`Deserialize` today
//! (this file was never compiled — `mod functions;` was commented out — so
//! that was never exercised). Restoring JSON support here is part of the
//! follow-up design, not this move.

use crate::{FieldName, PyroSchema, PyroType};

/// Documentation for the main function of a module
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleFunc<'a> {
    pub name: FieldName<'a>,
    pub description: Option<FieldName<'a>>,
    pub input: PyroSchema<'a>,
    pub output: PyroSchema<'a>,
}

/// The root specification object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceSpec<'a> {
    pub capability: FieldName<'a>,
    pub description: Option<FieldName<'a>>,
    pub classes: Vec<ClassSpec<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassSpec<'a> {
    pub name: FieldName<'a>,
    pub description: Option<FieldName<'a>>,
    pub methods: Vec<CapabilityFunc<'a>>,

    pub client: Option<PyroSchema<'a>>,
    pub config: Option<PyroSchema<'a>>,
}

/// Documentation for a capability function
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityFunc<'a> {
    pub name: FieldName<'a>,
    pub description: Option<FieldName<'a>>,
    pub input: PyroSchema<'a>,
    pub output: PyroType,
}
