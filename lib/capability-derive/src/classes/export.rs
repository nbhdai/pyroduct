//! FFI Export Table Generation for CapabilityService
//!
//! This module generates the `ClassExport` structure and associated
//! registration logic for a complete capability service.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, ItemImpl};

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
    pub fn generate_module_client(&self, module: Option<&Ident>) -> TokenStream {
        let client = self.client.expand();
        let client_methods = self.trait_def.generate_client_impl(module);

        quote! {
            #client
            #client_methods
        }
    }

    /// State and lifecycle
    pub fn generate_capability_state(&self) -> TokenStream {
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
    pub fn export_name(&self) -> Ident {
        quote::format_ident!("{}__CLASS_EXPORTS", self.trait_def.ident.class_name_static())
    }
    /// Generates the complete FFI export table for this service.
    ///
    /// This creates:
    /// 1. An array of `FunctionExport` entries (one per method)
    /// 2. A `ClassExport` struct containing:
    ///    - Pointer to the export array
    ///    - Init function pointer
    ///    - Drop function pointer
    ///    - Reset function pointer
    ///    - Array length and capacity
    pub fn generate_ffi_exports(&self) -> TokenStream {
        let class_name_static = &self.trait_def.ident.class_name_static();

        // Get all capability FFIs (includes constructor + methods)
        let capability_ffis = self.trait_def.capability_ffis();

        // Generate init, drop, and reset function pointers
        let init_export = self.struct_def.generate_init_export();
        let drop_export = self.struct_def.generate_drop_export();
        let reset_export = self.struct_def.generate_reset_export();

        // Generate the FunctionExport array entries
        let plugin_exports: Vec<_> = capability_ffis
            .iter()
            .map(|ffi| ffi.generate_vtable_entry())
            .collect();

        let plugin_static_str: Vec<_> = capability_ffis
            .iter()
            .map(|ffi| {
                let fn_ffi_name = ffi.trace_name().to_string();
                let fn_name_static = ffi.trace_name_static();
                quote! {
                    const #fn_name_static: &'static str = #fn_ffi_name;
                }
            })
            .collect();

        let num_exports = plugin_exports.len();

        // Generate the static export array name
        let exports_array_name = quote::format_ident!("{}__EXPORTS", class_name_static);

        // Generate the ClassExport struct name
        let plugin_exports_name = self.export_name();
        
        quote! {
            #(#plugin_static_str)*

            // Generate the static export array
            const #exports_array_name: [::pyroduct::capability_host::ffi::FunctionExport; #num_exports] = [
                #(#plugin_exports),*
            ];

            // Generate the ClassExport struct
            const #plugin_exports_name: ::pyroduct::capability_host::ffi::ClassExport = ::pyroduct::capability_host::ffi::ClassExport {
                ptr: #exports_array_name.as_ptr(),
                init: #init_export,
                drop: #drop_export,
                reset: #reset_export,
                len: #num_exports,
            };
        }
    }

    pub fn generate_ffi_functions(&self) -> TokenStream {
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

        quote! {
            // Generate all FFI functions
            #init_ffi_func
            #drop_ffi_func
            #reset_ffi_func

            #(#capability_ffi_funcs)*
        }
    }

    pub fn generate_wasm_imports(&self) -> Vec<TokenStream> {
        // Get all capability FFIs (includes constructor + methods)
        let capability_ffis = self.trait_def.capability_ffis();

        // Generate all WASM import declarations
        capability_ffis
            .iter()
            .map(|ffi| ffi.generate_client_wasm())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classes::{client::CapClient, definition::CapabilityDefTrait, state::CapServer};
    use quote::quote;
    use syn::parse2;

    #[test]
    fn test_generate_ffi_exports() {
        let expected_cap = "cap".into();
        // 1. Define the complete capability setup
        let client_code = quote! {
            pub struct TestClient {
                pub id: u32,
            }
        };

        let server_code = quote! {
            pub struct TestServer {
                count: u32,
            }
        };

        let trait_code = quote! {
            trait TestTrait {
                type Client = TestClient;

                fn new(id: u32) -> TestClient {
                    TestClient { id }
                }

                fn get_value() -> u32;
                async fn async_op(x: u32) -> u32;
            }
        };

        let impl_code = quote! {
            impl TestTrait for TestServer {
                type Client = TestClient;

                fn new_client(&self, client: &TestClient) {}
                fn get_value(&self, client: &TestClient) -> u32 { 42 }
                async fn async_op(&self, client: &TestClient, x: u32) -> u32 { x }
            }
        };

        // 2. Parse all components
        let client_struct = parse2(client_code).unwrap();
        let server_struct = parse2(server_code).unwrap();
        let trait_def = parse2(trait_code).unwrap();
        let orig_impl = parse2(impl_code).unwrap();

        let client = CapClient::new(client_struct).unwrap();
        let server_attr = quote! { service = TestTrait, config = TestConfig };
        let server = CapServer::new(server_attr, server_struct).unwrap();
        let trait_def =
            CapabilityDefTrait::from_trait(todo!(), trait_def, &expected_cap).unwrap();

        // 3. Create the service
        let service = CapabilityService {
            struct_def: server,
            trait_def,
            orig_impl,
            client,
        };

        // 4. Generate the FFI exports
        let output = service.generate_ffi_exports();

        // 5. Define expected output
        let expected = quote! {
            static __TEST_TRAIT__TEST_SERVER: &'static str = "__test_trait__test_server";
            static __TEST_TRAIT__TEST_SERVER__NEW_CLIENT: &'static str = "__test_trait__test_server__new_client";
            static __TEST_TRAIT__TEST_SERVER__GET_VALUE: &'static str = "__test_trait__test_server__get_value";
            static __TEST_TRAIT__TEST_SERVER__ASYNC_OP: &'static str = "__test_trait__test_server__async_op";

            static __TEST_TRAIT__TEST_SERVER__EXPORTS: [::pyroduct::capability_host::ffi::FunctionExport; 3usize] = [
                ::pyroduct::capability_host::ffi::FunctionExport {
                    module: __TEST_TRAIT__TEST_SERVER.as_ptr(),
                    module_len: __TEST_TRAIT__TEST_SERVER.len(),
                    name: __TEST_TRAIT__TEST_SERVER__NEW_CLIENT.as_ptr(),
                    name_len: __TEST_TRAIT__TEST_SERVER__NEW_CLIENT.len(),
                    func: ::pyroduct::capability_host::ffi::Function::Sync(__test_trait__test_server__new_client__ffi),
                },
                ::pyroduct::capability_host::ffi::FunctionExport {
                    module: __TEST_TRAIT__TEST_SERVER.as_ptr(),
                    module_len: __TEST_TRAIT__TEST_SERVER.len(),
                    name: __TEST_TRAIT__TEST_SERVER__GET_VALUE.as_ptr(),
                    name_len: __TEST_TRAIT__TEST_SERVER__GET_VALUE.len(),
                    func: ::pyroduct::capability_host::ffi::Function::Sync(__test_trait__test_server__get_value__ffi),
                },
                ::pyroduct::capability_host::ffi::FunctionExport {
                    module: __TEST_TRAIT__TEST_SERVER.as_ptr(),
                    module_len: __TEST_TRAIT__TEST_SERVER.len(),
                    name: __TEST_TRAIT__TEST_SERVER__ASYNC_OP.as_ptr(),
                    name_len: __TEST_TRAIT__TEST_SERVER__ASYNC_OP.len(),
                    func: ::pyroduct::capability_host::ffi::Function::Async(__test_trait__test_server__async_op__ffi),
                },
            ];

            pub static __TEST_TRAIT__TEST_SERVER__PLUGIN_EXPORTS: ::pyroduct::capability_host::ffi::ClassExport =
                ::pyroduct::capability_host::ffi::ClassExport {
                    ptr: exports.as_ptr(),
                    init: ::pyroduct::capability_host::ffi::ClassInitFn::Sync(__test_server__ffi_init),
                    drop: ::pyroduct::capability_host::ffi::ClassDropFn::Sync(__test_server__ffi_drop),
                    reset: ::pyroduct::capability_host::ffi::ClassResetFn::Sync(__test_server__ffi_reset),
                    len: 3usize,
                };
        };

        crate::fmt::assert_code_eq_token(&output, &expected);
    }
}
