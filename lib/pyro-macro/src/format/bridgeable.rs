use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    ItemStruct, Path,
    parse_quote,
};

use crate::format::BridgeableArgs;

/// Whitelist of derives that are safe to forward to the rkyv `Archived` type.
const DERIVE_WHITELIST: &[&str] = &["Debug", "PartialEq", "Eq", "PartialOrd", "Ord"];

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

    let mut rkyv_pass = Vec::with_capacity(5);
    rkyv_pass.push(quote!(crate = #import_location::format::rkyv_8::rkyv));
    if !derives_to_pass.is_empty() {
        rkyv_pass.push(quote! { attr(derive(#(#derives_to_pass),*)) });
    };

    if !compares_to_add.is_empty() {
        rkyv_pass.push(quote! { compare(#(#compares_to_add),*) });
    };

    item.attrs.push(parse_quote!(#[derive(#import_location::format::rkyv_8::rkyv::Archive, #import_location::format::rkyv_8::rkyv::Serialize, #import_location::format::rkyv_8::rkyv::Deserialize)]));
    item.attrs.push(parse_quote!(#[rkyv(#(#rkyv_pass),*)]));

    let (name, impl_generics, ty_generics, where_clause) = (
        &item.ident,
        item.generics.split_for_impl().0,
        item.generics.split_for_impl().1,
        &item.generics.where_clause,
    );

    // 3. Generate UserHeaderValues + Bridgeable impls (matching common.rs)
    let bridgeable_block = quote! {
        impl #impl_generics #import_location::format::format::UserHeaderValues for #name #ty_generics #where_clause {
            const VERSION: u8 = 0;
        }

        impl #impl_generics #import_location::format::Bridgeable for #name #ty_generics #where_clause {
            type Format = #import_location::format::rkyv_8::Rkyv<#name #ty_generics>;
        }
    };

    // 5. Output
    Ok(TokenStream::from(quote! {
        #item
        #bridgeable_block
    }))
}
