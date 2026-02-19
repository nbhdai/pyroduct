use std::rc::Rc;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Error, FnArg, GenericArgument, Ident, ImplItemFn, PathArguments, ReturnType, Type};

use crate::{
    ffi::paths::{CapabilityIdent, FnName},
    utils::extract_ident_from_type,
};

#[derive(Debug, Clone)]
pub struct NewClientFn {
    pub fn_name: FnName,
    pub class: Rc<CapabilityIdent>,
    pub client_type: Ident,
    pub error_type: Option<Type>,
    pub body: syn::Block,
    pub attrs: Vec<syn::Attribute>,
    pub is_async: bool,
}

impl NewClientFn {
    pub fn parse(f: &ImplItemFn, class: &Rc<CapabilityIdent>) -> syn::Result<Self> {
        let sig = &f.sig;

        // 1. Validate name
        if sig.ident != "register" {
            return Err(Error::new_spanned(
                &sig.ident,
                "Expected function named 'register'",
            ));
        }

        // 2. Validate not async
        let is_async = sig.asyncness.is_some();

        // 3. Validate &self as first parameter
        match sig.inputs.first() {
            Some(FnArg::Receiver(r)) => {
                if r.mutability.is_some() {
                    return Err(Error::new_spanned(
                        r,
                        "fn register must take &self (not &mut self)",
                    ));
                }
                if r.reference.is_none() {
                    return Err(Error::new_spanned(
                        r,
                        "fn register must take &self (not self)",
                    ));
                }
            }
            Some(arg) => {
                return Err(Error::new_spanned(
                    arg,
                    "fn register must take &self as its first parameter",
                ));
            }
            None => {
                return Err(Error::new_spanned(&sig, "fn register must take &self"));
            }
        }

        // 4. Validate second parameter is client: &ClientType
        if sig.inputs.len() != 2 {
            return Err(Error::new_spanned(
                &sig.inputs,
                "fn register must take exactly two parameters: &self and client: &ClientType",
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
            fn_name: FnName(format_ident!("register")),
            class: class.clone(),
            client_type,
            error_type,
            body: f.block.clone(),
            attrs: f.attrs.clone(),
            is_async,
        })
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

    /// Generate the export entry for the init function
    pub fn generate_export(&self) -> TokenStream {
        let init_name = self.class.ffi_name(&self.fn_name);

        if self.is_async {
            quote!(::pyroduct::ffi::ClientRegisterFn::Async(#init_name))
        } else {
            quote!(::pyroduct::ffi::ClientRegisterFn::Sync(#init_name))
        }
    }

    pub fn generate_capability_ffi(&self) -> TokenStream {
        let fn_ffi_name = self.class.ffi_name(&self.fn_name);
        let client_type = &self.client_type;
        let state_type = &self.class.state_tn;

        // Helper to deserialize the client and call the method
        let body = if let Some(_) = &self.error_type {
            quote! {
                let client: #client_type = match ::pyroduct::ffi::guest::panic_wrap::deserialize_input(client_state_ptr, std::panic::Location::caller()) {
                    Ok(v) => v,
                    Err(e) => return e.encode().into_raw(),
                };

                ::pyroduct::ffi::guest::panic_wrap::execute_safe_result(|| {
                    // Reconstruct state from raw pointer
                    let state = unsafe { &*(capability_state_ptr.state as *const #state_type) };
                    state.new_client(&client)
                })
            }
        } else {
            quote! {
                let client: #client_type = match ::pyroduct::ffi::guest::panic_wrap::deserialize_input(client_state_ptr, std::panic::Location::caller()) {
                    Ok(v) => v,
                    Err(e) => return e.encode().into_raw(),
                };

                ::pyroduct::ffi::guest::panic_wrap::execute_safe(|| {
                    let state = unsafe { &*(capability_state_ptr.state as *const #state_type) };
                    state.new_client(&client)
                })
            }
        };

        quote! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn #fn_ffi_name(
                capability_state_ptr: ::pyroduct::ffi::PyroRefObjectPtr,
                client_state_ptr: ::pyroduct::PyroViewPtr,
            ) -> ::pyroduct::PyroVecPtr {
                #body
            }
        }
    }

    /// Generates the extern declaration for the WASM import.
    /// This corresponds to `generate_client_wasm` requested in the prompt.
    pub fn generate_client_wasm(&self) -> TokenStream {
        //let fn_wasm_name = self.class.wasm_name(&self.fn_name);
        quote! {
            pub fn register(ptr: *const u8) -> *mut u8;
        }
    }

    /// Generates the call expression used inside the client's register method.
    /// This corresponds to `generate_wasm_call`.
    pub fn generate_wasm_call(&self, module: Option<&Ident>) -> TokenStream {
        // let fn_wasm_name = self.class.wasm_name(&self.fn_name);
        let module_prefix = if let Some(m) = module {
            quote!(#m::)
        } else {
            quote!()
        };

        quote! {
            #module_prefix register
        }
    }

    /// Generates the full client-side implementation of the register method.
    /// The user prompt referred to this as "generate_client_wasm needs to generate impl MyClient".
    pub fn generate_client_impl(&self, module: Option<&Ident>) -> TokenStream {
        let client_type = &self.client_type;
        let wasm_call = self.generate_wasm_call(module);

        let return_type = if let Some(err) = &self.error_type {
            quote!(Result<::pyroduct::wasm::Client<Self>, #err>)
        } else {
            quote!(::pyroduct::wasm::Client<Self>)
        };

        let result_handling = if let Some(error_ty) = &self.error_type {
            quote! {

                ::pyroduct::wasm::Client::<Self>::__register_result::<#error_ty, _>(self, |ptr| unsafe { #wasm_call(ptr) })
            }
        } else {
            quote! {
                ::pyroduct::wasm::Client::<Self>::__register(self, |ptr| unsafe { #wasm_call(ptr) })
            }
        };

        quote! {
            impl #client_type {
                pub fn register(self) -> #return_type {
                    #result_handling
                }
            }
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
