use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{
    GenericArgument, GenericParam, ItemStruct, Lifetime, LifetimeParam, Path, PathArguments,
    TraitBound, Type, TypeParamBound, TypePath,
};

use crate::format::deep_ref::map_type_to_ref;

/// Generates `impl TryFrom<PyroRow> for Struct` and `impl TryFrom<PyroValue> for Struct`
pub fn from_row(input: &ItemStruct, import_location: &Path) -> syn::Result<TokenStream> {
    // 1. Parse Input

    let struct_name = &input.ident;

    // 3. Prepare Generics
    // We need impl<'a, T: TryFrom<PyroValue<'a>>> ... for Struct<T>
    let mut impl_generics = input.generics.clone();
    let lifetime = Lifetime::new("'a", Span::call_site());

    // Construct the bound: std::convert::TryFrom<#import_location::PyroValue<'a>>
    let bound_tokens = quote! { std::convert::TryFrom<#import_location::PyroValue<'a>> };
    let bound: TypeParamBound = syn::parse2(bound_tokens)?;

    // Add the bound to every Type parameter in the impl generics
    for param in impl_generics.params.iter_mut() {
        if let GenericParam::Type(t) = param {
            t.bounds.push(bound.clone());
        }
    }

    // Insert the lifetime 'a at the beginning of the params list
    impl_generics.params.insert(
        0,
        GenericParam::Lifetime(LifetimeParam::new(lifetime.clone())),
    );

    let (impl_g, _, where_clause) = impl_generics.split_for_impl();
    let (_, ty_g, _) = input.generics.split_for_impl(); // Original generics for the Struct<T>

    // =========================================================================
    // Field Extraction Logic (Owned)
    // =========================================================================
    let mut owned_field_extractions = Vec::with_capacity(input.fields.len());

    for f in &input.fields {
        let name = f
            .ident
            .as_ref()
            .ok_or_else(|| syn::Error::new_spanned(f, "FromRow requires named fields"))?;

        let name_str = name.to_string();
        let ty = &f.ty;
        let missing_err = format!("Missing field: {}", name_str);
        let field_err = format!("Failed to convert field '{}'", name_str);

        let stream = generate_field_try_from_owned(
            name,
            &name_str,
            &missing_err,
            &field_err,
            ty,
            import_location,
        )?;
        owned_field_extractions.push(stream);
    }

    let expanded = quote! {
        // -----------------------------------------------------------------
        // TryFrom<PyroRow<'a>> for Struct (Owned)
        // -----------------------------------------------------------------
        impl #impl_g std::convert::TryFrom<#import_location::PyroRow<'a>> for #struct_name #ty_g #where_clause {
            type Error = #import_location::PyroRow<'a>;

            fn try_from(row: #import_location::PyroRow<'a>) -> Result<Self, Self::Error> {
                let result = (|| -> Result<Self, &'static str> {
                    Ok(Self {
                        #(#owned_field_extractions,)*
                    })
                })();

                result.map_err(|_| row)
            }
        }

        // -----------------------------------------------------------------
        // TryFrom<&PyroRow<'a>> for Struct (Reference)
        // -----------------------------------------------------------------
        impl #impl_g std::convert::TryFrom<& #import_location::PyroRow<'a>> for #struct_name #ty_g #where_clause {
            type Error = &'static str;

            fn try_from(row: & #import_location::PyroRow<'a>) -> Result<Self, Self::Error> {
                Ok(Self {
                    #(#owned_field_extractions,)*
                })
            }
        }

        // -----------------------------------------------------------------
        // TryFrom<PyroValue<'a>> for Struct (Owned)
        // -----------------------------------------------------------------
        impl #impl_g std::convert::TryFrom<#import_location::PyroValue<'a>> for #struct_name #ty_g #where_clause {
            type Error = #import_location::PyroValue<'a>;

            fn try_from(value: #import_location::PyroValue<'a>) -> Result<Self, Self::Error> {
                match value {
                    #import_location::PyroValue::Group(r) => match <Self as std::convert::TryFrom<#import_location::PyroRow<'a>>>::try_from(r) {
                        Ok(s) => Ok(s),
                        Err(r) => Err(#import_location::PyroValue::Group(r)),
                    },
                    v => Err(v)
                }
            }
        }

        // -----------------------------------------------------------------
        // TryFrom<&PyroValue<'a>> for Struct (Reference)
        // -----------------------------------------------------------------
        impl #impl_g std::convert::TryFrom<& #import_location::PyroValue<'a>> for #struct_name #ty_g #where_clause {
            type Error = &'static str;

            fn try_from(value: & #import_location::PyroValue<'a>) -> Result<Self, Self::Error> {
                match value {
                    #import_location::PyroValue::Group(r) => {
                        <Self as std::convert::TryFrom<& #import_location::PyroRow<'a>>>::try_from(r)
                    }
                    _ => Err("Expected Group")
                }
            }
        }
    };

    Ok(expanded)
}

/// Generates `impl TryFrom<PyroRow> for StructRef` and `impl TryFrom<PyroValue> for StructRef`
pub fn ref_from_row(input: &ItemStruct, import_location: &Path) -> syn::Result<TokenStream> {
    // 1. Parse Input
    let struct_name = &input.ident;
    let ref_struct_name = format_ident!("{}Ref", struct_name);

    // 2. Prepare Generics for Impl
    // We need impl<'a, T: DeepRef> ... for StructRef<'a, <T as DeepRef>::Ref<'a>>
    let mut impl_generics = input.generics.clone();
    let lifetime = Lifetime::new("'a", Span::call_site());

    // Add DeepRef bound to all Type params
    let mut deep_ref_bound_path = import_location.clone();
    deep_ref_bound_path
        .segments
        .push(syn::PathSegment::from(format_ident!("DeepRef")));

    for param in impl_generics.params.iter_mut() {
        if let GenericParam::Type(t) = param {
            t.bounds.push(TypeParamBound::Trait(TraitBound {
                paren_token: None,
                modifier: syn::TraitBoundModifier::None,
                lifetimes: None,
                path: deep_ref_bound_path.clone(),
            }));
            // Also ensure T lives as long as 'a if necessary, but usually DeepRef handles this projection
            t.bounds.push(TypeParamBound::Lifetime(lifetime.clone()));
        }
    }

    // Insert 'a into impl params
    impl_generics.params.insert(
        0,
        GenericParam::Lifetime(LifetimeParam::new(lifetime.clone())),
    );

    let (impl_g, _, where_clause) = impl_generics.split_for_impl();

    // 3. Prepare Arguments for the Ref Struct
    // StructRef < 'a, <T as DeepRef>::Ref<'a>, ... >
    let mut ref_struct_args = Vec::new();
    ref_struct_args.push(quote! { #lifetime });

    for param in &input.generics.params {
        match param {
            GenericParam::Type(t) => {
                let ident = &t.ident;
                ref_struct_args
                    .push(quote! { <#ident as #import_location::format::DeepRef>::Ref<#lifetime> });
            }
            GenericParam::Const(c) => {
                let ident = &c.ident;
                ref_struct_args.push(quote! { #ident });
            }
            GenericParam::Lifetime(l) => {
                let ident = &l.lifetime;
                ref_struct_args.push(quote! { #ident });
            }
        }
    }

    // =========================================================================
    // Field Extraction Logic
    // =========================================================================
    let mut ref_field_extractions = Vec::with_capacity(input.fields.len());
    let mut lifetime_used = false;

    for f in &input.fields {
        let name = f
            .ident
            .as_ref()
            .ok_or_else(|| syn::Error::new_spanned(f, "FromRow requires named fields"))?;

        let name_str = name.to_string();
        let ty = &f.ty;

        // IMPORTANT: The mapped_type provided here (via map_type_to_ref) is correct.
        // It correctly maps Option<String> -> Option<&'a str> and Option<i32> -> Option<i32>.
        let (mapped_type, is_primitive) = map_type_to_ref(ty);
        if !is_primitive {
            lifetime_used = true;
        }

        let missing_err = format!("Missing field: {}", name_str);
        let field_err = format!("Failed to convert field '{}'", name_str);

        let stream = generate_field_try_from_ref(
            name,
            &name_str,
            &missing_err,
            &field_err,
            &mapped_type,
            ty,
            import_location,
        )?;
        ref_field_extractions.push(stream);
    }

    let phantom_init = if !lifetime_used {
        quote! { _phantom: std::marker::PhantomData }
    } else {
        quote! {}
    };

    let expanded = quote! {
        // -----------------------------------------------------------------
        // TryFrom<PyroRow<'a>> for StructRef<'a> (Owned)
        // -----------------------------------------------------------------
        impl #impl_g std::convert::TryFrom<#import_location::PyroRow<'a>> for #ref_struct_name < #(#ref_struct_args),* > #where_clause {
            type Error = #import_location::PyroRow<'a>;

            fn try_from(row: #import_location::PyroRow<'a>) -> Result<Self, Self::Error> {
                let result = (|| -> Result<Self, &'static str> {
                    Ok(Self {
                        #(#ref_field_extractions,)*
                        #phantom_init
                    })
                })();

                result.map_err(|_| row)
            }
        }

        // -----------------------------------------------------------------
        // TryFrom<&PyroRow<'a>> for StructRef<'a> (Reference)
        // -----------------------------------------------------------------
        impl #impl_g std::convert::TryFrom<& #import_location::PyroRow<'a>> for #ref_struct_name < #(#ref_struct_args),* > #where_clause {
            type Error = &'static str;

            fn try_from(row: & #import_location::PyroRow<'a>) -> Result<Self, Self::Error> {
                Ok(Self {
                    #(#ref_field_extractions,)*
                    #phantom_init
                })
            }
        }

        // -----------------------------------------------------------------
        // TryFrom<PyroValue<'a>> for StructRef<'a> (Owned)
        // -----------------------------------------------------------------
        impl #impl_g std::convert::TryFrom<#import_location::PyroValue<'a>> for #ref_struct_name < #(#ref_struct_args),* > #where_clause {
            type Error = #import_location::PyroValue<'a>;

            fn try_from(value: #import_location::PyroValue<'a>) -> Result<Self, Self::Error> {
                match value {
                    #import_location::PyroValue::Group(r) => match <Self as std::convert::TryFrom<#import_location::PyroRow<'a>>>::try_from(r) {
                        Ok(s) => Ok(s),
                        Err(r) => Err(#import_location::PyroValue::Group(r)),
                    },
                    v => Err(v)
                }
            }
        }

        // -----------------------------------------------------------------
        // TryFrom<&PyroValue<'a>> for StructRef<'a> (Reference)
        // -----------------------------------------------------------------
        impl #impl_g std::convert::TryFrom<& #import_location::PyroValue<'a>> for #ref_struct_name < #(#ref_struct_args),* > #where_clause {
            type Error = &'static str;

            fn try_from(value: & #import_location::PyroValue<'a>) -> Result<Self, Self::Error> {
                match value {
                    #import_location::PyroValue::Group(r) => {
                        <Self as std::convert::TryFrom<& #import_location::PyroRow<'a>>>::try_from(r)
                    }
                    _ => Err("Expected Group")
                }
            }
        }
    };

    Ok(expanded)
}

// =============================================================================
// Code generation helpers for REF fields
// =============================================================================

fn generate_field_try_from_ref(
    name: &syn::Ident,
    name_str: &str,
    missing_err: &str,
    field_err: &str,
    _mapped_type: &TokenStream, // Unused variable, we calculate inner type logic manually
    original_ty: &Type,
    import_location: &Path,
) -> syn::Result<TokenStream> {
    if is_option(original_ty) {
        // For Option, we need to unwrap the inner type and map it to its ref type
        let inner_ty = get_option_inner(original_ty)
            .ok_or_else(|| syn::Error::new_spanned(original_ty, "Malformed Option type"))?;

        // FIX: Do not blindly project <inner_ty as DeepRef>.
        // Use map_type_to_ref to get the actual target type (e.g. i32, &'a str, or Ref).
        let (inner_mapped, _) = map_type_to_ref(inner_ty);

        Ok(quote! {
            #name: {
                match row.get(#name_str) {
                    Some(#import_location::PyroValue::Null) | None => None,
                    Some(val) => Some(
                        <#inner_mapped as std::convert::TryFrom<#import_location::PyroValue<'a>>>::try_from(val.clone())
                            .map_err(|_| #field_err)?
                    ),
                }
            }
        })
    } else {
        // For non-Option, mapped_type (from argument) is already correct.
        // Recalculating it to be safe and consistent with Option fix above.
        let (mapped_type, _) = map_type_to_ref(original_ty);

        Ok(quote! {
            #name: {
                let val = row.get(#name_str)
                    .ok_or_else(|| #missing_err)?
                    .clone();
                <#mapped_type as std::convert::TryFrom<#import_location::PyroValue<'a>>>::try_from(val)
                    .map_err(|_| #field_err)?
            }
        })
    }
}

// =============================================================================
// Code generation helpers for OWNED fields
// =============================================================================

fn generate_field_try_from_owned(
    name: &syn::Ident,
    name_str: &str,
    missing_err: &str,
    field_err: &str,
    ty: &Type,
    import_location: &Path,
) -> syn::Result<TokenStream> {
    if is_option(ty) {
        let inner_ty = get_option_inner(ty)
            .ok_or_else(|| syn::Error::new_spanned(ty, "Malformed Option type"))?;

        Ok(quote! {
            #name: {
                match row.get(#name_str) {
                    Some(#import_location::PyroValue::Null) | None => None,
                    Some(val) => {
                        let owned: #inner_ty = val.clone().try_into()
                            .map_err(|_| #field_err)?;
                        Some(owned)
                    }
                }
            }
        })
    } else if is_nested_struct(ty) {
        Ok(quote! {
            #name: {
                let val = row.get(#name_str)
                    .ok_or_else(|| #missing_err)?
                    .clone();
                val.try_into()
                    .map_err(|_| #field_err)?
            }
        })
    } else if is_vec_of_struct(ty) {
        let inner_ty =
            get_vec_inner(ty).ok_or_else(|| syn::Error::new_spanned(ty, "Malformed Vec type"))?;
        let fail = format!("Failed to convert element in field '{}'", name_str);
        let unexpected = format!("Expected List for field '{}'", name_str);

        // FIX: map_err must be applied inside the closure, on the Result returned by try_into()
        Ok(quote! {
            #name: {
                match row.get(#name_str)
                    .ok_or_else(|| #missing_err)?
                {
                    #import_location::PyroValue::List(items) => {
                        items.iter()
                            .map(|v| v.clone().try_into().map_err(|_| #fail))
                            .collect::<Result<Vec<#inner_ty>, _>>()?
                    }
                    _ => return Err(#unexpected),
                }
            }
        })
    } else {
        Ok(quote! {
            #name: {
                let val = row.get(#name_str)
                    .ok_or_else(|| #missing_err)?
                    .clone();
                val.try_into()
                    .map_err(|_| #field_err)?
            }
        })
    }
}

// =============================================================================
// Type inspection helpers
// =============================================================================

fn is_option(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty
        && let Some(seg) = path.segments.last()
    {
        return seg.ident == "Option";
    }
    false
}

fn get_option_inner(ty: &Type) -> Option<&Type> {
    if let Type::Path(TypePath { path, .. }) = ty
        && let Some(seg) = path.segments.last()
        && let PathArguments::AngleBracketed(args) = &seg.arguments
        && let Some(GenericArgument::Type(inner)) = args.args.first()
    {
        return Some(inner);
    }
    None
}

fn get_vec_inner(ty: &Type) -> Option<&Type> {
    if let Type::Path(TypePath { path, .. }) = ty
        && let Some(seg) = path.segments.last()
        && let PathArguments::AngleBracketed(args) = &seg.arguments
        && let Some(GenericArgument::Type(inner)) = args.args.first()
    {
        return Some(inner);
    }
    None
}

fn is_nested_struct(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty
        && let Some(seg) = path.segments.last()
    {
        let ident_str = seg.ident.to_string();
        // Expanded list to be safe, but generic T will return true (good)
        return !matches!(
            ident_str.as_str(),
            "bool"
                | "i8"
                | "i16"
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
                | "String"
                | "Vec"
                | "Option"
        );
    }
    false
}

fn is_vec_of_struct(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty
        && let Some(seg) = path.segments.last()
        && seg.ident == "Vec"
        && let PathArguments::AngleBracketed(args) = &seg.arguments
        && let Some(GenericArgument::Type(inner)) = args.args.first()
    {
        return is_nested_struct(inner);
    }
    false
}
