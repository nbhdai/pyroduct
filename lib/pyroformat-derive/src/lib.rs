//! Derives for `pyroformat`, one per trait:
//!
//! ```rust,ignore
//! #[derive(Layout, Writer, Reader)]
//! pub struct LogEntry {
//!     pub id: u64,
//!     pub level: Option<i32>,
//!     pub msg: String,
//! }
//! ```
//!
//! * [`Layout`](macro@Layout): the struct's const inode table,
//!   as `impl Layout`, including the `SCHEMA_HASH` `Writer::encode`
//!   seals a buffer with.
//!   Both of the others need it.
//! * [`Writer`](macro@Writer): `impl Writer`,
//! * [`Reader`](macro@Reader): `impl GroupForeignReader` plus the
//!   `LogEntryRef<'a>` / `LogEntrySlice<'a>` views, and so `Reader`.
//!
//! None of them know how to write or read a field: each field goes through
//! its own type's `ForeignWriter`/`ForeignReader`, so a nested struct is
//! just another field type that has those impls.

mod layout;
mod offsets;
mod parse;
mod reader;
mod writer;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

use parse::StructInput;

#[proc_macro_derive(Layout, attributes(pyro))]
pub fn derive_pyro_layout(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match StructInput::parse(&input) {
        Ok(input) => layout::expand(&input).into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// The strategies' group impls and `impl Writer` over the chosen one.
fn expand_writer(input: &StructInput) -> proc_macro2::TokenStream {
    let name = &input.name;
    let pf = &input.krate;

    let group = writer::expand_group(input);

    let write = writer::write_body(input);
    let variable_length = writer::variable_length_body(input);

    quote::quote! {
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

#[proc_macro_derive(Writer, attributes(pyro))]
pub fn derive_writer(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match StructInput::parse(&input) {
        Ok(input) => expand_writer(&input).into(),
        Err(err) => err.to_compile_error().into(),
    }
}

#[proc_macro_derive(Reader, attributes(pyro))]
pub fn derive_reader(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match StructInput::parse(&input) {
        Ok(input) => reader::expand(&input).into(),
        Err(err) => err.to_compile_error().into(),
    }
}
