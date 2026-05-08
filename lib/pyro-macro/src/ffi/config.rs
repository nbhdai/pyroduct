//! #[capability_config] - Marks a struct as configuration

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, ItemStruct, Result, Visibility, parse_quote};

use crate::format::DocRec;

#[derive(Debug, Clone)]
pub struct CapConfig {
    pub input: ItemStruct,
}

impl CapConfig {
    pub fn new(mut input: ItemStruct, doc_rec: DocRec) -> Result<Self> {
        // 1. Validate Visibility (Must be pub)
        if !matches!(input.vis, Visibility::Public(_)) {
            return Err(syn::Error::new_spanned(
                &input.vis,
                "capability_config structs must be public",
            ));
        }

        // 2. Validate Documentation
        Self::validate_docs(&input, doc_rec)?;

        // // 3. Add cfg(feature = "capability") gate
        // let cfg_gate: Attribute = parse_quote!(
        //     #[cfg(feature = "capability")]
        // );

        let serde_crate: Attribute = parse_quote!(
            #[serde(crate = "::pyroduct::format::serde")]
        );

        // 4. Decorate with Serde attributes
        let serde_derive: Attribute = parse_quote!(
            #[derive(::pyroduct::format::serde::Serialize, ::pyroduct::format::serde::Deserialize)]
        );

        // Insert in reverse order of appearance
        input.attrs.insert(0, serde_crate);
        input.attrs.insert(0, serde_derive);
        // input.attrs.insert(0, cfg_gate);

        Ok(Self { input })
    }

    fn validate_docs(input: &ItemStruct, doc_rec: DocRec) -> Result<()> {
        let has_struct_doc = input.attrs.iter().any(|a| a.path().is_ident("doc"));

        match doc_rec {
            DocRec::StructDoc | DocRec::AllDoc => {
                if !has_struct_doc {
                    return Err(syn::Error::new_spanned(
                        &input.ident,
                        "Configuration struct must be documented",
                    ));
                }
            }
            _ => {}
        }

        if doc_rec == DocRec::AllDoc {
            if let syn::Fields::Named(fields) = &input.fields {
                for field in &fields.named {
                    let has_field_doc = field.attrs.iter().any(|a| a.path().is_ident("doc"));
                    if !has_field_doc {
                        // Use the field identifier for the error location if available
                        let tokens = if let Some(ident) = &field.ident {
                            quote! { #ident }
                        } else {
                            quote! { #field }
                        };

                        return Err(syn::Error::new_spanned(
                            tokens,
                            "Configuration fields must be documented",
                        ));
                    }
                }
            }
        }
        Ok(())
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
    fn expand_config(code: TokenStream, doc_rec: DocRec) -> TokenStream {
        let item = parse2(code).expect("Failed to parse struct input");
        CapConfig::new(item, doc_rec)
            .expect("CapConfig validation failed")
            .expand()
    }

    #[test]
    fn test_config_basic() {
        let code = quote! {
            pub struct MyConfig {
                pub host: String,
                pub port: u16,
            }
        };

        let output = expand_config(code, DocRec::NoReq);

        let expected = quote! {
            #[derive(::pyroduct::format::serde::Serialize, ::pyroduct::format::serde::Deserialize)]
            #[serde(crate = "::pyroduct::format::serde")]
            pub struct MyConfig {
                pub host: String,
                pub port: u16,
            }
        };

        crate::fmt::assert_code_eq_token(&output, &expected);
    }

    #[test]
    fn test_doc_rec_struct_missing() {
        let code = quote! {
            pub struct Undocumented {
                pub x: i32,
            }
        };
        let item = parse2(code).unwrap();

        // Should fail
        let err = CapConfig::new(item, DocRec::StructDoc).unwrap_err();
        assert_eq!(err.to_string(), "Configuration struct must be documented");
    }

    #[test]
    fn test_doc_rec_field_missing() {
        let code = quote! {
            /// Top level docs
            pub struct PartiallyDocumented {
                /// documented
                pub x: i32,
                pub y: i32, // Undocumented
            }
        };
        let item: ItemStruct = parse2(code).unwrap();

        // Should pass StructDoc
        assert!(CapConfig::new(item.clone(), DocRec::StructDoc).is_ok());

        // Should fail AllDoc
        let err = CapConfig::new(item, DocRec::AllDoc).unwrap_err();
        assert_eq!(err.to_string(), "Configuration fields must be documented");
    }

    #[test]
    fn test_doc_rec_full_success() {
        let code = quote! {
            /// Configures the server
            pub struct ServerConfig {
                /// The host
                pub host: String,
                /// The port
                pub port: u16,
            }
        };
        let item = parse2(code).unwrap();
        assert!(CapConfig::new(item, DocRec::AllDoc).is_ok());
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
        let output = expand_config(code, DocRec::NoReq);

        // 3. Define Expected Output
        // Derives added, generics preserved, debug preserved.
        let expected = quote! {
            #[derive(::pyroduct::format::serde::Serialize, ::pyroduct::format::serde::Deserialize)]
            #[serde(crate = "::pyroduct::format::serde")]
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
        let output = expand_config(code, DocRec::NoReq);

        // 3. Define Expected Output
        let expected = quote! {
            #[derive(::pyroduct::format::serde::Serialize, ::pyroduct::format::serde::Deserialize)]
            #[serde(crate = "::pyroduct::format::serde")]
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
        let res_vis = CapConfig::new(item_vis, DocRec::NoReq);
        assert!(res_vis.is_err());
        assert_eq!(
            res_vis.unwrap_err().to_string(),
            "capability_config structs must be public"
        );
    }
}
