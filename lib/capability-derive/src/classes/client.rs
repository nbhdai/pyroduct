//! #[capability_client] - Marks a struct as client-side state

use proc_macro2::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{
    Attribute, Fields, FieldsNamed, Ident, ItemStruct, Path, Result, Token, Visibility, parse_quote
};

#[derive(Debug, Clone)]
pub struct CapClient {
    pub attrs: Vec<Attribute>,
    pub ident: Ident,
    pub fields: FieldsNamed,
    pub debug_impl: TokenStream,
}

impl CapClient {
    pub fn new(input: ItemStruct) -> Result<Self> {
        // 1. Validate Visibility (Must be pub)
        if !matches!(input.vis, Visibility::Public(_)) {
            return Err(syn::Error::new_spanned(
                &input.vis,
                "capability_client structs must be public",
            ));
        }

        // 2. Validate Generics (Must be empty)
        if !input.generics.params.is_empty() {
            return Err(syn::Error::new_spanned(
                &input.generics,
                "capability_client structs cannot have generics",
            ));
        }

        // 3. Validate Fields (Must be Named e.g., { x: i32 })
        let mut fields = match input.fields {
            Fields::Named(named) => named,
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "capability_client only supports structs with named fields",
                ));
            }
        };

        // 4. Parse Attributes to figure out Debug strategy
        let (processed_attrs, should_generate_debug) = Self::process_attributes(input.attrs);

        let debug_impl = if should_generate_debug {
            Self::generate_custom_debug(&input.ident, &fields)
        } else {
            quote! {}
        };

        // 5. Add the private buffer field to the list of fields for the struct definition
        if Self::has_serde_derive(&processed_attrs) {
            fields.named.push(parse_quote! {
                #[serde(skip)]
                #[rkyv(with = rkyv::with::Skip)]
                __config_buf: Vec<u8>
            });
        } else {
            fields.named.push(parse_quote! {
                #[rkyv(with = rkyv::with::Skip)]
                __config_buf: Vec<u8>
            });
        }

        Ok(Self {
            attrs: processed_attrs,
            ident: input.ident,
            fields,
            debug_impl,
        })
    }

    /// Generates the final code.
    /// Does not mutate the CapClient state; creates modified clones for output.
    pub fn expand(&self) -> TokenStream {
        let ident = &self.ident;

        // 1. Prepare Attributes (Clone and Add Derives)
        let final_attrs = &self.attrs;

        // 2. Prepare Fields (Clone and Add Buffer)
        let final_fields = &self.fields;

        let debug_impl = &self.debug_impl;

        // 4. Generate Trait Impl
        let trait_impl = self.generate_client_trait();

        // 5. Output
        quote! {
            #(#final_attrs)*
            pub struct #ident #final_fields

            #debug_impl
            #trait_impl
        }
    }

    fn generate_custom_debug(ident: &Ident, fields: &FieldsNamed) -> TokenStream {
        let struct_name = &ident;
        let mut field_debugs = TokenStream::new();

        // Iterate over ORIGINAL fields only
        for field in &fields.named {
            let fname = field.ident.as_ref().unwrap();
            let fname_str = fname.to_string();

            field_debugs.extend(quote! {
                .field(#fname_str, &self.#fname)
            });
        }
        let struct_string = struct_name.to_string();

        quote! {
            impl std::fmt::Debug for #struct_name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.debug_struct(#struct_string)
                        #field_debugs
                        .finish()
                }
            }
        }
    }

    fn generate_client_trait(&self) -> TokenStream {
        let struct_name = &self.ident;

        quote! {
            impl ::pyroduct::module_capability::CapabilityClient for #struct_name {
                fn config_buffer(&self) -> &[u8] {
                    &self.__config_buf
                }
            }
        }
    }

    fn has_serde_derive(attrs: &[Attribute]) -> bool {
        for attr in attrs {
            if attr.path().is_ident("derive") {
                if let Ok(paths) =
                    attr.parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated)
                {
                    for path in paths {
                        if path.is_ident("Serialize") || path.is_ident("Deserialize") {
                            return true;
                        }
                        if path.segments.len() == 2 && path.segments[0].ident == "serde" {
                            let last = &path.segments[1].ident;
                            if last == "Serialize" || last == "Deserialize" {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    fn process_attributes(input_attrs: Vec<Attribute>) -> (Vec<Attribute>, bool) {
        let mut found_debug = false;
        let mut new_attrs = Vec::new();

        new_attrs.push(parse_quote!(#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]));
        new_attrs.push(parse_quote!(#[rkyv(compare(PartialEq), derive(Debug))]));

        for attr in input_attrs {
            if attr.path().is_ident("derive") {
                if let Ok(paths) =
                    attr.parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated)
                {
                    let mut new_paths = Punctuated::<Path, Token![,]>::new();
                    let mut modified_this_attr = false;

                    for path in paths {
                        let is_debug = path.is_ident("Debug")
                            || (path.segments.last().map(|s| s.ident == "Debug").unwrap_or(false)
                                && path.segments.first().map(|s| s.ident == "std" || s.ident == "core").unwrap_or(false));

                        if is_debug {
                            found_debug = true;
                            modified_this_attr = true;
                        } else {
                            new_paths.push(path);
                        }
                    }

                    if modified_this_attr {
                        if !new_paths.is_empty() {
                            new_attrs.push(parse_quote!(#[derive(#new_paths)]));
                        }
                    } else {
                        new_attrs.push(attr);
                    }
                } else {
                    new_attrs.push(attr);
                }
            } else {
                new_attrs.push(attr);
            }
        }

        (new_attrs, found_debug)
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
    fn test_client_basic_no_serde() {
        // 1. Define Input: Basic struct without specific derives
        let code = quote! {
            pub struct MyClient {
                pub id: u32,
            }
        };

        // 2. Generate Output
        let output = expand_client(code);

        // 3. Define Expected Output
        // Note: rkyv attributes are prepended, config_buf is appended, and CapabilityClient is implemented.
        let expected = quote! {
            #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
            #[rkyv(compare(PartialEq), derive(Debug))]
            pub struct MyClient {
                pub id: u32,
                #[rkyv(with = rkyv::with::Skip)]
                __config_buf: Vec<u8>
            }

            impl ::pyroduct::module_capability::CapabilityClient for MyClient {
                fn config_buffer(&self) -> &[u8] {
                    &self.__config_buf
                }
            }
        };

        crate::fmt::assert_code_eq_token(&output, &expected);
    }

    #[test]
    fn test_client_with_serde_derive() {
        // 1. Define Input: Struct with Serialize/Deserialize
        let code = quote! {
            #[derive(Clone, Serialize, Deserialize)]
            pub struct MyClient {
                pub id: u32,
            }
        };

        // 2. Generate Output
        let output = expand_client(code);

        // 3. Define Expected Output
        // Note: 'serde(skip)' should be added to the buffer field because Serialize/Deserialize were detected.
        let expected = quote! {
            #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
            #[rkyv(compare(PartialEq), derive(Debug))]
            #[derive(Clone, Serialize, Deserialize)]
            pub struct MyClient {
                pub id: u32,
                #[serde(skip)]
                #[rkyv(with = rkyv::with::Skip)]
                __config_buf: Vec<u8>
            }

            impl ::pyroduct::module_capability::CapabilityClient for MyClient {
                fn config_buffer(&self) -> &[u8] {
                    &self.__config_buf
                }
            }
        };

        crate::fmt::assert_code_eq_token(&output, &expected);
    }

    #[test]
    fn test_client_debug_override() {
        // 1. Define Input: Struct with Debug derived
        let code = quote! {
            #[derive(Clone, Debug)]
            pub struct MyDebugClient {
                pub name: String,
            }
        };

        // 2. Generate Output
        let output = expand_client(code);

        // 3. Define Expected Output
        // Note:
        // - 'Debug' is stripped from the #[derive(...)] list.
        // - A custom impl std::fmt::Debug is generated.
        // - The buffer field is NOT included in the debug output.
        let expected = quote! {
            #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
            #[rkyv(compare(PartialEq), derive(Debug))]
            #[derive(Clone)] 
            pub struct MyDebugClient {
                pub name: String,
                #[rkyv(with = rkyv::with::Skip)]
                __config_buf: Vec<u8>
            }

            impl std::fmt::Debug for MyDebugClient {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.debug_struct("MyDebugClient")
                        .field("name", &self.name)
                        .finish()
                }
            }

            impl ::pyroduct::module_capability::CapabilityClient for MyDebugClient {
                fn config_buffer(&self) -> &[u8] {
                    &self.__config_buf
                }
            }
        };

        crate::fmt::assert_code_eq_token(&output, &expected);
    }

    #[test]
    fn test_validation_rejects_invalid_inputs() {
        // Case 1: Non-public struct
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

        // Case 2: Generics
        let code_gen = quote! {
            pub struct GenericClient<T> { id: T }
        };
        let item_gen = parse2(code_gen).unwrap();
        let res_gen = CapClient::new(item_gen);
        assert!(res_gen.is_err());
        assert_eq!(
            res_gen.unwrap_err().to_string(),
            "capability_client structs cannot have generics"
        );

        // Case 3: Tuple Structs
        let code_tuple = quote! {
            pub struct TupleClient(u32, u32);
        };
        let item_tuple = parse2(code_tuple).unwrap();
        let res_tuple = CapClient::new(item_tuple);
        assert!(res_tuple.is_err());
        assert_eq!(
            res_tuple.unwrap_err().to_string(),
            "capability_client only supports structs with named fields"
        );
    }
}