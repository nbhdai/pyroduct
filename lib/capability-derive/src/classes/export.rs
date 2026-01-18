//! FFI Export Table Generation for CapabilityService
//!
//! This module generates the `PluginExports` structure and associated
//! registration logic for a complete capability service.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemImpl, Result};

use crate::classes::{client::CapClient, definition::CapabilityDefTrait, state::CapServer};

/// Represents a fully resolved service consisting of a Server, its Trait definition,
/// the User's Implementation, and the associated Client.
pub struct CapabilityService {
    pub struct_def: CapServer,
    pub trait_def: CapabilityDefTrait,
    pub orig_impl: ItemImpl,
    pub client: CapClient,
}

impl CapabilityService {
    /// Client
    pub fn client(&self) -> Result<TokenStream> {
        let client = self.client.expand();
        let client_methods = self.trait_def.generate_client_impl();
        
        Ok(quote! {
            #client
            #client_methods
        })
    }

    /// State and lifecycle
    pub fn state_and_lifecycle(&self) -> TokenStream {
        let state = &self.struct_def.input;
        let lifecycle = self.struct_def.generate_init_trait();
        
        quote! {
            #state
            #lifecycle
        }
    }
    /// Original Impl
    pub fn orig_impl(&self) -> &ItemImpl {
        &self.orig_impl
    }
    /// Generates the complete FFI export table for this service.
    ///
    /// This creates:
    /// 1. An array of `PluginExport` entries (one per method)
    /// 2. A `PluginExports` struct containing:
    ///    - Pointer to the export array
    ///    - Init function pointer
    ///    - Drop function pointer
    ///    - Reset function pointer
    ///    - Array length and capacity
    pub fn generate_ffi_exports(&self) -> TokenStream {
        let struct_name = &self.struct_def.struct_name;
        let trait_name = &self.trait_def.trait_name;
        
        // Get all capability FFIs (includes constructor + methods)
        let capability_ffis = self.trait_def.capability_ffis();
        
        // Generate init, drop, and reset function pointers
        let init_export = self.struct_def.generate_init_export();
        let drop_export = self.struct_def.generate_drop_export();
        let reset_export = self.struct_def.generate_reset_export();
        
        // Generate all capability FFI functions
        let capability_ffi_funcs: Vec<_> = capability_ffis
            .iter()
            .map(|ffi| ffi.generate_capability_ffi())
            .collect();
        
        // Generate the PluginExport array entries
        let plugin_exports: Vec<_> = capability_ffis
            .iter()
            .map(|ffi| {
                let library = ffi.library.to_string();
                let fn_name = ffi.fn_name.to_string();
                let fn_ffi_name = &ffi.fn_ffi_name;
                
                let func_variant = if ffi.is_async {
                    quote! {
                        ::pyroduct::capability_host::ffi::PluginFunction::Async(#fn_ffi_name)
                    }
                } else {
                    quote! {
                        ::pyroduct::capability_host::ffi::PluginFunction::Sync(#fn_ffi_name)
                    }
                };
                
                quote! {
                    ::pyroduct::capability_host::ffi::PluginExport {
                        module: #library.as_ptr(),
                        module_len: #library.len(),
                        name: #fn_name.as_ptr(),
                        name_len: #fn_name.len(),
                        func: #func_variant,
                    }
                }
            })
            .collect();
        
        let num_exports = plugin_exports.len();
        
        // Generate the static export array name
        let exports_array_name = quote::format_ident!("__{}__{}__EXPORTS", 
            heck::AsSnakeCase(trait_name.to_string()).to_string().to_uppercase(),
            heck::AsSnakeCase(struct_name.to_string()).to_string().to_uppercase()
        );
        
        // Generate the PluginExports struct name
        let plugin_exports_name = quote::format_ident!("__{}__{}__PLUGIN_EXPORTS",
            heck::AsSnakeCase(trait_name.to_string()).to_string().to_uppercase(),
            heck::AsSnakeCase(struct_name.to_string()).to_string().to_uppercase()
        );
        
        quote! {            
            #(#capability_ffi_funcs)*
            
            // Generate the static export array
            static #exports_array_name: [::pyroduct::capability_host::ffi::PluginExport; #num_exports] = [
                #(#plugin_exports),*
            ];
            
            // Generate the PluginExports struct
            pub static #plugin_exports_name: ::pyroduct::capability_host::ffi::PluginExports = ::pyroduct::capability_host::ffi::PluginExports {
                ptr: exports.as_mut_ptr(),
                init: #init_export,
                drop: #drop_export,
                reset: #reset_export,
                len: #num_exports,
                cap: #num_exports,
            };
        }
    }

    pub fn generate_ffi_functions(&self) -> TokenStream {
        let struct_name = &self.struct_def.struct_name;
        let trait_name = &self.trait_def.trait_name;
        
        // Get all capability FFIs (includes constructor + methods)
        let capability_ffis = self.trait_def.capability_ffis();
        
        // Generate init, drop, and reset function pointers
        let init_ffi_func = self.struct_def.generate_init_fn();
        let drop_ffi_func = self.struct_def.generate_drop_fn();
        let reset_ffi_func = self.struct_def.generate_reset_fn();
        
        // Generate all capability FFI functions
        let capability_ffi_funcs: Vec<_> = capability_ffis
            .iter()
            .map(|ffi| ffi.generate_capability_ffi())
            .collect();
        
        // Generate the PluginExport array entries
        let plugin_exports: Vec<_> = capability_ffis
            .iter()
            .map(|ffi| {
                let library = ffi.library.to_string();
                let fn_name = ffi.fn_name.to_string();
                let fn_ffi_name = &ffi.fn_ffi_name;
                
                let func_variant = if ffi.is_async {
                    quote! {
                        ::pyroduct::capability_host::ffi::PluginFunction::Async(#fn_ffi_name)
                    }
                } else {
                    quote! {
                        ::pyroduct::capability_host::ffi::PluginFunction::Sync(#fn_ffi_name)
                    }
                };
                
                quote! {
                    ::pyroduct::capability_host::ffi::PluginExport {
                        module: #library.as_ptr(),
                        module_len: #library.len(),
                        name: #fn_name.as_ptr(),
                        name_len: #fn_name.len(),
                        func: #func_variant,
                    }
                }
            })
            .collect();
        
        let num_exports = plugin_exports.len();
        
        // Generate the static export array name
        let exports_array_name = quote::format_ident!("__{}__{}__EXPORTS", 
            heck::AsSnakeCase(trait_name.to_string()).to_string().to_uppercase(),
            heck::AsSnakeCase(struct_name.to_string()).to_string().to_uppercase()
        );
        
        // Generate the PluginExports struct name
        let plugin_exports_name = quote::format_ident!("__{}__{}__PLUGIN_EXPORTS",
            heck::AsSnakeCase(trait_name.to_string()).to_string().to_uppercase(),
            heck::AsSnakeCase(struct_name.to_string()).to_string().to_uppercase()
        );
        
        quote! {
            // Generate all FFI functions
            #init_ffi_func
            #drop_ffi_func
            #reset_ffi_func
            
            #(#capability_ffi_funcs)*
            
            // Generate the static export array
            static #exports_array_name: [::pyroduct::capability_host::ffi::PluginExport; #num_exports] = [
                #(#plugin_exports),*
            ];
            
            // Generate the PluginExports struct
            pub static #plugin_exports_name: ::pyroduct::capability_host::ffi::PluginExports = ::pyroduct::capability_host::ffi::PluginExports {
                ptr: exports.as_mut_ptr(),
                init: #init_export,
                drop: #drop_export,
                reset: #reset_export,
                len: #num_exports,
                cap: #num_exports,
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
    fn test_generate_ffi_exports() {
        // Create a complete service definition
        let server_attr = quote! { service = MyTrait, config = MyConfig };
        let server_struct = quote! {
            pub struct MyServer {
                count: u32,
            }
        };
        
        let trait_def = quote! {
            trait MyTrait {
                type Client = MyClient;
                
                fn new(id: u32) -> MyClient {
                    MyClient { id }
                }
                
                fn get_info() -> Result<u32, MyError>;
                async fn process(val: u32) -> Result<bool, MyError>;
            }
        };
        
        let impl_def = quote! {
            impl MyTrait for MyServer {
                type Client = MyClient;
                
                fn new_client(&self, client: &MyClient) -> Result<(), MyError> {
                    Ok(())
                }
                
                fn get_info(&self, client: &MyClient) -> Result<u32, MyError> {
                    Ok(42)
                }
            }
        };
        
        let client_struct = quote! {
            pub struct MyClient {
                id: u32,
            }
        };
        
        // Parse all components
        let server = CapServer::new(server_attr, parse2(server_struct).unwrap()).unwrap();
        let (init_func, init_export) = server.generate_init_fn();
        let (reset_func, reset_export) = server.generate_init_fn();
        let (drop_func, drop_export) = server.generate_init_fn();

        let trait_item = parse2(trait_def).unwrap();
        let trait_parsed = CapabilityDefTrait::from_trait(trait_item, quote::format_ident!("MyServer")).unwrap();
        let orig_impl = parse2(impl_def).unwrap();
        let client = CapClient::new(parse2(client_struct).unwrap()).unwrap();
        
        let service = CapabilityService {
            struct_def: server,
            trait_def: trait_parsed,
            orig_impl,
            client,
        };
        
        // Generate FFI exports
        let output = service.generate_ffi_exports();
        
        // Expected output
        let expected = quote! {
            // Generate all FFI functions
            #init_func
            #drop_func
            #reset_func
            
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn __my_trait__my_server__new_client__ffi(
                client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize,
                capability_state_ptr: *mut std::ffi::c_void,
            ) -> ::pyroduct::capability_host::ffi::FfiResult {
                ::pyroduct::capability::safe_call::sc_call::
                    MyServer,
                    Self,
                    Self,
                    _,
                >(
                    client_state_ptr,
                    client_state_len,
                    input_ptr,
                    input_len,
                    capability_state_ptr,
                    |state, client| state.new_client(client),
                )
            }
            
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn __my_trait__my_server__get_info__ffi(
                client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize,
                capability_state_ptr: *mut std::ffi::c_void,
            ) -> ::pyroduct::capability_host::ffi::FfiResult {
                ::pyroduct::capability::safe_call::sc_call::
                    MyServer,
                    MyClient,
                    Result<u32, MyError>,
                    _,
                >(
                    client_state_ptr,
                    client_state_len,
                    input_ptr,
                    input_len,
                    capability_state_ptr,
                    |state, client| state.get_info(client),
                )
            }
            
            // Generate the static export array
            static __MY_TRAIT__MY_SERVER__EXPORTS: [::pyroduct::capability_host::ffi::PluginExport; 3] = [
                ::pyroduct::capability_host::ffi::PluginExport {
                    module: "__my_trait__my_server__new_client".as_ptr(),
                    module_len: "__my_trait__my_server__new_client".len(),
                    name: "new_client".as_ptr(),
                    name_len: "new_client".len(),
                    func: ::pyroduct::capability_host::ffi::PluginFunction::Sync(__my_trait__my_server__new_client__ffi),
                },
                ::pyroduct::capability_host::ffi::PluginExport {
                    module: "__my_trait__my_server__get_info".as_ptr(),
                    module_len: "__my_trait__my_server__get_info".len(),
                    name: "get_info".as_ptr(),
                    name_len: "get_info".len(),
                    func: ::pyroduct::capability_host::ffi::PluginFunction::Sync(__my_trait__my_server__get_info__ffi),
                },
                ::pyroduct::capability_host::ffi::PluginExport {
                    module: "__my_trait__my_server__process".as_ptr(),
                    module_len: "__my_trait__my_server__process".len(),
                    name: "process".as_ptr(),
                    name_len: "process".len(),
                    func: ::pyroduct::capability_host::ffi::PluginFunction::Async(__my_trait__my_server__process__ffi),
                }
            ];
            
            // Generate the PluginExports struct
            pub static __MY_TRAIT__MY_SERVER__PLUGIN_EXPORTS: ::pyroduct::capability_host::ffi::PluginExports = {
                ::pyroduct::capability_host::ffi::PluginExports {
                    ptr: exports.as_mut_ptr(),
                    init: #init_export,
                    drop: #drop_export,
                    reset: #reset_export,
                    len: 3,
                    cap: 3,
                }
            };
        };
        
        crate::fmt::assert_code_eq_token(&output, &expected);
    }
}