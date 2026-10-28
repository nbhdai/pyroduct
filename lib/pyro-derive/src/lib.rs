use proc_macro::TokenStream;
use pyro_macro::ffi::config::DocRec;
use syn::{
    ItemFn, ItemStruct, Token, parse_macro_input, parse_quote, parse2, punctuated::Punctuated,
};

/// A struct that goes over the wire: the struct as written, plus pyroformat's
/// `Layout`, `Writer` and `Reader` impls. `#[magma(derive(...))]` adds those
/// derives to the struct.
#[proc_macro_attribute]
pub fn magma(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut item = parse_macro_input!(input as ItemStruct);
    // The compiler has already taken `#[magma(...)]` off the struct; put it
    // back so its `derive(...)`s are read like those of a `#[magma]` in a
    // source file.
    let args = parse_macro_input!(args with Punctuated::<syn::Meta, Token![,]>::parse_terminated);
    item.attrs.push(parse_quote!(#[magma(#args)]));

    match pyro_macro::ffi::magma(item, &parse_quote!(::pyroduct)) {
        Ok(v) => v.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro]
pub fn library(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as pyro_macro::library::LibraryArgs);

    pyro_macro::library::create_ident(syn::parse_quote!(::pyroduct), &args.meta, args.no_ffi).into()
}

#[proc_macro_attribute]
pub fn capability(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as syn::ItemImpl);
    let name = std::env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "unknown".to_string());
    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_string());

    match pyro_macro::ffi::capability::CapabilityImpl::new(item, false, &name, &version) {
        Ok(cap) => cap.expand_capability().into(),
        Err(e) => e.to_compile_error().into(),
    }
}

#[proc_macro_attribute]
pub fn config(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as syn::ItemStruct);
    match pyro_macro::ffi::config::CapConfig::new(item, DocRec::NoReq) {
        Ok(cfg) => cfg.expand().into(),
        Err(e) => e.to_compile_error().into(),
    }
}

#[proc_macro_attribute]
pub fn module(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attrs: pyro_macro::module::ModuleAttrs = match parse2(attr.into()) {
        Ok(attrs) => attrs,
        Err(error) => return error.to_compile_error().into(),
    };
    let input_fn: ItemFn = match parse2(item.into()) {
        Ok(attrs) => attrs,
        Err(error) => return error.to_compile_error().into(),
    };
    if attrs.session {
        pyro_macro::module::expand_session(attrs, input_fn)
            .unwrap_or_else(|e| e.to_compile_error())
            .into()
    } else {
        pyro_macro::module::expand(attrs, input_fn)
            .unwrap_or_else(|e| e.to_compile_error())
            .into()
    }
}
