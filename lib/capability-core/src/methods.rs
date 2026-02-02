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
    pub client_param: Ident,
    pub inputs: Vec<(Ident, Type)>,
    pub output: ReturnType,
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

        // 4. Validate return type
        let output = if let Some(expected_error) = &class.error_tn {
            validate_return_type(&sig.output, expected_error)?
        } else {
            sig.output.clone()
        };

        Ok(Self {
            name,
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

    /// Generate the server-side impl method.
    /// This preserves &mut self if the user wrote it, to allow state mutation on the server.
    pub fn generate_server_method(&self) -> TokenStream {
        let name = &self.name;
        let attrs = &self.attrs;
        let client_type = &self.class.client_tn;
        let client_var = &self.client_param;
        let body = &self.body;
        let output = &self.output;

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

    /// Generate the client-side method.
    /// This ALWAYS uses &self, because the client proxy should be immutable.
    pub fn generate_client_method(&self, module: &Ident) -> TokenStream {
        let name = &self.name;
        let attrs = &self.attrs;
        let output = &self.output;

        let ffi = self.build_ffi();
        let wasm_call = ffi.generate_wasm_call(Some(module));

        // Client method forces &self (immutable)
        let args: Vec<_> = self.inputs.iter().map(|(n, t)| quote!(#n: #t)).collect();

        quote! {
            #(#attrs)*
            fn #name(&self, #(#args),*) #output {
                #wasm_call
            }
        }
    }

    pub fn doc_attrs(&self) -> Vec<&syn::Attribute> {
        self.attrs
            .iter()
            .filter(|attr| attr.path().is_ident("doc"))
            .collect()
    }
}

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
            let (ok_type, err_type) = extract_result_parts(ty).ok_or_else(|| {
                Error::new_spanned(
                    ty,
                    format!(
                        "Method must return Result<T, {}> or Result<T, Self::Error>",
                        quote!(#target_error)
                    ),
                )
            })?;

            let actual_err_str = quote!(#err_type).to_string().replace(" ", "");
            let target_err_str = quote!(#target_error).to_string().replace(" ", "");
            let self_err_str = "Self::Error";

            if actual_err_str != target_err_str && actual_err_str != self_err_str {
                return Err(Error::new_spanned(
                    err_type,
                    format!(
                        "Invalid error type. Expected '{}' or 'Self::Error', found '{}'",
                        target_err_str, actual_err_str
                    ),
                ));
            }

            let new_ty: Type = syn::parse_quote! {
                Result<#ok_type, #target_error>
            };

            Ok(ReturnType::Type(*arrow, Box::new(new_ty)))
        }
    }
}

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
        let output = method.generate_client_method(&module);

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
}
