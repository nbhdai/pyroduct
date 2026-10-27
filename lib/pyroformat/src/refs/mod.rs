//! Zero-copy views of nested columns, shaped like the `Vec`s they stand in
//! for.
//!
//! Every type here borrows straight out of a `.pyro` buffer — nothing is
//! copied or allocated until you ask for it (`realize()`, `to_vec()`, …). A
//! `Vec<Vec<f32>>` column is read back as a [`Nested<W, f32, 1>`], and
//! that view indexes, iterates, compares and hashes like the
//! `Vec<Vec<f32>>` it replaces.
//!
//! # Which view for which Rust type
//!
//! `N` is always the number of `Vec`s *above* the leaf, i.e. the number of
//! [`Topology`] levels the view carries, and is at least 1. A leaf run with
//! no topology above it is a plain slice, or a `…Slice` view where a plain
//! slice can't hold it (a validity mask, bit-packed arrays); those carry no
//! word type `W`.
//!
//! | Rust type                      | view                               | row (`get(i)` / iter item)          |
//! |--------------------------------|------------------------------------|-------------------------------------|
//! | `Vec<T>`                       | `&'a [T]`                          | `T`                                 |
//! | `Vec<Vec<T>>`                  | `Nested<W, T, 1>`                  | `&'a [T]`                           |
//! | `Vec<Vec<Vec<T>>>`             | `Nested<W, T, 2>`                  | `Nested<W, T, 1>`                   |
//! | `Vec<[T; M]>`                  | `&'a [[T; M]]`                     | `[T; M]`                            |
//! | `Vec<Vec<[T; M]>>`             | `Nested<W, [T; M], 1>`             | `&'a [[T; M]]`                      |
//! | `Vec<bool>`                    | `&'a BitSlice<u8, Lsb0>`           | `bool`                              |
//! | `Vec<Vec<bool>>`               | `BoolNested<W, 1>`                 | `&'a BitSlice<u8, Lsb0>`            |
//! | `Vec<[bool; M]>`               | `FixedBoolSlice<M>`                | `[bool; M]`                         |
//! | `Vec<Vec<[bool; M]>>`          | `FixedBoolNested<W, M, 1>`         | `FixedBoolSlice<M>`                 |
//! | `String`                       | `&'a str`                          | —                                   |
//! | `Vec<String>`                  | `StrNested<W, 1>`                  | `&'a str`                           |
//! | `Vec<Vec<String>>`             | `StrNested<W, 2>`                  | `StrNested<W, 1>`                   |
//! | `Vec<Option<T>>`               | `OptionSlice<T>`                   | `Option<T>`                         |
//! | `Vec<Vec<Option<T>>>`          | `OptionNested<W, T, 1>`            | `OptionSlice<T>`                    |
//! | `Vec<Option<bool>>`            | `OptionBoolSlice`                  | `Option<bool>`                      |
//! | `Vec<Vec<Option<bool>>>`       | `OptionBoolNested<W, 1>`           | `OptionBoolSlice`                   |
//! | `Vec<Option<[bool; M]>>`       | `OptionFixedBoolSlice<M>`          | `Option<[bool; M]>`                 |
//! | `Vec<Foo>` (derived `Foo`)     | `FooSlice<'a>`                     | `FooRef<'a>`                        |
//! | `Vec<Vec<Foo>>`                | `GroupNested<'a, W, FooSlice<'a>, 1>` | `FooSlice<'a>`                   |
//! | `Vec<Option<Foo>>`             | `OptionList<'a, FooSlice<'a>>`     | `Option<FooRef<'a>>`                |
//! | `Vec<Vec<Option<Foo>>>`        | `OptionNest<'a, W, FooSlice<'a>, 1>` | `OptionList<'a, FooSlice<'a>>`    |
//! | `Vec<[T; n]>`, runtime `n`     | `BorrowedFixedNest`                | `&'a [T]` of length `n`             |
//!
//! A view's row is always the view the row's own Rust type reads as: a
//! `Vec<Vec<T>>`'s rows are `&'a [T]`, exactly what a `Vec<T>` reads as. Fixed
//! arrays (`[T; M]`, `[bool; M]`) read by value, like the primitives in
//! them.
//!
//! The `{Variant}Nested` / `Fixed{Variant}Nested` / `Option{Variant}Nested`
//! / `Option{Variant}Slice` aliases (`F32Nested<'a, W, N>` =
//! `Nested<'a, W, f32, N>`, …) are the same types.
//!
//! Group columns (`FooSlice`, derived) and the [`GroupNested`] /
//! [`OptionList`] / [`OptionNest`] levels above them are described in
//! [`list`]'s module doc.
//!
//! Only the *leaf* is ever nullable: `Vec<Vec<Option<T>>>` is expressible,
//! `Vec<Option<Vec<T>>>` is not, so an option view's rows are plain
//! sub-views, never `Option<sub-view>`.
//!
//! # Getting one
//!
//! You normally don't build these by hand. A derive-generated reader, or
//! [`crate::wire::readers::ForeignReader::foreign_iter_read`], resolves the
//! inodes and hands you the right view — e.g. `<[f32]>::foreign_iter_read`
//! returns a `F32Nested<'_, W, 1>`. The value path (`wire::value_readers`)
//! builds the same views internally and feeds them to
//! [`Topology::eat_values`].
//!
//! Building one by hand is also fine, and is how the examples below work.
//! A [`Topology`] level is a cumulative-count array whose first word is
//! the word's `StructureWord::RESET` standing in for the implicit leading `0`:
//! `[R, 2, 3]` means "row 0 is `[0, 2)`, row 1 is `[2, 3)`". Each level's
//! counts index into the level below it; the last level's index into the
//! leaf slice.
//!
//! # Using a view
//!
//! ```
//! use pyroformat::refs::{Nested, Topology};
//! use pyroformat::schema::StructureWord;
//! const R: u32 = <u32 as StructureWord>::RESET;
//! use pyroformat::Realize;
//!
//! // [[1, 2], [3], []] — one topology level over a 3-element leaf.
//! let rows = [R, 2, 3, 3];
//! let leaf = [1u32, 2, 3];
//! let nest = Nested::new(Topology { topology: [&rows[..]] }, &leaf[..]).unwrap();
//!
//! // Length and indexing, like a slice.
//! assert_eq!(nest.len(), 3);
//! assert_eq!(nest.get(0), Some(&[1, 2][..]));
//! assert_eq!(nest.get(9), None);
//!
//! // A range keeps the depth: this is still a `Nested<W, u32, 1>`,
//! // borrowing the same leaf — O(1), nothing copied.
//! let tail = nest.get(1..).unwrap();
//! assert_eq!(tail, vec![vec![3], vec![]]);
//!
//! // Iteration yields rows; `for` works on the view or a reference to it.
//! let total: u32 = nest.iter().flatten().sum();
//! assert_eq!(total, 6);
//! for row in &nest {
//!     assert!(row.len() <= 2);
//! }
//!
//! // Compares against, and realizes to, the owned type.
//! assert_eq!(nest, vec![vec![1, 2], vec![3], vec![]]);
//! let owned: Vec<Vec<u32>> = nest.realize();
//! assert_eq!(owned.len(), 3);
//! ```
//!
//! Deeper views work the same, one level at a time — `get(i)` hands back
//! the next view down, which you index again:
//!
//! ```
//! use pyroformat::refs::{Nested, Topology};
//! use pyroformat::schema::StructureWord;
//! const R: u32 = <u32 as StructureWord>::RESET;
//!
//! // [[[1], [2, 3]], [[4, 5]]]
//! let outer = [R, 2, 3];
//! let inner = [R, 1, 3, 5];
//! let leaf = [1u32, 2, 3, 4, 5];
//! let nest = Nested::new(Topology { topology: [&outer[..], &inner[..]] }, &leaf[..]).unwrap();
//!
//! let first = nest.get(0).unwrap();          // Nested<W, u32, 1>
//! assert_eq!(first.get(1), Some(&[2, 3][..]));
//! assert_eq!(nest.get(1).unwrap().get(0), Some(&[4, 5][..]));
//! ```
//!
//! # Equality and hashing
//!
//! [`Nested`] implements `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash` and
//! `Debug` so that it is interchangeable with its realized type:
//! `nest == nest.realize()`, and `hash(nest) == hash(nest.realize())` under
//! any `Hasher`, so hash-based logic (sharding, dedup keys, content
//! fingerprints) gives the same answer whether it sees the view or the
//! realized value. (A `HashMap<Vec<Vec<u32>>, _>` still can't be *looked
//! up* with a view — that needs `Borrow`, which can't hand out a
//! `&Vec<_>`.) The impls exist only where the element type has them: no
//! `Hash` for `f32`, just like `Vec<f32>`.
//!
//! Every other view — [`BoolNested`], [`StrNested`], the option views,
//! [`GroupNested`], [`OptionList`] and [`OptionNest`] — implements
//! `PartialEq` (and `Eq` where the element type is `Eq`) against itself and
//! against the matching `Vec`s in both directions, at every depth: a
//! `StrNested<W, 2>` compares with a `Vec<Vec<String>>` or a
//! `Vec<Vec<&str>>`, an `OptionNest<'a, W, FooSlice<'a>, 1>` with a
//! `Vec<Vec<Option<Foo>>>`.
//!
//! # Strings are validated at load
//!
//! A [`StrNested`] is only built by [`Topology::new_str`], which checks
//! every string as UTF-8 in the same pass as it reads the topology — one
//! `from_utf8` over the whole blob plus a char-boundary test per string. A
//! column that loads is all valid, so its rows are plain `&str`s with no
//! further check; `get_bytes` hands out the same bytes unparsed.
//!
//! # Costs
//!
//! * `len`, `get(usize)`, `get(range)`/`window`, and advancing an iterator
//!   are `O(1)` for [`Nested`], [`BoolNested`] and [`StrNested`].
//! * The option views store only the `Some` values densely, so finding
//!   where sample `i` lives costs one popcount over the validity bits
//!   before it. Iterators avoid that by carrying a running dense cursor —
//!   prefer `iter()` over `get(i)` in a loop.
//! * Nothing ever allocates except `realize()`.
//!
//! # Empty levels
//!
//! The writer omits a topology level entirely when every list at that
//! level is empty (`TopologyCache::write`). [`Topology::descend`] and
//! [`Topology::window`] treat that as empty children, not as
//! out-of-bounds, so every view here does too: `[[], []]` at depth 2 still
//! iterates twice, yielding two empty `Nested<W, T, 1>`s.

mod bool;
mod bool_option;
pub mod dynamic_fixed;
mod fixed;
mod list;
mod map;
mod nesting;
mod option_list;
mod primitive;
mod primitive_option;
mod str_leaf;

pub use bool::{
    BoolNested, BoolNestedIter, FixedBoolNested, FixedBoolNestedIter, FixedBoolSlice,
    FixedBoolSliceIter,
};
pub use bool_option::{
    OptionBoolNested, OptionBoolNestedIter, OptionBoolSlice, OptionBoolSliceIter,
    OptionFixedBoolNested, OptionFixedBoolNestedIter, OptionFixedBoolSlice,
    OptionFixedBoolSliceIter,
};
pub use fixed::{FixedSlice, OptionFixedSlice, OptionFixedSliceIter};
pub use list::{GroupNested, GroupNestedIter};
pub use map::{MapIter, MapRef, NestedMapIter, NestedMapRef};
pub use nesting::{Topology, TopologyDecendIter, TopologyIter};
pub use option_list::{OptionList, OptionListIter, OptionNest, OptionNestIter};
pub use primitive::{
    F16Nested, F32Nested, F64Nested, FixedF16Nested, FixedF32Nested, FixedF64Nested, FixedI8Nested,
    FixedI16Nested, FixedI32Nested, FixedI64Nested, FixedTimeNested, FixedU8Nested, FixedU16Nested,
    FixedU32Nested, FixedU64Nested, I8Nested, I16Nested, I32Nested, I64Nested, NestIndex, Nested,
    NestedIter, TimeNested, U8Nested, U16Nested, U32Nested, U64Nested,
};
pub use primitive_option::{
    OptionF16Nested, OptionF16Slice, OptionF32Nested, OptionF32Slice, OptionF64Nested,
    OptionF64Slice, OptionI8Nested, OptionI8Slice, OptionI16Nested, OptionI16Slice,
    OptionI32Nested, OptionI32Slice, OptionI64Nested, OptionI64Slice, OptionNested,
    OptionNestedIter, OptionSlice, OptionSliceIter, OptionTimeNested, OptionTimeSlice,
    OptionU8Nested, OptionU8Slice, OptionU16Nested, OptionU16Slice, OptionU32Nested,
    OptionU32Slice, OptionU64Nested, OptionU64Slice,
};

pub use str_leaf::{StrNested, StrNestedIter};
pub(crate) use str_leaf::{check_str, check_strs};
// =============================================================================
// ColumnLen — every column view's outermost length. Most delegate to the
// view's own `len`, which (unlike `RowsRef::len`) is defined at every
// depth. `GroupNested`, `OptionNest` and `NestedMapRef` only define `len`
// per literal depth, so theirs reads the topology directly.
// =============================================================================

mod column_len {
    use super::*;
    use crate::ColumnLen;
    use bitvec::{order::Lsb0, slice::BitSlice};
    use pyro_schema::StructureWord;

    macro_rules! column_len {
        ($([$($g:tt)*] $ty:ty => |$s:ident| $len:expr;)*) => {
            $(
                impl<$($g)*> ColumnLen for $ty {
                    #[inline]
                    fn column_len(&self) -> usize {
                        let $s = self;
                        $len
                    }
                }
            )*
        };
    }

    column_len! {
        ['a, T] &'a [T] => |s| s.len();
        ['a] &'a BitSlice<u8, Lsb0> => |s| BitSlice::len(s);
        ['a, T] OptionSlice<'a, T> => |s| s.len();
        ['a, T, const N: usize] FixedSlice<'a, T, N> => |s| s.len();
        ['a, T, const N: usize] OptionFixedSlice<'a, T, N> => |s| s.len();
        ['a] OptionBoolSlice<'a> => |s| s.len();
        ['a, const M: usize] FixedBoolSlice<'a, M> => |s| s.len();
        ['a, const M: usize] OptionFixedBoolSlice<'a, M> => |s| s.len();
        ['a, T] OptionList<'a, T> => |s| s.len;
        ['a, W: StructureWord, T, const N: usize] Nested<'a, W, T, N> => |s| s.len();
        ['a, W: StructureWord, T, const N: usize] OptionNested<'a, W, T, N> => |s| s.len();
        ['a, W: StructureWord, const N: usize] BoolNested<'a, W, N> => |s| s.len();
        ['a, W: StructureWord, const M: usize, const N: usize] FixedBoolNested<'a, W, M, N> => |s| s.len();
        ['a, W: StructureWord, const N: usize] OptionBoolNested<'a, W, N> => |s| s.len();
        ['a, W: StructureWord, const M: usize, const N: usize] OptionFixedBoolNested<'a, W, M, N> => |s| s.len();
        ['a, W: StructureWord, const N: usize] StrNested<'a, W, N> => |s| s.len();
        ['a, W: StructureWord, T, const N: usize] GroupNested<'a, W, T, N> => |s| s.topology.len();
        ['a, W: StructureWord, T, const N: usize] OptionNest<'a, W, T, N> => |s| s.topology.len();
        ['a, W: StructureWord, K, V, const N: usize] NestedMapRef<'a, W, K, V, N> => |s| s.topology.len();
    }
}
