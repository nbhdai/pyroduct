use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Ident, ItemStruct, Meta, Path, Token,
    parse::{Parse, ParseStream},
    parse_quote,
    punctuated::Punctuated,
    token::Comma,
};

/// Whitelist of derives that are safe to forward to the rkyv `Archived` type.
const DERIVE_WHITELIST: &[&str] = &["Debug", "PartialEq", "Eq", "PartialOrd", "Ord"];

/// Defines the documentation requirements for the configuration struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocRec {
    /// No documentation required.
    NoReq,
    /// The top-level struct must be documented.
    StructDoc,
    /// The struct and all its fields must be documented.
    AllDoc,
}

impl DocRec {
    pub fn need_struct(&self) -> bool {
        match self {
            DocRec::NoReq => false,
            DocRec::StructDoc => true,
            DocRec::AllDoc => true,
        }
    }
}

/// Parsed arguments for the `#[bridgeable(...)]` attribute macro.
///
/// Supports the following syntax:
///
/// ```text
/// #[bridgeable]                                    // defaults
/// #[bridgeable(Document)]                          // enable Documented impl
/// #[bridgeable(derive(Debug, PartialEq))]          // forward derives
/// #[bridgeable(derive(Debug, PartialEq), Document)] // both
/// ```
///
/// - `derive(...)` — derives to forward to the archived type (whitelisted to
///   Debug, PartialEq, Eq, PartialOrd, Ord). PartialEq and PartialOrd also
///   generate `#[rkyv(compare(...))]` attributes.
/// - `Document` — opt-in flag that generates a `Documented` impl producing a
///   JSON type spec for FFI/RPC consumers.
pub struct BridgeableArgs {
    /// Whitelisted derives to pass through to the rkyv `Archived` type.
    pub derives_to_pass: Vec<Ident>,
    /// Comparison traits (PartialEq / PartialOrd) found in the derives —
    /// these also become `#[rkyv(compare(...))]` entries.
    pub compares_to_add: Vec<Ident>,
    pub doc_rec: DocRec,
}

impl BridgeableArgs {
    /// Build from an already-parsed `Punctuated<Meta, Comma>` list.
    /// This is the shared logic used by both `Parse` and a direct call site.
    pub fn from_metas(metas: Punctuated<Meta, Comma>) -> syn::Result<Self> {
        let mut derives_to_pass = Vec::new();
        let mut compares_to_add = Vec::new();

        for meta in metas {
            match &meta {
                // derive(Debug, PartialEq, ...)
                Meta::List(list) if list.path.is_ident("derive") => {
                    let nested = list
                        .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                        .unwrap_or_default();

                    for nested_meta in nested {
                        if let Meta::Path(path) = nested_meta {
                            if let Some(ident) = path.get_ident() {
                                let s = ident.to_string();

                                if DERIVE_WHITELIST.contains(&s.as_str()) {
                                    derives_to_pass.push(ident.clone());
                                }

                                if s == "PartialEq" || s == "PartialOrd" {
                                    compares_to_add.push(ident.clone());
                                }
                            }
                        }
                    }
                }
                other => {
                    return Err(syn::Error::new_spanned(
                        other,
                        "unexpected bridgeable argument; expected `derive(...)` or `Document`",
                    ));
                }
            }
        }

        Ok(BridgeableArgs {
            derives_to_pass,
            compares_to_add,
            doc_rec: DocRec::NoReq,
        })
    }
}

/// Allows `parse_macro_input!(args as BridgeableArgs)` in proc-macro crates.
impl Parse for BridgeableArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let metas = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;
        Self::from_metas(metas)
    }
}

/// Attribute macro that adds rkyv serialization, the `Bridgeable` trait, and
/// optionally the `Documented` trait for generating a JSON type spec.
///
/// # Usage
///
/// ```text
/// #[bridgeable]                                     // bare minimum
/// #[bridgeable(Document)]                           // + Documented impl
/// #[bridgeable(derive(Debug, PartialEq))]           // + archived derives
/// #[bridgeable(derive(Debug, PartialEq), Document)] // all of the above
/// ```
pub fn bridgeable(
    args: &BridgeableArgs,
    item: &mut ItemStruct,
    import_location: &Path,
) -> syn::Result<TokenStream> {
    let BridgeableArgs {
        derives_to_pass,
        compares_to_add,
        ..
    } = args;

    // 1. Construct the `rkyv` attributes
    let base_derives = quote! {
        #[derive(#import_location::rkyv_8::rkyv::Archive, #import_location::rkyv_8::rkyv::Serialize, #import_location::rkyv_8::rkyv::Deserialize)]
    };

    let pass_through_attr = if !derives_to_pass.is_empty() {
        quote! { #[rkyv(attr(derive(#(#derives_to_pass),*)))] }
    } else {
        quote! {}
    };

    let compare_attr = if !compares_to_add.is_empty() {
        quote! { #[rkyv(compare(#(#compares_to_add),*))] }
    } else {
        quote! {}
    };

    item.attrs.push(parse_quote!(#base_derives));
    if !derives_to_pass.is_empty() {
        item.attrs.push(parse_quote!(#pass_through_attr));
    }
    if !compares_to_add.is_empty() {
        item.attrs.push(parse_quote!(#compare_attr));
    }
    let (name, impl_generics, ty_generics, where_clause) = (
        &item.ident,
        item.generics.split_for_impl().0,
        item.generics.split_for_impl().1,
        &item.generics.where_clause,
    );

    // 3. Generate UserHeaderValues + Bridgeable impls (matching common.rs)
    let bridgeable_block = quote! {
        impl #impl_generics #import_location::format::UserHeaderValues for #name #ty_generics #where_clause {
            const VERSION: u8 = 0;
        }

        impl #impl_generics #import_location::Bridgeable for #name #ty_generics #where_clause {
            type Format = #import_location::rkyv_8::Rkyv<#name #ty_generics>;
        }
    };

    // 5. Output
    Ok(TokenStream::from(quote! {
        #item
        #bridgeable_block
    }))
}
