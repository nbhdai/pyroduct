//! Reset function parsing and validation
//!
//! Valid signatures:
//! - `fn reset(&mut self)`
//! - `async fn reset(&mut self)`

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Error, FnArg, Ident, ImplItemFn, ReturnType};

use heck::AsSnakeCase;

#[derive(Debug, Clone)]
pub struct ResetFn {
    pub is_async: bool,
    pub body: syn::Block,
    pub attrs: Vec<syn::Attribute>,
}

impl ResetFn {
    pub fn parse(f: &ImplItemFn) -> syn::Result<Self> {
        let sig = &f.sig;

        // 1. Validate name
        if sig.ident != "reset" {
            return Err(Error::new_spanned(
                &sig.ident,
                "Expected function named 'reset'",
            ));
        }

        // 2. Validate return type is () or default
        match &sig.output {
            ReturnType::Default => {}
            ReturnType::Type(_, ty) => {
                let ty_str = quote!(#ty).to_string().replace(" ", "");
                if ty_str != "()" {
                    return Err(Error::new_spanned(
                        &sig.output,
                        "fn reset must return () or have no return type",
                    ));
                }
            }
        }

        // 3. Validate &mut self as first and only parameter
        if sig.inputs.len() != 1 {
            return Err(Error::new_spanned(
                &sig.inputs,
                "fn reset must take exactly &mut self",
            ));
        }

        match sig.inputs.first() {
            Some(FnArg::Receiver(r)) => {
                if r.mutability.is_none() {
                    return Err(Error::new_spanned(
                        r,
                        "fn reset must take &mut self (not &self)",
                    ));
                }
                if r.reference.is_none() {
                    return Err(Error::new_spanned(
                        r,
                        "fn reset must take &mut self (not mut self)",
                    ));
                }
            }
            Some(arg) => {
                return Err(Error::new_spanned(
                    arg,
                    "fn reset must take &mut self as its only parameter",
                ));
            }
            None => {
                return Err(Error::new_spanned(&sig, "fn reset must take &mut self"));
            }
        }

        Ok(Self {
            is_async: sig.asyncness.is_some(),
            body: f.block.clone(),
            attrs: f.attrs.clone(),
        })
    }

    /// Generate the FFI reset function
    pub fn generate_ffi(&self, server: &Ident) -> TokenStream {
        let server_snake = AsSnakeCase(server.to_string()).to_string();
        let reset_name = format_ident!("p__{}__ffi_reset", server_snake);

        if self.is_async {
            quote! {
                #[unsafe(no_mangle)]
                pub unsafe extern "C" fn #reset_name<'a>(
                    state: ::pyroduct::ffi::PyroRefObjectPtr,
                ) -> ::pyroduct::ffi::FuturePyroVec<'a> {
                    ::pyroduct::ffi::guest::safe_lifecycle::execute_safe_async_reset::<#server, _, _>(
                        state,
                        |state| async move { state.reset().await },
                    )
                }
            }
        } else {
            quote! {
                #[unsafe(no_mangle)]
                pub unsafe extern "C" fn #reset_name(
                    state: ::pyroduct::ffi::PyroRefObjectPtr,
                ) -> ::pyroduct::PyroVecPtr {
                    ::pyroduct::ffi::guest::safe_lifecycle::execute_safe_reset::<#server, _>(
                        state,
                        |state| state.reset(),
                    )
                }
            }
        }
    }

    /// Generate the export entry for the reset function
    pub fn generate_export(&self, server: &Ident) -> TokenStream {
        let server_snake = AsSnakeCase(server.to_string()).to_string();
        let reset_name = format_ident!("p__{}__ffi_reset", server_snake);

        if self.is_async {
            quote!(::pyroduct::ffi::ClassResetFn::Async(#reset_name))
        } else {
            quote!(::pyroduct::ffi::ClassResetFn::Sync(#reset_name))
        }
    }

    /// Generate the impl method (preserves original)
    pub fn generate_impl_method(&self) -> TokenStream {
        let attrs = &self.attrs;
        let body = &self.body;
        let async_kw = if self.is_async {
            quote!(async)
        } else {
            quote!()
        };

        quote! {
            #(#attrs)*
            pub #async_kw fn reset(&mut self) #body
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::{format_ident, quote};
    use syn::{ImplItemFn, parse_quote};

    #[test]
    fn test_sync_server_reset_fn() {
        let server_ident = format_ident!("GreeterServer");
        let item: ImplItemFn = parse_quote! {
            fn reset(&mut self) {
                self.count = 0;
            }
        };
        let reset_fn = ResetFn::parse(&item).expect("Failed to parse reset fn");
        let result = reset_fn.generate_ffi(&server_ident);
        let expected = quote! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn p__greeter_server__ffi_reset(
                state: ::pyroduct::ffi::PyroRefObjectPtr,
            ) -> ::pyroduct::PyroVecPtr {
                ::pyroduct::ffi::guest::safe_lifecycle::execute_safe_reset::<GreeterServer, _>(
                    state,
                    |state| state.reset(),
                )
            }
        };

        crate::fmt::assert_code_eq_token(&result, &expected);
    }

    #[test]
    fn test_async_server_reset_fn() {
        let server_ident = format_ident!("GreeterServer");
        let item: ImplItemFn = parse_quote! {
            async fn reset(&mut self) {
                self.count = 0;
            }
        };

        let reset_fn = ResetFn::parse(&item).expect("Failed to parse reset fn");
        let result = reset_fn.generate_ffi(&server_ident);
        let expected = quote! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn p__greeter_server__ffi_reset<'a>(
                state: ::pyroduct::ffi::PyroRefObjectPtr,
            ) -> ::pyroduct::ffi::FuturePyroVec<'a> {
                ::pyroduct::ffi::guest::safe_lifecycle::execute_safe_async_reset::<GreeterServer, _, _>(
                    state,
                    |state| async move { state.reset().await },
                )
            }
        };

        crate::fmt::assert_code_eq_token(&result, &expected);
    }
}
