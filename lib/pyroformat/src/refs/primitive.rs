//! Required primitive leaves: [`Nested`] for every fixed-width
//! scalar (and `[T; M]` arrays of them), and the `Nested{Variant}Ref` /
//! `NestedFixed{Variant}Ref` aliases the readers hand back. `bool` lives in
//! [`super::bool`].
//!
//! See the [module-level guide](super) for which view matches which Rust
//! type and how to use them; this file documents the details.
//!
//! # Layout
//!
//! A leaf run (the `slice`) holding every element of the column back to
//! back, plus a [`Topology`] whose levels say where each list starts and
//! ends. Level `k`'s cumulative counts index into level `k + 1`; the last
//! level's index into the leaf run. A view never copies or re-bases either:
//! narrowing to row `i` narrows only the topology (and, at the last level,
//! slices the leaf run), so every sub-view is `Copy` and `O(1)` to make.

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::{
    Bound, Range, RangeBounds, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive,
};

use crate::PyroValue;
use crate::errors::WireError;
use crate::interface::{Realize, RowsRef};
use crate::refs::Topology;
use crate::value::Time;
use pyro_schema::StructureWord;

/// [`Nested`] — a zero-copy `Vec<…<Vec<T>>…>` over a borrowed leaf
/// slice plus a [`Topology`].
///
/// `N` counts topology levels, i.e. the `Vec`s *above* the leaf slice:
///
/// | view                    | behaves like        | `get(usize)` / iter item |
/// |-------------------------|---------------------|--------------------------|
/// | `Nested<W, T, 1>`    | `Vec<Vec<T>>`       | `&'a [T]`                |
/// | `Nested<W, T, 2>`    | `Vec<Vec<Vec<T>>>`  | `Nested<W, T, 1>`     |
/// | …up to `N = 5` ([`crate::wire::nesting::MAX_VALUE_DEPTH`]) | | |
///
/// A `Vec<T>` has no topology, so it is never a `Nested`: it reads as the
/// plain `&'a [T]`, which is also what a `Nested<W, T, 1>`'s rows are.
///
/// Every view compares, orders, hashes and `Debug`-prints exactly like the
/// owned type it [`Realize`]s to: `nest == nest.realize()`, and
/// `hash(nest) == hash(nest.realize())` for any [`Hasher`], because the
/// leaf rows are `&[T]`s, which hash through `<[T] as Hash>` (length prefix
/// + `T::hash_slice`) just as `Vec<T>` does, and every level above writes
/// the same `write_usize(len)` length prefix `[Vec<_>]` does.
///
/// [`Nested::get`] takes either a `usize` (one element, one level
/// down) or any range (a same-depth window), like `<[T]>::get`.
///
/// Each level's child type is `Nested<W, T, N - 1>`, which stable Rust
/// can't express generically over `N`, so the per-level impl<W: StructureWord>s are stamped
/// out by `nest_level!`.
pub struct Nested<'a, W: StructureWord, T, const N: usize> {
    topology: Topology<'a, W, N>,
    slice: &'a [T],
}

// Manual so `T` needn't be `Clone`/`Copy` — only references are copied.
impl<'a, W: StructureWord, T, const N: usize> Clone for Nested<'a, W, T, N> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, W: StructureWord, T, const N: usize> Copy for Nested<'a, W, T, N> {}

impl<'a, W: StructureWord, T, const N: usize> Nested<'a, W, T, N> {
    pub fn new(topology: Topology<'a, W, N>, slice: &'a [T]) -> Result<Self, WireError> {
        if topology.total() != slice.len() {
            return Err(WireError::InvalidTopologyLen {
                topo: topology.total(),
                data: slice.len(),
            });
        }
        Ok(Self { topology, slice })
    }

    /// Element count at this view's own (outermost) level.
    pub fn len(&self) -> usize {
        self.topology.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// `usize` → one element, a level down; any range → a same-depth
    /// window. `None` when out of bounds, like `<[T]>::get`.
    pub fn get<I: NestIndex<'a, W, T, N>>(&self, index: I) -> Option<I::Output> {
        index.get(self)
    }

    /// Elements `[start, end)`, same depth. `O(1)`: the leaf slice is
    /// shared, only the outermost boundary array is narrowed.
    pub fn window(&self, start: usize, end: usize) -> Option<Self> {
        if start > end || end > self.len() {
            return None;
        }
        Some(Nested {
            topology: self.topology.window(start, end)?,
            slice: self.slice,
        })
    }

    pub fn iter(&self) -> NestedIter<'a, W, T, N> {
        NestedIter {
            nest: *self,
            front: 0,
            back: self.len(),
        }
    }
}

// =============================================================================
// Indexing
// =============================================================================

/// What [`Nested::get`] accepts — the nest analogue of
/// `std::slice::SliceIndex`.
pub trait NestIndex<'a, W: StructureWord, T, const N: usize> {
    type Output;
    fn get(self, nest: &Nested<'a, W, T, N>) -> Option<Self::Output>;
}

fn resolve_range<R: RangeBounds<usize>>(range: &R, len: usize) -> Option<(usize, usize)> {
    let start = match range.start_bound() {
        Bound::Included(&s) => s,
        Bound::Excluded(&s) => s.checked_add(1)?,
        Bound::Unbounded => 0,
    };
    let end = match range.end_bound() {
        Bound::Included(&e) => e.checked_add(1)?,
        Bound::Excluded(&e) => e,
        Bound::Unbounded => len,
    };
    Some((start, end))
}

// One impl per range type rather than a `RangeBounds` blanket: a blanket
// would overlap the `usize` impls (coherence can't rule out an upstream
// `impl RangeBounds<usize> for usize`).
macro_rules! nest_range_index {
    ($($range:ty),* $(,)?) => {
        $(
            impl<'a, W: StructureWord, T: 'a, const N: usize> NestIndex<'a, W, T, N> for $range {
                type Output = Nested<'a, W, T, N>;

                fn get(self, nest: &Nested<'a, W, T, N>) -> Option<Self::Output> {
                    let (start, end) = resolve_range(&self, nest.len())?;
                    nest.window(start, end)
                }
            }
        )*
    };
}

nest_range_index!(
    Range<usize>,
    RangeInclusive<usize>,
    RangeFrom<usize>,
    RangeTo<usize>,
    RangeToInclusive<usize>,
    RangeFull,
);

// =============================================================================
// Iteration
// =============================================================================

pub struct NestedIter<'a, W: StructureWord, T, const N: usize> {
    nest: Nested<'a, W, T, N>,
    front: usize,
    back: usize,
}

impl<'a, W: StructureWord, T, const N: usize> Iterator for NestedIter<'a, W, T, N>
where
    usize: NestIndex<'a, W, T, N>,
{
    type Item = <usize as NestIndex<'a, W, T, N>>::Output;

    #[cfg_attr(
        feature = "trace-logging",
        tracing::instrument(level = "trace", skip_all)
    )]
    fn next(&mut self) -> Option<Self::Item> {
        if self.front >= self.back {
            return None;
        }
        let i = self.front;
        self.front += 1;
        self.nest.get(i)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.back - self.front;
        (n, Some(n))
    }
}

impl<'a, W: StructureWord, T, const N: usize> DoubleEndedIterator for NestedIter<'a, W, T, N>
where
    usize: NestIndex<'a, W, T, N>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.front >= self.back {
            return None;
        }
        self.back -= 1;
        self.nest.get(self.back)
    }
}

impl<'a, W: StructureWord, T, const N: usize> ExactSizeIterator for NestedIter<'a, W, T, N> where
    usize: NestIndex<'a, W, T, N>
{
}

impl<'a, W: StructureWord, T, const N: usize> IntoIterator for Nested<'a, W, T, N>
where
    usize: NestIndex<'a, W, T, N>,
{
    type Item = <usize as NestIndex<'a, W, T, N>>::Output;
    type IntoIter = NestedIter<'a, W, T, N>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, 'n, W: StructureWord, T, const N: usize> IntoIterator for &'n Nested<'a, W, T, N>
where
    usize: NestIndex<'a, W, T, N>,
{
    type Item = <usize as NestIndex<'a, W, T, N>>::Output;
    type IntoIter = NestedIter<'a, W, T, N>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// =============================================================================
// N = 1 — `Vec<Vec<T>>`: rows are plain `&'a [T]`
// =============================================================================

impl<'a, W: StructureWord, T: 'a> NestIndex<'a, W, T, 1> for usize {
    type Output = &'a [T];

    fn get(self, nest: &Nested<'a, W, T, 1>) -> Option<&'a [T]> {
        let (start, end) = nest.topology.leaf_range(self)?;
        nest.slice.get(start..end)
    }
}

// =============================================================================
// N >= 1 — everything that is defined in terms of the child type
// =============================================================================

macro_rules! nest_common {
    ($n:literal) => {
        impl<'a, 'b, W: StructureWord, T: PartialEq<U>, U> PartialEq<Nested<'b, W, U, $n>>
            for Nested<'a, W, T, $n>
        {
            fn eq(&self, other: &Nested<'b, W, U, $n>) -> bool {
                self.len() == other.len() && self.iter().zip(other.iter()).all(|(a, b)| a == b)
            }
        }

        impl<'a, W: StructureWord, T: Eq> Eq for Nested<'a, W, T, $n> {}

        impl<'a, W: StructureWord, T, V> PartialEq<[V]> for Nested<'a, W, T, $n>
        where
            <usize as NestIndex<'a, W, T, $n>>::Output: PartialEq<V>,
        {
            fn eq(&self, other: &[V]) -> bool {
                self.len() == other.len() && self.iter().zip(other).all(|(a, b)| a == *b)
            }
        }

        impl<'a, W: StructureWord, T, V> PartialEq<Vec<V>> for Nested<'a, W, T, $n>
        where
            <usize as NestIndex<'a, W, T, $n>>::Output: PartialEq<V>,
        {
            fn eq(&self, other: &Vec<V>) -> bool {
                *self == *other.as_slice()
            }
        }

        impl<'a, W: StructureWord, T, V> PartialEq<Nested<'a, W, T, $n>> for Vec<V>
        where
            <usize as NestIndex<'a, W, T, $n>>::Output: PartialEq<V>,
        {
            fn eq(&self, other: &Nested<'a, W, T, $n>) -> bool {
                *other == *self.as_slice()
            }
        }

        impl<'a, W: StructureWord, T: PartialOrd> PartialOrd for Nested<'a, W, T, $n> {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                self.iter().partial_cmp(other.iter())
            }
        }

        impl<'a, W: StructureWord, T: Ord> Ord for Nested<'a, W, T, $n> {
            fn cmp(&self, other: &Self) -> Ordering {
                self.iter().cmp(other.iter())
            }
        }

        impl<'a, W: StructureWord, T: Hash> Hash for Nested<'a, W, T, $n> {
            fn hash<H: Hasher>(&self, state: &mut H) {
                // `<[Vec<_>] as Hash>`: `write_length_prefix` (whose stable
                // behaviour is `write_usize`), then each element in turn.
                state.write_usize(self.len());
                for child in self.iter() {
                    child.hash(state);
                }
            }
        }

        impl<'a, W: StructureWord, T: fmt::Debug> fmt::Debug for Nested<'a, W, T, $n> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_list().entries(self.iter()).finish()
            }
        }
    };
}

nest_common!(1);
nest_common!(2);
nest_common!(3);
nest_common!(4);
nest_common!(5);

impl<'a, W: StructureWord, T: 'a, const N: usize> Nested<'a, W, T, N> {
    /// The elements of this view's outermost level as nested
    /// `PyroValue::List`s, each innermost list (a `&[T]` row) through `row`
    /// — e.g. `PyroValue::Bytes` for a blob column, which reads as the same
    /// `Nested<W, u8, N>` a `u8` list does.
    ///
    /// `levels` is how many of the view's `N` levels the data uses
    /// (`1 <= levels <= N`): a view read at [`crate::MAX_TIERS`] levels for a
    /// schema only known at runtime uses fewer.
    pub fn into_value_with(
        &self,
        levels: usize,
        row: impl Fn(&'a [T]) -> PyroValue<'a>,
    ) -> Vec<PyroValue<'a>> {
        let slice = self.slice;
        self.topology.fold_rows(levels, |leaf| {
            Nested::<'a, W, T, 1> {
                topology: leaf,
                slice,
            }
            .iter()
            .map(row)
        })
    }
}

impl<'a, W: StructureWord, T: 'a, const N: usize> Nested<'a, W, T, N>
where
    PyroValue<'a>: From<&'a [T]>,
{
    /// [`Self::into_value_with`], each innermost list a borrowed
    /// `PrimitiveList`.
    pub fn into_value(&self, levels: usize) -> Vec<PyroValue<'a>> {
        self.into_value_with(levels, PyroValue::from)
    }
}

impl<'a, W: StructureWord, T: Clone> Realize<Vec<Vec<T>>> for Nested<'a, W, T, 1> {
    fn realize(&self) -> Vec<Vec<T>> {
        self.iter().map(<[T]>::to_vec).collect()
    }
}

// A list of byte blobs reads as the same `Nested<W, u8, 1>` a
// `Vec<Vec<u8>>` does, and realizes into whichever blob type it was.

impl<'a, W: StructureWord> Realize<Vec<Box<[u8]>>> for Nested<'a, W, u8, 1> {
    fn realize(&self) -> Vec<Box<[u8]>> {
        self.iter().map(Box::from).collect()
    }
}

impl<'a, W: StructureWord> Realize<Vec<bytes::Bytes>> for Nested<'a, W, u8, 1> {
    fn realize(&self) -> Vec<bytes::Bytes> {
        self.iter().map(bytes::Bytes::copy_from_slice).collect()
    }
}

impl<'a, W: StructureWord> Realize<Vec<bytes::BytesMut>> for Nested<'a, W, u8, 1> {
    fn realize(&self) -> Vec<bytes::BytesMut> {
        self.iter().map(bytes::BytesMut::from).collect()
    }
}

// =============================================================================
// N >= 2 — children are themselves `Nested<W, T, N - 1>`
// =============================================================================

macro_rules! nest_level {
    ($n:literal => $m:literal) => {
        impl<'a, W: StructureWord, T: 'a> NestIndex<'a, W, T, $n> for usize {
            type Output = Nested<'a, W, T, $m>;

            fn get(self, nest: &Nested<'a, W, T, $n>) -> Option<Self::Output> {
                Some(Nested {
                    topology: nest.topology.descend(self)?,
                    slice: nest.slice,
                })
            }
        }

        impl<'a, W: StructureWord, T> Nested<'a, W, T, $n> {
            /// Alias for `get(i)` with a `usize`.
            pub fn descend(&self, i: usize) -> Option<Nested<'a, W, T, $m>> {
                self.get(i)
            }
        }

        impl<'a, W: StructureWord, T: Clone, O> Realize<Vec<O>> for Nested<'a, W, T, $n>
        where
            Nested<'a, W, T, $m>: Realize<O>,
        {
            fn realize(&self) -> Vec<O> {
                self.iter().map(|child| child.realize()).collect()
            }
        }
    };
}

nest_level!(2 => 1);
nest_level!(3 => 2);
nest_level!(4 => 3);
nest_level!(5 => 4);

// =============================================================================
// RowsRef — forwards to each level's own `get` (not `descend`)
// =============================================================================

macro_rules! nest_rows_ref {
    ($($n:literal => $row:ty),* $(,)?) => {
        $(
            impl<'a, W: StructureWord, T: 'a> RowsRef for Nested<'a, W, T, $n> {
                type Row = $row;

                fn len(&self) -> usize {
                    Self::len(self)
                }

                fn get(&self, i: usize) -> Option<$row> {
                    Self::get(self, i)
                }

                fn window(&self, start: usize, end: usize) -> Option<Self> {
                    Self::window(self, start, end)
                }
            }
        )*
    };
}

nest_rows_ref!(
    1 => &'a [T],
    2 => Nested<'a, W, T, 1>,
    3 => Nested<'a, W, T, 2>,
    4 => Nested<'a, W, T, 3>,
    5 => Nested<'a, W, T, 4>,
);

pub type F16Nested<'a, W, const N: usize> = Nested<'a, W, half::f16, N>;
pub type F32Nested<'a, W, const N: usize> = Nested<'a, W, f32, N>;
pub type F64Nested<'a, W, const N: usize> = Nested<'a, W, f64, N>;
pub type I8Nested<'a, W, const N: usize> = Nested<'a, W, i8, N>;
pub type I16Nested<'a, W, const N: usize> = Nested<'a, W, i16, N>;
pub type I32Nested<'a, W, const N: usize> = Nested<'a, W, i32, N>;
pub type I64Nested<'a, W, const N: usize> = Nested<'a, W, i64, N>;
pub type U8Nested<'a, W, const N: usize> = Nested<'a, W, u8, N>;
pub type U16Nested<'a, W, const N: usize> = Nested<'a, W, u16, N>;
pub type U32Nested<'a, W, const N: usize> = Nested<'a, W, u32, N>;
pub type U64Nested<'a, W, const N: usize> = Nested<'a, W, u64, N>;
pub type TimeNested<'a, W, const N: usize> = Nested<'a, W, Time, N>;

pub type FixedF16Nested<'a, W, const M: usize, const N: usize> = Nested<'a, W, [half::f16; M], N>;
pub type FixedF32Nested<'a, W, const M: usize, const N: usize> = Nested<'a, W, [f32; M], N>;
pub type FixedF64Nested<'a, W, const M: usize, const N: usize> = Nested<'a, W, [f64; M], N>;
pub type FixedI8Nested<'a, W, const M: usize, const N: usize> = Nested<'a, W, [i8; M], N>;
pub type FixedI16Nested<'a, W, const M: usize, const N: usize> = Nested<'a, W, [i16; M], N>;
pub type FixedI32Nested<'a, W, const M: usize, const N: usize> = Nested<'a, W, [i32; M], N>;
pub type FixedI64Nested<'a, W, const M: usize, const N: usize> = Nested<'a, W, [i64; M], N>;
pub type FixedU8Nested<'a, W, const M: usize, const N: usize> = Nested<'a, W, [u8; M], N>;
pub type FixedU16Nested<'a, W, const M: usize, const N: usize> = Nested<'a, W, [u16; M], N>;
pub type FixedU32Nested<'a, W, const M: usize, const N: usize> = Nested<'a, W, [u32; M], N>;
pub type FixedU64Nested<'a, W, const M: usize, const N: usize> = Nested<'a, W, [u64; M], N>;
pub type FixedTimeNested<'a, W, const M: usize, const N: usize> = Nested<'a, W, [Time; M], N>;

#[cfg(test)]
mod tests {
    use super::*;
    use pyro_schema::StructureWord;

    const R: u32 = <u32 as StructureWord>::RESET;
    use std::collections::hash_map::DefaultHasher;

    fn hash_of<H: Hash + ?Sized>(v: &H) -> u64 {
        let mut h = DefaultHasher::new();
        v.hash(&mut h);
        h.finish()
    }

    // [[[1], [2, 3]], [[4, 5]], []]
    const L0: [u32; 4] = [R, 2, 3, 3];
    const L1: [u32; 4] = [R, 1, 3, 5];
    const LEAF: [u32; 5] = [1, 2, 3, 4, 5];

    fn nest3() -> Nested<'static, u32, u32, 2> {
        Nested {
            topology: Topology {
                topology: [&L0, &L1],
            },
            slice: &LEAF,
        }
    }

    fn owned3() -> Vec<Vec<Vec<u32>>> {
        vec![vec![vec![1], vec![2, 3]], vec![vec![4, 5]], vec![]]
    }

    #[test]
    fn realize_eq_hash_match_std() {
        let n = nest3();
        assert_eq!(n.realize(), owned3());
        assert_eq!(n, owned3());
        assert_eq!(owned3(), n);
        assert_eq!(hash_of(&n), hash_of(&owned3()));
        assert_eq!(format!("{n:?}"), format!("{:?}", owned3()));

        let row = n.get(0).unwrap();
        assert_eq!(row, vec![vec![1], vec![2, 3]]);
        assert_eq!(hash_of(&row), hash_of(&vec![vec![1u32], vec![2, 3]]));
        assert_eq!(row.get(1), Some(&[2u32, 3][..]));
        assert_eq!(n.get(3), None);
    }

    #[test]
    fn ranges_window_at_same_depth() {
        let n = nest3();
        let tail = n.get(1..).unwrap();
        assert_eq!(tail, owned3()[1..].to_vec());
        assert_eq!(hash_of(&tail), hash_of(&owned3()[1..]));
        assert_eq!(n.get(..=0).unwrap(), owned3()[..=0].to_vec());
        assert_eq!(n.get(1..1).unwrap().len(), 0);
        assert!(n.get(2..5).is_none());
        assert_eq!(n.get(..).unwrap(), n);

        let row = n.get(0).unwrap();
        assert_eq!(row.get(1..).unwrap(), vec![vec![2u32, 3]]);
        assert_eq!(row.get(1).unwrap(), &[2, 3]);
    }

    #[test]
    fn omitted_lower_level_yields_empty_children() {
        // [[], []] at depth 2: the level below is never written.
        let l0 = [R, 0, 0];
        let n: Nested<'_, u32, u32, 2> = Nested {
            topology: Topology {
                topology: [&l0, &[]],
            },
            slice: &[],
        };
        let owned: Vec<Vec<Vec<u32>>> = vec![vec![], vec![]];
        assert_eq!(n.iter().count(), 2);
        assert_eq!(n, owned);
        assert_eq!(hash_of(&n), hash_of(&owned));
    }

    #[test]
    fn ordering_and_iter_ends() {
        let n = nest3();
        assert!(n.get(0).unwrap() < n.get(1).unwrap());
        assert_eq!(n.iter().len(), 3);
        assert_eq!(n.iter().rev().next().unwrap().len(), 0);
    }
}
