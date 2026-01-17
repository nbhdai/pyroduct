use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{
    Error, Ident, ImplItem, ImplItemConst, ImplItemType, ItemImpl, ItemTrait, PathArguments,
    Result, ReturnType, TraitItem, TraitItemConst, TraitItemType, Type, parse_quote,
};

mod constructors;
use constructors::{ClientConstructor, client_constructor_ffi_meta};
mod methods;
use methods::CapabilityMethod;

use crate::capability_ffi::CapabilityFuncFFI;

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
    pub state_name: Ident,
    pub from_impl: bool,
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
    pub fn from_trait(input: ItemTrait) -> syn::Result<Self> {
        let trait_name = input.ident.clone();

        // --- 1. Attribute Processing ---
        let mut state_name: Option<Ident> = None;
        let mut original_attrs = Vec::new();

        for attr in input.attrs {
            if attr.path().is_ident("capability_provider") {
                // Parse #[capability_provider(MyState)]
                state_name = Some(attr.parse_args::<Ident>().map_err(|e| {
                    syn::Error::new_spanned(
                        &attr,
                        format!("Invalid capability_provider format. Expected #[capability_provider(StateStructName)]. Error: {}", e)
                    )
                })?);
            } else {
                original_attrs.push(attr);
            }
        }

        let state_name = state_name.ok_or(syn::Error::new_spanned(
            &trait_name,
            "Missing required attribute: #[capability_provider(StateStructName)]. \
                This is required to generate the correct client-side FFI bindings.",
        ))?;

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
                        return Err(syn::Error::new_spanned(
                            ty,
                            "Missing Error type, need a default = ...;",
                        ));
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
                                        "Client type cannot use qualified paths (e.g. <Type as Trait>::Assoc).",
                                    ));
                                }
                                for segment in &type_path.path.segments {
                                    if !matches!(segment.arguments, PathArguments::None) {
                                        return Err(syn::Error::new_spanned(
                                            client_type,
                                            "Client type cannot have generic arguments or lifetimes (e.g., Client<T>).",
                                        ));
                                    }
                                }
                            }
                            _ => {
                                return Err(syn::Error::new_spanned(
                                    client_type,
                                    "Client type must be a simple struct path. References, pointers, or arrays are not allowed.",
                                ));
                            }
                        }
                        explicit_client_type = Some(client_type.clone());
                        other_items.push(item.clone());
                    } else {
                        return Err(syn::Error::new_spanned(
                            ty,
                            "Missing Client type, need a default = ...;",
                        ));
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
                    let ctor =
                        ClientConstructor::new(method, client_type, explicit_error_type.as_ref())?;
                    constructors.push(ctor);
                } else {
                    let cap_method = CapabilityMethod::from_trait(
                        method.clone(),
                        explicit_client_type.as_ref(),
                        explicit_error_type.as_ref(),
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
                    "A 'Client' type is defined, so you must provide at least one Client Constructor (static method returning Self::Client).",
                ));
            }
        } else {
            if !constructors.is_empty() {
                return Err(syn::Error::new(
                    Span::call_site(),
                    "Client Constructors defined but no 'Client' type found.",
                ));
            }
        }

        Ok(Self {
            original_attrs,
            generics: input.generics,
            methods,
            trait_name,
            state_name,
            from_impl: false,
            constructors,
            other_items,
            explicit_error_type,
            explicit_client_type,
        })
    }

    /// Ingests a Trait Implementation (`impl Trait for State`).
    pub fn from_impl(input: &ItemImpl) -> syn::Result<Self> {
        // 1. Extract Trait Name
        let trait_name = match &input.trait_ {
            Some((_, path, _)) => path.segments.last().unwrap().ident.clone(),
            None => {
                return Err(Error::new_spanned(
                    input,
                    "Capability definition must be a trait implementation (impl Trait for State).",
                ));
            }
        };

        // 2. Extract State Name
        let state_name = if let Type::Path(type_path) = &*input.self_ty {
            type_path.path.segments.last().unwrap().ident.clone()
        } else {
            return Err(Error::new_spanned(
                input.self_ty.clone(),
                "Implementation target must be a struct (TypePath).",
            ));
        };

        let mut methods = Vec::new();
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
                                return Err(syn::Error::new_spanned(
                                    client_type,
                                    "Client type cannot use qualified paths.",
                                ));
                            }
                            for segment in &type_path.path.segments {
                                if !matches!(segment.arguments, PathArguments::None) {
                                    return Err(syn::Error::new_spanned(
                                        client_type,
                                        "Client type cannot have generic arguments.",
                                    ));
                                }
                            }
                        }
                        _ => {
                            return Err(syn::Error::new_spanned(
                                client_type,
                                "Client type must be a simple struct path.",
                            ));
                        }
                    }
                    explicit_client_type = Some(client_type.clone());
                    other_items.push(impl_type_to_trait_type(ty));
                }
                ImplItem::Fn(_) => {}
                ImplItem::Const(c) => other_items.push(impl_const_to_trait_const(c)),
                _ => {} // Ignore macros etc.
            }
        }

        let mut has_new_client = false;
        // 4. Second Pass: Methods
        for item in &input.items {
            if let ImplItem::Fn(impl_method) = item {
                // In Host implementations, we look for 'new_client' to satisfy the constructor requirement.
                // We do NOT use ClientConstructor::from_impl.
                if impl_method.sig.ident == "new_client" {
                    has_new_client = true;
                } else {
                    let cap_method = CapabilityMethod::from_impl(
                        impl_method,
                        explicit_client_type.as_ref(),
                        explicit_error_type.as_ref(),
                    )?;
                    methods.push(cap_method);
                }
            }
        }

        // 5. Final Validation (this is pointless as the rust won't compile with an incorrect trait definition).
        if explicit_client_type.is_some() && !has_new_client {
            return Err(syn::Error::new(
                Span::call_site(),
                "Client type defined, but no Client Constructor ('new_client') found in implementation.",
            ));
        }
        if explicit_client_type.is_none() && has_new_client {
            return Err(syn::Error::new(
                Span::call_site(),
                "Client Constructors defined but no 'Client' type found.",
            ));
        }

        Ok(Self {
            original_attrs: input.attrs.clone(),
            generics: input.generics.clone(),
            methods,
            trait_name,
            state_name,
            from_impl: true,
            constructors: Vec::new(),
            other_items,
            explicit_error_type,
            explicit_client_type,
        })
    }

    /// Generates the final TokenStream for the Trait definition.
    pub fn generate_trait_definition(&self) -> Result<TokenStream> {
        if self.from_impl {
            return Err(Error::new_spanned(
                &self.trait_name,
                "Unable to generate a trait definition from an impl.",
            ));
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
        if self.from_impl {
            return Err(Error::new_spanned(
                &self.trait_name,
                "Unable to generate a client definition from an impl.",
            ));
        }
        let client_type = match &self.explicit_client_type {
            Some(ty) => ty,
            None => return Ok(TokenStream::new()),
        };

        let (impl_generics, _, where_clause) = self.generics.split_for_impl();
        let trait_name = &self.trait_name;

        let capability_methods = self
            .methods
            .iter()
            .map(|m| m.client_method_generation(trait_name, &self.state_name));

        let constructors = self
            .constructors
            .iter()
            .map(|c| c.client_method_generation(trait_name, &self.state_name));

        Ok(quote! {
            impl #impl_generics #client_type #where_clause {
                #(#constructors)*

                #(#capability_methods)*
            }
        })
    }

    pub fn capability_ffis(&self) -> Vec<CapabilityFuncFFI> {
        let trait_name = &self.trait_name;

        let constructor_ffi = client_constructor_ffi_meta(
            trait_name,
            &self.state_name,
            self.explicit_error_type.as_ref(),
            false,
        );
        let mut capability_ffis = Vec::with_capacity(self.methods.len() + 1);
        capability_ffis.push(constructor_ffi);

        capability_ffis.extend(self
            .methods
            .iter()
            .map(|m| m.build_ffi_meta(trait_name, &self.state_name)));
        capability_ffis
    }
}

// --- Converters: ImplItem -> TraitItem ---

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
    use quote::format_ident;
    use syn::parse2;

    use super::*;
    use crate::fmt::{assert_code_eq, assert_code_eq_token};

    #[test]
    fn test_generate_client_impl_integration() {
        let code = parse2(quote! {
            #[capability_provider(MyState)]
            trait MyTrait {
                type Client = MyClient;

                fn new(id: u32) -> MyClient {
                    MyClient { id }
                }

                fn get_info() -> u32;
            }
        }).unwrap();

        let constructor = parse2(quote! {
            fn new(id: u32) -> MyClient {
                MyClient { id }
            }
        }).unwrap();

        let method = parse2(quote! {
            fn get_info() -> u32;
        }).unwrap();
        let client_name = syn::parse_str("MyClient").unwrap();
        let state_name = format_ident!("MyState");
        let trait_name = format_ident!("MyTrait");

        let expected_constructor = ClientConstructor::new(&constructor, &client_name, None).unwrap();
        let expected_method = CapabilityMethod::from_trait(method, Some(&client_name), None).unwrap();

        let expected_constructor = expected_constructor.client_method_generation(&trait_name, &state_name);
        let expected_method = expected_method.client_method_generation(&trait_name, &state_name);

        let def = CapabilityDefTrait::from_trait(code).expect("Failed to parse capability trait");
        let output = def.generate_client_impl().unwrap();

        let expected = quote! {
            impl MyClient {
                #expected_constructor
                #expected_method
            }
        };

        assert_code_eq_token(&output, &expected);
    }

    #[test]
    fn test_generate_client_impl_with_error_and_input_structs() {
        let code = parse2(quote! {
            #[capability_provider(MyState)]
            trait AdvancedTrait {
                type Client = AdvancedClient;
                type Error = MyError;

                fn create(name: String) -> AdvancedClient {
                    AdvancedClient { name }
                }

                fn process(val: u32, flag: bool) -> u32;
            }
        }).unwrap();

        let constructor = parse2(quote! {
            fn create(name: String) -> AdvancedClient {
                AdvancedClient { name }
            }
        }).unwrap();

        let method = parse2(quote! {
            fn process(val: u32, flag: bool) -> u32;
        }).unwrap();

        let client_name: Type = syn::parse_str("AdvancedClient").unwrap();
        let error_name: Type = syn::parse_str("MyError").unwrap();
        let state_name = format_ident!("MyState");
        let trait_name = format_ident!("AdvancedTrait");

        let expected_constructor = ClientConstructor::new(&constructor, &client_name, Some(&error_name)).unwrap();
        let expected_method = CapabilityMethod::from_trait(method, Some(&client_name), Some(&error_name)).unwrap();

        let expected_constructor = expected_constructor.client_method_generation(&trait_name, &state_name);
        let expected_method = expected_method.client_method_generation(&trait_name, &state_name);

        let def = CapabilityDefTrait::from_trait(code).expect("Failed to parse capability trait");
        let output = def.generate_client_impl().unwrap();

        let expected = quote! {
            impl AdvancedClient {
                #expected_constructor
                #expected_method
            }
        };

        assert_code_eq_token(&output, &expected);
    }

    #[test]
    fn test_no_client_impl_generated_if_no_client_type() {
        let code = parse2(quote! {
            #[capability_provider(MyState)]
            trait PureInterface {
                fn do_thing();
            }
        }).unwrap();

        let def = CapabilityDefTrait::from_trait(code).expect("Failed to parse capability trait");
        let output = def.generate_client_impl().unwrap();
        assert!(output.is_empty());
    }

    #[test]
    fn test_server_trait_generation() {
        let code = parse2(quote! {
            #[capability_provider(MyState)]
            trait SensorFeature {
                type Client = SensorClient;
                type Error = SensorError;

                fn new(id: String) -> SensorClient {
                    SensorClient { id }
                }

                fn calibrate(offset: i32, scale: f32) -> bool;

                async fn read_async(timeout: u32) -> f64;
            }
        }).unwrap();
        let def = CapabilityDefTrait::from_trait(code).expect("Failed to parse capability trait");

        let server_trait = def.generate_trait_definition().unwrap();

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
    }

    #[test]
    fn test_generate_server_trait_correctness() {
        let code = parse2(quote! {
            #[capability_provider(MyState)]
            trait ControlPlane {
                type Client = ControlClient;
                type Error = ControlError;

                fn new(api_key: String) -> ControlClient {
                    ControlClient { api_key }
                }

                fn sync_op(level: u8) -> bool;

                async fn async_op(data: Vec<u8>);
            }
        }).unwrap();

        let def = CapabilityDefTrait::from_trait(code).expect("Failed to parse capability trait");
        let output = def
            .generate_trait_definition()
            .expect("Failed to generate trait definition");

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
        let code = parse2(quote! {
            impl MyTrait for MyState {
                type Client = MyClient;
                type Error = MyError;

                fn new_client(&self, client: &MyClient) -> Result<(), Self::Error> {
                    Ok(())
                }

                fn do_thing(&self, client: &MyClient) -> Result<u32, Self::Error> {
                    Ok(42)
                }
            }
        }).unwrap();

        let def = CapabilityDefTrait::from_impl(&code).expect("Failed to parse impl");

        // Verify extracted names
        assert_eq!(def.trait_name.to_string(), "MyTrait");
        assert_eq!(def.state_name.to_string(), "MyState");

        // Verify Client Implementation Generation is forbidden
        let client_impl_result = def.generate_client_impl();
        assert_eq!(
            client_impl_result.unwrap_err().to_string(),
            "Unable to generate a client definition from an impl."
        );

        // Verify that Trait Definition generation is explicitly forbidden for Impl inputs
        let trait_gen_result = def.generate_trait_definition();
        assert!(
            trait_gen_result.is_err(),
            "Should not be able to generate a trait definition from an impl block"
        );
        assert_eq!(
            trait_gen_result.unwrap_err().to_string(),
            "Unable to generate a trait definition from an impl."
        );
    }

    #[test]
    fn test_trait_generation_consistency_between_source_and_impl() {
        let trait_code = parse2(quote! {
            #[capability_provider(MyState)]
            trait Database {
                type Client = DbClient;
                type Error = DbError;

                fn connect(url: String) -> DbClient {
                    DbClient { url }
                }

                fn query(sql: String) -> String;
                async fn execute(sql: String) -> u64;
            }
        }).unwrap();

        let impl_code = parse2(quote! {
            impl Database for PostgresDriver {
                type Client = DbClient;
                type Error = DbError;

                fn new_client(&self, client: &DbClient) -> Result<(), DbError> {
                    Ok(())
                }

                fn query(&self, client: &DbClient, sql: String) -> Result<String, DbError> {
                    Ok("row".to_string())
                }

                async fn execute(&self, client: &DbClient, sql: String) -> Result<u64, DbError> {
                    Ok(1)
                }
            }
        }).unwrap();

        let def_from_trait =
            CapabilityDefTrait::from_trait(trait_code).expect("Failed to parse trait");
        let trait_output = def_from_trait
            .generate_trait_definition()
            .expect("Failed to generate definition from trait");

        let mut def_from_impl =
            CapabilityDefTrait::from_impl(&impl_code).expect("Failed to parse impl");

        // HACK: Force from_impl flag to false to bypass the guard for this specific comparison test
        def_from_impl.from_impl = false;

        let impl_output = def_from_impl
            .generate_trait_definition()
            .expect("Failed to generate definition from impl");

        // Verify they are identical
        assert_code_eq_token(&trait_output, &impl_output);

        // Verify the content is what we expect
        let expected = r#"
            pub trait Database {
                type Client = DbClient;
                type Error = DbError;

                fn new_client(&self, client: &Self::Client) -> Result<(), Self::Error>;

                fn query(&self, client: &Self::Client, sql: String) -> Result<String, Self::Error>;
                async fn execute(&self, client: &Self::Client, sql: String) -> Result<u64, Self::Error>;
            }
        "#;
        assert_code_eq(&trait_output, expected);
    }

    #[test]
    fn test_trait_generation_consistency_between_source_and_impl_no_client() {
        let trait_code = parse2(quote! {
            #[capability_provider(MyState)]
            trait Database {
                type Error = DbError;

                fn query(sql: String) -> String;
                async fn execute(sql: String) -> u64;
            }
        }).unwrap();

        let impl_code = parse2(quote! {
            impl Database for PostgresDriver {
                type Error = DbError;

                fn query(&self, sql: String) -> Result<String, DbError> {
                    Ok("row".to_string())
                }

                async fn execute(&self, sql: String) -> Result<u64, DbError> {
                    Ok(1)
                }
            }
        }).unwrap();

        let def_from_trait =
            CapabilityDefTrait::from_trait(trait_code).expect("Failed to parse trait");
        let trait_output = def_from_trait
            .generate_trait_definition()
            .expect("Failed to generate definition from trait");
        let mut def_from_impl =
            CapabilityDefTrait::from_impl(&impl_code).expect("Failed to parse impl");

        // HACK: Force from_impl flag to false to bypass the guard for this specific comparison test
        def_from_impl.from_impl = false;

        let impl_output = def_from_impl
            .generate_trait_definition()
            .expect("Failed to generate definition from impl");

        assert_code_eq_token(&trait_output, &impl_output);

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