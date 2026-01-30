//! #[capability_client] - Marks a struct as client-side state

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, ItemStruct, Result, Visibility, parse_quote};

#[derive(Debug, Clone)]
pub struct CapClient {
    pub input: ItemStruct,
}

impl CapClient {
    pub fn new(mut input: ItemStruct) -> Result<Self> {
        // 1. Validate Visibility (Must be pub)
        if !matches!(input.vis, Visibility::Public(_)) {
            return Err(syn::Error::new_spanned(
                &input.vis,
                "capability_client structs must be public",
            ));
        }

        // 2. Decorate with Rkyv attributes
        let rkyv_derive: Attribute = parse_quote!(
            #[derive(::pyroduct::rkyv::Archive, ::pyroduct::rkyv::Serialize, ::pyroduct::rkyv::Deserialize)]
        );
        input.attrs.insert(0, rkyv_derive);
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

    /// Helper to expand the client macro from raw struct code.
    fn expand_client(code: TokenStream) -> TokenStream {
        let item = parse2(code).expect("Failed to parse struct input");
        CapClient::new(item)
            .expect("CapClient validation failed")
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
        let res_vis = CapClient::new(item_vis);
        assert!(res_vis.is_err());
        assert_eq!(
            res_vis.unwrap_err().to_string(),
            "capability_client structs must be public"
        );
    }
}