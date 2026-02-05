use proc_macro::TokenStream;
use syn::{Item, Meta, parse_macro_input, parse_quote, punctuated::Punctuated};

#[proc_macro_attribute]
pub fn bridgeable(args: TokenStream, input: TokenStream) -> TokenStream {
    let attribute_args =
        parse_macro_input!(args with Punctuated::<Meta, syn::Token![,]>::parse_terminated);
    let item = parse_macro_input!(input as Item);

    match bridge_core::bridgeable(attribute_args, item, parse_quote!(::bridge_vec)) {
        Ok(v) => v.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro]
pub fn library(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as bridge_core::LibraryArgs);

    bridge_core::create_ident(
        syn::parse_quote!(::bridge_vec), 
        &args.meta, 
        args.no_ffi
    ).into()
}