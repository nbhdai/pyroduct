use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemStruct, Path};

pub fn to_row(input: &ItemStruct, import_location: &Path) -> syn::Result<TokenStream> {
    // 2. Validate Generics
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "ToRow cannot be derived for structs with generic parameters (types, lifetimes, or consts)",
        ));
    }

    let struct_name = &input.ident;

    // 4. Handle Empty Structs
    if input.fields.is_empty() {
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
    let mut field_conversions = Vec::with_capacity(input.fields.len());

    for f in &input.fields {
        // Safe check for named fields
        let name = f.ident.as_ref().ok_or_else(|| {
            syn::Error::new_spanned(f, "ToRow can only be derived for structs with named fields")
        })?;

        let name_str = name.to_string();

        field_conversions.push(quote! {
            (#name_str, #import_location::PyroValue::from(&self.#name))
        });
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
