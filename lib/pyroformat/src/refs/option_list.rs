//! [`OptionList`] / [`OptionNest`] — `Vec<Option<Group>>`,
//! `Vec<Vec<Option<Group>>>`, … over a group column.
//!
//! This is the nullable twin of [`super::GroupNested`]; read that module's
//! doc first for how group views nest (`FooRef` = one `Foo`, `FooSlice` = a
//! column of them, and everything stays flat). These views put a validity
//! mask between the topology and the group column, so rows can be absent
//! without taking any room in the column.
//!
//! | Rust type                   | view                            | row                           |
//! |-----------------------------|---------------------------------|-------------------------------|
//! | `Vec<Option<Foo>>`          | `OptionList<'a, FooSlice<'a>>`  | `Option<FooRef<'a>>`          |
//! | `Vec<Vec<Option<Foo>>>`     | `OptionNest<'a, W, FooSlice<'a>, 1>` | `OptionList<'a, FooSlice<'a>>` |
//! | `Vec<Vec<Vec<Option<Foo>>>>`| `OptionNest<'a, W, FooSlice<'a>, 2>` | `OptionNest<'a, W, FooSlice<'a>, 1>` |
//! | …up to `N = 5`              |                                 |                               |
//!
//! `N` counts the `Vec`s above the `Vec<Option<_>>` leaf, matching
//! [`super::OptionNested`]. A bare `Vec<Option<Foo>>` has no topology, so
//! it is an [`OptionList`], which carries no word type. Only the leaf is
//! nullable; the lists above it never are, so rows at every level are plain
//! sub-views.
//!
//! # In a containing struct
//!
//! For `struct Bar { f: Vec<Vec<Option<Foo>>> }`:
//!
//! ```text
//! struct BarRef<'a>   { f: OptionNest<'a, W, FooSlice<'a>, 1> }  // one Bar: realizes to Bar
//! struct BarSlice<'a> { f: OptionNest<'a, W, FooSlice<'a>, 2> }  // Vec<Bar>; row i is a BarRef
//! ```
//!
//! # Fields
//!
//! * `validity` — one bit per sample, nulls included (`true` = present).
//! * `len` — samples in this view, nulls included (on an [`OptionList`]);
//!   on an [`OptionNest`] the topology decides the length and `len` is the
//!   whole column's.
//! * `data` — the *whole column's* present rows, e.g. a `FooSlice<'a>`.
//!   Never windowed (see below).
//! * `prior_valid` — how many present rows of `data` precede this view's
//!   first sample. Sample `i`'s row, when present, is
//!   `data.get(prior_valid + validity[..i].count_ones())`.
//! * `topology` — sample boundaries, nulls included; `N` levels (an
//!   [`OptionNest`] only).
//!
//! # `data` is never windowed
//!
//! `data` is any group column, not a slice, so narrowing it would mean
//! windowing every one of its field columns. Instead narrowing an
//! `OptionNest` narrows `validity` and advances `prior_valid` by one
//! popcount, and `data` is carried along as-is — a cheap copy for every
//! group column the derive emits.
//!
//! # Using an `OptionNest`
//!
//! Like nested `Vec`s around an `Option`:
//!
//! * `len()`, `is_empty()`.
//! * `get(i)` — on an [`OptionList`] the sample itself
//!   (`Option<Option<FooRef>>`: outer `None` = out of bounds, inner `None` =
//!   absent); on an [`OptionNest`] the next view down.
//! * `window(start, end)` — same depth.
//! * `iter()` — the outermost level; `for` works on the view or a
//!   reference to it.
//! * On an [`OptionList`], also `present()` (every present row as one group
//!   column, e.g. a `FooSlice`), `validity()` and `null_count()`.
//! * `==` against another view of the same shape (when `T: PartialEq<U>`)
//!   and against `Vec`s in both directions — `Vec<Option<V>>` for an
//!   [`OptionList`] when `T::Row: PartialEq<V>`, nested `Vec`s of those
//!   above.
//!
//! # Cost
//!
//! Finding sample `i`'s row costs a popcount over the validity bits before
//! it, so `get(i)` on an [`OptionList`] and an `OptionNest<_, _, 1>` pays
//! that per call. `iter()` on those pays it once and then carries a running
//! cursor into `data`, which keeps a full walk linear — prefer it in loops.
//! Above `N = 1`, `get` and every iterator step are `O(1)`. The
//! [`OptionList`] and `N = 1` iterators are forward-only for that reason;
//! the others also run backwards.

use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;

use crate::interface::{Realize, RowsRef};
use crate::refs::Topology;
use pyro_schema::StructureWord;

// =============================================================================
// OptionList — `Vec<Option<Group>>`: rows are `Option<T::Row>`
// =============================================================================

/// A nullable group column: a validity mask over `data`'s present rows,
/// plus how many of those precede this view — see the [module doc](self).
#[derive(Debug, Clone)]
pub struct OptionList<'b, T> {
    pub len: usize,
    pub validity: &'b BitSlice<u8, Lsb0>,
    pub prior_valid: usize,
    pub data: T,
}

impl<'b, T: RowsRef + Clone> OptionList<'b, T> {
    /// Samples in this view, nulls included.
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Samples `[start, end)`. `None` when out of bounds.
    ///
    /// Narrows `validity` and advances `prior_valid` by one popcount;
    /// `data` is never touched.
    pub fn window(&self, start: usize, end: usize) -> Option<Self> {
        if start > end || end > self.len {
            return None;
        }
        let validity = self.validity.get(..end)?;
        Some(OptionList {
            len: end - start,
            validity: &validity[start..],
            prior_valid: self.prior_valid + validity[..start].count_ones(),
            data: self.data.clone(),
        })
    }

    /// Sample `i`: `None` when out of bounds, `Some(None)` when absent.
    /// Costs a popcount over the validity bits before `i`; use
    /// [`Self::iter`] to walk every sample.
    pub fn get(&self, i: usize) -> Option<Option<T::Row>> {
        if i >= self.len {
            return None;
        }
        if !*self.validity.get(i)? {
            return Some(None);
        }
        let row = self.prior_valid + self.validity[..i].count_ones();
        Some(Some(self.data.get(row)?))
    }

    /// Only the present rows, as a group column (e.g. a `FooSlice` of
    /// every `Some(Foo)` in this list, in order).
    pub fn present(&self) -> Option<T> {
        let start = self.prior_valid;
        self.data
            .window(start, start + self.validity().count_ones())
    }

    /// One bit per sample, nulls included (`true` = present), trimmed to
    /// this view.
    pub fn validity(&self) -> &'b BitSlice<u8, Lsb0> {
        &self.validity[..self.len.min(self.validity.len())]
    }

    /// Number of `None`s.
    pub fn null_count(&self) -> usize {
        self.validity().count_zeros()
    }

    /// Iterates the samples, yielding what `get(i)` would; carries a
    /// running cursor into `data` instead of popcounting per item, so
    /// walking a whole list is linear.
    pub fn iter(&self) -> OptionListIter<'b, T> {
        OptionListIter {
            list: self.clone(),
            front: 0,
            back: self.len(),
            next_row: None,
        }
    }
}

/// Iterator over an [`OptionList`]'s samples; yields what `get(i)` would.
pub struct OptionListIter<'b, T> {
    list: OptionList<'b, T>,
    front: usize,
    back: usize,
    /// Index into `list.data` of the next present row; `None` until the
    /// first item computes it.
    next_row: Option<usize>,
}

impl<'b, T: RowsRef + Clone> Iterator for OptionListIter<'b, T> {
    type Item = Option<T::Row>;

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
        if !*self.list.validity.get(i)? {
            return Some(None);
        }
        let row = self.next_row.get_or_insert(self.list.prior_valid);
        let value = self.list.data.get(*row)?;
        *row += 1;
        Some(Some(value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.back - self.front;
        (n, Some(n))
    }
}

impl<'b, T: RowsRef + Clone> ExactSizeIterator for OptionListIter<'b, T> {}

impl<'b, T: RowsRef + Clone> IntoIterator for OptionList<'b, T> {
    type Item = Option<T::Row>;
    type IntoIter = OptionListIter<'b, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'b, 'n, T: RowsRef + Clone> IntoIterator for &'n OptionList<'b, T> {
    type Item = Option<T::Row>;
    type IntoIter = OptionListIter<'b, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Two `Vec<Option<_>>`s are equal when the same samples are present and
/// the present rows are equal — compared as whole group columns, so only
/// `T: PartialEq<U>` is needed, not a per-row impl.
impl<'b, 'c, T, U> PartialEq<OptionList<'c, U>> for OptionList<'b, T>
where
    T: RowsRef + Clone + PartialEq<U>,
    U: RowsRef + Clone,
{
    fn eq(&self, other: &OptionList<'c, U>) -> bool {
        self.len == other.len
            && self.validity() == other.validity()
            && match (self.present(), other.present()) {
                (Some(a), Some(b)) => a == b,
                _ => false,
            }
    }
}

impl<'b, T: RowsRef + Clone + Eq> Eq for OptionList<'b, T> {}

impl<'b, T, V> PartialEq<[Option<V>]> for OptionList<'b, T>
where
    T: RowsRef + Clone,
    T::Row: PartialEq<V>,
{
    fn eq(&self, other: &[Option<V>]) -> bool {
        self.len() == other.len()
            && self.iter().zip(other).all(|(a, b)| match (a, b) {
                (Some(a), Some(b)) => a == *b,
                (None, None) => true,
                _ => false,
            })
    }
}

impl<'b, T, V> PartialEq<Vec<Option<V>>> for OptionList<'b, T>
where
    T: RowsRef + Clone,
    T::Row: PartialEq<V>,
{
    fn eq(&self, other: &Vec<Option<V>>) -> bool {
        *self == *other.as_slice()
    }
}

impl<'b, T, V> PartialEq<OptionList<'b, T>> for Vec<Option<V>>
where
    T: RowsRef + Clone,
    T::Row: PartialEq<V>,
{
    fn eq(&self, other: &OptionList<'b, T>) -> bool {
        *other == *self.as_slice()
    }
}

impl<'b, T: RowsRef + Clone> RowsRef for OptionList<'b, T> {
    type Row = Option<T::Row>;

    fn len(&self) -> usize {
        Self::len(self)
    }

    fn window(&self, start: usize, end: usize) -> Option<Self> {
        Self::window(self, start, end)
    }

    fn get(&self, i: usize) -> Option<Option<T::Row>> {
        Self::get(self, i)
    }
}

impl<'b, T, O> Realize<Vec<Option<O>>> for OptionList<'b, T>
where
    T: RowsRef + Clone,
    T::Row: Realize<O>,
{
    fn realize(&self) -> Vec<Option<O>> {
        self.iter()
            .map(|row| row.map(|row| row.realize()))
            .collect()
    }
}

// =============================================================================
// OptionNest — `N` lists above a nullable group column
// =============================================================================

/// `N` extra `Vec`s over a nullable group column: a validity mask over
/// `data`'s present rows, plus how many of those precede this view — see
/// the [module doc](self). `N` is at least 1; a bare `Vec<Option<Group>>`
/// is an [`OptionList`].
#[derive(Debug, Clone)]
pub struct OptionNest<'b, W: StructureWord, T, const N: usize> {
    pub topology: Topology<'b, W, N>,
    pub len: usize,
    pub validity: &'b BitSlice<u8, Lsb0>,
    pub prior_valid: usize,
    pub data: T,
}

macro_rules! option_list_common {
    ($($n:literal),* $(,)?) => {
        $(
            impl<'b, W: StructureWord, T: RowsRef + Clone> OptionNest<'b, W, T, $n> {
                /// Number of lists at this view's outermost level.
                pub fn len(&self) -> usize {
                    self.topology.len()
                }

                pub fn is_empty(&self) -> bool {
                    self.len() == 0
                }

                /// Lists `[start, end)`, same depth, `O(1)`: only the
                /// topology narrows; `data` is never touched. `None` when
                /// out of bounds.
                pub fn window(&self, start: usize, end: usize) -> Option<Self> {
                    Some(OptionNest {
                        topology: self.topology.window(start, end)?,
                        len: self.len,
                        validity: self.validity,
                        prior_valid: self.prior_valid,
                        data: self.data.clone(),
                    })
                }

                /// Iterates this view's outermost level, yielding what
                /// `get(i)` would. At `N = 1` it carries a running cursor
                /// into `data` instead of popcounting per item, so walking
                /// a whole column is linear.
                pub fn iter(&self) -> OptionNestIter<'b, W, T, $n> {
                    OptionNestIter {
                        list: self.clone(),
                        front: 0,
                        back: self.len(),
                        next_row: None,
                    }
                }
            }

            impl<'b, W: StructureWord, T: RowsRef + Clone> ExactSizeIterator for OptionNestIter<'b, W, T, $n> {}

            impl<'b, W: StructureWord, T: RowsRef + Clone> IntoIterator for OptionNest<'b, W, T, $n> {
                type Item = <OptionNestIter<'b, W, T, $n> as Iterator>::Item;
                type IntoIter = OptionNestIter<'b, W, T, $n>;

                fn into_iter(self) -> Self::IntoIter {
                    self.iter()
                }
            }

            impl<'b, 'n, W: StructureWord, T: RowsRef + Clone> IntoIterator for &'n OptionNest<'b, W, T, $n> {
                type Item = <OptionNestIter<'b, W, T, $n> as Iterator>::Item;
                type IntoIter = OptionNestIter<'b, W, T, $n>;

                fn into_iter(self) -> Self::IntoIter {
                    self.iter()
                }
            }

            impl<'b, W: StructureWord, T: RowsRef + Clone + Eq> Eq for OptionNest<'b, W, T, $n> {}
        )*
    };
}

option_list_common!(1, 2, 3, 4, 5);

/// Iterator over an [`OptionNest`]'s outermost level; yields what
/// `get(i)` would.
pub struct OptionNestIter<'b, W: StructureWord, T, const N: usize> {
    list: OptionNest<'b, W, T, N>,
    front: usize,
    back: usize,
    /// At `N = 1`, index into `list.data` of the next list's first present
    /// row; `None` until the first item computes it.
    next_row: Option<usize>,
}

// =============================================================================
// N = 1 — `Vec<Vec<Option<Group>>>`: rows are `OptionList<T>`
// =============================================================================

impl<'b, W: StructureWord, T: RowsRef + Clone> OptionNest<'b, W, T, 1> {
    /// List `i` as a self-contained [`OptionList`]. Costs a popcount over
    /// every sample before list `i`; use [`Self::iter`] to walk every list.
    pub fn get(&self, i: usize) -> Option<OptionList<'b, T>> {
        let (start, end) = self.topology.leaf_range(i)?;
        OptionList {
            len: self.len,
            validity: self.validity,
            prior_valid: self.prior_valid,
            data: self.data.clone(),
        }
        .window(start, end)
    }

    /// Alias for [`Self::get`].
    pub fn descend(&self, i: usize) -> Option<OptionList<'b, T>> {
        self.get(i)
    }
}

impl<'b, W: StructureWord, T: RowsRef + Clone> Iterator for OptionNestIter<'b, W, T, 1> {
    type Item = OptionList<'b, T>;

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
        let (start, end) = self.list.topology.leaf_range(i)?;
        // Topology offsets are absolute sample positions, so the first
        // list's first row is one popcount; every later list's is the
        // previous list's end.
        let prior_valid = match self.next_row {
            Some(row) => row,
            None => self.list.prior_valid + self.list.validity.get(..start)?.count_ones(),
        };
        let validity = self.list.validity.get(start..end)?;
        self.next_row = Some(prior_valid + validity.count_ones());
        Some(OptionList {
            len: end - start,
            validity,
            prior_valid,
            data: self.list.data.clone(),
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.back - self.front;
        (n, Some(n))
    }
}

impl<'b, W: StructureWord, T: RowsRef + Clone> RowsRef for OptionNest<'b, W, T, 1> {
    type Row = OptionList<'b, T>;

    fn len(&self) -> usize {
        Self::len(self)
    }

    fn window(&self, start: usize, end: usize) -> Option<Self> {
        Self::window(self, start, end)
    }

    fn get(&self, i: usize) -> Option<OptionList<'b, T>> {
        Self::get(self, i)
    }
}

impl<'b, W: StructureWord, T, O> Realize<Vec<Vec<Option<O>>>> for OptionNest<'b, W, T, 1>
where
    T: RowsRef + Clone,
    T::Row: Realize<O>,
{
    fn realize(&self) -> Vec<Vec<Option<O>>> {
        self.iter().map(|list| list.realize()).collect()
    }
}

// =============================================================================
// N = 2..=6 — rows are the next `OptionNest` down
// =============================================================================

macro_rules! option_list_level {
    ($n:literal => $m:literal) => {
        impl<'b, W: StructureWord, T: RowsRef + Clone> OptionNest<'b, W, T, $n> {
            /// List `i`, one level down, `O(1)`. `None` when out of bounds.
            pub fn get(&self, i: usize) -> Option<OptionNest<'b, W, T, $m>> {
                Some(OptionNest {
                    topology: self.topology.descend(i)?,
                    len: self.len,
                    validity: self.validity,
                    prior_valid: self.prior_valid,
                    data: self.data.clone(),
                })
            }

            /// Alias for [`Self::get`].
            pub fn descend(&self, i: usize) -> Option<OptionNest<'b, W, T, $m>> {
                self.get(i)
            }
        }

        impl<'b, W: StructureWord, T: RowsRef + Clone> Iterator for OptionNestIter<'b, W, T, $n> {
            type Item = OptionNest<'b, W, T, $m>;

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
                self.list.get(i)
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                let n = self.back - self.front;
                (n, Some(n))
            }
        }

        impl<'b, W: StructureWord, T: RowsRef + Clone> DoubleEndedIterator
            for OptionNestIter<'b, W, T, $n>
        {
            fn next_back(&mut self) -> Option<Self::Item> {
                if self.front >= self.back {
                    return None;
                }
                self.back -= 1;
                self.list.get(self.back)
            }
        }

        impl<'b, W: StructureWord, T: RowsRef + Clone> RowsRef for OptionNest<'b, W, T, $n> {
            type Row = OptionNest<'b, W, T, $m>;

            fn len(&self) -> usize {
                Self::len(self)
            }

            fn window(&self, start: usize, end: usize) -> Option<Self> {
                Self::window(self, start, end)
            }

            fn get(&self, i: usize) -> Option<OptionNest<'b, W, T, $m>> {
                Self::get(self, i)
            }
        }

        impl<'b, W: StructureWord, T, O> Realize<Vec<O>> for OptionNest<'b, W, T, $n>
        where
            T: RowsRef + Clone,
            OptionNest<'b, W, T, $m>: Realize<O>,
        {
            fn realize(&self) -> Vec<O> {
                self.iter().map(|list| list.realize()).collect()
            }
        }
    };
}

option_list_level!(2 => 1);
option_list_level!(3 => 2);
option_list_level!(4 => 3);
option_list_level!(5 => 4);

// =============================================================================
// N >= 1 — equality, level by level
// =============================================================================

macro_rules! option_list_eq {
    ($($n:literal),* $(,)?) => {
        $(
            impl<'b, 'c, W: StructureWord, T, U> PartialEq<OptionNest<'c, W, U, $n>> for OptionNest<'b, W, T, $n>
            where
                T: RowsRef + Clone + PartialEq<U>,
                U: RowsRef + Clone,
            {
                fn eq(&self, other: &OptionNest<'c, W, U, $n>) -> bool {
                    self.len() == other.len()
                        && self.iter().zip(other.iter()).all(|(a, b)| a == b)
                }
            }

            impl<'b, W: StructureWord, T, V> PartialEq<[V]> for OptionNest<'b, W, T, $n>
            where
                T: RowsRef + Clone,
                <OptionNestIter<'b, W, T, $n> as Iterator>::Item: PartialEq<V>,
            {
                fn eq(&self, other: &[V]) -> bool {
                    self.len() == other.len() && self.iter().zip(other).all(|(a, b)| a == *b)
                }
            }

            impl<'b, W: StructureWord, T, V> PartialEq<Vec<V>> for OptionNest<'b, W, T, $n>
            where
                T: RowsRef + Clone,
                <OptionNestIter<'b, W, T, $n> as Iterator>::Item: PartialEq<V>,
            {
                fn eq(&self, other: &Vec<V>) -> bool {
                    *self == *other.as_slice()
                }
            }

            impl<'b, W: StructureWord, T, V> PartialEq<OptionNest<'b, W, T, $n>> for Vec<V>
            where
                T: RowsRef + Clone,
                <OptionNestIter<'b, W, T, $n> as Iterator>::Item: PartialEq<V>,
            {
                fn eq(&self, other: &OptionNest<'b, W, T, $n>) -> bool {
                    *other == *self.as_slice()
                }
            }
        )*
    };
}

option_list_eq!(1, 2, 3, 4, 5);
