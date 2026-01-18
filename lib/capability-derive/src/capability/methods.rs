use heck::AsSnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Error, FnArg, GenericArgument, Ident, ImplItemFn, PathArguments, ReturnType, TraitItemFn, Type,
    parse_quote, parse2,
};

use crate::{capability_ffi::{CapabilityFuncFFI, InputParams}, utils::{extract_ident_ignoring_ref, is_self_ref_or_type}};

/// Represents a validated method within the Capability trait.
#[derive(Debug, Clone, PartialEq)]
pub struct CapabilityMethod {
    pub name: Ident,
    pub inputs: Vec<(Ident, Type)>,
    pub output: ReturnType,
    pub original_sig: syn::Signature,
    pub attrs: Vec<syn::Attribute>,
    pub client: Option<Ident>,
    pub error_type: Option<Type>,
}

impl CapabilityMethod {
    pub fn from_trait(
        method: TraitItemFn,
        explicit_client: Option<&Ident>,
        explicit_error_type: Option<&Type>, // Passed in as requested, even if unused for current validation rules
    ) -> syn::Result<Self> {
        let sig = &method.sig;

        // --------------------------------------------------------
        // Rule 1: Do not have a &self (or self, &mut self)
        // --------------------------------------------------------
        for input in &sig.inputs {
            if is_self_ref_or_type(input) {
                return Err(Error::new_spanned(
                    input,
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
            if let Some(client) = explicit_client {
                if quote!(#ty).to_string().contains(&client.to_string()) {
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
                if let Some(client) = explicit_client {
                    let input_type = pat_type.ty.as_ref();
                    if let Some(input_ident) = extract_ident_ignoring_ref(input_type) {
                        if input_ident == client {
                            return Err(Error::new_spanned(
                                &pat_type.pat,
                                "Cannot input a client into the trait of a capability, automatically added",
                            ));
                        }
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

                clean_inputs.push((arg_name, pat_type.ty.as_ref().clone()));
            }
        }

        // --------------------------------------------------------
        // Rule 5: Strict Error Type Enforcement
        // --------------------------------------------------------
        let output = if let Some(expected_error) = explicit_error_type {
            transform_return_type(&sig.output, expected_error)?
        } else {
            sig.output.clone()
        };

        Ok(Self {
            name: sig.ident.clone(),
            inputs: clean_inputs,
            output,
            original_sig: sig.clone(),
            attrs: method.attrs,
            client: explicit_client.cloned(),
            error_type: explicit_error_type.cloned(),
        })
    }

    /// Generates the transformed trait method signature.
    ///
    /// Transformation Rules:
    /// 1. Adds `&self` to all methods.
    /// 2. If `client` exists, prepends `client: &Self::Client` to args.
    /// 3. If `error_type` exists, wraps return type in `Result<T, Self::Error>`.
    pub fn trait_method_generation(&self) -> TokenStream {
        let name = &self.name;
        let attrs = &self.attrs;

        let mut args_tokens = Vec::new();

        // If a Client is defined, inject it as the first argument after &self
        if self.client.is_some() {
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

        let return_tokens = &self.original_sig.output;

        quote! {
            #(#attrs)*
            #async_symbol fn #name(&self, #(#args_tokens),*) #return_tokens;
        }
    }

    /// Helper to construct the CapabilityFuncFFI configuration.
    /// Encapsulates all logic regarding return type calculation, client ID extraction,
    /// and input parameter wrapping.
    pub fn build_ffi_meta(&self, trait_name: &Ident, state_name: &Ident) -> CapabilityFuncFFI {
        let name = &self.name;

        // 2. Determine Client Identity
        // If client is set, we extract the ident (e.g. "MyClient").
        // CapabilityFuncFFI expects Option<Ident>.
        let client_ident = self.client.clone();
        let ident_root = format_ident!("__{}__{}__{}", AsSnakeCase(trait_name.to_string()).to_string(), AsSnakeCase(state_name.to_string()).to_string(), AsSnakeCase(name.to_string()).to_string());

        // 3. Determine Input Parameters
        let input_params = if self.inputs.is_empty() {
            None
        } else if self.inputs.len() == 1 {
            let (n, t) = &self.inputs[0];
            Some(InputParams::One(n.clone(), t.clone()))
        } else {
            let params = self.inputs.clone();
            let struct_name = format_ident!("{}__Input", ident_root);
            Some(InputParams::Many {
                params,
                input_struct_name: struct_name,
            })
        };

        // 4. Construct Struct
        CapabilityFuncFFI {
            // Library path: __user_trait__struct_name__method_name
            library: ident_root.clone(),
            fn_name: name.clone(),
            // These names are primarily used for host-side generation, but required by struct
            fn_ffi_name: format_ident!("{}__ffi", ident_root),
            fn_wasm_name: format_ident!("{}__wasm", ident_root),
            vis: syn::Visibility::Public(parse_quote!(pub)),
            is_async: self.original_sig.asyncness.is_some(),
            return_type: self.output.clone(),
            input: input_params,
            client: client_ident,
            server: Some(state_name.clone()),
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
        let return_tokens = &ffi.return_type;

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
}

/// Validates that if an Error type is provided, the return type is Result<T, Error>.
pub fn transform_return_type(output: &ReturnType, target_error: &Type) -> syn::Result<ReturnType> {
    match output {
        // Handle cases where no return type is specified (e.g., fn logic())
        ReturnType::Default => {
            Err(Error::new_spanned(
                output,
                format!("Method must return Result<T, {}> or Result<T, Self::Error>.", quote!(#target_error).to_string()),
            ))
        }
        ReturnType::Type(arrow, ty) => {
            // 1. Extract the inner T and E from Result<T, E>
            let (ok_type, err_type) = extract_result_parts(ty)
                .ok_or_else(|| Error::new_spanned(
                    ty,
                    format!("Method must return Result<T, {}> or Result<T, Self::Error>.", quote!(#target_error).to_string()),
                ))?;
            
            // 2. Normalize types to strings for comparison
            let actual_err_str = quote!(#err_type).to_string().replace(" ", "");
            let target_err_str = quote!(#target_error).to_string().replace(" ", "");
            let self_err_str = "Self::Error";
            
            // 3. Validation Logic
            // We allow the change if it's already the target error or if it's "Self::Error"
            if actual_err_str != target_err_str && actual_err_str != self_err_str {
                return Err(Error::new_spanned(
                    err_type,
                    format!(
                        "Invalid error type. Expected '{}' or 'Self::Error', found '{}'.", 
                        target_err_str, 
                        actual_err_str
                    )
                ));
            }

            // 4. Construct the new Result<T, MyError>
            // This replaces whatever was there (even Self::Error) with MyError
            let new_ty: Type = parse2(quote! {
                Result<#ok_type, #target_error>
            })?;

            Ok(ReturnType::Type(*arrow, Box::new(new_ty)))
        }
    }
}

/// Helper to decompose a Result<T, E> into (T, E).
fn extract_result_parts(ty: &Type) -> Option<(&Type, &Type)> {
    if let Type::Path(tp) = ty {
        if let Some(segment) = tp.path.segments.last() {
             if segment.ident == "Result" {
                 if let PathArguments::AngleBracketed(args) = &segment.arguments {
                     if args.args.len() == 2 {
                         let t = if let GenericArgument::Type(ty) = &args.args[0] { ty } else { return None; };
                         let e = if let GenericArgument::Type(ty) = &args.args[1] { ty } else { return None; };
                         return Some((t, e));
                     }
                 }
             }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fmt::assert_code_eq_token;
    use syn::{TraitItemFn, Type};

    /// Helper to create a CapabilityMethod from a raw function signature string.
    fn create_method_trait(
        sig_code: TokenStream,
        client_str: Option<&str>,
        error_type_str: Option<&str>,
    ) -> CapabilityMethod {
        let method: TraitItemFn = syn::parse2(sig_code).expect("Failed to parse method signature");

        let client: Option<Ident> =
            client_str.map(|s| format_ident!("{s}"));
        let error_type: Option<Type> =
            error_type_str.map(|s| syn::parse_str(s).expect("Failed to parse error type"));

        CapabilityMethod::from_trait(method, client.as_ref(), error_type.as_ref())
            .expect("CapabilityMethod validation failed")
    }

    #[test]
    fn test_basic_sync_no_args() {
        // 1. Define the input method signature
        let code = quote! {
            fn get_status() -> u32;
        };

        // 2. Parse and process
        let method = create_method_trait(code, Some("MyClient"), None);
        let state_name = format_ident!("MyState");
        let trait_name = format_ident!("MyTrait");
        let method_ffi = method.build_ffi_meta(&trait_name, &state_name);
        let struct_tokens = method_ffi.generate_input_struct();
        let wasm_call = method_ffi.generate_module_function();
        // 3. Generate
        let output = method.client_method_generation(&trait_name, &state_name);

        // 4. Expected
        let expected = quote! {
            pub fn get_status(&self) -> u32 {
                #struct_tokens
                #wasm_call
            }
        };

        assert_code_eq_token(&output, &expected);
    }

    #[test]
    fn test_async_single_arg_with_error() {
        // 1. Define input: Async method, takes a String
        let code = quote! {
            async fn set_name(name: String) -> Result<(), Self::Error>;
        };

        // 2. Parse with an Error type defined
        let method = create_method_trait(code, Some("MyClient"), Some("MyError"));
        let state_name = format_ident!("MyState");
        let trait_name = format_ident!("MyTrait");
        let method_ffi = method.build_ffi_meta(&trait_name, &state_name);
        let struct_tokens = method_ffi.generate_input_struct();
        let wasm_call = method_ffi.generate_module_function();
        // 3. Generate
        let output = method.client_method_generation(&trait_name, &state_name);

        // 4. Expected
        let expected = quote! {
            pub fn set_name(&self, name: String) -> Result<(), MyError> {
                #struct_tokens
                #wasm_call
            }
        };

        assert_code_eq_token(&output, &expected);
    }

    #[test]
    fn test_sync_multi_args_struct_generation() {
        // 1. Define input: Multiple arguments
        let code = quote! {
            fn configure(port: u16, active: bool);
        };

        // 2. Parse
        let method = create_method_trait(code, Some("MyClient"), None);
        let state_name = format_ident!("MyState");
        let trait_name = format_ident!("MyTrait");

        let method_ffi = method.build_ffi_meta(&trait_name, &state_name);
        let struct_tokens = method_ffi.generate_input_struct();
        let wasm_call = method_ffi.generate_module_function();
        // 3. Generate
        let output = method.client_method_generation(&trait_name, &state_name);

        // 4. Expected
        let expected = quote! {
            pub fn configure(&self, port: u16, active: bool) {
                #struct_tokens
                #wasm_call
            }
        };

        assert_code_eq_token(&output, &expected);
    }

    #[test]
    fn test_validation_rejects_forbidden_patterns() {
        let code = quote! {
            fn invalid(other: &Self) -> ();
        };
        let res = create_method_result(code, None, None);
        assert!(res.is_err(), "Capability methods cannot take self");
        assert_eq!(
            res.unwrap_err().to_string(),
            "Capability methods cannot take variant of 'self', or 'Self'"
        );

        let client = "MyClient";

        let code_ret_client = quote! {
            fn make_client(&self) -> MyClient;
        };
        let res = create_method_result(code_ret_client, Some(client), None);
        assert!(res.is_err(), "Should have rejected returning Client type");
        assert_eq!(
            res.unwrap_err().to_string(),
            "Capability methods cannot take variant of 'self', or 'Self'" // Note: the validator catches 'Self' returns as "Cannot return Self" but typed client returns get caught by check #2
        );

        let code_arg_client = quote! {
            fn process(c: MyClient);
        };
        let res = create_method_result(code_arg_client, Some(client), None);
        assert!(
            res.is_err(),
            "Should have rejected accepting Client type argument"
        );
        assert_eq!(
            res.unwrap_err().to_string(),
            "Capability methods must not accept the 'Client' type as an argument."
        );

        let code_arg_client = quote! {
            fn process(c: f32) -> u32;
        };
        let res = create_method_result(code_arg_client, None, Some("MyError"));
        assert!(
            res.is_err(),
            "Should have rejected a non result return"
        );
        assert_eq!(
            res.unwrap_err().to_string(),
            "Method must return Result<T, MyError> or Result<T, Self::Error>."
        );
    }

    fn create_method_result(
        sig_code: TokenStream,
        client_str: Option<&str>,
        error_type_str: Option<&str>,
    ) -> syn::Result<CapabilityMethod> {
        let method: TraitItemFn = syn::parse2(sig_code).unwrap();
        let client: Option<Ident> = client_str.map(|s| format_ident!("{s}"));
        let error_type: Option<Type> = error_type_str.map(|s| syn::parse_str(s).unwrap());

        CapabilityMethod::from_trait(method, client.as_ref(), error_type.as_ref())
    }

    #[test]
    fn test_ffi_generation_sync() {
        // 1. Define sync method with one input
        let code = quote! {
            fn sync_op(val: u32) -> bool;
        };
        let method = create_method_trait(code, Some("MyClient"), None);
        let trait_name = format_ident!("MyTrait");
        let state_name = format_ident!("MyState");

        // 2. Generate FFI export
        let output = method.build_ffi_meta(&trait_name, &state_name);

        let expected = CapabilityFuncFFI {
            library: format_ident!("__my_trait__my_state__sync_op"),
            fn_name: format_ident!("sync_op"), // The actual method name
            fn_ffi_name: format_ident!("__my_trait__my_state__sync_op__ffi"),
            fn_wasm_name: format_ident!("__my_trait__my_state__sync_op__wasm"),
            vis: syn::Visibility::Public(parse_quote!(pub)),
            is_async: false,
            return_type: parse_quote!(-> bool), 
            input: Some(InputParams::One(format_ident!("val"), parse_quote!(u32))),
            client: Some(format_ident!("MyClient")),
            server: Some(format_ident!("MyState")),
        };

        // 3. Verify that the struct fields match expectation
        assert_eq!(&output, &expected);
    }

    #[test]
    fn test_ffi_generation_async() {
        // 1. Define async method with one input
        let code = quote! {
            async fn async_op(data: String) -> u64;
        };
        let method = create_method_trait(code, Some("MyClient"), None);
        let state_name = format_ident!("MyState");
        let trait_name = format_ident!("MyTrait");

        // 2. Generate FFI export
        let output = method.build_ffi_meta(&trait_name, &state_name);

        let expected = CapabilityFuncFFI {
            library: format_ident!("__my_trait__my_state__async_op"),
            fn_name: format_ident!("async_op"),
            fn_ffi_name: format_ident!("__my_trait__my_state__async_op__ffi"),
            fn_wasm_name: format_ident!("__my_trait__my_state__async_op__wasm"),
            vis: syn::Visibility::Public(parse_quote!(pub)),
            is_async: true,
            return_type: parse_quote!(-> u64),
            input: Some(InputParams::One(format_ident!("data"), parse_quote!(String))),
            client: Some(format_ident!("MyClient")),
            server: Some(format_ident!("MyState")),
        };

        // 3. Verify that the struct fields match expectation
        assert_eq!(&output, &expected);
    }
}