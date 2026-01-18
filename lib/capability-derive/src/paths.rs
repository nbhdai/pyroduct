//! Path and naming utilities for capability FFI generation
//!
//! This module centralizes all naming conventions used throughout the capability system
//! to ensure consistency between client and server sides.

use heck::{AsSnakeCase, AsUpperCamelCase};
use quote::format_ident;
use syn::{Ident, Type};

/// Contains all the core identifiers for a capability class and generates paths on demand
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClassIdent {
    pub trait_tn: Ident,
    pub state_tn: Ident,
    pub client_tn: Ident,
    pub error_tn: Option<Type>,
}

impl ClassIdent {
    // ========================================================================
    // Method Paths
    // ========================================================================

    /// Library identifier for a method (e.g., __my_trait__my_state__method_name)
    pub fn class_name(&self) -> Ident {
        let trait_snake = AsSnakeCase(self.trait_tn.to_string()).to_string();
        let state_snake = AsSnakeCase(self.state_tn.to_string()).to_string();
        format_ident!("__{}__{}", trait_snake, state_snake)
    }

    /// Library identifier for a method (e.g., __my_trait__my_state__method_name)
    pub fn trace_name(&self, name: &Ident) -> Ident {
        let trait_snake = AsSnakeCase(self.trait_tn.to_string()).to_string();
        let state_snake = AsSnakeCase(self.state_tn.to_string()).to_string();
        let snake = AsSnakeCase(name.to_string()).to_string();
        format_ident!("__{}__{}__{}", trait_snake, state_snake, snake)
    }

    /// Library identifier for a method (e.g., __my_trait__my_state__method_name)
    pub fn class_name_static(&self) -> Ident {
        let trait_snake = AsSnakeCase(self.trait_tn.to_string())
            .to_string()
            .to_uppercase();
        let state_snake = AsSnakeCase(self.state_tn.to_string())
            .to_string()
            .to_uppercase();
        format_ident!("__{}__{}", trait_snake, state_snake)
    }

    /// Library identifier for a method (e.g., __my_trait__my_state__method_name)
    pub fn trace_name_static(&self, name: &Ident) -> Ident {
        let trait_snake = AsSnakeCase(self.trait_tn.to_string())
            .to_string()
            .to_uppercase();
        let state_snake = AsSnakeCase(self.state_tn.to_string())
            .to_string()
            .to_uppercase();
        let snake = AsSnakeCase(name.to_string()).to_string().to_uppercase();
        format_ident!("__{}__{}__{}", trait_snake, state_snake, snake)
    }

    /// FFI function name for a method (e.g., __my_trait__my_state__name__ffi)
    pub fn ffi_name(&self, name: &Ident) -> Ident {
        let trait_snake = AsSnakeCase(self.trait_tn.to_string()).to_string();
        let state_snake = AsSnakeCase(self.state_tn.to_string()).to_string();
        let snake = AsSnakeCase(name.to_string()).to_string();
        format_ident!("__{}__{}__{}__ffi", trait_snake, state_snake, snake)
    }

    /// WASM import name for a method (e.g., __my_trait__my_state__name__wasm)
    pub fn wasm_name(&self, name: &Ident) -> Ident {
        let trait_snake = AsSnakeCase(self.trait_tn.to_string()).to_string();
        let state_snake = AsSnakeCase(self.state_tn.to_string()).to_string();
        let snake = AsSnakeCase(name.to_string()).to_string();
        format_ident!("__{}__{}__{}__wasm", trait_snake, state_snake, snake)
    }

    /// Input struct name for a method with multiple parameters
    pub fn input_struct(&self, name: &Ident) -> Ident {
        let trait_snake = AsUpperCamelCase(self.trait_tn.to_string()).to_string();
        let state_snake = AsUpperCamelCase(self.state_tn.to_string()).to_string();
        let snake = AsUpperCamelCase(name.to_string()).to_string();
        format_ident!("__{}__{}__{}__Input", trait_snake, state_snake, snake)
    }
}