use heck::AsSnakeCase;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::visit_mut::{self, VisitMut};
use syn::{
    Error, Expr, ExprStruct, FnArg, GenericArgument, Ident, ImplItem, ImplItemFn, ItemImpl, ItemTrait, Member, Meta, Path, PathArguments, Result, ReturnType, Token, TraitItem, TraitItemFn, Type, TypePath, parse_quote, parse2
};

use crate::capability_ffi::{CapabilityFuncFFI, InputParams};

#[derive(Debug)]
pub struct ClientConstructor {
    pub sig: syn::Signature,
    pub block: syn::Block,
    pub client: Type,
    pub client_name: String,
    pub inputs: Vec<(Ident, Type)>,
    pub error_type: Option<Type>,
}

impl ClientConstructor {
    pub fn new(
        method: &TraitItemFn, 
        explicit_client_type: &Type,
        explicit_error_type: Option<&Type>,
    ) -> syn::Result<Self> {
        let sig = &method.sig;

        // 1. Check for Body (Must be a "full function")
        let block = match &method.default {
            Some(b) => b.clone(),
            None => {
                return Err(Error::new_spanned(
                    &sig.ident,
                    "Client constructors must have a body (default implementation) defined in the trait.",
                ));
            }
        };

        // 2. Check for &self
        for input in &sig.inputs {
            if let FnArg::Receiver(rec) = input {
                return Err(Error::new_spanned(
                    rec,
                    "Client constructors must be static functions (cannot take 'self').",
                ));
            }
        }

        // 3. STRICT Validation of Return Type
        // The ORIGINAL function must ALWAYS return the ClientType (not a Result).
        if let ReturnType::Type(_, ty) = &sig.output {
            let ty_str = quote!(#ty).to_string();
            let client_str = quote!(#explicit_client_type).to_string();
            
            // Allow explicit type (MyClient) OR generic alias (Self::Client)
            let is_valid_return = ty_str == client_str || ty_str == "Self :: Client" || ty_str == "Self";

            if !is_valid_return {
                return Err(Error::new_spanned(
                    ty,
                    format!("Client constructor must return the defined Client type '{}' directly (do not return Result).", client_str),
                ));
            }
        } else {
             return Err(Error::new_spanned(
                &sig.output,
                "Client constructor must return a value (the Client).",
            ));
        }

        // 4. Extract inputs for FFI metadata
        let mut clean_inputs = Vec::new();
        for input in &sig.inputs {
            if let FnArg::Typed(pat_type) = input {
                let ty = &pat_type.ty;
                if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                    clean_inputs.push((pat_ident.ident.clone(), *ty.clone()));
                }
            }
        }

        // 5: Get the client name
        let client_name = if let Type::Path(type_path) = explicit_client_type {
            type_path.path.segments.last().unwrap().ident.to_string()
        } else {
            quote!(#explicit_client_type).to_string()
        };

        Ok(Self {
            sig: sig.clone(),
            block,
            client: explicit_client_type.clone(),
            client_name,
            inputs: clean_inputs,
            error_type: explicit_error_type.cloned(),
        })
    }

    pub fn client_method_generation(&self, trait_name: &Ident, state_name: &Ident) -> TokenStream {
        let sig = &self.sig;
        let name = &sig.ident;

        // 1. Clone the block
        let mut modified_block = self.block.clone();

        // 2. Initialize visitor with the specific client name
        let mut injector = ConfigBufInjector {
            target_ident: self.client_name.clone(),
        };
        
        // 3. Run the visitor (Injects __config_buf)
        injector.visit_block_mut(&mut modified_block);

        // 4. Build FFI Metadata
        // Determine Final Return Type
        let final_return_type: Type = if let Some(err_type) = &self.error_type {            
            parse_quote!(Result<Self, #err_type>)
        } else {
            parse_quote!(Self)
        };
        let ffi = client_constructor_ffi_meta(trait_name, &state_name, final_return_type, self.sig.asyncness.is_some());

        // 5. Generate the implementation
        // We reconstruct the signature because the return type might change.
        let inputs = &sig.inputs;
        let generics = &sig.generics;
        let where_clause = &sig.generics.where_clause;
        let final_return_type = &ffi.return_type;

        let wasm_call = ffi.generate_module_function();
        
        let logic = quote! {
            let mut new_self = (|| #modified_block )();

            new_self.__config_buf = ::rkyv::to_bytes::<_, 256>(&new_self)
                .expect("Failed to serialize config")
                .into_vec();

            #wasm_call
        };

        quote! {
            pub fn #name #generics (#inputs) -> #final_return_type #where_clause {
                #logic
            }
        }
    }
}

/// Helper to construct the CapabilityFuncFFI configuration for constructors.
/// Encapsulates naming conventions and return type logic.
fn client_constructor_ffi_meta(
    trait_name: &Ident,
    state_name: &Ident,
    final_return_type: Type,
    is_async: bool,
) -> CapabilityFuncFFI {
    CapabilityFuncFFI {
        // Updated Path: __{trait}_{state}_new_client
        library: format_ident!("__{}_{}_new_client", trait_name, state_name), 
        fn_name: format_ident!("new_client"), 
        // Host/Wasm names aligned with library path
        fn_ffi_name: format_ident!("__{}_{}_new_client_ffi", trait_name, state_name),
        fn_wasm_name: format_ident!("__{}_{}_new_client_wasm", trait_name, state_name),
        vis: syn::Visibility::Public(parse_quote!(pub)),
        is_async,
        return_type: final_return_type,
        input: None,
        client: Some(format_ident!("Self")),
        server: None, 
        has_self: false,
    }
}

pub struct ConfigBufInjector {
    /// The specific identifier of the client struct (e.g., "MyClient")
    pub target_ident: String,
}

impl ConfigBufInjector {
    fn is_target_struct(&self, path: &Path) -> bool {
        // 1. Check for `Self`
        if path.is_ident("Self") {
            return true;
        }

        // 2. Check for matching struct name (Last Segment Matching)
        if let Some(last_segment) = path.segments.last() {
            return last_segment.ident.to_string() == self.target_ident;
        }

        false
    }
}

impl VisitMut for ConfigBufInjector {
    fn visit_expr_struct_mut(&mut self, node: &mut ExprStruct) {
        // Visit children first
        visit_mut::visit_expr_struct_mut(self, node);

        // Check if this is the struct we want to modify
        if !self.is_target_struct(&node.path) {
            return;
        }

        // Check if field already exists
        let has_buf = node.fields.iter().any(|f| {
            if let Member::Named(ident) = &f.member {
                ident == "__config_buf"
            } else {
                false
            }
        });

        // Inject
        if !has_buf {
            node.fields.push(parse_quote! {
                __config_buf: std::vec::Vec::new()
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fmt::assert_code_eq; 
    use syn::{parse_quote, Type, TraitItemFn};
    use quote::ToTokens;
    use super::super::ClientConstructor;

    /// Helper to create a ClientConstructor from raw code
    fn create_constructor(
        func_code: TokenStream, 
        client_type_str: &str,
        error_type_str: Option<&str>
    ) -> ClientConstructor {
        let method: TraitItemFn = syn::parse2(func_code).expect("Failed to parse fn");
        let client_type: Type = syn::parse_str(client_type_str).expect("Failed to parse type");
        let error_type: Option<Type> = error_type_str.map(|s| syn::parse_str(s).expect("Failed to parse error"));
        
        ClientConstructor::new(&method, &client_type, error_type.as_ref()).expect("Constructor creation failed")
    }

    #[test]
    fn test_constructor_no_error_type() {
        // 1. Define Input
        let code = quote! {
            fn new(id: u32) -> MyClient {
                MyClient { id, val: 10 }
            }
        };

        // 2. Parse
        let ctor = create_constructor(code, "MyClient", None);
        let trait_name = format_ident!("MyTrait");
        let state_name = format_ident!("MyState");

        // 3. Generate
        let output = ctor.client_method_generation(&trait_name, &state_name);

        // 4. Expected Output:
        // - Library path includes Client name: "__MyTrait_MyState_new_client"
        let expected = r#"
            pub fn new(id: u32) -> Self {
                let mut new_self = (|| {
                    MyClient {
                        id,
                        val: 10,
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
                    |
                        client_state_ptr: *const u8,
                        client_state_len: usize,
                        input_ptr: *const u8,
                        input_len: usize|
                    {
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
        "#;

        assert_code_eq(&output, expected);
    }

    #[test]
    fn test_constructor_with_error_type() {
        // 1. Define Input
        let code = quote! {
            fn create(name: String) -> MyClient {
                MyClient { name }
            }
        };

        // 2. Parse
        let ctor = create_constructor(code, "MyClient", Some("MyError"));
        let trait_name = format_ident!("MyTrait");
        let state_name = format_ident!("MyState");

        // 3. Generate
        let output = ctor.client_method_generation(&trait_name, &state_name);

        // 4. Expected Output:
        // - Library path: "__MyTrait_MyClient_new_client"
        let expected = r#"
            pub fn create(name: String) -> Result<Self, MyError> {
                let mut new_self = (|| {
                    MyClient {
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
                    "__MyTrait_MyState_new_client",
                    Some(client),
                    None,
                    |
                        client_state_ptr: *const u8,
                        client_state_len: usize,
                        input_ptr: *const u8,
                        input_len: usize|
                    {
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
        "#;

        assert_code_eq(&output, expected);
    }

    #[test]
    fn test_validation_rejects_result_in_signature() {
        // User definition tries to return Result - Should be Rejected
        let code = quote! {
            fn new() -> Result<MyClient, String> {
                Ok(MyClient {})
            }
        };
        let method: TraitItemFn = syn::parse2(code).unwrap();
        let client_type: Type = syn::parse_str("MyClient").unwrap();
        
        let res = ClientConstructor::new(&method, &client_type, None);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("must return the defined Client type 'MyClient' directly"));
    }

    #[test]
    fn test_complex_injection_logic() {
        // Ensure nesting works and Self aliases work
        let code = quote! {
            fn build(x: usize, y: usize) -> Self::Client {
                let z = x + y;
                let c = {
                    Self { z }
                };
                c
            }
        };

        let ctor = create_constructor(code, "MyClient", None);
        let trait_name = format_ident!("MyTrait");
        let state_name = format_ident!("MyState");
        let output = ctor.client_method_generation(&trait_name, &state_name);

        let expected = r#"
            pub fn build(x: usize, y: usize) -> Self {
                let mut new_self = (|| {
                    let z = x + y;
                    let c = {
                        Self {
                            z,
                            __config_buf: std::vec::Vec::new(),
                        }
                    };
                    c
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
                    |
                        client_state_ptr: *const u8,
                        client_state_len: usize,
                        input_ptr: *const u8,
                        input_len: usize|
                    {
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
        "#;
        
        assert_code_eq(&output, expected);
    }
}