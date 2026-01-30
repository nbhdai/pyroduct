//! Capability method parsing and validation for impl blocks
//!
//! Valid signatures:
//! - `fn method_name(&self, client: &ClientType, ...args) -> ReturnType`
//! - `async fn method_name(&self, client: &ClientType, ...args) -> ReturnType`
//!
//! If an error type is defined, return types must be `Result<T, ErrorType>` or `Result<T, Self::Error>`

use std::rc::Rc;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Error, FnArg, GenericArgument, Ident, ImplItemFn, Pat, PathArguments, ReturnType, Type,
    parse_quote,
};

use crate::ffi::{CapabilityFuncFFI, InputParams};
use crate::paths::CapabilityIdent;
use crate::utils::extract_ident_from_type;

/// Represents a validated method within a capability impl block.
#[derive(Debug, Clone)]
pub struct ImplMethod {
    pub name: Ident,
    pub class: Rc<CapabilityIdent>,
    pub inputs: Vec<(Ident, Type)>,
    pub output: ReturnType,
    pub is_async: bool,
    pub body: syn::Block,
    pub attrs: Vec<syn::Attribute>,
}

impl ImplMethod {
    /// Parse and validate a method from an impl block
    pub fn parse(
        f: &ImplItemFn,
        class: &Rc<CapabilityIdent>,
    ) -> syn::Result<Self> {
        let sig = &f.sig;
        let name = sig.ident.clone();

        // 1. Validate &self as first parameter
        match sig.inputs.first() {
            Some(FnArg::Receiver(r)) => {
                if r.mutability.is_some() {
                    return Err(Error::new_spanned(
                        r,
                        "Capability methods must take &self (not &mut self)",
                    ));
                }
                if r.reference.is_none() {
                    return Err(Error::new_spanned(
                        r,
                        "Capability methods must take &self (not self)",
                    ));
                }
            }
            Some(arg) => {
                return Err(Error::new_spanned(
                    arg,
                    "Capability methods must take &self as first parameter",
                ));
            }
            None => {
                return Err(Error::new_spanned(
                    &sig,
                    "Capability methods must take &self",
                ));
            }
        }

        // 2. Validate second parameter is client: &ClientType
        let client_param = sig.inputs.iter().nth(1);
        match client_param {
            Some(FnArg::Typed(pt)) => {
                // Validate parameter name
                if let Pat::Ident(pi) = &*pt.pat {
                    let param_name = pi.ident.to_string();
                    if !param_name.starts_with('_') && param_name != "client" {
                        return Err(Error::new_spanned(
                            &pi.ident,
                            "Second parameter should be named 'client' or '_client'",
                        ));
                    }
                } else {
                    return Err(Error::new_spanned(&pt.pat, "Expected simple identifier"));
                }

                // Validate type is reference to Client
                if let Type::Reference(r) = &*pt.ty {
                    let param_type = extract_ident_from_type(&r.elem)?;
                    if param_type != class.client_tn {
                        return Err(Error::new_spanned(
                            &pt.ty,
                            format!(
                                "Expected &{}, found &{}",
                                class.client_tn, param_type
                            ),
                        ));
                    }
                } else {
                    return Err(Error::new_spanned(
                        &pt.ty,
                        format!("Expected &{}", class.client_tn),
                    ));
                }
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
        }

        // 3. Collect remaining inputs (skip &self and &Client)
        let mut inputs = Vec::new();
        for arg in sig.inputs.iter().skip(2) {
            if let FnArg::Typed(pt) = arg {
                let arg_name = if let Pat::Ident(pi) = &*pt.pat {
                    pi.ident.clone()
                } else {
                    return Err(Error::new_spanned(
                        &pt.pat,
                        "Method arguments must be named identifiers (no patterns allowed)",
                    ));
                };
                inputs.push((arg_name, (*pt.ty).clone()));
            }
        }

        // 4. Validate return type if error type is defined
        let output = if let Some(expected_error) = &class.error_tn {
            validate_return_type(&sig.output, expected_error)?
        } else {
            sig.output.clone()
        };

        Ok(Self {
            name,
            class: class.clone(),
            inputs,
            output,
            is_async: sig.asyncness.is_some(),
            body: f.block.clone(),
            attrs: f.attrs.clone(),
        })
    }

    /// Build the FFI metadata for this method
    pub fn build_ffi(&self) -> CapabilityFuncFFI {
        let input = if self.inputs.is_empty() {
            None
        } else if self.inputs.len() == 1 {
            let (n, t) = &self.inputs[0];
            Some(InputParams::One(n.clone(), t.clone()))
        } else {
            Some(InputParams::Many {
                params: self.inputs.clone(),
            })
        };

        CapabilityFuncFFI {
            class: Some(self.class.clone()),
            constructor: false,
            fn_name: self.name.clone(),
            vis: parse_quote!(pub),
            is_async: self.is_async,
            return_type: self.output.clone(),
            input,
        }
    }

    /// Generate the server-side impl method
    pub fn generate_server_method(&self) -> TokenStream {
        let name = &self.name;
        let attrs = &self.attrs;
        let client = &self.class.client_tn;
        let body = &self.body;
        let output = &self.output;

        let async_kw = if self.is_async {
            quote!(async)
        } else {
            quote!()
        };

        let args: Vec<_> = self.inputs.iter().map(|(n, t)| quote!(#n: #t)).collect();

        quote! {
            #(#attrs)*
            pub #async_kw fn #name(&self, _client: &#client, #(#args),*) #output #body
        }
    }

    /// Generate the client-side method that delegates to WASM
    pub fn generate_client_method(&self, module: &Ident) -> TokenStream {
        let name = &self.name;
        let attrs = &self.attrs;
        let output = &self.output;

        let ffi = self.build_ffi();
        let struct_def = ffi.generate_input_struct();
        let wasm_call = ffi.generate_wasm_call(Some(module));

        let args: Vec<_> = self.inputs.iter().map(|(n, t)| quote!(#n: #t)).collect();

        quote! {
            #(#attrs)*
            pub fn #name(&self, #(#args),*) #output {
                #struct_def
                #wasm_call
            }
        }
    }
}

/// Validates that if an Error type is provided, the return type is Result<T, Error>.
fn validate_return_type(output: &ReturnType, target_error: &Type) -> syn::Result<ReturnType> {
    match output {
        ReturnType::Default => Err(Error::new_spanned(
            output,
            format!(
                "Method must return Result<T, {}> or Result<T, Self::Error> when error type is defined",
                quote!(#target_error)
            ),
        )),
        ReturnType::Type(arrow, ty) => {
            // Extract the inner T and E from Result<T, E>
            let (ok_type, err_type) = extract_result_parts(ty).ok_or_else(|| {
                Error::new_spanned(
                    ty,
                    format!(
                        "Method must return Result<T, {}> or Result<T, Self::Error>",
                        quote!(#target_error)
                    ),
                )
            })?;

            // Normalize types to strings for comparison
            let actual_err_str = quote!(#err_type).to_string().replace(" ", "");
            let target_err_str = quote!(#target_error).to_string().replace(" ", "");
            let self_err_str = "Self::Error";

            // Validation: allow target error or Self::Error
            if actual_err_str != target_err_str && actual_err_str != self_err_str {
                return Err(Error::new_spanned(
                    err_type,
                    format!(
                        "Invalid error type. Expected '{}' or 'Self::Error', found '{}'",
                        target_err_str, actual_err_str
                    ),
                ));
            }

            // Construct the normalized Result<T, TargetError>
            let new_ty: Type = syn::parse_quote! {
                Result<#ok_type, #target_error>
            };

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
    use syn::parse_quote;

    fn mock_class(error: Option<&str>) -> Rc<CapabilityIdent> {
        Rc::new(CapabilityIdent {
            config_tn: None,
            state_tn: format_ident!("MyServer"),
            client_tn: format_ident!("MyClient"),
            error_tn: error.map(|s| syn::parse_str(s).unwrap()),
        })
    }

    #[test]
    fn test_parse_basic_method() {
        let class = mock_class(None);
        let f: ImplItemFn = parse_quote! {
            fn get_value(&self, _client: &MyClient) -> u32 { 42 }
        };

        let method = ImplMethod::parse(&f, &class).unwrap();
        assert_eq!(method.name.to_string(), "get_value");
        assert!(method.inputs.is_empty());
        assert!(!method.is_async);
    }

    #[test]
    fn test_parse_async_method() {
        let class = mock_class(None);
        let f: ImplItemFn = parse_quote! {
            async fn fetch(&self, client: &MyClient) -> String { String::new() }
        };

        let method = ImplMethod::parse(&f, &class).unwrap();
        assert!(method.is_async);
    }

    #[test]
    fn test_parse_method_with_args() {
        let class = mock_class(None);
        let f: ImplItemFn = parse_quote! {
            fn set_value(&self, _client: &MyClient, value: u32, name: String) { }
        };

        let method = ImplMethod::parse(&f, &class).unwrap();
        assert_eq!(method.inputs.len(), 2);
        assert_eq!(method.inputs[0].0.to_string(), "value");
        assert_eq!(method.inputs[1].0.to_string(), "name");
    }

    #[test]
    fn test_parse_method_with_error() {
        let class = mock_class(Some("MyError"));
        let f: ImplItemFn = parse_quote! {
            fn fallible(&self, _client: &MyClient) -> Result<u32, MyError> { Ok(42) }
        };

        let method = ImplMethod::parse(&f, &class).unwrap();
        let output = method.output;
        let output_str = quote!(#output).to_string();
        assert!(output_str.contains("Result"));
    }

    #[test]
    fn test_parse_method_self_error() {
        let class = mock_class(Some("MyError"));
        let f: ImplItemFn = parse_quote! {
            fn fallible(&self, _client: &MyClient) -> Result<u32, Self::Error> { Ok(42) }
        };

        let method = ImplMethod::parse(&f, &class).unwrap();
        // Should normalize Self::Error to MyError
        let output = &method.output;
        let output_str = quote!(#output).to_string();
        assert!(output_str.contains("MyError"));
    }

    #[test]
    fn test_reject_missing_self() {
        let class = mock_class(None);
        let f: ImplItemFn = parse_quote! {
            fn bad(client: &MyClient) -> u32 { 42 }
        };

        let err = ImplMethod::parse(&f, &class).unwrap_err();
        assert!(err.to_string().contains("&self"));
    }

    #[test]
    fn test_reject_mut_self() {
        let class = mock_class(None);
        let f: ImplItemFn = parse_quote! {
            fn bad(&mut self, _client: &MyClient) -> u32 { 42 }
        };

        let err = ImplMethod::parse(&f, &class).unwrap_err();
        assert!(err.to_string().contains("&self"));
    }

    #[test]
    fn test_reject_missing_client() {
        let class = mock_class(None);
        let f: ImplItemFn = parse_quote! {
            fn bad(&self) -> u32 { 42 }
        };

        let err = ImplMethod::parse(&f, &class).unwrap_err();
        assert!(err.to_string().contains("client"));
    }

    #[test]
    fn test_reject_wrong_client_type() {
        let class = mock_class(None);
        let f: ImplItemFn = parse_quote! {
            fn bad(&self, client: &OtherClient) -> u32 { 42 }
        };

        let err = ImplMethod::parse(&f, &class).unwrap_err();
        assert!(err.to_string().contains("MyClient"));
    }

    #[test]
    fn test_reject_non_reference_client() {
        let class = mock_class(None);
        let f: ImplItemFn = parse_quote! {
            fn bad(&self, client: MyClient) -> u32 { 42 }
        };

        let err = ImplMethod::parse(&f, &class).unwrap_err();
        assert!(err.to_string().contains("&MyClient"));
    }

    #[test]
    fn test_reject_wrong_error_type() {
        let class = mock_class(Some("MyError"));
        let f: ImplItemFn = parse_quote! {
            fn bad(&self, _client: &MyClient) -> Result<u32, OtherError> { Ok(42) }
        };

        let err = ImplMethod::parse(&f, &class).unwrap_err();
        assert!(err.to_string().contains("MyError"));
    }

    #[test]
    fn test_reject_non_result_with_error_type() {
        let class = mock_class(Some("MyError"));
        let f: ImplItemFn = parse_quote! {
            fn bad(&self, _client: &MyClient) -> u32 { 42 }
        };

        let err = ImplMethod::parse(&f, &class).unwrap_err();
        assert!(err.to_string().contains("Result"));
    }

    #[test]
    fn test_reject_no_return_with_error_type() {
        let class = mock_class(Some("MyError"));
        let f: ImplItemFn = parse_quote! {
            fn bad(&self, _client: &MyClient) { }
        };

        let err = ImplMethod::parse(&f, &class).unwrap_err();
        assert!(err.to_string().contains("Result"));
    }

    #[test]
    fn test_build_ffi_no_args() {
        let class = mock_class(None);
        let f: ImplItemFn = parse_quote! {
            fn get(&self, _client: &MyClient) -> u32 { 42 }
        };

        let method = ImplMethod::parse(&f, &class).unwrap();
        let ffi = method.build_ffi();

        assert!(ffi.input.is_none());
        assert!(!ffi.is_async);
        assert!(!ffi.constructor);
    }

    #[test]
    fn test_build_ffi_single_arg() {
        let class = mock_class(None);
        let f: ImplItemFn = parse_quote! {
            fn set(&self, _client: &MyClient, value: u32) { }
        };

        let method = ImplMethod::parse(&f, &class).unwrap();
        let ffi = method.build_ffi();

        assert!(matches!(ffi.input, Some(InputParams::One(_, _))));
    }

    #[test]
    fn test_build_ffi_multi_args() {
        let class = mock_class(None);
        let f: ImplItemFn = parse_quote! {
            fn configure(&self, _client: &MyClient, port: u16, host: String) { }
        };

        let method = ImplMethod::parse(&f, &class).unwrap();
        let ffi = method.build_ffi();

        assert!(matches!(ffi.input, Some(InputParams::Many { .. })));
    }
}