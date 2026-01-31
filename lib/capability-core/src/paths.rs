//! Path and naming utilities for capability FFI generation
//!
//! This module centralizes all naming conventions used throughout the capability system
//! to ensure consistency between client and server sides.

use heck::{AsSnakeCase, AsUpperCamelCase};
use quote::format_ident;
use syn::{Ident, Type};

/// Identity of the capability (State, Client, Error)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CapabilityIdent {
    /// The struct being implemented (e.g., "MyStruct")
    pub state_tn: Ident,
    /// The client type identifier (e.g., "MyClient")
    pub client_tn: Ident,
    /// The config type identifier (e.g., "MyConfig")
    pub config_tn: Option<Type>,
    /// The error type, if present (e.g., "MyError")
    pub error_tn: Option<Type>,
}

impl CapabilityIdent {
    // ========================================================================
    // Method Paths
    // ========================================================================

    /// Library identifier for a method (e.g., __my_trait__my_state__method_name)
    pub fn trace_name(&self, name: &Ident) -> Ident {
        let state_snake = AsSnakeCase(self.state_tn.to_string()).to_string();
        let snake = AsSnakeCase(name.to_string()).to_string();
        format_ident!("__{}__{}", state_snake, snake)
    }

    /// Library identifier for a method (e.g., __my_trait__my_state__method_name)
    pub fn class_name_static(&self) -> Ident {
        let state_snake = AsSnakeCase(self.state_tn.to_string())
            .to_string()
            .to_uppercase();
        format_ident!("__{}", state_snake)
    }

    /// Library identifier for a method (e.g., __my_trait__my_state__method_name)
    pub fn trace_name_static(&self, name: &Ident) -> Ident {
        let state_snake = AsSnakeCase(self.state_tn.to_string())
            .to_string()
            .to_uppercase();
        let snake = AsSnakeCase(name.to_string()).to_string().to_uppercase();
        format_ident!("__{}__{}", state_snake, snake)
    }

    /// FFI function name for a method (e.g., __my_trait__my_state__name__ffi)
    pub fn ffi_name(&self, name: &Ident) -> Ident {
        let state_snake = AsSnakeCase(self.state_tn.to_string()).to_string();
        let snake = AsSnakeCase(name.to_string()).to_string();
        format_ident!("__{}__{}__ffi", state_snake, snake)
    }

    /// WASM import name for a method (e.g., __my_trait__my_state__name__wasm)
    pub fn wasm_name(&self, name: &Ident) -> Ident {
        let state_snake = AsSnakeCase(self.state_tn.to_string()).to_string();
        let snake = AsSnakeCase(name.to_string()).to_string();
        format_ident!("__{}__{}__wasm", state_snake, snake)
    }

    /// Input struct name for a method with multiple parameters
    pub fn input_struct(&self, name: &Ident) -> Ident {
        let state_snake = AsUpperCamelCase(self.state_tn.to_string()).to_string();
        let snake = AsUpperCamelCase(name.to_string()).to_string();
        format_ident!("__{}__{}__Input", state_snake, snake)
    }
}