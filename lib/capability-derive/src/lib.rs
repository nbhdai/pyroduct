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
use quote::format_ident;
use syn::{ItemStruct, ItemTrait, parse_macro_input};

pub(crate) mod client;
pub(crate) mod ffi;
pub(crate) mod paths;
pub(crate) mod utils;
pub(crate) mod methods;
pub(crate) mod lifecycle;
pub(crate) mod capability;

/// Marks a struct as client-side state.
/// Adds rkyv serialization and the internal configuration buffer.
#[proc_macro_attribute]
pub fn client(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    match crate::client::CapClient::new(input) {
        Ok(client) => client.expand().into(),
        Err(e) => e.to_compile_error().into(),
    }
}


#[cfg(test)]
pub mod fmt {
    use proc_macro2::TokenStream;

    pub fn format_tokens(tokens: &TokenStream) -> String {
        match syn::parse_file(&tokens.to_string()) {
            Ok(file) => prettyplease::unparse(&file),
            Err(err) => {
                println!("Parsing Error {err:?}");
                tokens.to_string()
            }
        }
    }

    pub fn assert_code_eq_token(tokens: &TokenStream, expected: &TokenStream) {
        let actual_file: syn::File = syn::parse2(tokens.clone())
            .expect("Generated tokens are not valid Rust code (syn::File)");
        let expected_file: syn::File = syn::parse2(expected.clone())
            .expect("Expected string is not valid Rust code (syn::File)");
        let actual_str = prettyplease::unparse(&actual_file);
        let expected_str = prettyplease::unparse(&expected_file);
        if actual_str != expected_str {
            panic!(
                "Code mismatch!\n\nEXPECTED:\n{}\n\nACTUAL:\n{}\n",
                expected_str, actual_str
            );
        }
    }
}