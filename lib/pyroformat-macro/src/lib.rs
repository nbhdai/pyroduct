//! Code generation for `pyroformat`'s derives — [`Layout`](layout),
//! [`Writer`](expand_writer), [`Reader`](reader) — for `pyroformat-derive` to
//! call into.
//!
//! This crate is not a proc-macro crate: it has no `#[proc_macro_derive]` of
//! its own. It parses nothing from a `proc_macro::TokenStream` either — that
//! type only exists inside a proc-macro crate. `pyroformat-derive` parses the
//! `DeriveInput` into a [`parse::StructInput`] and hands it to the functions
//! here, which return plain `proc_macro2::TokenStream`.

pub mod layout;
mod offsets;
pub mod parse;
pub mod reader;
pub mod schema;
mod writer;

use proc_macro2::TokenStream;
use quote::quote;

use parse::StructInput;

/// The struct itself, every field `pub`, followed by everything
/// `#[derive(Layout, Writer, Reader)]` would generate for it: for a struct
/// that a macro builds rather than one the user wrote.
pub fn expand_struct(input: &StructInput) -> TokenStream {
    let name = &input.name;
    let vis = &input.vis;
    let fields = input.fields.iter().map(|f| {
        let ident = &f.ident;
        let ty = &f.ty;
        quote! { pub #ident: #ty }
    });
    let impls = expand_impls(input);

    quote! {
        #vis struct #name {
            #(#fields),*
        }

        #impls
    }
}

/// Everything `#[derive(Layout, Writer, Reader)]` generates for a struct,
/// without the struct: for a struct a macro passes through as written.
pub fn expand_impls(input: &StructInput) -> TokenStream {
    let layout = layout::expand(input);
    let writer = expand_writer(input);
    let reader = reader::expand(input);

    quote! {
        #layout
        #writer
        #reader
    }
}

/// The strategies' group impls and `impl Writer` over the chosen one.
pub fn expand_writer(input: &StructInput) -> TokenStream {
    let name = &input.name;
    let pf = &input.krate;

    let group = writer::expand_group(input);

    let write = writer::write_body(input);
    let variable_length = writer::variable_length_body(input);

    quote! {
        #group

        impl #pf::Writer for #name {
            fn max_variable() -> &'static ::std::sync::atomic::AtomicUsize {
                static __MAX_VARIABLE: ::std::sync::atomic::AtomicUsize =
                    ::std::sync::atomic::AtomicUsize::new(0);
                &__MAX_VARIABLE
            }

            fn pyro_write<__S: #pf::buf::PyroSink>(
                &self,
                buffer: &mut __S,
            ) -> ::core::result::Result<(), #pf::errors::WireError> {
                #write
            }

            fn fixed_len(&self) -> usize {
                <Self as #pf::Layout>::FIXED_SIZE
            }

            fn variable_length(&self) -> usize {
                #variable_length
            }
        }
    }
}
