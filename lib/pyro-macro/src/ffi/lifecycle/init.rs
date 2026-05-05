//! Init function parsing and validation

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Error, FnArg, GenericArgument, Ident, ImplItemFn, Pat, PathArguments, ReturnType, Type};

use heck::AsSnakeCase;

#[derive(Debug, Clone)]
pub struct InitFn {
    pub is_async: bool,
    pub config_type: Option<Type>,
    pub body: syn::Block,
    pub attrs: Vec<syn::Attribute>,
    pub arg_name: Option<Ident>,
}

impl InitFn {
    /// Parse the init function and validate it against the expected configuration.
    pub fn parse(expected_config: Option<Type>, f: &ImplItemFn) -> syn::Result<Self> {
        let sig = &f.sig;

        // 1. Validate name
        if sig.ident != "new" {
            return Err(Error::new_spanned(
                &sig.ident,
                "Expected function named 'new'",
            ));
        }

        // 2. Validate return type is Self
        match &sig.output {
            ReturnType::Type(_, ty) => {
                let ty_str = quote!(#ty).to_string().replace(" ", "");
                if ty_str != "Self" {
                    return Err(Error::new_spanned(&sig.output, "fn new must return Self"));
                }
            }
            ReturnType::Default => {
                return Err(Error::new_spanned(&sig, "fn new must return Self"));
            }
        }

        // 3. Validate no &self receiver
        if let Some(FnArg::Receiver(r)) = sig.inputs.first() {
            return Err(Error::new_spanned(
                r,
                "fn new must be a static function (no self parameter)",
            ));
        }

        let mut user_arg_name = None;

        // 4. Validate Argument consistency against Expected Config
        match &expected_config {
            // Case A: Attribute said `config = MyType`
            Some(expected_ty) => {
                if sig.inputs.len() != 1 {
                    return Err(Error::new_spanned(
                        &sig.inputs,
                        format!(
                            "Macro attribute defined 'config = {}', so fn new must take exactly one argument: 'arg: Option<{}>'",
                            quote!(#expected_ty),
                            quote!(#expected_ty)
                        ),
                    ));
                }

                let arg = sig.inputs.first().unwrap();
                if let FnArg::Typed(pt) = arg {
                    // Capture argument name (don't validate it)
                    if let Pat::Ident(pi) = &*pt.pat {
                        user_arg_name = Some(pi.ident.clone());
                    } else {
                        return Err(Error::new_spanned(
                            &pt.pat,
                            "Expected simple identifier for argument",
                        ));
                    }

                    // Check type is Option<T>
                    let valid_option = if let Type::Path(tp) = &*pt.ty {
                        if let Some(segment) = tp.path.segments.last() {
                            if segment.ident == "Option" {
                                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                                    if let Some(GenericArgument::Type(inner_ty)) = args.args.first()
                                    {
                                        // Compare inner type with expected type
                                        let inner_str =
                                            quote!(#inner_ty).to_string().replace(" ", "");
                                        let expected_str =
                                            quote!(#expected_ty).to_string().replace(" ", "");

                                        if inner_str == expected_str {
                                            Some(())
                                        } else {
                                            return Err(Error::new_spanned(
                                                &pt.ty,
                                                format!(
                                                    "Type mismatch. Expected 'Option<{}>' based on macro attribute, found 'Option<{}>'",
                                                    expected_str, inner_str
                                                ),
                                            ));
                                        }
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    if valid_option.is_none() {
                        return Err(Error::new_spanned(
                            &pt.ty,
                            format!(
                                "Config parameter must be 'Option<{}>'",
                                quote!(#expected_ty)
                            ),
                        ));
                    }
                }
            }
            // Case B: No config attribute
            None => {
                if !sig.inputs.is_empty() {
                    return Err(Error::new_spanned(
                        &sig.inputs,
                        "No 'config' attribute specified in macro, so fn new() must take 0 arguments.",
                    ));
                }
            }
        }

        Ok(Self {
            is_async: sig.asyncness.is_some(),
            config_type: expected_config,
            body: f.block.clone(),
            attrs: f.attrs.clone(),
            arg_name: user_arg_name,
        })
    }

    /// Generate the FFI init function
    pub fn generate_ffi(&self, server: &Ident) -> TokenStream {
        let server_snake = AsSnakeCase(server.to_string()).to_string();
        let init_name = format_ident!("p__{}__ffi_init", server_snake);

        // Determine config type and closure body
        // The safe_lifecycle functions expect Option<T> to be passed through
        let (return_ty, closure) = match (&self.config_type, self.is_async) {
            (Some(c), false) => (
                quote!(::pyroduct::ffi::InitResult),
                quote! {::pyroduct::ffi::guest::safe_lifecycle::execute_safe_init(|object_id| {
                    let config = match ::pyroduct::ffi::guest::safe_lifecycle::deserialize_config::<#c>(config_ptr) {
                        Ok(config) => config,
                        Err(err) => return ::pyroduct::ffi::InitResult::init_err(err, object_id),
                    };
                    ::pyroduct::ffi::InitResult::init_ok(#server::new(config), object_id)
                }, object_id)},
            ),
            (None, false) => (
                quote!(::pyroduct::ffi::InitResult),
                quote! {::pyroduct::ffi::guest::safe_lifecycle::execute_safe_init(|object_id| {
                    ::pyroduct::ffi::InitResult::init_ok(#server::new(), object_id)
                }, object_id)},
            ),
            (Some(c), true) => (
                quote!(::pyroduct::ffi::FutureInitResult),
                quote! { ::pyroduct::ffi::guest::safe_lifecycle::execute_safe_async_init(|object_id| async move {
                    let config = match ::pyroduct::ffi::guest::safe_lifecycle::deserialize_config::<#c>(config_ptr) {
                        Ok(config) => config,
                        Err(err) => return ::pyroduct::ffi::InitResult::init_err(err, object_id),
                    };
                    ::pyroduct::ffi::InitResult::init_ok(#server::new(config).await, object_id)
                }, object_id)},
            ),
            (None, true) => (
                quote!(::pyroduct::ffi::FutureInitResult),
                quote! { ::pyroduct::ffi::guest::safe_lifecycle::execute_safe_async_init(|object_id| async move {
                    ::pyroduct::ffi::InitResult::init_ok(#server::new().await, object_id)
                }, object_id)},
            ),
        };

        quote! {
            #[unsafe(no_mangle)]
            pub extern "C" fn #init_name(
                config_ptr: ::pyroduct::format::PyroRefPtr,
                object_id: u64,
            ) -> #return_ty {
                #closure
            }
        }
    }

    /// Generate the export entry for the init function
    pub fn generate_export(&self, server: &Ident) -> TokenStream {
        let server_snake = AsSnakeCase(server.to_string()).to_string();
        let init_name = format_ident!("p__{}__ffi_init", server_snake);

        if self.is_async {
            quote!(::pyroduct::ffi::ClassInitFn::Async(#init_name))
        } else {
            quote!(::pyroduct::ffi::ClassInitFn::Sync(#init_name))
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

        let params = if let Some(config) = &self.config_type {
            // Use the user's variable name, fallback to 'config' if something went weird
            let name = self.arg_name.clone().unwrap_or(format_ident!("config"));
            quote!(#name: Option<#config>)
        } else {
            quote!()
        };

        quote! {
            #(#attrs)*
            pub #async_kw fn new(#params) -> Self #body
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::fmt::assert_code_eq_token;

    use super::*;
    use quote::{format_ident, quote};
    use syn::parse_quote;

    #[test]
    fn test_sync_server_init_fn() {
        // 1. Simulate the config attribute passed from the macro
        let config_type: Type = parse_quote!(GreeterConfig);

        // 2. Simulate the user's implementation (Using Option, and variable name 'cfg')
        let item: ImplItemFn = parse_quote! {
            fn new(cfg: Option<GreeterConfig>) -> Self {
                Self { count: 0 }
            }
        };

        // 3. Parse with validation
        let init_fn = InitFn::parse(Some(config_type), &item).expect("Parse failed");

        let server_ident = format_ident!("GreeterServer");
        let result = init_fn.generate_ffi(&server_ident);

        // Note: Closure now calls new(config) directly (config is already Option<T>)
        let expected = quote! {
            #[unsafe(no_mangle)]
            pub extern "C" fn p__greeter_server__ffi_init(
                config_ptr: ::pyroduct::format::PyroRefPtr,
                object_id: u64,
            ) -> ::pyroduct::ffi::InitResult {
                ::pyroduct::ffi::guest::safe_lifecycle::execute_safe_init(|object_id| {
                    let config = match ::pyroduct::ffi::guest::safe_lifecycle::deserialize_config::<GreeterConfig>(config_ptr) {
                        Ok(config) => config,
                        Err(err) => return ::pyroduct::ffi::InitResult::init_err(err, object_id),
                    };
                    ::pyroduct::ffi::InitResult::init_ok(GreeterServer::new(config), object_id)
                }, object_id)
            }
        };

        assert_code_eq_token(&result, &expected);
    }

    #[test]
    fn test_async_server_init_fn() {
        // 1. Config attribute
        let config_type: Type = parse_quote!(GreeterConfig);

        // 2. User implementation
        let item: ImplItemFn = parse_quote! {
            async fn new(val: Option<GreeterConfig>) -> Self {
                Self { count: 0 }
            }
        };

        // 3. Parse
        let init_fn = InitFn::parse(Some(config_type), &item).expect("Parse failed");

        let server_ident = format_ident!("GreeterServer");
        let result = init_fn.generate_ffi(&server_ident);

        let expected = quote! {
            #[unsafe(no_mangle)]
            pub extern "C" fn p__greeter_server__ffi_init(
                config_ptr: ::pyroduct::format::PyroRefPtr,
                object_id: u64,
            ) -> ::pyroduct::ffi::FutureInitResult {
                ::pyroduct::ffi::guest::safe_lifecycle::execute_safe_async_init(|object_id| async move {
                    let config = match ::pyroduct::ffi::guest::safe_lifecycle::deserialize_config::<GreeterConfig>(config_ptr) {
                        Ok(config) => config,
                        Err(err) => return ::pyroduct::ffi::InitResult::init_err(err, object_id),
                    };
                    ::pyroduct::ffi::InitResult::init_ok(GreeterServer::new(config).await, object_id)
                }, object_id)
            }
        };

        assert_code_eq_token(&result, &expected);
    }

    #[test]
    fn test_arbitrary_arg_name() {
        let config_type: Type = parse_quote!(MyConfig);
        // User uses 'settings' instead of 'config'
        let item: ImplItemFn = parse_quote! {
            fn new(settings: Option<MyConfig>) -> Self { Self }
        };

        let init_fn =
            InitFn::parse(Some(config_type), &item).expect("Should allow arbitrary names");

        // Check if generate_impl_method preserves the name 'settings'
        let impl_code = init_fn.generate_impl_method();
        let impl_str = impl_code.to_string();
        assert!(impl_str.contains("settings : Option < MyConfig >"));
    }

    #[test]
    fn test_validation_errors() {
        let config_type: Type = parse_quote!(MyConfig);

        // Case: Not Option<T>
        let item: ImplItemFn = parse_quote! { fn new(c: MyConfig) -> Self { Self } };
        assert!(InitFn::parse(Some(config_type.clone()), &item).is_err());

        // Case: Not Option<T> (Reference)
        let item: ImplItemFn = parse_quote! { fn new(c: &MyConfig) -> Self { Self } };
        assert!(InitFn::parse(Some(config_type.clone()), &item).is_err());

        // Case: Option<WrongType>
        let item: ImplItemFn = parse_quote! { fn new(c: Option<WrongConfig>) -> Self { Self } };
        assert!(InitFn::parse(Some(config_type.clone()), &item).is_err());
    }
}
