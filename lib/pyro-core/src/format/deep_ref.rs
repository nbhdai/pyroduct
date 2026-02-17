use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    GenericArgument, Ident, ItemStruct, Path, PathArguments, Type, TypePath,
};

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

    Ok(quote! {
        #struct_def
        #impl_owned
    })
}

/// Specialized function for Rkyv.
/// Generates the standard DeepRef stuff, PLUS an implementation for the Archived variant.
pub fn deep_ref_rkyv(input: &ItemStruct, import_location: &Path) -> syn::Result<TokenStream> {
    let struct_name = &input.ident;
    // Standard rkyv naming convention: Archived + StructName
    let archived_struct_name = format_ident!("Archived{}", struct_name);
    let ref_struct_name = format_ident!("{}Ref", struct_name);

    // 2. Generate the conversions for the Archived fields
    // We iterate over the *original* fields to determine types, but generate logic
    // that assumes we are operating on the *Archived* struct.
    let fields = &input.fields;
    let rkyv_field_conversions = fields.iter().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        let ty = &f.ty;
        generate_rkyv_field_conversion(field_name, ty)
    });

    // Determine if we need phantom data init (same logic as main function)
    let mut lifetime_used = false;
    for f in fields {
        let (_, is_prim) = map_type_to_ref(&f.ty);
        if !is_prim {
            lifetime_used = true;
            break;
        }
    }

    let phantom_init = if !lifetime_used {
        quote! { _phantom: std::marker::PhantomData }
    } else {
        quote! {}
    };

    // 3. Generate the DeepRef implementation for the Archived struct
    let impl_archived = quote! {
        impl #import_location::DeepRef for #archived_struct_name {
            type Ref<'a> = #ref_struct_name<'a>;

            fn as_deep_ref(&self) -> Self::Ref<'_> {
                #ref_struct_name {
                    #(#rkyv_field_conversions,)*
                    #phantom_init
                }
            }
        }
    };

    // Combine base (Ref definition + Owned impl) with the new Archived impl
    Ok(quote! {
        #impl_archived
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
fn generate_field_conversion(field_name: &Ident, ty: &Type) -> TokenStream {
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

// Generate the conversion logic for as_deep_ref (Archived -> Borrowed)
// Handles rkyv specific types (ArchivedString, ArchivedVec, etc.)
fn generate_rkyv_field_conversion(field_name: &Ident, ty: &Type) -> TokenStream {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            let segment = path.segments.last().unwrap();
            let ident_str = segment.ident.to_string();

            match ident_str.as_str() {
                "bool" | "i8" | "i16_le" | "i32_le" | "i64_le" | "u8_le" | "u16_le" | "u32_le" | "u64_le" | "f16_le"
                | "f32_le" | "f64_le" => {
                    quote! { #field_name: self.#field_name.into() }
                }

                // String: ArchivedString has .as_str()
                "ArchivedString" => {
                    quote! { #field_name: self.#field_name.as_str() }
                }

                // Vec
                "ArchivedVec" => {
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                            if is_primitive(inner_ty) {
                                // Primitive vec: borrow as slice
                                // Note: This assumes Archived<T> has the same layout as T (Little Endian Context)
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
                "ArchivedOption" => {
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                            if is_primitive(inner_ty) {
                                // ArchivedOption<ArchivedPrimitive> -> .as_ref() gives Option<&ArchivedPrimitive>
                                // We need to convert &ArchivedPrimitive to Primitive.
                                // We deref (copy) and convert into native.
                                quote! { 
                                    #field_name: self.#field_name.as_ref().map(|x| (*x).into()) 
                                }
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