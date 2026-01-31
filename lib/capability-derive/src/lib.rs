//! Capability Derive Macros
//!
//! This crate provides proc macros to generate FFI boilerplate for capabilities.
//!
//! # Patterns Supported
//!
//! 1. **Stateless functions** - `#[capability_function]` on standalone functions
//! 2. **Host state only** - `#[capability]` trait + `#[capability_server]` struct
//! 3. **Client state only** - `#[capability_client]` struct + `#[capability(stateless)]` trait
//! 4. **Both states** - `#[capability_client]` struct + `#[capability]` trait + `#[capability_server]` struct

use proc_macro::TokenStream;

/// Marks a struct as client-side state.
/// Adds rkyv serialization and the internal configuration buffer.
#[proc_macro_attribute]
pub fn client(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemStruct);
    match capability_core::client::CapClient::new(input) {
        Ok(client) => client.expand().into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Marks a struct as configuration for a struct.
/// Adds rkyv serialization and the internal configuration buffer.
#[proc_macro_attribute]
pub fn config(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemStruct);
    match capability_core::config::CapConfig::new(input) {
        Ok(config) => config.expand().into(),
        Err(e) => e.to_compile_error().into(),
    }
}


/// Marks a struct as client-side state.
/// Adds rkyv serialization and the internal configuration buffer.
#[proc_macro_attribute]
pub fn capability(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemImpl);
    match capability_core::capability::CapabilityImpl::new(input) {
        Ok(capability) => capability.expand().into(),
        Err(e) => e.to_compile_error().into(),
    }
}
