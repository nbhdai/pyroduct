//! `#[config]` and `#[magma]` structs: passed through as written, followed by
//! pyroformat's `Layout`, `Writer` and `Reader` impls, so they go over the
//! wire.

use proc_macro2::TokenStream;
use pyroformat_macro::parse::StructInput;
use quote::quote;
use syn::{Attribute, ItemStruct, Path, Result, Visibility};

/// Defines the documentation requirements for a wire struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocRec {
    /// No documentation required.
    NoReq,
    /// The top-level struct must be documented.
    StructDoc,
    /// The struct and all its fields must be documented.
    AllDoc,
}

impl DocRec {
    pub fn need_struct(&self) -> bool {
        match self {
            DocRec::NoReq => false,
            DocRec::StructDoc => true,
            DocRec::AllDoc => true,
        }
    }
}

/// A struct that goes over the wire.
#[derive(Debug, Clone)]
pub struct WireStruct {
    /// The struct as written, without the attributes only the macros read.
    pub input: ItemStruct,
    wire: StructInput,
}

impl WireStruct {
    /// `import` is the path generated code reaches `pyroformat` through,
    /// e.g. `::pyroduct`.
    pub fn new(mut input: ItemStruct, doc_rec: DocRec, import: &Path) -> Result<Self> {
        validate_docs(&input, doc_rec)?;
        let wire = StructInput::parse(&input.clone().into())?.with_krate(quote!(#import));
        strip_macro_attrs(&mut input);
        Ok(Self { input, wire })
    }

    /// Generates the final code.
    pub fn expand(&self) -> TokenStream {
        let input = &self.input;
        let impls = pyroformat_macro::expand_impls(&self.wire);
        quote! {
            #input
            #impls
        }
    }
}

#[derive(Debug, Clone)]
pub struct CapConfig {
    pub wire: WireStruct,
}

impl CapConfig {
    pub fn new(input: ItemStruct, doc_rec: DocRec) -> Result<Self> {
        // 1. Validate Visibility (Must be pub)
        if !matches!(input.vis, Visibility::Public(_)) {
            return Err(syn::Error::new_spanned(
                &input.vis,
                "capability_config structs must be public",
            ));
        }

        // 2. Validate Documentation, and lay it out for the wire
        let wire = WireStruct::new(input, doc_rec, &syn::parse_quote!(::pyroduct))?;
        Ok(Self { wire })
    }

    /// Generates the final code.
    pub fn expand(&self) -> TokenStream {
        self.wire.expand()
    }
}

fn validate_docs(input: &ItemStruct, doc_rec: DocRec) -> Result<()> {
    let has_struct_doc = input.attrs.iter().any(|a| a.path().is_ident("doc"));

    if doc_rec.need_struct() && !has_struct_doc {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "Configuration struct must be documented",
        ));
    }

    if doc_rec == DocRec::AllDoc
        && let syn::Fields::Named(fields) = &input.fields
    {
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
    Ok(())
}

/// Drops `#[config]`, `#[magma]` and every `#[pyro(...)]`: they've been read,
/// and nothing would resolve them on the struct as emitted.
fn strip_macro_attrs(input: &mut ItemStruct) {
    let is_macro_attr = |a: &Attribute| {
        let path = a.path();
        path.is_ident("pyro")
            || ["config", "magma"].iter().any(|name| {
                path.is_ident(name)
                    || (path.segments.len() == 2
                        && path.segments[0].ident == "pyroduct"
                        && path.segments[1].ident == name)
            })
    };
    input.attrs.retain(|a| !is_macro_attr(a));
    for field in input.fields.iter_mut() {
        field.attrs.retain(|a| !a.path().is_ident("pyro"));
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

        let output = expand_config(code.clone(), DocRec::NoReq);

        let impls = pyroformat_macro::expand_impls(
            &StructInput::parse(&parse2(code.clone()).unwrap())
                .unwrap()
                .with_krate(quote!(::pyroduct)),
        );
        let expected = quote! {
            #code
            #impls
        };

        crate::fmt::assert_code_eq_token(&output, &expected);
        assert!(
            output
                .to_string()
                .contains(":: pyroduct :: Writer for MyConfig")
        );
    }

    #[test]
    fn test_config_strips_macro_attrs() {
        let code = quote! {
            #[pyroduct::config]
            #[derive(Clone, Debug)]
            #[pyro(Word = u32)]
            pub struct MyConfig {
                #[pyro(numeric)]
                pub weights: Vec<u8>,
            }
        };
        let config = CapConfig::new(parse2(code).unwrap(), DocRec::NoReq).unwrap();

        let expected: ItemStruct = syn::parse_quote! {
            #[derive(Clone, Debug)]
            pub struct MyConfig {
                pub weights: Vec<u8>,
            }
        };
        assert_eq!(config.wire.input, expected);
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
    fn test_config_rejects_generics() {
        let code = quote! {
            #[derive(Clone, Debug)]
            pub struct GenericConfig<T> {
                pub options: T,
            }
        };
        let err = CapConfig::new(parse2(code).unwrap(), DocRec::NoReq).unwrap_err();
        assert!(err.to_string().contains("generic"));
    }

    #[test]
    fn test_config_rejects_tuple_struct() {
        let code = quote! {
            pub struct TupleConfig(String, u32);
        };
        let err = CapConfig::new(parse2(code).unwrap(), DocRec::NoReq).unwrap_err();
        assert!(err.to_string().contains("named fields"));
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
