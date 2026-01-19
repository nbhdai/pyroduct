use std::rc::Rc;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Error, FnArg, GenericArgument, Ident, PathArguments, ReturnType, TraitItemFn, Type,
    parse_quote, parse2,
};

use crate::{
    ffi::{CapabilityFuncFFI, InputParams},
    paths::ClassIdent,
    utils::{extract_ident_ignoring_ref, is_self_ref_or_type},
};

/// Represents a validated method within the Capability trait.
#[derive(Debug, Clone, PartialEq)]
pub struct CapabilityMethod {
    pub capability_name: Rc<str>,
    pub name: Ident,
    pub class: Rc<ClassIdent>,
    pub inputs: Vec<(Ident, Type)>,
    pub output: ReturnType,
    pub original_sig: syn::Signature,
    pub attrs: Vec<syn::Attribute>,
}

impl CapabilityMethod {
    pub fn from_trait(method: TraitItemFn, class: &Rc<ClassIdent>, capability_name: &Rc<str>) -> syn::Result<Self> {
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
            if quote!(#ty)
                .to_string()
                .contains(&class.client_tn.to_string())
            {
                return Err(Error::new_spanned(
                    ty,
                    "Capability methods cannot return the defined 'Client' type.",
                ));
            }
        }

        // --------------------------------------------------------
        // Rule 3: Input Validation (NO Client passed in)
        // --------------------------------------------------------
        let mut clean_inputs = Vec::new();

        for input in &sig.inputs {
            if let FnArg::Typed(pat_type) = input {
                let input_type = pat_type.ty.as_ref();
                if let Some(input_ident) = extract_ident_ignoring_ref(input_type) {
                    if input_ident == &class.client_tn {
                        return Err(Error::new_spanned(
                            &pat_type.pat,
                            "Trait methods of a capability cannot accept a Client",
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

                clean_inputs.push((arg_name, pat_type.ty.as_ref().clone()));
            }
        }

        // --------------------------------------------------------
        // Rule 5: Strict Error Type Enforcement
        // --------------------------------------------------------
        let output = if let Some(expected_error) = &class.error_tn {
            transform_return_type(&sig.output, expected_error)?
        } else {
            sig.output.clone()
        };

        Ok(Self {
            capability_name: capability_name.clone(),
            name: sig.ident.clone(),
            class: class.clone(),
            inputs: clean_inputs,
            output,
            original_sig: sig.clone(),
            attrs: method.attrs,
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
        let client = &self.class.client_tn;

        let mut args_tokens = Vec::new();

        args_tokens.push(quote! { client: &#client });

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
    pub fn build_ffi_meta(&self) -> CapabilityFuncFFI {
        let name = &self.name;

        // 3. Determine Input Parameters
        let input_params = if self.inputs.is_empty() {
            None
        } else if self.inputs.len() == 1 {
            let (n, t) = &self.inputs[0];
            Some(InputParams::One(n.clone(), t.clone()))
        } else {
            let params = self.inputs.clone();
            Some(InputParams::Many { params })
        };

        // 4. Construct Struct
        CapabilityFuncFFI {
            capability_name: self.capability_name.clone(),
            class: Some(self.class.clone()),
            fn_name: name.clone(),
            constructor: false,
            vis: syn::Visibility::Public(parse_quote!(pub)),
            is_async: self.original_sig.asyncness.is_some(),
            return_type: self.output.clone(),
            input: input_params,
        }
    }

    /// Generates the full method implementation for the `impl Client` block.
    ///
    /// This generates the signature AND the body which delegates to the host via WASM.
    ///
    /// # Arguments
    /// * `trait_name` - The name of the trait, used to generate the library path `__trait_method`.
    pub fn client_method_generation(&self, module: Option<&Ident>) -> TokenStream {
        let name = &self.name;
        let attrs = &self.attrs;

        // 1. Prepare Arguments for Signature
        let mut args_tokens = Vec::new();
        for (arg_name, arg_type) in &self.inputs {
            args_tokens.push(quote! { #arg_name: #arg_type });
        }

        // 3. Build FFI Metadata
        let ffi = self.build_ffi_meta();

        // 4. Prepare Return Type for Signature
        let return_tokens = &ffi.return_type;

        // 5. Generate Logic components
        // If there are multiple inputs, this returns the struct definition.
        let struct_def = ffi.generate_input_struct();

        // This returns the `call_from_wasm` block.
        let body_delegation = ffi.generate_wasm_call(module);

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
        ReturnType::Default => Err(Error::new_spanned(
            output,
            format!(
                "Method must return Result<T, {}> or Result<T, Self::Error>.",
                quote!(#target_error).to_string()
            ),
        )),
        ReturnType::Type(arrow, ty) => {
            // 1. Extract the inner T and E from Result<T, E>
            let (ok_type, err_type) = extract_result_parts(ty).ok_or_else(|| {
                Error::new_spanned(
                    ty,
                    format!(
                        "Method must return Result<T, {}> or Result<T, Self::Error>.",
                        quote!(#target_error).to_string()
                    ),
                )
            })?;

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
                        target_err_str, actual_err_str
                    ),
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
                        let t = if let GenericArgument::Type(ty) = &args.args[0] {
                            ty
                        } else {
                            return None;
                        };
                        let e = if let GenericArgument::Type(ty) = &args.args[1] {
                            ty
                        } else {
                            return None;
                        };
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
    use quote::format_ident;
    use syn::{TraitItemFn, Type};

    /// Helper to create a CapabilityMethod from a raw function signature string.
    fn create_method_trait(
        sig_code: TokenStream,
        error_type_str: Option<&str>,
    ) -> syn::Result<CapabilityMethod> {
        let method: TraitItemFn = syn::parse2(sig_code).expect("Failed to parse method signature");

        let error_type: Option<Type> =
            error_type_str.map(|s| syn::parse_str(s).expect("Failed to parse error"));

        let class = Rc::new(ClassIdent {
            trait_tn: format_ident!("MyTrait"),
            state_tn: format_ident!("MyServer"),
            client_tn: format_ident!("MyClient"),
            error_tn: error_type,
        });

        CapabilityMethod::from_trait(method, &class, &"cap".into())
    }

    #[test]
    fn test_basic_sync_no_args() {
        // 1. Define the input method signature
        let code = quote! {
            fn get_status() -> u32;
        };

        // 2. Parse and process
        let method = create_method_trait(code, None).unwrap();
        let method_ffi = method.build_ffi_meta();
        let struct_tokens = method_ffi.generate_input_struct();
        let wasm_call = method_ffi.generate_wasm_call(None);
        // 3. Generate
        let output = method.client_method_generation(None);

        // 4. Expected
        let expected = quote! {
            pub fn get_status(&self) -> u32 {
                #struct_tokens
                #wasm_call
            }
        };

        crate::fmt::assert_code_eq_token(&output, &expected);
    }

    #[test]
    fn test_async_single_arg_with_error() {
        // 1. Define input: Async method, takes a String
        let code = quote! {
            async fn set_name(name: String) -> Result<(), Self::Error>;
        };

        // 2. Parse with an Error type defined
        let method = create_method_trait(code, Some("MyError")).unwrap();
        let method_ffi = method.build_ffi_meta();
        let struct_tokens = method_ffi.generate_input_struct();
        let wasm_call = method_ffi.generate_wasm_call(None);
        // 3. Generate
        let output = method.client_method_generation(None);

        // 4. Expected
        let expected = quote! {
            pub fn set_name(&self, name: String) -> Result<(), MyError> {
                #struct_tokens
                #wasm_call
            }
        };

        crate::fmt::assert_code_eq_token(&output, &expected);
    }

    #[test]
    fn test_sync_multi_args_struct_generation() {
        // 1. Define input: Multiple arguments
        let code = quote! {
            fn configure(port: u16, active: bool);
        };

        // 2. Parse
        let method = create_method_trait(code, None).unwrap();

        let method_ffi = method.build_ffi_meta();
        let struct_tokens = method_ffi.generate_input_struct();
        let wasm_call = method_ffi.generate_wasm_call(None);
        // 3. Generate
        let output = method.client_method_generation(None);

        // 4. Expected
        let expected = quote! {
            pub fn configure(&self, port: u16, active: bool) {
                #struct_tokens
                #wasm_call
            }
        };

        crate::fmt::assert_code_eq_token(&output, &expected);
    }

    #[test]
    fn test_validation_rejects_forbidden_patterns() {
        let code = quote! {
            fn invalid(other: &Self) -> ();
        };
        let res = create_method_trait(code, None);
        assert!(res.is_err(), "Capability methods cannot take self");
        assert_eq!(
            res.unwrap_err().to_string(),
            "Capability methods cannot take variant of 'self', or 'Self'"
        );

        let code_ret_client = quote! {
            fn make_client(&self) -> MyClient;
        };
        let res = create_method_trait(code_ret_client, None);
        assert!(res.is_err(), "Should have rejected returning Client type");
        assert_eq!(
            res.unwrap_err().to_string(),
            "Capability methods cannot take variant of 'self', or 'Self'" // Note: the validator catches 'Self' returns as "Cannot return Self" but typed client returns get caught by check #2
        );

        let code_arg_client = quote! {
            fn process(c: MyClient);
        };
        let res = create_method_trait(code_arg_client, None);
        assert!(
            res.is_err(),
            "Should have rejected accepting Client type argument"
        );
        assert_eq!(
            res.unwrap_err().to_string(),
            "Trait methods of a capability cannot accept a Client"
        );

        let code_arg_client = quote! {
            fn process(c: f32) -> u32;
        };
        let res = create_method_trait(code_arg_client, Some("MyError"));
        assert!(res.is_err(), "Should have rejected a non result return");
        assert_eq!(
            res.unwrap_err().to_string(),
            "Method must return Result<T, MyError> or Result<T, Self::Error>."
        );
    }
}
