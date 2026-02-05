use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Item, LitStr, Meta, Path, Token, parse::{Parse, ParseStream}, parse_quote, punctuated::Punctuated, token::Comma};

pub struct LibraryArgs {
    pub meta: String,
    pub no_ffi: bool,
}

impl Parse for LibraryArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut meta = String::new();
        let mut no_ffi = false;

        // Loop through comma-separated arguments
        while !input.is_empty() {
            if input.peek(LitStr) {
                let s: LitStr = input.parse()?;
                meta = s.value();
            } else if input.peek(Ident) {
                let id: Ident = input.parse()?;
                if id == "NoFFI" {
                    no_ffi = true;
                }
            }

            // Consume optional comma
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            } else {
                break;
            }
        }

        Ok(LibraryArgs { meta, no_ffi })
    }
}

/// Creates a function we can use to register the library identity
pub fn create_ident(import_location: syn::Path, meta: &str, no_ffi: bool) -> proc_macro2::TokenStream {
    // Determine if we should generate FFI-related tokens
    let ffi_exports = if !no_ffi {
        quote::quote! {
            #[unsafe(no_mangle)]
            pub extern "C" fn library_json_ptr() -> *const u8 {
                LIBRARY_IDENTITY_DATA.as_ptr()
            }

            #[unsafe(no_mangle)]
            pub extern "C" fn library_json_len() -> usize {
                LIBRARY_IDENTITY_DATA.len()
            }
        }
    } else {
        quote::quote! {}
    };

    quote::quote! {
        #[derive(Debug, Clone, Copy)]
        pub struct Library;

        pub const LIBRARY_IDENTITY_DATA: &'static str = concat!(
            "{",
            "\"meta\": \"", #meta, "\",",
            "\"name\": \"", env!("CARGO_PKG_NAME"), "\",",
            "\"version\": \"", env!("CARGO_PKG_VERSION"), "\"",
            "}"
        );

        impl Library {
            pub const META: &'static str = #meta;
            pub const NAME: &'static str = env!("CARGO_PKG_NAME");
            pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");

            pub fn register_info() -> #import_location::captured::LibraryInfo<'static> {
                let info = #import_location::captured::LibraryInfo {
                    meta: ::std::borrow::Cow::Borrowed(Self::META),
                    name: ::std::borrow::Cow::Borrowed(Self::NAME),
                    version: ::std::borrow::Cow::Borrowed(Self::VERSION),
                };
                #import_location::captured::register_app_identity(info.clone(), LIBRARY_IDENTITY_DATA);
                
                info
            }
        }

        #ffi_exports
    }
}

/// Attribute macro that adds rkyv serialization and the Bridgeable trait.
///
/// Usage: #[bridgeable(derive(Debug, PartialEq))]
pub fn bridgeable(
    attribute_args: Punctuated<Meta, Comma>,
    mut item: Item,
    import_location: Path,
) -> syn::Result<TokenStream> {
    // Whitelist of derives that are safe/common to pass through to the Archived type
    let derive_whitelist = ["Debug", "PartialEq", "Eq", "PartialOrd", "Ord"];

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

    // 4. Construct the `rkyv` attributes based on our analysis
    // Always add the base rkyv derives
    let base_derives = quote! {
        #[derive(#import_location::rkyv::Archive, #import_location::rkyv::Serialize, #import_location::rkyv::Deserialize)]
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
            return Err(syn::Error::new_spanned(
                &item,
                "Bridgeable can only be used on structs and enums",
            ));
        }
    };

    // 6. Generate the implementation block
    let impl_block = quote! {
        impl #impl_generics #import_location::Bridgeable for #name #ty_generics #where_clause {
            fn serialize(&self) -> Result<#import_location::BridgeVec, #import_location::BridgeError> {
                #import_location::BridgeVec::serialize_from(self).map(|mut v| {
                    v.set_version(#import_location::PROTOCOL_VERSION);
                    v
                })
            }

            fn deserialize(vec: &#import_location::TypedBuf<Self>) -> Result<Self, #import_location::BridgeError> {
                vec.deserialize()
            }

            fn unchecked_parse(vec: #import_location::BridgeVec) -> Result<#import_location::TypedBuf<Self>, #import_location::BridgeError> {
                vec.unchecked_parse::<Self>()
            }
        }
    };

    // 7. Output the result
    Ok(TokenStream::from(quote! {
        #item
        #impl_block
    }))
}
