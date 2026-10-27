//! Every field's inode offset within its group, for the writer and reader
//! derives.
//!
//! Field `k` sits at the group's offset `__O` shifted by the inodes of the
//! fields before it. That shift is a prefix sum of the fields'
//! `INODE_COUNT`s, computed once into a const array, so each field's offset
//! is the one-level `Shift<__O, { TABLE[k] }>` however many fields come
//! before it. (Chaining the offsets as types, `Past<Past<__O, A>, B>`, nests
//! one level per field and runs into the recursion limit near 128 fields;
//! chaining them as consts, `OFF_k = OFF_{k-1} + ..`, does the same to
//! rustc's query depth.)
//!
//! A derived struct is never generic, so the table depends on no generic
//! parameter and `TABLE[k]` is an ordinary const argument.

use proc_macro2::TokenStream;
use quote::quote;

/// The `const` item holding the table, named `table`, and each field's
/// offset type in order. A field's inodes are counted by its
/// [`InodeWidth`](pyroformat::offsets::InodeWidth), the one count its writer
/// and reader share.
pub(crate) fn offset_table(
    pf: &TokenStream,
    table: &syn::Ident,
    types: &[&syn::Type],
) -> (TokenStream, Vec<TokenStream>) {
    let len = types.len();
    let item = quote! {
        #[allow(dead_code)]
        const #table: [usize; #len] = {
            let counts: [usize; #len] = [#( <#types as #pf::offsets::InodeWidth>::INODE_COUNT ),*];
            let mut offsets = [0usize; #len];
            let mut i = 1;
            while i < #len {
                offsets[i] = offsets[i - 1] + counts[i - 1];
                i += 1;
            }
            offsets
        };
    };
    let offsets = (0..len)
        .map(|k| quote! { #pf::offsets::Shift<__O, { #table[#k] }> })
        .collect();
    (item, offsets)
}
