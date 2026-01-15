//! capability_export! - Generates the plugin manifest

use heck::AsSnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Ident, Item, LitStr, Result, Token, parse2};

use crate::capability_function::CapFn;
use crate::capability_server::{CapServer};

#[derive(Debug, Clone)]
pub enum ExportItem {
    Function(CapFn),
    Server(CapServer),
}

pub struct CapabilityExport {
    pub path: Ident,
    pub items: Vec<ExportItem>,
}

impl CapabilityExport {
    pub fn new(path: Ident, items: Vec<ExportItem>) -> Self {
        Self { path, items }
    }

    pub fn generate_function_export(&self, cap_fn: &CapFn) -> TokenStream {
        let fn_name = &cap_fn.to_ffi(&self.path).fn_name;
        let capability_name = format_ident!("capability_{}", fn_name);
        let capability_name_str = capability_name.to_string();
        let is_async = cap_fn.to_ffi(&self.path).is_async;

        let func_variant = if is_async {
            quote!(::pyroduct::capability_host::ffi::PluginFunction::Async(#capability_name))
        } else {
            quote!(::pyroduct::capability_host::ffi::PluginFunction::Sync(#capability_name))
        };

        quote! {
            ::pyroduct::capability_host::ffi::PluginExport {
                module: MOD_NAME.as_ptr(),
                module_len: MOD_NAME.len(),
                name: #capability_name_str.as_ptr(),
                name_len: #capability_name_str.len(),
                func: #func_variant,
            }
        }
    }

    pub fn generate_function_definitions(&self) -> Vec<TokenStream> {
        self.items
            .iter()
            .filter_map(|item| match item {
                ExportItem::Function(cap_fn) => {
                    let input = &cap_fn.input;
                    let input_struct = cap_fn.to_ffi(&self.path).generate_input_struct();
                    let capability_fn = cap_fn.to_ffi(&self.path).generate_capability_ffi();
                    
                    Some(quote! {
                        #[cfg(feature = "capability")]
                        #input

                        #[cfg(feature = "capability")]
                        #input_struct

                        #[cfg(feature = "capability")]
                        #capability_fn
                    })
                }
                ExportItem::Server(_) => None,
            })
            .collect()
    }

    pub fn generate_server_definitions(&self) -> Vec<TokenStream> {
        self.items
            .iter()
            .filter_map(|item| match item {
                ExportItem::Server(server) => {
                    // Server definitions are handled by capability_server and capability_impl macros
                    // We just need to reference them for the manifest
                    None
                }
                ExportItem::Function(_) => None,
            })
            .collect()
    }

    pub fn generate_function_exports(&self) -> Vec<TokenStream> {
        self.items
            .iter()
            .filter_map(|item| match item {
                ExportItem::Function(cap_fn) => Some(self.generate_function_export(cap_fn)),
                ExportItem::Server(_) => None,
            })
            .collect()
    }

    pub fn has_servers(&self) -> bool {
        self.items.iter().any(|item| matches!(item, ExportItem::Server(_)))
    }

    pub fn get_first_server(&self) -> Option<&CapServer> {
        self.items.iter().find_map(|item| match item {
            ExportItem::Server(s) => Some(s),
            _ => None,
        })
    }

    pub fn generate_manifest(&self) -> Result<TokenStream> {
        if self.has_servers() {
            let server = self.get_first_server()
                .ok_or_else(|| syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "Expected server but none found"
                ))?;
            self.generate_server_manifest(server)
        } else {
            self.generate_functions_manifest()
        }
    }

    fn generate_functions_manifest(&self) -> Result<TokenStream> {
        let path = &self.path;
        let exports = self.generate_function_exports();
        let definitions = self.generate_function_definitions();

        Ok(quote! {
            // Generate all function definitions
            #(#definitions)*

            #[cfg(feature = "capability")]
            #[unsafe(no_mangle)]
            pub extern "C" fn plugin_manifest(
                id: u64,
                log_callback: ::pyroduct::capability_host::ffi::LogCallback
            ) -> ::pyroduct::capability_host::ffi::PluginExports {
                static MOD_NAME: &str = #path;

                ::pyroduct::capability_host::ffi::init_logging(id, log_callback);

                let mut export_vec = vec![
                    #(#exports),*
                ];

                let exports = ::pyroduct::capability_host::ffi::PluginExports {
                    len: export_vec.len(),
                    cap: export_vec.capacity(),
                    ptr: export_vec.as_mut_ptr(),
                    reset: ::pyroduct::capability_host::ffi::PluginResetFn::Null,
                    init: ::pyroduct::capability_host::ffi::PluginInitFn::Null,
                    drop: ::pyroduct::capability_host::ffi::PluginDropFn::Null,
                };
                std::mem::forget(export_vec);
                exports
            }
        })
    }

    fn generate_server_manifest(&self, server: &CapServer) -> Result<TokenStream> {
        let server_name = server.struct_name.clone();
        let ffi_mod_name = format_ident!("__{}_ffi",  &AsSnakeCase(server_name.to_string()).to_string());

        Ok(quote! {
            #[cfg(feature = "capability")]
            #[unsafe(no_mangle)]
            pub extern "C" fn plugin_manifest(
                id: u64,
                log_callback: ::pyroduct::capability_host::ffi::LogCallback
            ) -> ::pyroduct::capability_host::ffi::PluginExports {
                ::pyroduct::capability_host::ffi::init_logging(id, log_callback);

                // Get exports from the server implementation
                let mut export_vec = #server_name::__capability_exports();

                let exports = ::pyroduct::capability_host::ffi::PluginExports {
                    len: export_vec.len(),
                    cap: export_vec.capacity(),
                    ptr: export_vec.as_mut_ptr(),
                    reset: #ffi_mod_name::RESET_FN,
                    init: #ffi_mod_name::INIT_FN,
                    drop: #ffi_mod_name::DROP_FN,
                };
                std::mem::forget(export_vec);
                exports
            }
        })
    }
}

struct ExportInput {
    env: String,
    items: Vec<Item>,
}

impl Parse for ExportInput {
    fn parse(input: ParseStream) -> Result<Self> {
        // Parse: env = "..."
        let _env_ident: syn::Ident = input.parse()?;
        let _: Token![=] = input.parse()?;
        let env_lit: LitStr = input.parse()?;
        let env = env_lit.value();

        // Optional comma after env
        if input.peek(Token![,]) {
            let _: Token![,] = input.parse()?;
        }

        // Parse all items until end
        let mut items = Vec::new();
        while !input.is_empty() {
            items.push(input.parse()?);
        }

        Ok(ExportInput { env, items })
    }
}

pub fn expand(input: TokenStream) -> Result<TokenStream> {
    let parsed: ExportInput = parse2(input)?;

    let mut export_items = Vec::new();

    for item in parsed.items {
        match item {
            Item::Fn(item_fn) => {
                // Parse as capability function
                let cap_fn = CapFn::new(item_fn)?;
                export_items.push(ExportItem::Function(cap_fn));
            }
            Item::Struct(item_struct) => {
                // Look for #[capability_server] attribute to extract ServerAttrs
                let server_attr = item_struct
                    .attrs
                    .iter()
                    .find(|attr| {
                        attr.path()
                            .segments
                            .last()
                            .map(|seg| seg.ident == "capability_server")
                            .unwrap_or(false)
                    });

                if let Some(attr) = server_attr {
                    // Parse the attribute to get ServerAttrs
                    let attr_tokens = attr.meta.require_list()?.tokens.clone();
                    let cap_server = CapServer::new(attr_tokens, item_struct)?;
                    export_items.push(ExportItem::Server(cap_server));
                }
            }
            Item::Impl(_item_impl) => {
                // Impl blocks are handled by their associated struct
                // They don't need separate export entries
                continue;
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    item,
                    "Only functions, structs with #[capability_server], and impl blocks are supported in capability_export"
                ));
            }
        }
    }

    let export = CapabilityExport::new(format_ident!("{}", parsed.env), export_items);
    export.generate_manifest()
}