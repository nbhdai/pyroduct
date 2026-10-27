//! The `pyroformat::wire::writers` strategy of `#[derive(Writer)]`:
//! `impl Group for Foo`, which gets `Foo` and `Option<Foo>` the blanket
//! `Writer` and `Nest` in `pyroformat::wire::writers::group`, and so
//! every list of them as a field of another struct, plus the body
//! `impl Writer for Foo` writes `Foo` as the root at `Root` with.
//!
//! Every method hands each field to its own `Writer`, in declaration
//! order. Each field sits at the group's offset `O` shifted past the fields
//! before it (`crate::offsets`). The fixed passes thread `byte_offset` from
//! one field to the next.
//!
//! Unlike the `writers` derive, a `Vec<X>` field needs no detour through
//! `[X]`: the pre writers implement `Vec<T>` directly.

use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned;

use crate::parse::{Leaf, ResolvedType, StructInput, is_vec_bool};

pub(crate) fn expand_group(input: &StructInput) -> TokenStream {
    let name = &input.name;
    let pf = &input.krate;

    let types: Vec<_> = input.fields.iter().map(|field| &field.ty).collect();
    let idents: Vec<_> = input.fields.iter().map(|field| &field.ident).collect();

    // Every field's offset, as a type: `Shift<__O, { __WRITE_OFFSETS[k] }>`.
    //
    // Nothing is imported and every generic is `__`-prefixed: the field types
    // are spelled inside these impls, so any name brought in here would
    // shadow a user type of the same name.
    let (offset_table, offsets) =
        crate::offsets::offset_table(pf, &format_ident!("__WRITE_OFFSETS"), &types);

    // `Vec<bool>` is packed bit by bit on write, where `pyroformat::BitVec`
    // is already packed and writes with one copy, so nudge toward it. A
    // proc macro can't emit a warning, so this is the usual workaround: a
    // `#[deprecated]` unit struct used right at the field's type, which
    // rustc then warns about. `#[pyro(allow(vec_bool))]` turns it off.
    let mut advisories = TokenStream::new();
    for (i, field) in input.fields.iter().enumerate() {
        if field.allow_vec_bool || !is_vec_bool(&field.resolved) {
            continue;
        }
        let span = field.ty.span();
        let advisory = format_ident!("__PyroformatVecBoolAdvisory_{}_{}", name, i, span = span);
        let note = format!(
            "field `{}`: `Vec<bool>` is packed bit-by-bit on write; use `pyroformat::BitVec` \
             (bitvec::vec::BitVec<u8, Lsb0>) for a memcpy-fast write/read path — silence with \
             #[pyro(allow(vec_bool))]",
            field.ident,
        );
        advisories.extend(quote_spanned! { span =>
            #[allow(non_camel_case_types)]
            #[doc(hidden)]
            #[deprecated(note = #note)]
            struct #advisory;

            #[allow(non_upper_case_globals, dead_code)]
            const _: fn() = || {
                let _ = #advisory;
            };
        });
    }

    // The fixed passes, one value form and one iterated form each: the
    // structure pass, then the payload passes from the widest alignment down.
    let mut fixed = TokenStream::new();
    for lane in ["structure", "16", "8", "4", "2", "1"] {
        let group_write = format_ident!("group_write_{}_fixed", lane);
        let group_iter_write = format_ident!("group_iter_write_{}_fixed", lane);
        let write = format_ident!("write_{}_fixed", lane);
        let iter_write = format_ident!("iter_write_{}_fixed", lane);
        fixed.extend(quote! {
            #[inline]
            fn #group_write<
                __Inodes: #pf::Layout,
                __O: #pf::offsets::InodeOffset,
                const __INODE_LEN: usize, const __FIXED: usize,
            >(
                &self,
                byte_offset: usize,
                buffer: &mut #pf::wire::writers::PreBuffer<'_, __Inodes, __FIXED, __INODE_LEN>,
            ) -> usize {
                let mut byte_offset = byte_offset;
                #(
                    byte_offset = <#types as #pf::wire::writers::Writer>::#write::<__Inodes, #offsets, __INODE_LEN, __FIXED>(
                        &self.#idents,
                        byte_offset,
                        buffer,
                    );
                )*
                byte_offset
            }

            #[inline]
            fn #group_iter_write<
                __Inodes: #pf::Layout,
                __O: #pf::offsets::InodeOffset,
                const __INODE_LEN: usize, const __FIXED: usize,
            >(
                byte_offset: usize,
                buffer: &mut #pf::wire::writers::PreBuffer<'_, __Inodes, __FIXED, __INODE_LEN>,
            ) -> usize {
                let mut byte_offset = byte_offset;
                #(
                    byte_offset = <#types as #pf::wire::writers::Writer>::#iter_write::<__Inodes, #offsets, __INODE_LEN, __FIXED>(
                        byte_offset,
                        buffer,
                    );
                )*
                byte_offset
            }
        });
    }

    // The absent passes: each field written as the value path writes a
    // child of a null group, through a value that holds nothing (see
    // `absent_value`). A field with nothing to write is left out.
    let absent: Vec<_> = input
        .fields
        .iter()
        .zip(&offsets)
        .filter_map(|(field, offset)| {
            absent_value(&field.ty, &field.resolved).map(|(ty, value)| (ty, value, offset))
        })
        .collect();
    let absent_tys: Vec<_> = absent.iter().map(|(ty, _, _)| ty).collect();
    let absent_values: Vec<_> = absent.iter().map(|(_, value, _)| value).collect();
    let absent_offsets: Vec<_> = absent.iter().map(|(_, _, offset)| *offset).collect();
    let mut absent_fixed = TokenStream::new();
    for lane in ["structure", "16", "8", "4", "2", "1"] {
        let group_write = format_ident!("group_write_{}_fixed_absent", lane);
        let write = format_ident!("write_{}_fixed", lane);
        absent_fixed.extend(quote! {
            #[inline]
            fn #group_write<
                __Inodes: #pf::Layout,
                __O: #pf::offsets::InodeOffset,
                const __INODE_LEN: usize, const __FIXED: usize,
            >(
                byte_offset: usize,
                buffer: &mut #pf::wire::writers::PreBuffer<'_, __Inodes, __FIXED, __INODE_LEN>,
            ) -> usize {
                let mut byte_offset = byte_offset;
                #(
                    byte_offset = <#absent_tys as #pf::wire::writers::Writer>::#write::<__Inodes, #absent_offsets, __INODE_LEN, __FIXED>(
                        &#absent_values,
                        byte_offset,
                        buffer,
                    );
                )*
                byte_offset
            }
        });
    }

    quote! {
        #advisories

        const _: () = {
            #offset_table

            // A struct with no fields writes nothing, so its methods leave
            // their arguments unused.
            #[allow(unused_variables, unused_mut)]
            impl #pf::wire::writers::Group for #name {

                #[inline]
                fn group_foreign_plan<
                    __Inodes: #pf::Layout,
                    __O: #pf::offsets::InodeOffset,
                    const __INODE_LEN: usize, const __FIXED: usize,
                >(
                    &self,
                    buffer: &mut #pf::wire::writers::PreBuffer<'_, __Inodes, __FIXED, __INODE_LEN>,
                ) -> usize {
                    0 #(
                        + <#types as #pf::wire::writers::Writer>::foreign_plan::<__Inodes, #offsets, __INODE_LEN, __FIXED>(
                            &self.#idents,
                            buffer,
                        )
                    )*
                }

                #[inline]
                fn group_write_variable<
                    __Inodes: #pf::Layout,
                    __O: #pf::offsets::InodeOffset,
                    const __INODE_LEN: usize, const __FIXED: usize,
                >(
                    &self,
                    buffer: &mut #pf::wire::writers::PreBuffer<'_, __Inodes, __FIXED, __INODE_LEN>,
                ) {
                    #(
                        <#types as #pf::wire::writers::Writer>::write_variable::<__Inodes, #offsets, __INODE_LEN, __FIXED>(
                            &self.#idents,
                            buffer,
                        );
                    )*
                }

                #[inline]
                fn group_iter_foreign_plan<
                    __Inodes: #pf::Layout,
                    __O: #pf::offsets::InodeOffset,
                    const __INODE_LEN: usize, const __FIXED: usize,
                >(
                    &self,
                    buffer: &mut #pf::wire::writers::PreBuffer<'_, __Inodes, __FIXED, __INODE_LEN>,
                ) -> usize {
                    0 #(
                        + <#types as #pf::wire::writers::Writer>::iter_foreign_plan::<__Inodes, #offsets, __INODE_LEN, __FIXED>(
                            &self.#idents,
                            buffer,
                        )
                    )*
                }

                #[inline]
                fn group_iter_write_variable<
                    __Inodes: #pf::Layout,
                    __O: #pf::offsets::InodeOffset,
                    const __INODE_LEN: usize, const __FIXED: usize,
                >(
                    &self,
                    buffer: &mut #pf::wire::writers::PreBuffer<'_, __Inodes, __FIXED, __INODE_LEN>,
                ) {
                    #(
                        <#types as #pf::wire::writers::Writer>::iter_write_variable::<__Inodes, #offsets, __INODE_LEN, __FIXED>(
                            &self.#idents,
                            buffer,
                        );
                    )*
                }

                #fixed

                #[inline]
                fn group_foreign_plan_absent<
                    __Inodes: #pf::Layout,
                    __O: #pf::offsets::InodeOffset,
                    const __INODE_LEN: usize, const __FIXED: usize,
                >(
                    buffer: &mut #pf::wire::writers::PreBuffer<'_, __Inodes, __FIXED, __INODE_LEN>,
                ) -> usize {
                    0 #(
                        + <#absent_tys as #pf::wire::writers::Writer>::foreign_plan::<__Inodes, #absent_offsets, __INODE_LEN, __FIXED>(
                            &#absent_values,
                            buffer,
                        )
                    )*
                }

                #[inline]
                fn group_write_variable_absent<
                    __Inodes: #pf::Layout,
                    __O: #pf::offsets::InodeOffset,
                    const __INODE_LEN: usize, const __FIXED: usize,
                >(
                    buffer: &mut #pf::wire::writers::PreBuffer<'_, __Inodes, __FIXED, __INODE_LEN>,
                ) {
                    #(
                        <#absent_tys as #pf::wire::writers::Writer>::write_variable::<__Inodes, #absent_offsets, __INODE_LEN, __FIXED>(
                            &#absent_values,
                            buffer,
                        );
                    )*
                }

                #absent_fixed
            }
        };
    }
}

/// What a field of an absent group is written as: `(type, value)`, the
/// value one of that type that holds nothing, or `None` for a field with
/// nothing to write.
///
/// * A list (`Vec`, a slice, a string, bytes, a `BitVec`) or a map: an
///   empty one of the field's own type. Its lanes get their start pointers
///   and nothing in them.
/// * An `Option`: `None`.
/// * A struct: `None::<Foo>`, whose `Option<Foo>` writer writes `Foo`'s
///   fields absent in turn.
/// * A scalar or a fixed array: nothing. Its slot stays as the fixed region
///   starts out, zeroed, which is what a null leaves it as. It is not
///   written as `None::<T>` either: a nullable scalar clears its validity
///   bit, and a scalar's inode has no bit of its own.
fn absent_value(ty: &syn::Type, resolved: &ResolvedType) -> Option<(TokenStream, TokenStream)> {
    let empty = || {
        Some((
            quote! { #ty },
            quote! { <#ty as ::core::default::Default>::default() },
        ))
    };
    let none = |ty: TokenStream| Some((ty, quote! { ::core::option::Option::None }));
    match resolved {
        ResolvedType::Leaf { vec_depth, .. } | ResolvedType::Map { vec_depth, .. }
            if *vec_depth > 0 =>
        {
            empty()
        }
        ResolvedType::Struct { vec_depth, .. } if *vec_depth > 0 => empty(),
        ResolvedType::Map { .. } => empty(),
        ResolvedType::Leaf { leaf, .. } => match leaf {
            Leaf::Str | Leaf::Bytes | Leaf::BitVec => empty(),
            Leaf::OptionPrim(_) | Leaf::OptionArray(..) => none(quote! { #ty }),
            Leaf::Prim(_) | Leaf::Array(..) => None,
        },
        ResolvedType::Struct { optional: true, .. } => none(quote! { #ty }),
        ResolvedType::Struct { ty: inner, .. } => none(quote! { ::core::option::Option<#inner> }),
    }
}

/// `pyro_write`'s body: plan the root group at `Root`, size the sink, lay out
/// every lane in the fixed passes, then fill in the variable data.
///
/// The structure is written in the struct's `Layout::Word`.
/// The plan returns an upper bound on the variable region, and the sink is
/// grown to it ahead of the fixed passes, since those already write past the
/// fixed region; nothing has been written yet, so a grown sink carries
/// nothing over. Where the passes end is the real length, and the sink is
/// shrunk to it once the variable data is in.
///
/// The plan's counts live in the `PreBuffer`, which borrows the sink, so
/// they are carried over by hand to the one built after the sink is grown.
pub(crate) fn write_body(input: &StructInput) -> TokenStream {
    let name = &input.name;
    let pf = &input.krate;
    let group = quote! { <#name as #pf::wire::writers::Group> };
    let root = quote! { #pf::offsets::Root };
    let pre = quote! { #pf::wire::writers::PreBuffer::<#name, __FIXED, __INODE_LEN> };
    quote! {
        type __Word = <#name as #pf::Layout>::Word;
        const __INODE_LEN: usize = <#name as #pf::Layout>::RAW_LEN;
        const __FIXED: usize = <#name as #pf::Layout>::FIXED_SIZE;
        const __PADDING: usize = #pf::schema::worst_padding::<__Word>(<#name as #pf::Layout>::INODES);
        // Every tier's reset, which the plan's per-node words leave out.
        const __RESETS: usize = #pf::schema::topology_reset_count(<#name as #pf::Layout>::INODES)
            * <__Word as #pf::schema::StructureWord>::SIZE;
        // The repeated bool and `Option` leaves, whose counts `check_sizes`
        // looks at: none for most layouts, so nothing is checked.
        const __CHECKED_LEN: usize =
            #pf::wire::writers::checked_sizes_len(<#name as #pf::Layout>::INODES);
        const __CHECKED: [usize; __CHECKED_LEN] =
            #pf::wire::writers::checked_sizes::<__CHECKED_LEN>(<#name as #pf::Layout>::INODES);

        let mut sizes: [#pf::wire::writers::InodeSize<__Word>; __INODE_LEN] =
            ::core::array::from_fn(|_| ::core::default::Default::default());
        let mut planned = #pre::new(buffer, &mut sizes);
        let planned_len = #group::group_foreign_plan::<#name, #root, __INODE_LEN, __FIXED>(self, &mut planned);
        #pf::wire::writers::check_sizes(&__CHECKED, planned.sizes)?;
        // The plan's counts are stored as words, which stop at `RESET`, and
        // the fixed passes lay out from them, so `end` below can't see a
        // count that got there. `check_sizes` rejects the bool and `Option`
        // leaves' counts that did; every other count is at most the plan's
        // own total, a `usize`, checked here. It leaves out padding, so this
        // never rejects a message that fits; `end` catches the padding.
        let max = #pf::schema::StructureWord::to_usize(<__Word as #pf::schema::StructureWord>::RESET);
        if __FIXED + planned_len + __RESETS >= max {
            return ::core::result::Result::Err(#pf::errors::WireError::OffsetOverflow {
                end: __FIXED + planned_len + __RESETS,
                max,
            });
        }
        let bound = planned_len + __PADDING + __RESETS;
        if __FIXED + bound > #pf::buf::PyroSink::len(buffer) {
            // `encode*` records only the trimmed length, which is under the
            // bound, so the next message of this size would grow again.
            <#name as #pf::Writer>::max_variable()
                .fetch_max(bound, ::core::sync::atomic::Ordering::Relaxed);
            #pf::buf::PyroSink::reallocate_sync(buffer, bound)?;
        }
        let payload = &mut #pre::new(buffer, &mut sizes);
        let mut end = __FIXED;
        end = #group::group_write_structure_fixed::<#name, #root, __INODE_LEN, __FIXED>(self, end, payload);
        end = #group::group_write_16_fixed::<#name, #root, __INODE_LEN, __FIXED>(self, end, payload);
        end = #group::group_write_8_fixed::<#name, #root, __INODE_LEN, __FIXED>(self, end, payload);
        end = #group::group_write_4_fixed::<#name, #root, __INODE_LEN, __FIXED>(self, end, payload);
        end = #group::group_write_2_fixed::<#name, #root, __INODE_LEN, __FIXED>(self, end, payload);
        end = #group::group_write_1_fixed::<#name, #root, __INODE_LEN, __FIXED>(self, end, payload);
        // Every offset the passes wrote is under `end`, so one check covers
        // them all: past the word's `RESET` some of them have wrapped.
        if end >= max {
            return ::core::result::Result::Err(#pf::errors::WireError::OffsetOverflow { end, max });
        }
        #group::group_write_variable::<#name, #root, __INODE_LEN, __FIXED>(self, payload);
        // The bound is loose: the message is just the part laid out.
        #pf::buf::PyroSink::shrink_to(buffer, end - __FIXED);
        ::core::result::Result::Ok(())
    }
}

/// `variable_length`'s body: the root group's lane lengths, plus the most
/// padding and tier resets its layout can need — an upper bound, not the
/// exact length.
///
/// The plan only counts into the `PreBuffer`'s sizes, so it runs over a
/// scratch fixed region and no variable one.
pub(crate) fn variable_length_body(input: &StructInput) -> TokenStream {
    let name = &input.name;
    let pf = &input.krate;
    quote! {
        type __Word = <#name as #pf::Layout>::Word;
        const __INODE_LEN: usize = <#name as #pf::Layout>::RAW_LEN;
        const __FIXED: usize = <#name as #pf::Layout>::FIXED_SIZE;
        const __PADDING: usize = #pf::schema::worst_padding::<__Word>(<#name as #pf::Layout>::INODES);
        // Every tier's reset, which the plan's per-node words leave out.
        const __RESETS: usize = #pf::schema::topology_reset_count(<#name as #pf::Layout>::INODES)
            * <__Word as #pf::schema::StructureWord>::SIZE;

        let mut scratch = [0u8; __FIXED];
        let mut sizes: [#pf::wire::writers::InodeSize<__Word>; __INODE_LEN] =
            ::core::array::from_fn(|_| ::core::default::Default::default());
        let mut planned =
            #pf::wire::writers::PreBuffer::<#name, __FIXED, __INODE_LEN>::from_payload(&mut scratch, &mut sizes);
        <#name as #pf::wire::writers::Group>::group_foreign_plan::<#name, #pf::offsets::Root, __INODE_LEN, __FIXED>(self, &mut planned) + __PADDING + __RESETS
    }
}
