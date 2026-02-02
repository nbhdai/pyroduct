//! #[interface] - Marks a struct as client-side state
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, ItemStruct, Result, Visibility, parse_quote};

#[derive(Debug, Clone)]
pub struct CapInterfaceItem {
    pub input: ItemStruct,
}

impl CapInterfaceItem {
    pub fn new(mut input: ItemStruct, required_docs: bool) -> Result<Self> {
        // 1. Validate Visibility
        if !matches!(input.vis, Visibility::Public(_)) {
            return Err(syn::Error::new_spanned(
                &input.vis,
                "interface structs must be public",
            ));
        }

        let has_docs = input.attrs.iter().any(|attr| attr.path().is_ident("doc"));
        if !has_docs && required_docs {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "Client structs must have documentation (///) to generate API specs.",
            ));
        }

        // 2. Detect Configuration and Existing Attributes
        let (has_manual_flag, client_attr_index) = parse_client_config(&input.attrs)?;
        let has_manual_attrs = check_manual_rkyv(&input.attrs)?;

        // 3. Validate Logic
        if has_manual_flag && !has_manual_attrs {
            return Err(syn::Error::new_spanned(
                &input,
                "#[client(manual_rkyv)] is set, but no manual #[rkyv] or Derive attributes were found on the struct.",
            ));
        }

        if !has_manual_flag && has_manual_attrs {
            return Err(syn::Error::new_spanned(
                &input,
                "Manual #[rkyv] attributes detected, but 'manual_rkyv' option was not set.\n\
                 Either remove the manual attributes to use auto-generation, or add #[pyroduct::client(manual_rkyv)].",
            ));
        }

        // 4. Clean up the Marker Attribute
        // We always remove the #[client] / #[pyroduct::client] attribute so it doesn't
        // trigger again in the generated code and cause conflicts.
        if let Some(idx) = client_attr_index {
            input.attrs.remove(idx);
        }

        // 5. Inject Defaults (only if NOT manual)
        if !has_manual_flag {
            let rkyv_crate: Attribute = parse_quote!(
                #[rkyv(crate = ::pyroduct::rkyv)]
            );
            let rkyv_derive: Attribute = parse_quote!(
                #[derive(::pyroduct::rkyv::Archive, ::pyroduct::rkyv::Serialize, ::pyroduct::rkyv::Deserialize)]
            );
            input.attrs.insert(0, rkyv_crate);
            input.attrs.insert(0, rkyv_derive);
        }

        Ok(Self { input })
    }

    pub fn expand(&self) -> TokenStream {
        let input = &self.input;
        quote! { #input }
    }
}

/// Helper: Parses #[pyroduct::client(manual_rkyv)]
/// Returns: (is_manual_set, index_of_attribute)
fn parse_client_config(attrs: &[Attribute]) -> Result<(bool, Option<usize>)> {
    for (i, attr) in attrs.iter().enumerate() {
        // Match #[client] or #[pyroduct::client]
        if is_client_attr(attr) {
            let mut is_manual = false;
            if attr.meta.require_list().is_ok() {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("manual_rkyv") {
                        is_manual = true;
                        Ok(())
                    } else {
                        // Strict check: error on unknown options to prevent typos
                        Err(meta.error("Unknown client option. Supported: manual_rkyv"))
                    }
                })?;
            }

            return Ok((is_manual, Some(i)));
        }
    }
    Ok((false, None))
}

/// Helper: Checks for #[rkyv] or derived Archive/Serialize/Deserialize
fn check_manual_rkyv(attrs: &[Attribute]) -> Result<bool> {
    for attr in attrs {
        // Check for #[rkyv(...)]
        if attr.path().is_ident("rkyv") {
            return Ok(true);
        }

        // Check for #[derive(...)] containing Archive, etc.
        if attr.path().is_ident("derive") {
            let mut found_rkyv_trait = false;
            attr.parse_nested_meta(|meta| {
                if let Some(ident) = meta.path.segments.last().map(|s| &s.ident) {
                    if ident == "Archive" || ident == "Serialize" || ident == "Deserialize" {
                        found_rkyv_trait = true;
                    }
                }
                Ok(())
            })?;

            if found_rkyv_trait {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn is_client_attr(attr: &Attribute) -> bool {
    attr.path().is_ident("interface_item")
        || (attr.path().segments.len() == 2
            && attr.path().segments[0].ident == "pyroduct"
            && attr.path().segments[1].ident == "interface_item")
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse2;

    /// Helper to expand the client macro from raw struct code.
    fn expand_client(code: TokenStream) -> TokenStream {
        let item = parse2(code).expect("Failed to parse struct input");
        CapInterfaceItem::new(item, false)
            .expect("CapInterfaceItem validation failed")
            .expand()
    }

    #[test]
    fn test_client_basic() {
        // 1. Define Input
        let code = quote! {
            pub struct MyClient {
                pub id: u32,
            }
        };

        // 2. Generate Output
        let output = expand_client(code);

        // 3. Define Expected Output
        // Just adds the derives.
        let expected = quote! {
            #[derive(::pyroduct::rkyv::Archive, ::pyroduct::rkyv::Serialize, ::pyroduct::rkyv::Deserialize)]
            #[rkyv(crate = ::pyroduct::rkyv)]
            pub struct MyClient {
                pub id: u32,
            }
        };

        crate::fmt::assert_code_eq_token(&output, &expected);
    }

    #[test]
    fn test_client_with_generics_allowed() {
        // 1. Define Input: Struct with generics (previously banned)
        let code = quote! {
            #[derive(Clone, Debug)]
            pub struct GenericClient<T> {
                pub data: T,
            }
        };

        // 2. Generate Output
        let output = expand_client(code);

        // 3. Define Expected Output
        // Derives added, generics preserved, debug preserved.
        let expected = quote! {
            #[derive(::pyroduct::rkyv::Archive, ::pyroduct::rkyv::Serialize, ::pyroduct::rkyv::Deserialize)]
            #[rkyv(crate = ::pyroduct::rkyv)]
            #[derive(Clone, Debug)]
            pub struct GenericClient<T> {
                pub data: T,
            }
        };

        crate::fmt::assert_code_eq_token(&output, &expected);
    }

    #[test]
    fn test_client_tuple_struct() {
        // 1. Define Input: Tuple struct (previously banned due to named field requirement)
        let code = quote! {
            pub struct TupleClient(u32, String);
        };

        // 2. Generate Output
        let output = expand_client(code);

        // 3. Define Expected Output
        let expected = quote! {
            #[derive(::pyroduct::rkyv::Archive, ::pyroduct::rkyv::Serialize, ::pyroduct::rkyv::Deserialize)]
            #[rkyv(crate = ::pyroduct::rkyv)]
            pub struct TupleClient(u32, String);
        };

        crate::fmt::assert_code_eq_token(&output, &expected);
    }

    #[test]
    fn test_validation_still_requires_pub() {
        let code_vis = quote! {
            struct PrivateClient { id: u32 }
        };
        let item_vis = parse2(code_vis).unwrap();
        let res_vis = CapInterfaceItem::new(item_vis, false);
        assert!(res_vis.is_err());
        assert_eq!(
            res_vis.unwrap_err().to_string(),
            "interface structs must be public"
        );
    }
}
