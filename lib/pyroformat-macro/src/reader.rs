//! `#[derive(Reader)]`: `impl GroupForeignReader for Foo`, which gets `Foo`
//! the blanket `ForeignReader` for `Foo` as a field of another struct, and
//! `impl Reader for Foo` (with `Layout`), which reads it at the root.
//!
//! Two views come with it, as `src/refs/list.rs` describes:
//!
//! * `FooRef<'a, W>`: one `Foo`, each field its own `ForeignReader::Ref`.
//! * `FooSlice<'a, W>`: a column of `Foo`s, each field its own
//!   `ForeignReader::NestedRef`. Row `i` is a `FooRef` of every column's
//!   row `i`.
//!
//! `W` is the word of the message being read, which is the outermost
//! struct's: a `Foo` nested in a `u32` struct reads as `FooRef<'a, u32>`
//! whatever `Foo`'s own word. It defaults to `Foo`'s own word, the one
//! `Foo::pyro_read` reads in.
//!
//! Both realize back into owned `Foo`s.
//!
//! It also emits `unsafe impl CovariantRef for Foo`, which `PyroCart` needs,
//! but only behind checks that fail the build when the promise doesn't hold:
//!
//! * Each field without a struct in it proves its own view is covariant: a
//!   function that hands back its `Ref<'long>` / `NestedRef<'long>` as a
//!   `'short` one. Inside a function body the projection is resolved to the
//!   real view type, so this compiles only if that type really is covariant.
//! * A field with a struct in it (`Inner`, `Vec<Inner>`, a map of `Inner`s)
//!   can't be proven that way: rustc sees `InnerRef<'a>` as invariant, since
//!   its own fields are projections. Instead a sentinel requires `Inner:
//!   CovariantRef`, which `Inner`'s own derive only emits after proving its
//!   fields the same way.
//! * Neither `FooRef<'static>` nor `FooSlice<'static>` may need dropping,
//!   checked as a const assertion.
//!
//! Both checks run at `Foo`'s own word, the only one `CovariantRef` (through
//! `Reader`) ever reads `Foo` in; `W` only picks the views' type parameters,
//! not their lifetimes.

use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned;

use crate::parse::{ResolvedType, StructInput};

pub(crate) fn expand(input: &StructInput) -> TokenStream {
    let name = &input.name;
    let pf = &input.krate;
    let vis = &input.vis;
    let ref_name = format_ident!("{}Ref", name);
    let slice_name = format_ident!("{}Slice", name);
    let word = input.word_ty();
    let sw = quote! { #pf::schema::StructureWord };
    let ref_doc = format!("One borrowed [`{name}`], read straight out of a buffer.");
    let slice_doc = format!("A borrowed column of [`{name}`]s: every field as its own column.");

    let idents: Vec<&syn::Ident> = input.fields.iter().map(|f| &f.ident).collect();
    let types: Vec<&syn::Type> = input.fields.iter().map(|f| &f.ty).collect();
    let column_len = quote! { #pf::ColumnLen::column_len };

    // A struct with no fields still needs its views to use `'a`, and its
    // column has no field to take a length from: the column keeps its own
    // row count instead, read from the group's slot, and its rows are
    // checked against it.
    let empty = idents.is_empty();
    let (marker_field, marker_value, slice_len) = match idents.first() {
        Some(first) => (quote! {}, quote! {}, quote! { #column_len(&self.#first) }),
        None => (
            quote! {
                #[doc(hidden)]
                pub _marker: ::core::marker::PhantomData<(&'a (), __W)>,
            },
            quote! { _marker: ::core::marker::PhantomData, },
            quote! { self._rows },
        ),
    };
    let (rows_field, rows_read, rows_get_check, rows_window, rows_window_check) = if empty {
        (
            quote! {
                #[doc(hidden)]
                pub _rows: usize,
            },
            // `__O` is the group's own inode.
            quote! {
                _rows: buffer.read_group_rows::<__O>()?,
            },
            quote! {
                if i >= self._rows {
                    return ::core::option::Option::None;
                }
            },
            quote! { _rows: end - start, },
            quote! {
                if start > end || end > self._rows {
                    return ::core::option::Option::None;
                }
            },
        )
    } else {
        Default::default()
    };

    // Every field after the first must hold as many rows as the first, as
    // `MapRef::new` checks a map's values against its keys. `index` is the
    // field's position in the struct.
    let rest_checks = idents.iter().enumerate().skip(1).map(|(index, ident)| {
        quote! {
            let have = #column_len(&slice.#ident);
            if have != expected {
                return ::core::result::Result::Err(#pf::errors::WireError::LengthMismatch {
                    expected,
                    have,
                    index: #index,
                });
            }
        }
    });
    let length_check = match idents.first() {
        Some(first) if idents.len() > 1 => quote! {
            let expected = #column_len(&slice.#first);
            #( #rest_checks )*
        },
        _ => quote! {},
    };

    // Every field's offset, as a type: `Shift<__O, { __READ_OFFSETS[k] }>`.
    //
    // Nothing is imported and every generic is `__`-prefixed: the field types
    // are spelled inside these impls, so any name brought in here would
    // shadow a user type of the same name.
    let (offset_table, offsets) =
        crate::offsets::offset_table(pf, &format_ident!("__READ_OFFSETS"), &types);
    // A column is read at its group's own inode, `__O`: the fields start
    // one past it.
    let iter_offsets: Vec<TokenStream> = (0..types.len())
        .map(|k| quote! { #pf::offsets::Shift<__O, { __READ_OFFSETS[#k] + 1 }> })
        .collect();

    // The `CovariantRef` proof: a covariance function per plain field, and
    // the sentinel for every struct nested anywhere in a field.
    let mut covariance_checks = TokenStream::new();
    let mut substructs = Vec::new();
    for field in &input.fields {
        let before = substructs.len();
        collect_structs(&field.resolved, &mut substructs);
        if substructs.len() > before {
            continue;
        }
        let ty = &field.ty;
        let check = format_ident!("__covariant_{}", field.ident);
        // Spanned at the field's type, so a view that isn't covariant is
        // reported on the field that has it.
        covariance_checks.extend(quote_spanned! { ty.span() =>
            #[allow(non_snake_case, dead_code)]
            fn #check<'__long: '__short, '__short>(
                row: <#ty as #pf::wire::readers::ForeignReader>::Ref<'__long, #word>,
                column: <#ty as #pf::wire::readers::ForeignReader>::NestedRef<'__long, #word>,
            ) -> (
                <#ty as #pf::wire::readers::ForeignReader>::Ref<'__short, #word>,
                <#ty as #pf::wire::readers::ForeignReader>::NestedRef<'__short, #word>,
            ) {
                (row, column)
            }
        });
    }

    quote! {
        #[doc = #ref_doc]
        #[derive(::core::fmt::Debug, ::core::clone::Clone)]
        #vis struct #ref_name<'a, __W: #sw = #word> {
            #( pub #idents: <#types as #pf::wire::readers::ForeignReader>::Ref<'a, __W>, )*
            #marker_field
        }

        #[doc = #slice_doc]
        #[derive(::core::fmt::Debug, ::core::clone::Clone)]
        #vis struct #slice_name<'a, __W: #sw = #word> {
            #( pub #idents: <#types as #pf::wire::readers::ForeignReader>::NestedRef<'a, __W>, )*
            #marker_field
            #rows_field
        }

        const _: () = {
            #offset_table

            // A struct with no fields reads nothing.
            #[allow(unused_variables)]
            impl #pf::wire::readers::group::GroupForeignReader for #name {

                type Ref<'a, __W: #sw> = #ref_name<'a, __W>;

                type Slice<'a, __W: #sw> = #slice_name<'a, __W>;

                fn group_read<
                    __Inodes: #pf::Layout,
                    __O: #pf::offsets::InodeOffset,
                    const __FIXED: usize,
                >(
                    buffer: #pf::wire::readers::SplitSource<'_, __Inodes, __FIXED>,
                ) -> ::core::result::Result<#ref_name<'_, __Inodes::Word>, #pf::errors::WireError> {
                    ::core::result::Result::Ok(#ref_name {
                        #( #idents: <#types as #pf::wire::readers::ForeignReader>::foreign_read::<__Inodes, #offsets, __FIXED>(buffer)?, )*
                        #marker_value
                    })
                }

                fn group_iter_read<
                    __Inodes: #pf::Layout,
                    __O: #pf::offsets::InodeOffset,
                    const __FIXED: usize,
                >(
                    buffer: #pf::wire::readers::SplitSource<'_, __Inodes, __FIXED>,
                ) -> ::core::result::Result<#slice_name<'_, __Inodes::Word>, #pf::errors::WireError> {
                    let slice = #slice_name {
                        #( #idents: <#types as #pf::wire::readers::ForeignReader>::foreign_iter_read::<__Inodes, #iter_offsets, __FIXED>(buffer)?, )*
                        #marker_value
                        #rows_read
                    };
                    #length_check
                    ::core::result::Result::Ok(slice)
                }
            }

            // Per struct, not blanket: the fixed region's size is a const
            // argument, which only a concrete type's const can be.
            impl #pf::Reader for #name {
                type Ref<'a> = #ref_name<'a, #word>;

                fn pyro_read(
                    buffer: &[u8],
                ) -> ::core::result::Result<#ref_name<'_, #word>, #pf::errors::WireError> {
                    let source = #pf::wire::readers::SplitSource::<
                        #name,
                        { <#name as #pf::Layout>::FIXED_SIZE },
                    >::from_payload(buffer)?;
                    <#name as #pf::wire::readers::group::GroupForeignReader>::group_read::<
                        #name,
                        #pf::offsets::Root,
                        { <#name as #pf::Layout>::FIXED_SIZE },
                    >(source)
                }
            }

            // Spelled out: a struct named `Rows` has a view named `RowsRef`.
            #[allow(unused_variables)]
            impl<'a, __W: #sw> #pf::RowsRef for #slice_name<'a, __W> {
                type Row = #ref_name<'a, __W>;

                // `RowsRef::..`, not method calls: every field column's
                // `RowsRef::Row` is that field's own `Ref`, while a plain
                // column's inherent `get` (`<[T]>::get`, `BitSlice::get`)
                // hands back a reference instead.
                fn len(&self) -> usize {
                    #slice_len
                }

                fn get(&self, i: usize) -> ::core::option::Option<#ref_name<'a, __W>> {
                    #rows_get_check
                    ::core::option::Option::Some(#ref_name {
                        #( #idents: #pf::RowsRef::get(&self.#idents, i)?, )*
                        #marker_value
                    })
                }

                fn window(&self, start: usize, end: usize) -> ::core::option::Option<Self> {
                    #rows_window_check
                    ::core::option::Option::Some(#slice_name {
                        #( #idents: #pf::RowsRef::window(&self.#idents, start, end)?, )*
                        #marker_value
                        #rows_window
                    })
                }
            }

            #[allow(unused_variables)]
            impl<'a, __W: #sw> #pf::Realize<#name> for #ref_name<'a, __W> {
                fn realize(&self) -> #name {
                    #name {
                        #( #idents: #pf::Realize::<#types>::realize(&self.#idents), )*
                    }
                }
            }

            #[allow(unused_variables)]
            impl<'a, __W: #sw> #pf::ColumnLen for #slice_name<'a, __W> {
                fn column_len(&self) -> usize {
                    #slice_len
                }
            }

            impl<'a, __W: #sw> #pf::Realize<::std::vec::Vec<#name>> for #slice_name<'a, __W> {
                fn realize(&self) -> ::std::vec::Vec<#name> {
                    ::core::iter::Iterator::collect(::core::iter::Iterator::map(
                        #pf::RowsRef::iter(self),
                        |row| #pf::Realize::<#name>::realize(&row),
                    ))
                }
            }

            #covariance_checks

            /// The sentinel: every struct nested in a field must itself be
            /// `CovariantRef`, and so have proven its own fields.
            #[allow(dead_code)]
            fn __assert_covariant_ref<T: #pf::buf::CovariantRef>() {}

            #[allow(dead_code)]
            fn __substructs_are_covariant_ref() {
                #( __assert_covariant_ref::<#substructs>(); )*
            }

            const _: () = ::core::assert!(
                !::core::mem::needs_drop::<#ref_name<'static>>(),
                "a derived view must not need dropping: PyroCart erases its lifetime"
            );
            const _: () = ::core::assert!(
                !::core::mem::needs_drop::<#slice_name<'static>>(),
                "a derived view must not need dropping: PyroCart erases its lifetime"
            );

            // SAFETY: covariance of every field's `Ref` and `NestedRef` is
            // proven above, per field or through the nested struct's own
            // `CovariantRef`, and neither view needs dropping.
            unsafe impl #pf::buf::CovariantRef for #name {}
        };
    }
}

/// Every struct type nested in a field, through `Vec`s, `Option`s and map
/// keys and values.
fn collect_structs(resolved: &ResolvedType, out: &mut Vec<syn::Type>) {
    match resolved {
        ResolvedType::Leaf { .. } => {}
        ResolvedType::Map { key, value, .. } => {
            collect_structs(key, out);
            collect_structs(value, out);
        }
        ResolvedType::Struct { ty, .. } => out.push(ty.clone()),
    }
}
