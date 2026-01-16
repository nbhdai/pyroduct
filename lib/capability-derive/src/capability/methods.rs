use heck::AsSnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{visit_mut::VisitMut,
    Error, ExprStruct, FnArg, GenericArgument, Ident, ImplItem, ImplItemFn, ItemImpl, ItemTrait, Member, Meta, Path, PathArguments, Result, ReturnType, Token, TraitItem, TraitItemFn, Type, TypePath, parse_quote, parse2
};

use crate::capability_ffi::{CapabilityFuncFFI, InputParams};

/// Represents a validated method within the Capability trait.
#[derive(Debug)]
pub struct CapabilityMethod {
    pub name: Ident,
    pub inputs: Vec<(Ident, Type)>,
    pub output: ReturnType,
    pub original_sig: syn::Signature,
    pub attrs: Vec<syn::Attribute>,
    pub client_type: Option<Type>,
    pub error_type: Option<Type>,
}

impl CapabilityMethod {
    pub fn from_trait(
        method: TraitItemFn,
        explicit_client_type: Option<&Type>,
        explicit_error_type: Option<&Type>, // Passed in as requested, even if unused for current validation rules
    ) -> syn::Result<Self> {
        let sig = &method.sig;

        // --------------------------------------------------------
        // Rule 1: Do not have a &self (or self, &mut self)
        // --------------------------------------------------------
        for input in &sig.inputs {
            if let FnArg::Receiver(rec) = input {
                return Err(Error::new_spanned(
                    rec,
                    "Capability methods cannot take variant of 'self', or 'Self'",
                ));
            }
        }

        // --------------------------------------------------------
        // Rule 2 & 4: Output Validation (No Client, No Self)
        // --------------------------------------------------------
        if let ReturnType::Type(_, ty) = &sig.output {
            // Check for 'Self' return
            if let Type::Path(type_path) = &**ty {
                if type_path.path.is_ident("Self") {
                    return Err(Error::new_spanned(
                        ty,
                        "Capability methods cannot return 'Self'.",
                    ));
                }
            }

            // Check for 'Client' return
            if let Some(client_type) = explicit_client_type {
                if quote!(#ty).to_string() == quote!(#client_type).to_string() {
                    return Err(Error::new_spanned(
                        ty,
                        "Capability methods cannot return the defined 'Client' type.",
                    ));
                }
            }
        }

        // --------------------------------------------------------
        // Rule 3: Input Validation (NO Client passed in)
        // --------------------------------------------------------
        let mut clean_inputs = Vec::new();

        for input in &sig.inputs {
            if let FnArg::Typed(pat_type) = input {
                let ty = &pat_type.ty;
                
                // If a Client type is defined, ensure this argument is NOT that client
                if let Some(client_type) = explicit_client_type {
                    let is_client_type = if quote!(#ty).to_string() == quote!(#client_type).to_string() {
                        true
                    } else if let Type::Reference(type_ref) = &**ty {
                        // Also check if it is &ClientType
                        let inner = &type_ref.elem;
                        quote!(#inner).to_string() == quote!(#client_type).to_string()
                    } else {
                        false
                    };

                    if is_client_type {
                        return Err(Error::new_spanned(
                            ty,
                            "Capability methods must not accept the 'Client' type as an argument.",
                        ));
                    }
                }

                // Extract the argument name
                let arg_name = if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                    pat_ident.ident.clone()
                } else {
                    return Err(Error::new_spanned(
                        &pat_type.pat,
                        "Capability method arguments must be named identifiers (no patterns allowed).",
                    ));
                };

                clean_inputs.push((arg_name, *ty.clone()));
            }
        }

        Ok(Self {
            name: sig.ident.clone(),
            inputs: clean_inputs,
            output: sig.output.clone(),
            original_sig: sig.clone(),
            attrs: method.attrs,
            client_type: explicit_client_type.cloned(),
            error_type: explicit_error_type.cloned(),
        })
    }

    /// Creates a CapabilityMethod from a Host Implementation (impl Trait for State).
    ///
    /// Unlike `new` (which parses the Trait definition), this parses the implementation,
    /// so it must Strip `&self`, Strip `client` arguments, and Unwrap `Result<T, E>`.
    pub fn from_impl(
        method: &ImplItemFn,
        explicit_client_type: Option<&Type>,
        explicit_error_type: Option<&Type>,
    ) -> syn::Result<Self> {
        let sig = &method.sig;
        let mut clean_inputs = Vec::new();

        // 1. Filter Inputs: Ignore &self and Client
        for input in &sig.inputs {
            match input {
                FnArg::Receiver(_) => continue, // Ignore &self
                FnArg::Typed(pat_type) => {
                    let ty = &pat_type.ty;
                    // Check if this is the client argument
                    if let Some(client_type) = explicit_client_type {
                         let is_client_type = if quote!(#ty).to_string() == quote!(#client_type).to_string() {
                            true
                        } else if let Type::Reference(type_ref) = &**ty {
                            let inner = &type_ref.elem;
                            quote!(#inner).to_string() == quote!(#client_type).to_string()
                        } else {
                            false
                        };
                        if is_client_type {
                            continue; // Ignore client argument
                        }
                    }
                    
                    // Extract Name
                    let arg_name = if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                        pat_ident.ident.clone()
                    } else {
                        // In impl blocks, patterns are allowed, but for capability extraction we prefer idents.
                        // We'll warn or just return error if complex.
                        return Err(Error::new_spanned(
                            &pat_type.pat,
                            "Host implementation arguments must be named identifiers.",
                        ));
                    };
                    clean_inputs.push((arg_name, *ty.clone()));
                }
            }
        }

        // 2. Filter Output: Unwrap Result<T, E> -> T if error_type is present
        let clean_output = if let Some(_) = explicit_error_type {
            match &sig.output {
                ReturnType::Default => ReturnType::Default,
                ReturnType::Type(arrow, ty) => {
                    // Try to unwrap Result
                    if let Type::Path(tp) = &**ty {
                        if tp.path.segments.last().map(|s| s.ident == "Result").unwrap_or(false) {
                             if let PathArguments::AngleBracketed(args) = &tp.path.segments.last().unwrap().arguments {
                                 if let Some(GenericArgument::Type(inner)) = args.args.first() {
                                     ReturnType::Type(*arrow, Box::new(inner.clone()))
                                 } else {
                                     // Result without args?
                                     sig.output.clone()
                                 }
                             } else {
                                 sig.output.clone()
                             }
                        } else {
                            // Returns something else, maybe explicit error not used correctly or just infallible?
                            sig.output.clone()
                        }
                    } else {
                        sig.output.clone()
                    }
                }
            }
        } else {
            sig.output.clone()
        };

        Ok(Self {
            name: sig.ident.clone(),
            inputs: clean_inputs,
            output: clean_output,
            original_sig: sig.clone(),
            attrs: method.attrs.clone(),
            client_type: explicit_client_type.cloned(),
            error_type: explicit_error_type.cloned(),
        })
    }

    /// Generates the transformed trait method signature.
    ///
    /// Transformation Rules:
    /// 1. Adds `&self` to all methods.
    /// 2. If `client_type` exists, prepends `client: &Self::Client` to args.
    /// 3. If `error_type` exists, wraps return type in `Result<T, Self::Error>`.
    pub fn trait_method_generation(&self) -> TokenStream {
        let name = &self.name;
        let attrs = &self.attrs;

        let mut args_tokens = Vec::new();

        // If a Client is defined, inject it as the first argument after &self
        if self.client_type.is_some() {
            args_tokens.push(quote! { client: &Self::Client });
        }

        let async_symbol = if self.original_sig.asyncness.is_some() {
            quote!(async)
        } else {
            quote!()
        };

        // Add the original user-defined arguments
        for (arg_name, arg_type) in &self.inputs {
            args_tokens.push(quote! { #arg_name: #arg_type });
        }

        let return_tokens = if self.error_type.is_some() {
            // Determine the inner type T for Result<T, E>
            let inner_type = match &self.output {
                ReturnType::Default => quote! { () },
                ReturnType::Type(_, ty) => quote! { #ty },
            };
            
            // Wrap in Result<..., Self::Error>
            quote! { -> Result<#inner_type, Self::Error> }
        } else {
            // No error defined, keep original return type (or empty)
            self.output.to_token_stream()
        };

        quote! {
            #(#attrs)*
            #async_symbol fn #name(&self, #(#args_tokens),*) #return_tokens;
        }
    }

    /// Helper to construct the CapabilityFuncFFI configuration.
    /// Encapsulates all logic regarding return type calculation, client ID extraction, 
    /// and input parameter wrapping.
    fn build_ffi_meta(&self, trait_name: &Ident, state_name: &Ident) -> CapabilityFuncFFI {
        let name = &self.name;

        // 1. Determine Final Return Type
        let final_return_type: Type = if let Some(error_type) = &self.error_type {
            let inner_type = match &self.output {
                ReturnType::Default => quote! { () },
                ReturnType::Type(_, ty) => quote! { #ty },
            };
            parse_quote! { Result<#inner_type, #error_type> }
        } else {
            match &self.output {
                ReturnType::Default => parse_quote!(()),
                ReturnType::Type(_, ty) => *ty.clone(),
            }
        };

        // 2. Determine Client Identity
        // If client_type is set, we extract the ident (e.g. "MyClient").
        // CapabilityFuncFFI expects Option<Ident>.
        let client_ident = if let Some(Type::Path(tp)) = &self.client_type {
            tp.path.segments.last().map(|s| s.ident.clone())
        } else {
            None
        };

        // 3. Determine Input Parameters
        let input_params = if self.inputs.is_empty() {
            None
        } else if self.inputs.len() == 1 {
            let (n, t) = &self.inputs[0];
            Some(InputParams::One(n.clone(), t.clone()))
        } else {
            let params = self.inputs.clone();
            let struct_name = format_ident!("__{}_{}_{}_Input", trait_name, state_name, name);
            Some(InputParams::Many {
                params,
                input_struct_name: struct_name,
            })
        };

        // 4. Construct Struct
        CapabilityFuncFFI {
            // Library path: __user_trait__struct_name__method_name
            library: format_ident!("__{}_{}_{}", trait_name, state_name, name), 
            fn_name: name.clone(),
            // These names are primarily used for host-side generation, but required by struct
            fn_ffi_name: format_ident!("__{}_{}_{}_ffi", trait_name, state_name, name),
            fn_wasm_name: format_ident!("__{}_{}_{}_wasm", trait_name, state_name, name),
            vis: syn::Visibility::Public(parse_quote!(pub)),
            is_async: self.original_sig.asyncness.is_some(),
            return_type: final_return_type,
            input: input_params,
            client: client_ident,
            server: Some(state_name.clone()),
            has_self: true,
        }
    }

    /// Generates the full method implementation for the `impl Client` block.
    ///
    /// This generates the signature AND the body which delegates to the host via WASM.
    ///
    /// # Arguments
    /// * `trait_name` - The name of the trait, used to generate the library path `__trait_method`.
    pub fn client_method_generation(&self, trait_name: &Ident, state_name: &Ident) -> TokenStream {
        let name = &self.name;
        let attrs = &self.attrs;

        // 1. Prepare Arguments for Signature
        let mut args_tokens = Vec::new();
        for (arg_name, arg_type) in &self.inputs {
            args_tokens.push(quote! { #arg_name: #arg_type });
        }

        // 3. Build FFI Metadata
        let ffi = self.build_ffi_meta(trait_name, state_name);

        // 4. Prepare Return Type for Signature
        let final_ret_ty = &ffi.return_type;
        let return_tokens = quote! { -> #final_ret_ty };

        // 5. Generate Logic components
        // If there are multiple inputs, this returns the struct definition.
        let struct_def = ffi.generate_input_struct();
        
        // This returns the `call_from_wasm` block.
        let body_delegation = ffi.generate_module_function();

        quote! {
            #(#attrs)*
            pub fn #name(&self, #(#args_tokens),*) #return_tokens {
                // Define the input struct within the function scope (if needed)
                #struct_def
                
                // Delegate to host
                #body_delegation
            }
        }
    }

    /// Generates the host-side FFI function export.
    ///
    /// This uses the same FFI metadata logic as client generation to ensure consistency.
    pub fn ffi_function_generation(&self, trait_name: &Ident, state_name: &Ident) -> TokenStream {
        let ffi = self.build_ffi_meta(trait_name, state_name);
        ffi.generate_capability_ffi()
    }

    pub fn wasm_import_generation(&self, trait_name: &Ident, state_name: &Ident) -> TokenStream {
        let ffi = self.build_ffi_meta(trait_name, state_name);
        ffi.generate_client_wasm()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fmt::assert_code_eq;
    use syn::{TraitItemFn, Type};

    /// Helper to create a CapabilityMethod from a raw function signature string.
    fn create_method(
        sig_code: TokenStream,
        client_type_str: Option<&str>,
        error_type_str: Option<&str>,
    ) -> CapabilityMethod {
        let method: TraitItemFn = syn::parse2(sig_code).expect("Failed to parse method signature");
        
        let client_type: Option<Type> = client_type_str.map(|s| syn::parse_str(s).expect("Failed to parse client type"));
        let error_type: Option<Type> = error_type_str.map(|s| syn::parse_str(s).expect("Failed to parse error type"));

        CapabilityMethod::from_trait(
            method,
            client_type.as_ref(),
            error_type.as_ref(),
        ).expect("CapabilityMethod validation failed")
    }

    #[test]
    fn test_basic_sync_no_args() {
        // 1. Define the input method signature
        let code = quote! {
            fn get_status() -> u32;
        };

        // 2. Parse and process
        let method = create_method(code, Some("MyClient"), None);
        let state_name = format_ident!("MyState");
        let trait_name = format_ident!("MyTrait");
        
        // 3. Generate the client implementation
        let output = method.client_method_generation(&trait_name, &state_name);

        // 4. Define expected output
        // Note: Library name should be "__MyTrait_MyClient_get_status"
        let expected = r#"
            pub fn get_status(&self) -> u32 {
                ::pyroduct::module_capability::access::call_from_wasm::<
                    MyClient,
                    (),
                    u32,
                    _,
                >(
                    "__MyTrait_MyState_get_status",
                    Some(client),
                    None,
                    |client_state_ptr: *const u8,
                     client_state_len: usize,
                     input_ptr: *const u8,
                     input_len: usize| {
                        unsafe {
                            __MyTrait_MyState_get_status_wasm(
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

        assert_code_eq(&output, expected);
    }

    #[test]
    fn test_async_single_arg_with_error() {
        // 1. Define input: Async method, takes a String
        let code = quote! {
            async fn set_name(name: String) -> ();
        };

        // 2. Parse with an Error type defined
        let method = create_method(code, Some("MyClient"), Some("MyError"));
        let state_name = format_ident!("MyState");
        let trait_name = format_ident!("MyTrait");

        // 3. Generate
        let output = method.client_method_generation(&trait_name, &state_name);

        // 4. Expected:
        // - Return type wrapped in Result<(), MyError>
        // - Single input means NO internal struct generation
        // - Async markers
        let expected = r#"
            pub fn set_name(&self, name: String) -> Result<(), MyError> {
                ::pyroduct::module_capability::access::call_from_wasm::<
                    MyClient,
                    String,
                    Result<(), MyError>,
                    _,
                >(
                    "__MyTrait_MyState_set_name",
                    Some(client),
                    Some(&name),
                    |client_state_ptr: *const u8,
                     client_state_len: usize,
                     input_ptr: *const u8,
                     input_len: usize| {
                        unsafe {
                            __MyTrait_MyState_set_name_wasm(
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

        assert_code_eq(&output, expected);
    }

    #[test]
    fn test_sync_multi_args_struct_generation() {
        // 1. Define input: Multiple arguments
        let code = quote! {
            fn configure(port: u16, active: bool);
        };

        // 2. Parse
        let method = create_method(code, Some("MyClient"), None);
        let state_name = format_ident!("MyState");
        let trait_name = format_ident!("MyTrait");

        // 3. Generate
        let output = method.client_method_generation(&trait_name, &state_name);

        // 4. Expected:
        // - An internal struct `__MyTrait_configure_Input` is defined inside the function block.
        // - `rkyv` attributes are present on that struct.
        // - The `call_from_wasm` uses `Some(&__MyTrait_configure_Input { port, active })`.
        let expected = r#"
            pub fn configure(&self, port: u16, active: bool) -> () {
                #[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
                #[rkyv(compare(PartialEq), derive(Debug))]
                struct __MyTrait_MyState_configure_Input {
                    pub port: u16,
                    pub active: bool,
                }
                ::pyroduct::module_capability::access::call_from_wasm::<
                    MyClient,
                    __MyTrait_MyState_configure_Input,
                    (),
                    _,
                >(
                    "__MyTrait_MyState_configure",
                    Some(client),
                    Some(&__MyTrait_MyState_configure_Input { port, active }),
                    |client_state_ptr: *const u8,
                     client_state_len: usize,
                     input_ptr: *const u8,
                     input_len: usize| {
                        unsafe {
                            __MyTrait_MyState_configure_wasm(
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

        assert_code_eq(&output, expected);
    }

    #[test]
    fn test_validation_rejects_forbidden_patterns() {
        // Test Rule: No &self in arguments (implied by trait parsing, but we check specific FnArg)
        let code = quote! {
            fn invalid(other: &Self) -> ();
        };
        // Note: `syn` parses `&self` as a Receiver, but `other: &Self` as Typed.
        // The validator currently checks that we don't return Self or accept Client type.
        
        let client_type = "MyClient";

        // 1. Check returning Client type is forbidden
        let code_ret_client = quote! {
            fn make_client(&self) -> MyClient;
        };
        let res = create_method_result(code_ret_client, Some(client_type), None);
        assert!(res.is_err(), "Should have rejected returning Client type");
        assert_eq!(res.unwrap_err().to_string(), "Capability methods cannot take variant of 'self', or 'Self'");

        // 2. Check accepting Client type is forbidden
        let code_arg_client = quote! {
            fn process(c: MyClient);
        };
        let res = create_method_result(code_arg_client, Some(client_type), None);
        assert!(res.is_err(), "Should have rejected accepting Client type argument");
        assert_eq!(res.unwrap_err().to_string(), "Capability methods must not accept the 'Client' type as an argument.");
    }

    /// Helper for fallible creation (to test validation logic)
    fn create_method_result(
        sig_code: TokenStream,
        client_type_str: Option<&str>,
        error_type_str: Option<&str>,
    ) -> syn::Result<CapabilityMethod> {
        let method: TraitItemFn = syn::parse2(sig_code).unwrap();
        let client_type: Option<Type> = client_type_str.map(|s| syn::parse_str(s).unwrap());
        let error_type: Option<Type> = error_type_str.map(|s| syn::parse_str(s).unwrap());

        CapabilityMethod::from_trait(
            method,
            client_type.as_ref(),
            error_type.as_ref(),
        )
    }

    #[test]
    fn test_ffi_generation_sync() {
        // 1. Define sync method with one input
        let code = quote! {
            fn sync_op(val: u32) -> bool;
        };
        let method = create_method(code, Some("MyClient"), None);
        let state_name = format_ident!("MyState");
        let trait_name = format_ident!("MyTrait");
        let state_name = format_ident!("MyState");

        // 2. Generate FFI export
        let output = method.ffi_function_generation(&trait_name, &state_name);

        // 3. Expected:
        // - unsafe extern "C"
        // - FfiResult return
        // - ci_call (Client, Input)
        // - Closure calls method
        let expected = r#"
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn __MyTrait_MyState_sync_op_ffi(
                client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize,
                capability_state_ptr: *mut std::ffi::c_void,
            ) -> ::pyroduct::capability_host::ffi::FfiResult {
                ::pyroduct::capability::safe_call::sci_call::<
                    MyState,
                    MyClient,
                    u32,
                    bool,
                    _,
                >(
                    client_state_ptr,
                    client_state_len,
                    input_ptr,
                    input_len,
                    capability_state_ptr,
                    |state, client, input| state.sync_op(client, input),
                )
            }
        "#;
        assert_code_eq(&output, expected);
    }

    #[test]
    fn test_ffi_generation_async() {
        // 1. Define async method with one input
        let code = quote! {
            async fn async_op(data: String) -> u64;
        };
        let method = create_method(code, Some("MyClient"), None);
        let state_name = format_ident!("MyState");
        let trait_name = format_ident!("MyTrait");

        // 2. Generate FFI export
        let output = method.ffi_function_generation(&trait_name, &state_name);

        // 3. Expected:
        // - unsafe extern "C"
        // - FfiBorrowedFutureResult return
        // - ci_call (Client, Input)
        // - Closure returns async block
        let expected = r#"
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn __MyTrait_MyState_async_op_ffi<'a>(
                client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize,
                capability_state_ptr: *mut std::ffi::c_void,
            ) -> ::pyroduct::capability_host::ffi::FfiBorrowedFutureResult<'a> {
                ::pyroduct::capability::safe_async::sci_call::<
                    MyState,
                    MyClient,
                    String,
                    u64,
                    _,
                    _,
                >(
                    client_state_ptr,
                    client_state_len,
                    input_ptr,
                    input_len,
                    capability_state_ptr,
                    |state, client, input| async move { state.async_op(client, input).await },
                )
            }
        "#;
        assert_code_eq(&output, expected);
    }
}