//! `Vec<Option<T>>` and deeper (`Vec<Vec<Option<T>>>`, …) for a
//! primitive `T`: the `Some` values packed densely, plus one validity bit
//! per sample, both borrowed straight out of the buffer.
//!
//! | view                           | behaves like             | `get(i)` / iter item            |
//! |--------------------------------|--------------------------|---------------------------------|
//! | `OptionSlice<T>`               | `Vec<Option<T>>`         | `Option<T>`                     |
//! | `OptionNested<W, T, 1>`        | `Vec<Vec<Option<T>>>`    | `OptionSlice<T>`                |
//! | `OptionNested<W, T, 2>`        | `Vec<Vec<Vec<Option<T>>>>` | `OptionNested<W, T, 1>`       |
//! | …up to `N = 5`                 |                          |                                 |
//!
//! A `Vec<Option<T>>` has no topology, so it is an [`OptionSlice`], which
//! carries no word type `W`. [`super::OptionBoolSlice`] /
//! [`super::OptionBoolNested`] (in [`super::bool_option`]) are the same thing
//! for `bool`, whose dense side is bit-packed. `[T; M]` elements use these
//! views with `[T; M]` as the element.
//!
//! # Layout
//!
//! * `values` / `slice` — only the `Some` values, in sample order.
//! * `validity` / `option` — one bit per sample, nulls included, LSB first
//!   (`true` = `Some`).
//! * `topology` — boundaries in *samples* (nulls included), exactly like a
//!   required list's; they never point into the values directly.
//!
//! Only the leaf is ever nullable, so rows at every level are plain
//! sub-views — never `Option<sub-view>`.
//!
//! Because nulls take no room in the values, sample `i`'s value lives at
//! `values[validity[..i].count_ones()]`. That popcount is the one real cost
//! here: `get(i)` on an [`OptionSlice`] and on an `OptionNested<W, T, 1>`
//! pays it every call, while `iter()` pays it once and then carries a
//! running cursor. Walk with `iter()`; reach for `get` when you need one
//! element.
//!
//! An [`OptionSlice`] is always self-contained: its values start at its own
//! first `Some` and its validity at its own first sample, however it was
//! reached. `OptionNested` views share the whole column's values and
//! validity and only narrow the topology.
//!
//! ```
//! use pyroformat::refs::{OptionNested, Topology};
//! use pyroformat::schema::StructureWord;
//! const R: u32 = <u32 as StructureWord>::RESET;
//! use pyroformat::{BitSlice, Lsb0, Realize};
//!
//! // [[Some(1), None], [], [None, Some(2), Some(3)]]
//! let values = [1u32, 2, 3];
//! let bits = [0b0001_1001u8]; // samples: 1 0 | 0 1 1
//! let rows = [R, 2, 2, 5];
//! let nest = OptionNested::new(
//!     Topology { topology: [&rows[..]] },
//!     &values[..],
//!     &BitSlice::<u8, Lsb0>::from_slice(&bits)[..5],
//! )
//! .unwrap();
//!
//! assert_eq!(nest.len(), 3);
//! let last = nest.get(2).unwrap();          // OptionSlice<u32>
//! assert_eq!(last.get(0), Some(None));
//! assert_eq!(last.get(1), Some(Some(2)));
//! assert_eq!(last.values(), &[2, 3]);       // just the Somes
//! assert_eq!(last.null_count(), 1);
//!
//! let sums: Vec<u32> = nest.iter().map(|row| row.iter().flatten().sum()).collect();
//! assert_eq!(sums, [1, 0, 5]);
//! assert_eq!(nest.realize(), vec![vec![Some(1), None], vec![], vec![None, Some(2), Some(3)]]);
//! ```

use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;

use crate::PyroValue;
use crate::errors::WireError;
use crate::interface::{Realize, RowsRef};
use crate::refs::Topology;
use crate::value::Time;
use pyro_schema::StructureWord;

// =============================================================================
// OptionSlice — `Vec<Option<T>>`: a dense `&[T]` run plus its validity
// =============================================================================

/// A zero-copy `Vec<Option<T>>`: the `Some` values densely packed, plus one
/// validity bit per sample. No topology, so no word type — see the
/// [module doc](self).
#[derive(Debug)]
pub struct OptionSlice<'a, T> {
    /// The `Some` values only, densely packed.
    values: &'a [T],
    /// One bit per sample, nulls included; `true` = `Some`.
    validity: &'a BitSlice<u8, Lsb0>,
}

impl<'a, T> Clone for OptionSlice<'a, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T> Copy for OptionSlice<'a, T> {}

impl<'a, T> OptionSlice<'a, T> {
    /// A view over `values` (the `Some` values, densely packed) and
    /// `validity` (one bit per sample, nulls included), which must have one
    /// set bit per value.
    pub fn new(values: &'a [T], validity: &'a BitSlice<u8, Lsb0>) -> Result<Self, WireError> {
        if validity.count_ones() != values.len() {
            return Err(WireError::InvalidOptionLen {
                valid: validity.count_ones(),
                data: values.len(),
            });
        }
        Ok(Self { values, validity })
    }

    /// Samples, nulls included.
    pub fn len(&self) -> usize {
        self.validity.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Samples `[start, end)`. `None` when out of bounds.
    ///
    /// This is where the popcount lives: the values are re-sliced to
    /// exactly the `Some`s inside the window, so the result is
    /// self-contained.
    pub fn window(&self, start: usize, end: usize) -> Option<Self> {
        let validity = self.validity.get(start..end)?;
        let dense_start = self.validity[..start].count_ones();
        let dense_end = dense_start + validity.count_ones();
        Some(OptionSlice {
            values: self.values.get(dense_start..dense_end)?,
            validity,
        })
    }

    /// Only the `Some` values, densely packed, in order.
    pub fn values(&self) -> &'a [T] {
        self.values
    }

    /// One bit per sample, nulls included (`true` = `Some`).
    pub fn validity(&self) -> &'a BitSlice<u8, Lsb0> {
        self.validity
    }

    /// Number of `None`s.
    pub fn null_count(&self) -> usize {
        self.validity.count_zeros()
    }

    pub fn iter(&self) -> OptionSliceIter<'a, T> {
        OptionSliceIter {
            slice: *self,
            front: 0,
            back: self.len(),
            dense: 0,
        }
    }
}

impl<'a, T: Copy> OptionSlice<'a, T> {
    /// Sample `i`: `None` when out of bounds, `Some(None)` for a null.
    /// Costs a popcount over the validity bits before `i`; use
    /// [`Self::iter`] to walk every sample.
    pub fn get(&self, i: usize) -> Option<Option<T>> {
        if !*self.validity.get(i)? {
            return Some(None);
        }
        let rank = self.validity[..i].count_ones();
        Some(Some(self.values.get(rank).copied()?))
    }
}

/// Iterator over an [`OptionSlice`]'s samples; carries a running cursor
/// into the dense values instead of popcounting per item.
pub struct OptionSliceIter<'a, T> {
    slice: OptionSlice<'a, T>,
    front: usize,
    back: usize,
    /// Index into the values of the next `Some`.
    dense: usize,
}

impl<'a, T: Copy> Iterator for OptionSliceIter<'a, T> {
    type Item = Option<T>;

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
        if !*self.slice.validity.get(i)? {
            return Some(None);
        }
        let value = self.slice.values.get(self.dense).copied()?;
        self.dense += 1;
        Some(Some(value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.back - self.front;
        (n, Some(n))
    }
}

impl<'a, T: Copy> ExactSizeIterator for OptionSliceIter<'a, T> {}

impl<'a, T: Copy> IntoIterator for OptionSlice<'a, T> {
    type Item = Option<T>;
    type IntoIter = OptionSliceIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, 'n, T: Copy> IntoIterator for &'n OptionSlice<'a, T> {
    type Item = Option<T>;
    type IntoIter = OptionSliceIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T: Copy> RowsRef for OptionSlice<'a, T> {
    type Row = Option<T>;

    fn len(&self) -> usize {
        Self::len(self)
    }

    fn get(&self, i: usize) -> Option<Option<T>> {
        Self::get(self, i)
    }

    fn window(&self, start: usize, end: usize) -> Option<Self> {
        Self::window(self, start, end)
    }
}

impl<'a, T: Copy> OptionSlice<'a, T>
where
    PyroValue<'a>: From<T>,
{
    /// The samples as a `PyroValue::List`: each `Some` as its scalar, each
    /// `None` as `PyroValue::Null`.
    pub fn into_value(&self) -> PyroValue<'a> {
        PyroValue::List(
            self.iter()
                .map(|v| v.map_or(PyroValue::Null, PyroValue::from))
                .collect(),
        )
    }
}

impl<'a, T: Copy> Realize<Vec<Option<T>>> for OptionSlice<'a, T> {
    fn realize(&self) -> Vec<Option<T>> {
        self.iter().collect()
    }
}

/// Equal when the same samples are present and the present values are
/// equal: compares the validity bits and the dense values directly, never
/// sample by sample.
impl<'a, 'b, T, U> PartialEq<OptionSlice<'b, U>> for OptionSlice<'a, T>
where
    T: PartialEq<U>,
{
    fn eq(&self, other: &OptionSlice<'b, U>) -> bool {
        self.validity == other.validity && self.values == other.values
    }
}

impl<'a, T: Eq> Eq for OptionSlice<'a, T> {}

impl<'a, T, U> PartialEq<[Option<U>]> for OptionSlice<'a, T>
where
    T: Copy + PartialEq<U>,
{
    fn eq(&self, other: &[Option<U>]) -> bool {
        self.len() == other.len()
            && self.iter().zip(other).all(|(a, b)| match (a, b) {
                (Some(a), Some(b)) => a == *b,
                (None, None) => true,
                _ => false,
            })
    }
}

impl<'a, T, U> PartialEq<Vec<Option<U>>> for OptionSlice<'a, T>
where
    T: Copy + PartialEq<U>,
{
    fn eq(&self, other: &Vec<Option<U>>) -> bool {
        *self == *other.as_slice()
    }
}

impl<'a, T, U> PartialEq<OptionSlice<'a, T>> for Vec<Option<U>>
where
    T: Copy + PartialEq<U>,
{
    fn eq(&self, other: &OptionSlice<'a, T>) -> bool {
        *other == *self.as_slice()
    }
}

// =============================================================================
// OptionNested — `Vec<…<Vec<Option<T>>>…>` over a whole column's values
// =============================================================================

/// `N` `Vec`s over a nullable primitive column — see the
/// [module doc](self). `N` is at least 1; a bare `Vec<Option<T>>` is an
/// [`OptionSlice`].
#[derive(Debug)]
pub struct OptionNested<'a, W: StructureWord, T, const N: usize> {
    topology: Topology<'a, W, N>,
    /// The `Some` values only, densely packed.
    slice: &'a [T],
    /// One bit per sample, nulls included; `true` = `Some`.
    option: &'a BitSlice<u8, Lsb0>,
}

impl<'a, W: StructureWord, T, const N: usize> Clone for OptionNested<'a, W, T, N> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, W: StructureWord, T, const N: usize> Copy for OptionNested<'a, W, T, N> {}

impl<'a, W: StructureWord, T, const N: usize> OptionNested<'a, W, T, N> {
    /// A view over `slice` (the `Some` values, densely packed) and `option`
    /// (one bit per sample, nulls included). The topology's last level must
    /// end at the sample count; `option` must have one set bit per value in
    /// `slice`.
    pub fn new(
        topology: Topology<'a, W, N>,
        slice: &'a [T],
        option: &'a BitSlice<u8, Lsb0>,
    ) -> Result<Self, WireError> {
        if topology.total() != option.len() {
            return Err(WireError::InvalidTopologyLen {
                topo: topology.total(),
                data: option.len(),
            });
        }
        if option.count_ones() != slice.len() {
            return Err(WireError::InvalidOptionLen {
                valid: option.count_ones(),
                data: slice.len(),
            });
        }
        Ok(Self {
            topology,
            slice,
            option,
        })
    }

    /// Number of lists at this view's outermost level.
    pub fn len(&self) -> usize {
        self.topology.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Lists `[start, end)`, same depth, `O(1)`: only the topology narrows.
    /// `None` when out of bounds.
    pub fn window(&self, start: usize, end: usize) -> Option<Self> {
        Some(OptionNested {
            topology: self.topology.window(start, end)?,
            ..*self
        })
    }

    pub fn iter(&self) -> OptionNestedIter<'a, W, T, N> {
        OptionNestedIter {
            nest: *self,
            front: 0,
            back: self.len(),
            dense: None,
        }
    }

    /// The whole column as one [`OptionSlice`], for windowing into.
    fn column(&self) -> OptionSlice<'a, T> {
        OptionSlice {
            values: self.slice,
            validity: self.option,
        }
    }
}

impl<'a, W: StructureWord, T: Copy> OptionNested<'a, W, T, 1> {
    /// List `i` as a self-contained [`OptionSlice`]. Costs a popcount over
    /// every sample before list `i`; use [`Self::iter`] to walk every list.
    pub fn get(&self, i: usize) -> Option<OptionSlice<'a, T>> {
        let (start, end) = self.topology.leaf_range(i)?;
        self.column().window(start, end)
    }

    /// Alias for [`Self::get`].
    pub fn descend(&self, i: usize) -> Option<OptionSlice<'a, T>> {
        self.get(i)
    }
}

impl<'a, W: StructureWord, T: Copy, const N: usize> OptionNested<'a, W, T, N>
where
    PyroValue<'a>: From<T>,
{
    /// The elements of this view's outermost level as nested
    /// `PyroValue::List`s down to the samples (see
    /// [`OptionSlice::into_value`]).
    ///
    /// `levels` is how many of the view's `N` levels the data uses
    /// (`1 <= levels <= N`): a view read at [`crate::MAX_TIERS`] levels for a
    /// schema only known at runtime uses fewer.
    pub fn into_value(&self, levels: usize) -> Vec<PyroValue<'a>> {
        let (slice, option) = (self.slice, self.option);
        self.topology.fold_rows(levels, |leaf| {
            OptionNested::<'a, W, T, 1> {
                topology: leaf,
                slice,
                option,
            }
            .iter()
            .map(|row| row.into_value())
        })
    }
}

impl<'a, W: StructureWord, T: Copy> Realize<Vec<Vec<Option<T>>>> for OptionNested<'a, W, T, 1> {
    fn realize(&self) -> Vec<Vec<Option<T>>> {
        self.iter().map(|list| list.realize()).collect()
    }
}

macro_rules! option_slice_level {
    ($n:literal => $m:literal) => {
        impl<'a, W: StructureWord, T: Copy> OptionNested<'a, W, T, $n> {
            /// Element `i`, one level down, `O(1)`.
            pub fn get(&self, i: usize) -> Option<OptionNested<'a, W, T, $m>> {
                Some(OptionNested {
                    topology: self.topology.descend(i)?,
                    slice: self.slice,
                    option: self.option,
                })
            }

            /// Alias for [`Self::get`].
            pub fn descend(&self, i: usize) -> Option<OptionNested<'a, W, T, $m>> {
                self.get(i)
            }
        }

        impl<'a, W: StructureWord, T: Copy, O> Realize<Vec<O>> for OptionNested<'a, W, T, $n>
        where
            OptionNested<'a, W, T, $m>: Realize<O>,
        {
            fn realize(&self) -> Vec<O> {
                self.iter().map(|child| child.realize()).collect()
            }
        }
    };
}

option_slice_level!(2 => 1);
option_slice_level!(3 => 2);
option_slice_level!(4 => 3);
option_slice_level!(5 => 4);

/// Iterator over a [`OptionNested`]'s outermost level; yields what `get(i)`
/// would. At `N = 1` it carries a running cursor into the dense values
/// instead of popcounting per item, so walking a whole column is linear.
pub struct OptionNestedIter<'a, W: StructureWord, T, const N: usize> {
    nest: OptionNested<'a, W, T, N>,
    front: usize,
    back: usize,
    /// Index into `nest.slice` of the next list's first `Some` value;
    /// `None` until the first item computes it.
    dense: Option<usize>,
}

impl<'a, W: StructureWord, T: Copy> Iterator for OptionNestedIter<'a, W, T, 1> {
    type Item = OptionSlice<'a, T>;

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
        let (start, end) = self.nest.topology.leaf_range(i)?;
        // Topology offsets are absolute sample positions, so the first
        // list's dense start is one popcount; every later list's is the
        // previous list's end.
        let dense = match self.dense {
            Some(dense) => dense,
            None => self.nest.option.get(..start)?.count_ones(),
        };
        let validity = self.nest.option.get(start..end)?;
        let dense_end = dense + validity.count_ones();
        self.dense = Some(dense_end);
        Some(OptionSlice {
            values: self.nest.slice.get(dense..dense_end)?,
            validity,
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.back - self.front;
        (n, Some(n))
    }
}

macro_rules! option_slice_iter {
    ($n:literal => $m:literal) => {
        impl<'a, W: StructureWord, T: Copy> Iterator for OptionNestedIter<'a, W, T, $n> {
            type Item = OptionNested<'a, W, T, $m>;

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
    };
}

option_slice_iter!(2 => 1);
option_slice_iter!(3 => 2);
option_slice_iter!(4 => 3);
option_slice_iter!(5 => 4);

macro_rules! option_slice_exact {
    ($($n:literal),*) => {
        $(
            impl<'a, W: StructureWord, T: Copy> ExactSizeIterator for OptionNestedIter<'a, W, T, $n> {}

            impl<'a, W: StructureWord, T: Copy> IntoIterator for OptionNested<'a, W, T, $n> {
                type Item = <OptionNestedIter<'a, W, T, $n> as Iterator>::Item;
                type IntoIter = OptionNestedIter<'a, W, T, $n>;

                fn into_iter(self) -> Self::IntoIter {
                    self.iter()
                }
            }
        )*
    };
}

option_slice_exact!(1, 2, 3, 4, 5);

// =============================================================================
// RowsRef — forwards to each level's own `get` (not `descend`)
// =============================================================================

macro_rules! option_slice_rows_ref {
    ($($n:literal => $row:ty),* $(,)?) => {
        $(
            impl<'a, W: StructureWord, T: Copy> RowsRef for OptionNested<'a, W, T, $n> {
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

option_slice_rows_ref!(
    1 => OptionSlice<'a, T>,
    2 => OptionNested<'a, W, T, 1>,
    3 => OptionNested<'a, W, T, 2>,
    4 => OptionNested<'a, W, T, 3>,
    5 => OptionNested<'a, W, T, 4>,
);

// =============================================================================
// OptionNested equality — compares like the `Vec<…<Vec<Option<T>>>…>` it
// stands for, against another view or against `Vec`s (both directions)
// =============================================================================

macro_rules! option_slice_eq {
    ($($n:literal => $child:ty),* $(,)?) => {
        $(
            impl<'a, 'b, W: StructureWord, T, U> PartialEq<OptionNested<'b, W, U, $n>> for OptionNested<'a, W, T, $n>
            where
                T: Copy + PartialEq<U>,
                U: Copy,
            {
                fn eq(&self, other: &OptionNested<'b, W, U, $n>) -> bool {
                    self.len() == other.len()
                        && self.iter().zip(other.iter()).all(|(a, b)| a == b)
                }
            }

            impl<'a, W: StructureWord, T: Copy + Eq> Eq for OptionNested<'a, W, T, $n> {}

            impl<'a, W: StructureWord, V, T> PartialEq<[V]> for OptionNested<'a, W, T, $n>
            where
                T: Copy,
                $child: PartialEq<V>,
            {
                fn eq(&self, other: &[V]) -> bool {
                    self.len() == other.len() && self.iter().zip(other).all(|(a, b)| a == *b)
                }
            }

            impl<'a, W: StructureWord, V, T> PartialEq<Vec<V>> for OptionNested<'a, W, T, $n>
            where
                T: Copy,
                $child: PartialEq<V>,
            {
                fn eq(&self, other: &Vec<V>) -> bool {
                    *self == *other.as_slice()
                }
            }

            impl<'a, W: StructureWord, V, T> PartialEq<OptionNested<'a, W, T, $n>> for Vec<V>
            where
                T: Copy,
                $child: PartialEq<V>,
            {
                fn eq(&self, other: &OptionNested<'a, W, T, $n>) -> bool {
                    *other == *self.as_slice()
                }
            }
        )*
    };
}

option_slice_eq!(
    1 => OptionSlice<'a, T>,
    2 => OptionNested<'a, W, T, 1>,
    3 => OptionNested<'a, W, T, 2>,
    4 => OptionNested<'a, W, T, 3>,
    5 => OptionNested<'a, W, T, 4>,
);

pub type OptionF16Slice<'a> = OptionSlice<'a, half::f16>;
pub type OptionF32Slice<'a> = OptionSlice<'a, f32>;
pub type OptionF64Slice<'a> = OptionSlice<'a, f64>;
pub type OptionI8Slice<'a> = OptionSlice<'a, i8>;
pub type OptionI16Slice<'a> = OptionSlice<'a, i16>;
pub type OptionI32Slice<'a> = OptionSlice<'a, i32>;
pub type OptionI64Slice<'a> = OptionSlice<'a, i64>;
pub type OptionU8Slice<'a> = OptionSlice<'a, u8>;
pub type OptionU16Slice<'a> = OptionSlice<'a, u16>;
pub type OptionU32Slice<'a> = OptionSlice<'a, u32>;
pub type OptionU64Slice<'a> = OptionSlice<'a, u64>;
pub type OptionTimeSlice<'a> = OptionSlice<'a, Time>;

pub type OptionF16Nested<'a, W, const N: usize> = OptionNested<'a, W, half::f16, N>;
pub type OptionF32Nested<'a, W, const N: usize> = OptionNested<'a, W, f32, N>;
pub type OptionF64Nested<'a, W, const N: usize> = OptionNested<'a, W, f64, N>;
pub type OptionI8Nested<'a, W, const N: usize> = OptionNested<'a, W, i8, N>;
pub type OptionI16Nested<'a, W, const N: usize> = OptionNested<'a, W, i16, N>;
pub type OptionI32Nested<'a, W, const N: usize> = OptionNested<'a, W, i32, N>;
pub type OptionI64Nested<'a, W, const N: usize> = OptionNested<'a, W, i64, N>;
pub type OptionU8Nested<'a, W, const N: usize> = OptionNested<'a, W, u8, N>;
pub type OptionU16Nested<'a, W, const N: usize> = OptionNested<'a, W, u16, N>;
pub type OptionU32Nested<'a, W, const N: usize> = OptionNested<'a, W, u32, N>;
pub type OptionU64Nested<'a, W, const N: usize> = OptionNested<'a, W, u64, N>;
pub type OptionTimeNested<'a, W, const N: usize> = OptionNested<'a, W, Time, N>;

#[cfg(test)]
mod tests {
    use super::*;
    use pyro_schema::StructureWord;

    const R: u32 = <u32 as StructureWord>::RESET;

    fn bits(pattern: &[bool]) -> Vec<u8> {
        let mut out = vec![0u8; pattern.len().div_ceil(8)];
        for (i, &b) in pattern.iter().enumerate() {
            out[i / 8] |= (b as u8) << (i % 8);
        }
        out
    }

    // [[[Some(1)], [None, Some(2)]], [[]], [[None], [Some(3), Some(4), None]]]
    fn owned() -> Vec<Vec<Vec<Option<u32>>>> {
        vec![
            vec![vec![Some(1)], vec![None, Some(2)]],
            vec![vec![]],
            vec![vec![None], vec![Some(3), Some(4), None]],
        ]
    }

    #[test]
    fn realize_get_and_iter_agree() {
        let l0 = [R, 2, 3, 5];
        let l1 = [R, 1, 3, 3, 4, 7];
        let values = [1u32, 2, 3, 4];
        let mask = bits(&[true, false, true, false, true, true, false]);
        let nest: OptionNested<'_, u32, u32, 2> = OptionNested {
            topology: Topology {
                topology: [&l0, &l1],
            },
            slice: &values,
            option: &BitSlice::from_slice(&mask)[..7],
        };
        assert_eq!(nest.realize(), owned());

        // `get` (popcount) and `iter` (running cursor) must agree, including
        // for a list reached mid-column.
        for (i, lists) in nest.iter().enumerate() {
            let by_get = nest.get(i).unwrap();
            for (j, list) in lists.iter().enumerate() {
                let expected = &owned()[i][j];
                assert_eq!(&list.realize(), expected);
                assert_eq!(&by_get.get(j).unwrap().realize(), expected);
                for (k, v) in expected.iter().enumerate() {
                    assert_eq!(list.get(k), Some(*v));
                }
                assert_eq!(list.get(expected.len()), None);
            }
        }

        let last = nest.get(2).unwrap().get(1).unwrap();
        assert_eq!(last.values(), &[3, 4]);
        assert_eq!(last.null_count(), 1);
        assert_eq!(last.window(1, 3).unwrap().realize(), vec![Some(4), None]);
        assert_eq!(nest.window(1, 3).unwrap().realize(), owned()[1..].to_vec());
        assert!(nest.get(3).is_none());
    }

    #[test]
    fn equality_matches_owned_options() {
        let l0 = [R, 2, 3, 5];
        let l1 = [R, 1, 3, 3, 4, 7];
        let values = [1u32, 2, 3, 4];
        let mask = bits(&[true, false, true, false, true, true, false]);
        let nest: OptionNested<'_, u32, u32, 2> = OptionNested {
            topology: Topology {
                topology: [&l0, &l1],
            },
            slice: &values,
            option: &BitSlice::from_slice(&mask)[..7],
        };
        assert_eq!(nest, owned());
        assert_eq!(owned(), nest);
        assert_eq!(nest, nest);
        // Same content reached two different ways compares equal.
        let a = nest.get(2).unwrap().get(1).unwrap();
        let b = nest.get(2).unwrap().iter().nth(1).unwrap();
        assert_eq!(a, b);
        assert_ne!(a, nest.get(0).unwrap().get(1).unwrap());
        assert_eq!(a, vec![Some(3), Some(4), None]);
    }

    #[test]
    fn option_slice_checks_and_windows() {
        let values = [7u32, 8];
        let mask = bits(&[false, true, true]);
        let slice = OptionSlice::new(&values, &BitSlice::from_slice(&mask)[..3]).unwrap();
        assert_eq!(slice, vec![None, Some(7), Some(8)]);
        assert_eq!(RowsRef::get(&slice, 1), Some(Some(7)));
        assert_eq!(slice.window(2, 3).unwrap(), vec![Some(8)]);
        assert!(slice.window(2, 4).is_none());
        assert!(OptionSlice::new(&values[..1], &BitSlice::from_slice(&mask)[..3]).is_err());
    }
}
