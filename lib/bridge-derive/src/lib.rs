use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Bridgeable)]
pub fn derive_bridgeable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote! {
        // 1. Derive standard rkyv traits
        //    We use absolute paths to ensure we use the versions re-exported by pyroduct.
        #[derive(
            ::pyroduct::rkyv::Archive, 
            ::pyroduct::rkyv::Serialize, 
            ::pyroduct::rkyv::Deserialize
        )]
        // Tell rkyv to use the crate re-exported by pyroduct
        #[rkyv(crate = ::pyroduct::rkyv)]
        #input

        // 2. Implement the Bridgable trait
        impl #impl_generics ::pyroduct::rkyv::Bridgable for #name #ty_generics #where_clause {
            
            fn serialize(&self) -> Result<::pyroduct::BridgeVec, ::pyroduct::rkyv::rancor::Error> {
                // Delegates to the logic defined in your rkyv.rs
                ::pyroduct::BridgeVec::serialize_from(self)
            }

            fn parse(vec: ::pyroduct::BridgeVec) -> Result<::pyroduct::rkyv::TypedBuf<Self>, ::pyroduct::rkyv::rancor::Error> {
                 // Delegates to BridgeVec::parse::<T>
                vec.parse::<Self>()
            }

            fn deserialize(buf: ::pyroduct::rkyv::TypedBuf<Self>) -> Result<Self, ::pyroduct::rkyv::rancor::Error> {
                // Delegates to TypedBuf::deserialize
                buf.deserialize()
            }
        }
    };

    TokenStream::from(expanded)
}