//! NewClient function parsing and validation
//!
//! Valid signatures:
//! - `fn new_client(&self, client: &ClientType)`
//! - `fn new_client(&self, client: &ClientType) -> Result<(), ErrorType>`

use std::rc::Rc;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Error, FnArg, GenericArgument, Ident, ImplItemFn, PathArguments, ReturnType, Type};

use crate::ffi::CapabilityFuncFFI;
use crate::paths::CapabilityIdent;
use crate::utils::{extract_ident_from_type, type_to_return};

#[derive(Debug, Clone)]
pub struct NewClientFn {
    pub client_type: Ident,
    pub error_type: Option<Type>,
    pub body: syn::Block,
    pub attrs: Vec<syn::Attribute>,
}

impl NewClientFn {
    pub fn parse(f: &ImplItemFn) -> syn::Result<Self> {
        let sig = &f.sig;

        // 1. Validate name
        if sig.ident != "new_client" {
            return Err(Error::new_spanned(&sig.ident, "Expected function named 'new_client'"));
        }

        // 2. Validate not async
        if sig.asyncness.is_some() {
            return Err(Error::new_spanned(
                &sig,
                "fn new_client cannot be async",
            ));
        }

        // 3. Validate &self as first parameter
        match sig.inputs.first() {
            Some(FnArg::Receiver(r)) => {
                if r.mutability.is_some() {
                    return Err(Error::new_spanned(
                        r,
                        "fn new_client must take &self (not &mut self)",
                    ));
                }
                if r.reference.is_none() {
                    return Err(Error::new_spanned(
                        r,
                        "fn new_client must take &self (not self)",
                    ));
                }
            }
            Some(arg) => {
                return Err(Error::new_spanned(
                    arg,
                    "fn new_client must take &self as its first parameter",
                ));
            }
            None => {
                return Err(Error::new_spanned(
                    &sig,
                    "fn new_client must take &self",
                ));
            }
        }

        // 4. Validate second parameter is client: &ClientType
        if sig.inputs.len() != 2 {
            return Err(Error::new_spanned(
                &sig.inputs,
                "fn new_client must take exactly two parameters: &self and client: &ClientType",
            ));
        }

        let client_type = match sig.inputs.iter().nth(1) {
            Some(FnArg::Typed(pt)) => {
                // Extract the type (should be a reference)
                let ty = &*pt.ty;
                if let Type::Reference(r) = ty {
                    extract_ident_from_type(&r.elem)?
                } else {
                    return Err(Error::new_spanned(
                        ty,
                        "Client parameter must be a reference: &ClientType",
                    ));
                }
            }
            _ => {
                return Err(Error::new_spanned(
                    &sig.inputs,
                    "fn new_client must have client: &ClientType as second parameter",
                ));
            }
        };

        // 5. Validate return type: () or Result<(), ErrorType>
        let error_type = match &sig.output {
            ReturnType::Default => None,
            ReturnType::Type(_, ty) => {
                let ty_str = quote!(#ty).to_string().replace(" ", "");
                if ty_str == "()" {
                    None
                } else {
                    // Must be Result<(), ErrorType>
                    extract_result_error_type(ty)?
                }
            }
        };

        Ok(Self {
            client_type,
            error_type,
            body: f.block.clone(),
            attrs: f.attrs.clone(),
        })
    }

    /// Build the FFI metadata for new_client
    pub fn build_ffi(&self, class: &Rc<CapabilityIdent>) -> CapabilityFuncFFI {
        let return_type: ReturnType = if let Some(err) = &self.error_type {
            type_to_return(&syn::parse_quote!(Result<(), #err>))
        } else {
            type_to_return(&syn::parse_quote!(()))
        };

        CapabilityFuncFFI {
            class: Some(class.clone()),
            constructor: true,
            fn_name: format_ident!("new_client"),
            vis: syn::parse_quote!(pub),
            is_async: false,
            return_type,
            input: None,
        }
    }

    /// Generate the impl method (preserves original)
    pub fn generate_impl_method(&self) -> TokenStream {
        let attrs = &self.attrs;
        let body = &self.body;
        let client = &self.client_type;

        let return_type = if let Some(err) = &self.error_type {
            quote!(-> Result<(), #err>)
        } else {
            quote!()
        };

        quote! {
            #(#attrs)*
            pub fn new_client(&self, client: &#client) #return_type #body
        }
    }
}

/// Extract the error type from Result<(), ErrorType>
fn extract_result_error_type(ty: &Type) -> syn::Result<Option<Type>> {
    if let Type::Path(tp) = ty {
        if let Some(segment) = tp.path.segments.last() {
            if segment.ident == "Result" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    if args.args.len() == 2 {
                        // Validate first arg is ()
                        if let GenericArgument::Type(ok_ty) = &args.args[0] {
                            let ok_str = quote!(#ok_ty).to_string().replace(" ", "");
                            if ok_str != "()" {
                                return Err(Error::new_spanned(
                                    ok_ty,
                                    "new_client must return Result<(), Error>, not Result<T, Error>",
                                ));
                            }
                        }

                        // Extract error type
                        if let GenericArgument::Type(err_ty) = &args.args[1] {
                            return Ok(Some(err_ty.clone()));
                        }
                    }
                }
            }
        }
    }

    Err(Error::new_spanned(
        ty,
        "Return type must be () or Result<(), ErrorType>",
    ))
}

