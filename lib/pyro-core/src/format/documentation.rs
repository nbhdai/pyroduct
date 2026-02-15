use crate::format::bridgeable::DocRec;
use quote::quote;
use syn::{Fields, ItemStruct, Path, Type, parse_quote};

/// Holds the parsed structure data required to generate the API docs.
pub struct MagmaDocumentation {
    pub ident: syn::Ident,
    pub generics: syn::Generics,
    pub doc: Option<String>,
    pub fields: Vec<MagmaField>,
}

pub struct MagmaField {
    pub name: String,
    pub ty: Type,
    pub doc: Option<String>,
}

impl MagmaDocumentation {
    /// Phase 1: Parse the Item into our intermediate representation.
    pub fn from_item(s: &ItemStruct, doc_rec: DocRec) -> syn::Result<Self> {
        // 1. Extract Struct Documentation
        let struct_docs = extract_docs(&s.attrs);

        // Validation: Check if docs are required but missing
        if struct_docs.is_none() && doc_rec.need_struct() {
            return Err(syn::Error::new_spanned(
                &s.ident,
                "Client structs must have documentation (///) to generate API specs.",
            ));
        }

        // 2. Extract Fields and their Documentation
        let fields = match &s.fields {
            Fields::Named(named) => named
                .named
                .iter()
                .map(|f| {
                    Ok(MagmaField {
                        name: f.ident.as_ref().unwrap().to_string(),
                        ty: f.ty.clone(),
                        doc: extract_docs(&f.attrs),
                    })
                })
                .collect::<syn::Result<Vec<_>>>()?,

            Fields::Unnamed(unnamed) => unnamed
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    Ok(MagmaField {
                        name: i.to_string(),
                        ty: f.ty.clone(),
                        doc: extract_docs(&f.attrs),
                    })
                })
                .collect::<syn::Result<Vec<_>>>()?,

            Fields::Unit => vec![],
        };

        Ok(Self {
            ident: s.ident.clone(),
            generics: s.generics.clone(),
            doc: struct_docs,
            fields,
        })
    }

    /// Phase 2: Generate the TokenStream from the intermediate representation.
    pub fn generate(&self, import_location: &Path) -> syn::Result<proc_macro2::TokenStream> {
        let values_path = quote! { #import_location::value };
        let name = &self.ident;

        // Add `Typeable` bound to all generics: impl<T: Typeable> Typeable for MyStruct<T>
        let mut generics = self.generics.clone();
        for param in &mut generics.params {
            if let syn::GenericParam::Type(ref mut type_param) = *param {
                type_param.bounds.push(parse_quote!(#values_path::Typeable));
            }
        }
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        // Generate Field Entries
        let field_entries: Vec<proc_macro2::TokenStream> = self
            .fields
            .iter()
            .map(|f| {
                let fname = &f.name;
                let fty = &f.ty;

                // Handle Field Documentation
                let doc_setter = match &f.doc {
                    Some(doc_str) => quote! {
                        field.documentation = Some(::std::borrow::Cow::Borrowed(#doc_str));
                    },
                    None => quote! {},
                };

                quote! {
                    {
                        let mut field = #values_path::PyroField::<'static>::new(
                            #fname,
                            <#fty as #values_path::Typeable>::pyro_type(),
                            <#fty as #values_path::Typeable>::is_nullable(),
                        );
                        #doc_setter
                        field
                    }
                }
            })
            .collect();

        // Handle Struct Documentation
        let struct_doc_quote = match &self.doc {
            Some(d) => quote! { Some(::std::borrow::Cow::Borrowed(#d)) },
            None => quote! { None },
        };

        Ok(quote! {
            impl #impl_generics #values_path::TypeableRow for #name #ty_generics #where_clause {
                fn schema() -> #values_path::PyroSchema<'static> {
                    #values_path::PyroSchema {
                        fields: ::std::borrow::Cow::Owned(vec![
                            #(#field_entries),*
                        ]),
                        documentation: #struct_doc_quote,
                    }
                }
            }
        })
    }
}

/// Generates the `Typeable` impl for the item.
pub fn generate_documented_impl(
    item: &ItemStruct,
    import_location: &Path,
    doc_rec: DocRec,
) -> syn::Result<proc_macro2::TokenStream> {
    // Phase 1: Parse
    let documentation = MagmaDocumentation::from_item(item, doc_rec)?;

    // Phase 2: Generate
    documentation.generate(import_location)
}

/// Helper: Iterates over attributes to find `#[doc = "..."]` and joins them.
fn extract_docs(attrs: &[syn::Attribute]) -> Option<String> {
    let docs: Vec<String> = attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .filter_map(|attr| match &attr.meta {
            syn::Meta::NameValue(nv) => {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit),
                    ..
                }) = &nv.value
                {
                    Some(lit.value().trim().to_string())
                } else {
                    None
                }
            }
            _ => None,
        })
        .collect();

    if docs.is_empty() {
        None
    } else {
        Some(docs.join("\n"))
    }
}
