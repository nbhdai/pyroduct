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
        let trait_snake = AsSnakeCase(self.trait_tn.to_string()).to_string().to_uppercase();
        let state_snake = AsSnakeCase(self.state_tn.to_string()).to_string().to_uppercase();
        format_ident!("__{}__{}", trait_snake, state_snake)
    }

    /// Library identifier for a method (e.g., __my_trait__my_state__method_name)
    pub fn trace_name_static(&self, name: &Ident) -> Ident {
        let trait_snake = AsSnakeCase(self.trait_tn.to_string()).to_string().to_uppercase();
        let state_snake = AsSnakeCase(self.state_tn.to_string()).to_string().to_uppercase();
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

    // ========================================================================
    // Lifecycle Paths
    // ========================================================================

    /// Init trait name (e.g., GreeterServerInit)
    pub fn init_trait_name(&self) -> Ident {
        format_ident!("{}Init", self.state_tn)
    }

    /// Init FFI function name (e.g., __greeter_server_ffi_init)
    pub fn init_ffi_name(&self) -> Ident {
        let state_snake = AsSnakeCase(self.state_tn.to_string()).to_string();
        format_ident!("__{}_ffi_init", state_snake)
    }

    /// Drop FFI function name (e.g., __greeter_server_ffi_drop)
    pub fn drop_ffi_name(&self) -> Ident {
        let state_snake = AsSnakeCase(self.state_tn.to_string()).to_string();
        format_ident!("__{}_ffi_drop", state_snake)
    }

    /// Reset FFI function name (e.g., __greeter_server_ffi_reset)
    pub fn reset_ffi_name(&self) -> Ident {
        let state_snake = AsSnakeCase(self.state_tn.to_string()).to_string();
        format_ident!("__{}_ffi_reset", state_snake)
    }

    // ========================================================================
    // Export Table Paths
    // ========================================================================

    /// Static exports array name (e.g., __MY_TRAIT__MY_STATE__EXPORTS)
    pub fn exports_array_name(&self) -> Ident {
        let trait_upper = AsSnakeCase(self.trait_tn.to_string()).to_string().to_uppercase();
        let state_upper = AsSnakeCase(self.state_tn.to_string()).to_string().to_uppercase();
        format_ident!("__{}__{}__EXPORTS", trait_upper, state_upper)
    }

    /// PluginExports struct name (e.g., __MY_TRAIT__MY_STATE__PLUGIN_EXPORTS)
    pub fn plugin_exports_name(&self) -> Ident {
        let trait_upper = AsSnakeCase(self.trait_tn.to_string()).to_string().to_uppercase();
        let state_upper = AsSnakeCase(self.state_tn.to_string()).to_string().to_uppercase();
        format_ident!("__{}__{}__PLUGIN_EXPORTS", trait_upper, state_upper)
    }
}

// ========================================================================
// Standalone Function Paths (for functions not part of a class)
// ========================================================================

/// FFI function name for a standalone function (e.g., __get_count__ffi)
pub fn function_ffi_name(function_name: &Ident) -> Ident {
    let function_snake = AsSnakeCase(function_name.to_string()).to_string();
    format_ident!("__{}__ffi", function_snake)
}

/// WASM import name for a standalone function (e.g., __get_count__wasm)
pub fn function_wasm_name(function_name: &Ident) -> Ident {
    let function_snake = AsSnakeCase(function_name.to_string()).to_string();
    format_ident!("__{}__wasm", function_snake)
}
