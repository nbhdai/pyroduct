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
//!
//! The actual code generation lives in `pyroformat-macro`; this crate only
//! parses the `DeriveInput` and hands it off.

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

use pyroformat_macro::parse::StructInput;

#[proc_macro_derive(Layout, attributes(pyro))]
pub fn derive_pyro_layout(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match StructInput::parse(&input) {
        Ok(input) => pyroformat_macro::layout::expand(&input).into(),
        Err(err) => err.to_compile_error().into(),
    }
}

#[proc_macro_derive(Writer, attributes(pyro))]
pub fn derive_writer(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match StructInput::parse(&input) {
        Ok(input) => pyroformat_macro::expand_writer(&input).into(),
        Err(err) => err.to_compile_error().into(),
    }
}

#[proc_macro_derive(Reader, attributes(pyro))]
pub fn derive_reader(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match StructInput::parse(&input) {
        Ok(input) => pyroformat_macro::reader::expand(&input).into(),
        Err(err) => err.to_compile_error().into(),
    }
}
