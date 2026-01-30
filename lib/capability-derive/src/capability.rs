//! #[capability] on impl blocks - Unified capability definition
//!
//! Transforms:
//! ```ignore
//! #[pyroduct::capability]
//! impl StatefulServer {
//!     type Config = MyConfig;
//!     type Client = SimpleClient;
//!     type Error = MyError; // Optional
//!
//!     fn new(config: &MyConfig) -> Self { Self }
//!     fn reset(&mut self) {}
//!     fn new_client(&self, _client: &SimpleClient) {}
//!     fn call(&self, _client: &SimpleClient) -> f32 { 42.0 }
//! }
//! ```

use std::rc::Rc;

use heck::AsSnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Error, Ident, ImplItem, ItemImpl, Type};

use crate::methods::ImplMethod;
use crate::lifecycle::{InitFn, NewClientFn, ResetFn};
use crate::paths::CapabilityIdent;
use crate::utils::extract_ident_from_type;

/// Parsed capability from an impl block
#[derive(Debug)]
pub struct CapabilityImpl {    
    // Identity storage
    pub ident: Rc<CapabilityIdent>,

    // Lifecycle
    pub init_fn: InitFn,
    pub reset_fn: ResetFn,
    pub new_client_fn: NewClientFn,

    // Methods
    pub methods: Vec<ImplMethod>,

    // Other items (consts, etc.)
    pub other_items: Vec<ImplItem>,
}

impl CapabilityImpl {
    pub fn new(input: ItemImpl) -> syn::Result<Self> {
        // 1. Extract state/server type name
        let state_tn = match &*input.self_ty {
            Type::Path(tp) => tp
                .path
                .get_ident()
                .cloned()
                .ok_or_else(|| Error::new_spanned(&input.self_ty, "Expected simple type name"))?,
            _ => {
                return Err(Error::new_spanned(
                    &input.self_ty,
                    "Expected simple type name",
                ))
            }
        };

        // 2. Ensure no trait impl
        if input.trait_.is_some() {
            return Err(Error::new_spanned(
                &input,
                "#[capability] cannot be used on trait implementations",
            ));
        }

        // 3. First pass: collect types
        let mut client_tn: Option<Ident> = None;
        let mut config_tn: Option<Type> = None;
        let mut error_tn: Option<Type> = None;
        
        let mut init_fn: Option<InitFn> = None;
        let mut reset_fn: Option<ResetFn> = None;
        let mut new_client_fn: Option<NewClientFn> = None;
        let mut method_fns = Vec::new();
        let mut other_items = Vec::new();

        for item in &input.items {
            match item {
                ImplItem::Type(ty) => {
                    if ty.ident == "Client" {
                        client_tn = Some(extract_ident_from_type(&ty.ty)?);
                    } else if ty.ident == "Config" {
                        config_tn = Some(ty.ty.clone());
                    } else if ty.ident == "Error" {
                        error_tn = Some(ty.ty.clone());
                    } else {
                        other_items.push(item.clone());
                    }
                }
                _ => {}
            }
        }

        let client_tn = client_tn.ok_or_else(|| {
            Error::new_spanned(&state_tn, "Missing `type Client = ...;`")
        })?;

        // Build identifiers
        let ident = Rc::new(CapabilityIdent {
            state_tn,
            client_tn,
            config_tn,
            error_tn,
        });

        for item in &input.items {
            match item {
                ImplItem::Fn(f) => {
                    let name = f.sig.ident.to_string();
                    match name.as_str() {
                        "new" => {
                            init_fn = Some(InitFn::parse(ident.config_tn.clone(), f)?);
                        }
                        "reset" => {
                            reset_fn = Some(ResetFn::parse(f)?);
                        }
                        "new_client" => {
                            new_client_fn = Some(NewClientFn::parse(f)?);
                        }
                        _ => {
                            // Defer method parsing until we have the class
                            method_fns.push(f.clone());
                        }
                    }
                }
                other => other_items.push(other.clone()),
            }
        }

        let new_client_fn = new_client_fn.ok_or_else(|| {
            Error::new_spanned(&ident.state_tn, "Missing `fn new_client(&self, client: &Client)`")
        })?;
        let init_fn = init_fn.ok_or_else(|| {
            Error::new_spanned(&ident.state_tn, "Missing `fn new() -> Self` or `fn new(config: &Config) -> Self`")
        })?;
        let reset_fn = reset_fn.ok_or_else(|| {
            Error::new_spanned(&ident.state_tn, "Missing `fn reset(&mut self)`")
        })?;

        // 5. Second pass: parse methods with class context
        let methods: Result<Vec<_>, _> = method_fns
            .iter()
            .map(|f| ImplMethod::parse(f, &ident))
            .collect();
        let methods = methods?;

        Ok(Self {
            ident,
            init_fn,
            reset_fn,
            new_client_fn,
            methods,
            other_items,
        })
    }

    /// Generate all output code
    pub fn expand(&self) -> TokenStream {
        let server_impl = self.generate_server_impl();
        let lifecycle_ffi = self.generate_lifecycle_ffi();
        let method_ffis = self.generate_method_ffis();
        let export_table = self.generate_export_table();
        let wasm_imports = self.generate_wasm_imports();
        let client_impl = self.generate_client_impl();

        quote! {
            #server_impl
            #client_impl
            #wasm_imports
            #lifecycle_ffi
            #method_ffis
            #export_table
        }
    }

    fn generate_server_impl(&self) -> TokenStream {
        let server = &self.ident.state_tn;
        let init_method = self.init_fn.generate_impl_method();
        let reset_method = self.reset_fn.generate_impl_method();
        let new_client_method = self.new_client_fn.generate_impl_method();
        let other_items = &self.other_items;

        let methods: Vec<_> = self.methods.iter()
            .map(|m| m.generate_server_method())
            .collect();

        quote! {
            impl #server {
                #init_method
                #reset_method
                #new_client_method
                #(#other_items)*
                #(#methods)*
            }
        }
    }

    fn generate_client_impl(&self) -> TokenStream {
        let client = &self.ident.client_tn;
        let module = format_ident!("wasm");

        // Generate constructor using NewClientFn
        let constructor = self.new_client_fn.generate_client_constructor(
            &module,
            &self.ident,
        );

        // Generate methods
        let methods: Vec<_> = self.methods.iter()
            .map(|m| m.generate_client_method(&module))
            .collect();

        quote! {
            impl #client {
                #constructor
                #(#methods)*
            }
        }
    }

    fn generate_lifecycle_ffi(&self) -> TokenStream {
        let server = &self.ident.state_tn;

        let init_ffi = self.init_fn.generate_ffi(server);
        let reset_ffi = self.reset_fn.generate_ffi(server);
        let drop_ffi = self.generate_drop_ffi();
        let new_client_ffi = self.new_client_fn
            .build_ffi(&self.ident)
            .generate_capability_ffi();

        quote! {
            #init_ffi
            #drop_ffi
            #reset_ffi
            #new_client_ffi
        }
    }

    fn generate_drop_ffi(&self) -> TokenStream {
        let server = &self.ident.state_tn;
        let server_snake = AsSnakeCase(server.to_string()).to_string();
        let drop_name = format_ident!("__{}__ffi_drop", server_snake);

        quote! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn #drop_name(state: *mut std::ffi::c_void) {
                if !state.is_null() {
                    drop(unsafe { Box::from_raw(state as *mut #server) });
                }
            }
        }
    }

    fn generate_method_ffis(&self) -> TokenStream {
        let method_ffis: Vec<_> = self.methods.iter()
            .map(|m| m.build_ffi().generate_capability_ffi())
            .collect();

        quote! {
            #(#method_ffis)*
        }
    }

    fn generate_export_table(&self) -> TokenStream {
        let server = &self.ident.state_tn;
        let server_snake = AsSnakeCase(server.to_string()).to_string();
        let server_upper = server_snake.to_uppercase();

        let class_name_static = format_ident!("__{}", server_upper);
        let class_name_string = format!("__{}", server_snake);

        let drop_name = format_ident!("__{}__ffi_drop", server_snake);

        // Collect all FFIs
        let new_client_ffi = self.new_client_fn.build_ffi(&self.ident);
        let method_ffis: Vec<_> = self.methods.iter()
            .map(|m| m.build_ffi())
            .collect();

        let all_ffis: Vec<_> = std::iter::once(&new_client_ffi)
            .chain(method_ffis.iter())
            .collect();

        let static_strs: Vec<_> = all_ffis.iter().map(|ffi| {
            let trace_name = ffi.trace_name().to_string();
            let static_name = ffi.trace_name_static();
            quote! { const #static_name: &'static str = #trace_name; }
        }).collect();

        let exports: Vec<_> = all_ffis.iter()
            .map(|ffi| ffi.generate_vtable_entry())
            .collect();

        let num_exports = exports.len();
        let exports_array_name = format_ident!("{}__METHODS", class_name_static);

        let init_export = self.init_fn.generate_export(server);
        let reset_export = self.reset_fn.generate_export(server);

        let plugin_exports_name = format_ident!("{}__EXPORT", class_name_static);

        quote! {
            const #class_name_static: &'static str = #class_name_string;
            #(#static_strs)*

            const #exports_array_name: [::pyroduct::capability_host::ffi::FunctionExport; #num_exports] = [
                #(#exports),*
            ];

            const #plugin_exports_name: ::pyroduct::capability_host::ffi::ClassExport =
                ::pyroduct::capability_host::ffi::ClassExport {
                    ptr: #exports_array_name.as_ptr(),
                    init: #init_export,
                    drop: ::pyroduct::capability_host::ffi::ClassDropFn::Sync(#drop_name),
                    reset: #reset_export,
                    len: #exports_array_name.len(),
                };
        }
    }

    fn generate_wasm_imports(&self) -> TokenStream {
        let new_client_decl = self.new_client_fn
            .build_ffi(&self.ident)
            .generate_client_wasm();

        let method_decls: Vec<_> = self.methods.iter()
            .map(|m| m.build_ffi().generate_client_wasm())
            .collect();

        quote! {
            pub mod wasm {
                use super::*;
                #[link(wasm_import_module = "env")]
                unsafe extern "C" {
                    #new_client_decl
                    #(#method_decls)*
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse2;

    #[test]
    fn test_basic_capability_impl() {
        let code = quote! {
            impl StatefulServer {
                type Client = SimpleClient;

                fn new() -> Self { Self }
                fn reset(&mut self) {}
                fn new_client(&self, _client: &SimpleClient) {}
                fn call(&self, _client: &SimpleClient) -> f32 { 42.0 }
            }
        };

        let input: ItemImpl = parse2(code).unwrap();
        let cap = CapabilityImpl::new(input).unwrap();

        assert_eq!(cap.ident.state_tn.to_string(), "StatefulServer");
        assert_eq!(cap.ident.client_tn.to_string(), "SimpleClient");
        assert_eq!(cap.methods.len(), 1);
        assert_eq!(cap.methods[0].name.to_string(), "call");
        assert!(!cap.init_fn.is_async);
        assert!(cap.init_fn.config_type.is_none());
        assert!(cap.ident.config_tn.is_none());
    }

    #[test]
    fn test_with_config() {
        let code = quote! {
            impl StatefulServer {
                type Config = MyConfig;
                type Client = SimpleClient;

                fn new(config: &MyConfig) -> Self { Self }
                fn reset(&mut self) {}
                fn new_client(&self, client: &SimpleClient) {}
            }
        };

        let input: ItemImpl = parse2(code).unwrap();
        let cap = CapabilityImpl::new(input).unwrap();

        assert!(cap.init_fn.config_type.is_some());
        assert!(cap.ident.config_tn.is_some());
        
        let cfg = cap.ident.config_tn.as_ref().unwrap();
        assert_eq!(quote!(#cfg).to_string(), "MyConfig");
    }

    #[test]
    fn test_config_mismatch() {
        let code = quote! {
            impl StatefulServer {
                type Config = MyConfig;
                type Client = SimpleClient;

                fn new(config: &OtherConfig) -> Self { Self }
                fn reset(&mut self) {}
                fn new_client(&self, client: &SimpleClient) {}
            }
        };

        let input: ItemImpl = parse2(code).unwrap();
        let err = CapabilityImpl::new(input).unwrap_err();
        assert!(err.to_string().contains("Config type mismatch"));
    }

    #[test]
    fn test_async_lifecycle() {
        let code = quote! {
            impl StatefulServer {
                type Client = SimpleClient;

                async fn new() -> Self { Self }
                async fn reset(&mut self) {}
                fn new_client(&self, client: &SimpleClient) {}
            }
        };

        let input: ItemImpl = parse2(code).unwrap();
        let cap = CapabilityImpl::new(input).unwrap();

        assert!(cap.init_fn.is_async);
        assert!(cap.reset_fn.is_async);
    }

    #[test]
    fn test_with_error_type() {
        let code = quote! {
            impl StatefulServer {
                type Client = SimpleClient;
                type Error = MyError;

                fn new() -> Self { Self }
                fn reset(&mut self) {}
                fn new_client(&self, client: &SimpleClient) -> Result<(), MyError> { Ok(()) }
                fn fallible(&self, _client: &SimpleClient) -> Result<u32, MyError> { Ok(42) }
            }
        };

        let input: ItemImpl = parse2(code).unwrap();
        let cap = CapabilityImpl::new(input).unwrap();

        assert!(cap.ident.error_tn.is_some());
        assert!(cap.new_client_fn.error_type.is_some());
        assert_eq!(cap.methods.len(), 1);
    }


    #[test]
    fn test_generate_ffi_exports() {
        let attr = quote! { TestServer };
        let expected_cap = "cap".into();

        let trait_code = quote! {
            impl TestServer {
                type Client = TestClient;

                fn new_client(&self, client: &TestClient);
                fn get_value(&self, client: &TestClient) -> u32;
                async fn async_op(&self, client: &TestClient, x: u32) -> u32;
            }
        };

        // 2. Parse all components
        let trait_def = parse2(trait_code).unwrap();

        let trait_def =
            CapabilityImpl::new(trait_def).unwrap();

        // 3. Generate the FFI exports
        let output = trait_def.generate_ffi_exports();

        let capability_ffi_funcs: Vec<_> = trait_def.capability_ffis()
            .iter()
            .map(|ffi| ffi.generate_capability_ffi())
            .collect();

        // 4. Define expected output
        let expected = quote! {
            #(#capability_ffi_funcs)*

            const __TEST_SERVER: &'static str = "__test_server";
            const __TEST_SERVER__NEW_CLIENT: &'static str = "__test_server__new_client";
            const __TEST_SERVER__GET_VALUE: &'static str = "__test_server__get_value";
            const __TEST_SERVER__ASYNC_OP: &'static str = "__test_server__async_op";
            const __TEST_SERVER__METHODS: [::pyroduct::capability_host::ffi::FunctionExport; 3usize] = [
                ::pyroduct::capability_host::ffi::FunctionExport {
                    module: __TEST_SERVER.as_ptr(),
                    module_len: __TEST_SERVER.len(),
                    name: __TEST_SERVER__NEW_CLIENT.as_ptr(),
                    name_len: __TEST_SERVER__NEW_CLIENT.len(),
                    func: ::pyroduct::capability_host::ffi::Function::Sync(__test_server__new_client__ffi),
                },
                ::pyroduct::capability_host::ffi::FunctionExport {
                    module: __TEST_SERVER.as_ptr(),
                    module_len: __TEST_SERVER.len(),
                    name: __TEST_SERVER__GET_VALUE.as_ptr(),
                    name_len: __TEST_SERVER__GET_VALUE.len(),
                    func: ::pyroduct::capability_host::ffi::Function::Sync(__test_server__get_value__ffi),
                },
                ::pyroduct::capability_host::ffi::FunctionExport {
                    module: __TEST_SERVER.as_ptr(),
                    module_len: __TEST_SERVER.len(),
                    name: __TEST_SERVER__ASYNC_OP.as_ptr(),
                    name_len: __TEST_SERVER__ASYNC_OP.len(),
                    func: ::pyroduct::capability_host::ffi::Function::Async(__test_server__async_op__ffi),
                },
            ];
        };

        crate::fmt::assert_code_eq_token(&output, &expected);
    }

        #[test]
    fn test_generate_client_impl_integration() {
        let attr = quote! { MyState };
        let expected_cap = "cap".into();
        let expected_class = Rc::new(CapabilityIdent {
            config_tn: Some(syn::parse_str("MyConfig").unwrap()),
            state_tn: format_ident!("MyState"),
            client_tn: format_ident!("MyClient"),
            error_tn: None,
        });

        let code = parse2(quote! {
            impl MyState {
                type Client = MyClient;
                type Config = MyConfig;

                fn new(config: Option<MyConfig>) -> MyState {
                    if let Some(config) = config {
                        MyState { config }
                    } else {
                        MyState { config: MyConfig::default() }
                    }
                }

                fn get_info(&self, client: &MyClient) -> u32;
            }
        })
        .unwrap();

        let expected = quote! {
            impl MyClient {
                todo!()
            }
        };

        crate::fmt::assert_code_eq_token(&output, &expected);
    }

    #[test]
    fn test_generate_client_impl_with_error_and_input_structs() {
        let attr = quote! { MyState };
        let expected_class = Rc::new(CapabilityIdent {
            config_tn: None,
            state_tn: format_ident!("MyState"),
            client_tn: format_ident!("AdvancedClient"),
            error_tn: Some(syn::parse_str("MyError").unwrap()),
        });
        let expected_cap = "cap".into();

        let code = parse2(quote! {
            impl AdvancedStruct {
                type Client = AdvancedClient;
                type Error = MyError;

                fn new_client(&self, client: &AdvancedClient) -> Result<(), MyError> {
                    AdvancedClient { name }
                }

                fn reset(&mut self) {}

                async fn process(&self, client: &AdvancedClient, val: u32, flag: bool) -> Result<u32, MyError>;
                fn sync_op(&self, client: &AdvancedClient, level: u8) -> Result<bool, MyError>;
            }
        })
        .unwrap();


        let client_method = quote! {
            async fn process(&self, val: u32, flag: bool) -> Result<u32, MyError>;
        };
        let client_method_2 = quote! {
            fn sync_op(&self, level: u8) -> Result<bool, MyError>;
        };

        let def = CapabilityDefTrait::from_trait(attr, code, &expected_cap)
            .expect("Failed to parse capability trait");
        let output = def.generate_client_impl(None);

        let expected = quote! {
            impl AdvancedClient {
                #client_method
                #client_method_2
            }
        };

        crate::fmt::assert_code_eq_token(&output, &expected);
    }
}