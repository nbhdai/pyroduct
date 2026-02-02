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

use capability_core::config::{CapConfig, DocRec};
use proc_macro::TokenStream;

/// Marks a struct as client-side state.
/// Adds rkyv serialization and the internal configuration buffer.
#[proc_macro_attribute]
pub fn interface_item(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemStruct);
    match capability_core::client::CapInterfaceItem::new(input, false) {
        Ok(client) => client.expand().into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Marks a struct as configuration for a struct.
/// Adds rkyv serialization and the internal configuration buffer.
#[proc_macro_attribute]
pub fn config(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemStruct);
    match CapConfig::new(input, DocRec::NoReq) {
        Ok(config) => config.expand().into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Marks a struct as client-side state.
/// Adds rkyv serialization and the internal configuration buffer.
#[proc_macro_attribute]
pub fn capability(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // 2. Fetch crate info at compile time

    let input = syn::parse_macro_input!(item as syn::ItemImpl);
    let pkg_name = match std::env::var("CARGO_PKG_NAME") {
        Ok(pkg_name) => pkg_name,
        Err(_) => {
            return syn::Error::new_spanned(&input, "Unable to retrieve CARGO_PKG_NAME")
                .to_compile_error()
                .into();
        }
    };
    let pkg_version = match std::env::var("CARGO_PKG_VERSION") {
        Ok(pkg_name) => pkg_name,
        Err(_) => {
            return syn::Error::new_spanned(&input, "Unable to retrieve CARGO_PKG_VERSION")
                .to_compile_error()
                .into();
        }
    };
    match capability_core::capability::CapabilityImpl::new(input, false, &pkg_name, &pkg_version) {
        Ok(capability) => capability.expand_capability().into(),
        Err(e) => e.to_compile_error().into(),
    }
}
