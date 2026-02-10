use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, GenericArgument, Path, PathArguments, Type, TypePath};

pub fn to_row(input: TokenStream, import_location: Path) -> syn::Result<TokenStream> {
    // 1. Parse Input
    let input: DeriveInput = syn::parse2(input)?;

    // 2. Validate Generics
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "ToRow cannot be derived for structs with generic parameters (types, lifetimes, or consts)",
        ));
    }

    let struct_name = input.ident;

    // 3. Validate Data Structure
    let fields = match input.data {
        Data::Struct(ref data) => &data.fields,
        _ => {
            return Err(syn::Error::new_spanned(
                struct_name,
                "ToRow can only be derived for structs with fields",
            ));
        }
    };

    // 4. Handle Empty Structs
    if fields.is_empty() {
        let expanded = quote! {
            impl #import_location::ToRow for #struct_name {
                fn to_row(&self) -> #import_location::PyroRow<'_> {
                    #import_location::PyroRow::new()
                }
            }
        };
        return Ok(TokenStream::from(expanded));
    }

    // 5. Generate Field Conversions
    let mut field_conversions = Vec::with_capacity(fields.len());

    for f in fields {
        // Safe check for named fields
        let name = f.ident.as_ref().ok_or_else(|| {
            syn::Error::new_spanned(f, "ToRow can only be derived for structs with named fields")
        })?;

        let name_str = name.to_string();
        let ty = &f.ty;

        let conversion = generate_conversion(quote!(self.#name), ty, &import_location)?;

        field_conversions.push(quote! { (#name_str, #conversion) });
    }

    let expanded = quote! {
        impl #import_location::ToRow for #struct_name {
            fn to_row(&self) -> #import_location::PyroRow<'_> {
                #import_location::PyroRow::from([
                    #(#field_conversions),*
                ])
            }
        }
    };

    Ok(TokenStream::from(expanded))
}

/// Generate the expression that converts a field reference into a `PyroValue<'_>`.
fn generate_conversion(
    field_expr: proc_macro2::TokenStream,
    ty: &Type,
    import_location: &Path,
) -> syn::Result<proc_macro2::TokenStream> {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            // Safely get the last segment
            let segment = path
                .segments
                .last()
                .ok_or_else(|| syn::Error::new_spanned(path, "Field type path cannot be empty"))?;

            let ident_str = segment.ident.to_string();

            if is_primitive_or_string(&ident_str) {
                // Primitives & String: From<&T> for PyroValue exists
                Ok(quote! { #import_location::PyroValue::from(&#field_expr) })
            } else if ident_str == "Vec" {
                generate_vec_conversion(field_expr, segment, import_location)
            } else if ident_str == "Option" {
                generate_option_conversion(field_expr, segment, import_location)
            } else {
                // Nested struct implementing ToRow
                Ok(
                    quote! { #import_location::PyroValue::Group(#import_location::ToRow::to_row(&#field_expr)) },
                )
            }
        }
        _ => Ok(quote! { #import_location::PyroValue::Null }),
    }
}

fn generate_vec_conversion(
    field_expr: proc_macro2::TokenStream,
    segment: &syn::PathSegment,
    import_location: &Path,
) -> syn::Result<proc_macro2::TokenStream> {
    if let PathArguments::AngleBracketed(args) = &segment.arguments {
        if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
            if is_from_convertible(inner_ty) {
                // Vec<primitive> (but not Vec<String>): From<&Vec<T>> for PyroValue exists
                Ok(quote! { #import_location::PyroValue::from(&#field_expr) })
            } else {
                // Vec<Struct> or Vec<String>: map each element through ToRow or logic
                Ok(quote! {
                    #import_location::PyroValue::List(
                        #field_expr.iter().map(|v| #import_location::PyroValue::Group(
                            #import_location::ToRow::to_row(v)
                        )).collect()
                    )
                })
            }
        } else {
            // Vec without generic args (unlikely in valid Rust but handled safely)
            Ok(quote! { #import_location::PyroValue::Null })
        }
    } else {
        Ok(quote! { #import_location::PyroValue::Null })
    }
}

fn generate_option_conversion(
    field_expr: proc_macro2::TokenStream,
    segment: &syn::PathSegment,
    import_location: &Path,
) -> syn::Result<proc_macro2::TokenStream> {
    if let PathArguments::AngleBracketed(args) = &segment.arguments {
        if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
            if is_primitive_or_string_ty(inner_ty) {
                // Option<primitive> / Option<String>: From<&Option<T>> for PyroValue exists
                Ok(quote! { #import_location::PyroValue::from(&#field_expr) })
            } else if is_vec_primitive(inner_ty) {
                // Option<Vec<primitive>>: no direct From, match and delegate
                Ok(quote! {
                    match &#field_expr {
                        Some(v) => #import_location::PyroValue::from(v),
                        None => #import_location::PyroValue::Null,
                    }
                })
            } else if is_vec_struct(inner_ty) {
                // Option<Vec<Struct>>
                Ok(quote! {
                    match &#field_expr {
                        Some(v) => #import_location::PyroValue::List(
                            v.iter().map(|item| #import_location::PyroValue::Group(
                                #import_location::ToRow::to_row(item)
                            )).collect()
                        ),
                        None => #import_location::PyroValue::Null,
                    }
                })
            } else {
                // Option<Struct>
                Ok(quote! {
                    match &#field_expr {
                        Some(v) => #import_location::PyroValue::Group(#import_location::ToRow::to_row(v)),
                        None => #import_location::PyroValue::Null,
                    }
                })
            }
        } else {
            Ok(quote! { #import_location::PyroValue::Null })
        }
    } else {
        Ok(quote! { #import_location::PyroValue::Null })
    }
}

// ---------------------------------------------------------------------------
// Type classification helpers (Refactored to be panic-free)
// ---------------------------------------------------------------------------

fn is_primitive_or_string(name: &str) -> bool {
    matches!(
        name,
        "bool"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "f16"
            | "f32"
            | "f64"
            | "String"
    )
}

/// Returns true if the type name is a primitive (not String).
fn is_primitive(name: &str) -> bool {
    matches!(
        name,
        "bool"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "f16"
            | "f32"
            | "f64"
    )
}

fn get_path_segment(ty: &Type) -> Option<&syn::PathSegment> {
    if let Type::Path(TypePath { path, .. }) = ty {
        path.segments.last()
    } else {
        None
    }
}

/// Check if a Type is a primitive or String (by examining the path).
fn is_primitive_or_string_ty(ty: &Type) -> bool {
    if let Some(segment) = get_path_segment(ty) {
        is_primitive_or_string(&segment.ident.to_string())
    } else {
        false
    }
}

/// Check if a Type has a `From<&T> for PyroValue` impl (primitives or String).
/// This covers scalars and their Vec/slice variants.
fn is_from_convertible(ty: &Type) -> bool {
    if let Some(segment) = get_path_segment(ty) {
        // Vec<primitive> has From<&Vec<T>>, Vec<String> does NOT (it's a List of Str)
        is_primitive(&segment.ident.to_string())
    } else {
        false
    }
}

/// Check if a type is Vec<primitive>
fn is_vec_primitive(ty: &Type) -> bool {
    if let Some(segment) = get_path_segment(ty) {
        if segment.ident == "Vec" {
            if let PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                    return is_from_convertible(inner_ty);
                }
            }
        }
    }
    false
}

/// Check if a type is Vec<Struct> (i.e., Vec<T> where T is not a primitive and not String)
fn is_vec_struct(ty: &Type) -> bool {
    if let Some(segment) = get_path_segment(ty) {
        if segment.ident == "Vec" {
            if let PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                    return !is_from_convertible(inner_ty) && !is_string_ty(inner_ty);
                }
            }
        }
    }
    false
}

fn is_string_ty(ty: &Type) -> bool {
    if let Some(segment) = get_path_segment(ty) {
        segment.ident == "String"
    } else {
        false
    }
}
