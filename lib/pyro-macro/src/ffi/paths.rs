//! Path and naming utilities for capability FFI generation
//!
//! This module centralizes all naming conventions used throughout the capability system
//! to ensure consistency between client and server sides.

use std::ops::Deref;

use heck::{AsSnakeCase, AsUpperCamelCase};
use pyroformat_macro::parse::StructInput;
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

    /// The input struct's name, e.g. `__PyroFormatUpdateInput`
    pub fn input_struct_name(&self) -> Ident {
        format_ident!(
            "__PyroFormat{}Input",
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

/// `__PyroFormat{Fn}Input`: a method's parameters after `self` and the
/// client, as the struct that carries them over the wire.
pub fn input_struct(fn_name: &FnName, params: &[(Ident, Type)]) -> syn::Result<StructInput> {
    StructInput::from_params(fn_name.input_struct_name(), quote!(::pyroduct), params)
}

pub fn is_captured_error(ty: &Type) -> bool {
    let ty_str = quote!(#ty).to_string().replace(" ", "");
    ty_str == "CapturedError"
        || ty_str == "pyroduct::CapturedError"
        || ty_str == "::pyroduct::CapturedError"
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
                        if let (Some(GenericArgument::Type(t)), Some(GenericArgument::Type(e))) =
                            (iter.next(), iter.next())
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
