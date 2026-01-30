//! #[capability_server] - Marks a struct as server-side implementation

use heck::AsSnakeCase;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::punctuated::Punctuated;
use syn::{Expr, Ident, ItemStruct, Meta, Result, Token, Type, parse::Parser};

#[derive(Debug, Clone)]
pub struct ServerAttrs {
    pub config: Type,
    pub is_async: bool,
}

pub fn parse_server_attrs(attr: TokenStream) -> Result<ServerAttrs> {
    let mut config: Option<Type> = None;

    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    let metas = parser.parse2(attr)?;
    let mut is_async = false;

    for meta in metas {
        match &meta {
            Meta::NameValue(nv) => {
                if nv.path.is_ident("config") {
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
    let config = config
        .ok_or_else(|| syn::Error::new(Span::call_site(), "Missing `config = Struct` attribute"))?;

    Ok(ServerAttrs {
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

    fn export_init_ident(&self) -> Ident {
        format_ident!(
            "__{}__ffi_init",
            &AsSnakeCase(self.input.ident.to_string()).to_string()
        )
    }

    pub fn generate_init_fn(&self) -> TokenStream {
        let struct_name = self.struct_name.clone();
        let config_type = &self.attrs.config;
        let init_trait_name = &self.init_trait_name;
        let init_ffi_name = self.export_init_ident();

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
            ffi_func
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
            ffi_func
        }
    }

    pub fn generate_init_export(&self) -> TokenStream {
        let init_ffi_name = self.export_init_ident();

        if self.attrs.is_async {
            quote!(::pyroduct::capability_host::ffi::ClassInitFn::Async(#init_ffi_name))
        } else {
            quote!(::pyroduct::capability_host::ffi::ClassInitFn::Sync(#init_ffi_name))
        }
    }

    fn export_drop_ident(&self) -> Ident {
        format_ident!(
            "__{}__ffi_drop",
            &AsSnakeCase(self.input.ident.to_string()).to_string()
        )
    }

    pub fn generate_drop_fn(&self) -> TokenStream {
        let struct_name = self.struct_name.clone();
        let drop_ffi_name = self.export_drop_ident();
        let ffi_func = quote! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn #drop_ffi_name(state: *mut std::ffi::c_void) {
                if !state.is_null() {
                    drop(unsafe { Box::from_raw(state as *mut #struct_name) });
                }
            }
        };
        ffi_func
    }

    pub fn generate_drop_export(&self) -> TokenStream {
        let drop_ffi_name = self.export_drop_ident();
        quote!(::pyroduct::capability_host::ffi::ClassDropFn::Sync(#drop_ffi_name))
    }

    fn export_reset_ident(&self) -> Ident {
        format_ident!(
            "__{}__ffi_reset",
            &AsSnakeCase(self.input.ident.to_string()).to_string()
        )
    }

    pub fn generate_reset_fn(&self) -> TokenStream {
        let struct_name = self.struct_name.clone();
        let init_trait_name = &self.init_trait_name;
        let reset_ffi_name = self.export_reset_ident();
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
            ffi_func
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
            ffi_func
        }
    }

    pub fn generate_reset_export(&self) -> TokenStream {
        let reset_ffi_name = self.export_reset_ident();
        if self.attrs.is_async {
            quote!(::pyroduct::capability_host::ffi::ClassResetFn::Async(#reset_ffi_name))
        } else {
            quote!(::pyroduct::capability_host::ffi::ClassResetFn::Sync(#reset_ffi_name))
        }
    }

    pub fn generate_ffi_exports(&self) -> TokenStream {
        let class_name_static = &self.struct_name;

        let init_ffi_func = self.generate_init_fn();
        let drop_ffi_func = self.generate_drop_fn();
        let reset_ffi_func = self.generate_reset_fn();

        // Generate init, drop, and reset function pointers
        let init_export = self.generate_init_export();
        let drop_export = self.generate_drop_export();
        let reset_export = self.generate_reset_export();

        // Generate the static export array name
        let exports_array_name = quote::format_ident!("{}__METHODS", class_name_static);
        let plugin_exports_name = quote::format_ident!("{}__EXPORT", class_name_static);
        
        quote! {
            #init_ffi_func
            #drop_ffi_func
            #reset_ffi_func

            // Generate the ClassExport struct
            const #plugin_exports_name: ::pyroduct::capability_host::ffi::ClassExport = ::pyroduct::capability_host::ffi::ClassExport {
                ptr: super::methods::#exports_array_name.as_ptr(),
                init: #init_export,
                drop: #drop_export,
                reset: #reset_export,
                len: super::methods::#exports_array_name.len(),
            };
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use syn::parse2;
    #[tracing_test::traced_test]
    #[test]
    fn test_sync_server_init_trait() {
        let attr = quote! { service = Greeter, config = GreeterConfig };
        let item = quote! {
            pub struct GreeterServer {
                count: u32,
            }
        };

        let item = parse2(item).expect("error");
        let server = CapServer::new(attr, item).expect("Expansion failed");
        let result = server.generate_init_trait();

        let expected = quote! {
            pub trait GreeterServerInit {
                fn new(config: &GreeterConfig) -> Self;
                fn reset(&mut self);
            }
        };

        crate::fmt::assert_code_eq_token(&result, &expected);
    }

    #[tracing_test::traced_test]
    #[test]
    fn test_sync_server_init_fn() {
        let attr = quote! { service = Greeter, config = GreeterConfig };
        let item = quote! {
            pub struct GreeterServer {
                count: u32,
            }
        };

        let item = parse2(item).expect("error");
        let server = CapServer::new(attr, item).expect("Expansion failed");
        let result = server.generate_init_fn();

        let expected = quote! {
            #[unsafe(no_mangle)]
            pub extern "C" fn __greeter_server__ffi_init(
                config_ptr: *const u8,
                config_len: usize
            ) -> ::pyroduct::capability_host::ffi::FfiInitResult {
                unsafe {
                    ::pyroduct::capability::safe_lifecycle::execute_safe_init::<GreeterConfig, GreeterServer, _>(
                        config_ptr,
                        config_len,
                        |config| <GreeterServer as GreeterServerInit>::new(&config)
                    )
                }
            }
        };

        crate::fmt::assert_code_eq_token(&result, &expected);
    }

    #[tracing_test::traced_test]
    #[test]
    fn test_sync_server_drop_fn() {
        let attr = quote! { service = Greeter, config = GreeterConfig };
        let item = quote! {
            pub struct GreeterServer {
                count: u32,
            }
        };

        let item = parse2(item).expect("error");
        let server = CapServer::new(attr, item).expect("Expansion failed");
        let result = server.generate_drop_fn();

        let expected = quote! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn __greeter_server__ffi_drop(state: *mut std::ffi::c_void) {
                if !state.is_null() {
                    drop(unsafe { Box::from_raw(state as *mut GreeterServer) });
                }
            }
        };

        crate::fmt::assert_code_eq_token(&result, &expected);
    }

    #[tracing_test::traced_test]
    #[test]
    fn test_sync_server_reset_fn() {
        let attr = quote! { service = Greeter, config = GreeterConfig };
        let item = quote! {
            pub struct GreeterServer {
                count: u32,
            }
        };

        let item = parse2(item).expect("error");
        let server = CapServer::new(attr, item).expect("Expansion failed");
        let result = server.generate_reset_fn();

        let expected = quote! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn __greeter_server__ffi_reset(
                state: *mut std::ffi::c_void
            ) -> ::pyroduct::capability_host::ffi::FfiResult {
                ::pyroduct::capability::safe_lifecycle::execute_safe_reset::<GreeterServer, _>(
                    state,
                    |state| <GreeterServer as GreeterServerInit>::reset(state)
                )
            }
        };

        crate::fmt::assert_code_eq_token(&result, &expected);
    }

    #[tracing_test::traced_test]
    #[test]
    fn test_async_server_init_trait() {
        let attr = quote! { service = Greeter, config = GreeterConfig, async_init };
        let item = quote! {
            pub struct GreeterServer {
                count: u32,
            }
        };

        let item = parse2(item).expect("error");
        let server = CapServer::new(attr, item).expect("Expansion failed");
        let result = server.generate_init_trait();

        let expected = quote! {
            pub trait GreeterServerInit {
                fn new(config: &GreeterConfig) -> Self;
                fn reset(&mut self);
            }
        };

        crate::fmt::assert_code_eq_token(&result, &expected);
    }

    #[tracing_test::traced_test]
    #[test]
    fn test_async_server_init_fn() {
        let attr = quote! { service = Greeter, config = GreeterConfig, async_init };
        let item = quote! {
            pub struct GreeterServer {
                count: u32,
            }
        };

        let item = parse2(item).expect("error");
        let server = CapServer::new(attr, item).expect("Expansion failed");
        let result = server.generate_init_fn();

        let expected = quote! {
            #[unsafe(no_mangle)]
            pub extern "C" fn __greeter_server__ffi_init<'a>(
                config_ptr: *const u8,
                config_len: usize
            ) -> ::pyroduct::capability_host::ffi::FfiBorrowedFutureObjectResult<'a> {
                unsafe {
                    ::pyroduct::capability::safe_lifecycle::execute_safe_async_init::<GreeterConfig, GreeterServer, _, _>(
                        config_ptr,
                        config_len,
                        |config| async move { <GreeterServer as GreeterServerInit>::new(&config).await },
                    )
                }
            }
        };

        crate::fmt::assert_code_eq_token(&result, &expected);
    }

    #[tracing_test::traced_test]
    #[test]
    fn test_async_server_drop_fn() {
        let attr = quote! { service = Greeter, config = GreeterConfig, async_init };
        let item = quote! {
            pub struct GreeterServer {
                count: u32,
            }
        };

        let item = parse2(item).expect("error");
        let server = CapServer::new(attr, item).expect("Expansion failed");
        let result = server.generate_drop_fn();

        let expected = quote! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn __greeter_server__ffi_drop(state: *mut std::ffi::c_void) {
                if !state.is_null() {
                    drop(unsafe { Box::from_raw(state as *mut GreeterServer) });
                }
            }
        };

        crate::fmt::assert_code_eq_token(&result, &expected);
    }

    #[tracing_test::traced_test]
    #[test]
    fn test_async_server_reset_fn() {
        let attr = quote! { service = Greeter, config = GreeterConfig, async_init };
        let item = quote! {
            pub struct GreeterServer {
                count: u32,
            }
        };

        let item = parse2(item).expect("error");
        let server = CapServer::new(attr, item).expect("Expansion failed");
        let result = server.generate_reset_fn();

        let expected = quote! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn __greeter_server__ffi_reset<'a>(
                state: *mut std::ffi::c_void
            ) -> ::pyroduct::capability_host::ffi::FfiBorrowedFutureResult<'a> {
                ::pyroduct::capability::safe_lifecycle::execute_safe_async_reset::<GreeterServer, _, _>(
                    state,
                    |state| async move { <GreeterServer as GreeterServerInit>::reset(state).await },
                )
            }
        };

        crate::fmt::assert_code_eq_token(&result, &expected);
    }

    #[tracing_test::traced_test]
    #[test]
    fn test_server_export() {
        let attr = quote! { service = Greeter, config = GreeterConfig, async_init };
        let item = quote! {
            pub struct GreeterServer {
                count: u32,
            }
        };
        let expected = quote! {
            pub static __GREETER_SERVER__EXPORTS: ::pyroduct::capability_host::ffi::ClassExport =
            ::pyroduct::capability_host::ffi::ClassExport {
                ptr: exports.as_ptr(),
                init: ::pyroduct::capability_host::ffi::ClassInitFn::Sync(__greeter_server__ffi_init),
                drop: ::pyroduct::capability_host::ffi::ClassDropFn::Sync(__greeter_server__ffi_drop),
                reset: ::pyroduct::capability_host::ffi::ClassResetFn::Sync(__greeter_server__ffi_reset),
                len: 3usize,
            };
        };
    }
}
