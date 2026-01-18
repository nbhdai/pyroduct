use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{
    Ident, ImplItemConst, ImplItemType, ItemTrait,
    ReturnType, TraitItem, TraitItemConst, TraitItemType, Type,
};

use super::constructors::{ClientConstructor, client_constructor_ffi_meta};
use super::methods::CapabilityMethod;

use crate::ffi::CapabilityFuncFFI;
use crate::utils::extract_ident_from_type;

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
#[derive(Debug, Clone, PartialEq)]
pub struct CapabilityDefTrait {
    pub trait_name: Ident,
    pub state_name: Ident,
    pub client_name: Ident,
    pub original_attrs: Vec<syn::Attribute>,
    pub generics: syn::Generics,
    pub methods: Vec<CapabilityMethod>,
    pub constructors: Vec<ClientConstructor>,
    pub other_items: Vec<TraitItem>,
    pub explicit_error_type: Option<Type>,
}

impl CapabilityDefTrait {
    /// Ingests a Trait Definition (original logic).
    pub fn from_trait(input: ItemTrait, state_name: Ident) -> syn::Result<Self> {
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
                        let client_type = extract_ident_from_type(client_type)?;
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
        let client_name = if let Some(explicit_client_type) = explicit_client_type {
            explicit_client_type
        } else {
            return Err(syn::Error::new_spanned(
                input,
                "Missing Client type",
            ));
        };

        // 3. Final Validation
        if constructors.is_empty() {
            return Err(syn::Error::new(
                Span::call_site(),
                "You must provide at least one Client Constructor (static method returning Self::Client).",
            ));
        }

        Ok(Self {
            original_attrs: input.attrs.clone(),
            generics: input.generics,
            methods,
            trait_name,
            state_name,
            client_name,
            constructors,
            other_items,
            explicit_error_type,
        })
    }

    /// Generates the final TokenStream for the Trait definition.
    pub fn generate_trait_definition(&self) -> TokenStream {
        let trait_name = &self.trait_name;
        let attrs = &self.original_attrs;
        let generics = &self.generics;
        let other_items = &self.other_items;
        let method_signatures = self.methods.iter().map(|m| m.trait_method_generation());
        let new_client_method = self.generate_new_client_signature();

        quote! {
            #(#attrs)*
            pub trait #trait_name #generics {
                #(#other_items)*
                #new_client_method
                #(#method_signatures)*
            }
        }
    }

    /// Helper to determine the signature of `new_client`
    fn generate_new_client_signature(&self) -> TokenStream {
        let client_name = &self.client_name;
        if let Some(error) = &self.explicit_error_type {
            quote! {
                fn new_client(&self, client: &#client_name) -> Result<(), #error>;
            }
        } else {
            quote! {
                fn new_client(&self, client: &#client_name) -> ();
            }
        }
    }

    /// Generates the `impl ClientType { ... }` block.
    pub fn generate_client_impl(&self) -> TokenStream {
        let client_name = &self.client_name;


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

        quote! {
            impl #impl_generics #client_name #where_clause {
                #(#constructors)*

                #(#capability_methods)*
            }
        }
    }

    pub fn capability_ffis(&self) -> Vec<CapabilityFuncFFI> {
        let trait_name = &self.trait_name;
        let mut capability_ffis = Vec::with_capacity(self.methods.len() + 1);
        let constructor_ffi = client_constructor_ffi_meta(
            trait_name,
            &self.state_name,
            self.explicit_error_type.as_ref(),
            false,
        );
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
    use crate::fmt::assert_code_eq_token;

    #[test]
    fn test_generate_client_impl_integration() {
        let code = parse2(quote! {
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

        let def = CapabilityDefTrait::from_trait(code, state_name.clone()).expect("Failed to parse capability trait");
        let output = def.generate_client_impl();

        let expected = quote! {
            impl MyClient {
                #expected_constructor
                #expected_method
            }
        };

        assert_code_eq_token(&output, &expected);

        // Test FFI generation
        let ffis = def.capability_ffis();
        assert_eq!(ffis.len(), 2, "Should have 2 FFIs: 1 constructor + 1 method");
        
        // Verify constructor FFI
        assert_eq!(ffis[0].library.to_string(), "__my_trait__my_state__new_client");
        assert_eq!(ffis[0].fn_name.to_string(), "new_client");
        assert_eq!(ffis[0].fn_ffi_name.to_string(), "__my_trait__my_state__new_client__ffi");
        assert_eq!(ffis[0].fn_wasm_name.to_string(), "__my_trait__my_state__new_client__wasm");
        
        // Verify method FFI
        assert_eq!(ffis[1].library.to_string(), "__my_trait__my_state__get_info");
        assert_eq!(ffis[1].fn_name.to_string(), "get_info");
        assert_eq!(ffis[1].fn_ffi_name.to_string(), "__my_trait__my_state__get_info__ffi");
        assert_eq!(ffis[1].fn_wasm_name.to_string(), "__my_trait__my_state__get_info__wasm");
    }

    #[test]
    fn test_generate_client_impl_with_error_and_input_structs() {
        let code = parse2(quote! {
            trait AdvancedTrait {
                type Client = AdvancedClient;
                type Error = MyError;

                fn create(name: String) -> AdvancedClient {
                    AdvancedClient { name }
                }

                fn create_2(name: String) -> AdvancedClient {
                    AdvancedClient { name }
                }

                async fn process(val: u32, flag: bool) -> Result<u32, MyError>;
                fn sync_op(level: u8) -> Result<bool, MyError>;
            }
        }).unwrap();

        let constructor = parse2(quote! {
            fn create(name: String) -> AdvancedClient {
                AdvancedClient { name }
            }
        }).unwrap();

        let constructor_2 = parse2(quote! {
            fn create_2(name: String) -> AdvancedClient {
                AdvancedClient { name }
            }
        }).unwrap();

        let method = parse2(quote! {
            async fn process(val: u32, flag: bool) -> Result<u32, MyError>;
        }).unwrap();
        let method_2 = parse2(quote! {
            fn sync_op(level: u8) -> Result<bool, MyError>;
        }).unwrap();

        let client_name = format_ident!("AdvancedClient");
        let error_name: Type = syn::parse_str("MyError").unwrap();
        let state_name = format_ident!("MyState");
        let trait_name = format_ident!("AdvancedTrait");

        let expected_constructor = ClientConstructor::new(&constructor, &client_name, Some(&error_name)).unwrap();
        let expected_constructor_2 = ClientConstructor::new(&constructor_2, &client_name, Some(&error_name)).unwrap();
        let expected_method = CapabilityMethod::from_trait(method, Some(&client_name), Some(&error_name)).unwrap();
        let expected_method_2 = CapabilityMethod::from_trait(method_2, Some(&client_name), Some(&error_name)).unwrap();

        let expected_constructor = expected_constructor.client_method_generation(&trait_name, &state_name);
        let expected_constructor_2 = expected_constructor_2.client_method_generation(&trait_name, &state_name);
        let expected_method = expected_method.client_method_generation(&trait_name, &state_name);
        let expected_method_2 = expected_method_2.client_method_generation(&trait_name, &state_name);

        let def = CapabilityDefTrait::from_trait(code, state_name.clone()).expect("Failed to parse capability trait");
        let output = def.generate_client_impl();

        let expected = quote! {
            impl AdvancedClient {
                #expected_constructor
                #expected_constructor_2
                #expected_method
                #expected_method_2
            }
        };

        assert_code_eq_token(&output, &expected);

        // Test FFI generation
        let ffis = def.capability_ffis();
        assert_eq!(ffis.len(), 3, "Should have 2 FFIs: 1 constructor + 2 methods");
        
        // Verify constructor FFI
        assert_eq!(ffis[0].library.to_string(), "__advanced_trait__my_state__new_client");
        assert_eq!(ffis[0].fn_name.to_string(), "new_client");
        assert_eq!(ffis[0].fn_ffi_name.to_string(), "__advanced_trait__my_state__new_client__ffi");
        assert_eq!(ffis[0].fn_wasm_name.to_string(), "__advanced_trait__my_state__new_client__wasm");
        
        // Verify method FFI
        assert_eq!(ffis[1].library.to_string(), "__advanced_trait__my_state__process");
        assert_eq!(ffis[1].fn_name.to_string(), "process");
        assert_eq!(ffis[1].fn_ffi_name.to_string(), "__advanced_trait__my_state__process__ffi");
        assert_eq!(ffis[1].fn_wasm_name.to_string(), "__advanced_trait__my_state__process__wasm");

        let server_trait = def.generate_trait_definition();
        let expected_trait = quote! {
            pub trait AdvancedTrait {
                type Client = AdvancedClient;
                type Error = MyError;
                fn new_client(&self, client: &Self::Client) -> Result<(), MyError>;
                async fn process(&self, client: &Self::Client, val: u32, flag: bool) -> Result<u32, MyError>;
                fn sync_op(&self, client: &Self::Client, level: u8) -> Result<bool, MyError>;
            }
        };
        assert_code_eq_token(&server_trait, &expected_trait);
    }

    #[test]
    fn test_no_client_impl_generated_if_no_client_type() {
        let code = parse2(quote! {
            trait PureInterface {
                fn do_thing();
            }
        }).unwrap();
        let state_name = format_ident!("MyState");

        let def = CapabilityDefTrait::from_trait(code, state_name).expect("Failed to parse capability trait");
        let output = def.generate_client_impl();
        assert!(output.is_empty());
    }
}