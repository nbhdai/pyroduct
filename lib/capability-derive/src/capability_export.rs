//! capability! - Unified capability export macro
//!
//! This macro handles all capability exports in a single place, generating:
//! - FFI host functions for servers and standalone functions
//! - WASM client imports and wrapper functions  
//! - The plugin_manifest export returning CapabilityExports VTable
//!
//! Usage:
//! ```ignore
//! capability! {
//!     env = "my_capability",
//!     
//!     // Client state (optional)
//!     #[capability_client]
//!     pub struct MyClient { ... }
//!     
//!     // Capability trait
//!     #[capability]
//!     pub trait MyCapability { ... }
//!     
//!     // Server implementation (optional, for stateful capabilities)
//!     #[capability_server(service = MyCapability, config = MyConfig)]
//!     pub struct MyServer { ... }
//!     
//!     // Standalone functions (optional)
//!     #[capability_function]  
//!     pub fn standalone_func() -> u32 { ... }
//!     
//!     // Trait implementation
//!     impl MyCapability for MyServer { ... }
//! }
//! ```

use heck::AsSnakeCase;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{
    Attribute, Error, Ident, Item, ItemFn, ItemImpl, ItemStruct, ItemTrait, LitStr, Result, Token,
    braced, parse2,
};

use crate::capability::CapabilityDefTrait;
use crate::capability_client::CapClient;
use crate::capability_function::CapFn;
use crate::capability_server::CapServer;

/// Parsed content from the capability! macro
#[derive(Debug)]
pub struct CapabilityModule {
    /// Environment/module name for WASM imports (e.g., "http_client")
    pub env: String,
    /// Client struct marked with #[capability_client]
    pub client: Option<ParsedClient>,
    /// Capability trait marked with #[capability]
    pub capability_trait: Option<ParsedTrait>,
    /// Server struct marked with #[capability_server]
    pub server: Option<ParsedServer>,
    /// Standalone functions marked with #[capability_function]
    pub functions: Vec<ParsedFunction>,
    /// Impl blocks for the capability trait
    pub impls: Vec<ItemImpl>,
    /// Other items to pass through unchanged
    pub other_items: Vec<Item>,
}

#[derive(Debug)]
pub struct ParsedClient {
    pub attrs: TokenStream,
    pub item: ItemStruct,
}

#[derive(Debug)]
pub struct ParsedTrait {
    pub attrs: TokenStream,
    pub item: ItemTrait,
    pub is_stateless: bool,
}

#[derive(Debug)]
pub struct ParsedServer {
    pub attrs: TokenStream,
    pub item: ItemStruct,
    pub service: Option<Ident>,
    pub config: Option<Ident>,
    pub is_stateless: bool,
    pub is_async: bool,
}

#[derive(Debug)]
pub struct ParsedFunction {
    pub attrs: TokenStream,
    pub item: ItemFn,
}

impl Parse for CapabilityModule {
    fn parse(input: ParseStream) -> Result<Self> {
        // Parse: env = "..."
        let _env_ident: Ident = input.parse()?;
        let _: Token![=] = input.parse()?;
        let env_lit: LitStr = input.parse()?;
        let env = env_lit.value();

        // Optional comma after env
        if input.peek(Token![,]) {
            let _: Token![,] = input.parse()?;
        }

        let mut client = None;
        let mut capability_trait = None;
        let mut server = None;
        let mut functions = Vec::new();
        let mut impls = Vec::new();
        let mut other_items = Vec::new();

        // Parse all items
        while !input.is_empty() {
            let item: Item = input.parse()?;

            match item {
                Item::Struct(mut item_struct) => {
                    if let Some((attr_idx, parsed_attrs)) =
                        find_capability_attr(&item_struct.attrs, "capability_client")
                    {
                        item_struct.attrs.remove(attr_idx);
                        client = Some(ParsedClient {
                            attrs: parsed_attrs,
                            item: item_struct,
                        });
                    } else if let Some((attr_idx, parsed_attrs)) =
                        find_capability_attr(&item_struct.attrs, "capability_server")
                    {
                        let (service, config, is_stateless, is_async) =
                            parse_server_attrs(&parsed_attrs)?;
                        item_struct.attrs.remove(attr_idx);
                        server = Some(ParsedServer {
                            attrs: parsed_attrs,
                            item: item_struct,
                            service,
                            config,
                            is_stateless,
                            is_async,
                        });
                    } else {
                        other_items.push(Item::Struct(item_struct));
                    }
                }
                Item::Trait(mut item_trait) => {
                    if let Some((attr_idx, parsed_attrs)) =
                        find_capability_attr(&item_trait.attrs, "capability")
                    {
                        let is_stateless = parsed_attrs.to_string().contains("stateless");
                        item_trait.attrs.remove(attr_idx);
                        capability_trait = Some(ParsedTrait {
                            attrs: parsed_attrs,
                            item: item_trait,
                            is_stateless,
                        });
                    } else {
                        other_items.push(Item::Trait(item_trait));
                    }
                }
                Item::Fn(mut item_fn) => {
                    if let Some((attr_idx, parsed_attrs)) =
                        find_capability_attr(&item_fn.attrs, "capability_function")
                    {
                        item_fn.attrs.remove(attr_idx);
                        functions.push(ParsedFunction {
                            attrs: parsed_attrs,
                            item: item_fn,
                        });
                    } else {
                        other_items.push(Item::Fn(item_fn));
                    }
                }
                Item::Impl(item_impl) => {
                    // Check if this is a capability impl (impl Trait for Server)
                    if item_impl.trait_.is_some() {
                        impls.push(item_impl);
                    } else {
                        other_items.push(Item::Impl(item_impl));
                    }
                }
                other => {
                    other_items.push(other);
                }
            }
        }

        Ok(CapabilityModule {
            env,
            client,
            capability_trait,
            server,
            functions,
            impls,
            other_items,
        })
    }
}

/// Find an attribute by name and return its index and token content
fn find_capability_attr(attrs: &[Attribute], name: &str) -> Option<(usize, TokenStream)> {
    attrs.iter().enumerate().find_map(|(idx, attr)| {
        let path = attr.path();
        if path
            .segments
            .last()
            .map(|s| s.ident == name)
            .unwrap_or(false)
        {
            let tokens = match &attr.meta {
                syn::Meta::Path(_) => quote!(),
                syn::Meta::List(list) => list.tokens.clone(),
                syn::Meta::NameValue(nv) => quote!(#nv),
            };
            Some((idx, tokens))
        } else {
            None
        }
    })
}

/// Parse server attribute tokens to extract service, config, stateless, async flags
fn parse_server_attrs(tokens: &TokenStream) -> Result<(Option<Ident>, Option<Ident>, bool, bool)> {
    let mut service = None;
    let mut config = None;
    let mut is_stateless = false;
    let mut is_async = false;

    let token_str = tokens.to_string();

    // Parse service = Ident
    if let Some(idx) = token_str.find("service") {
        let rest = &token_str[idx..];
        if let Some(eq_idx) = rest.find('=') {
            let after_eq = rest[eq_idx + 1..].trim();
            let end = after_eq
                .find(|c: char| c == ',' || c == ')')
                .unwrap_or(after_eq.len());
            let ident_str = after_eq[..end].trim();
            if !ident_str.is_empty() {
                service = Some(format_ident!("{}", ident_str));
            }
        }
    }

    // Parse config = Ident
    if let Some(idx) = token_str.find("config") {
        let rest = &token_str[idx..];
        if let Some(eq_idx) = rest.find('=') {
            let after_eq = rest[eq_idx + 1..].trim();
            let end = after_eq
                .find(|c: char| c == ',' || c == ')')
                .unwrap_or(after_eq.len());
            let ident_str = after_eq[..end].trim();
            if !ident_str.is_empty() {
                config = Some(format_ident!("{}", ident_str));
            }
        }
    }

    is_stateless = token_str.contains("stateless");
    is_async = token_str.contains("async");

    Ok((service, config, is_stateless, is_async))
}

impl CapabilityModule {
    /// Generate all output code
    pub fn generate(&self) -> Result<TokenStream> {
        let env_ident = format_ident!("{}", self.env);
        let env_str = &self.env;

        // 1. Generate client definitions (always compiled)
        let client_defs = self.generate_client_definitions()?;

        // 2. Generate trait definition (always compiled)
        let trait_defs = self.generate_trait_definitions()?;

        // 3. Generate other items (always compiled)
        let other_items = &self.other_items;

        // 4. Generate module-side (WASM) code
        let module_code = self.generate_module_code(&env_ident)?;

        // 5. Generate capability-side (host FFI) code in pub mod capability {}
        let capability_mod = self.generate_capability_module(&env_ident, env_str)?;

        Ok(quote! {
            // ============================================================
            // SHARED DEFINITIONS (always compiled)
            // ============================================================

            #client_defs
            #trait_defs
            #(#other_items)*

            // ============================================================
            // MODULE-SIDE CODE (WASM clients)
            // Written in root namespace for module use
            // ============================================================

            #module_code

            // ============================================================
            // CAPABILITY-SIDE CODE (Host FFI)
            // Written in pub mod capability {}
            // ============================================================

            #capability_mod
        })
    }

    /// Generate client struct definitions
    fn generate_client_definitions(&self) -> Result<TokenStream> {
        if let Some(client) = &self.client {
            let cap_client = CapClient::new(client.attrs.clone(), client.item.clone())?;
            cap_client.expand()
        } else {
            Ok(quote!())
        }
    }

    /// Generate trait definitions
    fn generate_trait_definitions(&self) -> Result<TokenStream> {
        if let Some(trait_def) = &self.capability_trait {
            let trait_item = &trait_def.item;
            let trait_tokens = quote!(#trait_item);

            // Parse through CapabilityDefTrait to get proper trait generation
            let cap_def = CapabilityDefTrait::from_trait(trait_tokens)?;
            cap_def.generate_trait_definition()
        } else {
            Ok(quote!())
        }
    }

    /// Generate module-side (WASM) code - client wrapper functions
    fn generate_module_code(&self, env: &Ident) -> Result<TokenStream> {
        let mut wasm_imports = Vec::new();
        let mut client_functions = Vec::new();

        // Generate WASM imports and client wrappers for standalone functions
        for func in &self.functions {
            let cap_fn = CapFn::new(func.item.clone())?;
            let ffi = cap_fn.to_ffi(env);

            // Generate the extern "C" import declaration
            let wasm_import = ffi.generate_client_wasm();
            wasm_imports.push(wasm_import);

            // Generate the public wrapper function
            let client_fn = cap_fn.generate_module_function(env);
            client_functions.push(client_fn);
        }

        // Generate WASM imports and client methods for trait methods
        if let (Some(trait_def), Some(server)) = (&self.capability_trait, &self.server) {
            // Find the impl block for this trait
            let trait_name = &trait_def.item.ident;
            let server_name = &server.item.ident;

            for impl_block in &self.impls {
                if let Some((_, path, _)) = &impl_block.trait_ {
                    let impl_trait_name = path.segments.last().map(|s| &s.ident);
                    if impl_trait_name == Some(trait_name) {
                        // Parse as CapabilityDefTrait to generate client code
                        let impl_tokens = quote!(#impl_block);
                        let cap_def = CapabilityDefTrait::from_impl(impl_tokens)?;

                        // Generate WASM imports
                        let imports = cap_def.generate_wasm_import_functions()?;
                        wasm_imports.push(imports);

                        // Generate client impl if there's a client type
                        let client_impl = cap_def.generate_client_impl()?;
                        if !client_impl.is_empty() {
                            client_functions.push(client_impl);
                        }
                    }
                }
            }
        }

        if wasm_imports.is_empty() && client_functions.is_empty() {
            return Ok(quote!());
        }

        let env_str = env.to_string();

        Ok(quote! {
            #[cfg(feature = "module")]
            mod __capability_module_imports {
                use super::*;

                #[link(wasm_import_module = #env_str)]
                extern "C" {
                    #(#wasm_imports)*
                }
            }

            #(#client_functions)*
        })
    }

    /// Generate the capability module with all host-side FFI code
    fn generate_capability_module(&self, env: &Ident, env_str: &str) -> Result<TokenStream> {
        let mut server_code = quote!();
        let mut function_defs = Vec::new();
        let mut function_exports = Vec::new();
        let mut impl_code = quote!();

        // Process server if present
        let (init_fn, drop_fn, reset_fn) = if let Some(server) = &self.server {
            let cap_server = CapServer::new(server.attrs.clone(), server.item.clone())?;

            let init_trait = cap_server.generate_init_trait();
            let (init_fn_code, init_export) = cap_server.generate_init_fn();
            let (drop_fn_code, drop_export) = cap_server.generate_drop_fn();
            let (reset_fn_code, reset_export) = cap_server.generate_reset_fn();

            let server_struct = &server.item;

            server_code = quote! {
                #server_struct
                #init_trait
                #init_fn_code
                #drop_fn_code
                #reset_fn_code
            };

            (init_export, drop_export, reset_export)
        } else {
            (
                quote!(::pyroduct::capability_host::ffi::PluginInitFn::Null),
                quote!(::pyroduct::capability_host::ffi::PluginDropFn::Null),
                quote!(::pyroduct::capability_host::ffi::PluginResetFn::Null),
            )
        };

        // Process standalone functions
        for func in &self.functions {
            let cap_fn = CapFn::new(func.item.clone())?;
            let ffi = cap_fn.to_ffi(env);

            // Original function
            let fn_item = &func.item;
            function_defs.push(quote! { #fn_item });

            // Input struct if needed
            let input_struct = ffi.generate_input_struct();
            if !input_struct.is_empty() {
                function_defs.push(input_struct);
            }

            // FFI wrapper
            let capability_fn = ffi.generate_capability_ffi();
            function_defs.push(capability_fn);

            // Export entry
            let fn_name = &ffi.fn_name;
            let fn_ffi_name = &ffi.fn_ffi_name;
            let fn_ffi_name_str = fn_ffi_name.to_string();
            let func_variant = if ffi.is_async {
                quote!(::pyroduct::capability_host::ffi::PluginFunction::Async(#fn_ffi_name))
            } else {
                quote!(::pyroduct::capability_host::ffi::PluginFunction::Sync(#fn_ffi_name))
            };

            function_exports.push(quote! {
                ::pyroduct::capability_host::ffi::PluginExport {
                    module: MOD_NAME.as_ptr(),
                    module_len: MOD_NAME.len(),
                    name: #fn_ffi_name_str.as_ptr(),
                    name_len: #fn_ffi_name_str.len(),
                    func: #func_variant,
                }
            });
        }

        // Process trait implementations
        if let Some(server) = &self.server {
            let server_name = &server.item.ident;

            for impl_block in &self.impls {
                if let Some((_, path, _)) = &impl_block.trait_ {
                    // Generate FFI functions from the impl
                    let impl_tokens = quote!(#impl_block);
                    let cap_def = CapabilityDefTrait::from_impl(impl_tokens.clone())?;

                    // Generate host FFI functions
                    let host_ffis = cap_def.generate_host_ffi_functions()?;

                    // Generate __capability_exports() method
                    let trait_name = path.segments.last().map(|s| &s.ident).unwrap();
                    let exports_method = generate_capability_exports_method(
                        &cap_def,
                        env_str,
                        trait_name,
                        server_name,
                    )?;

                    impl_code = quote! {
                        #impl_code

                        #impl_block

                        impl #server_name {
                            #host_ffis
                            #exports_method
                        }
                    };
                }
            }
        }

        // Generate the plugin_manifest function
        let manifest_fn = if self.server.is_some() {
            let server_name = &self.server.as_ref().unwrap().item.ident;
            let ffi_mod_name =
                format_ident!("__{}_ffi", AsSnakeCase(server_name.to_string()).to_string());

            quote! {
                #[unsafe(no_mangle)]
                pub extern "C" fn plugin_manifest<'a>(
                    id: u64,
                    log_callback: ::pyroduct::capability_host::ffi::LogCallback
                ) -> ::pyroduct::capability_host::ffi::PluginExports<'a> {
                    ::pyroduct::capability_host::ffi::init_logging(id, log_callback);

                    // Get exports from the server implementation
                    let mut export_vec = #server_name::__capability_exports();

                    // Add standalone function exports
                    #(export_vec.push(#function_exports);)*

                    let exports = ::pyroduct::capability_host::ffi::PluginExports {
                        len: export_vec.len(),
                        cap: export_vec.capacity(),
                        ptr: export_vec.as_mut_ptr(),
                        init: #init_fn,
                        drop: #drop_fn,
                        reset: #reset_fn,
                    };
                    std::mem::forget(export_vec);
                    exports
                }
            }
        } else {
            // Functions only - no server
            quote! {
                #[unsafe(no_mangle)]
                pub extern "C" fn plugin_manifest<'a>(
                    id: u64,
                    log_callback: ::pyroduct::capability_host::ffi::LogCallback
                ) -> ::pyroduct::capability_host::ffi::PluginExports<'a> {
                    static MOD_NAME: &str = #env_str;

                    ::pyroduct::capability_host::ffi::init_logging(id, log_callback);

                    let mut export_vec = vec![
                        #(#function_exports),*
                    ];

                    let exports = ::pyroduct::capability_host::ffi::PluginExports {
                        len: export_vec.len(),
                        cap: export_vec.capacity(),
                        ptr: export_vec.as_mut_ptr(),
                        init: ::pyroduct::capability_host::ffi::PluginInitFn::Null,
                        drop: ::pyroduct::capability_host::ffi::PluginDropFn::Null,
                        reset: ::pyroduct::capability_host::ffi::PluginResetFn::Null,
                    };
                    std::mem::forget(export_vec);
                    exports
                }
            }
        };

        Ok(quote! {
            #[cfg(feature = "capability")]
            pub mod capability {
                use super::*;

                static MOD_NAME: &str = #env_str;

                #server_code

                #(#function_defs)*

                #impl_code

                #manifest_fn
            }
        })
    }
}

/// Generate the __capability_exports() method for a server
fn generate_capability_exports_method(
    cap_def: &CapabilityDefTrait,
    env_str: &str,
    trait_name: &Ident,
    server_name: &Ident,
) -> Result<TokenStream> {
    let mut export_entries = Vec::new();

    // Generate export entries for each method
    for method in &cap_def.methods {
        let method_name = &method.name;
        let ffi_name = format_ident!("__{}_{}_{}_ffi", trait_name, server_name, method_name);
        let ffi_name_str = format!("host_{}", method_name);

        let is_async = method.original_sig.asyncness.is_some();
        let func_variant = if is_async {
            quote!(::pyroduct::capability_host::ffi::PluginFunction::Async(Self::#ffi_name))
        } else {
            quote!(::pyroduct::capability_host::ffi::PluginFunction::Sync(Self::#ffi_name))
        };

        export_entries.push(quote! {
            ::pyroduct::capability_host::ffi::PluginExport {
                module: #env_str.as_ptr(),
                module_len: #env_str.len(),
                name: #ffi_name_str.as_ptr(),
                name_len: #ffi_name_str.len(),
                func: #func_variant,
            }
        });
    }

    // Add constructor if present
    if !cap_def.constructors.is_empty() {
        let ffi_name = format_ident!("__{}_{}_{}_ffi", trait_name, server_name, "new_client");
        let ffi_name_str = format!("host_{}_new_client", AsSnakeCase(trait_name.to_string()));

        export_entries.push(quote! {
            ::pyroduct::capability_host::ffi::PluginExport {
                module: #env_str.as_ptr(),
                module_len: #env_str.len(),
                name: #ffi_name_str.as_ptr(),
                name_len: #ffi_name_str.len(),
                func: ::pyroduct::capability_host::ffi::PluginFunction::Sync(Self::#ffi_name),
            }
        });
    }

    Ok(quote! {
        pub fn __capability_exports() -> Vec<::pyroduct::capability_host::ffi::PluginExport<'static>> {
            vec![
                #(#export_entries),*
            ]
        }
    })
}

/// Main expansion function called by the proc macro
pub fn expand(input: TokenStream) -> Result<TokenStream> {
    let module: CapabilityModule = parse2(input)?;
    module.generate()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_module() {
        let input = quote! {
            env = "test_cap",

            #[capability_function]
            pub fn get_value() -> u32 {
                42
            }
        };

        let module: CapabilityModule = parse2(input).expect("Failed to parse");
        assert_eq!(module.env, "test_cap");
        assert_eq!(module.functions.len(), 1);
        assert!(module.client.is_none());
        assert!(module.server.is_none());
    }

    #[test]
    fn test_parse_with_server() {
        let input = quote! {
            env = "reporter",

            #[capability_server(service = Reporter, config = ReporterConfig)]
            pub struct ReporterServer {
                count: u32,
            }
        };

        let module: CapabilityModule = parse2(input).expect("Failed to parse");
        assert_eq!(module.env, "reporter");
        assert!(module.server.is_some());
        let server = module.server.unwrap();
        assert_eq!(server.service.unwrap().to_string(), "Reporter");
        assert_eq!(server.config.unwrap().to_string(), "ReporterConfig");
    }
}
