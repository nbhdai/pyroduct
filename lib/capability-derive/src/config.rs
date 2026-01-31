//! #[capability_config] - Marks a struct as configuration

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, ItemStruct, Result, Visibility, parse_quote};

#[derive(Debug, Clone)]
pub struct CapConfig {
    pub input: ItemStruct,
}

impl CapConfig {
    pub fn new(mut input: ItemStruct) -> Result<Self> {
        // 1. Validate Visibility (Must be pub)
        if !matches!(input.vis, Visibility::Public(_)) {
            return Err(syn::Error::new_spanned(
                &input.vis,
                "capability_config structs must be public",
            ));
        }
        let serde_crate: Attribute = parse_quote!(
            #[serde(crate = "::pyroduct::serde")]
        );
        // 2. Decorate with Serde attributes
        let serde_derive: Attribute = parse_quote!(
            #[derive(::pyroduct::serde::Serialize, ::pyroduct::serde::Deserialize)]
        );
        input.attrs.insert(0, serde_crate);
        input.attrs.insert(0, serde_derive);
        Ok(Self { input })
    }

    /// Generates the final code.
    pub fn expand(&self) -> TokenStream {
        let input = &self.input;
        quote! { #input }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse2;

    /// Helper to expand the config macro from raw struct code.
    fn expand_config(code: TokenStream) -> TokenStream {
        let item = parse2(code).expect("Failed to parse struct input");
        CapConfig::new(item)
            .expect("CapConfig validation failed")
            .expand()
    }

    #[test]
    fn test_config_basic() {
        // 1. Define Input
        let code = quote! {
            pub struct MyConfig {
                pub host: String,
                pub port: u16,
            }
        };

        // 2. Generate Output
        let output = expand_config(code);

        // 3. Define Expected Output
        // Just adds the derives.
        let expected = quote! {
            #[derive(::pyroduct::serde::Serialize, ::pyroduct::serde::Deserialize)]
            #[serde(crate = "::pyroduct::serde")]
            pub struct MyConfig {
                pub host: String,
                pub port: u16,
            }
        };

        crate::fmt::assert_code_eq_token(&output, &expected);
    }

    #[test]
    fn test_config_with_generics_allowed() {
        // 1. Define Input: Struct with generics
        let code = quote! {
            #[derive(Clone, Debug)]
            pub struct GenericConfig<T> {
                pub options: T,
            }
        };

        // 2. Generate Output
        let output = expand_config(code);

        // 3. Define Expected Output
        // Derives added, generics preserved, debug preserved.
        let expected = quote! {
            #[derive(::pyroduct::serde::Serialize, ::pyroduct::serde::Deserialize)]
            #[serde(crate = "::pyroduct::serde")]
            #[derive(Clone, Debug)]
            pub struct GenericConfig<T> {
                pub options: T,
            }
        };

        crate::fmt::assert_code_eq_token(&output, &expected);
    }

    #[test]
    fn test_config_tuple_struct() {
        // 1. Define Input: Tuple struct
        let code = quote! {
            pub struct TupleConfig(String, u32);
        };

        // 2. Generate Output
        let output = expand_config(code);

        // 3. Define Expected Output
        let expected = quote! {
            #[derive(::pyroduct::serde::Serialize, ::pyroduct::serde::Deserialize)]
            #[serde(crate = "::pyroduct::serde")]
            pub struct TupleConfig(String, u32);
        };

        crate::fmt::assert_code_eq_token(&output, &expected);
    }

    #[test]
    fn test_validation_still_requires_pub() {
        let code_vis = quote! {
            struct PrivateConfig { timeout: u64 }
        };
        let item_vis = parse2(code_vis).unwrap();
        let res_vis = CapConfig::new(item_vis);
        assert!(res_vis.is_err());
        assert_eq!(
            res_vis.unwrap_err().to_string(),
            "capability_config structs must be public"
        );
    }
}