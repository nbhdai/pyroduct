//! Path and naming utilities for capability FFI generation
//!
//! This module centralizes all naming conventions used throughout the capability system
//! to ensure consistency between client and server sides.

use std::{ops::Deref, slice::Iter};

use heck::{AsSnakeCase, AsUpperCamelCase};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Error, GenericArgument, Ident, PathArguments, ReturnType, Type, parse_quote, token::RArrow,
};

/// Identity of the capability (State, Client, Error)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CapabilityIdent {
    pub pkg_name: String,
    pub pkg_version: String,
    /// The struct being implemented (e.g., "MyStruct")
    pub state_tn: Ident,
    /// The client type identifier (e.g., "MyClient")
    pub client_tn: Ident,
    /// The config type identifier (e.g., "MyConfig")
    pub config_tn: Option<Ident>,
    /// The error type, if present (e.g., "MyError")
    pub error_tn: Option<Type>,
}

impl CapabilityIdent {
    // ========================================================================
    // Method Paths
    // ========================================================================

    /// Library identifier for a method (e.g., __my_trait__my_state__method_name)
    pub fn cap_id(&self) -> String {
        self.pkg_name.to_string()
    }

    /// Library identifier for a method (e.g., __my_trait__my_state__method_name)
    pub fn trace_name(&self, name: &FnName) -> Ident {
        let state_snake = AsSnakeCase(self.state_tn.to_string()).to_string();
        let snake = AsSnakeCase(name.0.to_string()).to_string();
        format_ident!("p__{}__{}", state_snake, snake)
    }

    /// Library identifier for a method (e.g., __my_trait__my_state__method_name)
    pub fn class_name_static(&self) -> Ident {
        let state_snake = AsSnakeCase(self.state_tn.to_string())
            .to_string()
            .to_uppercase();
        format_ident!("p__{}", state_snake)
    }

    /// Library identifier for a method (e.g., __my_trait__my_state__method_name)
    pub fn trace_name_static(&self, name: &FnName) -> Ident {
        let state_snake = AsSnakeCase(self.state_tn.to_string())
            .to_string()
            .to_uppercase();
        let snake = AsSnakeCase(name.0.to_string()).to_string().to_uppercase();
        format_ident!("p__{}__{}", state_snake, snake)
    }

    /// FFI function name for a method (e.g., __my_trait__my_state__name__ffi)
    pub fn ffi_name(&self, name: &FnName) -> Ident {
        let state_snake = AsSnakeCase(self.state_tn.to_string()).to_string();
        let snake = AsSnakeCase(name.0.to_string()).to_string();
        format_ident!("p__{}__{}__ffi", state_snake, snake)
    }

    /// WASM import name for a method (e.g., __my_trait__my_state__name__wasm)
    pub fn wasm_name(&self, name: &FnName) -> Ident {
        let state_snake = AsSnakeCase(self.state_tn.to_string()).to_string();
        let snake = AsSnakeCase(name.0.to_string()).to_string();
        format_ident!("p__{}__{}__wasm", state_snake, snake)
    }

    /// Input struct name for a method with multiple parameters
    pub fn input_struct(&self, name: &FnName) -> Ident {
        let state_snake = AsUpperCamelCase(self.state_tn.to_string()).to_string();
        let snake = AsUpperCamelCase(name.0.to_string()).to_string();
        format_ident!("p__{}__{}__Input", state_snake, snake)
    }
}

#[derive(Debug, Clone)]
pub struct FnName(pub Ident);

impl FnName {
    pub fn trace_name(&self) -> Ident {
        format_ident!("p__{}", AsSnakeCase(self.0.to_string()).to_string())
    }

    pub fn trace_name_static(&self) -> Ident {
        format_ident!(
            "p__{}",
            AsSnakeCase(self.0.to_string()).to_string().to_uppercase()
        )
    }

    /// Get the FFI function name
    pub fn fn_ffi_name(&self) -> Ident {
        format_ident!("p__{}__ffi", AsSnakeCase(self.0.to_string()).to_string())
    }

    /// Get the WASM import name
    pub fn fn_wasm_name(&self) -> Ident {
        format_ident!("p__{}__wasm", AsSnakeCase(self.0.to_string()).to_string())
    }

    /// Get the input struct name (if multiple parameters)
    pub fn input_struct_name(&self) -> Ident {
        format_ident!(
            "p__{}__Input",
            AsUpperCamelCase(self.0.to_string()).to_string()
        )
    }
}

impl Deref for FnName {
    type Target = Ident;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputParams {
    None,
    One(Ident, Type),
    Many(Vec<(Ident, Type)>),
}

pub enum InputParamsIter<'a> {
    None,
    One(Option<(&'a Ident, &'a Type)>),
    Many(Iter<'a, (Ident, Type)>),
}

impl<'a> Iterator for InputParamsIter<'a> {
    type Item = (&'a Ident, &'a Type);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            InputParamsIter::None => None,
            InputParamsIter::One(t) => t.take(),
            InputParamsIter::Many(params) => params.next().map(|(i, t)| (i, t)),
        }
    }
}

impl InputParams {
    pub fn is_empty(&self) -> bool {
        match self {
            InputParams::None => true,
            InputParams::One(_, _) => false,
            InputParams::Many(_) => false,
        }
    }

    pub fn iter(&self) -> InputParamsIter<'_> {
        match self {
            InputParams::None => InputParamsIter::None,
            InputParams::One(i, t) => InputParamsIter::One(Some((i, t))),
            InputParams::Many(params) => InputParamsIter::Many(params.iter()),
        }
    }

    pub fn input_type(&self, fn_name: &FnName, class: Option<&CapabilityIdent>) -> TokenStream {
        match &self {
            InputParams::Many(_) => {
                let input_struct_name = class
                    .map(|c| c.input_struct(fn_name))
                    .unwrap_or(fn_name.input_struct_name());
                quote!(#input_struct_name)
            }
            InputParams::One(_, param_ty) => quote!(#param_ty),
            InputParams::None => quote!(()),
        }
    }

    pub fn input_serialization(
        &self,
        fn_name: &FnName,
        class: Option<&CapabilityIdent>,
    ) -> TokenStream {
        match &self {
            InputParams::Many(params) => {
                let input_struct_name = class
                    .map(|c| c.input_struct(fn_name))
                    .unwrap_or(fn_name.input_struct_name());
                let args = params.iter().map(|(n, _)| quote!(#n));
                quote!(Some(&#input_struct_name { #(#args),* }))
            }
            InputParams::One(param_name, _) => quote!(Some(&#param_name)),
            InputParams::None => quote!(None),
        }
    }

    pub fn input_args(&self) -> Vec<TokenStream> {
        match &self {
            InputParams::Many(params) => params.iter().map(|(n, _)| quote!(input.#n)).collect(),
            InputParams::One(..) => vec![quote!(input)],
            InputParams::None => Vec::new(),
        }
    }

    pub fn input_struct(&self, fn_name: &FnName, class: Option<&CapabilityIdent>) -> TokenStream {
        match &self {
            InputParams::Many(params) => {
                let input_struct_name = class
                    .map(|c| c.input_struct(fn_name))
                    .unwrap_or(fn_name.input_struct_name());
                let fields: Vec<_> = params.iter().map(|(n, t)| quote! { pub #n: #t }).collect();
                quote! {
                    #[::pyroduct::magma]
                    struct #input_struct_name {
                        #(#fields),*
                    }
                }
            }
            InputParams::One(_, _) => quote! {},
            InputParams::None => quote! {},
        }
    }
}

#[derive(Debug, Clone)]
pub enum FnOutput {
    None,
    Single(Type),
    Result(Type, Type),
}

impl FnOutput {
    pub fn parse(ret: &ReturnType, expected_err: Option<&Type>) -> syn::Result<FnOutput> {
        let mut output = FnOutput::None;
        match ret {
            // Handle "-> " (Default)
            ReturnType::Default => {}

            ReturnType::Type(_, ty) => {
                let ty = ty.as_ref();
                output = FnOutput::Single(ty.clone());
                match ty {
                    Type::Tuple(tuple) if tuple.elems.is_empty() => output = FnOutput::None,
                    Type::Path(type_path) => {
                        // Check if the last segment is "Result" (heuristic)
                        if let Some(segment) = type_path.path.segments.last()
                            && segment.ident == "Result"
                            && let PathArguments::AngleBracketed(args) = &segment.arguments
                        {
                            // Ensure we have exactly 2 generic arguments: <T, E>
                            if args.args.len() == 2 {
                                let mut iter = args.args.iter();
                                // Ensure both arguments are Types (not lifetimes or consts)
                                if let (
                                    Some(GenericArgument::Type(t)),
                                    Some(GenericArgument::Type(e)),
                                ) = (iter.next(), iter.next())
                                {
                                    output = FnOutput::Result(t.clone(), e.clone());
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        match (output, expected_err) {
            (a @ FnOutput::None, None)
            | (a @ FnOutput::Single(_), None)
            | (a @ FnOutput::Result(_, _), None) => Ok(a),
            (FnOutput::None, Some(target_error)) | (FnOutput::Single(_), Some(target_error)) => {
                let target_err_str = quote!(#target_error).to_string().replace(" ", "");
                Err(Error::new_spanned(
                    ret,
                    format!(
                        "Expected a result with '{}' or 'Self::Error' error type",
                        target_err_str
                    ),
                ))
            }
            (FnOutput::Result(val, err_type), Some(target_error)) => {
                let self_err_str: Type = parse_quote!(Self::Error);
                if &err_type != target_error && err_type != self_err_str {
                    let actual_err_str = quote!(#err_type).to_string().replace(" ", "");
                    let target_err_str = quote!(#target_error).to_string().replace(" ", "");
                    Err(Error::new_spanned(
                        err_type,
                        format!(
                            "Invalid error type. Expected '{}' or 'Self::Error', found '{}'",
                            target_err_str, actual_err_str
                        ),
                    ))
                } else {
                    Ok(FnOutput::Result(val, err_type))
                }
            }
        }
    }

    pub fn to_return_type(&self) -> ReturnType {
        match self {
            // Maps back to no return arrow (void)
            FnOutput::None => ReturnType::Default,

            // Maps back to "-> T"
            FnOutput::Single(ty) => ReturnType::Type(RArrow::default(), Box::new(ty.clone())),

            // Maps back to "-> Result<T, E>"
            FnOutput::Result(ok, err) => {
                let result_ty: Type = parse_quote!(Result<#ok, #err>);
                ReturnType::Type(RArrow::default(), Box::new(result_ty))
            }
        }
    }

    pub fn ty(&self) -> Type {
        match self {
            // Maps back to no return arrow (void)
            FnOutput::None => parse_quote!(()),

            // Maps back to "-> T"
            FnOutput::Single(ty) => ty.clone(),

            // Maps back to "-> Result<T, E>"
            FnOutput::Result(ok, _) => ok.clone(),
        }
    }

    pub fn err(&self) -> Option<&Type> {
        match self {
            // Maps back to no return arrow (void)
            FnOutput::None => None,

            // Maps back to "-> T"
            FnOutput::Single(_) => None,

            // Maps back to "-> Result<T, E>"
            FnOutput::Result(_, err) => Some(err),
        }
    }
}
