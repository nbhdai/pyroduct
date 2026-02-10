use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, GenericArgument, Path, PathArguments, Type, TypePath};

// Assuming this crate exists as per original code
use crate::deep_ref::map_type_to_ref;

pub fn from_row(input: TokenStream, import_location: Path) -> syn::Result<TokenStream> {
    // 1. Parse Input
    let input: DeriveInput = syn::parse2(input)?;

    // 2. Validate Generics (Must be empty)
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "FromRow cannot be derived for structs with generic parameters (types, lifetimes, or consts)",
        ));
    }

    let struct_name = &input.ident;
    let ref_struct_name = format_ident!("{}Ref", struct_name);

    // 3. Validate Data Type (Must be Struct)
    let fields = match input.data {
        Data::Struct(ref data) => &data.fields,
        _ => {
            return Err(syn::Error::new_spanned(
                struct_name,
                "FromRow can only be derived for structs",
            ));
        }
    };

    // =========================================================================
    // Detect whether the ref struct needs a lifetime
    // =========================================================================
    let mut lifetime_used = false;
    for f in fields {
        let (_, is_prim) = map_type_to_ref(&f.ty)?;
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

    // =========================================================================
    // TryFrom<PyroValue<'a>> for StructRef<'a> (the ref/borrowed path)
    // =========================================================================
    let mut ref_field_extractions = Vec::with_capacity(fields.len());

    for f in fields {
        // Safe unwrap of named field
        let name = f.ident.as_ref().ok_or_else(|| {
            syn::Error::new_spanned(
                f,
                "FromRow can only be derived for structs with named fields",
            )
        })?;

        let name_str = name.to_string();
        let ty = &f.ty;
        let (mapped_type, _) = map_type_to_ref(ty)?;
        let err_msg = format!("Missing field: {}", name_str);

        let stream = generate_field_try_from(
            name,
            &name_str,
            &err_msg,
            &mapped_type,
            ty,
            &import_location,
        )?;
        ref_field_extractions.push(stream);
    }

    // =========================================================================
    // TryFrom<PyroValue<'a>> for Struct (the owned path)
    // =========================================================================
    let mut owned_field_extractions = Vec::with_capacity(fields.len());

    for f in fields {
        // Safe unwrap of named field
        let name = f.ident.as_ref().ok_or_else(|| {
            syn::Error::new_spanned(
                f,
                "FromRow can only be derived for structs with named fields",
            )
        })?;

        let name_str = name.to_string();
        let ty = &f.ty;
        let err_msg = format!("Missing field: {}", name_str);

        let stream =
            generate_field_try_from_owned(name, &name_str, &err_msg, ty, &import_location)?;
        owned_field_extractions.push(stream);
    }

    let expanded = quote! {
        // -----------------------------------------------------------------
        // TryFrom<PyroValue<'a>> for StructRef<'a>
        // -----------------------------------------------------------------
        impl<'a> TryFrom<#import_location::PyroValue<'a>> for #ref_struct_name<'a> {
            type Error = #import_location::PyroValue<'a>;

            fn try_from(value: #import_location::PyroValue<'a>) -> Result<Self, Self::Error> {
                let mut row = match value {
                    #import_location::PyroValue::Group(row) => row,
                    other => return Err(other),
                };

                // Wrapped in an IIFE closure to return Strings easily, mapped to Error later
                let result = (|| -> Result<Self, String> {
                    Ok(Self {
                        #(#ref_field_extractions,)*
                        #phantom_init
                    })
                })();

                result.map_err(|_| #import_location::PyroValue::Group(row))
            }
        }

        // -----------------------------------------------------------------
        // TryFrom<PyroValue<'a>> for Struct (owned)
        // -----------------------------------------------------------------
        impl<'a> TryFrom<#import_location::PyroValue<'a>> for #struct_name {
            type Error = #import_location::PyroValue<'a>;

            fn try_from(value: #import_location::PyroValue<'a>) -> Result<Self, Self::Error> {
                let mut row = match value {
                    #import_location::PyroValue::Group(row) => row,
                    other => return Err(other),
                };

                let result = (|| -> Result<Self, String> {
                    Ok(Self {
                        #(#owned_field_extractions,)*
                    })
                })();

                result.map_err(|_| #import_location::PyroValue::Group(row))
            }
        }
    };

    Ok(TokenStream::from(expanded))
}

// =============================================================================
// Code generation helpers for ref fields
// =============================================================================

fn generate_field_try_from(
    name: &syn::Ident,
    name_str: &str,
    err_msg: &str,
    _mapped_type: &proc_macro2::TokenStream, // Kept signature for compatibility, though unused in logic below
    original_ty: &Type,
    import_location: &Path,
) -> syn::Result<proc_macro2::TokenStream> {
    if is_option(original_ty) {
        Ok(quote! {
            #name: {
                match row.get(#name_str) {
                    Some(#import_location::PyroValue::Null) | None => None,
                    Some(val) => Some(
                        val.clone().try_into()
                            .map_err(|_| format!("Failed to convert field '{}'", #name_str))?
                    ),
                }
            }
        })
    } else {
        Ok(quote! {
            #name: {
                let val = row.get(#name_str)
                    .ok_or_else(|| #err_msg.to_string())?
                    .clone();
                val.try_into()
                    .map_err(|_| format!("Failed to convert field '{}'", #name_str))?
            }
        })
    }
}

// =============================================================================
// Code generation helpers for owned fields
// =============================================================================

fn generate_field_try_from_owned(
    name: &syn::Ident,
    name_str: &str,
    err_msg: &str,
    ty: &Type,
    import_location: &Path,
) -> syn::Result<proc_macro2::TokenStream> {
    if is_option(ty) {
        // Unwrap logic safe here because is_option(ty) is true
        let inner_ty = get_option_inner(ty).ok_or_else(|| {
            syn::Error::new_spanned(ty, "Malformed Option type, could not extract inner type")
        })?;

        Ok(quote! {
            #name: {
                match row.get(#name_str) {
                    Some(#import_location::PyroValue::Null) | None => None,
                    Some(val) => {
                        let owned: #inner_ty = val.clone().try_into()
                            .map_err(|_| format!("Failed to convert field '{}'", #name_str))?;
                        Some(owned)
                    }
                }
            }
        })
    } else if is_nested_struct(ty) {
        Ok(quote! {
            #name: {
                let val = row.get(#name_str)
                    .ok_or_else(|| #err_msg.to_string())?
                    .clone();
                val.try_into()
                    .map_err(|_| format!("Failed to convert field '{}'", #name_str))?
            }
        })
    } else if is_vec_of_struct(ty) {
        let inner_ty = get_vec_inner(ty).ok_or_else(|| {
            syn::Error::new_spanned(ty, "Malformed Vec type, could not extract inner type")
        })?;

        Ok(quote! {
            #name: {
                match row.get(#name_str)
                    .ok_or_else(|| #err_msg.to_string())?
                {
                    #import_location::PyroValue::List(items) => {
                        items.iter()
                            .map(|v| v.clone().try_into()
                                .map_err(|_| format!("Failed to convert element in field '{}'", #name_str)))
                            .collect::<Result<Vec<#inner_ty>, _>>()?
                    }
                    _ => return Err(format!("Expected List for field '{}'", #name_str)),
                }
            }
        })
    } else {
        Ok(quote! {
            #name: {
                let val = row.get(#name_str)
                    .ok_or_else(|| #err_msg.to_string())?
                    .clone();
                val.try_into()
                    .map_err(|_| format!("Failed to convert field '{}'", #name_str))?
            }
        })
    }
}

// =============================================================================
// Type inspection helpers
// =============================================================================

fn is_option(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        if let Some(seg) = path.segments.last() {
            return seg.ident == "Option";
        }
    }
    false
}

/// Returns Some(&inner_type) if found, None otherwise.
fn get_option_inner(ty: &Type) -> Option<&Type> {
    if let Type::Path(TypePath { path, .. }) = ty {
        if let Some(seg) = path.segments.last() {
            if let PathArguments::AngleBracketed(args) = &seg.arguments {
                if let Some(GenericArgument::Type(inner)) = args.args.first() {
                    return Some(inner);
                }
            }
        }
    }
    None
}

/// Returns Some(&inner_type) if found, None otherwise.
fn get_vec_inner(ty: &Type) -> Option<&Type> {
    if let Type::Path(TypePath { path, .. }) = ty {
        if let Some(seg) = path.segments.last() {
            if let PathArguments::AngleBracketed(args) = &seg.arguments {
                if let Some(GenericArgument::Type(inner)) = args.args.first() {
                    return Some(inner);
                }
            }
        }
    }
    None
}

fn is_nested_struct(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        if let Some(seg) = path.segments.last() {
            let ident_str = seg.ident.to_string();
            return !matches!(
                ident_str.as_str(),
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
                    | "Vec"
                    | "Option"
            );
        }
    }
    false
}

fn is_vec_of_struct(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        if let Some(seg) = path.segments.last() {
            if seg.ident == "Vec" {
                if let PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() {
                        return is_nested_struct(inner);
                    }
                }
            }
        }
    }
    false
}
