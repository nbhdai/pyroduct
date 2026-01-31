use heck::{AsSnakeCase, AsUpperCamelCase};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::rc::Rc;
use syn::{Ident, ReturnType, Type, Visibility};

use crate::paths::CapabilityIdent;
use crate::utils::return_to_type;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputParams {
    One(Ident, Type),
    Many { params: Vec<(Ident, Type)> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityFuncFFI {
    pub class: Option<Rc<CapabilityIdent>>,
    pub constructor: bool,
    pub fn_name: Ident,

    pub vis: Visibility,
    pub is_async: bool,
    pub return_type: ReturnType,
    pub input: Option<InputParams>,
}

impl CapabilityFuncFFI {
    pub fn trace_name(&self) -> Ident {
        if let Some(class) = &self.class {
            class.trace_name(&self.fn_name)
        } else {
            format_ident!("__{}", AsSnakeCase(self.fn_name.to_string()).to_string())
        }
    }

    pub fn trace_name_static(&self) -> Ident {
        if let Some(class) = &self.class {
            class.trace_name_static(&self.fn_name)
        } else {
            format_ident!("__{}", AsSnakeCase(self.fn_name.to_string()).to_string().to_uppercase())
        }
    }

    /// Get the FFI function name
    pub fn fn_ffi_name(&self) -> Ident {
        if let Some(class) = &self.class {
            class.ffi_name(&self.fn_name)
        } else {
            format_ident!(
                "__{}__ffi",
                AsSnakeCase(self.fn_name.to_string()).to_string()
            )
        }
    }

    /// Get the WASM import name
    pub fn fn_wasm_name(&self) -> Ident {
        if let Some(class) = &self.class {
            class.wasm_name(&self.fn_name)
        } else {
            format_ident!(
                "__{}__wasm",
                AsSnakeCase(self.fn_name.to_string()).to_string()
            )
        }
    }

    /// Get the input struct name (if multiple parameters)
    pub fn input_struct_name(&self) -> Option<Ident> {
        match (&self.input, &self.class) {
            (Some(InputParams::Many { .. }), None) => Some(format_ident!(
                "__{}__Input",
                AsUpperCamelCase(self.fn_name.to_string()).to_string()
            )),
            (Some(InputParams::Many { .. }), Some(class)) => {
                Some(class.input_struct(&self.fn_name))
            }
            (_, _) => None,
        }
    }

    /// Check if this FFI has a client parameter
    pub fn has_class(&self) -> bool {
        self.class.is_some()
    }

    /// Generate the input struct if needed
    pub fn generate_input_struct(&self) -> TokenStream {
        if let (Some(InputParams::Many { params }), Some(input_struct_name)) =
            (&self.input, self.input_struct_name())
        {
            let fields: Vec<_> = params.iter().map(|(n, t)| quote! { pub #n: #t }).collect();

            quote! {
                #[derive(::pyroduct::rkyv::Archive, ::pyroduct::rkyv::Deserialize, ::pyroduct::rkyv::Serialize)]
                #[rkyv(crate = ::pyroduct::rkyv)]
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
        let fn_ffi_name = self.fn_ffi_name();

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

    pub fn generate_vtable_entry(&self) -> TokenStream {
        let fn_ffi_name = self.fn_ffi_name();
        let fn_name_static = self.trace_name_static();

        let module_name = if let Some(class) = &self.class {
            class.class_name_static()
        } else {
            format_ident!("__CAPABILITY_NAME")
        };

        let func_variant = if self.is_async {
            quote! {
                ::pyroduct::capability_host::ffi::Function::Async(#fn_ffi_name)
            }
        } else {
            quote! {
                ::pyroduct::capability_host::ffi::Function::Sync(#fn_ffi_name)
            }
        };

        quote! {
            ::pyroduct::capability_host::ffi::FunctionExport {
                module: #module_name.as_ptr(),
                module_len: #module_name.len(),
                name: #fn_name_static.as_ptr(),
                name_len: #fn_name_static.len(),
                func: #func_variant,
            }
        }
    }

    /// Generate the client-side WASM wrapper
    pub fn generate_wasm_call(&self, module: Option<&Ident>) -> TokenStream {
        let trace_name = self.trace_name().to_string();
        let fn_wasm_name = self.fn_wasm_name();
        let return_type = return_to_type(&self.return_type);

        // Determine input type and serialization
        let (input_type, input_expr) = self.determine_input_serialization();

        // Determine client serialization
        let client_expr = if let Some(_) = &self.class {
            if self.constructor {
                quote!(Some(&__config_buf))
            } else {
                quote!(Some(self.buffer()))
            }
        } else {
            quote!(None)
        };
        let module_tn = if let Some(module) = module {
            quote!(#module::)
        } else {
            quote!()
        };

        quote! {
            ::pyroduct::module_capability::access::call_from_wasm::<
                #input_type,
                #return_type,
                _
            >(
                #trace_name,
                #client_expr,
                #input_expr,
                |client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize| {
                    unsafe {
                        #module_tn #fn_wasm_name(client_state_ptr, client_state_len, input_ptr, input_len)
                    }
                },
            )
        }
    }

    pub fn generate_client_wasm(&self) -> TokenStream {
        let fn_wasm_name = self.fn_wasm_name();
        quote! {
            pub fn #fn_wasm_name(
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
        let has_s = self.has_class();
        let has_i = self.input.is_some();

        let suffix = match (has_s, has_i) {
            (true, true) => "sci_call",
            (true, false) => "sc_call",
            (false, true) => "i_call",
            (false, false) => "empty_call",
        };

        format_ident!("{}", suffix)
    }

    /// Determine generic parameters for the helper function call
    fn determine_generics(&self) -> TokenStream {
        let mut generics = Vec::new();

        if let Some(class) = &self.class {
            let state_tn = &class.state_tn;
            let client_tn = &class.client_tn;
            generics.push(quote!(#state_tn));
            generics.push(quote!(#client_tn));
        }

        // Input type (I)
        match (&self.input, self.input_struct_name()) {
            (Some(InputParams::One(_, ty)), _) => generics.push(quote!(#ty)),
            (Some(InputParams::Many { .. }), Some(input_struct_name)) => {
                generics.push(quote!(#input_struct_name))
            }
            (Some(InputParams::Many { .. }), None) => unreachable!(),
            (None, _) => {}
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
        if self.has_class() {
            closure_params.push(quote!(state));
            call_args.push(quote!(state));
            closure_params.push(quote!(client));
            call_args.push(quote!(&client));
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

        let method_call = if self.has_class() {
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
        match (&self.input, self.input_struct_name()) {
            (Some(InputParams::One(param_name, param_ty)), None) => {
                (quote!(#param_ty), quote!(Some(&#param_name)))
            }
            (Some(InputParams::Many { params }), Some(input_struct_name)) => {
                let args = params.iter().map(|(n, _)| quote!(#n));
                (
                    quote!(#input_struct_name),
                    quote!(Some(&#input_struct_name { #(#args),* })),
                )
            }
            _ => (quote!(()), quote!(None)),
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

    fn mock_class() -> Rc<CapabilityIdent> {
        Rc::new(CapabilityIdent {
            config_tn: None,
            state_tn: format_ident!("MockServer"),
            client_tn: format_ident!("MockClient"),
            error_tn: None,
        })
    }

    fn mock_ffi_base(name: &str, is_async: bool, is_class: bool) -> CapabilityFuncFFI {
        if is_class {
            CapabilityFuncFFI {
                class: Some(mock_class()),
                constructor: false,
                fn_name: format_ident!("{}", name),
                vis: Visibility::Public(parse_quote!(pub)),
                is_async,
                return_type: type_to_return(&parse_quote!(u32)), // Default return type
                input: None,
            }
        } else {
            CapabilityFuncFFI {
                class: None,
                constructor: false,
                fn_name: format_ident!("{}", name),
                vis: Visibility::Public(parse_quote!(pub)),
                is_async,
                return_type: type_to_return(&parse_quote!(u32)), // Default return type
                input: None,
            }
        }
    }

    // ========================================================================
    // 1. Sync, No Input, No Client
    // ========================================================================
    #[test]
    fn test_case_1_sync_no_input_no_client() {
        let ffi = mock_ffi_base("test_sync_empty", false, false);

        let struct_tokens = ffi.generate_input_struct();
        let capability_tokens = ffi.generate_capability_ffi();
        let module_tokens = ffi.generate_wasm_call(None);
        let module_tokens = quote! {
            fn func() {
                #module_tokens
            }
        };

        // Struct: should be empty
        assert!(struct_tokens.is_empty());

        let output_capability = quote! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn __test_sync_empty__ffi(
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
                    "__test_sync_empty",
                    None,
                    None,
                    |client_state_ptr: *const u8,
                    client_state_len: usize,
                    input_ptr: *const u8,
                    input_len: usize,| {
                        unsafe {
                            __test_sync_empty__wasm(client_state_ptr, client_state_len, input_ptr, input_len,)
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
        let mut ffi = mock_ffi_base("test_async_single", true, false);
        ffi.input = Some(InputParams::One(
            format_ident!("input_arg"),
            parse_quote!(String),
        ));

        let struct_tokens = ffi.generate_input_struct();
        let capability_tokens = ffi.generate_capability_ffi();
        let module_tokens = ffi.generate_wasm_call(None);
        let module_tokens = quote! {
            fn func() {
                #module_tokens
            }
        };

        // Struct: should be empty for single input
        assert!(struct_tokens.is_empty());

        let output_capability = quote! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn __test_async_single__ffi<'a>(
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
                    "__test_async_single",
                    None,
                    Some(&input_arg),
                    |client_state_ptr: *const u8,
                    client_state_len: usize,
                    input_ptr: *const u8,
                    input_len: usize| {
                        unsafe {
                            __test_async_single__wasm(client_state_ptr, client_state_len, input_ptr, input_len)
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
        let mut ffi = mock_ffi_base("test_sync_multi", false, false);
        ffi.input = Some(InputParams::Many {
            params: vec![
                (format_ident!("a"), parse_quote!(i32)),
                (format_ident!("b"), parse_quote!(i32)),
            ],
        });

        let struct_tokens = ffi.generate_input_struct();
        let capability_tokens = ffi.generate_capability_ffi();
        let module_tokens = ffi.generate_wasm_call(Some(&format_ident!("wasm")));
        let module_tokens = quote! {
            fn func() {
                #module_tokens
            }
        };

        let output_struct = quote! {
            #[derive(::pyroduct::rkyv::Archive, ::pyroduct::rkyv::Deserialize, ::pyroduct::rkyv::Serialize)]
            #[rkyv(crate = ::pyroduct::rkyv)]
            struct __TestSyncMulti__Input {
                pub a: i32,
                pub b: i32,
            }
        };

        crate::fmt::assert_code_eq_token(&struct_tokens, &output_struct);

        let output_capability = quote! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn __test_sync_multi__ffi(
                client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize,
                capability_state_ptr: *mut std::ffi::c_void,
            ) -> ::pyroduct::capability_host::ffi::FfiResult {
                ::pyroduct::capability::safe_call::i_call::<
                    __TestSyncMulti__Input,
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
                    __TestSyncMulti__Input,
                    u32,
                    _,
                >(
                    "__test_sync_multi",
                    None,
                    Some(&__TestSyncMulti__Input { a , b }),
                    |client_state_ptr: *const u8,
                    client_state_len: usize,
                    input_ptr: *const u8,
                    input_len: usize| {
                        unsafe {
                            wasm::__test_sync_multi__wasm(client_state_ptr, client_state_len, input_ptr, input_len)
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
        let ffi = mock_ffi_base("test_async_client", true, true);

        let struct_tokens = ffi.generate_input_struct();
        let capability_tokens = ffi.generate_capability_ffi();
        let module_tokens = ffi.generate_wasm_call(None);
        let module_tokens = quote! {
            fn func() {
                #module_tokens
            }
        };

        // Struct: should be empty
        assert!(struct_tokens.is_empty());

        let output_capability = quote! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn __mock_server__test_async_client__ffi<'a>(
                client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize,
                capability_state_ptr: *mut std::ffi::c_void,
            ) -> ::pyroduct::capability_host::ffi::FfiBorrowedFutureResult<'a> {
                ::pyroduct::capability::safe_async::sc_call::<
                    MockServer,
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
                    |state, client| async move { state.test_async_client(&client).await },
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
                    "__mock_server__test_async_client",
                    Some(&self),
                    None,
                    |client_state_ptr: *const u8,
                    client_state_len: usize,
                    input_ptr: *const u8,
                    input_len: usize| {
                        unsafe {
                            __mock_server__test_async_client__wasm(client_state_ptr, client_state_len, input_ptr, input_len)
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
        let mut ffi = mock_ffi_base("test_sync_client_input", false, true);
        ffi.input = Some(InputParams::One(format_ident!("x"), parse_quote!(i32)));

        let struct_tokens = ffi.generate_input_struct();
        let capability_tokens = ffi.generate_capability_ffi();
        let module_tokens = ffi.generate_wasm_call(None);
        let module_tokens = quote! {
            fn func() {
                #module_tokens
            }
        };

        // Struct: should be empty for single input
        assert!(struct_tokens.is_empty());

        let output_capability = quote! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn __mock_server__test_sync_client_input__ffi(
                client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize,
                capability_state_ptr: *mut std::ffi::c_void,
            ) -> ::pyroduct::capability_host::ffi::FfiResult {
                ::pyroduct::capability::safe_call::sci_call::<
                    MockServer,
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
                    |state, client, input| state.test_sync_client_input(&client, input),
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
                    "__mock_server__test_sync_client_input",
                    Some(&self),
                    Some(&x),
                    |client_state_ptr: *const u8,
                    client_state_len: usize,
                    input_ptr: *const u8,
                    input_len: usize| {
                        unsafe {
                            __mock_server__test_sync_client_input__wasm(client_state_ptr, client_state_len, input_ptr, input_len)
                        }
                    },
                )
            }
        };

        crate::fmt::assert_code_eq_token(&module_tokens, &output_module);
    }

    // ========================================================================
    // 9: Full SCI (Server, Client, Input)
    // ========================================================================
    #[test]
    fn test_case_full_sci() {
        let mut ffi = mock_ffi_base("test_sci_multi", true, true);
        ffi.input = Some(InputParams::Many {
            params: vec![
                (format_ident!("a"), parse_quote!(i32)),
                (format_ident!("b"), parse_quote!(i32)),
            ],
        });

        let struct_tokens = ffi.generate_input_struct();
        let capability_tokens = ffi.generate_capability_ffi();
        let module_tokens = ffi.generate_wasm_call(None);
        let module_tokens = quote! {
            fn func() {
                #module_tokens
            }
        };

        // Struct: should be empty for single input
        let output_struct = quote! {
            #[derive(::pyroduct::rkyv::Archive, ::pyroduct::rkyv::Deserialize, ::pyroduct::rkyv::Serialize)]
            #[rkyv(crate = ::pyroduct::rkyv)]
            struct __MockServer__TestSciMulti__Input {
                pub a: i32,
                pub b: i32,
            }
        };
        crate::fmt::assert_code_eq_token(&struct_tokens, &output_struct);

        let output_capability = quote! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn __mock_server__test_sci_multi__ffi<'a>(
                client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize,
                capability_state_ptr: *mut std::ffi::c_void,
            ) -> ::pyroduct::capability_host::ffi::FfiBorrowedFutureResult<'a> {
                ::pyroduct::capability::safe_async::sci_call::<
                    MockServer,
                    MockClient,
                    __MockServer__TestSciMulti__Input,
                    u32,
                    _,
                    _,
                >(
                    client_state_ptr,
                    client_state_len,
                    input_ptr,
                    input_len,
                    capability_state_ptr,
                    |state, client, input| async move {
                        state.test_sci_multi(&client, input.a, input.b).await
                    },
                )
            }
        };

        // Capability: Should use sci_call
        crate::fmt::assert_code_eq_token(&capability_tokens, &output_capability);

        let output_module = quote! {
            fn func() {
                ::pyroduct::module_capability::access::call_from_wasm::<
                    MockClient,
                    __MockServer__TestSciMulti__Input,
                    u32,
                    _,
                >(
                    "__mock_server__test_sci_multi",
                    Some(&self),
                    Some(&__MockServer__TestSciMulti__Input { a , b }),
                    |client_state_ptr: *const u8,
                    client_state_len: usize,
                    input_ptr: *const u8,
                    input_len: usize| {
                        unsafe {
                            __mock_server__test_sci_multi__wasm(client_state_ptr, client_state_len, input_ptr, input_len)
                        }
                    },
                )
            }
        };

        // Module: should take input parameter
        crate::fmt::assert_code_eq_token(&module_tokens, &output_module);
    }
}
