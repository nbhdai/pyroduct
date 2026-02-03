use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Item, parse_quote};

/// Attribute macro that adds rkyv serialization and the Bridgable trait.
/// 
/// Usage: #[bridgeable] instead of #[derive(Bridgeable)]
#[proc_macro_attribute]
pub fn bridgeable(_args: TokenStream, input: TokenStream) -> TokenStream {
    // Parse the input as an Item (Enum, Struct, etc.)
    let mut item = parse_macro_input!(input as Item);

    // We need to extract the name and generics for the impl block
    let (name, impl_generics, ty_generics, where_clause) = match &mut item {
        Item::Struct(s) => {
            s.attrs.push(parse_quote!(#[derive(::rkyv::Archive, ::rkyv::Serialize, ::rkyv::Deserialize)]));
            (&s.ident, s.generics.split_for_impl().0, s.generics.split_for_impl().1, &s.generics.where_clause)
        },
        Item::Enum(e) => {
            e.attrs.push(parse_quote!(#[derive(::rkyv::Archive, ::rkyv::Serialize, ::rkyv::Deserialize)]));
            (&e.ident, e.generics.split_for_impl().0, e.generics.split_for_impl().1, &e.generics.where_clause)
        },
        _ => {
            return syn::Error::new_spanned(&item, "Bridgeable can only be used on structs and enums")
                .to_compile_error()
                .into();
        }
    };

    // Generate the implementation block
    let impl_block = quote! {
        impl #impl_generics ::bridge_vec::Bridgable for #name #ty_generics #where_clause {
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

    // Output the modified item (with rkyv derives) AND the impl block
    TokenStream::from(quote! {
        #item
        #impl_block
    })
}