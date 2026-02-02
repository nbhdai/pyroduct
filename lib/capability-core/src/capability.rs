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

use crate::lifecycle::{InitFn, NewClientFn, ResetFn};
use crate::methods::ImplMethod;
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

    // Other items (consts, etc.) - excluding type aliases
    pub other_items: Vec<ImplItem>,
    pub attrs: Vec<syn::Attribute>,
}

impl CapabilityImpl {
    pub fn new(
        input: ItemImpl,
        required_docs: bool,
        cap_name: &str,
        cap_semver: &str,
    ) -> syn::Result<Self> {
        // 1. Extract state/server type name
        let state_tn =
            match &*input.self_ty {
                Type::Path(tp) => tp.path.get_ident().cloned().ok_or_else(|| {
                    Error::new_spanned(&input.self_ty, "Expected simple type name")
                })?,
                _ => {
                    return Err(Error::new_spanned(
                        &input.self_ty,
                        "Expected simple type name",
                    ));
                }
            };

        // 2. Ensure no trait impl
        if input.trait_.is_some() {
            return Err(Error::new_spanned(
                &input,
                "#[capability] cannot be used on trait implementations",
            ));
        }
        let attrs = input.attrs.clone();

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
                    }
                    // Note: We intentionally do NOT add type aliases to other_items
                    // because inherent associated types are unstable in Rust
                }
                _ => {}
            }
        }

        let client_tn = client_tn
            .ok_or_else(|| Error::new_spanned(&state_tn, "Missing `type Client = ...;`"))?;

        // Build identifiers
        let ident = Rc::new(CapabilityIdent {
            pkg_name: cap_name.to_string(),
            pkg_version: cap_semver.to_string(),
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
                ImplItem::Type(_) => {
                    // Skip type aliases - they were already processed above
                    // and we don't want them in the output impl block
                }
                other => other_items.push(other.clone()),
            }
        }

        let new_client_fn = new_client_fn.ok_or_else(|| {
            Error::new_spanned(
                &ident.state_tn,
                "Missing `fn new_client(&self, client: &Client)`",
            )
        })?;
        let init_fn = init_fn.ok_or_else(|| {
            Error::new_spanned(
                &ident.state_tn,
                "Missing `fn new() -> Self` or `fn new(config: &Config) -> Self`",
            )
        })?;
        let reset_fn = reset_fn
            .ok_or_else(|| Error::new_spanned(&ident.state_tn, "Missing `fn reset(&mut self)`"))?;

        // 5. Second pass: parse methods with class context
        let methods: Result<Vec<_>, _> = method_fns
            .iter()
            .map(|f| ImplMethod::parse(f, &ident, required_docs))
            .collect();
        let methods = methods?;

        Ok(Self {
            ident,
            init_fn,
            reset_fn,
            new_client_fn,
            methods,
            other_items,
            attrs,
        })
    }

    /// Generate all output code
    pub fn expand_capability(&self) -> TokenStream {
        let server_impl = self.generate_server_impl();
        let lifecycle_ffi = self.generate_lifecycle_ffi();
        let method_ffis = self.generate_method_ffis();
        let export_table = self.generate_export_table();

        quote! {
            #server_impl
            #lifecycle_ffi
            #method_ffis
            #export_table
        }
    }

    /// Generate all output code
    pub fn expand_module(&self) -> TokenStream {
        let wasm_imports = self.generate_wasm_imports();
        let client_impl = self.generate_client_impl();

        quote! {
            #client_impl
            #wasm_imports
        }
    }

    fn generate_server_impl(&self) -> TokenStream {
        let server = &self.ident.state_tn;
        let init_method = self.init_fn.generate_impl_method();
        let reset_method = self.reset_fn.generate_impl_method();
        let new_client_method = self.new_client_fn.generate_impl_method();
        let other_items = &self.other_items;

        let methods: Vec<_> = self
            .methods
            .iter()
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

        // 1. Generate the Register Method (on the user struct)
        let register_method = {
            let ffi = self.new_client_fn.build_ffi(&self.ident);
            let wasm_call = ffi.generate_wasm_call(Some(&module));
            let error_type = &self.new_client_fn.error_type;

            let (return_sig, body) = if let Some(err) = error_type {
                (
                    quote!(Result<::pyroduct::module_capability::Client<Self>, #err>),
                    quote! {
                         let ffi_result = #wasm_call;
                         match ffi_result {
                             Ok(_) => Ok(
                                ::pyroduct::module_capability::Client::new(self, __config_buf)
                             ),
                             Err(e) => Err(e.into()),
                         }
                    },
                )
            } else {
                (
                    quote!(::pyroduct::module_capability::Client<Self>),
                    quote! {
                        #wasm_call;
                        ::pyroduct::module_capability::Client::new(self, __config_buf)
                    },
                )
            };

            quote! {
                pub fn register(self) -> #return_sig {
                    let __config_buf = ::pyroduct::rkyv::to_bytes::<::pyroduct::rkyv::rancor::Error>(&self)
                        .expect("Failed to serialize config");

                    #body
                }
            }
        };

        let client_impl = quote! {
            impl #client {
                #register_method
            }
        };

        // 2. Generate the trait with method signatures
        let trait_name = format_ident!("{}Methods", client);

        let trait_methods: Vec<_> = self
            .methods
            .iter()
            .map(|m| {
                let name = &m.name;
                let output = &m.output;
                let args: Vec<_> = m.inputs.iter().map(|(n, t)| quote!(#n: #t)).collect();
                let docs = m.doc_attrs();

                quote! {
                    #(#docs)*
                    fn #name(&self, #(#args),*) #output;
                }
            })
            .collect();

        let trait_def = quote! {
            pub trait #trait_name {
                #(#trait_methods)*
            }
        };

        // 3. Generate the trait implementation for Client<T>
        let method_impls: Vec<_> = self
            .methods
            .iter()
            .map(|m| m.generate_client_method(&module))
            .collect();

        let trait_impl = quote! {
            impl #trait_name for ::pyroduct::module_capability::Client<#client> {
                #(#method_impls)*
            }
        };

        quote! {
            #client_impl
            #trait_def
            #trait_impl
        }
    }

    fn generate_lifecycle_ffi(&self) -> TokenStream {
        let server = &self.ident.state_tn;

        let init_ffi = self.init_fn.generate_ffi(server);
        let reset_ffi = self.reset_fn.generate_ffi(server);
        let drop_ffi = self.generate_drop_ffi();
        let new_client_ffi = self
            .new_client_fn
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
        let method_ffis: Vec<_> = self
            .methods
            .iter()
            .map(|m| m.build_ffi().generate_capability_ffi())
            .collect();

        quote! {
            #(#method_ffis)*
        }
    }

    fn generate_export_table(&self) -> TokenStream {
        let cap_id = self.ident.cap_id();

        let server = &self.ident.state_tn;
        let server_snake = AsSnakeCase(server.to_string()).to_string();
        let server_upper = server_snake.to_uppercase();

        let class_name_static = format_ident!("__{}", server_upper);
        let class_name_string = format!("__{}", server_snake);

        let drop_name = format_ident!("__{}__ffi_drop", server_snake);

        // Collect all FFIs
        let new_client_ffi = self.new_client_fn.build_ffi(&self.ident);
        let method_ffis: Vec<_> = self.methods.iter().map(|m| m.build_ffi()).collect();

        let all_ffis: Vec<_> = std::iter::once(&new_client_ffi)
            .chain(method_ffis.iter())
            .collect();

        let static_strs: Vec<_> = all_ffis
            .iter()
            .map(|ffi| {
                let trace_name = ffi.trace_name().to_string();
                let static_name = ffi.trace_name_static();
                quote! { const #static_name: &'static str = #trace_name; }
            })
            .collect();

        let exports: Vec<_> = all_ffis
            .iter()
            .map(|ffi| ffi.generate_vtable_entry())
            .collect();

        let num_exports = exports.len();
        let exports_array_name = format_ident!("{}__METHODS", class_name_static);

        let init_export = self.init_fn.generate_export(server);
        let reset_export = self.reset_fn.generate_export(server);

        let capability_manifest_fn = quote! {
            #[unsafe(no_mangle)]
            pub extern "C" fn capability_manifest<'a>(
                id: u64,
                log_callback: ::pyroduct::capability_host::ffi::LogCallback,
            ) -> ::pyroduct::capability_host::ffi::ClassExport<'a> {
                ::pyroduct::capability::init_logging(id, log_callback);

                ::pyroduct::capability_host::ffi::ClassExport {
                    len: #exports_array_name.len(),
                    ptr: #exports_array_name.as_ptr() as *mut _,
                    init: #init_export,
                    drop: ::pyroduct::capability_host::ffi::ClassDropFn::Sync(#drop_name),
                    reset: #reset_export,
                }
            }
        };

        quote! {
            const CAPABILITY_NAME_VERSION: &'static str = #cap_id;
            const #class_name_static: &'static str = #class_name_string;
            #(#static_strs)*

            const #exports_array_name: [::pyroduct::capability_host::ffi::FunctionExport; #num_exports] = [
                #(#exports),*
            ];

            #capability_manifest_fn
        }
    }

    fn generate_wasm_imports(&self) -> TokenStream {
        let new_client_decl = self
            .new_client_fn
            .build_ffi(&self.ident)
            .generate_client_wasm();

        let method_decls: Vec<_> = self
            .methods
            .iter()
            .map(|m| m.build_ffi().generate_client_wasm())
            .collect();

        quote! {
            mod wasm {
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
        let cap = CapabilityImpl::new(input, false, "cap_name", "0.1.0").unwrap();

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

                fn new(config: Option<MyConfig>) -> Self { Self }
                fn reset(&mut self) {}
                fn new_client(&self, client: &SimpleClient) {}
            }
        };

        let input: ItemImpl = parse2(code).unwrap();
        let cap = CapabilityImpl::new(input, false, "cap_name", "0.1.0").unwrap();

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

                fn new(config: Option<OtherConfig>) -> Self { Self }
                fn reset(&mut self) {}
                fn new_client(&self, client: &SimpleClient) {}
            }
        };

        let input: ItemImpl = parse2(code).unwrap();
        let err = CapabilityImpl::new(input, false, "cap_name", "0.1.0").unwrap_err();
        println!("{}", err);
        assert!(err.to_string().contains("Type mismatch. Expected 'Option<MyConfig>' based on macro attribute, found 'Option<OtherConfig>'"));
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
        let cap = CapabilityImpl::new(input, false, "cap_name", "0.1.0").unwrap();

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
        let cap = CapabilityImpl::new(input, false, "cap_name", "0.1.0").unwrap();

        assert!(cap.ident.error_tn.is_some());
        assert!(cap.new_client_fn.error_type.is_some());
        assert_eq!(cap.methods.len(), 1);
    }

    #[test]
    fn test_generate_export_table() {
        let code = quote! {
            impl TestServer {
                type Client = TestClient;

                fn new() -> Self { Self }
                fn reset(&mut self) {}
                fn new_client(&self, client: &TestClient) {}
                fn get_value(&self, client: &TestClient) -> u32 { 0 }
            }
        };

        let input: ItemImpl = parse2(code).unwrap();
        let cap = CapabilityImpl::new(input, false, "cap_name", "0.1.0").unwrap();

        let output = cap.generate_export_table();

        let expected = quote! {
            const CAPABILITY_NAME_VERSION: &'static str = "cap_name:0.1.0";
            const __TEST_SERVER: &'static str = "__test_server";
            const __TEST_SERVER: &'static str = "__test_server";
            const __TEST_SERVER__NEW_CLIENT: &'static str = "__test_server__new_client";
            const __TEST_SERVER__GET_VALUE: &'static str = "__test_server__get_value";

            const __TEST_SERVER__METHODS: [::pyroduct::capability_host::ffi::FunctionExport; 2usize] = [
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
                }
            ];

            #[unsafe(no_mangle)]
            pub extern "C" fn capability_manifest<'a>(
                id: u64,
                log_callback: ::pyroduct::capability_host::ffi::LogCallback,
            ) -> ::pyroduct::capability_host::ffi::ClassExport<'a> {
                ::pyroduct::capability::init_logging(id, log_callback);

                ::pyroduct::capability_host::ffi::ClassExport {
                    len: __TEST_SERVER__METHODS.len(),
                    ptr: __TEST_SERVER__METHODS.as_ptr() as *mut _,
                    init: ::pyroduct::capability_host::ffi::ClassInitFn::Sync(__test_server__ffi_init),
                    drop: ::pyroduct::capability_host::ffi::ClassDropFn::Sync(__test_server__ffi_drop),
                    reset: ::pyroduct::capability_host::ffi::ClassResetFn::Sync(__test_server__ffi_reset),
                }
            }
        };

        crate::fmt::assert_code_eq_token(&output, &expected);
    }

    #[test]
    fn test_generate_client_impl_integration() {
        // 1. Define Input
        let code = quote! {
            impl MyState {
                type Client = MyClient;
                type Config = MyConfig;

                fn new(config: Option<MyConfig>) -> Self { Self }
                fn reset(&mut self) {}
                fn new_client(&self, client: &MyClient) {}
                fn get_info(&self, client: &MyClient) -> u32 { 0 }
                fn get_other_info(&self, client: &MyClient, data: f32) -> u32 { 0 }
            }
        };

        // 2. Parse
        let input: ItemImpl = parse2(code).unwrap();
        let cap = CapabilityImpl::new(input, false, "cap_name", "0.1.0").unwrap();

        // 3. Generate Output
        let output = cap.generate_client_impl();

        // 4. Define Expected Output
        // Checks constructor generation (using rkyv for config) and normal method generation.
        let expected = quote! {
            impl MyClient {
                pub fn register(self) -> ::pyroduct::module_capability::Client<Self> {
                    let __config_buf = ::pyroduct::rkyv::to_bytes::<::pyroduct::rkyv::rancor::Error>(&self)
                        .expect("Failed to serialize config");

                    ::pyroduct::module_capability::access::call_from_wasm::<
                        (),
                        (),
                        _
                    >(
                        "__my_state__new_client",
                        Some(&__config_buf),
                        None,
                        |client_state_ptr: *const u8,
                         client_state_len: usize,
                         input_ptr: *const u8,
                         input_len: usize| {
                            unsafe {
                                wasm::__my_state__new_client__wasm(
                                    client_state_ptr,
                                    client_state_len,
                                    input_ptr,
                                    input_len
                                )
                            }
                        }
                    );
                    ::pyroduct::module_capability::Client::new(self, __config_buf)
                }
            }
            pub trait MyClientMethods {
                fn get_info(&self) -> u32;
                fn get_other_info(&self, data: f32) -> u32;
            }
            impl MyClientMethods for ::pyroduct::module_capability::Client<MyClient> {
                fn get_info(&self) -> u32 {
                    ::pyroduct::module_capability::access::call_from_wasm::<
                        (),
                        u32,
                        _
                    >(
                        "__my_state__get_info",
                        Some(self.buffer()),
                        None,
                        |client_state_ptr: *const u8,
                         client_state_len: usize,
                         input_ptr: *const u8,
                         input_len: usize| {
                            unsafe {
                                wasm::__my_state__get_info__wasm(
                                    client_state_ptr,
                                    client_state_len,
                                    input_ptr,
                                    input_len
                                )
                            }
                        }
                    )
                }

                fn get_other_info(&self, data: f32) -> u32 {
                    ::pyroduct::module_capability::access::call_from_wasm::<
                        f32,
                        u32,
                        _
                    >(
                        "__my_state__get_other_info",
                        Some(self.buffer()),
                        Some(&data),
                        |client_state_ptr: *const u8,
                         client_state_len: usize,
                         input_ptr: *const u8,
                         input_len: usize| {
                            unsafe {
                                wasm::__my_state__get_other_info__wasm(
                                    client_state_ptr,
                                    client_state_len,
                                    input_ptr,
                                    input_len
                                )
                            }
                        }
                    )
                }
            }
        };

        crate::fmt::assert_code_eq_token(&output, &expected);
    }

    #[test]
    fn test_generate_client_impl_with_error_and_input_structs() {
        // 1. Define Input: Complex case with Errors and Arguments
        let code = quote! {
            impl AdvancedStruct {
                type Client = AdvancedClient;
                type Error = MyError;

                fn new() -> Self { Self }
                fn reset(&mut self) {}

                fn new_client(&self, client: &AdvancedClient) -> Result<(), MyError> {
                    Ok(())
                }

                async fn process(&self, client: &AdvancedClient, val: u32, flag: bool) -> Result<u32, MyError> {
                    Ok(val)
                }
            }
        };

        // 2. Parse
        let input: ItemImpl = parse2(code).unwrap();
        let cap = CapabilityImpl::new(input, false, "cap_name", "0.1.0").unwrap();

        // 3. Generate Output
        let output = cap.generate_client_impl();

        // 4. Define Expected Output
        // - new_client constructor should return Result<Self, MyError>.
        // - process method should define an input struct and return Result<u32, MyError>.
        let expected = quote! {
            impl AdvancedClient {
                pub fn register(self) -> Result<::pyroduct::module_capability::Client<Self>, MyError> {

                    let __config_buf = ::pyroduct::rkyv::to_bytes::<::pyroduct::rkyv::rancor::Error>(&self)
                        .expect("Failed to serialize config");

                    let ffi_result = ::pyroduct::module_capability::access::call_from_wasm::<
                        (),
                        Result<(), MyError>,
                        _
                    >(
                        "__advanced_struct__new_client",
                        Some(&__config_buf),
                        None,
                        |client_state_ptr: *const u8,
                         client_state_len: usize,
                         input_ptr: *const u8,
                         input_len: usize| {
                            unsafe {
                                wasm::__advanced_struct__new_client__wasm(
                                    client_state_ptr,
                                    client_state_len,
                                    input_ptr,
                                    input_len
                                )
                            }
                        }
                    );

                    match ffi_result {
                        Ok(_) => Ok(::pyroduct::module_capability::Client::new(self, __config_buf)),
                        Err(e) => Err(e.into()),
                    }
                }
            }
            pub trait AdvancedClientMethods {
                fn process(&self, val: u32, flag: bool) -> Result<u32, MyError>;
            }
            impl AdvancedClientMethods for ::pyroduct::module_capability::Client<AdvancedClient> {
                fn process(&self, val: u32, flag: bool) -> Result<u32, MyError> {
                    #[derive(::pyroduct::rkyv::Archive, ::pyroduct::rkyv::Deserialize, ::pyroduct::rkyv::Serialize)]
                    #[rkyv(crate = ::pyroduct::rkyv)]
                    struct __AdvancedStruct__Process__Input {
                        pub val: u32,
                        pub flag: bool
                    }

                    ::pyroduct::module_capability::access::call_from_wasm::<
                        __AdvancedStruct__Process__Input,
                        Result<u32, MyError>,
                        _
                    >(
                        "__advanced_struct__process",
                        Some(self.buffer()),
                        Some(&__AdvancedStruct__Process__Input { val, flag }),
                        |client_state_ptr: *const u8,
                         client_state_len: usize,
                         input_ptr: *const u8,
                         input_len: usize| {
                            unsafe {
                                wasm::__advanced_struct__process__wasm(
                                    client_state_ptr,
                                    client_state_len,
                                    input_ptr,
                                    input_len
                                )
                            }
                        }
                    )
                }
            }
        };

        crate::fmt::assert_code_eq_token(&output, &expected);
    }
}
