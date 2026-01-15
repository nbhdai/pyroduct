//! #[capability_function] - Marks a standalone function as a capability (Pattern 1)

use heck::AsSnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, Ident, ItemFn, Result, parse2};

use crate::capability_ffi::{CapabilityFuncFFI, InputParams};
use crate::utils::{get_param_name, get_param_type, get_return_type};

#[derive(Debug, Clone)]
pub(crate) struct CapFn {
    pub input: ItemFn,
}

impl CapFn {
    pub fn new(input: ItemFn) -> Result<Self> {
        Ok(CapFn { input })
    }

    pub fn input(&self) -> Option<InputParams> {
        let params: Vec<_> = self.input.sig.inputs.iter().filter_map(|arg| {
            if let FnArg::Typed(_) = arg {
                let name = get_param_name(arg)?;
                let ty = get_param_type(arg)?;
                Some((name, ty.clone()))
            } else {
                None
            }
        }).collect();

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
            library: format_ident!("__{}__{}__func", AsSnakeCase(library.to_string()).to_string(), AsSnakeCase(self.input.sig.ident.to_string()).to_string()),
            fn_name: self.input.sig.ident.clone(),
            fn_ffi_name: format_ident!("__{}_ffi", AsSnakeCase(self.input.sig.ident.to_string()).to_string()),
            fn_wasm_name: format_ident!("__{}_wasm", AsSnakeCase(self.input.sig.ident.to_string()).to_string()),
            vis: self.input.vis.clone(),
            is_async: self.input.sig.asyncness.is_some(),
            return_type: get_return_type(&self.input.sig.output).into(),
            input: self.input(),
            client: None,  // No client for standalone functions
            server: None,  // No server for standalone functions
            has_self: false,      // No self for standalone functions
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
        let wasm_component = ffi.generate_module_function();

        quote! {
            #[cfg(feature = "module")]
            #vis fn #fn_name(#(#fn_params),*) -> #return_type {
                #wasm_component
            }
        }
    }
}

pub(crate) fn expand(_attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    let input = parse2(item)?;
    let cap_fn = CapFn::new(input)?;
    
    let ffi = cap_fn.to_ffi(&format_ident!("env"));
    
    let input = &cap_fn.input;
    let input_struct = ffi.generate_input_struct();
    let input_struct_complex = if input_struct.is_empty() {
        TokenStream::new()
    } else {
        quote! {
            #[cfg(feature = "capability")]
            #input_struct
        }
    };
    let capability_fn = ffi.generate_capability_ffi();
    let client_fn = cap_fn.generate_module_function(&format_ident!("env"));

    let output = quote! {
        // Original function (host-side implementation)
        #[cfg(feature = "capability")]
        #input

        // Input struct for serialization
        #input_struct_complex

        // Host FFI function
        #[cfg(feature = "capability")]
        #capability_fn

        // Client-side wrapper
        #client_fn
    };

    Ok(output)
}


#[cfg(test)]
mod tests {
    use crate::fmt::assert_code_eq;

    use super::*;
    use quote::quote;

    #[tracing_test::traced_test]
    #[test]
    fn test_simple_sync_function() {
        let item = quote! {
            pub fn get_count() -> u32 {
                42
            }
        };

        let result = expand(quote!(), item).expect("Expansion failed");
        
        let expected = r#"
            #[cfg(feature = "capability")]
            pub fn get_count() -> u32 {
                42
            }
            #[cfg(feature = "capability")]
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn __get_count_ffi(
                client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize,
                capability_state_ptr: *mut std::ffi::c_void,
            ) -> ::pyroduct::capability_host::ffi::FfiResult {
                ::pyroduct::capability::safe_call::empty_call::<u32, _>(
                    client_state_ptr,
                    client_state_len,
                    input_ptr,
                    input_len,
                    capability_state_ptr,
                    || get_count(),
                )
            }
            #[cfg(feature = "module")]
            pub fn get_count() -> u32 {
                ::pyroduct::module_capability::access::call_from_wasm::<
                    (),
                    (),
                    u32,
                    _,
                >(
                    "__env__get_count__func",
                    None,
                    None,
                    |client_state_ptr: *const u8,
                     client_state_len: usize,
                     input_ptr: *const u8,
                     input_len: usize| {
                        unsafe {
                            __get_count_wasm(
                                client_state_ptr,
                                client_state_len,
                                input_ptr,
                                input_len,
                            )
                        }
                    },
                )
            }
        "#;

        assert_code_eq(&result, expected);
    }

    #[tracing_test::traced_test]
    #[test]
    fn test_async_single_input() {
        let item = quote! {
            async fn fetch_data(url: String) -> String {
                url
            }
        };

        let result = expand(quote!(), item).expect("Expansion failed");

        let expected = r#"
            #[cfg(feature = "capability")]
            async fn fetch_data(url: String) -> String {
                url
            }
            #[cfg(feature = "capability")]
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn __fetch_data_ffi<'a>(
                client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize,
                capability_state_ptr: *mut std::ffi::c_void,
            ) -> ::pyroduct::capability_host::ffi::FfiBorrowedFutureResult<'a> {
                ::pyroduct::capability::safe_async::i_call::<
                    String,
                    String,
                    _,
                    _,
                >(
                    client_state_ptr,
                    client_state_len,
                    input_ptr,
                    input_len,
                    capability_state_ptr,
                    |input| async move { fetch_data(input).await },
                )
            }
            #[cfg(feature = "module")]
            fn fetch_data(url: String) -> String {
                ::pyroduct::module_capability::access::call_from_wasm::<
                    (),
                    String,
                    String,
                    _,
                >(
                    "__env__fetch_data__func",
                    None,
                    Some(&url),
                    |client_state_ptr: *const u8,
                     client_state_len: usize,
                     input_ptr: *const u8,
                     input_len: usize| {
                        unsafe {
                            __fetch_data_wasm(
                                client_state_ptr,
                                client_state_len,
                                input_ptr,
                                input_len,
                            )
                        }
                    },
                )
            }
        "#;

        assert_code_eq(&result, expected);
    }

    #[tracing_test::traced_test]
    #[test]
    fn test_sync_multiple_inputs() {
        let item = quote! {
            pub fn add(a: u32, b: u32) -> u32 {
                a + b
            }
        };

        let result = expand(quote!(), item).expect("Expansion failed");

        let expected = r#"
            #[cfg(feature = "capability")]
            pub fn add(a: u32, b: u32) -> u32 {
                a + b
            }
            #[cfg(feature = "capability")]
            #[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
            #[rkyv(compare(PartialEq), derive(Debug))]
            struct __add_Input {
                pub a: u32,
                pub b: u32,
            }
            #[cfg(feature = "capability")]
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn __add_ffi(
                client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize,
                capability_state_ptr: *mut std::ffi::c_void,
            ) -> ::pyroduct::capability_host::ffi::FfiResult {
                ::pyroduct::capability::safe_call::i_call::<__add_Input, u32, _>(
                    client_state_ptr,
                    client_state_len,
                    input_ptr,
                    input_len,
                    capability_state_ptr,
                    |input| add(input.a, input.b),
                )
            }
            #[cfg(feature = "module")]
            pub fn add(a: u32, b: u32) -> u32 {
                ::pyroduct::module_capability::access::call_from_wasm::<
                    (),
                    __add_Input,
                    u32,
                    _,
                >(
                    "__env__add__func",
                    None,
                    Some(&__add_Input { a, b }),
                    |client_state_ptr: *const u8,
                     client_state_len: usize,
                     input_ptr: *const u8,
                     input_len: usize| {
                        unsafe {
                            __add_wasm(
                                client_state_ptr,
                                client_state_len,
                                input_ptr,
                                input_len,
                            )
                        }
                    },
                )
            }
        "#;

        assert_code_eq(&result, expected);
    }
}