use proc_macro::TokenStream;
use pyro_macro::format::DocRec;
use quote::quote;
use syn::{ItemFn, ItemStruct, parse_macro_input, parse_quote, parse2};

#[proc_macro_attribute]
pub fn magma(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as pyro_macro::format::BridgeableArgs);
    let mut item = parse_macro_input!(input as ItemStruct);

    match pyro_macro::format::magma(args, &mut item, &parse_quote!(::pyroduct)) {
        Ok(v) => v.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro]
pub fn library(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as pyro_macro::format::library::LibraryArgs);

    pyro_macro::format::library::create_ident(
        syn::parse_quote!(::pyroduct),
        &args.meta,
        args.no_ffi,
    )
    .into()
}

#[proc_macro_derive(Document)]
pub fn document(input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemStruct);
    match pyro_macro::format::documentation::generate_documented_impl(
        &item,
        &parse_quote!(::pyroduct),
        DocRec::NoReq,
    ) {
        Ok(v) => v.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro_derive(DocumentRef)]
pub fn document_ref(input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemStruct);
    match pyro_macro::format::documentation::ref_documentation(
        &item,
        &parse_quote!(::pyroduct),
        DocRec::NoReq,
    ) {
        Ok(v) => v.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro_derive(DeepRef)]
pub fn derive_deep_ref(input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemStruct);
    match pyro_macro::format::deep_ref::deep_ref(&item, &parse_quote!(::pyroduct), &vec![]) {
        Ok(v) => v.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro_derive(FromRow)]
pub fn derive_from_row(input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemStruct);
    let doc = match pyro_macro::format::documentation::generate_documented_impl(
        &item,
        &parse_quote!(::pyroduct),
        DocRec::NoReq,
    ) {
        Ok(v) => v,
        Err(error) => return error.to_compile_error().into(),
    };
    let from = match pyro_macro::format::from_row::from_row(&item, &parse_quote!(::pyroduct)) {
        Ok(v) => v,
        Err(error) => return error.to_compile_error().into(),
    };
    quote! {
        #doc
        #from
    }
    .into()
}

#[proc_macro_derive(RefFromRow)]
pub fn derive_ref_from_row(input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemStruct);
    let doc = match pyro_macro::format::documentation::ref_documentation(
        &item,
        &parse_quote!(::pyroduct),
        DocRec::NoReq,
    ) {
        Ok(v) => v,
        Err(error) => return error.to_compile_error().into(),
    };
    let from = match pyro_macro::format::from_row::ref_from_row(&item, &parse_quote!(::pyroduct)) {
        Ok(v) => v,
        Err(error) => return error.to_compile_error().into(),
    };
    quote! {
        #doc
        #from
    }
    .into()
}

#[proc_macro_derive(ToRow)]
pub fn derive_to_row(input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemStruct);
    match pyro_macro::format::to_row::to_row(&item, &parse_quote!(::pyroduct)) {
        Ok(v) => v.into(),
        Err(error) => error.to_compile_error().into(),
    }
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

    pyro_macro::module::expand(attrs, input_fn)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
