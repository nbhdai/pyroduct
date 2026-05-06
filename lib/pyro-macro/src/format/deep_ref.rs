use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{GenericArgument, Ident, ItemStruct, Path, PathArguments, Type, TypePath};

/// Main function to generate the Deep Reference struct and implementation.
/// Accepts a list of additional derives to apply to the generated struct.
pub fn deep_ref(
    input: &ItemStruct,
    import_location: &Path,
    derives_to_pass: &Vec<Ident>,
) -> syn::Result<TokenStream> {
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "DeepRef cannot be derived for structs with generic parameters (types, lifetimes, or consts)",
        ));
    }

    let struct_name = &input.ident;
    let ref_struct_name = format_ident!("{}Ref", struct_name);

    // ItemStruct stores fields directly
    let fields = &input.fields;

    // 1. Generate the Reference Struct Definition
    let mut lifetime_used = false;
    let ref_fields = fields
        .iter()
        .map(|f| {
            let name = &f.ident;
            let vis = &f.vis;
            let ty = &f.ty;
            let (mapped_type, is_primitive) = map_type_to_ref(ty);

            if !is_primitive {
                lifetime_used = true;
            }

            quote! { #vis #name: #mapped_type }
        })
        .collect::<Vec<_>>();

    let phantom_field = if !lifetime_used {
        quote! { _phantom: std::marker::PhantomData<&'a ()> }
    } else {
        quote! {}
    };

    // Inject the user-provided derives here
    let struct_def = quote! {
        #[derive(#(#derives_to_pass),*)]
        pub struct #ref_struct_name<'a> {
            #(#ref_fields,)*
            #phantom_field
        }
    };

    // 2. Generate the DeepRef Implementation for the Owned type
    let field_conversions = fields.iter().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        let ty = &f.ty;
        generate_field_conversion(field_name, ty)
    });

    let phantom_init = if !lifetime_used {
        quote! { _phantom: std::marker::PhantomData }
    } else {
        quote! {}
    };

    let impl_owned = quote! {
        impl #import_location::format::DeepRef for #struct_name {
            type Ref<'a> = #ref_struct_name<'a>;

            fn as_deep_ref(&self) -> Self::Ref<'_> {
                #ref_struct_name {
                    #(#field_conversions,)*
                    #phantom_init
                }
            }
        }
    };

    // 3. Generate FromRef<DeepRef> Implementation (borrowed ref → owned)
    let from_ref_conversions: Vec<_> = fields.iter().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        let ty = &f.ty;
        generate_from_ref_conversion(field_name, ty, import_location)
    }).collect();

    let impl_from_ref = quote! {
        impl<'a> #import_location::format::FromRef<#ref_struct_name<'a>> for #struct_name {
            fn from_ref(reference: &#ref_struct_name<'a>) -> Self {
                Self {
                    #(#from_ref_conversions,)*
                }
            }
        }
    };

    Ok(quote! {
        #struct_def
        #impl_owned
        #impl_from_ref
    })
}

// -------------------------------------------------------------------------
// Helper Functions
// -------------------------------------------------------------------------

// Map Owned types to Borrowed types for the struct definition
pub(crate) fn map_type_to_ref(ty: &Type) -> (TokenStream, bool) {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            let segment = path.segments.last().unwrap();
            let ident_str = segment.ident.to_string();

            // Check if it is a wrapper around `str` (Arc<str>, Box<str>, etc)
            if is_string_like(ty) {
                return (quote! { &'a str }, false);
            }

            match ident_str.as_str() {
                "bool" | "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64"
                | "usize" | "f16" | "f32" | "f64" => {
                    let ident = &segment.ident;
                    (quote! { #ident }, true)
                }
                "Vec" => {
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                            let (inner_ref, is_prim) = map_type_to_ref(inner_ty);
                            if is_prim {
                                return (quote! { &'a [#inner_ref] }, false);
                            } else {
                                // For complex types, we return Vec<Ref>
                                return (quote! { Vec<#inner_ref> }, false);
                            }
                        }
                    }
                    (quote! { Vec<()> }, false)
                }
                "Option" => {
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                            // Recursively map the inner type

                            // 1. If inner is primitive (i32), Option<i32> is Copy (effectively),
                            //    so we keep Option<i32>.
                            if is_primitive(inner_ty) {
                                return (quote! { Option<#inner_ty> }, true);
                            }

                            // 2. If inner is string-like (String, Arc<str>), we want Option<&'a str>
                            if is_string_like(inner_ty) {
                                return (quote! { Option<&'a str> }, false);
                            }

                            // 3. Otherwise, map normally (e.g., Option<MyStruct> -> Option<MyStructRef>)
                            let (inner_ref, _) = map_type_to_ref(inner_ty);
                            return (quote! { Option<#inner_ref> }, false);
                        }
                    }
                    (quote! { Option<()> }, false)
                }
                // Nested struct - assume it has a Ref variant
                other => {
                    let ref_name = format_ident!("{}Ref", other);
                    (quote! { #ref_name<'a> }, false)
                }
            }
        }
        _ => (quote! { () }, true),
    }
}

// Generate the conversion logic for as_deep_ref (Owned -> Borrowed)
fn generate_field_conversion(field_name: &Ident, ty: &Type) -> TokenStream {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            let segment = path.segments.last().unwrap();
            let ident_str = segment.ident.to_string();

            // Direct String-like types (Arc<str>, String, Box<str>) -> &str
            if is_string_like(ty) {
                // String: .as_str()
                // Arc<str>: &**self.field or .as_ref()
                if ident_str == "String" {
                    return quote! { #field_name: self.#field_name.as_str() };
                } else {
                    return quote! { #field_name: &self.#field_name };
                }
            }

            match ident_str.as_str() {
                // Primitives: Copy
                "bool" | "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64"
                | "usize" | "f16" | "f32" | "f64" => {
                    quote! { #field_name: self.#field_name }
                }

                // Vec
                "Vec" => {
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                            if is_primitive(inner_ty) {
                                // Primitive vec: borrow as slice
                                quote! { #field_name: self.#field_name.as_slice() }
                            } else if is_string_like(inner_ty) {
                                // Vec<String>, Vec<Arc<str>> -> Vec<&str>
                                if ident_str == "String" || is_string(inner_ty) {
                                    quote! { #field_name: self.#field_name.iter().map(|x| x.as_str()).collect() }
                                } else {
                                    quote! { #field_name: self.#field_name.iter().map(|x| x.as_ref()).collect() }
                                }
                            } else {
                                // Complex vec: map to Vec<Ref>
                                quote! {
                                    #field_name: self.#field_name.iter().map(|x| x.as_deep_ref()).collect()
                                }
                            }
                        } else {
                            quote! { #field_name: vec![] }
                        }
                    } else {
                        quote! { #field_name: vec![] }
                    }
                }

                // Option
                "Option" => {
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                            if is_primitive(inner_ty) {
                                // Option<i32>: Copy
                                quote! { #field_name: self.#field_name }
                            } else if is_string_like(inner_ty) {
                                // Option<String>, Option<Arc<str>> -> .as_deref() gives Option<&str>
                                quote! { #field_name: self.#field_name.as_deref() }
                            } else {
                                // Option<Struct> -> map(as_deep_ref)
                                quote! { #field_name: self.#field_name.as_ref().map(|x| x.as_deep_ref()) }
                            }
                        } else {
                            quote! { #field_name: None }
                        }
                    } else {
                        quote! { #field_name: None }
                    }
                }

                // Nested Structs
                _ => {
                    quote! { #field_name: self.#field_name.as_deep_ref() }
                }
            }
        }
        _ => quote! { #field_name: self.#field_name.as_deep_ref() },
    }
}

// Generate the conversion logic for from_ref (Borrowed -> Owned)
// Uses FromRef::from_ref() for all conversions to maintain consistency
// with the FromRef trait system.
fn generate_from_ref_conversion(field_name: &Ident, ty: &Type, import_location: &Path) -> TokenStream {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            let segment = path.segments.last().unwrap();
            let ident = &segment.ident;
            let ident_str = ident.to_string();

            if is_string_like(ty) {
                if ident_str == "String" {
                    return quote! { #field_name: reference.#field_name.to_string() };
                } else if matches!(ident_str.as_str(), "Arc" | "Box" | "Rc") {
                    return quote! { #field_name: #import_location::format::FromRef::from_ref(*reference.#field_name) };
                } else if ident_str == "Cow" {
                    return quote! { #field_name: std::borrow::Cow::Owned(reference.#field_name.to_string()) };
                }
            }

            match ident_str.as_str() {
                "bool" | "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64"
                | "usize" | "f16" | "f32" | "f64" => {
                    quote! { #field_name: reference.#field_name }
                }
                "Vec" => {
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                            if is_primitive(inner_ty) {
                                quote! { #field_name: reference.#field_name.to_vec() }
                            } else if is_string_like(inner_ty) {
                                if is_string(inner_ty) {
                                    quote! { #field_name: reference.#field_name.iter().map(|x| x.to_string()).collect() }
                                } else {
                                    let inner_ident =
                                        if let Type::Path(TypePath { path, .. }) = inner_ty {
                                            path.segments.last().unwrap().ident.to_string()
                                        } else {
                                            "".to_string()
                                        };
                                    if matches!(inner_ident.as_str(), "Arc" | "Box" | "Rc") {
                                        quote! { #field_name: reference.#field_name.iter().map(|x| #import_location::format::FromRef::from_ref(*x)).collect() }
                                    } else {
                                        quote! { #field_name: reference.#field_name.iter().map(|x| #import_location::format::FromRef::from_ref(*x)).collect() }
                                    }
                                }
                            } else {
                                quote! {
                                    #field_name: reference.#field_name.iter().map(|x| #import_location::format::FromRef::from_ref(x.clone())).collect()
                                }
                            }
                        } else {
                            quote! { #field_name: vec![] }
                        }
                    } else {
                        quote! { #field_name: vec![] }
                    }
                }
                "Option" => {
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                            if is_primitive(inner_ty) {
                                quote! { #field_name: reference.#field_name }
                            } else if is_string_like(inner_ty) {
                                if is_string(inner_ty) {
                                    quote! { #field_name: reference.#field_name.map(|x| x.to_string()) }
                                } else {
                                    let inner_ident =
                                        if let Type::Path(TypePath { path, .. }) = inner_ty {
                                            path.segments.last().unwrap().ident.to_string()
                                        } else {
                                            "".to_string()
                                        };
                                    if matches!(inner_ident.as_str(), "Arc" | "Box" | "Rc") {
                                        quote! { #field_name: reference.#field_name.map(|x| #import_location::format::FromRef::from_ref(x)) }
                                    } else {
                                        quote! { #field_name: reference.#field_name.map(|x| #import_location::format::FromRef::from_ref(x)) }
                                    }
                                }
                            } else {
                                quote! { #field_name: reference.#field_name.as_ref().map(|x| #import_location::format::FromRef::from_ref(x.clone())) }
                            }
                        } else {
                            quote! { #field_name: None }
                        }
                    } else {
                        quote! { #field_name: None }
                    }
                }
                _ => {
                    quote! { #field_name: #import_location::format::FromRef::from_ref(reference.#field_name.as_deep_ref()) }
                }
            }
        }
        _ => {
            quote! { #field_name: #import_location::format::FromRef::from_ref(reference.#field_name.as_deep_ref()) }
        }
    }
}

// Helper to identify primitives
fn is_primitive(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        let ident = path.segments.last().unwrap().ident.to_string();
        matches!(
            ident.as_str(),
            "i8" | "i16"
                | "i32"
                | "i64"
                | "isize"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "usize"
                | "f16"
                | "f32"
                | "f64"
                | "bool"
        )
    } else {
        false
    }
}

fn is_string(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        let ident = path.segments.last().unwrap().ident.to_string();
        ident == "String"
    } else {
        false
    }
}

// Helper to identify "String-like" types that should become &'a str
// Covers: String, Arc<str>, Box<str>, Cow<str>
fn is_string_like(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        let segment = path.segments.last().unwrap();
        let ident = segment.ident.to_string();

        // 1. Simple String
        if ident == "String" {
            return true;
        }

        // 2. Wrappers (Arc, Box, Cow)
        if matches!(ident.as_str(), "Arc" | "Box" | "Cow" | "Rc") {
            if let PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                    // Check if inner is "str"
                    if let Type::Path(TypePath {
                        path: inner_path, ..
                    }) = inner_ty
                    {
                        if let Some(inner_seg) = inner_path.segments.last() {
                            return inner_seg.ident == "str";
                        }
                    }
                }
            }
        }
    }
    false
}
