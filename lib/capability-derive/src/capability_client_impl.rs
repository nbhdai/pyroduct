use heck::AsSnakeCase;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse2, parse_quote, Error, FnArg, GenericArgument, Ident, ImplItem, ImplItemFn, ItemImpl,
    PathArguments, ReturnType, Token, TraitItem, TraitItemFn, Type, TypePath,
};

use crate::capability_ffi::{CapabilityFuncFFI, InputParams};
use crate::utils::{get_param_name, get_param_type};

// ==============================================================================
// 1. Helpers & Analysis
// ==============================================================================

/// Enum to classify the return type of a potential constructor.
#[derive(Debug)]
pub enum ConstructorReturnKind {
    /// Returns `Self` or `MyStruct`
    Direct,
    /// Returns `Result<Self, E>` or `Result<MyStruct, E>`, holding the type `E`
    WrappedResult(Type),
    /// Invalid return type for a constructor
    Invalid,
}

/// Analyzes a return type to determine if it points to `Self` or `Result<Self, E>`.
fn analyze_constructor_return(ret: &ReturnType, struct_name: &Ident) -> ConstructorReturnKind {
    let ty = match ret {
        ReturnType::Type(_, ty) => ty,
        ReturnType::Default => return ConstructorReturnKind::Invalid,
    };

    if let Type::Path(TypePath { path, .. }) = &**ty {
        if let Some(segment) = path.segments.last() {
            // Case 1: Direct Return (Self or StructName)
            if segment.ident == "Self" || segment.ident == *struct_name {
                return ConstructorReturnKind::Direct;
            }

            // Case 2: Result Wrapper (Result<Self, Error>)
            if segment.ident == "Result" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    // Check first argument is Self/Struct
                    if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                        let is_self = if let Type::Path(tp) = inner_ty {
                            if let Some(inner_seg) = tp.path.segments.last() {
                                inner_seg.ident == "Self" || inner_seg.ident == *struct_name
                            } else {
                                false
                            }
                        } else {
                            false
                        };

                        if is_self {
                            // Check for the second argument (The Error Type)
                            if args.args.len() >= 2 {
                                if let GenericArgument::Type(error_ty) = &args.args[1] {
                                    return ConstructorReturnKind::WrappedResult(error_ty.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    ConstructorReturnKind::Invalid
}

/// Extracts the simple struct name from the impl block type (e.g., `impl MyClient`).
fn get_simple_struct_ident(ty: &Type) -> syn::Result<&Ident> {
    if let Type::Path(TypePath { qself: None, path }) = ty {
        if path.segments.len() == 1 {
            let segment = path.segments.first().unwrap();
            if segment.arguments.is_empty() {
                return Ok(&segment.ident);
            }
        }
    }
    Err(Error::new_spanned(
        ty,
        "Expected a simple static struct name (e.g. 'MyClient').",
    ))
}

/// Shared logic to parse any method signature into FFI capability metadata.
fn parse_method_to_ffi(
    method: &ImplItemFn,
    library: &Ident,
    client_struct_name: Option<&Ident>,
) -> syn::Result<CapabilityFuncFFI> {
    let fn_name = &method.sig.ident;
    let is_async = method.sig.asyncness.is_some();

    // 1. Collect Parameters (excluding self)
    let mut params = Vec::new();
    for arg in &method.sig.inputs {
        match arg {
            FnArg::Receiver(_) => continue, // Skip &self
            _ => {
                let name = get_param_name(arg)
                    .ok_or_else(|| Error::new_spanned(arg, "Could not determine parameter name"))?;
                let ty = get_param_type(arg)
                    .ok_or_else(|| Error::new_spanned(arg, "Could not determine parameter type"))?;
                params.push((name, ty.clone()));
            }
        }
    }

    // 2. Determine FFI Input Shape
    let input = match params.len() {
        0 => None,
        1 => {
            let (name, ty) = params.into_iter().next().unwrap();
            Some(InputParams::One(name, ty))
        }
        _ => {
            let input_struct_name = format_ident!("__{}_Input", fn_name);
            Some(InputParams::Many {
                params,
                input_struct_name,
            })
        }
    };

    // 3. Determine Return Type
    let return_type = match &method.sig.output {
        ReturnType::Default => parse_quote!(()),
        ReturnType::Type(_, ty) => (**ty).clone(),
    };

    // 4. Construct FFI Metadata
    let lib_snake = AsSnakeCase(library.to_string()).to_string();
    let fn_snake = AsSnakeCase(fn_name.to_string()).to_string();

    Ok(CapabilityFuncFFI {
        library: format_ident!("__{}__{}__func", lib_snake, fn_snake),
        fn_name: fn_name.clone(),
        fn_ffi_name: format_ident!("__{}_ffi", fn_snake),
        fn_wasm_name: format_ident!("__{}_wasm", fn_snake),
        vis: method.vis.clone(),
        is_async,
        return_type,
        input,
        client: client_struct_name.cloned(),
        server: None, // Used in server macros, not client
        has_self: method.sig.inputs.iter().any(|arg| matches!(arg, FnArg::Receiver(_))),
    })
}

// ==============================================================================
// 2. Struct Definitions
// ==============================================================================

/// Represents a standard instance method (takes &self).
pub struct ClientMethod {
    pub method: ImplItemFn,
    pub ffi: CapabilityFuncFFI,
}

impl ClientMethod {
    pub fn new(method: ImplItemFn, struct_name: &Ident, library: &Ident) -> syn::Result<Self> {
        // Validation 1: Async Check
        if method.sig.asyncness.is_some() {
            return Err(Error::new_spanned(
                &method.sig.asyncness,
                "Async functions are not allowed in capability_client_impl. Use sync functions only."
            ));
        }

        // Validation 2: Receiver Check
        let receiver = method.sig.inputs.iter().find_map(|arg| {
            if let FnArg::Receiver(r) = arg { Some(r) } else { None }
        });

        match receiver {
            Some(r) => {
                if r.reference.is_none() {
                    return Err(Error::new_spanned(
                        r, 
                        "Value receivers (self) are not allowed. Use shared references (&self) only."
                    ));
                }
                if r.mutability.is_some() {
                    return Err(Error::new_spanned(
                        r, 
                        "Mutable receivers (&mut self) are not allowed in capability_client_impl. Use shared references (&self) only."
                    ));
                }
            },
            None => return Err(Error::new_spanned(&method.sig, "Instance method requires `&self`")),
        }

        let ffi = parse_method_to_ffi(&method, library, Some(struct_name))?;
        Ok(Self { method, ffi })
    }
}

/// Represents a constructor method (returns Self, static).
pub struct ClientConstructor {
    pub method: ImplItemFn,
    pub ffi: CapabilityFuncFFI,
    pub return_kind: ConstructorReturnKind,
}

impl ClientConstructor {
    pub fn new(
        mut method: ImplItemFn, 
        library: &Ident, 
        return_kind: ConstructorReturnKind
    ) -> syn::Result<Self> {
        
        // Validation: Must NOT have receiver
        if method.sig.inputs.iter().any(|arg| matches!(arg, FnArg::Receiver(_))) {
            return Err(Error::new_spanned(&method.sig, "Constructors cannot take `self`"));
        }

        let ffi = parse_method_to_ffi(&method, library, None)?;
        
        // Constructors need their bodies modified immediately to support config serialization
        Self::inject_config_serialization(&mut method, &return_kind);

        Ok(Self { method, ffi, return_kind })
    }

    /// Modifies the AST of the function body to serialize configuration after creation.
    fn inject_config_serialization(method: &mut ImplItemFn, kind: &ConstructorReturnKind) {
        let old_block = &method.block;

        method.block = match kind {
            ConstructorReturnKind::WrappedResult(_) => parse_quote! {
                {
                    let result = (|| #old_block)();
                    match result {
                        Ok(mut new_self) => {
                            // Serialize self to bytes and store in __config_buf
                            new_self.__config_buf = ::rkyv::to_bytes::<_, 256>(&new_self)
                                .expect("Failed to serialize config").into_vec();
                            Ok(new_self)
                        }
                        Err(e) => Err(e),
                    }
                }
            },
            ConstructorReturnKind::Direct => parse_quote! {
                {
                    let mut new_self = (|| #old_block)();
                    new_self.__config_buf = ::rkyv::to_bytes::<_, 256>(&new_self)
                        .expect("Failed to serialize config").into_vec();
                    new_self
                }
            },
            ConstructorReturnKind::Invalid => panic!("Attempted to wrap invalid constructor body"),
        };
    }
}

// ==============================================================================
// 3. Main Processor
// ==============================================================================

pub struct CapabilityClientImpl {
    pub original_attrs: Vec<syn::Attribute>,
    pub self_ty: Box<Type>,
    pub generics: syn::Generics,
    pub methods: Vec<ClientMethod>,
    pub constructors: Vec<ClientConstructor>,
    pub other_items: Vec<ImplItem>,
    pub explicit_error_type: Option<Type>,
}

impl CapabilityClientImpl {
    pub fn new(item: TokenStream) -> syn::Result<Self> {
        let input: ItemImpl = parse2(item)?;
        let struct_name = get_simple_struct_ident(&input.self_ty)?.clone();
        
        // In this context, library name is strictly the struct name
        let library = struct_name.clone();

        let mut methods = Vec::new();
        let mut constructors = Vec::new();
        let mut other_items = Vec::new();
        let mut explicit_error_type = None;

        // 1. First Pass: Collect items and identify implicit/explicit Error types
        for item in &input.items {
            match item {
                ImplItem::Fn(method) => {
                    let has_receiver = method.sig.inputs.iter().any(|arg| matches!(arg, FnArg::Receiver(_)));

                    if has_receiver {
                        // It's an instance method
                        methods.push(ClientMethod::new(method.clone(), &struct_name, &library)?);
                    } else {
                        // It's a constructor candidate
                        let return_kind = analyze_constructor_return(&method.sig.output, &struct_name);
                        
                        match return_kind {
                            ConstructorReturnKind::Direct | ConstructorReturnKind::WrappedResult(_) => {
                                constructors.push(ClientConstructor::new(method.clone(), &library, return_kind)?);
                            }
                            ConstructorReturnKind::Invalid => {
                                return Err(Error::new_spanned(
                                    &method.sig.output,
                                    format!("Static method must return `Self` or `Result<Self, Self::Error>` to be a valid capability constructor for `{}`", struct_name)
                                ));
                            }
                        }
                    }
                }
                ImplItem::Type(ty) if ty.ident == "Error" => {
                    // Capture the explicit error type to use in the trait
                    explicit_error_type = Some(ty.ty.clone());
                    // Keep it in other_items so it remains in the generated impl block
                    other_items.push(item.clone());
                }
                // Keep consts, types, macros, etc. as-is
                _ => other_items.push(item.clone()),
            }
        }

        if constructors.is_empty() {
            return Err(Error::new_spanned(
                &input,
                format!("A constructor method is required for `{}` (something that returns Self, or Result<Self,_>)", struct_name)
            ));
        }

        // 2. Second Pass: Enforce Consistency based on `type Error`
        for ctor in &constructors {
            match (&explicit_error_type, &ctor.return_kind) {
                // Case A: No explicit Error type defined
                (None, ConstructorReturnKind::Direct) => {
                    // OK: Direct return (Self) allowed when no Error type exists
                },
                (None, ConstructorReturnKind::WrappedResult(_)) => {
                    // Error: Result return not allowed without explicit Error type
                    return Err(Error::new_spanned(
                        &ctor.method.sig.output,
                        format!(
                            "Constructor `{}` returns a Result, but no `type Error = ...;` is defined in the impl block. Define `type Error` or return `{}` directly.", 
                            ctor.method.sig.ident, 
                            struct_name
                        )
                    ));
                },

                // Case B: Explicit Error type IS defined
                (Some(expected_error), ConstructorReturnKind::Direct) => {
                    // Error: Direct return not allowed when Error type exists
                     return Err(Error::new_spanned(
                        &ctor.method.sig.output,
                        format!(
                            "Constructor `{}` returns `{}` directly, but `type Error = {};` is defined. It must return `Result<{}, {}>`.", 
                            ctor.method.sig.ident,
                            struct_name,
                            expected_error.to_token_stream(),
                            struct_name,
                            expected_error.to_token_stream()
                        )
                    ));
                },
                (Some(expected_error), ConstructorReturnKind::WrappedResult(found_error)) => {
                    // Check if the error types match (using string representation for loose equality)
                    let expected_str = expected_error.to_token_stream().to_string().replace(" ", "");
                    let found_str = found_error.to_token_stream().to_string().replace(" ", "");

                    if expected_str != found_str {
                         return Err(Error::new_spanned(
                            found_error,
                            format!(
                                "Constructor return type mismatch. Expected error type `{}`, but found `{}`.", 
                                expected_error.to_token_stream(),
                                found_error.to_token_stream()
                            )
                        ));
                    }
                    // OK
                },
                
                (_, ConstructorReturnKind::Invalid) => unreachable!("Invalid returns filtered in first pass"),
            }
        }

        Ok(Self {
            original_attrs: input.attrs,
            self_ty: input.self_ty,
            generics: input.generics,
            methods,
            constructors,
            other_items,
            explicit_error_type,
        })
    }

    /// Generates a trait definition corresponding to the client implementation.
    /// The trait contains all constructors and methods defined in the impl block.
    pub fn generate_trait(&self) -> syn::Result<TokenStream> {
        let struct_name = get_simple_struct_ident(&self.self_ty)?;
        let trait_name = format_ident!("{}Trait", struct_name);

        // Use the user-defined Error type if present, otherwise default to ()
        let error_type = self.explicit_error_type.clone()
            .unwrap_or_else(|| parse_quote!(()));

        let mut trait_items = Vec::new();

        // Add the Error type definition at the top of the trait
        trait_items.push(parse_quote! {
            type Error = #error_type;
        });

        if self.explicit_error_type.is_some() {
            trait_items.push(parse_quote! {
                fn new_client(client: &#struct_name) -> Result<(), #error_type>;
            });
        } else {
            trait_items.push(parse_quote! {
                fn new_client(client: &#struct_name);
            });
        }

        // Helper to convert ImplItemFn to TraitItemFn (no body, no visibility, semicolon)
        fn to_trait_item(method: &ImplItemFn) -> TraitItem {
            TraitItem::Fn(TraitItemFn {
                attrs: method.attrs.clone(),
                sig: method.sig.clone(),
                default: None,
                semi_token: Some(Token![;](Span::call_site())),
            })
        }

        // 2. Add methods
        for method in &self.methods {
            trait_items.push(to_trait_item(&method.method));
        }

        Ok(quote! {
            pub trait #trait_name {
                #(#trait_items)*
            }
        })
    }

    pub fn expand(self) -> syn::Result<TokenStream> {
        let trait_def = self.generate_trait()?;
        
        let Self { original_attrs, self_ty, generics, methods, constructors, other_items, .. } = self;

        // Reconstruct the impl block items
        let mut new_items = Vec::new();

        // 1. Add back non-function items (consts, types, including `type Error`)
        new_items.extend(other_items);

        // 2. Add modified constructors
        for ctor in constructors {
            new_items.push(ImplItem::Fn(ctor.method));
        }

        // 3. Add methods
        for method in methods {
            new_items.push(ImplItem::Fn(method.method));
        }

        Ok(quote! {
            #(#original_attrs)*
            impl #generics #self_ty {
                #(#new_items)*
            }

            #trait_def
        })
    }
}

pub fn expand(_attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let client_impl = CapabilityClientImpl::new(item)?;
    client_impl.expand()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fmt::{format_tokens, assert_code_eq};
    use quote::quote;

    /// Tests that a constructor returning `Self` gets the configuration 
    /// serialization logic injected at the end of the block.
    #[test]
    fn test_client_constructor_direct() {
        let item = quote! {
            impl MyClient {
                pub fn new(id: u32) -> Self {
                    let x = id * 2;
                    Self {
                        id,
                        x,
                        __config_buf: vec![]
                    }
                }
            }
        };

        let result = expand(quote!(), item).expect("Expansion failed");

        let expected = r#"
            impl MyClient {
                pub fn new(id: u32) -> Self {
                    let mut new_self = (|| {
                        let x = id * 2;
                        Self {
                            id,
                            x,
                            __config_buf: vec![]
                        }
                    })();
                    new_self.__config_buf = ::rkyv::to_bytes::<_, 256>(&new_self)
                        .expect("Failed to serialize config")
                        .into_vec();
                    new_self
                }
            }
            pub trait MyClientTrait {
                type Error = ();
                fn new_client(client: &MyClient);
            }
        "#;

        assert_code_eq(&result, expected);
    }

    /// Tests that a constructor returning `Result<Self, Error>` works when `type Error` is defined.
    #[test]
    fn test_client_constructor_result() {
        let item = quote! {
            impl MyClient {
                type Error = String;
                pub fn try_new(id: u32) -> Result<Self, String> {
                    if id == 0 { return Err("Bad ID".to_string()); }
                    Ok(Self { id, __config_buf: vec![] })
                }
            }
        };

        let result = expand(quote!(), item).expect("Expansion failed");

        let expected = r#"
            impl MyClient {
                type Error = String;
                pub fn try_new(id: u32) -> Result<Self, String> {
                    let result = (|| {
                        if id == 0 {
                            return Err("Bad ID".to_string());
                        }
                        Ok(Self {
                            id,
                            __config_buf: vec![],
                        })
                    })();
                    match result {
                        Ok(mut new_self) => {
                            new_self
                                .__config_buf = ::rkyv::to_bytes::<_, 256>(&new_self)
                                .expect("Failed to serialize config")
                                .into_vec();
                            Ok(new_self)
                        }
                        Err(e) => Err(e),
                    }
                }
            }
            pub trait MyClientTrait {
                type Error = String;
                fn new_client(client: &MyClient) -> Result<(), String>;
            }
        "#;

        assert_code_eq(&result, expected);
    }

    /// Test Failure: `type Error` is defined, but constructor returns `Self`.
    #[test]
    fn test_fail_defined_error_returned_self() {
        let item = quote! {
            impl MyClient {
                type Error = String;
                pub fn new() -> Self {
                    Self { __config_buf: vec![] }
                }
            }
        };

        let result = expand(quote!(), item);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("is defined. It must return `Result<MyClient, String>`."));
    }

    /// Test Failure: No `type Error` defined, but constructor returns `Result`.
    #[test]
    fn test_fail_no_error_returned_result() {
        let item = quote! {
            impl MyClient {
                pub fn new() -> Result<Self, String> {
                    Ok(Self { __config_buf: vec![] })
                }
            }
        };

        let result = expand(quote!(), item);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no `type Error = ...;` is defined"));
    }

    /// Test Failure: `type Error` mismatch.
    #[test]
    fn test_fail_error_mismatch() {
        let item = quote! {
            impl MyClient {
                type Error = String;
                pub fn new() -> Result<Self, u32> {
                    Ok(Self { __config_buf: vec![] })
                }
            }
        };

        let result = expand(quote!(), item);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Expected error type `String`"));
        assert!(err.contains("found `u32`"));
    }

    /// Tests that standard instance methods (taking &self) are parsed correctly 
    #[test]
    fn test_client_instance_method_passthrough() {
        let item = quote! {
            impl MyClient {
                pub fn new() -> Self { Self }

                pub fn get_data(&self) -> u32 {
                    42
                }
            }
        };

        let result = expand(quote!(), item).expect("Expansion failed");

        let expected = r#"
            impl MyClient {
                pub fn new() -> Self {
                    let mut new_self = (|| { Self })();
                    new_self . __config_buf = :: rkyv :: to_bytes :: < _ , 256 > (& new_self) . expect ("Failed to serialize config") . into_vec ();
                    new_self
                }
                pub fn get_data(&self) -> u32 {
                    42
                }
            }
            pub trait MyClientTrait {
                type Error = ();
                fn new_client(client: &MyClient);
                fn get_data(&self) -> u32;
            }
        "#;

        assert_code_eq(&result, expected);
    }

    /// Tests that invalid constructors (those not returning Self or Result<Self>)
    /// trigger a specific parser error.
    #[test]
    fn test_invalid_constructor_return_type() {
        let item = quote! {
            impl MyClient {
                // This looks like a constructor (no &self) but returns wrong type
                pub fn new() -> u32 {
                    42
                }
            }
        };

        let result = expand(quote!(), item);
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.to_string(), "Static method must return `Self` or `Result<Self, Self::Error>` to be a valid capability constructor for `MyClient`");
    }


    #[test]
    fn test_expand_async_fails() {
        let attr = quote! {};
        let item = quote! {
            impl MyClient {
                pub fn new() -> Self { Self }
                
                pub async fn refresh(&self) -> bool {
                    true
                }
            }
        };

        let result = expand(attr, item);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Async functions are not allowed in capability_client_impl. Use sync functions only.");
    }

    #[test]
    fn test_expand_return_self_fails() {
        let attr = quote! {};
        let item = quote! {
            impl MyClient {
                pub fn new() -> Self { Self }

                pub fn with_option(self, opt: bool) -> Self {
                    self
                }
            }
        };

        let result = expand(attr, item);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert_eq!(err_msg, "Value receivers (self) are not allowed. Use shared references (&self) only.");
    }

    #[tracing_test::traced_test]
    #[test]
    fn test_expand_read_only() {
        let attr = quote! {};
        let item = quote! {
            impl MyClient {
                pub fn new() -> Self { Self }

                pub fn get_id(&self) -> u32 {
                    self.id
                }
            }
        };

        let result = expand(attr, item).expect("Expansion failed");
        tracing::info!("Generated tokens:\n{}", format_tokens(&result));

        let expected = r#"
            impl MyClient {
                pub fn new() -> Self {
                    let mut new_self = (|| { Self })();
                    new_self . __config_buf = :: rkyv :: to_bytes :: < _ , 256 > (& new_self) . expect ("Failed to serialize config") . into_vec ();
                    new_self
                }
                pub fn get_id(&self) -> u32 {
                    self.id
                }
            }
            pub trait MyClientTrait {
                type Error = ();
                fn new_client(client: &MyClient);
                fn get_id(&self) -> u32;
            }
        "#;

        assert_code_eq(&result, expected);
    }

    #[test]
    fn test_expand_mutation_fails() {
        let attr = quote! {};
        let item = quote! {
            impl MyClient {
                pub fn new() -> Self { Self }

                pub fn set_timeout(&mut self, timeout: u64) {
                    self.timeout = timeout;
                }
            }
        };

        let result = expand(attr, item);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Mutable receivers (&mut self) are not allowed in capability_client_impl. Use shared references (&self) only.");
    }

    #[tracing_test::traced_test]
    #[test]
    fn test_expand_returns_other_type() {
        let attr = quote! {};
        let item = quote! {
            impl MyClient {
                type Error = String;
                pub fn new() -> Result<MyClient, String> { Ok(Self) }

                pub fn calculate(&self, x: i32, y: i32) -> i32 {
                    x + y
                }
            }
        };

        let result = expand(attr, item).expect("Expansion failed");
        tracing::info!("Generated tokens:\n{}", format_tokens(&result));

        let expected = r#"
            impl MyClient {
                type Error = String;
                pub fn new() -> Result<MyClient, String> {
                    let result = (|| { Ok(Self) })();
                    match result {
                        Ok(mut new_self) => {
                            new_self
                                .__config_buf = ::rkyv::to_bytes::<_, 256>(&new_self)
                                .expect("Failed to serialize config")
                                .into_vec();
                            Ok(new_self)
                        }
                        Err(e) => Err(e),
                    }
                }
                pub fn calculate(&self, x: i32, y: i32) -> i32 {
                    x + y
                }
            }
            pub trait MyClientTrait {
                type Error = String;
                fn new_client(client: &MyClient) -> Result<(), String>;
                fn calculate(&self, x: i32, y: i32) -> i32;
            }
        "#;

        assert_code_eq(&result, expected);
    }
}