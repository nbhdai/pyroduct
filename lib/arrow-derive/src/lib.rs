use proc_macro::TokenStream;

mod deep_ref;
mod from_row;
mod to_row;
use syn::{Path, parse_quote};

fn import_path() -> Path {
    if cfg!(feature = "pyroduct") && cfg!(not(feature = "arrow-scalars")) {
        parse_quote!(::pyroduct::arrow_scalars)
    } else {
        // Default / arrow-scalars
        parse_quote!(::arrow_scalars)
    }
}

#[proc_macro_derive(FromRow)]
pub fn derive_from_row(input: TokenStream) -> TokenStream {
    from_row::derive(input)
}

#[proc_macro_derive(DeepRef)]
pub fn derive_deep_ref(input: TokenStream) -> TokenStream {
    deep_ref::derive(input)
}

#[proc_macro_derive(ToRow)]
pub fn derive_to_arrow(input: TokenStream) -> TokenStream {
    to_row::derive(input)
}
