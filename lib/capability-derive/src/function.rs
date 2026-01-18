//! #[capability_function] - Marks a standalone function as a capability (Pattern 1)

use heck::AsSnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, Ident, ItemFn, Result};

use crate::ffi::{CapabilityFuncFFI, InputParams};
use crate::utils::{get_param_name, get_param_type};

#[derive(Debug, Clone)]
pub(crate) struct CapFn {
    pub input: ItemFn,
}

impl CapFn {
    pub fn new(input: ItemFn) -> Result<Self> {
        Ok(CapFn { input })
    }

    pub fn input(&self) -> Option<InputParams> {
        let params: Vec<_> = self
            .input
            .sig
            .inputs
            .iter()
            .filter_map(|arg| {
                if let FnArg::Typed(_) = arg {
                    let name = get_param_name(arg)?;
                    let ty = get_param_type(arg)?;
                    Some((name, ty.clone()))
                } else {
                    None
                }
            })
            .collect();

        match params.len() {
            0 => None,
            1 => {
                let (name, ty) = params.into_iter().next().unwrap();
                Some(InputParams::One(name, ty))
            }
            _ => {
                let input_struct_name = format_ident!("__{}_Input", self.input.sig.ident);
                Some(InputParams::Many {
                    params,
                    input_struct_name,
                })
            }
        }
    }

    /// Creates a CapabilityFuncFFI with no client or state (standalone function)
    pub fn to_ffi(&self, library: &Ident) -> CapabilityFuncFFI {
        CapabilityFuncFFI {
            library: format_ident!(
                "__{}__{}__func",
                AsSnakeCase(library.to_string()).to_string(),
                AsSnakeCase(self.input.sig.ident.to_string()).to_string()
            ),
            fn_name: self.input.sig.ident.clone(),
            fn_ffi_name: format_ident!(
                "__{}_ffi",
                AsSnakeCase(self.input.sig.ident.to_string()).to_string()
            ),
            fn_wasm_name: format_ident!(
                "__{}_wasm",
                AsSnakeCase(self.input.sig.ident.to_string()).to_string()
            ),
            vis: self.input.vis.clone(),
            is_async: self.input.sig.asyncness.is_some(),
            return_type: self.input.sig.output.clone(),
            input: self.input(),
            client: None, // No client for standalone functions
            server: None, // No server for standalone functions
        }
    }

    /// Generate the client-side WASM wrapper
    pub fn generate_module_function(&self, library: &Ident) -> TokenStream {
        let ffi = self.to_ffi(library);

        let fn_name = &ffi.fn_name;
        let vis = &ffi.vis;
        // let fn_ffi_name = &ffi.fn_ffi_name; // Not used in client wrapper directly
        let return_type = &ffi.return_type;

        // Build function parameters
        let mut fn_params = Vec::new();

        if let Some(input) = &ffi.input {
            match input {
                InputParams::One(param_name, param_ty) => {
                    fn_params.push(quote!(#param_name: #param_ty));
                }
                InputParams::Many { params, .. } => {
                    fn_params.extend(params.iter().map(|(n, t)| quote!(#n: #t)));
                }
            }
        }
        let struct_component = ffi.generate_input_struct();
        let wasm_component = ffi.generate_module_function();

        quote! {
            #[cfg(feature = "module")]
            #vis fn #fn_name(#(#fn_params),*) #return_type {
                #struct_component
                #wasm_component
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::fmt::{assert_code_eq, assert_code_eq_token};

    use super::*;
    use quote::quote;
    use syn::parse2;

    #[tracing_test::traced_test]
    #[test]
    fn test_simple_sync_function() {
        let item = quote! {
            pub fn get_count() -> u32 {
                42
            }
        };

        let lib_name = format_ident!("MyLib");
        let item = parse2(item).expect("error");
        let parsed = CapFn::new(item)
            .expect("Expansion failed");
        let ffi = parsed.to_ffi(&lib_name);
        let call_struct = ffi.generate_input_struct();
        let wasm_call = ffi.generate_module_function();
        
        let result = parsed.generate_module_function(&lib_name);

        let expected = quote! {
            #[cfg(feature = "module")]
            pub fn get_count() -> u32 {
                #call_struct
                #wasm_call
            }
        };

        assert_code_eq_token(&result, &expected);
    }

    #[tracing_test::traced_test]
    #[test]
    fn test_async_single_input() {
        let item = quote! {
            async fn fetch_data(url: String) -> String {
                url
            }
        };

        let lib_name = format_ident!("MyLib");
        let item = parse2(item).expect("error");
        let parsed = CapFn::new(item)
            .expect("Expansion failed");
        let ffi = parsed.to_ffi(&lib_name);
        let call_struct = ffi.generate_input_struct();
        let wasm_call = ffi.generate_module_function();
        
        let result = parsed.generate_module_function(&lib_name);

        let expected = quote! {
            #[cfg(feature = "module")]
            fn fetch_data(url: String) -> String {
                #call_struct
                #wasm_call
            }
        };

        assert_code_eq_token(&result, &expected);
    }

    #[tracing_test::traced_test]
    #[test]
    fn test_sync_multiple_inputs() {
        let item = quote! {
            pub fn add(a: u32, b: u32) -> u32 {
                a + b
            }
        };

        let lib_name = format_ident!("MyLib");
        let item = parse2(item).expect("error");
        let parsed = CapFn::new(item)
            .expect("Expansion failed");
        let ffi = parsed.to_ffi(&lib_name);
        let call_struct = ffi.generate_input_struct();
        let wasm_call = ffi.generate_module_function();
        
        let result = parsed.generate_module_function(&lib_name);

        let expected = quote! {
            #[cfg(feature = "module")]
            pub fn add(a: u32, b: u32) -> u32 {
                #call_struct
                #wasm_call
            }
        };

        assert_code_eq_token(&result, &expected);
    }
}
