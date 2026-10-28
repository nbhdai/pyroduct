//! `#[derive(Layout)]`: the struct's const inode table, as
//! `impl Layout for Foo`.
//!
//! The table is built by const evaluation, not by this macro: a nested
//! struct's inodes are copied in from `<Nested as Layout>::RAW`, which
//! rustc resolves after every derive has expanded, and `layout_in_place`
//! assigns every offset. So this macro only has to say what each field's
//! inodes are before layout.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::parse::{Leaf, ResolvedType, StructInput};

pub fn expand(input: &StructInput) -> TokenStream {
    let name = &input.name;
    let pf = &input.krate;
    // The structure word: `Layout::Word`, and what `INODES` are laid out in.
    let word = input.word_ty();
    let doc = &input.doc;
    let too_many = format!(
        "`{name}` needs more than MAX_INODES ({}) inodes, counting the inodes of every \
         struct nested in it",
        pyro_schema::MAX_INODES,
    );

    let mut raw_len = quote! { 0usize };
    let mut raw_writes = TokenStream::new();
    let mut flat_len = quote! { 0usize };
    let mut flat_name_writes = TokenStream::new();
    let mut flat_doc_writes = TokenStream::new();
    for field in &input.fields {
        let (len, writes) = raw_inodes(pf, &field.resolved);
        raw_len = quote! { #raw_len + #len };
        raw_writes.extend(writes);

        // A field's own name, then the names its type brings along (a
        // nested struct's whole table, a map's key's and value's).
        let field_name = field.ident.to_string();
        let field_doc = &field.doc;
        let (len, names) = flat_entries(pf, &field.resolved, &format_ident!("FLAT_NAMES"));
        let (_, docs) = flat_entries(pf, &field.resolved, &format_ident!("FLAT_DOCS"));
        flat_len = quote! { #flat_len + 1 + #len };
        flat_name_writes.extend(quote! {
            __flat[__i] = #field_name;
            __i += 1;
            #names
        });
        flat_doc_writes.extend(quote! {
            __flat[__i] = #field_doc;
            __i += 1;
            #docs
        });
    }

    // An empty struct has an empty table: `RAW_LEN` is zero and nothing is
    // written into it, which is fine, but leaves the builders' `mut`s unused.
    //
    // Nothing is imported and every local is `__`-prefixed: field types (and
    // an array's length) are spelled in here, and must still mean the user's.
    quote! {
        #[allow(unused_mut)]
        const _: () = {
            // Checked here, where every other table's length comes from,
            // so a struct past the limit fails with this message alone.
            const __RAW_LEN: usize = {
                let __len = #raw_len;
                ::core::assert!(__len <= #pf::schema::MAX_INODES, #too_many);
                __len
            };

            const __RAW: [#pf::schema::Inode; __RAW_LEN] = {
                const __EMPTY: #pf::schema::Inode = #pf::schema::Inode {
                    kind: #pf::schema::WireKind::Unknown,
                    kind_meta: #pf::schema::KindMeta::Primitive {
                        nesting: 0,
                        data_offset: 0,
                        topology_offset: 0,
                        fixed_size: 0,
                        nullable_bit_index: 0,
                        group: false,
                        group_nest: false,
                    },
                };
                let mut __raw = [__EMPTY; __RAW_LEN];
                let mut __i: usize = 0;
                #raw_writes
                ::core::assert!(__i == __RAW_LEN);
                __raw
            };

            const __LAID: ([#pf::schema::Inode; __RAW_LEN], usize) = {
                let mut __inodes = __RAW;
                let __fixed_size = #pf::schema::layout_in_place::<#word>(&mut __inodes);
                (__inodes, __fixed_size)
            };

            const __FLAT_LEN: usize = #flat_len;

            const __FLAT_NAMES: [&str; __FLAT_LEN] = {
                let mut __flat = [""; __FLAT_LEN];
                let mut __i: usize = 0;
                #flat_name_writes
                ::core::assert!(__i == __FLAT_LEN);
                __flat
            };

            const __FLAT_DOCS: [&str; __FLAT_LEN] = {
                let mut __flat = [""; __FLAT_LEN];
                let mut __i: usize = 0;
                #flat_doc_writes
                ::core::assert!(__i == __FLAT_LEN);
                __flat
            };

            // As a field of another struct: its own group inode, then its
            // fields' — the one width its writer and reader share.
            impl #pf::offsets::InodeWidth for #name {
                const INODE_COUNT: usize = 1 + __RAW_LEN;
            }

            impl #pf::Layout for #name {
                type Word = #word;
                const RAW_LEN: usize = __RAW_LEN;
                const RAW: &'static [#pf::schema::Inode] = &__RAW;
                const INODES: &'static [#pf::schema::Inode] = &__LAID.0;
                const FIXED_SIZE: usize = __LAID.1;
                const VALIDITY_START: usize =
                    #pf::schema::nullable_validity_start::<#word>(&__LAID.0);
                const DOC: &'static str = #doc;
                const FLAT_LEN: usize = __FLAT_LEN;
                const FLAT_NAMES: &'static [&'static str] = &__FLAT_NAMES;
                const FLAT_DOCS: &'static [&'static str] = &__FLAT_DOCS;
                const PYRO_TYPE: #pf::schema::PyroType =
                    #pf::schema::const_pyro_type!(<#name as #pf::Layout>::RAW);
                const SCHEMA_HASH: u64 =
                    #pf::schema::schema_hash(&<#name as #pf::Layout>::META, &__LAID.0);
            }
        };
    }
}

/// One field's inodes before layout: `(count, writes)`, where `writes` fills
/// `__raw[__i..]` and leaves `__i` just past them.
///
/// Every leaf is one inode. A map is its own inode, then its key's, then its
/// value's. A struct is its group inode, then its whole `RAW`.
fn raw_inodes(pf: &TokenStream, resolved: &ResolvedType) -> (TokenStream, TokenStream) {
    let nesting = resolved.vec_depth() as u8;
    match resolved {
        ResolvedType::Leaf { leaf, .. } => {
            let (kind, nesting, fixed_size) = match *leaf {
                Leaf::Prim(p) => (p.wire_kind(false), nesting, 0u16),
                Leaf::OptionPrim(p) => (p.wire_kind(true), nesting, 0),
                // A fixed array's `fixed_size` is its element count.
                Leaf::Array(n, p) => (p.wire_kind(false), nesting, n as u16),
                Leaf::OptionArray(n, p) => (p.wire_kind(true), nesting, n as u16),
                Leaf::Str => (pyro_schema::WireKind::Str, nesting, 0),
                Leaf::Bytes => (pyro_schema::WireKind::Bytes, nesting, 0),
                // A bool list whose list is the BitVec itself.
                Leaf::BitVec => (pyro_schema::WireKind::Bool, nesting + 1, 0),
            };
            let kind = format_ident!("{}", format!("{kind:?}"));
            let writes = quote! {
                __raw[__i] = #pf::schema::Inode {
                    kind: #pf::schema::WireKind::#kind,
                    kind_meta: #pf::schema::KindMeta::Primitive {
                        nesting: #nesting,
                        data_offset: 0,
                        topology_offset: 0,
                        fixed_size: #fixed_size,
                        nullable_bit_index: 0,
                        group: false,
                        group_nest: false,
                    },
                };
                __i += 1;
            };
            (quote! { 1 }, writes)
        }
        ResolvedType::Map { key, value, .. } => {
            let (key_len, key_writes) = raw_inodes(pf, key);
            let (value_len, value_writes) = raw_inodes(pf, value);
            let writes = quote! {
                __raw[__i] = #pf::schema::Inode {
                    kind: #pf::schema::WireKind::Map,
                    kind_meta: #pf::schema::KindMeta::Map {
                        nesting: #nesting,
                        topology_offset: 0,
                        group: false,
                        group_nest: false,
                    },
                };
                __i += 1;
                #key_writes
                #value_writes
            };
            (quote! { (1 + #key_len + #value_len) }, writes)
        }
        ResolvedType::Struct { ty, optional, .. } => {
            let kind = if *optional {
                quote! { #pf::schema::WireKind::OptionGroup }
            } else {
                quote! { #pf::schema::WireKind::Group }
            };
            let writes = quote! {
                __raw[__i] = #pf::schema::Inode {
                    kind: #kind,
                    kind_meta: #pf::schema::KindMeta::Group {
                        nesting: #nesting,
                        count: #pf::schema::count_children(<#ty as #pf::Layout>::RAW) as u16,
                        topology_offset: 0,
                        nullable_bit_index: 0,
                        group: false,
                        group_nest: false,
                    },
                };
                __i += 1;
                let __nested = <#ty as #pf::Layout>::RAW;
                let mut __j = 0;
                while __j < __nested.len() {
                    __raw[__i] = __nested[__j];
                    __i += 1;
                    __j += 1;
                }
            };
            (quote! { (1 + <#ty as #pf::Layout>::RAW_LEN) }, writes)
        }
    }
}

/// The flat name (or doc) entries a field's type adds after the field's own:
/// `(count, writes)`, where `writes` fills `__flat[__i..]`. `table` is
/// `FLAT_NAMES` or `FLAT_DOCS` (the `Layout` consts), whichever a nested struct's entries come from.
fn flat_entries(
    pf: &TokenStream,
    resolved: &ResolvedType,
    table: &syn::Ident,
) -> (TokenStream, TokenStream) {
    match resolved {
        ResolvedType::Leaf { .. } => (quote! { 0 }, TokenStream::new()),
        ResolvedType::Map { key, value, .. } => {
            let (key_len, key_writes) = flat_entries(pf, key, table);
            let (value_len, value_writes) = flat_entries(pf, value, table);
            (
                quote! { (#key_len + #value_len) },
                quote! { #key_writes #value_writes },
            )
        }
        ResolvedType::Struct { ty, .. } => {
            let writes = quote! {
                let __nested = <#ty as #pf::Layout>::#table;
                let mut __j = 0;
                while __j < __nested.len() {
                    __flat[__i] = __nested[__j];
                    __i += 1;
                    __j += 1;
                }
            };
            (quote! { <#ty as #pf::Layout>::FLAT_LEN }, writes)
        }
    }
}
