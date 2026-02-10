use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, GenericArgument, Path, PathArguments, Type, TypePath};

pub fn deep_ref(input: TokenStream, import_location: Path) -> syn::Result<TokenStream> {
    let input: DeriveInput = syn::parse2(input)?;

    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "DeepRef cannot be derived for structs with generic parameters (types, lifetimes, or consts)",
        ));
    }

    let struct_name = input.ident.clone();
    let ref_struct_name = format_ident!("{}Ref", struct_name);

    let fields = match input.data {
        Data::Struct(ref data) => &data.fields,
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "DeepRef can only be derived for structs",
            ));
        }
    };

    // 1. Generate the Reference Struct Definition
    let mut lifetime_used = false;
    let mut ref_fields = Vec::with_capacity(fields.len());

    for f in fields {
        let name = &f.ident;
        let vis = &f.vis;
        let ty = &f.ty;
        let (mapped_type, is_primitive) = map_type_to_ref(ty)?;

        if !is_primitive {
            lifetime_used = true;
        }

        ref_fields.push(quote! { #vis #name: #mapped_type });
    }

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
    let mut field_conversions = Vec::with_capacity(fields.len());
    for f in fields {
        let field_name = f
            .ident
            .as_ref()
            .ok_or_else(|| syn::Error::new_spanned(f, "DeepRef requires named fields"))?;
        let ty = &f.ty;
        field_conversions.push(generate_field_conversion(field_name, ty)?);
    }

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

    Ok(TokenStream::from(expanded))
}

// =============================================================================
// derive_archived_with_path (DeepRef on the ARCHIVED type)
// =============================================================================

pub fn deep_ref_archived(input: TokenStream, import_location: Path) -> syn::Result<TokenStream> {
    let input: DeriveInput = syn::parse2(input)?;

    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "DeepRef (Archived) cannot be derived for structs with generic parameters",
        ));
    }

    let struct_name = input.ident.clone();
    let archived_name = format_ident!("Archived{}", struct_name);
    let ref_struct_name = format_ident!("{}Ref", struct_name);

    let fields = match input.data {
        Data::Struct(ref data) => &data.fields,
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "DeepRef (Archived) can only be derived for structs",
            ));
        }
    };

    // Check if phantom is needed (mirrors the owned derive)
    let mut lifetime_used = false;
    for f in fields.iter() {
        let (_, is_prim) = map_type_to_ref(&f.ty)?;
        if !is_prim {
            lifetime_used = true;
            break;
        }
    }

    let mut field_conversions = Vec::with_capacity(fields.len());
    for f in fields {
        let field_name = f
            .ident
            .as_ref()
            .ok_or_else(|| syn::Error::new_spanned(f, "DeepRef requires named fields"))?;
        let ty = &f.ty;
        field_conversions.push(generate_archived_field_conversion(field_name, ty)?);
    }

    let phantom_init = if !lifetime_used {
        quote! { _phantom: std::marker::PhantomData }
    } else {
        quote! {}
    };

    let expanded = quote! {
        impl #import_location::DeepRef for #archived_name {
            type Ref<'a> = #ref_struct_name<'a>;

            fn as_deep_ref(&self) -> Self::Ref<'_> {
                #ref_struct_name {
                    #(#field_conversions,)*
                    #phantom_init
                }
            }
        }
    };

    Ok(TokenStream::from(expanded))
}

// =============================================================================
// Shared helpers
// =============================================================================

// Map Owned types to Borrowed types for the struct definition
pub(crate) fn map_type_to_ref(ty: &Type) -> syn::Result<(proc_macro2::TokenStream, bool)> {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            let segment = path
                .segments
                .last()
                .ok_or_else(|| syn::Error::new_spanned(path, "Type path cannot be empty"))?;
            let ident_str = segment.ident.to_string();

            match ident_str.as_str() {
                "String" => Ok((quote! { &'a str }, false)),
                "bool" | "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f16"
                | "f32" | "f64" => {
                    let ident = &segment.ident;
                    Ok((quote! { #ident }, true))
                }
                "Vec" => {
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                            let (inner_ref, is_prim) = map_type_to_ref(inner_ty)?;
                            if is_prim {
                                return Ok((quote! { &'a [#inner_ref] }, false));
                            } else {
                                // For complex types, we return Vec<Ref>
                                return Ok((quote! { Vec<#inner_ref> }, false));
                            }
                        }
                    }
                    Ok((quote! { Vec<()> }, false))
                }
                "Option" => {
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                            let (inner_ref, is_prim) = map_type_to_ref(inner_ty)?;
                            return Ok((quote! { Option<#inner_ref> }, is_prim));
                        }
                    }
                    Ok((quote! { Option<()> }, false))
                }
                // Nested struct - assume it has a Ref variant
                other => {
                    let ref_name = format_ident!("{}Ref", other);
                    Ok((quote! { #ref_name<'a> }, false))
                }
            }
        }
        _ => Ok((quote! { () }, true)),
    }
}

// =============================================================================
// Field conversion for OWNED types  (Foo -> FooRef)
// =============================================================================

fn generate_field_conversion(
    field_name: &syn::Ident,
    ty: &Type,
) -> syn::Result<proc_macro2::TokenStream> {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            let segment = path
                .segments
                .last()
                .ok_or_else(|| syn::Error::new_spanned(path, "Type path cannot be empty"))?;
            let ident_str = segment.ident.to_string();

            match ident_str.as_str() {
                // Primitives: Copy
                "bool" | "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f16"
                | "f32" | "f64" => Ok(quote! { #field_name: self.#field_name }),

                // String: Borrow as &str
                "String" => Ok(quote! { #field_name: self.#field_name.as_str() }),

                // Vec
                "Vec" => {
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                            if is_primitive(inner_ty) {
                                // Primitive vec: borrow as slice
                                Ok(quote! { #field_name: self.#field_name.as_slice() })
                            } else {
                                // Complex vec: map to Vec<Ref>
                                Ok(quote! {
                                    #field_name: self.#field_name.iter().map(|x| x.as_deep_ref()).collect()
                                })
                            }
                        } else {
                            Ok(quote! { #field_name: vec![] })
                        }
                    } else {
                        Ok(quote! { #field_name: vec![] })
                    }
                }

                // Option
                "Option" => {
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                            if is_primitive(inner_ty) {
                                Ok(quote! { #field_name: self.#field_name })
                            } else if is_string(inner_ty) {
                                Ok(quote! { #field_name: self.#field_name.as_deref() })
                            } else {
                                Ok(
                                    quote! { #field_name: self.#field_name.as_ref().map(|x| x.as_deep_ref()) },
                                )
                            }
                        } else {
                            Ok(quote! { #field_name: None })
                        }
                    } else {
                        Ok(quote! { #field_name: None })
                    }
                }

                // Nested Structs
                _ => Ok(quote! { #field_name: self.#field_name.as_deep_ref() }),
            }
        }
        _ => Ok(quote! { #field_name: self.#field_name.as_deep_ref() }),
    }
}

// =============================================================================
// Field conversion for ARCHIVED types  (ArchivedFoo -> FooRef)
// =============================================================================

fn generate_archived_field_conversion(
    field_name: &syn::Ident,
    ty: &Type,
) -> syn::Result<proc_macro2::TokenStream> {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            let segment = path
                .segments
                .last()
                .ok_or_else(|| syn::Error::new_spanned(path, "Type path cannot be empty"))?;
            let ident_str = segment.ident.to_string();

            match ident_str.as_str() {
                // Primitives: On little-endian the archived repr is identity.
                "bool" | "u8" | "i8" => Ok(quote! { #field_name: self.#field_name }),
                "i16" | "i32" | "i64" | "u16" | "u32" | "u64" | "f32" | "f64" => {
                    Ok(quote! { #field_name: self.#field_name.into() })
                }
                "f16" => Ok(quote! { #field_name: self.#field_name.into() }),

                // String -> ArchivedString: already has DeepRef -> &str
                "String" => Ok(quote! { #field_name: self.#field_name.as_deep_ref() }),

                // Vec<T> -> ArchivedVec<T::Archived>
                "Vec" => {
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                            if is_primitive(inner_ty) {
                                // ArchivedVec<prim> has DeepRef -> &[prim]
                                Ok(quote! { #field_name: self.#field_name.as_deep_ref() })
                            } else {
                                // ArchivedVec<Complex>: iterate and deep-ref each element
                                Ok(quote! {
                                    #field_name: self.#field_name.iter().map(|x| x.as_deep_ref()).collect()
                                })
                            }
                        } else {
                            Ok(quote! { #field_name: vec![] })
                        }
                    } else {
                        Ok(quote! { #field_name: vec![] })
                    }
                }

                // Option<T> -> ArchivedOption<T::Archived>
                "Option" => {
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                            if is_primitive(inner_ty) {
                                Ok(quote! { #field_name: self.#field_name.as_deep_ref() })
                            } else {
                                Ok(quote! { #field_name: self.#field_name.as_deep_ref() })
                            }
                        } else {
                            Ok(quote! { #field_name: None })
                        }
                    } else {
                        Ok(quote! { #field_name: None })
                    }
                }

                // Nested struct: ArchivedBar has DeepRef -> BarRef
                _ => Ok(quote! { #field_name: self.#field_name.as_deep_ref() }),
            }
        }
        _ => Ok(quote! { #field_name: self.#field_name.as_deep_ref() }),
    }
}

// =============================================================================
// Helpers
// =============================================================================

fn is_primitive(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        if let Some(segment) = path.segments.last() {
            let ident = segment.ident.to_string();
            return matches!(
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
            );
        }
    }
    false
}

fn is_string(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        if let Some(segment) = path.segments.last() {
            return segment.ident == "String";
        }
    }
    false
}
