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
    pub fn class_name(&self) -> String {
        AsSnakeCase(self.state_tn.to_string()).to_string()
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
    One(Ident, Box<Type>),
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

pub fn is_captured_error(ty: &Type) -> bool {
    let ty_str = quote!(#ty).to_string().replace(" ", "");
    ty_str == "CapturedError" || ty_str == "pyroduct::CapturedError" || ty_str == "::pyroduct::CapturedError"
}

pub fn verify_result_return_type(ret: &ReturnType) -> syn::Result<(Type, Type)> {
    match ret {
        ReturnType::Type(_, ty) => {
            let ty = ty.as_ref();
            if let Type::Path(type_path) = ty {
                if let Some(segment) = type_path.path.segments.last()
                    && segment.ident == "Result"
                    && let PathArguments::AngleBracketed(args) = &segment.arguments
                {
                    if args.args.len() == 2 {
                        let mut iter = args.args.iter();
                        if let (
                            Some(GenericArgument::Type(t)),
                            Some(GenericArgument::Type(e)),
                        ) = (iter.next(), iter.next())
                        {
                            if !is_captured_error(e) {
                                let actual_err_str = quote!(#e).to_string().replace(" ", "");
                                return Err(Error::new_spanned(
                                    e,
                                    format!(
                                        "Invalid error type. Expected 'CapturedError', found '{}'",
                                        actual_err_str
                                    ),
                                ));
                            }
                            let err_ty: Type = parse_quote!(::pyroduct::CapturedError);
                            return Ok((t.clone(), err_ty));
                        }
                    } else if args.args.len() == 1 {
                        let mut iter = args.args.iter();
                        if let Some(GenericArgument::Type(t)) = iter.next() {
                            let err_ty: Type = parse_quote!(::pyroduct::CapturedError);
                            return Ok((t.clone(), err_ty));
                        }
                    }
                }
            }
        }
        ReturnType::Default => {}
    }

    Err(Error::new_spanned(
        ret,
        "Function must return Result<T, CapturedError> or Result<T>",
    ))
}

#[derive(Debug, Clone)]
pub struct FnOutput {
    pub ok_type: Type,
    pub err_type: Type,
}

impl FnOutput {
    pub fn parse(ret: &ReturnType) -> syn::Result<FnOutput> {
        let (ok_type, err_type) = verify_result_return_type(ret)?;
        Ok(FnOutput { ok_type, err_type })
    }

    pub fn to_return_type(&self) -> ReturnType {
        let ok = &self.ok_type;
        let err = &self.err_type;
        let result_ty: Type = parse_quote!(Result<#ok, #err>);
        ReturnType::Type(RArrow::default(), Box::new(result_ty))
    }

    pub fn ty(&self) -> Box<Type> {
        Box::new(self.ok_type.clone())
    }

    pub fn err(&self) -> Option<&Type> {
        Some(&self.err_type)
    }
}
