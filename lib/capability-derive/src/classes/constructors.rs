use std::rc::Rc;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::visit_mut::{self, VisitMut};
use syn::{Error, Expr, ExprStruct, FnArg, Ident, Member, Path, ReturnType, TraitItemFn, parse_quote, parse2};

use crate::ffi::CapabilityFuncFFI;
use crate::paths::ClassIdent;
use crate::utils::type_to_return;

#[derive(Debug, Clone, PartialEq)]
pub struct ClientConstructor {
    pub capability_name: Rc<str>,
    pub sig: syn::Signature,
    pub block: syn::Block,
    pub class: Rc<ClassIdent>,
}

impl ClientConstructor {
    pub fn new(method: &TraitItemFn, class: &Rc<ClassIdent>, capability_name: &Rc<str>) -> syn::Result<Self> {
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
        let client = &class.client_tn;

        // 3. STRICT Validation of Return Type
        // The ORIGINAL function must ALWAYS return the ClientType (not a Result).
        if let ReturnType::Type(_, ty) = &sig.output {
            let ty_str = quote!(#ty).to_string();
            let client_str = quote!(#client).to_string();

            // Allow explicit type (MyClient) OR generic alias (Self::Client)
            let is_valid_return =
                ty_str == client_str || ty_str == "Self :: Client" || ty_str == "Self";

            if !is_valid_return {
                return Err(Error::new_spanned(
                    ty,
                    format!(
                        "Client constructor must return the defined Client type '{}' directly (do not return Result).",
                        client_str
                    ),
                ));
            }
        } else {
            return Err(Error::new_spanned(
                &sig.output,
                "Client constructor must return a value (the Client).",
            ));
        }

        Ok(Self {
            capability_name: capability_name.clone(),
            sig: sig.clone(),
            block,
            class: class.clone(),
        })
    }

    pub fn client_method_generation(&self, module: Option<&Ident>) -> TokenStream {
        let sig = &self.sig;
        let name = &sig.ident;

        // 1. Clone the block
        let mut modified_block = self.block.clone();

        // 2. Initialize visitor with the specific client name
        let mut injector: ConfigBufInjector = ConfigBufInjector {
            target_ident: self.class.client_tn.to_string(),
        };

        // 3. Run the visitor (Injects __config_buf)
        injector.visit_block_mut(&mut modified_block);

        // 4. Build FFI Metadata
        // Determine Final Return Type

        let ffi = client_constructor_ffi_meta(&self.class, self.sig.asyncness.is_some(), &self.capability_name);

        // 5. Generate the implementation
        // We reconstruct the signature because the return type might change.
        let inputs = &sig.inputs;
        let generics = &sig.generics;
        let where_clause = &sig.generics.where_clause;
        let output = &self.class.client_tn;

        let final_return_type = if let Some(error_type) = &self.class.error_tn {
            quote! { Result<#output, #error_type> }
        } else {
            quote! { #output }
        };

        let wasm_call = ffi.generate_wasm_call(module);
        
        // Constructors on the client side return Self, but the FFI returns () or Result<(), Error>.
        // We need to handle the FFI result if it exists (for error propagation) but the WASM call 
        // generation logic in FFI handles the conversion if we set up the FFI meta correctly.
        
        let result_handle = if self.class.error_tn.is_some() {
            quote! {
                let ffi_result = #wasm_call;
                match ffi_result {
                        Ok(_) => Ok(new_self),
                        Err(e) => Err(e.into()),
                    }
            }
        } else {
            quote! {
                #wasm_call;
                new_self
            }
        };

        let logic = quote! {
            let mut new_self = (|| #modified_block )();

            new_self.__config_buf = ::rkyv::to_bytes::<::rkyv::rancor::Error>(&new_self)
                .expect("Failed to serialize config")
                .into_vec();


            #result_handle
        };

        quote! {
            pub fn #name #generics (#inputs) #final_return_type #where_clause {
                #logic
            }
        }
    }
}

/// Helper to construct the CapabilityFuncFFI configuration for constructors.
/// Encapsulates naming conventions and return type logic.
pub fn client_constructor_ffi_meta(class: &Rc<ClassIdent>, is_async: bool, capability_name: &Rc<str>) -> CapabilityFuncFFI {
    // For the FFI of a constructor, the return type is Result<(), Error> or ().
    // The "Client" is passed as an argument, so the host doesn't return it.
    let return_type: ReturnType = if let Some(err_type) = &class.error_tn {
        type_to_return(&parse2(quote!(Result<(), #err_type>)).expect("This really should parse"))
    } else {
        type_to_return(&parse2(quote!(())).expect("This really should parse"))
    };
    
    CapabilityFuncFFI {
        capability_name: capability_name.clone(),
        class: Some(class.clone()),
        constructor: true,
        fn_name: format_ident!("new_client"),
        vis: syn::Visibility::Public(parse_quote!(pub)),
        is_async,
        return_type,
        input: None,
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
    fn visit_expr_mut(&mut self, node: &mut Expr) {
        match node {
            // FIX: Handle Unit Struct usage (e.g., `SimpleClient`) by converting to Struct Init
            Expr::Path(p) if self.is_target_struct(&p.path) => {
                let path = &p.path;
                *node = parse_quote! {
                    #path {
                        __config_buf: std::vec::Vec::new()
                    }
                };
            }
            Expr::Struct(s) => {
                self.visit_expr_struct_mut(s);
            }
            _ => {
                visit_mut::visit_expr_mut(self, node);
            }
        }
    }

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
    use syn::{TraitItemFn, Type};

    /// Helper to create a ClientConstructor from raw code
    fn create_constructor(
        func_code: TokenStream,
        error_type_str: Option<&str>,
    ) -> ClientConstructor {
        let method: TraitItemFn = syn::parse2(func_code).expect("Failed to parse fn");
        let error_type: Option<Type> =
            error_type_str.map(|s| syn::parse_str(s).expect("Failed to parse error"));

        let class = Rc::new(ClassIdent {
            trait_tn: format_ident!("MyTrait"),
            state_tn: format_ident!("MyServer"),
            client_tn: format_ident!("MyClient"),
            error_tn: error_type,
        });

        ClientConstructor::new(&method, &class, &"cap".into()).expect("Constructor creation failed")
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
        let ctor = create_constructor(code, None);

        // 3. Generate Expected FFI Call
        let wasm_call = client_constructor_ffi_meta(&ctor.class, false, &"cap".into()).generate_wasm_call(None);

        // 4. Generate
        let output = ctor.client_method_generation(None);

        // 5. Expected Output
        let expected = quote! {
            pub fn new(id: u32) -> MyClient {
                let mut new_self = (|| {
                    MyClient {
                        id,
                        val: 10,
                        __config_buf: std::vec::Vec::new(),
                    }
                })();
                new_self.__config_buf = ::rkyv::to_bytes::<::rkyv::rancor::Error>>(&new_self).expect("Failed to serialize config").into_vec();

                #wasm_call;
                new_self
            }
        };

        crate::fmt::assert_code_eq_token(&output, &expected);
    }
    
    #[test]
    fn test_constructor_unit_struct_rewrite() {
        // 1. Define Input: Using Unit Struct syntax "MyClient"
        let code = quote! {
            fn default() -> MyClient {
                MyClient
            }
        };
        
        let ctor = create_constructor(code, None);
        let output = ctor.client_method_generation(None);
        
        // Expected: MyClient is replaced with MyClient { __config_buf: ... }
        let expected_snippet = quote! {
            let mut new_self = (|| {
                MyClient {
                    __config_buf: std::vec::Vec::new()
                }
            })();
        };
        
        let actual_str = output.to_string();
        let expected_str = expected_snippet.to_string();
        
        // Weak assertion: check if the string contains the logic
        assert!(actual_str.contains("__config_buf : std :: vec :: Vec :: new ()"));
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
        let ctor = create_constructor(code, Some("MyError"));

        // 3. Generate Expected FFI Call
        let wasm_call = client_constructor_ffi_meta(&ctor.class, false, &"cap".into()).generate_wasm_call(None);

        // 4. Generate
        let output = ctor.client_method_generation(None);

        // 5. Expected Output
        // Note: The signature returns Result<Self, MyError>
        let expected = quote! {
            pub fn create(name: String) -> Result<MyClient, MyError> {
                let mut new_self = (|| {
                    MyClient {
                        name,
                        __config_buf: std::vec::Vec::new(),
                    }
                })();
                new_self.__config_buf = ::rkyv::to_bytes::<::rkyv::rancor::Error>>(&new_self)
                    .expect("Failed to serialize config")
                    .into_vec();

                let ffi_result = #wasm_call;

                match ffi_result {
                    Ok(_) => Ok(new_self),
                    Err(e) => Err(e.into()),
                }
            }
        };

        crate::fmt::assert_code_eq_token(&output, &expected);
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
        let class = Rc::new(ClassIdent {
            trait_tn: format_ident!("MyTrait"),
            state_tn: format_ident!("MyServer"),
            client_tn: format_ident!("MyClient"),
            error_tn: None,
        });

        let res = ClientConstructor::new(&method, &class, &"cap".into());
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("must return the defined Client type 'MyClient' directly")
        );
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

        let ctor = create_constructor(code, None);

        // Generate FFI call for expectation
        let wasm_call = client_constructor_ffi_meta(&ctor.class, false, &"cap".into()).generate_wasm_call(None);

        let output = ctor.client_method_generation(None);

        let expected = quote! {
            pub fn build(x: usize, y: usize) -> MyClient {
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
                new_self.__config_buf = ::rkyv::to_bytes::<::rkyv::rancor::Error>>(&new_self)
                    .expect("Failed to serialize config")
                    .into_vec();

                #wasm_call;
                new_self
            }
        };

        crate::fmt::assert_code_eq_token(&output, &expected);
    }
}
