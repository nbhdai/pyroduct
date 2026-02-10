use quote::quote;
use syn::{Fields, Item, Path, parse_quote};

/// Generates the `Typeable` impl for the item.
pub fn generate_documented_impl(
    item: &Item,
    import_location: &Path,
) -> syn::Result<proc_macro2::TokenStream> {
    match item {
        Item::Struct(s) => {
            let name = &s.ident;

            // Add `Typeable` bound to all generics: impl<T: Typeable> Typeable for MyStruct<T>
            let mut generics = s.generics.clone();
            for param in &mut generics.params {
                if let syn::GenericParam::Type(ref mut type_param) = *param {
                    type_param
                        .bounds
                        .push(parse_quote!(#import_location::typeable::Typeable));
                }
            }
            let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

            let field_entries: Vec<proc_macro2::TokenStream> = match &s.fields {
                Fields::Named(named) => named
                    .named
                    .iter()
                    .map(|f| {
                        let fname = f.ident.as_ref().unwrap().to_string();
                        let fty = &f.ty;
                        quote! {
                            #import_location::schema::PyroField::new(
                                #fname,
                                <#fty as #import_location::typeable::Typeable>::pyro_type(),
                                <#fty as #import_location::typeable::Typeable>::is_nullable(),
                            )
                        }
                    })
                    .collect(),
                Fields::Unnamed(unnamed) => unnamed
                    .unnamed
                    .iter()
                    .enumerate()
                    .map(|(i, f)| {
                        let fname = i.to_string();
                        let fty = &f.ty;
                        quote! {
                            #import_location::schema::PyroField::new(
                                #fname,
                                <#fty as #import_location::typeable::Typeable>::pyro_type(),
                                <#fty as #import_location::typeable::Typeable>::is_nullable(),
                            )
                        }
                    })
                    .collect(),
                Fields::Unit => vec![],
            };

            Ok(quote! {
                impl #impl_generics #import_location::typeable::Typeable for #name #ty_generics #where_clause {
                    fn pyro_type() -> #import_location::schema::PyroType {
                        #import_location::schema::PyroType::Group(vec![
                            #(#field_entries),*
                        ])
                    }
                }
            })
        }
        Item::Enum(_) => Err(syn::Error::new_spanned(
            item,
            "Typeable derive currently only supports structs (PyroType::Group). Enums are not yet supported in the Pyro schema.",
        )),
        _ => Err(syn::Error::new_spanned(
            item,
            "Typeable can only be derived for structs",
        )),
    }
}
