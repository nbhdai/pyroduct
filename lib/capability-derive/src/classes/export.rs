//! FFI Export Table Generation for CapabilityService
//!
//! This module generates the `ClassExport` structure and associated
//! registration logic for a complete capability service.

use proc_macro2::TokenStream;
use quote::quote;

use crate::classes::definition::CapabilityDefTrait;

impl CapabilityDefTrait {
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
        let class_name_static = &self.ident.class_name_static();

        // Get all capability FFIs (includes constructor + methods)
        let capability_ffis = self.capability_ffis();

        // Generate all capability FFI functions
        let capability_ffi_funcs: Vec<_> = capability_ffis
            .iter()
            .map(|ffi| ffi.generate_capability_ffi())
            .collect();

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
        let exports_array_name = quote::format_ident!("{}__METHODS", class_name_static);
        
        quote! {
            #(#capability_ffi_funcs)*

            #(#plugin_static_str)*
            const #exports_array_name: [::pyroduct::capability_host::ffi::FunctionExport; #num_exports] = [
                #(#plugin_exports),*
            ];
        }
    }

    pub fn generate_wasm_imports(&self) -> Vec<TokenStream> {
        // Get all capability FFIs (includes constructor + methods)
        let capability_ffis = self.capability_ffis();

        // Generate all WASM import declarations
        capability_ffis
            .iter()
            .map(|ffi| ffi.generate_client_wasm())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::classes::definition::CapabilityDefTrait;
    use quote::quote;
    use syn::parse2;

    #[test]
    fn test_generate_ffi_exports() {
        let attr = quote! { TestServer };
        let expected_cap = "cap".into();

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

        // 2. Parse all components
        let trait_def = parse2(trait_code).unwrap();

        let trait_def =
            CapabilityDefTrait::from_trait(attr, trait_def, &expected_cap).unwrap();

        // 3. Generate the FFI exports
        let output = trait_def.generate_ffi_exports();

        let capability_ffi_funcs: Vec<_> = trait_def.capability_ffis()
            .iter()
            .map(|ffi| ffi.generate_capability_ffi())
            .collect();

        // 4. Define expected output
        let expected = quote! {
            #(#capability_ffi_funcs)*

            const __TEST_TRAIT__TEST_SERVER__NEW_CLIENT: &'static str = "__test_trait__test_server__new_client";
            const __TEST_TRAIT__TEST_SERVER__GET_VALUE: &'static str = "__test_trait__test_server__get_value";
            const __TEST_TRAIT__TEST_SERVER__ASYNC_OP: &'static str = "__test_trait__test_server__async_op";
            const __TEST_TRAIT__TEST_SERVER__METHODS: [::pyroduct::capability_host::ffi::FunctionExport; 3usize] = [
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
        };

        crate::fmt::assert_code_eq_token(&output, &expected);
    }
}
