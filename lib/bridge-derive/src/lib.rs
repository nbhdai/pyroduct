use proc_macro::TokenStream;
use syn::{Item, Meta, punctuated::Punctuated, parse_macro_input, parse_quote};

#[proc_macro_attribute]
pub fn bridgeable(args: TokenStream, input: TokenStream) -> TokenStream {
    let attribute_args =
        parse_macro_input!(args with Punctuated::<Meta, syn::Token![,]>::parse_terminated);
    let item = parse_macro_input!(input as Item);
    
    match bridge_core::bridgeable(attribute_args, item, parse_quote!(::bridge_vec)) {
        Ok(v) => v.into(),
        Err(error) => error.to_compile_error()
            .into(),
    }
            
}