//! Capability method parsing and validation for impl blocks
//!
//! Valid signatures:
//! - `fn method_name(&self, client: &ClientType, ...args) -> ReturnType`
//! - `fn method_name(&mut self, client: &ClientType, ...args) -> ReturnType`
//! - `async fn method_name(&self, client: &ClientType, ...args) -> ReturnType`
//!
//! If an error type is defined, return types must be `Result<T, ErrorType>` or `Result<T, Self::Error>`

use std::rc::Rc;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, FnArg, Ident, ImplItemFn, Pat, Type};

use super::paths::{CapabilityIdent, FnName, FnOutput};
use crate::{ffi::paths::InputParams, utils::extract_ident_from_type};

/// Represents a validated method within a capability impl block.
#[derive(Debug, Clone)]
pub struct ImplMethod {
    pub name: FnName,
    pub class: Rc<CapabilityIdent>,
    pub client_param: Ident,
    pub inputs: InputParams,
    pub output: FnOutput,
    pub is_async: bool,
    pub is_mutable_self: bool,
    pub body: syn::Block,
    pub attrs: Vec<syn::Attribute>,
}

impl ImplMethod {
    /// Parse and validate a method from an impl block
    pub fn parse(
        f: &ImplItemFn,
        class: &Rc<CapabilityIdent>,
        required_docs: bool,
    ) -> syn::Result<Self> {
        let sig = &f.sig;
        let name = sig.ident.clone();

        let has_docs = f.attrs.iter().any(|attr| attr.path().is_ident("doc"));
        if !has_docs && required_docs {
            return Err(Error::new_spanned(
                &name,
                "Capability methods must have documentation (///) to generate API specs.",
            ));
        }

        // 1. Validate &self or &mut self as first parameter
        let is_mutable_self = match sig.inputs.first() {
            Some(FnArg::Receiver(r)) => {
                if r.reference.is_none() {
                    return Err(Error::new_spanned(
                        r,
                        "Capability methods must take &self or &mut self (not value self)",
                    ));
                }
                r.mutability.is_some()
            }
            Some(arg) => {
                return Err(Error::new_spanned(
                    arg,
                    "Capability methods must take &self or &mut self as first parameter",
                ));
            }
            None => {
                return Err(Error::new_spanned(
                    &sig,
                    "Capability methods must take &self or &mut self",
                ));
            }
        };

        // 2. Validate second parameter is client: &ClientType
        let client_param_arg = sig.inputs.iter().nth(1);
        let client_param_ident = match client_param_arg {
            Some(FnArg::Typed(pt)) => {
                let ident = if let Pat::Ident(pi) = &*pt.pat {
                    pi.ident.clone()
                } else {
                    return Err(Error::new_spanned(&pt.pat, "Expected simple identifier"));
                };

                if let Type::Reference(r) = &*pt.ty {
                    let param_type = extract_ident_from_type(&r.elem)?;
                    if param_type != class.client_tn {
                        return Err(Error::new_spanned(
                            &pt.ty,
                            format!("Expected &{}, found &{}", class.client_tn, param_type),
                        ));
                    }
                } else {
                    return Err(Error::new_spanned(
                        &pt.ty,
                        format!("Expected &{}", class.client_tn),
                    ));
                }
                ident
            }
            Some(arg) => {
                return Err(Error::new_spanned(
                    arg,
                    format!("Expected client: &{}", class.client_tn),
                ));
            }
            None => {
                return Err(Error::new_spanned(
                    &sig,
                    format!(
                        "Capability methods must take client: &{} as second parameter",
                        class.client_tn
                    ),
                ));
            }
        };

        // 3. Collect remaining inputs
        let mut inputs = Vec::new();
        for arg in sig.inputs.iter().skip(2) {
            if let FnArg::Typed(pt) = arg {
                let arg_name = if let Pat::Ident(pi) = &*pt.pat {
                    pi.ident.clone()
                } else {
                    return Err(Error::new_spanned(
                        &pt.pat,
                        "Method arguments must be named identifiers",
                    ));
                };
                inputs.push((arg_name, (*pt.ty).clone()));
            }
        }

        let inputs = if inputs.is_empty() {
            InputParams::None
        } else if inputs.len() == 1 {
            let (n, t) = inputs.pop().unwrap();
            InputParams::One(n, t)
        } else {
            InputParams::Many(inputs)
        };

        // 4. Validate return type
        let output = FnOutput::parse(&sig.output, class.error_tn.as_ref())?;

        Ok(Self {
            name: FnName(name),
            class: class.clone(),
            client_param: client_param_ident,
            inputs,
            output,
            is_async: sig.asyncness.is_some(),
            is_mutable_self,
            body: f.block.clone(),
            attrs: f.attrs.clone(),
        })
    }

    pub fn generate_input_struct(&self) -> TokenStream {
        self.inputs.input_struct(&self.name, Some(&self.class))
    }

    /// Generate the server-side impl method.
    /// This preserves &mut self if the user wrote it, to allow state mutation on the server.
    pub fn generate_server_method(&self) -> TokenStream {
        let name = &self.name.0;
        let attrs = &self.attrs;
        let client_type = &self.class.client_tn;
        let client_var = &self.client_param;
        let body = &self.body;
        let output = &self.output.to_return_type();

        let async_kw = if self.is_async {
            quote!(async)
        } else {
            quote!()
        };

        // Server method respects the user's choice of mutability
        let self_arg = if self.is_mutable_self {
            quote!(&mut self)
        } else {
            quote!(&self)
        };

        let args: Vec<_> = self.inputs.iter().map(|(n, t)| quote!(#n: #t)).collect();

        quote! {
            #(#attrs)*
            pub #async_kw fn #name(#self_arg, #client_var: &#client_type, #(#args),*) #output #body
        }
    }

    pub fn generate_server_ffi(&self) -> TokenStream {
        let fn_ffi_name = self.class.ffi_name(&self.name);
        let input_struct = self.inputs.input_struct(&self.name, Some(&self.class));
        let state_tn = &self.class.state_tn;
        let client_tn = &self.class.client_tn;
        let mut call_args = Vec::new();

        let state_retrieval = quote! {
            let state_ptr = match unsafe { ::pyroduct::ffi::PyroObjectRef::from_raw(capability_state_ptr) } {
                Ok(state) => state,
                Err(error) => return ::pyroduct::PyroError::CodePanic(error.into()).encode().view(),
            };
            let state = state_ptr.as_ref::<#state_tn>();
        };
        // Client stuff
        let client_retrieval = quote! {
            let client: #client_tn = match ::pyroduct::ffi::guest::deserialize_input(client_state_ptr) {
                Ok(buf) => buf,
                Err(err) => return err.encode().view(),
            };
        };
        call_args.push(quote! { &client });
        let input_retrieval = match &self.inputs {
            InputParams::One(_, ty) => {
                call_args.push(quote!(input));
                quote! {
                    let input: #ty = match ::pyroduct::ffi::guest::deserialize_input(input_ptr) {
                        Ok(buf) => buf,
                        Err(err) => return err.encode().view(),
                    };
                }
            }
            InputParams::Many(items) => {
                let input_struct_name = self.class.input_struct(&self.name);
                let args = items.iter().map(|(n, _)| quote!(input.#n));
                call_args.extend(args);
                quote! {
                    let input: #input_struct_name = match ::pyroduct::ffi::guest::deserialize_input(input_ptr) {
                        Ok(buf) => buf,
                        Err(err) => return err.encode().view(),
                    };
                }
            }
            InputParams::None => quote! {},
        };
        let fn_name = &self.name.0;
        let method_call = quote!(state.#fn_name(#(#call_args),*));

        // Return type and body wrapper
        let (ffi_ret, body) = match (self.is_async, &self.class.error_tn) {
            (true, Some(_)) => (
                quote!(::pyroduct::ffi::FuturePyroView),
                quote! {
                    ::pyroduct::ffi::guest::execute_safe_async(|| async move {
                        #state_retrieval
                        #client_retrieval
                        #input_retrieval
                        ::pyroduct::ffi::guest::serialize_result(#method_call.await)
                    }, capability_state_ptr.object_id)
                },
            ),
            (false, Some(_)) => (
                quote!(::pyroduct::format::PyroViewPtr),
                quote! {
                    ::pyroduct::ffi::guest::execute_safe(|| {
                        #state_retrieval
                        #client_retrieval
                        #input_retrieval
                        ::pyroduct::ffi::guest::serialize_result(#method_call)
                    }, capability_state_ptr.object_id)
                },
            ),
            (true, None) => (
                quote!(::pyroduct::ffi::FuturePyroView),
                quote! {
                    ::pyroduct::ffi::guest::execute_safe_async(|| async move {
                        #state_retrieval
                        #client_retrieval
                        #input_retrieval
                        ::pyroduct::ffi::guest::serialize_output(#method_call.await)
                    }, capability_state_ptr.object_id)
                },
            ),
            (false, None) => (
                quote!(::pyroduct::format::PyroViewPtr),
                quote! {
                    ::pyroduct::ffi::guest::execute_safe(|| {
                        #state_retrieval
                        #client_retrieval
                        #input_retrieval
                        ::pyroduct::ffi::guest::serialize_output(#method_call)
                    }, capability_state_ptr.object_id)
                },
            ),
        };

        quote! {
            #input_struct

            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn #fn_ffi_name (
                capability_state_ptr: ::pyroduct::ffi::PyroRefObjectPtr,
                client_state_ptr: ::pyroduct::format::PyroRefPtr,
                input_ptr: ::pyroduct::format::PyroRefPtr,
            ) -> #ffi_ret {
                #body
            }
        }
    }

    pub fn generate_vtable_entry(&self) -> TokenStream {
        let fn_ffi_name = self.class.ffi_name(&self.name);
        let wasm_name_ident = self.class.trace_name_static(&self.name);

        let func_variant = if self.is_async {
            quote! {
                ::pyroduct::ffi::Function::Async(#fn_ffi_name)
            }
        } else {
            quote! {
                ::pyroduct::ffi::Function::Sync(#fn_ffi_name)
            }
        };

        quote! {
            ::pyroduct::ffi::MethodExport {
                name: #wasm_name_ident.as_ptr(),
                name_len: #wasm_name_ident.len(),
                func: #func_variant,
            }
        }
    }

    /// Generate the client-side method.
    /// This ALWAYS uses &self, because the client proxy should be immutable.
    pub fn generate_client_method(&self, module: Option<&Ident>) -> TokenStream {
        let name = &self.name.0;
        let attrs = &self.attrs;

        let wasm_call = self.class.wasm_name(&self.name);
        let wasm_call = match module {
            Some(m) => quote! {#m::#wasm_call},
            None => quote! {#wasm_call},
        };

        let wasm_call = quote! {
            |client_state_ptr: *const u8,
                input_ptr: *const u8| {
                unsafe {
                    #wasm_call(client_state_ptr, input_ptr)
                }
            }
        };

        // Client method forces &self (immutable)
        let mut args: Vec<_> = self.inputs.iter().map(|(n, t)| quote!(#n: #t)).collect();
        args.insert(0, quote!(&self));

        let i_struct = self.inputs.input_struct(&self.name, Some(&self.class));
        let i_name = self.inputs.input_type(&self.name, Some(&self.class));
        let i_fill = self
            .inputs
            .input_serialization(&self.name, Some(&self.class));
        match &self.class.error_tn {
            Some(err_tn) => {
                let output_type = match &self.output {
                    FnOutput::Result(output_type, _) => output_type,
                    _ => unreachable!(),
                };
                let output_return = &self.output.to_return_type();
                quote! {
                    #(#attrs)*
                    fn #name(#(#args),*) #output_return {
                        #i_struct

                        self.__call_result_from_wasm::<#i_name, #output_type, #err_tn, _>(#i_fill, #wasm_call)
                    }
                }
            }
            None => {
                let output_type = &self.output.ty();
                let output_return = &self.output.to_return_type();
                match self.output.err() {
                    Some(err) => {
                        quote! {
                            #(#attrs)*
                            fn #name(#(#args),*) #output_return {
                                #i_struct

                                self.__call_result_from_wasm::<#i_name, #output_type, #err, _>(#i_fill, #wasm_call)
                            }
                        }
                    }
                    None => {
                        quote! {
                            #(#attrs)*
                            fn #name(#(#args),*) #output_return {
                                #i_struct

                                self.__call_from_wasm::<#i_name, #output_type, _>(#i_fill, #wasm_call)
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn generate_client_wasm(&self) -> TokenStream {
        let fn_wasm_name = self.class.wasm_name(&self.name);
        quote! {
            pub fn #fn_wasm_name(
                cs_ptr: *const u8,
                in_ptr: *const u8,
            ) -> *mut u8;
        }
    }

    pub fn doc_attrs(&self) -> Vec<&syn::Attribute> {
        self.attrs
            .iter()
            .filter(|attr| attr.path().is_ident("doc"))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::fmt::assert_code_eq_token;

    use super::*;
    use quote::format_ident;
    use syn::parse_quote;

    fn mock_class(error: Option<&str>) -> Rc<CapabilityIdent> {
        Rc::new(CapabilityIdent {
            pkg_name: "cap_name".to_string(),
            pkg_version: "0.1.0".to_string(),
            config_tn: None,
            state_tn: format_ident!("MyServer"),
            client_tn: format_ident!("MyClient"),
            error_tn: error.map(|s| syn::parse_str(s).unwrap()),
        })
    }

    #[test]
    fn test_server_method_preserves_mutability() {
        let class = mock_class(None);

        // 1. Define Input: Function with &mut self
        let f: ImplItemFn = parse_quote! {
            fn update(&mut self, ctx: &MyClient, val: u32) {
                self.val = val;
            }
        };

        // 2. Parse and Generate Output
        let method = ImplMethod::parse(&f, &class, false).unwrap();
        let output = method.generate_server_method();

        // 3. Define Expected Output
        // The server method MUST preserve &mut self
        let expected = quote! {
            pub fn update(&mut self, ctx: &MyClient, val: u32) {
                self.val = val;
            }
        };

        assert_code_eq_token(&output, &expected);
    }

    #[test]
    fn test_client_method_forces_immutability() {
        let class = mock_class(None);
        let module = format_ident!("wasm_bridge");

        // 1. Define Input: Function with &mut self
        let f: ImplItemFn = parse_quote! {
            fn update(&mut self, ctx: &MyClient, val: u32) { }
        };

        // 2. Parse and Generate Output
        let method = ImplMethod::parse(&f, &class, false).unwrap();
        let output = method.generate_client_method(Some(&module));

        let output_str = output.to_string();
        assert!(output_str.contains("fn update (& self"));
        assert!(!output_str.contains("& mut self"));
    }

    #[test]
    fn test_parse_validates_client_arg_name_capture() {
        let class = mock_class(None);

        // 1. Define Input: Custom client name 'c'
        let f: ImplItemFn = parse_quote! {
            fn get(&self, c: &MyClient) -> u32 { 10 }
        };

        // 2. Parse and Generate
        let method = ImplMethod::parse(&f, &class, false).unwrap();
        let output = method.generate_server_method();

        // 3. Expected: parameter name 'c' is preserved in signature
        let expected = quote! {
            pub fn get(&self, c: &MyClient) -> u32 { 10 }
        };

        assert_code_eq_token(&output, &expected);
    }

    #[test]
    fn test_reject_value_self() {
        let class = mock_class(None);

        // 1. Define Input: self by value
        let f: ImplItemFn = parse_quote! {
            fn consume(self, _c: &MyClient) {}
        };

        // 2. Assert Error
        let err = ImplMethod::parse(&f, &class, false).unwrap_err();
        assert!(err.to_string().contains("not value self"));
    }

    fn mock_method_base(name: &str, is_async: bool, has_err: bool) -> ImplMethod {
        let (error_tn, output) = if has_err {
            (
                Some(parse_quote!(MockError)),
                FnOutput::Result(parse_quote!(u32), parse_quote!(MockError)),
            )
        } else {
            (None, FnOutput::Single(parse_quote!(u32)))
        };
        let class = Rc::new(CapabilityIdent {
            pkg_name: "cap_name".to_string(),
            pkg_version: "0.1.0".to_string(),
            config_tn: None,
            state_tn: format_ident!("MockServer"),
            client_tn: format_ident!("MockClient"),
            error_tn,
        });

        ImplMethod {
            name: FnName(format_ident!("{}", name)),
            class,
            client_param: format_ident!("client"),
            inputs: InputParams::None,
            // Default to u32 as seen in test expectations
            output,
            is_async,
            is_mutable_self: false,
            body: parse_quote!({ 0 }),
            attrs: vec![],
        }
    }

    // ========================================================================
    // 4. Async, No Input, Client Present
    // ========================================================================
    #[test]
    fn test_case_4_async_no_input_with_client() {
        let ffi = mock_method_base("test_async_client", true, false);

        let capability_tokens = ffi.generate_server_ffi();
        let module_tokens = ffi.generate_client_method(None);
        let module_tokens = quote! {
            impl Mod {
                #module_tokens
            }
        };

        let output_capability = quote! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn p__mock_server__test_async_client__ffi(
                capability_state_ptr: ::pyroduct::ffi::PyroRefObjectPtr,
                client_state_ptr: ::pyroduct::format::PyroRefPtr,
                input_ptr: ::pyroduct::format::PyroRefPtr,
            ) -> ::pyroduct::ffi::FuturePyroView {
                ::pyroduct::ffi::guest::execute_safe_async(|| async move {
                    let state_ptr = match unsafe { ::pyroduct::ffi::PyroObjectRef::from_raw(capability_state_ptr) } {
                        Ok(state) => state,
                        Err(error) => return ::pyroduct::PyroError::CodePanic(error.into()).encode().view(),
                    };
                    let state = state_ptr.as_ref::<MockServer>();

                    let client: MockClient = match ::pyroduct::ffi::guest::deserialize_input(client_state_ptr) {
                        Ok(buf) => buf,
                        Err(err) => return err.encode().view(),
                    };
                    ::pyroduct::ffi::guest::serialize_output(state.test_async_client(&client).await)
                }, capability_state_ptr.object_id)
            }
        };

        crate::fmt::assert_code_eq_token(&capability_tokens, &output_capability);

        let output_module = quote! {
            impl Mod {
                fn test_async_client(&self) -> u32 {
                    self.__call_from_wasm::<
                            (),
                            u32,
                            _,
                        >(
                        None,
                        |client_state_ptr: *const u8,
                            input_ptr: *const u8| {
                            unsafe {
                                p__mock_server__test_async_client__wasm(
                                    client_state_ptr,
                                    input_ptr
                                )
                            }
                        }
                    )
                }
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
        let mut ffi = mock_method_base("test_sync_client_input", false, true);
        ffi.inputs = InputParams::One(format_ident!("x"), parse_quote!(i32));
        let capability_tokens = ffi.generate_server_ffi();
        let module_tokens = ffi.generate_client_method(None);
        let module_tokens = quote! {
            impl Mod {
                #module_tokens
            }
        };

        let output_capability = quote! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn p__mock_server__test_sync_client_input__ffi(
                capability_state_ptr: ::pyroduct::ffi::PyroRefObjectPtr,
                client_state_ptr: ::pyroduct::format::PyroRefPtr,
                input_ptr: ::pyroduct::format::PyroRefPtr,
            ) -> ::pyroduct::format::PyroViewPtr {
                ::pyroduct::ffi::guest::execute_safe(|| {
                    let state_ptr = match unsafe { ::pyroduct::ffi::PyroObjectRef::from_raw(capability_state_ptr) } {
                        Ok(state) => state,
                        Err(error) => return ::pyroduct::PyroError::CodePanic(error.into()).encode().view(),
                    };
                    let state = state_ptr.as_ref::<MockServer>();

                    let client: MockClient = match ::pyroduct::ffi::guest::deserialize_input(client_state_ptr) {
                        Ok(buf) => buf,
                        Err(err) => return err.encode().view(),
                    };

                    let input: i32 = match ::pyroduct::ffi::guest::deserialize_input(input_ptr) {
                        Ok(buf) => buf,
                        Err(err) => return err.encode().view(),
                    };
                    ::pyroduct::ffi::guest::serialize_result(state.test_sync_client_input(&client, input))
                }, capability_state_ptr.object_id)
            }
        };

        crate::fmt::assert_code_eq_token(&capability_tokens, &output_capability);

        let output_module = quote! {
            impl Mod {
                fn test_sync_client_input(&self, x: i32) -> Result<u32, MockError> {
                    self.__call_result_from_wasm::<
                            i32,
                            u32,
                            MockError,
                            _,
                        >(
                        Some(&x),
                        |client_state_ptr: *const u8,
                            input_ptr: *const u8| {
                            unsafe {
                                p__mock_server__test_sync_client_input__wasm(
                                    client_state_ptr,
                                    input_ptr,
                                )
                            }
                        },
                    )
                }
            }
        };

        crate::fmt::assert_code_eq_token(&module_tokens, &output_module);
    }

    // ========================================================================
    // 9: Full SCI (Server, Client, Input)
    // ========================================================================
    #[test]
    fn test_case_full_sci() {
        let mut ffi = mock_method_base("test_sci_multi", true, false);
        ffi.inputs = InputParams::Many(vec![
            (format_ident!("a"), parse_quote!(i32)),
            (format_ident!("b"), parse_quote!(i32)),
        ]);

        let capability_tokens = ffi.generate_server_ffi();
        let module_tokens = ffi.generate_client_method(None);
        let module_tokens = quote! {
            impl Mod {
                #module_tokens
            }
        };

        let output_capability = quote! {
            #[::pyroduct::magma]
            struct p__MockServer__TestSciMulti__Input {
                pub a: i32,
                pub b: i32,
            }

            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn p__mock_server__test_sci_multi__ffi(
                capability_state_ptr: ::pyroduct::ffi::PyroRefObjectPtr,
                client_state_ptr: ::pyroduct::format::PyroRefPtr,
                input_ptr: ::pyroduct::format::PyroRefPtr,
            ) -> ::pyroduct::ffi::FuturePyroView {
                ::pyroduct::ffi::guest::execute_safe_async(|| async move {
                    let state_ptr = match unsafe { ::pyroduct::ffi::PyroObjectRef::from_raw(capability_state_ptr) } {
                        Ok(state) => state,
                        Err(error) => return ::pyroduct::PyroError::CodePanic(error.into()).encode().view(),
                    };
                    let state = state_ptr.as_ref::<MockServer>();

                    let client: MockClient = match ::pyroduct::ffi::guest::deserialize_input(client_state_ptr) {
                        Ok(buf) => buf,
                        Err(err) => return err.encode().view(),
                    };

                    let input: p__MockServer__TestSciMulti__Input = match ::pyroduct::ffi::guest::deserialize_input(input_ptr) {
                        Ok(buf) => buf,
                        Err(err) => return err.encode().view(),
                    };
                    ::pyroduct::ffi::guest::serialize_output(state.test_sci_multi(&client, input.a, input.b).await)
                }, capability_state_ptr.object_id)
            }
        };

        // Capability: Should use sci_call
        crate::fmt::assert_code_eq_token(&capability_tokens, &output_capability);

        let output_module = quote! {
            impl Mod {
                fn test_sci_multi(&self, a: i32, b: i32) -> u32 {
                    #[::pyroduct::magma]
                    struct p__MockServer__TestSciMulti__Input {
                        pub a: i32,
                        pub b: i32,
                    }
                    self.__call_from_wasm::<
                            p__MockServer__TestSciMulti__Input,
                            u32,
                            _,
                        >(
                        Some(
                            &p__MockServer__TestSciMulti__Input {
                                a,
                                b,
                            },
                        ),
                        |client_state_ptr: *const u8,
                            input_ptr: *const u8| {
                            unsafe {
                                p__mock_server__test_sci_multi__wasm(
                                    client_state_ptr,
                                    input_ptr,
                                )
                            }
                        },
                    )
                }
            }
        };

        // Module: should take input parameter
        crate::fmt::assert_code_eq_token(&module_tokens, &output_module);
    }
}
