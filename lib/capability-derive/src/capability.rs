//
use heck::AsSnakeCase;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::visit_mut::{self, VisitMut};
use syn::{
    Error, Expr, ExprStruct, FnArg, GenericArgument, Ident, ImplItem, ImplItemConst, ImplItemFn,
    ImplItemType, ItemImpl, ItemTrait, Member, Meta, Path, PathArguments, Result, ReturnType,
    Token, TraitItem, TraitItemConst, TraitItemFn, TraitItemType, Type, TypePath, parse2, parse_quote
};

mod constructors;
use constructors::ClientConstructor;
mod methods;
use methods::CapabilityMethod;

// ==============================================================================
// 1. Main Processor
// ==============================================================================

/// Processes the input Trait definition to generate the glue code for the
/// capability system.
///
/// This structure manages the duality between the **Client Side** (WASM) and the
/// **Server Side** (Host):
///
/// 1.  **Client State (`type Client`):**
///     Represents the serializable configuration or state that exists inside the
///     WASM module. Methods generated for the Client rely *only* on this state
///     to package requests and send them to the host.
///
/// 2.  **Server State (Host Implementation):**
///     Represents the actual backing struct on the Host that holds resources
///     (database connections, hardware handles, etc.).
///
/// **Code Generation Responsibilities:**
/// * **Client Impl:** Generates methods for the `Client` struct that serialize
///     arguments and the `Client` state itself, delegating execution to the Host via FFI.
/// * **Host FFI:** Generates the `extern "C"` entry points that receive the
///     serialized `Client` state, deserialize it, and invoke the logic on the
///     corresponding `Server` state instance.
pub struct CapabilityDefTrait {
    pub trait_name: Ident,
    pub state_name: Option<Ident>,
    pub original_attrs: Vec<syn::Attribute>,
    pub generics: syn::Generics,
    pub methods: Vec<CapabilityMethod>,
    pub constructors: Vec<ClientConstructor>,
    pub other_items: Vec<TraitItem>,
    pub explicit_error_type: Option<Type>,
    pub explicit_client_type: Option<Type>,
}


impl CapabilityDefTrait {
    /// Ingests a Trait Definition (original logic).
    pub fn from_trait(item: TokenStream) -> syn::Result<Self> {
        let input: ItemTrait = parse2(item)?;
        let trait_name = input.ident.clone();
        let mut methods = Vec::new();
        let mut constructors = Vec::new();
        let mut other_items = Vec::new();
        let mut explicit_error_type: Option<Type> = None;
        let mut explicit_client_type = None;

        // 1. First Pass: Collect types and items.
        for item in &input.items {
            match item {
                TraitItem::Type(ty) if ty.ident == "Error" => {
                    if let Some((_, error_type)) = &ty.default {
                        explicit_error_type = Some(error_type.clone());
                        other_items.push(item.clone());
                    } else {
                        return Err(syn::Error::new_spanned(ty, "Missing Error type, need a default = ...;"));
                    }
                    
                }
                TraitItem::Type(ty) if ty.ident == "Client" => {
                    if let Some((_, client_type)) = &ty.default {
                        // --- VALIDATION START ---
                        match client_type {
                            Type::Path(type_path) => {
                                if type_path.qself.is_some() {
                                    return Err(syn::Error::new_spanned(
                                        client_type, 
                                        "Client type cannot use qualified paths (e.g. <Type as Trait>::Assoc)."
                                    ));
                                }
                                for segment in &type_path.path.segments {
                                    if !matches!(segment.arguments, PathArguments::None) {
                                        return Err(syn::Error::new_spanned(
                                            client_type, 
                                            "Client type cannot have generic arguments or lifetimes (e.g., Client<T>)."
                                        ));
                                    }
                                }
                            }
                            _ => {
                                return Err(syn::Error::new_spanned(
                                    client_type, 
                                    "Client type must be a simple struct path. References, pointers, or arrays are not allowed."
                                ));
                            }
                        }
                        explicit_client_type = Some(client_type.clone());
                        other_items.push(item.clone());
                    } else {
                        return Err(syn::Error::new_spanned(ty, "Missing Client type, need a default = ...;"));
                    }
                }
                TraitItem::Fn(_) => {} 
                _ => other_items.push(item.clone()),
            }
        }

        // 2. Second Pass: Method Verification and Collection
        for item in &input.items {
            if let TraitItem::Fn(method) = item {
                let is_constructor = if let Some(client_type) = &explicit_client_type {
                    if let ReturnType::Type(_, ty) = &method.sig.output {
                        let ty_str = quote!(#ty).to_string();
                        let client_str = quote!(#client_type).to_string();
                        ty_str == client_str || ty_str == "Self :: Client"
                    } else {
                        false
                    }
                } else {
                    false
                };

                if is_constructor {
                    let client_type = explicit_client_type.as_ref().unwrap();
                    let ctor = ClientConstructor::new(method, client_type, explicit_error_type.as_ref())?;
                    constructors.push(ctor);
                } else {
                    let cap_method = CapabilityMethod::new(
                        method.clone(), 
                        explicit_client_type.as_ref(), 
                        explicit_error_type.as_ref()
                    )?;
                    methods.push(cap_method);
                }
            }
        }

        // 3. Final Validation
        if explicit_client_type.is_some() {
            if constructors.is_empty() {
                return Err(syn::Error::new(
                    Span::call_site(), 
                    "A 'Client' type is defined, so you must provide at least one Client Constructor (static method returning Self::Client)."
                ));
            }
        } else {
            if !constructors.is_empty() {
                return Err(syn::Error::new(
                    Span::call_site(), 
                    "Client Constructors defined but no 'Client' type found."
                ));
            }
        }

        Ok(Self {
            original_attrs: input.attrs,
            generics: input.generics,
            methods,
            trait_name,
            state_name: None,
            constructors,
            other_items,
            explicit_error_type,
            explicit_client_type,
        })
    }

    /// Ingests a Trait Implementation (`impl Trait for State`).
    pub fn from_impl(item: TokenStream) -> syn::Result<Self> {
        let input: ItemImpl = parse2(item)?;

        // 1. Extract Trait Name
        let trait_name = match &input.trait_ {
            Some((_, path, _)) => path.segments.last().unwrap().ident.clone(),
            None => return Err(Error::new_spanned(input, "Capability definition must be a trait implementation (impl Trait for State).")),
        };

        // 2. Extract State Name
        let state_name = if let Type::Path(type_path) = &*input.self_ty {
             Some(type_path.path.segments.last().unwrap().ident.clone())
        } else {
             return Err(Error::new_spanned(input.self_ty, "Implementation target must be a struct (TypePath)."));
        };

        let mut methods = Vec::new();
        let mut constructors = Vec::new();
        let mut other_items = Vec::new();
        let mut explicit_error_type: Option<Type> = None;
        let mut explicit_client_type = None;

        // 3. First Pass (Impl Items -> Trait Items)
        for item in &input.items {
            match item {
                ImplItem::Type(ty) if ty.ident == "Error" => {
                    explicit_error_type = Some(ty.ty.clone());
                    // Convert ImplItemType to TraitItemType for storage
                    other_items.push(impl_type_to_trait_type(ty));
                }
                ImplItem::Type(ty) if ty.ident == "Client" => {
                    let client_type = &ty.ty;
                    // Validate Client Type (Copying logic from from_trait)
                    match client_type {
                        Type::Path(type_path) => {
                            if type_path.qself.is_some() {
                                return Err(syn::Error::new_spanned(client_type, "Client type cannot use qualified paths."));
                            }
                            for segment in &type_path.path.segments {
                                if !matches!(segment.arguments, PathArguments::None) {
                                    return Err(syn::Error::new_spanned(client_type, "Client type cannot have generic arguments."));
                                }
                            }
                        }
                        _ => return Err(syn::Error::new_spanned(client_type, "Client type must be a simple struct path.")),
                    }
                    explicit_client_type = Some(client_type.clone());
                    other_items.push(impl_type_to_trait_type(ty));
                }
                ImplItem::Fn(_) => {} 
                ImplItem::Const(c) => other_items.push(impl_const_to_trait_const(c)),
                _ => {} // Ignore macros etc.
            }
        }

        // 4. Second Pass: Methods
        for item in &input.items {
            if let ImplItem::Fn(impl_method) = item {
                // Convert ImplItemFn -> TraitItemFn to reuse existing logic
                let method = impl_fn_to_trait_fn(impl_method);

                let is_constructor = if let Some(client_type) = &explicit_client_type {
                    if let ReturnType::Type(_, ty) = &method.sig.output {
                        let ty_str = quote!(#ty).to_string();
                        let client_str = quote!(#client_type).to_string();
                        ty_str == client_str || ty_str == "Self :: Client"
                    } else {
                        false
                    }
                } else {
                    false
                };

                if is_constructor {
                    let client_type = explicit_client_type.as_ref().unwrap();
                    let ctor = ClientConstructor::new(&method, client_type, explicit_error_type.as_ref())?;
                    constructors.push(ctor);
                } else {
                    let cap_method = CapabilityMethod::new(
                        method.clone(), 
                        explicit_client_type.as_ref(), 
                        explicit_error_type.as_ref()
                    )?;
                    methods.push(cap_method);
                }
            }
        }

        // 5. Final Validation
        if explicit_client_type.is_some() {
            if constructors.is_empty() {
                return Err(syn::Error::new(Span::call_site(), "Client type defined, but no Client Constructor found."));
            }
        } else if !constructors.is_empty() {
             return Err(syn::Error::new(Span::call_site(), "Client Constructors defined but no 'Client' type found."));
        }

        Ok(Self {
            original_attrs: input.attrs,
            generics: input.generics,
            methods,
            trait_name,
            state_name,
            constructors,
            other_items,
            explicit_error_type,
            explicit_client_type,
        })
    }

    /// Generates the final TokenStream for the Trait definition.
    pub fn generate_trait_definition(&self) -> Result<TokenStream> {
        if self.state_name.is_some() {
            return Err(Error::new_spanned(&self.trait_name, "Unable to generate a trait definition from an impl."));
        }
        let trait_name = &self.trait_name;
        let attrs = &self.original_attrs;
        let generics = &self.generics;
        let other_items = &self.other_items;
        let method_signatures = self.methods.iter().map(|m| m.trait_method_generation());
        let new_client_method = self.generate_new_client_signature();

        Ok(quote! {
            #(#attrs)*
            pub trait #trait_name #generics {
                #(#other_items)*
                #new_client_method
                #(#method_signatures)*
            }
        })
    }

    /// Helper to determine the signature of `new_client`
    fn generate_new_client_signature(&self) -> TokenStream {
        if self.explicit_client_type.is_none() {
            return quote! {};
        }

        if self.explicit_error_type.is_some() {
            quote! {
                fn new_client(&self, client: &Self::Client) -> Result<(), Self::Error>;
            }
        } else {
            quote! {
                fn new_client(&self, client: &Self::Client) -> ();
            }
        }
    }

    /// Generates the `impl ClientType { ... }` block.
    pub fn generate_client_impl(&self) -> Result<TokenStream> {
        let state_name = self.state_name.as_ref().ok_or(Error::new_spanned(&self.trait_name, "Unable to generate a client without knowing the state, need to generate clients On implementations of this capability trait."))?;

        Ok(self.internal_generate_client_impl(state_name))
    }

    /// Generates the `impl ClientType { ... }` block.
    fn internal_generate_client_impl(&self, state_name: &Ident) -> TokenStream {
        let client_type = match &self.explicit_client_type {
            Some(ty) => ty,
            None => return TokenStream::new(),
        };

        let (impl_generics, _, where_clause) = self.generics.split_for_impl();
        let trait_name = &self.trait_name;
        
        let capability_methods = self.methods.iter().map(|m| {
            m.client_method_generation(trait_name, state_name)
        });

        let constructors = self.constructors.iter().map(|c| {
            c.client_method_generation(trait_name, state_name)
        });

        quote! {
            impl #impl_generics #client_type #where_clause {
                #(#constructors)*

                #(#capability_methods)*
            }
        }
    }
}

// --- Converters: ImplItem -> TraitItem ---

fn impl_fn_to_trait_fn(impl_fn: &ImplItemFn) -> TraitItemFn {
    TraitItemFn {
        attrs: impl_fn.attrs.clone(),
        sig: impl_fn.sig.clone(),
        default: Some(impl_fn.block.clone()),
        semi_token: None,
    }
}

fn impl_type_to_trait_type(impl_ty: &ImplItemType) -> TraitItem {
    TraitItem::Type(TraitItemType {
        attrs: impl_ty.attrs.clone(),
        type_token: impl_ty.type_token,
        ident: impl_ty.ident.clone(),
        generics: impl_ty.generics.clone(),
        colon_token: None,
        bounds: Punctuated::new(),
        // Map assignment (= Type) to Default (= Type)
        default: Some((impl_ty.eq_token, impl_ty.ty.clone())),
        semi_token: impl_ty.semi_token,
    })
}

fn impl_const_to_trait_const(impl_c: &ImplItemConst) -> TraitItem {
    TraitItem::Const(TraitItemConst {
        attrs: impl_c.attrs.clone(),
        const_token: impl_c.const_token,
        ident: impl_c.ident.clone(),
        colon_token: impl_c.colon_token,
        ty: impl_c.ty.clone(),
        // Map assignment (= Expr) to Default (= Expr)
        default: Some((impl_c.eq_token, impl_c.expr.clone())),
        semi_token: impl_c.semi_token,
        generics: impl_c.generics.clone(),
    })
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::fmt::assert_code_eq;

    /// Helper to create a CapabilityDefTrait from raw code (Trait)
    fn create_def(code: TokenStream) -> CapabilityDefTrait {
        CapabilityDefTrait::from_trait(code).expect("Failed to parse capability trait")
    }

    #[test]
    fn test_generate_client_impl_integration() {
        let code = quote! {
            trait MyTrait {
                type Client = MyClient;
                
                fn new(id: u32) -> MyClient {
                    MyClient { id }
                }

                fn get_info() -> u32;
            }
        };

        let def = create_def(code);
        let state_name = format_ident!("MyState");
        let output = def.internal_generate_client_impl(&state_name);

        let expected = r#"
            impl MyClient {
                pub fn new(id: u32) -> Self {
                    let mut new_self = (|| {
                        MyClient {
                            id,
                            __config_buf: std::vec::Vec::new(),
                        }
                    })();
                    new_self.__config_buf = ::rkyv::to_bytes::<_, 256>(&new_self)
                        .expect("Failed to serialize config")
                        .into_vec();
                    ::pyroduct::module_capability::access::call_from_wasm::<
                        Self,
                        (),
                        Self,
                        _,
                    >(
                        "__MyTrait_MyState_new_client",
                        Some(client),
                        None,
                        |client_state_ptr: *const u8,
                         client_state_len: usize,
                         input_ptr: *const u8,
                         input_len: usize| {
                            unsafe {
                                __MyTrait_MyState_new_client_wasm(
                                    client_state_ptr,
                                    client_state_len,
                                    input_ptr,
                                    input_len,
                                )
                            }
                        },
                    )
                }

                pub fn get_info(&self) -> u32 {
                    ::pyroduct::module_capability::access::call_from_wasm::<
                        MyClient,
                        (),
                        u32,
                        _,
                    >(
                        "__MyTrait_MyState_get_info",
                        Some(client),
                        None,
                        |client_state_ptr: *const u8,
                         client_state_len: usize,
                         input_ptr: *const u8,
                         input_len: usize| {
                            unsafe {
                                __MyTrait_MyState_get_info_wasm(
                                    client_state_ptr,
                                    client_state_len,
                                    input_ptr,
                                    input_len,
                                )
                            }
                        },
                    )
                }
            }
        "#;

        assert_code_eq(&output, expected);
    }

    #[test]
    fn test_generate_client_impl_with_error_and_input_structs() {
        let code = quote! {
            trait AdvancedTrait {
                type Client = AdvancedClient;
                type Error = MyError;

                fn create(name: String) -> AdvancedClient {
                    AdvancedClient { name }
                }

                fn process(val: u32, flag: bool) -> u32;
            }
        };

        let def = create_def(code);
        let state_name = format_ident!("MyState");
        let output = def.internal_generate_client_impl(&state_name);

        let expected = r#"
            impl AdvancedClient {
                pub fn create(name: String) -> Result<Self, MyError> {
                    let mut new_self = (|| {
                        AdvancedClient {
                            name,
                            __config_buf: std::vec::Vec::new(),
                        }
                    })();
                    new_self.__config_buf = ::rkyv::to_bytes::<_, 256>(&new_self)
                        .expect("Failed to serialize config")
                        .into_vec();
                    ::pyroduct::module_capability::access::call_from_wasm::<
                        Self,
                        (),
                        Result<Self, MyError>,
                        _,
                    >(
                        "__AdvancedTrait_MyState_new_client",
                        Some(client),
                        None,
                        |client_state_ptr: *const u8,
                         client_state_len: usize,
                         input_ptr: *const u8,
                         input_len: usize| {
                            unsafe {
                                __AdvancedTrait_MyState_new_client_wasm(
                                    client_state_ptr,
                                    client_state_len,
                                    input_ptr,
                                    input_len,
                                )
                            }
                        },
                    )
                }

                pub fn process(&self, val: u32, flag: bool) -> Result<u32, MyError> {
                    #[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
                    #[rkyv(compare(PartialEq), derive(Debug))]
                    struct __AdvancedTrait_MyState_process_Input {
                        pub val: u32,
                        pub flag: bool,
                    }
                    ::pyroduct::module_capability::access::call_from_wasm::<
                        AdvancedClient,
                        __AdvancedTrait_MyState_process_Input,
                        Result<u32, MyError>,
                        _,
                    >(
                        "__AdvancedTrait_MyState_process",
                        Some(client),
                        Some(&__AdvancedTrait_MyState_process_Input { val, flag }),
                        |client_state_ptr: *const u8,
                         client_state_len: usize,
                         input_ptr: *const u8,
                         input_len: usize| {
                            unsafe {
                                __AdvancedTrait_MyState_process_wasm(
                                    client_state_ptr,
                                    client_state_len,
                                    input_ptr,
                                    input_len,
                                )
                            }
                        },
                    )
                }
            }
        "#;

        assert_code_eq(&output, expected);
    }

    #[test]
    fn test_no_client_impl_generated_if_no_client_type() {
        let code = quote! {
            trait PureInterface {
                fn do_thing();
            }
        };

        let def = create_def(code);
        let state_name = format_ident!("MyState");
        let output = def.internal_generate_client_impl(&state_name);
        assert!(output.is_empty());
    }

    #[test]
    fn test_server_trait_and_ffi_generation() {
        let code = quote! {
            trait SensorFeature {
                type Client = SensorClient;
                type Error = SensorError;

                fn new(id: String) -> SensorClient {
                    SensorClient { id }
                }

                fn calibrate(offset: i32, scale: f32) -> bool;

                async fn read_async(timeout: u32) -> f64;
            }
        };
        let def = create_def(code);
        let trait_name = format_ident!("SensorFeature");
        let state_name = format_ident!("HardwareState");

        let server_trait = def.internal_generate_client_impl(&trait_name);
        
        let expected_trait = r#"
            pub trait SensorFeature {
                type Client = SensorClient;
                type Error = SensorError;
                fn new_client(&self, client: &Self::Client) -> Result<(), Self::Error>;
                fn calibrate(&self, client: &Self::Client, offset: i32, scale: f32) -> Result<bool, Self::Error>;
                async fn read_async(&self, client: &Self::Client, timeout: u32) -> Result<f64, Self::Error>;
            }
        "#;
        assert_code_eq(&server_trait, expected_trait);

        let calibrate_method = def.methods.iter().find(|m| m.name == "calibrate").expect("calibrate not found");
        let calibrate_ffi = calibrate_method.ffi_function_generation(&trait_name, &state_name);
        
        let expected_calibrate_ffi = r#"
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn __SensorFeature_HardwareState_calibrate_ffi(
                client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize,
                capability_state_ptr: *mut std::ffi::c_void,
            ) -> ::pyroduct::capability_host::ffi::FfiResult {
                ::pyroduct::capability::safe_call::sci_call::<
                    HardwareState,
                    SensorClient,
                    __SensorFeature_HardwareState_calibrate_Input,
                    Result<bool, SensorError>,
                    _,
                >(
                    client_state_ptr,
                    client_state_len,
                    input_ptr,
                    input_len,
                    capability_state_ptr,
                    |state, client, input| state.calibrate(client, input.offset, input.scale),
                )
            }
        "#;
        assert_code_eq(&calibrate_ffi, expected_calibrate_ffi);

        let read_method = def.methods.iter().find(|m| m.name == "read_async").expect("read_async not found");
        let read_ffi = read_method.ffi_function_generation(&trait_name, &state_name);

        let expected_read_ffi = r#"
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn __SensorFeature_HardwareState_read_async_ffi<'a>(
                client_state_ptr: *const u8,
                client_state_len: usize,
                input_ptr: *const u8,
                input_len: usize,
                capability_state_ptr: *mut std::ffi::c_void,
            ) -> ::pyroduct::capability_host::ffi::FfiBorrowedFutureResult<'a> {
                ::pyroduct::capability::safe_async::sci_call::<
                    HardwareState,
                    SensorClient,
                    u32,
                    Result<f64, SensorError>,
                    _,
                    _,
                >(
                    client_state_ptr,
                    client_state_len,
                    input_ptr,
                    input_len,
                    capability_state_ptr,
                    |state, client, input| async move { state.read_async(client, input).await },
                )
            }
        "#;
        assert_code_eq(&read_ffi, expected_read_ffi);
    }

    #[test]
    fn test_generate_server_trait_correctness() {
        // 1. Define the input trait (as written by the user)
        let code = quote! {
            trait ControlPlane {
                type Client = ControlClient;
                type Error = ControlError;

                // Constructor: Should result in `new_client` in server trait
                fn new(api_key: String) -> ControlClient {
                    ControlClient { api_key }
                }

                // Sync method: Should gain &self, client, and Result return
                fn sync_op(level: u8) -> bool;

                // Async method: Should stay async, gain params, and Result return
                async fn async_op(data: Vec<u8>);
            }
        };

        // 2. Parse
        let def = create_def(code);

        // 3. Generate the server-side trait definition
        let output = def.generate_trait_definition().expect("Failed to generate trait definition");

        // 4. Verify Output
        // - `new_client` signature is generated.
        // - Methods have `&self` and `client: &Self::Client` prepended.
        // - Return types are wrapped in `Result<_, Self::Error>`.
        let expected = r#"
            pub trait ControlPlane {
                type Client = ControlClient;
                type Error = ControlError;
                
                fn new_client(&self, client: &Self::Client) -> Result<(), Self::Error>;
                
                fn sync_op(&self, client: &Self::Client, level: u8) -> Result<bool, Self::Error>;
                
                async fn async_op(&self, client: &Self::Client, data: Vec<u8>) -> Result<(), Self::Error>;
            }
        "#;

        assert_code_eq(&output, expected);
    }

    #[test]
    fn test_from_impl_parsing() {
        // 1. Define an impl block (simulating `impl Trait for State`)
        let code = quote! {
            impl MyTrait for MyState {
                type Client = MyClient;
                type Error = MyError;

                fn new(id: u32) -> MyClient { 
                    MyClient { id } 
                }
                
                fn do_thing() -> u32 { 
                    42 
                }
            }
        };

        // 2. Parse from Impl
        let def = CapabilityDefTrait::from_impl(code).expect("Failed to parse impl");

        // 3. Verify extracted names
        assert_eq!(def.trait_name.to_string(), "MyTrait");
        assert_eq!(def.state_name.as_ref().unwrap().to_string(), "MyState");

        // 4. Verify Client Implementation Generation using assert_code_eq
        let client_impl = def.generate_client_impl().expect("Failed to generate client impl");
        
        // Expected Logic:
        // - Constructor `new` converts to `new_client` FFI call returning Result.
        // - Method `do_thing` converts to FFI call returning Result.
        // - `__config_buf` injection happens in the constructor.
        let expected_client_impl = r#"
            impl MyClient {
                pub fn new(id: u32) -> Result<Self, MyError> {
                    let mut new_self = (|| {
                        MyClient {
                            id,
                            __config_buf: std::vec::Vec::new(),
                        }
                    })();
                    new_self.__config_buf = ::rkyv::to_bytes::<_, 256>(&new_self)
                        .expect("Failed to serialize config")
                        .into_vec();
                    ::pyroduct::module_capability::access::call_from_wasm::<
                        Self,
                        (),
                        Result<Self, MyError>,
                        _,
                    >(
                        "__MyTrait_MyState_new_client",
                        Some(client),
                        None,
                        |client_state_ptr: *const u8,
                         client_state_len: usize,
                         input_ptr: *const u8,
                         input_len: usize| {
                            unsafe {
                                __MyTrait_MyState_new_client_wasm(
                                    client_state_ptr,
                                    client_state_len,
                                    input_ptr,
                                    input_len,
                                )
                            }
                        },
                    )
                }

                pub fn do_thing(&self) -> Result<u32, MyError> {
                    ::pyroduct::module_capability::access::call_from_wasm::<
                        MyClient,
                        (),
                        Result<u32, MyError>,
                        _,
                    >(
                        "__MyTrait_MyState_do_thing",
                        Some(client),
                        None,
                        |client_state_ptr: *const u8,
                         client_state_len: usize,
                         input_ptr: *const u8,
                         input_len: usize| {
                            unsafe {
                                __MyTrait_MyState_do_thing_wasm(
                                    client_state_ptr,
                                    client_state_len,
                                    input_ptr,
                                    input_len,
                                )
                            }
                        },
                    )
                }
            }
        "#;
        
        assert_code_eq(&client_impl, expected_client_impl);

        // 5. Verify that Trait Definition generation is explicitly forbidden for Impl inputs.
        // The `CapabilityDefTrait` logic returns an error if `state_name` is present.
        let trait_gen_result = def.generate_trait_definition();
        assert!(trait_gen_result.is_err(), "Should not be able to generate a trait definition from an impl block");
        assert_eq!(
            trait_gen_result.unwrap_err().to_string(), 
            "Unable to generate a trait definition from an impl."
        );
    }

    #[test]
    fn test_trait_generation_consistency_between_source_and_impl() {
        // 1. Define the Original Trait
        // This is what the user defines in the shared library.
        let trait_code = quote! {
            trait Database {
                type Client = DbClient;
                type Error = DbError;

                // Constructor
                fn connect(url: String) -> DbClient {
                    DbClient { url }
                }

                // Methods
                fn query(sql: String) -> Result<String, DbError>;
                async fn execute(sql: String) -> Result<u64, DbError>;
            }
        };

        // 2. Define the Implementation
        // This is what the user writes on the host side.
        let impl_code = quote! {
            impl Database for PostgresDriver {
                type Client = DbClient;
                type Error = DbError;

                fn connect(&self, client: DbClient) -> Result<(), DbError> {
                    Ok(())
                }

                fn query(&self, client: &DbClient, sql: String) -> Result<String, DbError> {
                    Ok("row".to_string())
                }

                async fn execute(&self, client: &DbClient, sql: String) -> Result<u64, DbError> {
                    Ok(1)
                }
            }
        };

        // 3. Generate Trait Definition from the TRAIT source
        let def_from_trait = CapabilityDefTrait::from_trait(trait_code)
            .expect("Failed to parse trait");
        let trait_output = def_from_trait.generate_trait_definition()
            .expect("Failed to generate definition from trait");
        let mut def_from_impl = CapabilityDefTrait::from_impl(impl_code)
            .expect("Failed to parse impl");
        
        // HACK: Force state_name to None to bypass the guard for this specific comparison test
        def_from_impl.state_name = None; 
        
        let impl_output = def_from_impl.generate_trait_definition()
            .expect("Failed to generate definition from impl");
        assert_code_eq(&trait_output, &impl_output.to_string());
        let expected = r#"
            pub trait Database {
                type Client = DbClient;
                type Error = DbError;

                // The 'connect' constructor is replaced by 'new_client'
                fn new_client(&self, client: &Self::Client) -> Result<(), Self::Error>;

                // Methods are transformed (injecting &self, client, and Result)
                fn query(&self, client: &Self::Client, sql: String) -> Result<String, Self::Error>;
                async fn execute(&self, client: &Self::Client, sql: String) -> Result<u64, Self::Error>;
            }
        "#;
        assert_code_eq(&trait_output, expected);
    }


    #[test]
    fn test_trait_generation_consistency_between_source_and_impl_no_client() {
        // 1. Define the Original Trait
        // This is what the user defines in the shared library.
        let trait_code = quote! {
            trait Database {
                type Error = DbError;

                // Methods
                fn query(sql: String) -> Result<String, DbError>;
                async fn execute(sql: String) -> Result<u64, DbError>;
            }
        };

        // 2. Define the Implementation
        // This is what the user writes on the host side.
        let impl_code = quote! {
            impl Database for PostgresDriver {
                type Error = DbError;

                fn query(&self, sql: String) -> Result<String, DbError> {
                    Ok("row".to_string())
                }

                async fn execute(&self, sql: String) -> Result<u64, DbError> {
                    Ok(1)
                }
            }
        };

        // 3. Generate Trait Definition from the TRAIT source
        let def_from_trait = CapabilityDefTrait::from_trait(trait_code)
            .expect("Failed to parse trait");
        let trait_output = def_from_trait.generate_trait_definition()
            .expect("Failed to generate definition from trait");
        let mut def_from_impl = CapabilityDefTrait::from_impl(impl_code)
            .expect("Failed to parse impl");
        
        // HACK: Force state_name to None to bypass the guard for this specific comparison test
        def_from_impl.state_name = None; 
        
        let impl_output = def_from_impl.generate_trait_definition()
            .expect("Failed to generate definition from impl");

        assert_code_eq(&trait_output, &impl_output.to_string());
    
        let expected = r#"
            pub trait Database {
                type Error = DbError;
                fn query(&self, sql: String) -> Result<String, Self::Error>;
                async fn execute(&self, sql: String) -> Result<u64, Self::Error>;
            }
        "#;
        assert_code_eq(&trait_output, expected);
    }
}
}