use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Data, DeriveInput, GenericArgument, Path, PathArguments, Type, TypePath, parse_macro_input,
};

pub fn derive_with_path(input: TokenStream, import_location: Path) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    if !input.generics.params.is_empty() {
        return syn::Error::new_spanned(
            &input.generics,
            "ToRow cannot be derived for structs with generic parameters (types, lifetimes, or consts)"
        )
        .to_compile_error()
        .into();
    }

    let struct_name = input.ident;

    let fields = match input.data {
        Data::Struct(ref data) => &data.fields,
        _ => panic!("ToRow can only be derived for structs with fields"),
    };

    // FIX: Handle empty structs by returning ArrowRow::new()
    if fields.is_empty() {
        let expanded = quote! {
            impl #import_location::ToRow for #struct_name {
                fn to_row(&self) -> #import_location::ArrowRow<'_> {
                    #import_location::ArrowRow::new()
                }
            }
        };
        return TokenStream::from(expanded);
    }

    // Generate field conversions for borrowed self
    let field_conversions_ref = fields.iter().map(|f| {
        let name = f.ident.as_ref().unwrap();
        let name_str = name.to_string();
        let ty = &f.ty;

        let conversion = generate_conversion_ref(quote!(self.#name), ty, &import_location);

        quote! {
            (#name_str, #conversion)
        }
    });

    let expanded = quote! {
        impl #import_location::ToRow for #struct_name {
            fn to_row(&self) -> #import_location::ArrowRow<'_> {
                #import_location::ArrowRow::from([
                    #(#field_conversions_ref),*
                ])
            }
        }
    };

    TokenStream::from(expanded)
}

// Generate conversion for borrowed reference (&T -> ArrowValue<'_>)
fn generate_conversion_ref(
    field_expr: proc_macro2::TokenStream,
    ty: &Type,
    import_location: &Path,
) -> proc_macro2::TokenStream {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            let segment = path.segments.last().unwrap();
            let ident_str = segment.ident.to_string();

            // Check if it's a known primitive/std type that works with ArrowValue::from
            if is_primitive_or_std(ident_str.as_str()) {
                quote! { #import_location::ArrowValue::from(&#field_expr) }
            } else if ident_str == "Vec" {
                // Check inner type
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                        if is_primitive_inner(inner_ty) {
                            // Vec<Primitive> -> PrimitiveList via ArrowValue::from
                            quote! { #import_location::ArrowValue::from(&#field_expr) }
                        } else {
                            // Vec<Struct> -> List of Values
                            // Relies on ToValue being implemented for the inner struct
                            quote! {
                                #import_location::ArrowValue::List(
                                    #field_expr.iter().map(|v|  #import_location::ToValue::to_value(v)).collect()
                                )
                            }
                        }
                    } else {
                        quote! { #import_location::ArrowValue::Null }
                    }
                } else {
                    quote! { #import_location::ArrowValue::Null }
                }
            } else if ident_str == "Option" {
                // For Option, we likely want to delegate to the inner type's conversion if present
                // But ArrowValue::from(Option<T>) works for primitives.
                // For Structs, we need to map.
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                        if is_primitive_inner(inner_ty) || is_string_inner(inner_ty) {
                            quote! { #import_location::ArrowValue::from(&#field_expr) }
                        } else if is_vec_primitive(inner_ty) {
                            // FIX: Option<Vec<Primitive>>
                            // ArrowValue::from does not support &Option<Vec<T>>, but supports &Vec<T>
                            quote! {
                                match &#field_expr {
                                    Some(v) => #import_location::ArrowValue::from(v),
                                    None => #import_location::ArrowValue::Null,
                                }
                            }
                        } else {
                            // Option<Struct>
                            quote! {
                                match &#field_expr {
                                    Some(v) => #import_location::ToValue::to_value(v),
                                    None => #import_location::ArrowValue::Null,
                                }
                            }
                        }
                    } else {
                        quote! { #import_location::ArrowValue::Null }
                    }
                } else {
                    quote! { #import_location::ArrowValue::Null }
                }
            } else {
                // Custom Struct - Use ToValue (Must be manually implemented)
                quote! { #import_location::ToValue::to_value(&#field_expr) }
            }
        }
        _ => quote! { #import_location::ArrowValue::Null },
    }
}

fn is_primitive_or_std(name: &str) -> bool {
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

fn is_primitive_inner(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        let ident = path.segments.last().unwrap().ident.to_string();
        is_primitive_or_std(&ident) && ident != "String" // String is std but not primitive for Vec<String> optimization check typically
    } else {
        false
    }
}

fn is_string_inner(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        let ident = path.segments.last().unwrap().ident.to_string();
        ident == "String"
    } else {
        false
    }
}

// Helper to check for Vec<Primitive>
fn is_vec_primitive(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        let segment = path.segments.last().unwrap();
        if segment.ident == "Vec" {
            if let PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                    return is_primitive_inner(inner_ty);
                }
            }
        }
    }
    false
}
