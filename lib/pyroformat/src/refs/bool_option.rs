//! `Vec<Option<bool>>` and deeper, plus `Vec<…<Vec<Option<[bool; M]>>>…>`:
//! the bit-packed counterparts of [`OptionSlice`](super::OptionSlice) /
//! [`OptionNested`](super::OptionNested).
//!
//! | view                              | behaves like                  | `get(i)` / iter item               |
//! |-----------------------------------|-------------------------------|------------------------------------|
//! | `OptionBoolSlice`                 | `Vec<Option<bool>>`           | `Option<bool>`                     |
//! | `OptionBoolNested<W, 1>`          | `Vec<Vec<Option<bool>>>`      | `OptionBoolSlice`                  |
//! | `OptionFixedBoolSlice<M>`         | `Vec<Option<[bool; M]>>`      | `Option<[bool; M]>`                |
//! | `OptionFixedBoolNested<W, M, 1>`  | `Vec<Vec<Option<[bool; M]>>>` | `OptionFixedBoolSlice<M>`          |
//! | …up to `N = 5`                    |                               |                                    |
//!
//! Same layout as [`OptionSlice`](super::OptionSlice) /
//! [`OptionNested`](super::OptionNested), except the dense `Some` side is a
//! `BitSlice` too (`M` bits per value for `[bool; M]`). The `…Slice` views
//! have no topology, so no word type.

use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;

use crate::PyroValue;
use crate::errors::WireError;
use crate::interface::{Realize, RowsRef};
use crate::refs::Topology;
use pyro_schema::StructureWord;

/// `values` must hold one value of `per` bits per set bit of `validity`.
fn check_option_len(
    values: &BitSlice<u8, Lsb0>,
    validity: &BitSlice<u8, Lsb0>,
    per: usize,
) -> Result<(), WireError> {
    if validity.count_ones().checked_mul(per) != Some(values.len()) {
        return Err(WireError::InvalidOptionLen {
            valid: validity.count_ones(),
            data: values.len() / per.max(1),
        });
    }
    Ok(())
}

// =============================================================================
// OptionBoolSlice — `Vec<Option<bool>>`
// =============================================================================

/// A zero-copy `Vec<Option<bool>>`: the `Some` values bit-packed densely,
/// plus one validity bit per sample.
#[derive(Debug, Clone, Copy)]
pub struct OptionBoolSlice<'a> {
    /// The `Some` values only, densely packed.
    values: &'a BitSlice<u8, Lsb0>,
    /// One bit per sample, nulls included; `true` = `Some`.
    validity: &'a BitSlice<u8, Lsb0>,
}

impl<'a> OptionBoolSlice<'a> {
    /// A view over `values` (the `Some` values, densely packed) and
    /// `validity` (one bit per sample), which must have one set bit per
    /// value.
    pub fn new(
        values: &'a BitSlice<u8, Lsb0>,
        validity: &'a BitSlice<u8, Lsb0>,
    ) -> Result<Self, WireError> {
        check_option_len(values, validity, 1)?;
        Ok(Self { values, validity })
    }

    /// Samples, nulls included.
    pub fn len(&self) -> usize {
        self.validity.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Sample `i`: `None` when out of bounds, `Some(None)` for a null.
    /// Costs a popcount over the validity bits before `i`; use
    /// [`Self::iter`] to walk every sample.
    pub fn get(&self, i: usize) -> Option<Option<bool>> {
        if !*self.validity.get(i)? {
            return Some(None);
        }
        let rank = self.validity[..i].count_ones();
        Some(Some(self.values.get(rank).map(|b| *b)?))
    }

    /// Samples `[start, end)`, re-sliced to exactly the `Some`s inside the
    /// window (one popcount). `None` when out of bounds.
    pub fn window(&self, start: usize, end: usize) -> Option<Self> {
        let validity = self.validity.get(start..end)?;
        let dense_start = self.validity[..start].count_ones();
        let dense_end = dense_start + validity.count_ones();
        Some(OptionBoolSlice {
            values: self.values.get(dense_start..dense_end)?,
            validity,
        })
    }

    /// Only the `Some` values, densely packed, in order.
    pub fn values(&self) -> &'a BitSlice<u8, Lsb0> {
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

    pub fn iter(&self) -> OptionBoolSliceIter<'a> {
        OptionBoolSliceIter {
            slice: *self,
            front: 0,
            back: self.len(),
            dense: 0,
        }
    }
}

/// Iterator over an [`OptionBoolSlice`]'s samples; carries a running cursor
/// into the dense values instead of popcounting per item.
pub struct OptionBoolSliceIter<'a> {
    slice: OptionBoolSlice<'a>,
    front: usize,
    back: usize,
    dense: usize,
}

impl<'a> Iterator for OptionBoolSliceIter<'a> {
    type Item = Option<bool>;

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
        let value = self.slice.values.get(self.dense).map(|b| *b)?;
        self.dense += 1;
        Some(Some(value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.back - self.front;
        (n, Some(n))
    }
}

impl<'a> ExactSizeIterator for OptionBoolSliceIter<'a> {}

impl<'a> IntoIterator for OptionBoolSlice<'a> {
    type Item = Option<bool>;
    type IntoIter = OptionBoolSliceIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, 'n> IntoIterator for &'n OptionBoolSlice<'a> {
    type Item = Option<bool>;
    type IntoIter = OptionBoolSliceIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> RowsRef for OptionBoolSlice<'a> {
    type Row = Option<bool>;

    fn len(&self) -> usize {
        Self::len(self)
    }

    fn get(&self, i: usize) -> Option<Option<bool>> {
        Self::get(self, i)
    }

    fn window(&self, start: usize, end: usize) -> Option<Self> {
        Self::window(self, start, end)
    }
}

impl<'a> OptionBoolSlice<'a> {
    /// The samples as a `PyroValue::List`: each `Some` as a
    /// `PyroValue::Bool`, each `None` as `PyroValue::Null`.
    pub fn into_value(&self) -> PyroValue<'a> {
        PyroValue::List(
            self.iter()
                .map(|v| v.map_or(PyroValue::Null, PyroValue::Bool))
                .collect(),
        )
    }
}

impl<'a> Realize<Vec<Option<bool>>> for OptionBoolSlice<'a> {
    fn realize(&self) -> Vec<Option<bool>> {
        self.iter().collect()
    }
}

/// Equal when the same samples are present and the present values are
/// equal: compares the validity bits and the dense values directly, never
/// sample by sample.
impl<'a, 'b> PartialEq<OptionBoolSlice<'b>> for OptionBoolSlice<'a> {
    fn eq(&self, other: &OptionBoolSlice<'b>) -> bool {
        self.validity == other.validity && self.values == other.values
    }
}

impl<'a> Eq for OptionBoolSlice<'a> {}

impl<'a> PartialEq<[Option<bool>]> for OptionBoolSlice<'a> {
    fn eq(&self, other: &[Option<bool>]) -> bool {
        self.len() == other.len() && self.iter().zip(other).all(|(a, b)| a == *b)
    }
}

impl<'a> PartialEq<Vec<Option<bool>>> for OptionBoolSlice<'a> {
    fn eq(&self, other: &Vec<Option<bool>>) -> bool {
        *self == *other.as_slice()
    }
}

impl<'a> PartialEq<OptionBoolSlice<'a>> for Vec<Option<bool>> {
    fn eq(&self, other: &OptionBoolSlice<'a>) -> bool {
        *other == *self.as_slice()
    }
}

// =============================================================================
// OptionBoolNested — `Vec<…<Vec<Option<bool>>>…>`
// =============================================================================

/// [`OptionNested`](super::OptionNested) for `bool`: identical API, but the
/// dense `Some` values are bit-packed too. `N` is at least 1; a bare
/// `Vec<Option<bool>>` is an [`OptionBoolSlice`].
#[derive(Debug, Clone, Copy)]
pub struct OptionBoolNested<'a, W: StructureWord, const N: usize> {
    topology: Topology<'a, W, N>,
    /// The `Some` values only, densely packed.
    slice: &'a BitSlice<u8, Lsb0>,
    /// One bit per sample, nulls included; `true` = `Some`.
    option: &'a BitSlice<u8, Lsb0>,
}

impl<'a, W: StructureWord, const N: usize> OptionBoolNested<'a, W, N> {
    pub fn new(
        topology: Topology<'a, W, N>,
        slice: &'a BitSlice<u8, Lsb0>,
        option: &'a BitSlice<u8, Lsb0>,
    ) -> Result<Self, WireError> {
        if topology.total() != option.len() {
            return Err(WireError::InvalidTopologyLen {
                topo: topology.total(),
                data: option.len(),
            });
        }
        check_option_len(slice, option, 1)?;
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
        Some(OptionBoolNested {
            topology: self.topology.window(start, end)?,
            ..*self
        })
    }

    pub fn iter(&self) -> OptionBoolNestedIter<'a, W, N> {
        OptionBoolNestedIter {
            nest: *self,
            front: 0,
            back: self.len(),
            dense: None,
        }
    }
}

impl<'a, W: StructureWord> OptionBoolNested<'a, W, 1> {
    /// List `i` as a self-contained [`OptionBoolSlice`]. Costs a popcount
    /// over every sample before list `i`; use [`Self::iter`] to walk every
    /// list.
    pub fn get(&self, i: usize) -> Option<OptionBoolSlice<'a>> {
        let (start, end) = self.topology.leaf_range(i)?;
        OptionBoolSlice {
            values: self.slice,
            validity: self.option,
        }
        .window(start, end)
    }

    /// Alias for [`Self::get`].
    pub fn descend(&self, i: usize) -> Option<OptionBoolSlice<'a>> {
        self.get(i)
    }
}

impl<'a, W: StructureWord, const N: usize> OptionBoolNested<'a, W, N> {
    /// The elements of this view's outermost level as nested
    /// `PyroValue::List`s down to the samples (see
    /// [`OptionBoolSlice::into_value`]).
    ///
    /// `levels` is how many of the view's `N` levels the data uses
    /// (`1 <= levels <= N`): a view read at [`crate::MAX_TIERS`] levels for a
    /// schema only known at runtime uses fewer.
    pub fn into_value(&self, levels: usize) -> Vec<PyroValue<'a>> {
        let (slice, option) = (self.slice, self.option);
        self.topology.fold_rows(levels, |leaf| {
            OptionBoolNested::<'a, W, 1> {
                topology: leaf,
                slice,
                option,
            }
            .iter()
            .map(|row| row.into_value())
        })
    }
}

impl<'a, W: StructureWord> Realize<Vec<Vec<Option<bool>>>> for OptionBoolNested<'a, W, 1> {
    fn realize(&self) -> Vec<Vec<Option<bool>>> {
        self.iter().map(|list| list.realize()).collect()
    }
}

macro_rules! bool_option_slice_level {
    ($n:literal => $m:literal) => {
        impl<'a, W: StructureWord> OptionBoolNested<'a, W, $n> {
            /// Element `i`, one level down, `O(1)`.
            pub fn get(&self, i: usize) -> Option<OptionBoolNested<'a, W, $m>> {
                Some(OptionBoolNested {
                    topology: self.topology.descend(i)?,
                    slice: self.slice,
                    option: self.option,
                })
            }

            /// Alias for [`Self::get`].
            pub fn descend(&self, i: usize) -> Option<OptionBoolNested<'a, W, $m>> {
                self.get(i)
            }
        }

        impl<'a, W: StructureWord, O> Realize<Vec<O>> for OptionBoolNested<'a, W, $n>
        where
            OptionBoolNested<'a, W, $m>: Realize<O>,
        {
            fn realize(&self) -> Vec<O> {
                self.iter().map(|child| child.realize()).collect()
            }
        }
    };
}

bool_option_slice_level!(2 => 1);
bool_option_slice_level!(3 => 2);
bool_option_slice_level!(4 => 3);
bool_option_slice_level!(5 => 4);

/// Iterator over a [`OptionBoolNested`]'s outermost level; yields what
/// `get(i)` would. At `N = 1` it carries a running cursor into the dense
/// values instead of popcounting per item, so walking a whole column is
/// linear.
pub struct OptionBoolNestedIter<'a, W: StructureWord, const N: usize> {
    nest: OptionBoolNested<'a, W, N>,
    front: usize,
    back: usize,
    /// Index into `nest.slice` of the next list's first `Some` value;
    /// `None` until the first item computes it.
    dense: Option<usize>,
}

impl<'a, W: StructureWord> Iterator for OptionBoolNestedIter<'a, W, 1> {
    type Item = OptionBoolSlice<'a>;

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
        Some(OptionBoolSlice {
            values: self.nest.slice.get(dense..dense_end)?,
            validity,
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.back - self.front;
        (n, Some(n))
    }
}

macro_rules! bool_option_slice_iter {
    ($n:literal => $m:literal) => {
        impl<'a, W: StructureWord> Iterator for OptionBoolNestedIter<'a, W, $n> {
            type Item = OptionBoolNested<'a, W, $m>;

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

bool_option_slice_iter!(2 => 1);
bool_option_slice_iter!(3 => 2);
bool_option_slice_iter!(4 => 3);
bool_option_slice_iter!(5 => 4);

macro_rules! bool_option_slice_exact {
    ($($n:literal),*) => {
        $(
            impl<'a, W: StructureWord> ExactSizeIterator for OptionBoolNestedIter<'a, W, $n> {}

            impl<'a, W: StructureWord> IntoIterator for OptionBoolNested<'a, W, $n> {
                type Item = <OptionBoolNestedIter<'a, W, $n> as Iterator>::Item;
                type IntoIter = OptionBoolNestedIter<'a, W, $n>;

                fn into_iter(self) -> Self::IntoIter {
                    self.iter()
                }
            }
        )*
    };
}

bool_option_slice_exact!(1, 2, 3, 4, 5);

macro_rules! bool_option_slice_rows_ref {
    ($($n:literal => $row:ty),* $(,)?) => {
        $(
            impl<'a, W: StructureWord> RowsRef for OptionBoolNested<'a, W, $n> {
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

bool_option_slice_rows_ref!(
    1 => OptionBoolSlice<'a>,
    2 => OptionBoolNested<'a, W, 1>,
    3 => OptionBoolNested<'a, W, 2>,
    4 => OptionBoolNested<'a, W, 3>,
    5 => OptionBoolNested<'a, W, 4>,
);

// =============================================================================
// OptionBoolNested equality — compares like the `Vec<…<Vec<Option<bool>>>…>`
// it stands for, against another view or against `Vec`s (both directions)
// =============================================================================

macro_rules! bool_option_slice_eq {
    ($($n:literal => $child:ty),* $(,)?) => {
        $(
            impl<'a, 'b, W: StructureWord> PartialEq<OptionBoolNested<'b, W, $n>> for OptionBoolNested<'a, W, $n> {
                fn eq(&self, other: &OptionBoolNested<'b, W, $n>) -> bool {
                    self.len() == other.len()
                        && self.iter().zip(other.iter()).all(|(a, b)| a == b)
                }
            }

            impl<'a, W: StructureWord> Eq for OptionBoolNested<'a, W, $n> {}

            impl<'a, W: StructureWord, V> PartialEq<[V]> for OptionBoolNested<'a, W, $n>
            where
                $child: PartialEq<V>,
            {
                fn eq(&self, other: &[V]) -> bool {
                    self.len() == other.len() && self.iter().zip(other).all(|(a, b)| a == *b)
                }
            }

            impl<'a, W: StructureWord, V> PartialEq<Vec<V>> for OptionBoolNested<'a, W, $n>
            where
                $child: PartialEq<V>,
            {
                fn eq(&self, other: &Vec<V>) -> bool {
                    *self == *other.as_slice()
                }
            }

            impl<'a, W: StructureWord, V> PartialEq<OptionBoolNested<'a, W, $n>> for Vec<V>
            where
                $child: PartialEq<V>,
            {
                fn eq(&self, other: &OptionBoolNested<'a, W, $n>) -> bool {
                    *other == *self.as_slice()
                }
            }
        )*
    };
}

bool_option_slice_eq!(
    1 => OptionBoolSlice<'a>,
    2 => OptionBoolNested<'a, W, 1>,
    3 => OptionBoolNested<'a, W, 2>,
    4 => OptionBoolNested<'a, W, 3>,
    5 => OptionBoolNested<'a, W, 4>,
);

// =============================================================================
// OptionFixedBoolSlice — `Vec<Option<[bool; M]>>`
// =============================================================================

/// Bits of dense arrays `[start, end)`.
fn dense_arrays<const M: usize>(
    slice: &BitSlice<u8, Lsb0>,
    start: usize,
    end: usize,
) -> Option<&BitSlice<u8, Lsb0>> {
    slice.get(start.checked_mul(M)?..end.checked_mul(M)?)
}

/// [`OptionBoolSlice`] for `[bool; M]`: identical API, but each dense
/// `Some` value is `M` bits wide. Rows are `Option<[bool; M]>` by value
/// (see [`FixedBoolSlice`](super::FixedBoolSlice) for why not a
/// `BitArray`); [`Self::get_bits`] borrows the bits instead.
///
/// ```
/// use pyroformat::refs::OptionFixedBoolSlice;
/// use pyroformat::{BitSlice, Lsb0, Realize};
///
/// // [None, Some([true, true])]
/// let values = [0b0000_0011u8];
/// let bits = [0b0000_0010u8];
/// let slice = OptionFixedBoolSlice::<2>::new(
///     &BitSlice::<u8, Lsb0>::from_slice(&values)[..2],
///     &BitSlice::<u8, Lsb0>::from_slice(&bits)[..2],
/// )
/// .unwrap();
/// assert_eq!(slice.get(0), Some(None));
/// assert_eq!(slice.get(1), Some(Some([true, true])));
/// assert_eq!(slice.null_count(), 1);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct OptionFixedBoolSlice<'a, const M: usize> {
    /// The `Some` arrays only, `M` bits each, densely packed.
    values: &'a BitSlice<u8, Lsb0>,
    /// One bit per sample, nulls included; `true` = `Some`.
    validity: &'a BitSlice<u8, Lsb0>,
}

impl<'a, const M: usize> OptionFixedBoolSlice<'a, M> {
    /// A view over `values` (the `Some` arrays' bits, densely packed) and
    /// `validity` (one bit per sample), which must have one set bit per
    /// `M`-bit array.
    pub fn new(
        values: &'a BitSlice<u8, Lsb0>,
        validity: &'a BitSlice<u8, Lsb0>,
    ) -> Result<Self, WireError> {
        check_option_len(values, validity, M)?;
        Ok(Self { values, validity })
    }

    /// Samples, nulls included.
    pub fn len(&self) -> usize {
        self.validity.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Sample `i`: `None` when out of bounds, `Some(None)` for a null.
    /// Costs a popcount over the validity bits before `i`; use
    /// [`Self::iter`] to walk every sample.
    pub fn get(&self, i: usize) -> Option<Option<[bool; M]>> {
        Some(
            self.get_bits(i)?
                .map(|bits| std::array::from_fn(|j| bits[j])),
        )
    }

    /// Sample `i` as its `M` bits, without copying them out.
    pub fn get_bits(&self, i: usize) -> Option<Option<&'a BitSlice<u8, Lsb0>>> {
        if !*self.validity.get(i)? {
            return Some(None);
        }
        let rank = self.validity[..i].count_ones();
        Some(Some(dense_arrays::<M>(self.values, rank, rank + 1)?))
    }

    /// Samples `[start, end)`, re-sliced to exactly the `Some` arrays inside
    /// the window (one popcount). `None` when out of bounds.
    pub fn window(&self, start: usize, end: usize) -> Option<Self> {
        let validity = self.validity.get(start..end)?;
        let dense_start = self.validity[..start].count_ones();
        let dense_end = dense_start + validity.count_ones();
        Some(OptionFixedBoolSlice {
            values: dense_arrays::<M>(self.values, dense_start, dense_end)?,
            validity,
        })
    }

    /// Only the `Some` arrays' bits, densely packed, in order.
    pub fn values(&self) -> &'a BitSlice<u8, Lsb0> {
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

    pub fn iter(&self) -> OptionFixedBoolSliceIter<'a, M> {
        OptionFixedBoolSliceIter {
            slice: *self,
            front: 0,
            back: self.len(),
            dense: 0,
        }
    }
}

/// Iterator over an [`OptionFixedBoolSlice`]'s samples; carries a running
/// cursor into the dense arrays instead of popcounting per item.
pub struct OptionFixedBoolSliceIter<'a, const M: usize> {
    slice: OptionFixedBoolSlice<'a, M>,
    front: usize,
    back: usize,
    /// Index (in arrays) of the next `Some`.
    dense: usize,
}

impl<'a, const M: usize> Iterator for OptionFixedBoolSliceIter<'a, M> {
    type Item = Option<[bool; M]>;

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
        let bits = dense_arrays::<M>(self.slice.values, self.dense, self.dense + 1)?;
        self.dense += 1;
        Some(Some(std::array::from_fn(|j| bits[j])))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.back - self.front;
        (n, Some(n))
    }
}

impl<'a, const M: usize> ExactSizeIterator for OptionFixedBoolSliceIter<'a, M> {}

impl<'a, const M: usize> IntoIterator for OptionFixedBoolSlice<'a, M> {
    type Item = Option<[bool; M]>;
    type IntoIter = OptionFixedBoolSliceIter<'a, M>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, 'n, const M: usize> IntoIterator for &'n OptionFixedBoolSlice<'a, M> {
    type Item = Option<[bool; M]>;
    type IntoIter = OptionFixedBoolSliceIter<'a, M>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, const M: usize> RowsRef for OptionFixedBoolSlice<'a, M> {
    type Row = Option<[bool; M]>;

    fn len(&self) -> usize {
        Self::len(self)
    }

    fn get(&self, i: usize) -> Option<Option<[bool; M]>> {
        Self::get(self, i)
    }

    fn window(&self, start: usize, end: usize) -> Option<Self> {
        Self::window(self, start, end)
    }
}

impl<'a, const M: usize> OptionFixedBoolSlice<'a, M> {
    /// The samples as a `PyroValue::List`: each `Some` array as a bool
    /// `PrimitiveList`, each `None` as `PyroValue::Null`.
    pub fn into_value(&self) -> PyroValue<'a> {
        PyroValue::List(
            self.iter()
                .map(|v| -> PyroValue<'a> { v.map_or(PyroValue::Null, PyroValue::from) })
                .collect(),
        )
    }
}

impl<'a, const M: usize> Realize<Vec<Option<[bool; M]>>> for OptionFixedBoolSlice<'a, M> {
    fn realize(&self) -> Vec<Option<[bool; M]>> {
        self.iter().collect()
    }
}

/// Equal when the same samples are present and the present arrays are
/// equal: compares the validity bits and the dense bits directly.
impl<'a, 'b, const M: usize> PartialEq<OptionFixedBoolSlice<'b, M>>
    for OptionFixedBoolSlice<'a, M>
{
    fn eq(&self, other: &OptionFixedBoolSlice<'b, M>) -> bool {
        self.validity == other.validity && self.values == other.values
    }
}

impl<'a, const M: usize> Eq for OptionFixedBoolSlice<'a, M> {}

impl<'a, const M: usize> PartialEq<[Option<[bool; M]>]> for OptionFixedBoolSlice<'a, M> {
    fn eq(&self, other: &[Option<[bool; M]>]) -> bool {
        self.len() == other.len() && self.iter().zip(other).all(|(a, b)| a == *b)
    }
}

impl<'a, const M: usize> PartialEq<Vec<Option<[bool; M]>>> for OptionFixedBoolSlice<'a, M> {
    fn eq(&self, other: &Vec<Option<[bool; M]>>) -> bool {
        *self == *other.as_slice()
    }
}

impl<'a, const M: usize> PartialEq<OptionFixedBoolSlice<'a, M>> for Vec<Option<[bool; M]>> {
    fn eq(&self, other: &OptionFixedBoolSlice<'a, M>) -> bool {
        *other == *self.as_slice()
    }
}

// =============================================================================
// OptionFixedBoolNested — `Vec<…<Vec<Option<[bool; M]>>>…>`
// =============================================================================

/// [`OptionBoolNested`] for `[bool; M]`: identical API, but each
/// dense `Some` value is `M` bits wide. `N` is at least 1; a bare
/// `Vec<Option<[bool; M]>>` is an [`OptionFixedBoolSlice`].
///
/// ```
/// use pyroformat::refs::{OptionFixedBoolNested, Topology};
/// use pyroformat::schema::StructureWord;
/// const R: u32 = <u32 as StructureWord>::RESET;
/// use pyroformat::{BitSlice, Lsb0, Realize};
///
/// // [[Some([true, false]), None], [None, Some([true, true])]]
/// let values = [0b0000_1101u8]; // the two `Some`s, two bits each: 10 11
/// let bits = [0b0000_1001u8];   // samples: 1 0 | 0 1
/// let rows = [R, 2, 4];
/// let nest = OptionFixedBoolNested::<u32, 2, 1> {
///     topology: Topology { topology: [&rows[..]] },
///     slice: &BitSlice::<u8, Lsb0>::from_slice(&values)[..4],
///     option: &BitSlice::<u8, Lsb0>::from_slice(&bits)[..4],
///     count: 4,
/// };
///
/// let last = nest.get(1).unwrap();
/// assert_eq!(last.get(0), Some(None));
/// assert_eq!(last.get(1), Some(Some([true, true])));
/// assert_eq!(last.null_count(), 1);
/// assert_eq!(
///     nest.realize(),
///     vec![vec![Some([true, false]), None], vec![None, Some([true, true])]]
/// );
/// ```
#[derive(Debug, Clone, Copy)]
pub struct OptionFixedBoolNested<'a, W: StructureWord, const M: usize, const N: usize> {
    pub topology: Topology<'a, W, N>,
    /// The `Some` arrays only, `M` bits each, densely packed.
    pub slice: &'a BitSlice<u8, Lsb0>,
    /// One bit per sample, nulls included; `true` = `Some`.
    pub option: &'a BitSlice<u8, Lsb0>,
    /// Samples, nulls included.
    pub count: usize,
}

impl<'a, W: StructureWord, const M: usize, const N: usize> OptionFixedBoolNested<'a, W, M, N> {
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
        Some(OptionFixedBoolNested {
            topology: self.topology.window(start, end)?,
            ..*self
        })
    }

    pub fn iter(&self) -> OptionFixedBoolNestedIter<'a, W, M, N> {
        OptionFixedBoolNestedIter {
            nest: *self,
            front: 0,
            back: self.len(),
            dense: None,
        }
    }
}

impl<'a, W: StructureWord, const M: usize> OptionFixedBoolNested<'a, W, M, 1> {
    /// List `i` as a self-contained [`OptionFixedBoolSlice`]. Costs a
    /// popcount over every sample before list `i`; use [`Self::iter`] to
    /// walk every list.
    pub fn get(&self, i: usize) -> Option<OptionFixedBoolSlice<'a, M>> {
        let (start, end) = self.topology.leaf_range(i)?;
        OptionFixedBoolSlice {
            values: self.slice,
            validity: &self.option[..self.count.min(self.option.len())],
        }
        .window(start, end)
    }

    /// Alias for [`Self::get`].
    pub fn descend(&self, i: usize) -> Option<OptionFixedBoolSlice<'a, M>> {
        self.get(i)
    }
}

impl<'a, W: StructureWord, const M: usize, const N: usize> OptionFixedBoolNested<'a, W, M, N> {
    /// The elements of this view's outermost level as nested
    /// `PyroValue::List`s down to the samples (see
    /// [`OptionFixedBoolSlice::into_value`]).
    ///
    /// `levels` is how many of the view's `N` levels the data uses
    /// (`1 <= levels <= N`): a view read at [`crate::MAX_TIERS`] levels for a
    /// schema only known at runtime uses fewer.
    pub fn into_value(&self, levels: usize) -> Vec<PyroValue<'a>> {
        let (slice, option, count) = (self.slice, self.option, self.count);
        self.topology.fold_rows(levels, |leaf| {
            OptionFixedBoolNested::<'a, W, M, 1> {
                topology: leaf,
                slice,
                option,
                count,
            }
            .iter()
            .map(|row| row.into_value())
        })
    }
}

impl<'a, W: StructureWord, const M: usize> Realize<Vec<Vec<Option<[bool; M]>>>>
    for OptionFixedBoolNested<'a, W, M, 1>
{
    fn realize(&self) -> Vec<Vec<Option<[bool; M]>>> {
        self.iter().map(|list| list.realize()).collect()
    }
}

macro_rules! fixed_bool_option_slice_level {
    ($n:literal => $m:literal) => {
        impl<'a, W: StructureWord, const M: usize> OptionFixedBoolNested<'a, W, M, $n> {
            /// Element `i`, one level down, `O(1)`.
            pub fn get(&self, i: usize) -> Option<OptionFixedBoolNested<'a, W, M, $m>> {
                Some(OptionFixedBoolNested {
                    topology: self.topology.descend(i)?,
                    slice: self.slice,
                    option: self.option,
                    count: self.count,
                })
            }

            /// Alias for [`Self::get`].
            pub fn descend(&self, i: usize) -> Option<OptionFixedBoolNested<'a, W, M, $m>> {
                self.get(i)
            }
        }

        impl<'a, W: StructureWord, const M: usize, O> Realize<Vec<O>>
            for OptionFixedBoolNested<'a, W, M, $n>
        where
            OptionFixedBoolNested<'a, W, M, $m>: Realize<O>,
        {
            fn realize(&self) -> Vec<O> {
                self.iter().map(|child| child.realize()).collect()
            }
        }
    };
}

fixed_bool_option_slice_level!(2 => 1);
fixed_bool_option_slice_level!(3 => 2);
fixed_bool_option_slice_level!(4 => 3);
fixed_bool_option_slice_level!(5 => 4);

/// Iterator over a [`OptionFixedBoolNested`]'s outermost level;
/// yields what `get(i)` would. At `N = 1` it carries a running cursor into
/// the dense arrays instead of popcounting per item, so walking a whole
/// column is linear.
pub struct OptionFixedBoolNestedIter<'a, W: StructureWord, const M: usize, const N: usize> {
    nest: OptionFixedBoolNested<'a, W, M, N>,
    front: usize,
    back: usize,
    /// Index (in arrays) into `nest.slice` of the next list's first `Some`;
    /// `None` until the first item computes it.
    dense: Option<usize>,
}

impl<'a, W: StructureWord, const M: usize> Iterator for OptionFixedBoolNestedIter<'a, W, M, 1> {
    type Item = OptionFixedBoolSlice<'a, M>;

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
        Some(OptionFixedBoolSlice {
            values: dense_arrays::<M>(self.nest.slice, dense, dense_end)?,
            validity,
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.back - self.front;
        (n, Some(n))
    }
}

macro_rules! fixed_bool_option_slice_iter {
    ($n:literal => $m:literal) => {
        impl<'a, W: StructureWord, const M: usize> Iterator
            for OptionFixedBoolNestedIter<'a, W, M, $n>
        {
            type Item = OptionFixedBoolNested<'a, W, M, $m>;

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

fixed_bool_option_slice_iter!(2 => 1);
fixed_bool_option_slice_iter!(3 => 2);
fixed_bool_option_slice_iter!(4 => 3);
fixed_bool_option_slice_iter!(5 => 4);

macro_rules! fixed_bool_option_slice_exact {
    ($($n:literal => $row:ty),* $(,)?) => {
        $(
            impl<'a, W: StructureWord, const M: usize> ExactSizeIterator
                for OptionFixedBoolNestedIter<'a, W, M, $n>
            {
            }

            impl<'a, W: StructureWord, const M: usize> IntoIterator for OptionFixedBoolNested<'a, W, M, $n> {
                type Item = $row;
                type IntoIter = OptionFixedBoolNestedIter<'a, W, M, $n>;

                fn into_iter(self) -> Self::IntoIter {
                    self.iter()
                }
            }

            impl<'a, 'n, W: StructureWord, const M: usize> IntoIterator
                for &'n OptionFixedBoolNested<'a, W, M, $n>
            {
                type Item = $row;
                type IntoIter = OptionFixedBoolNestedIter<'a, W, M, $n>;

                fn into_iter(self) -> Self::IntoIter {
                    self.iter()
                }
            }

            impl<'a, W: StructureWord, const M: usize> RowsRef for OptionFixedBoolNested<'a, W, M, $n> {
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

fixed_bool_option_slice_exact!(
    1 => OptionFixedBoolSlice<'a, M>,
    2 => OptionFixedBoolNested<'a, W, M, 1>,
    3 => OptionFixedBoolNested<'a, W, M, 2>,
    4 => OptionFixedBoolNested<'a, W, M, 3>,
    5 => OptionFixedBoolNested<'a, W, M, 4>,
);

// =============================================================================
// OptionFixedBoolNested equality — compares like the
// `Vec<…<Vec<Option<[bool; M]>>>…>` it stands for, against another view or
// against `Vec`s (both directions)
// =============================================================================

macro_rules! fixed_bool_option_slice_eq {
    ($($n:literal => $child:ty),* $(,)?) => {
        $(
            impl<'a, 'b, W: StructureWord, const M: usize> PartialEq<OptionFixedBoolNested<'b, W, M, $n>>
                for OptionFixedBoolNested<'a, W, M, $n>
            {
                fn eq(&self, other: &OptionFixedBoolNested<'b, W, M, $n>) -> bool {
                    self.len() == other.len()
                        && self.iter().zip(other.iter()).all(|(a, b)| a == b)
                }
            }

            impl<'a, W: StructureWord, const M: usize> Eq for OptionFixedBoolNested<'a, W, M, $n> {}

            impl<'a, W: StructureWord, V, const M: usize> PartialEq<[V]> for OptionFixedBoolNested<'a, W, M, $n>
            where
                $child: PartialEq<V>,
            {
                fn eq(&self, other: &[V]) -> bool {
                    self.len() == other.len() && self.iter().zip(other).all(|(a, b)| a == *b)
                }
            }

            impl<'a, W: StructureWord, V, const M: usize> PartialEq<Vec<V>> for OptionFixedBoolNested<'a, W, M, $n>
            where
                $child: PartialEq<V>,
            {
                fn eq(&self, other: &Vec<V>) -> bool {
                    *self == *other.as_slice()
                }
            }

            impl<'a, W: StructureWord, V, const M: usize> PartialEq<OptionFixedBoolNested<'a, W, M, $n>> for Vec<V>
            where
                $child: PartialEq<V>,
            {
                fn eq(&self, other: &OptionFixedBoolNested<'a, W, M, $n>) -> bool {
                    *other == *self.as_slice()
                }
            }
        )*
    };
}

fixed_bool_option_slice_eq!(
    1 => OptionFixedBoolSlice<'a, M>,
    2 => OptionFixedBoolNested<'a, W, M, 1>,
    3 => OptionFixedBoolNested<'a, W, M, 2>,
    4 => OptionFixedBoolNested<'a, W, M, 3>,
    5 => OptionFixedBoolNested<'a, W, M, 4>,
);

#[cfg(test)]
mod tests {
    use crate::BitVec;

    use super::*;
    use pyro_schema::StructureWord;

    const R: u32 = <u32 as StructureWord>::RESET;

    fn bits(pattern: &[bool]) -> BitVec {
        pattern.iter().copied().collect()
    }

    #[test]
    fn bool_option_matches_primitive() {
        let l0 = [R, 2, 2, 3];
        let dense = bits(&[true, false]);
        let mask = bits(&[true, false, true]);
        let nest: OptionBoolNested<'_, u32, 1> = OptionBoolNested {
            topology: Topology { topology: [&l0] },
            slice: &dense,
            option: &mask,
        };
        assert_eq!(
            nest.realize(),
            vec![vec![Some(true), None], vec![], vec![Some(false)]]
        );
        assert_eq!(nest.get(2).unwrap().get(0), Some(Some(false)));
        assert_eq!(nest.iter().len(), 3);
    }

    #[test]
    fn equality_matches_owned_bool_options() {
        let dense = bits(&[true, false]);
        let bmask = bits(&[true, false, true]);
        let bools = OptionBoolSlice::new(&dense, &bmask).unwrap();
        assert_eq!(bools, vec![Some(true), None, Some(false)]);
        assert_eq!(bools, bools);
        assert_eq!(RowsRef::get(&bools, 2), Some(Some(false)));
        assert!(OptionBoolSlice::new(&dense[..1], &bmask).is_err());
    }

    // [[Some([T, F]), None], [], [None, Some([F, F]), Some([T, T])]]
    fn fixed_bool_owned() -> Vec<Vec<Option<[bool; 2]>>> {
        vec![
            vec![Some([true, false]), None],
            vec![],
            vec![None, Some([false, false]), Some([true, true])],
        ]
    }

    fn fixed_bool_parts() -> (BitVec, BitVec) {
        let owned = fixed_bool_owned();
        let samples: Vec<Option<[bool; 2]>> = owned.into_iter().flatten().collect();
        let values: Vec<bool> = samples.iter().flatten().flatten().copied().collect();
        let validity: Vec<bool> = samples.iter().map(Option::is_some).collect();
        (bits(&values), bits(&validity))
    }

    #[test]
    fn fixed_bool_option_slice_behaves_like_nested_vecs() {
        let (values, validity) = fixed_bool_parts();
        let rows = [R, 2, 2, 5];
        let nest: OptionFixedBoolNested<'_, u32, 2, 1> = OptionFixedBoolNested {
            topology: Topology { topology: [&rows] },
            slice: &values,
            option: &&validity[..5],
            count: 5,
        };
        let owned = fixed_bool_owned();

        assert_eq!(nest.len(), 3);
        assert_eq!(nest, owned);
        assert_eq!(owned, nest);
        assert_eq!(nest.realize(), owned);
        assert!(nest.get(3).is_none());
        assert_eq!(nest.window(1, 3).unwrap(), owned[1..].to_vec());

        // `get` (popcount) and `iter` (running cursor) agree.
        for (i, by_iter) in nest.iter().enumerate() {
            let by_get = nest.get(i).unwrap();
            assert_eq!(by_iter, by_get);
            assert_eq!(by_get, owned[i]);
            assert_eq!(RowsRef::get(&nest, i).unwrap(), by_get);
        }

        let last = nest.get(2).unwrap();
        assert_eq!(last.null_count(), 1);
        assert_eq!(last.values().len(), 4);
        assert_eq!(last.get(0), Some(None));
        assert_eq!(last.get(2), Some(Some([true, true])));
        assert_eq!(last.get_bits(1).unwrap().unwrap().len(), 2);
        assert!(last.get(3).is_none());
        assert_eq!(last.window(1, 3).unwrap(), owned[2][1..].to_vec());
        assert_eq!(last.window(2, 3).unwrap().get(0), Some(Some([true, true])));
        assert_ne!(last, nest.get(0).unwrap());
        assert_eq!(last.iter().len(), 3);
    }

    #[test]
    fn deeper_fixed_bool_option_slices() {
        let (values, validity) = fixed_bool_parts();
        // [[l0, l1], [l2]]
        let l0 = [R, 2, 3];
        let l1 = [R, 2, 2, 5];
        let nest: OptionFixedBoolNested<'_, u32, 2, 2> = OptionFixedBoolNested {
            topology: Topology {
                topology: [&l0, &l1],
            },
            slice: &values,
            option: &&validity[..5],
            count: 5,
        };
        let o = fixed_bool_owned();
        let expected = vec![vec![o[0].clone(), o[1].clone()], vec![o[2].clone()]];
        assert_eq!(nest, expected);
        assert_eq!(nest.realize(), expected);
        assert_eq!(nest.get(1).unwrap(), expected[1]);
        assert_eq!(nest.descend(1).unwrap().get(0).unwrap(), o[2]);
    }
}
