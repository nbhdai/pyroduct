use proc_macro::TokenStream;
use syn::{Item, parse_macro_input, parse_quote};

#[proc_macro_attribute]
pub fn bridgeable(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as pyro_core::bridgeable::BridgeableArgs);
    let item = parse_macro_input!(input as Item);

    match pyro_core::bridgeable::bridgeable(args, item, parse_quote!(::pyro_vec)) {
        Ok(v) => v.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro]
pub fn library(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as pyro_core::library::LibraryArgs);

    pyro_core::library::create_ident(syn::parse_quote!(::pyro_vec), &args.meta, args.no_ffi)
        .into()
}

#[proc_macro_derive(DeepRef)]
pub fn derive_deep_ref(input: TokenStream) -> TokenStream {
    match pyro_core::deep_ref::deep_ref(input.into(), parse_quote!(::pyro_vec)) {
        Ok(v) => v.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro_derive(DeepRefArchived)]
pub fn derive_deep_ref_archived(input: TokenStream) -> TokenStream {
    match pyro_core::deep_ref::deep_ref_archived(input.into(), parse_quote!(::pyro_vec)) {
        Ok(v) => v.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro_derive(FromRow)]
pub fn derive_from_row(input: TokenStream) -> TokenStream {
    match pyro_core::from_row::from_row(input.into(), parse_quote!(::pyro_vec)) {
        Ok(v) => v.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro_derive(ToRow)]
pub fn derive_to_row(input: TokenStream) -> TokenStream {
    match pyro_core::to_row::to_row(input.into(), parse_quote!(::pyro_vec)) {
        Ok(v) => v.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
