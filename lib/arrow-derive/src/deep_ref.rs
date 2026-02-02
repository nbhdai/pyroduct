use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Data, DeriveInput, GenericArgument, Path, PathArguments, Type, TypePath, parse_macro_input,
};

pub fn derive_with_path(input: TokenStream, import_location: Path) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    if !input.generics.params.is_empty() {
        return syn::Error::new_spanned(
            &input.generics,
            "DeepRef cannot be derived for structs with generic parameters (types, lifetimes, or consts)"
        )
        .to_compile_error()
        .into();
    }

    let struct_name = input.ident.clone();
    let ref_struct_name = format_ident!("{}Ref", struct_name);

    let fields = match input.data {
        Data::Struct(ref data) => &data.fields,
        _ => panic!("DeepRef can only be derived for structs"),
    };

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

    let struct_def = quote! {
        #[derive(Debug, Clone, PartialEq)]
        pub struct #ref_struct_name<'a> {
            #(#ref_fields,)*
            #phantom_field
        }
    };

    // 2. Generate the DeepRef Implementation
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
        impl #import_location::DeepRef for #struct_name {
            type Ref<'a> = #ref_struct_name<'a>;

            fn as_deep_ref(&self) -> Self::Ref<'_> {
                #ref_struct_name {
                    #(#field_conversions,)*
                    #phantom_init
                }
            }
        }
    };

    let expanded = quote! {
        #struct_def
        #impl_owned
    };

    TokenStream::from(expanded)
}

// Map Owned types to Borrowed types for the struct definition
pub(crate) fn map_type_to_ref(ty: &Type) -> (proc_macro2::TokenStream, bool) {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            let segment = path.segments.last().unwrap();
            let ident_str = segment.ident.to_string();

            match ident_str.as_str() {
                "String" => (quote! { &'a str }, false),
                "bool" | "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f16"
                | "f32" | "f64" => {
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
                            let (inner_ref, is_prim) = map_type_to_ref(inner_ty);
                            return (quote! { Option<#inner_ref> }, is_prim);
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
fn generate_field_conversion(field_name: &syn::Ident, ty: &Type) -> proc_macro2::TokenStream {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            let segment = path.segments.last().unwrap();
            let ident_str = segment.ident.to_string();

            match ident_str.as_str() {
                // Primitives: Copy
                "bool" | "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f16"
                | "f32" | "f64" => {
                    quote! { #field_name: self.#field_name }
                }

                // String: Borrow as &str
                "String" => {
                    quote! { #field_name: self.#field_name.as_str() }
                }

                // Vec
                "Vec" => {
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                            if is_primitive(inner_ty) {
                                // Primitive vec: borrow as slice
                                quote! { #field_name: self.#field_name.as_slice() }
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
                                quote! { #field_name: self.#field_name }
                            } else if is_string(inner_ty) {
                                quote! { #field_name: self.#field_name.as_deref() }
                            } else {
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

// Helper to identify primitives
fn is_primitive(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        let ident = path.segments.last().unwrap().ident.to_string();
        matches!(
            ident.as_str(),
            "i8" | "i16"
                | "i32"
                | "i64"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
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
