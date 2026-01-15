//! #[capability_server] - Marks a struct as server-side implementation

use heck::AsSnakeCase;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::punctuated::Punctuated;
use syn::{Expr, Ident, ItemStruct, Meta, Result, Token, Type, parse::Parser, parse2};

#[derive(Debug, Clone)]
pub struct ServerAttrs {
    //pub service: Ident,
    pub config: Type,
    pub is_async: bool,
}

pub fn parse_server_attrs(attr: TokenStream) -> Result<ServerAttrs> {
    // let mut service: Option<Ident> = None;
    let mut config: Option<Type> = None;

    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    let metas = parser.parse2(attr)?;
    let mut is_async = false;

    for meta in metas {
        match &meta {
            Meta::NameValue(nv) => {
                if nv.path.is_ident("service") {
                    // if let Expr::Path(path) = &nv.value {
                    //     service = path.path.get_ident().cloned();
                    // }
                } else if nv.path.is_ident("config") {
                    if let Expr::Path(path) = &nv.value {
                        config = Some(Type::Path(syn::TypePath {
                            qself: None,
                            path: path.path.clone(),
                        }));
                    }
                }
            }
            Meta::Path(path) => {
                if path.is_ident("async_init") {
                    is_async = true;
                }
            }
            _ => {}
        }
    }

    // let service = service.ok_or_else(|| {
    //     syn::Error::new(Span::call_site(), "Missing `service = TraitName` attribute")
    // })?;
    let config = config.ok_or_else(|| {
        syn::Error::new(Span::call_site(), "Missing `config = Struct` attribute")
    })?;

    Ok(ServerAttrs {
        //service,
        config,
        is_async,
    })
}

#[derive(Debug, Clone)]
pub struct CapServer {
    pub input: ItemStruct,
    pub attrs: ServerAttrs,
    pub struct_name: Ident,
    pub struct_vis: syn::Visibility,
    pub init_trait_name: Ident,
}

impl CapServer {
    pub fn new(attr: TokenStream, input: ItemStruct) -> Result<Self> {
        let attrs = parse_server_attrs(attr)?;
        let struct_name = input.ident.clone();
        let struct_vis = input.vis.clone();
        let init_trait_name = format_ident!("{}Init", struct_name);
        Ok(Self {
            input,
            attrs,
            struct_name,
            struct_vis,
            init_trait_name,
        })
    }

    pub fn name(&self) -> &Ident {
        &self.input.ident
    }

    pub fn generate_init_trait(&self) -> TokenStream {
        let init_trait_name = &self.init_trait_name;
        let struct_vis = self.struct_vis.clone();
        let config_type = &self.attrs.config;

        quote! {
            #struct_vis trait #init_trait_name {
                fn new(config: &#config_type) -> Self;
                fn reset(&mut self);
            }
        }
    }

    pub fn generate_init_fn(&self) -> (TokenStream, TokenStream) {
        let struct_name = self.struct_name.clone();
        let config_type = &self.attrs.config;
        let init_trait_name = &self.init_trait_name;
        let init_ffi_name = format_ident!("__{}_ffi_init",  &AsSnakeCase(self.input.ident.to_string()).to_string());
        if self.attrs.is_async {
            let ffi_func = quote! {
                #[unsafe(no_mangle)]
                pub extern "C" fn #init_ffi_name<'a>(
                    config_ptr: *const u8,
                    config_len: usize
                ) -> ::pyroduct::capability_host::ffi::FfiBorrowedFutureObjectResult<'a> {
                    unsafe {
                        ::pyroduct::capability::safe_lifecycle::execute_safe_async_init::<#config_type, #struct_name, _, _>(
                            config_ptr,
                            config_len,
                            |config| async move { <#struct_name as #init_trait_name>::new(&config).await },
                        )
                    }
                }
            };
            let ffi_export = quote!(::pyroduct::capability_host::ffi::PluginInitFn::Async(#init_ffi_name));
            (ffi_func, ffi_export)
        } else {
            let ffi_func = quote! {
                #[unsafe(no_mangle)]
                pub extern "C" fn #init_ffi_name(
                    config_ptr: *const u8,
                    config_len: usize
                ) -> ::pyroduct::capability_host::ffi::FfiInitResult {
                    unsafe {
                        ::pyroduct::capability::safe_lifecycle::execute_safe_init::<#config_type, #struct_name, _>(
                            config_ptr,
                            config_len,
                            |config| <#struct_name as #init_trait_name>::new(&config)
                        )
                    }
                }
            };
            let ffi_export = quote!(::pyroduct::capability_host::ffi::PluginInitFn::Sync(#init_ffi_name));
            (ffi_func, ffi_export)
        }
    }

    pub fn generate_drop_fn(&self) -> (TokenStream, TokenStream) {
        let struct_name = self.struct_name.clone();
        let drop_ffi_name = format_ident!("__{}_ffi_drop",  &AsSnakeCase(self.input.ident.to_string()).to_string());
        let ffi_func = quote! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn #drop_ffi_name(state: *mut std::ffi::c_void) {
                if !state.is_null() {
                    drop(unsafe { Box::from_raw(state as *mut #struct_name) });
                }
            }
        };
        let ffi_export = quote!(::pyroduct::capability_host::ffi::PluginDropFn::Sync(#drop_ffi_name));
        (ffi_func, ffi_export)
        
    }

    pub fn generate_reset_fn(&self) -> (TokenStream, TokenStream) {
        let struct_name = self.struct_name.clone();
        let init_trait_name = &self.init_trait_name;
        let reset_ffi_name = format_ident!("__{}_ffi_reset",  &AsSnakeCase(self.input.ident.to_string()).to_string());
        if self.attrs.is_async {
            let ffi_func = quote! {
                #[unsafe(no_mangle)]
                pub unsafe extern "C" fn #reset_ffi_name<'a>(state: *mut std::ffi::c_void) -> ::pyroduct::capability_host::ffi::FfiBorrowedFutureResult<'a> {
                    ::pyroduct::capability::safe_lifecycle::execute_safe_async_reset::<#struct_name, _, _>(
                        state,
                        |state| async move { <#struct_name as #init_trait_name>::reset(state).await }, 
                    )
                }
            };
            let ffi_export = quote!(::pyroduct::capability_host::ffi::PluginResetFn::Async(#reset_ffi_name));
            (ffi_func, ffi_export)
        } else {
            let ffi_func = quote! {
                #[unsafe(no_mangle)]
                pub unsafe extern "C" fn #reset_ffi_name(state: *mut std::ffi::c_void) -> ::pyroduct::capability_host::ffi::FfiResult {
                    ::pyroduct::capability::safe_lifecycle::execute_safe_reset::<#struct_name, _>(
                        state,
                        |state| <#struct_name as #init_trait_name>::reset(state)
                    )
                }
            };
            let ffi_export = quote!(::pyroduct::capability_host::ffi::PluginResetFn::Sync(#reset_ffi_name));
            (ffi_func, ffi_export)
        }
    }
}

pub fn expand(attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    let input: ItemStruct = parse2(item)?;
    let cap_server = CapServer::new(attr, input.clone())?;

    let input = cap_server.input.clone();
    let init_trait = cap_server.generate_init_trait();
    let (init_fn, _init_fn_export) = cap_server.generate_init_fn();
    let (drop_fn, _drop_fn_export) = cap_server.generate_drop_fn();
    let (reset_fn, _reset_fn_export) = cap_server.generate_reset_fn();


    let output = quote! {
        // The struct definition
        #input

        // Init trait (if not stateless)
        #init_trait

        // FFI lifecycle functions
        #init_fn
        #drop_fn
        #reset_fn
    };

    Ok(output)
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
    fn test_simple_stateful_server() {
        let attr = quote! { service = Greeter, config = GreeterConfig};
        let item = quote! {
            pub struct GreeterServer {
                count: u32,
            }
        };

        let result = expand(attr, item).expect("Expansion failed");
        tracing::info!("Generated tokens: {}", format_tokens(&result));

        tracing::info!("Generated tokens: {}", &result);


        let result_str = result.to_string();

        // Should generate init trait
        assert!(result_str.contains("pub trait GreeterServerInit"));
        assert!(result_str.contains("fn new (config : & GreeterConfig) -> Self"));
        assert!(result_str.contains("fn reset (& mut self)"));
        
        // Should NOT have with_config method (no config provided)
        assert!(!result_str.contains("fn with_config"));
        
        // Should generate plugin_init
        //assert!(result_str.contains("PluginInitFn :: Sync ( __greeter_server_ffi_init )"));
        assert!(result_str.contains("pub extern \"C\" fn __greeter_server_ffi_init"));
        assert!(result_str.contains("| config | < GreeterServer as GreeterServerInit > :: new (& config)"));
        assert!(result_str.contains("execute_safe_init :: < GreeterConfig , GreeterServer , _ >"));
        
        // Should generate plugin_drop
        //assert!(result_str.contains("PluginDropFn :: Sync ( __greeter_server_ffi_drop )"));
        assert!(result_str.contains("pub unsafe extern \"C\" fn __greeter_server_ffi_drop"));
        assert!(result_str.contains("Box :: from_raw (state as * mut GreeterServer)"));
        
        // Should generate plugin_reset
        //assert!(result_str.contains("PluginResetFn :: Sync ( __greeter_server_ffi_reset )"));
        assert!(result_str.contains("pub unsafe extern \"C\" fn __greeter_server_ffi_reset"));
        assert!(result_str.contains("execute_safe_reset :: < GreeterServer , _ >"));
        assert!(result_str.contains("< GreeterServer as GreeterServerInit > :: reset (state)"));
        
        tracing::debug!("Simple stateful server test passed");
    }

    #[tracing_test::traced_test]
    #[test]
    fn test_simple_stateful_async_server() {
        let attr = quote! { service = Greeter, config = GreeterConfig, async_init};
        let item = quote! {
            pub struct GreeterServer {
                count: u32,
            }
        };

        let result = expand(attr, item).expect("Expansion failed");
        tracing::info!("Generated tokens: {}", format_tokens(&result));

        tracing::info!("Generated tokens: {}", &result);


        let result_str = result.to_string();

        // Should generate init trait
        assert!(result_str.contains("pub trait GreeterServerInit"));
        assert!(result_str.contains("fn new (config : & GreeterConfig) -> Self"));
        assert!(result_str.contains("fn reset (& mut self)"));
        
        // Should NOT have with_config method (no config provided)
        assert!(!result_str.contains("fn with_config"));
        
        // Should generate plugin_init
        //assert!(result_str.contains("PluginInitFn :: Sync ( __greeter_server_ffi_init )"));
        assert!(result_str.contains("pub extern \"C\" fn __greeter_server_ffi_init < 'a >"));
        assert!(result_str.contains("| config | async move { < GreeterServer as GreeterServerInit > :: new (& config) . await"));
        assert!(result_str.contains("execute_safe_async_init :: < GreeterConfig , GreeterServer , _ , _ >"));
        
        // Should generate plugin_drop
        //assert!(result_str.contains("PluginDropFn :: Sync ( __greeter_server_ffi_drop )"));
        assert!(result_str.contains("pub unsafe extern \"C\" fn __greeter_server_ffi_drop"));
        assert!(result_str.contains("Box :: from_raw (state as * mut GreeterServer)"));
        
        // Should generate plugin_reset
        //assert!(result_str.contains("PluginResetFn :: Sync ( __greeter_server_ffi_reset )"));
        assert!(result_str.contains("pub unsafe extern \"C\" fn __greeter_server_ffi_reset < 'a >"));
        assert!(result_str.contains("execute_safe_async_reset :: < GreeterServer , _ , _ >"));
        assert!(result_str.contains("| state | async move { < GreeterServer as GreeterServerInit > :: reset (state) . await }"));
        
        tracing::debug!("Simple stateful server test passed");
    }
}