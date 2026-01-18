use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, ReturnType, Type, Visibility};

use crate::utils::return_to_type;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputParams {
    One(Ident, Type),
    Many {
        params: Vec<(Ident, Type)>,
        input_struct_name: Ident,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityFuncFFI {
    pub library: Ident,
    pub fn_name: Ident,
    /// The name of the ffi capability function this calls
    pub fn_ffi_name: Ident,
    /// The name of the wasm module function this calls
    pub fn_wasm_name: Ident,

    pub vis: Visibility,
    pub is_async: bool,
    pub return_type: ReturnType,
    /// The name of the generated input struct
    pub input: Option<InputParams>,
    /// The client state parameter, if any (Name, Type)
    pub client: Option<Ident>,
    /// The server this is associated with
    pub server: Option<Ident>,
}

impl CapabilityFuncFFI {
    /// Generate the input struct if needed
    pub fn generate_input_struct(&self) -> TokenStream {
        if let Some(InputParams::Many {
            params,
            input_struct_name,
        }) = &self.input
        {
            let fields: Vec<_> = params.iter().map(|(n, t)| quote! { pub #n: #t }).collect();

            quote! {
                #[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
                #[rkyv(compare(PartialEq), derive(Debug))]
                struct #input_struct_name {
                    #(#fields),*
                }
            }
        } else {
            quote! {}
        }
    }

    /// Generate the host-side FFI function
    pub fn generate_capability_ffi(&self) -> TokenStream {
        let fn_ffi_name = &self.fn_ffi_name;

        // Determine the helper function based on what's present (in "sci" order)
        let helper_fn = self.determine_helper_fn();

        // Determine generic parameters based on what's present
        let generics = self.determine_generics();

        // Determine closure parameters and method call
        let (closure_params, method_call) = self.determine_closure_and_call();

        // Return type and body wrapper
        let (func_lifetime, ffi_ret, body) = if self.is_async {
            (
                quote!(<'a>),
                quote!(::pyroduct::capability_host::ffi::FfiBorrowedFutureResult<'a>),
                quote!(async move { #method_call.await }),
            )
        } else {
            (
                quote!(),
                quote!(::pyroduct::capability_host::ffi::FfiResult),
                quote!(#method_call),
            )
        };

        let call_path = if self.is_async {
            quote!(::pyroduct::capability::safe_async::#helper_fn)
        } else {
            quote!(::pyroduct::capability::safe_call::#helper_fn)
        };

        quote! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn #fn_ffi_name #func_lifetime(
                client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize,
                capability_state_ptr: *mut std::ffi::c_void,
            ) -> #ffi_ret {
                #call_path::<#generics>(
                    client_state_ptr,
                    client_state_len,
                    input_ptr,
                    input_len,
                    capability_state_ptr,
                    #closure_params #body
                )
            }
        }
    }

    pub fn export_vtable(&self) -> TokenStream {
        let library_str = format_ident!("{}", self.library.to_string().to_uppercase());
        let fn_name_str = format_ident!("{}", self.fn_name.to_string().to_uppercase());

        let fn_name = &self.fn_name;
        let func_variant = if self.is_async {
            quote! {
                ::pyroduct::capability_host::ffi::PluginFunction::Async(#fn_name)
            }
        } else {
            quote! {
                ::pyroduct::capability_host::ffi::PluginFunction::Sync(#fn_name)
            }
        };

        quote! {
            ::pyroduct::capability_host::ffi::PluginExport {
                module: #library_str.as_ptr(),
                module_len: #library_str.len(),
                name: #fn_name_str.as_ptr(),
                name_len: #fn_name_str.len(),
                func: #func_variant,
            }
        }
    }

    /// Generate the client-side WASM wrapper
    pub fn generate_wasm_call(&self) -> TokenStream {
        // let fn_name = &self.fn_name;
        // let vis = &self.vis;
        // let fn_ffi_name = &self.fn_ffi_name; // Not used in client wrapper directly
        let fn_wasm_name = &self.fn_wasm_name;
        let return_type = return_to_type(&self.return_type);
        let library_name = self.library.to_string();

        // Build function parameters
        let mut fn_params = Vec::new();

        if let Some(client_name) = &self.client {
            fn_params.push(quote!(client: &#client_name));
        }

        if let Some(input) = &self.input {
            match input {
                InputParams::One(param_name, param_ty) => {
                    fn_params.push(quote!(#param_name: #param_ty));
                }
                InputParams::Many { params, .. } => {
                    fn_params.extend(params.iter().map(|(n, t)| quote!(#n: #t)));
                }
            }
        }

        // Determine input type and serialization
        let (input_type, input_expr) = self.determine_input_serialization();

        // Determine client serialization
        let (client_type, client_expr) = if let Some(client_name) = &self.client {
            (quote!(#client_name), quote!(Some(client)))
        } else {
            (quote!(()), quote!(None))
        };

        quote! {
            ::pyroduct::module_capability::access::call_from_wasm::<
                #client_type,
                #input_type,
                #return_type,
                _
            >(
                #library_name,
                #client_expr,
                #input_expr,
                |client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize| {
                    unsafe {
                        #fn_wasm_name(client_state_ptr, client_state_len, input_ptr, input_len)
                    }
                },
            )
        }
    }

    pub fn generate_wasm_function(&self) -> TokenStream {
        let fn_wasm_name = &self.fn_wasm_name;
        quote! {
            fn #fn_wasm_name(
                cs_ptr: *const u8,
                cs_len: usize,
                in_ptr: *const u8,
                in_len: usize,
            ) -> *const u8;
        }
    }

    /// Determine the helper function name based on what parameters are present
    /// Order is always "sci" - server, client, input
    fn determine_helper_fn(&self) -> Ident {
        let has_s = self.server.is_some();
        let has_c = self.client.is_some();
        let has_i = self.input.is_some();

        let suffix = match (has_s, has_c, has_i) {
            (true, true, true) => "sci_call",
            (true, true, false) => "sc_call",
            (true, false, true) => "si_call",
            (true, false, false) => "s_call", // Note: s_call usually implies input is () or handled specially
            (false, true, true) => "ci_call",
            (false, true, false) => "c_call",
            (false, false, true) => "i_call",
            (false, false, false) => "empty_call",
        };

        format_ident!("{}", suffix)
    }

    /// Determine generic parameters for the helper function call
    fn determine_generics(&self) -> TokenStream {
        let mut generics = Vec::new();

        // Server type (S)
        if let Some(server_name) = &self.server {
            generics.push(quote!(#server_name));
        }

        // Client type (C)
        if let Some(client_name) = &self.client {
            generics.push(quote!(#client_name));
        }

        // Input type (I)
        match &self.input {
            Some(InputParams::One(_, ty)) => generics.push(quote!(#ty)),
            Some(InputParams::Many {
                input_struct_name, ..
            }) => generics.push(quote!(#input_struct_name)),
            None => {}
        }

        // Return type (O)
        let return_type = return_to_type(&self.return_type);
        generics.push(quote!(#return_type));

        // Function type for the closure
        generics.push(quote!(_));

        if self.is_async {
            // Future type returned by the async closure
            generics.push(quote!(_));
        }

        quote!(#(#generics),*)
    }

    /// Determine closure parameters and the method call expression
    fn determine_closure_and_call(&self) -> (TokenStream, TokenStream) {
        let fn_name = &self.fn_name;

        let mut closure_params = Vec::new();
        let mut call_args = Vec::new();

        // Server parameter
        if let Some(_) = &self.server {
            closure_params.push(quote!(state));
            call_args.push(quote!(state));
        }

        // Client parameter
        if let Some(_) = &self.client {
            closure_params.push(quote!(client));
            call_args.push(quote!(client));
        }

        // Input parameter
        if let Some(input) = &self.input {
            closure_params.push(quote!(input));

            match input {
                InputParams::One(..) => {
                    call_args.push(quote!(input));
                }
                InputParams::Many { params, .. } => {
                    // Destructure the input struct
                    let args = params.iter().map(|(n, _)| quote!(input.#n));
                    call_args.extend(args);
                }
            }
        }

        let closure_params_tokens = if closure_params.is_empty() {
            quote!(||)
        } else {
            quote!(|#(#closure_params),*|)
        };

        let method_call = if self.server.is_some() {
            // Method call on self
            let self_arg = closure_params.first().unwrap();
            let method_args = &call_args[1..]; // Skip self
            quote!(#self_arg.#fn_name(#(#method_args),*))
        } else {
            // Standalone function call
            quote!(#fn_name(#(#call_args),*))
        };

        (closure_params_tokens, method_call)
    }

    /// Determine input type and serialization expression for client WASM
    pub fn determine_input_serialization(&self) -> (TokenStream, TokenStream) {
        match &self.input {
            Some(InputParams::One(param_name, param_ty)) => {
                (quote!(#param_ty), quote!(Some(&#param_name)))
            }
            Some(InputParams::Many {
                input_struct_name,
                params,
            }) => {
                let args = params.iter().map(|(n, _)| quote!(#n));
                (
                    quote!(#input_struct_name),
                    quote!(Some(&#input_struct_name { #(#args),* })),
                )
            }
            None => (quote!(()), quote!(None)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::type_to_return;
    use quote::quote;
    use syn::{Visibility, parse_quote};

    // --- Helpers to construct mock objects ---

    fn mock_client() -> Ident {
        // Construct a dummy client struct
        format_ident!("MockClient")
    }

    fn mock_server() -> Ident {
        format_ident!("MockServer")
    }

    fn mock_ffi_base(name: &str, is_async: bool) -> CapabilityFuncFFI {
        CapabilityFuncFFI {
            library: format_ident!("{}_lib", name),
            fn_name: format_ident!("{}", name),
            fn_ffi_name: format_ident!("__{}_ffi", name),
            fn_wasm_name: format_ident!("__{}_wasm", name),
            vis: Visibility::Public(parse_quote!(pub)),
            is_async,
            return_type: type_to_return(&parse_quote!(u32)), // Default return type
            input: None,
            client: None,
            server: None,
        }
    }

    // ========================================================================
    // 1. Sync, No Input, No Client
    // ========================================================================
    #[test]
    fn test_case_1_sync_no_input_no_client() {
        let ffi = mock_ffi_base("test_sync_empty", false);

        let struct_tokens = ffi.generate_input_struct();
        let capability_tokens = ffi.generate_capability_ffi();
        let module_tokens = ffi.generate_wasm_call();
        let module_tokens = quote! {
            fn func() {
                #module_tokens
            }
        };

        // Struct: should be empty
        assert!(struct_tokens.is_empty());

        let output_capability = quote! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn __test_sync_empty_ffi(
                client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize,
                capability_state_ptr: *mut std::ffi::c_void,
            ) -> ::pyroduct::capability_host::ffi::FfiResult {
                ::pyroduct::capability::safe_call::empty_call::<
                    u32,
                    _,
                >(
                    client_state_ptr,
                    client_state_len,
                    input_ptr,
                    input_len,
                    capability_state_ptr,
                    || test_sync_empty(),
                )
            }
        };

        crate::fmt::assert_code_eq_token(&capability_tokens, &output_capability);

        let output_module = quote! {
            fn func() {
                ::pyroduct::module_capability::access::call_from_wasm::<
                    (),
                    (),
                    u32,
                    _,
                >(
                    "test_sync_empty_lib",
                    None,
                    None,
                    |client_state_ptr: *const u8,
                    client_state_len: usize,
                    input_ptr: *const u8,
                    input_len: usize,| {
                        unsafe {
                            __test_sync_empty_wasm(client_state_ptr, client_state_len, input_ptr, input_len,)
                        }
                    },
                )
            }
        };

        crate::fmt::assert_code_eq_token(&module_tokens, &output_module);
    }

    // ========================================================================
    // 2. Async, Single Input, No Client
    // ========================================================================
    #[test]
    fn test_case_2_async_single_input_no_client() {
        let mut ffi = mock_ffi_base("test_async_single", true);
        ffi.input = Some(InputParams::One(
            format_ident!("input_arg"),
            parse_quote!(String),
        ));

        let struct_tokens = ffi.generate_input_struct();
        let capability_tokens = ffi.generate_capability_ffi();
        let module_tokens = ffi.generate_wasm_call();
        let module_tokens = quote! {
            fn func() {
                #module_tokens
            }
        };

        // Struct: should be empty for single input
        assert!(struct_tokens.is_empty());

        let output_capability = quote! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn __test_async_single_ffi<'a>(
                client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize,
                capability_state_ptr: *mut std::ffi::c_void,
            ) -> ::pyroduct::capability_host::ffi::FfiBorrowedFutureResult<'a> {
                ::pyroduct::capability::safe_async::i_call::<
                    String,
                    u32,
                    _,
                    _,
                >(
                    client_state_ptr,
                    client_state_len,
                    input_ptr,
                    input_len,
                    capability_state_ptr,
                    |input| async move { test_async_single(input).await },
                )
            }
        };

        crate::fmt::assert_code_eq_token(&capability_tokens, &output_capability);

        let output_module = quote! {
            fn func() {
                ::pyroduct::module_capability::access::call_from_wasm::<
                    (),
                    String,
                    u32,
                    _,
                >(
                    "test_async_single_lib",
                    None,
                    Some(&input_arg),
                    |client_state_ptr: *const u8,
                    client_state_len: usize,
                    input_ptr: *const u8,
                    input_len: usize| {
                        unsafe {
                            __test_async_single_wasm(client_state_ptr, client_state_len, input_ptr, input_len)
                        }
                    },
                )
            }
        };

        // Module: Client Side
        crate::fmt::assert_code_eq_token(&module_tokens, &output_module);
    }

    // ========================================================================
    // 3. Sync, Multi Input, No Client
    // ========================================================================
    #[test]
    fn test_case_3_sync_multi_input_no_client() {
        let mut ffi = mock_ffi_base("test_sync_multi", false);
        ffi.input = Some(InputParams::Many {
            params: vec![
                (format_ident!("a"), parse_quote!(i32)),
                (format_ident!("b"), parse_quote!(i32)),
            ],
            input_struct_name: format_ident!("__MultiInput"),
        });

        let struct_tokens = ffi.generate_input_struct();
        let capability_tokens = ffi.generate_capability_ffi();
        let module_tokens = ffi.generate_wasm_call();
        let module_tokens = quote! {
            fn func() {
                #module_tokens
            }
        };

        let output_struct = quote! {
            #[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
            #[rkyv(compare(PartialEq), derive(Debug))]
            struct __MultiInput {
                pub a: i32,
                pub b: i32,
            }
        };

        crate::fmt::assert_code_eq_token(&struct_tokens, &output_struct);

        let output_capability = quote! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn __test_sync_multi_ffi(
                client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize,
                capability_state_ptr: *mut std::ffi::c_void,
            ) -> ::pyroduct::capability_host::ffi::FfiResult {
                ::pyroduct::capability::safe_call::i_call::<
                    __MultiInput,
                    u32,
                    _,
                >(
                    client_state_ptr,
                    client_state_len,
                    input_ptr,
                    input_len,
                    capability_state_ptr,
                    |input| test_sync_multi(input.a, input.b),
                )
            }
        };

        crate::fmt::assert_code_eq_token(&capability_tokens, &output_capability);

        let output_module = quote! {
            fn func() {
                ::pyroduct::module_capability::access::call_from_wasm::<
                    (),
                    __MultiInput,
                    u32,
                    _,
                >(
                    "test_sync_multi_lib",
                    None,
                    Some(&__MultiInput { a , b }),
                    |client_state_ptr: *const u8,
                    client_state_len: usize,
                    input_ptr: *const u8,
                    input_len: usize| {
                        unsafe {
                            __test_sync_multi_wasm(client_state_ptr, client_state_len, input_ptr, input_len)
                        }
                    },
                )
            }
        };

        // Module: Check client side
        crate::fmt::assert_code_eq_token(&module_tokens, &output_module);
    }

    // ========================================================================
    // 4. Async, No Input, Client Present
    // ========================================================================
    #[test]
    fn test_case_4_async_no_input_with_client() {
        let mut ffi = mock_ffi_base("test_async_client", true);
        ffi.client = Some(mock_client());

        let struct_tokens = ffi.generate_input_struct();
        let capability_tokens = ffi.generate_capability_ffi();
        let module_tokens = ffi.generate_wasm_call();
        let module_tokens = quote! {
            fn func() {
                #module_tokens
            }
        };

        // Struct: should be empty
        assert!(struct_tokens.is_empty());

        let output_capability = quote! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn __test_async_client_ffi<'a>(
                client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize,
                capability_state_ptr: *mut std::ffi::c_void,
            ) -> ::pyroduct::capability_host::ffi::FfiBorrowedFutureResult<'a> {
                ::pyroduct::capability::safe_async::c_call::<
                    MockClient,
                    u32,
                    _,
                    _,
                >(
                    client_state_ptr,
                    client_state_len,
                    input_ptr,
                    input_len,
                    capability_state_ptr,
                    |client| async move { test_async_client(client).await },
                )
            }
        };

        crate::fmt::assert_code_eq_token(&capability_tokens, &output_capability);

        let output_module = quote! {
            fn func() {
                ::pyroduct::module_capability::access::call_from_wasm::<
                    MockClient,
                    (),
                    u32,
                    _,
                >(
                    "test_async_client_lib",
                    Some(client),
                    None,
                    |client_state_ptr: *const u8,
                    client_state_len: usize,
                    input_ptr: *const u8,
                    input_len: usize| {
                        unsafe {
                            __test_async_client_wasm(client_state_ptr, client_state_len, input_ptr, input_len)
                        }
                    },
                )
            }
        };

        // Module: should not take input parameter
        crate::fmt::assert_code_eq_token(&module_tokens, &output_module);
    }

    // ========================================================================
    // 5. Sync, Single Input, Client Present
    // ========================================================================
    #[test]
    fn test_case_5_sync_single_input_with_client() {
        let mut ffi = mock_ffi_base("test_sync_client_input", false);
        ffi.client = Some(mock_client());
        ffi.input = Some(InputParams::One(format_ident!("x"), parse_quote!(i32)));

        let struct_tokens = ffi.generate_input_struct();
        let capability_tokens = ffi.generate_capability_ffi();
        let module_tokens = ffi.generate_wasm_call();
        let module_tokens = quote! {
            fn func() {
                #module_tokens
            }
        };

        // Struct: should be empty for single input
        assert!(struct_tokens.is_empty());

        let output_capability = quote! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn __test_sync_client_input_ffi(
                client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize,
                capability_state_ptr: *mut std::ffi::c_void,
            ) -> ::pyroduct::capability_host::ffi::FfiResult {
                ::pyroduct::capability::safe_call::ci_call::<
                    MockClient,
                    i32,
                    u32,
                    _,
                >(
                    client_state_ptr,
                    client_state_len,
                    input_ptr,
                    input_len,
                    capability_state_ptr,
                    |client, input| test_sync_client_input(client, input),
                )
            }
        };

        crate::fmt::assert_code_eq_token(&capability_tokens, &output_capability);

        let output_module = quote! {
            fn func() {
                ::pyroduct::module_capability::access::call_from_wasm::<
                    MockClient,
                    i32,
                    u32,
                    _,
                >(
                    "test_sync_client_input_lib",
                    Some(client),
                    Some(&x),
                    |client_state_ptr: *const u8,
                    client_state_len: usize,
                    input_ptr: *const u8,
                    input_len: usize| {
                        unsafe {
                            __test_sync_client_input_wasm(client_state_ptr, client_state_len, input_ptr, input_len)
                        }
                    },
                )
            }
        };

        crate::fmt::assert_code_eq_token(&module_tokens, &output_module);
    }

    // ========================================================================
    // 6. Async, Multi Input, Client Present
    // ========================================================================
    #[test]
    fn test_case_6_async_multi_input_with_client() {
        let mut ffi = mock_ffi_base("test_async_full", true);
        ffi.client = Some(mock_client());
        ffi.input = Some(InputParams::Many {
            params: vec![
                (format_ident!("port_name"), parse_quote!(String)),
                (format_ident!("baud_rate"), parse_quote!(u32)),
            ],
            input_struct_name: format_ident!("__FullInput"),
        });

        let struct_tokens = ffi.generate_input_struct();
        let capability_tokens = ffi.generate_capability_ffi();
        let module_tokens = ffi.generate_wasm_call();
        let module_tokens = quote! {
            fn func() {
                #module_tokens
            }
        };

        let output_struct = quote! {
            #[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
            #[rkyv(compare(PartialEq), derive(Debug))]
            struct __FullInput {
                pub port_name: String,
                pub baud_rate: u32,
            }
        };
        crate::fmt::assert_code_eq_token(&struct_tokens, &output_struct);

        let output_capability = quote! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn __test_async_full_ffi<'a>(
                client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize,
                capability_state_ptr: *mut std::ffi::c_void,
            ) -> ::pyroduct::capability_host::ffi::FfiBorrowedFutureResult<'a> {
                ::pyroduct::capability::safe_async::ci_call::<
                    MockClient,
                    __FullInput,
                    u32,
                    _,
                    _,
                >(
                    client_state_ptr,
                    client_state_len,
                    input_ptr,
                    input_len,
                    capability_state_ptr,
                    |client, input| async move { test_async_full(client, input.port_name, input.baud_rate).await },
                )
            }
        };

        crate::fmt::assert_code_eq_token(&capability_tokens, &output_capability);

        let output_module = quote! {
            fn func() {
                ::pyroduct::module_capability::access::call_from_wasm::<
                    MockClient,
                    __FullInput,
                    u32,
                    _,
                >(
                        "test_async_full_lib",
                        Some(client),
                        Some(&__FullInput { port_name , baud_rate }),
                        |client_state_ptr: *const u8,
                        client_state_len: usize,
                        input_ptr: *const u8,
                        input_len: usize| {
                            unsafe {
                                __test_async_full_wasm(client_state_ptr, client_state_len, input_ptr, input_len)
                            }
                        },
                    )
            }
        };

        // Module: should take multi input parameters
        crate::fmt::assert_code_eq_token(&module_tokens, &output_module)
    }

    // ========================================================================
    // 7. Sync, Input, Server Present (No Client)
    // ========================================================================
    #[test]
    fn test_case_7_sync_server_input() {
        let mut ffi = mock_ffi_base("test_server_sync", false);
        ffi.server = Some(mock_server());
        ffi.input = Some(InputParams::One(format_ident!("val"), parse_quote!(f64))); // Server methods usually implies self

        let struct_tokens = ffi.generate_input_struct();
        let capability_tokens = ffi.generate_capability_ffi();
        let module_tokens = ffi.generate_wasm_call();
        let module_tokens = quote! {
            fn func() {
                #module_tokens
            }
        };

        // Struct: should be empty for single input
        assert!(struct_tokens.is_empty());

        let output_capability = quote! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn __test_server_sync_ffi(
                client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize,
                capability_state_ptr: *mut std::ffi::c_void,
            ) -> ::pyroduct::capability_host::ffi::FfiResult {
                ::pyroduct::capability::safe_call::si_call::<
                    MockServer,
                    f64,
                    u32,
                    _,
                >(
                    client_state_ptr,
                    client_state_len,
                    input_ptr,
                    input_len,
                    capability_state_ptr,
                    |state, input| state.test_server_sync(input),
                )
            }
        };

        crate::fmt::assert_code_eq_token(&capability_tokens, &output_capability);

        let output_module = quote! {
            fn func() {
                ::pyroduct::module_capability::access::call_from_wasm::<
                    (),
                    f64,
                    u32,
                    _,
                >(
                    "test_server_sync_lib",
                    None,
                    Some(&val),
                    |client_state_ptr: *const u8,
                    client_state_len: usize,
                    input_ptr: *const u8,
                    input_len: usize| {
                        unsafe {
                            __test_server_sync_wasm(client_state_ptr, client_state_len, input_ptr, input_len)
                        }
                    },
                )
            }
        };

        // Module: should take input parameter
        crate::fmt::assert_code_eq_token(&module_tokens, &output_module);
    }

    // ========================================================================
    // 8. Async, Input, Server Present (No Client)
    // ========================================================================
    #[test]
    fn test_case_8_async_server_input() {
        let mut ffi = mock_ffi_base("test_server_async", true);
        ffi.server = Some(mock_server());
        ffi.input = Some(InputParams::One(
            format_ident!("query"),
            parse_quote!(String),
        ));

        let struct_tokens = ffi.generate_input_struct();
        let capability_tokens = ffi.generate_capability_ffi();
        let module_tokens = ffi.generate_wasm_call();
        let module_tokens = quote! {
            fn func() {
                #module_tokens
            }
        };

        // Struct: should be empty for single input
        assert!(struct_tokens.is_empty());

        let output_capability = quote! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn __test_server_async_ffi<'a>(
                client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize,
                capability_state_ptr: *mut std::ffi::c_void,
            ) -> ::pyroduct::capability_host::ffi::FfiBorrowedFutureResult<'a> {
                ::pyroduct::capability::safe_async::si_call::<
                    MockServer,
                    String,
                    u32,
                    _,
                    _,
                >(
                    client_state_ptr,
                    client_state_len,
                    input_ptr,
                    input_len,
                    capability_state_ptr,
                    |state, input| async move { state.test_server_async(input).await },
                )
            }
        };

        crate::fmt::assert_code_eq_token(&capability_tokens, &output_capability);

        let output_module = quote! {
            fn func() {
                ::pyroduct::module_capability::access::call_from_wasm::<
                    (),
                    String,
                    u32,
                    _,
                >(
                    "test_server_async_lib",
                    None,
                    Some(&query),
                    |client_state_ptr: *const u8,
                    client_state_len: usize,
                    input_ptr: *const u8,
                    input_len: usize| {
                        unsafe {
                            __test_server_async_wasm(client_state_ptr, client_state_len, input_ptr, input_len)
                        }
                    },
                )
            }
        };

        // Module: should be async and take input
        crate::fmt::assert_code_eq_token(&module_tokens, &output_module);
    }

    // ========================================================================
    // 9: Full SCI (Server, Client, Input)
    // ========================================================================
    #[test]
    fn test_case_full_sci() {
        let mut ffi = mock_ffi_base("test_sci", false);
        ffi.server = Some(mock_server());
        ffi.client = Some(mock_client());
        ffi.input = Some(InputParams::One(format_ident!("x"), parse_quote!(u8)));

        let struct_tokens = ffi.generate_input_struct();
        let capability_tokens = ffi.generate_capability_ffi();
        let module_tokens = ffi.generate_wasm_call();
        let module_tokens = quote! {
            fn func() {
                #module_tokens
            }
        };

        // Struct: should be empty for single input
        assert!(struct_tokens.is_empty());

        let output_capability = quote! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn __test_sci_ffi(
                client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize,
                capability_state_ptr: *mut std::ffi::c_void,
            ) -> ::pyroduct::capability_host::ffi::FfiResult {
                ::pyroduct::capability::safe_call::sci_call::<
                    MockServer,
                    MockClient,
                    u8,
                    u32,
                    _,
                >(
                    client_state_ptr,
                    client_state_len,
                    input_ptr,
                    input_len,
                    capability_state_ptr,
                    |state, client, input| state.test_sci(client, input),
                )
            }
        };

        // Capability: Should use sci_call
        crate::fmt::assert_code_eq_token(&capability_tokens, &output_capability);

        let output_module = quote! {
            fn func() {
                ::pyroduct::module_capability::access::call_from_wasm::<
                    MockClient,
                    u8,
                    u32,
                    _,
                >(
                    "test_sci_lib",
                    Some(client),
                    Some(&x),
                    |client_state_ptr: *const u8,
                    client_state_len: usize,
                    input_ptr: *const u8,
                    input_len: usize| {
                        unsafe {
                            __test_sci_wasm(client_state_ptr, client_state_len, input_ptr, input_len)
                        }
                    },
                )
            }
        };

        // Module: should take input parameter
        crate::fmt::assert_code_eq_token(&module_tokens, &output_module);
    }

    // ========================================================================
    // 10: Full SCI (Server, Client, Input)
    // ========================================================================
    #[test]
    fn test_case_sc() {
        let mut ffi = mock_ffi_base("test_sc", false);
        ffi.server = Some(mock_server());
        ffi.client = Some(mock_client());

        let struct_tokens = ffi.generate_input_struct();
        let capability_tokens = ffi.generate_capability_ffi();
        let module_tokens = ffi.generate_wasm_call();
        let module_tokens = quote! {
            fn func() {
                #module_tokens
            }
        };

        // Struct: should be empty for single input
        assert!(struct_tokens.is_empty());

        let output_capability = quote! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn __test_sc_ffi(
                client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize,
                capability_state_ptr: *mut std::ffi::c_void,
            ) -> ::pyroduct::capability_host::ffi::FfiResult {
                ::pyroduct::capability::safe_call::sc_call::<
                    MockServer,
                    MockClient,
                    u32,
                    _,
                >(
                    client_state_ptr,
                    client_state_len,
                    input_ptr,
                    input_len,
                    capability_state_ptr,
                    |state, client| state.test_sc(client),
                )
            }
        };

        // Capability: Should use sci_call
        crate::fmt::assert_code_eq_token(&capability_tokens, &output_capability);

        let output_module = quote! {
            fn func() {
                ::pyroduct::module_capability::access::call_from_wasm::<
                    MockClient,
                    (),
                    u32,
                    _,
                >(
                    "test_sc_lib",
                    Some(client),
                    None,
                    |client_state_ptr: *const u8,
                    client_state_len: usize,
                    input_ptr: *const u8,
                    input_len: usize| {
                        unsafe {
                            __test_sc_wasm(client_state_ptr, client_state_len, input_ptr, input_len)
                        }
                    },
                )
            }
        };

        // Module: should take input parameter
        crate::fmt::assert_code_eq_token(&module_tokens, &output_module);
    }
}
