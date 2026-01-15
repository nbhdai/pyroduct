//! #[capability_client] - Marks a struct as client-side state

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, Ident, ItemStruct, Path, Result, Token, parse_quote, parse2};
use syn::punctuated::Punctuated;

#[derive(Default, Debug, Clone)]
struct ClientAttrs {
    // Placeholder for future configuration attributes
}

fn parse_client_attrs(_attr: TokenStream) -> Result<ClientAttrs> {
    // Currently no attributes are parsed, but structure matches server for future extensibility
    Ok(ClientAttrs::default())
}

#[derive(Debug, Clone)]
pub struct CapClient {
    input: ItemStruct,
    #[allow(dead_code)]
    attrs: ClientAttrs,
}

impl CapClient {
    pub fn new(attr: TokenStream, input: ItemStruct) -> Result<Self> {
        let attrs = parse_client_attrs(attr)?;
        Ok(Self { input, attrs })
    }

    pub fn name(&self) -> &Ident {
        &self.input.ident
    }

    pub fn expand(mut self) -> Result<TokenStream> {
        // Check for and remove standard Debug derive if present
        let has_debug = self.remove_std_debug();

        // Add necessary derives and the config buffer field
        self.add_derives();
        self.add_buffer()?;
        
        let input = &self.input;
        
        // If Debug was requested, generate our custom implementation that hides the buffer
        let debug_impl = if has_debug {
            self.generate_custom_debug()
        } else {
            TokenStream::new()
        };

        let trait_impl = self.generate_client_trait();

        Ok(quote! { 
            #input 
            #debug_impl
            #trait_impl
        })
    }

    fn remove_std_debug(&mut self) -> bool {
        let mut found_debug = false;
        let mut new_attrs = Vec::new();

        for attr in self.input.attrs.drain(..) {
            if attr.path().is_ident("derive") {
                if let Ok(paths) = attr.parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated) {
                    let mut new_paths = Punctuated::<Path, Token![,]>::new();
                    let mut modified = false;

                    for path in paths {
                        // Check for "Debug", "std::fmt::Debug", "core::fmt::Debug"
                        let is_debug = path.is_ident("Debug") || 
                            (path.segments.last().map(|s| s.ident == "Debug").unwrap_or(false) && 
                             path.segments.first().map(|s| s.ident == "std" || s.ident == "core").unwrap_or(false));

                        if is_debug {
                            found_debug = true;
                            modified = true;
                        } else {
                            new_paths.push(path);
                        }
                    }

                    if modified {
                        if !new_paths.is_empty() {
                            new_attrs.push(parse_quote!(#[derive(#new_paths)]));
                        }
                        // If empty, we effectively remove the derive attribute entirely
                    } else {
                        new_attrs.push(attr);
                    }
                } else {
                    // If we can't parse args, keep original attr
                    new_attrs.push(attr);
                }
            } else {
                new_attrs.push(attr);
            }
        }

        self.input.attrs = new_attrs;
        found_debug
    }

    fn generate_custom_debug(&self) -> TokenStream {
        let struct_name = &self.input.ident;
        let (impl_generics, ty_generics, where_clause) = self.input.generics.split_for_impl();
        
        let mut field_debugs = TokenStream::new();
        if let Fields::Named(fields) = &self.input.fields {
            for field in &fields.named {
                let fname = field.ident.as_ref().unwrap();
                let fname_str = fname.to_string();
                
                // Skip our internal buffer field
                if fname_str == "__config_buf" {
                    continue; 
                }

                field_debugs.extend(quote! {
                    .field(#fname_str, &self.#fname)
                });
            }
        }

        quote! {
            impl #impl_generics std::fmt::Debug for #struct_name #ty_generics #where_clause {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.debug_struct(stringify!(#struct_name))
                        #field_debugs
                        .finish()
                }
            }
        }
    }

    fn generate_client_trait(&self) -> TokenStream {
        let struct_name = &self.input.ident;
        let (impl_generics, ty_generics, where_clause) = self.input.generics.split_for_impl();
        
        quote! {
            /// Trait providing access to the client capability configuration buffer.
            impl #impl_generics ::pyroduct::module_capability::CapabilityClient for #struct_name #ty_generics #where_clause {
                fn config_buffer(&self) -> &[u8] {
                    &self.__config_buf
                }
            }
        }
    }

    fn add_derives(&mut self) {
        // Add rkyv derives required for serialization
        self.input.attrs.push(parse_quote!(#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]));
        // Add rkyv specific configuration
        self.input.attrs.push(parse_quote!(#[rkyv(compare(PartialEq), derive(Debug))]));
    }

    fn has_serde_derive(&self) -> bool {
        for attr in &self.input.attrs {
            if attr.path().is_ident("derive") {
                // Try to parse the arguments as a comma-separated list of paths
                if let Ok(paths) = attr.parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated) {
                    for path in paths {
                        // Check for simple ident "Serialize" / "Deserialize"
                        if path.is_ident("Serialize") || path.is_ident("Deserialize") {
                            return true;
                        }
                        // Check for fully qualified "serde::Serialize" / "serde::Deserialize"
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

    fn add_buffer(&mut self) -> Result<()> {
        let include_serde_skip = self.has_serde_derive();

        match &mut self.input.fields {
            Fields::Named(fields) => {
                // Add the private side buffer as requested
                // We always skip rkyv (as we auto-derive it)
                // We conditionally skip serde (only if the user derives it)
                if include_serde_skip {
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
                
                Ok(())
            }
            _ => {
                Err(syn::Error::new_spanned(
                    &self.input,
                    "capability_client only supports structs with named fields"
                ))
            }
        }
    }
}

pub fn expand(attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    let input: ItemStruct = parse2(item)?;
    let client = CapClient::new(attr, input)?;
    client.expand()
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    fn format_tokens(tokens: &TokenStream) -> String {
        // Try to parse and format as a file, fallback to raw string if it fails
        match syn::parse_file(&tokens.to_string()) {
            Ok(file) => prettyplease::unparse(&file),
            Err(err) => {
                tracing::error!(?err, "Parsing Error");
                tokens.to_string()
            },
        }
    }

    #[tracing_test::traced_test]
    #[test]
    fn test_client_without_serde() {
        let attr = quote! {};
        let item = quote! {
            pub struct MyClient {
                pub id: u32,
            }
        };

        let result = expand(attr, item).expect("Expansion failed");
        tracing::info!("Generated tokens: {}", format_tokens(&result));
        let result_str = result.to_string();

        assert!(result_str.contains("__config_buf : Vec < u8 >"));
        assert!(result_str.contains("# [rkyv (with = rkyv :: with :: Skip)]"));
        assert!(!result_str.contains("# [serde (skip)]"));
        
        tracing::debug!("Client without serde passed");
    }

    #[tracing_test::traced_test]
    #[test]
    fn test_client_with_serde_derive() {
        let attr = quote! {};
        let item = quote! {
            #[derive(Clone, Serialize, Deserialize)]
            pub struct MyClient {
                pub id: u32,
            }
        };

        let result = expand(attr, item).expect("Expansion failed");
        tracing::info!("Generated tokens: {}", format_tokens(&result));

        let result_str = result.to_string();

        assert!(result_str.contains("__config_buf : Vec < u8 >"));
        assert!(result_str.contains("# [serde (skip)]"));
        
        tracing::debug!("Client with simple serde derive passed");
    }

    #[tracing_test::traced_test]
    #[test]
    fn test_client_debug_override() {
        let attr = quote! {};
        let item = quote! {
            #[derive(Clone, Debug)]
            pub struct MyDebugClient {
                pub name: String,
            }
        };

        let result = expand(attr, item).expect("Expansion failed");
        tracing::info!("Generated tokens: {}", format_tokens(&result));

        let result_str = result.to_string();

        // Check that `derive(Clone, Debug)` became `derive(Clone)`
        // Note: Exact string match depends on spacing, but we check conceptually
        assert!(result_str.contains("derive (Clone)")); 
        
        // Ensure standard Debug derive is NOT present in the list (or list is just Clone)
        // We might see `derive (rkyv::Archive...)` which is added separately.
        
        // Check that a manual impl was generated
        assert!(result_str.contains("impl std :: fmt :: Debug for MyDebugClient"));
        assert!(result_str.contains("f . debug_struct"));
        
        // Check that fields are included
        assert!(result_str.contains(". field (\"name\" , & self . name)"));
        
        // Check that buffer is NOT included in the debug fields
        assert!(!result_str.contains(". field (\"__config_buf\""));

        tracing::debug!("Client debug override passed");
    }


    #[tracing_test::traced_test]
    #[test]
    fn test_client_trait_generation() {
        let attr = quote! {};
        let item = quote! {
            pub struct ConfiguredClient {
                data: u64
            }
        };

        let result = expand(attr, item).expect("Expansion failed");
        tracing::info!("Generated tokens: {}", format_tokens(&result));
        tracing::info!("Generated tokens: {}", &result);
        let result_str = result.to_string();

        // Check for trait implementation
        assert!(result_str.contains("impl :: pyroduct :: module_capability :: CapabilityClient for ConfiguredClient"));
        assert!(result_str.contains("& self . __config_buf"));

        tracing::debug!("Client trait generation passed");
    }
}