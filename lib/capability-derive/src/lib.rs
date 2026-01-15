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

pub(crate) mod capability;
pub(crate) mod capability_ffi;
pub(crate) mod capability_client;
pub(crate) mod capability_export;
pub(crate) mod capability_function;
pub(crate) mod capability_server;
pub(crate) mod utils;


#[proc_macro_attribute]
pub fn capability_client(attr: TokenStream, item: TokenStream) -> TokenStream {
    capability_client::expand(attr.into(), item.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}


// #[proc_macro_attribute]
// pub fn capability(attr: TokenStream, item: TokenStream) -> TokenStream {
//     capability::expand(attr.into(), item.into())
//         .unwrap_or_else(|e| e.to_compile_error())
//         .into()
// }


#[proc_macro_attribute]
pub fn capability_server(attr: TokenStream, item: TokenStream) -> TokenStream {
    capability_server::expand(attr.into(), item.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn capability_function(attr: TokenStream, item: TokenStream) -> TokenStream {
    capability_function::expand(attr.into(), item.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}


#[proc_macro]
pub fn capability_export(input: TokenStream) -> TokenStream {
    capability_export::expand(input.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}


// #[proc_macro_attribute]
// pub fn capability_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
//     capability_impl::expand(attr.into(), item.into())
//         .unwrap_or_else(|e| e.to_compile_error())
//         .into()
// }

#[cfg(test)]
pub mod fmt {
    use proc_macro2::TokenStream;

    pub fn format_tokens(tokens: &TokenStream) -> String {
        // Try to parse and format as a file, fallback to raw string if it fails
        match syn::parse_file(&tokens.to_string()) {
            Ok(file) => prettyplease::unparse(&file),
            Err(err) => {
                tracing::error!(?err, "Parsing Error");
                tokens.to_string()
            },
        }
    }

    pub fn assert_code_eq(tokens: &TokenStream, expected: &str) {
        // 1. Parse the generated TokenStream into a syn::File
        let actual_file: syn::File = syn::parse2(tokens.clone())
            .expect("Generated tokens are not valid Rust code (syn::File)");

        // 2. Parse the expected string into a syn::File
        let expected_file: syn::File = syn::parse_str(expected)
            .expect("Expected string is not valid Rust code (syn::File)");

        // 3. Unparse both using prettyplease to normalize formatting
        let actual_str = prettyplease::unparse(&actual_file);
        let expected_str = prettyplease::unparse(&expected_file);

        // 4. Compare the normalized strings
        if actual_str != expected_str {
            panic!(
                "Code mismatch!\n\nEXPECTED:\n{}\n\nACTUAL:\n{}\n",
                expected_str, actual_str
            );
        }
    }

}