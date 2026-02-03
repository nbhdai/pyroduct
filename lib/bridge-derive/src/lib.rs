use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, Item, Meta};

/// Attribute macro that adds rkyv serialization and the Bridgeable trait.
///
/// Usage: #[bridgeable(derive(Debug, PartialEq))]
#[proc_macro_attribute]
pub fn bridgeable(args: TokenStream, input: TokenStream) -> TokenStream {
    // 1. Parse the macro arguments (e.g., derive(Debug, PartialEq))
    // We expect a comma-separated list of Metas
    let attribute_args =
        parse_macro_input!(args with syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated);

    // Whitelist of derives that are safe/common to pass through to the Archived type
    let derive_whitelist = [
        "Debug",
        "PartialEq",
        "Eq",
        "PartialOrd",
        "Ord",
    ];

    let mut derives_to_pass = Vec::new();
    let mut compares_to_add = Vec::new();

    // 2. Process arguments to build rkyv attributes
    for meta in attribute_args {
        if let Meta::List(list) = meta {
            if list.path.is_ident("derive") {
                // Parse the nested items inside derive(...)
                let nested_metas = list
                    .parse_args_with(
                        syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
                    )
                    .unwrap_or_default();

                for nested in nested_metas {
                    if let Meta::Path(path) = nested {
                        if let Some(ident) = path.get_ident() {
                            let ident_str = ident.to_string();

                            // Check Whitelist
                            if derive_whitelist.contains(&ident_str.as_str()) {
                                derives_to_pass.push(ident.clone());
                            }

                            // Check for Comparison traits
                            if ident_str == "PartialEq" || ident_str == "PartialOrd" {
                                compares_to_add.push(ident.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Parse the input Item (Struct/Enum)
    let mut item = parse_macro_input!(input as Item);

    // 4. Construct the `rkyv` attributes based on our analysis
    // Always add the base rkyv derives
    let base_derives = quote! {
        #[derive(::rkyv::Archive, ::rkyv::Serialize, ::rkyv::Deserialize)]
    };
    
    // Construct #[rkyv(attr(derive(...)))] if we found valid whitelist items
    let pass_through_attr = if !derives_to_pass.is_empty() {
        quote! { #[rkyv(attr(derive(#(#derives_to_pass),*)))] }
    } else {
        quote! {}
    };

    // Construct #[rkyv(compare(...))] if we found PartialEq/PartialOrd
    let compare_attr = if !compares_to_add.is_empty() {
        quote! { #[rkyv(compare(#(#compares_to_add),*))] }
    } else {
        quote! {}
    };

    // 5. Inject attributes and extract generics for the impl block
    let (name, impl_generics, ty_generics, where_clause) = match &mut item {
        Item::Struct(s) => {
            s.attrs.push(parse_quote!(#base_derives));
            if !derives_to_pass.is_empty() {
                s.attrs.push(parse_quote!(#pass_through_attr));
            }
            if !compares_to_add.is_empty() {
                s.attrs.push(parse_quote!(#compare_attr));
            }
            (
                &s.ident,
                s.generics.split_for_impl().0,
                s.generics.split_for_impl().1,
                &s.generics.where_clause,
            )
        }
        Item::Enum(e) => {
            e.attrs.push(parse_quote!(#base_derives));
            if !derives_to_pass.is_empty() {
                e.attrs.push(parse_quote!(#pass_through_attr));
            }
            if !compares_to_add.is_empty() {
                e.attrs.push(parse_quote!(#compare_attr));
            }
            (
                &e.ident,
                e.generics.split_for_impl().0,
                e.generics.split_for_impl().1,
                &e.generics.where_clause,
            )
        }
        _ => {
            return syn::Error::new_spanned(
                &item,
                "Bridgeable can only be used on structs and enums",
            )
            .to_compile_error()
            .into();
        }
    };

    // 6. Generate the implementation block
    let impl_block = quote! {
        impl #impl_generics ::bridge_vec::Bridgeable for #name #ty_generics #where_clause {
            fn serialize(&self) -> Result<::bridge_vec::BridgeVec, ::rkyv::rancor::Error> {
                ::bridge_vec::BridgeVec::serialize_from(self)
            }

            fn parse(vec: ::bridge_vec::BridgeVec) -> Result<::bridge_vec::rkyv::TypedBuf<Self>, ::rkyv::rancor::Error> {
                vec.parse::<Self>()
            }

            fn deserialize(buf: ::bridge_vec::rkyv::TypedBuf<Self>) -> Result<Self, ::rkyv::rancor::Error> {
                buf.deserialize()
            }
        }
    };

    // 7. Output the result
    TokenStream::from(quote! {
        #item
        #impl_block
    })
}


