//! Columns of bare fixed arrays, `[T; N]` and `Option<[T; N]>`, as fields of
//! a repeated struct: every row is a borrow into the buffer, `&'a [T; N]`.
//!
//! A plain `&'a [[T; N]]` would hand out its rows by value through the
//! blanket `RowsRef` for slices, and a derived struct's row view has to have
//! the same field type as its column's rows — so the column needs its own
//! type to keep the array a borrow. [`OptionFixedSlice`] is the same over an
//! [`OptionSlice`] of arrays.
//!
//! `[bool; N]` has neither: it is bit-packed, so there is no `[bool; N]` in
//! the buffer to borrow (see [`super::FixedBoolSlice`]).

use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;

use crate::{Realize, RowsRef, errors::WireError};

use super::OptionSlice;

// =============================================================================
// FixedSlice — a column of `[T; N]`
// =============================================================================

/// A column of `[T; N]` arrays, back to back in the buffer. Row `i` is a
/// borrow of array `i`.
#[derive(Debug)]
pub struct FixedSlice<'a, T, const N: usize> {
    arrays: &'a [[T; N]],
}

impl<'a, T, const N: usize> Clone for FixedSlice<'a, T, N> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T, const N: usize> Copy for FixedSlice<'a, T, N> {}

impl<'a, T, const N: usize> FixedSlice<'a, T, N> {
    pub fn new(arrays: &'a [[T; N]]) -> Self {
        Self { arrays }
    }

    /// Arrays in the column.
    pub fn len(&self) -> usize {
        self.arrays.len()
    }

    pub fn is_empty(&self) -> bool {
        self.arrays.is_empty()
    }

    /// Array `i`, borrowed; `None` when out of bounds.
    pub fn get(&self, i: usize) -> Option<&'a [T; N]> {
        self.arrays.get(i)
    }

    /// Arrays `[start, end)`. `None` when out of bounds.
    pub fn window(&self, start: usize, end: usize) -> Option<Self> {
        Some(Self {
            arrays: self.arrays.get(start..end)?,
        })
    }

    /// The whole column as a slice of arrays.
    pub fn as_slice(&self) -> &'a [[T; N]] {
        self.arrays
    }

    pub fn iter(&self) -> std::slice::Iter<'a, [T; N]> {
        self.arrays.iter()
    }
}

impl<'a, T, const N: usize> IntoIterator for FixedSlice<'a, T, N> {
    type Item = &'a [T; N];
    type IntoIter = std::slice::Iter<'a, [T; N]>;

    fn into_iter(self) -> Self::IntoIter {
        self.arrays.iter()
    }
}

impl<'a, T, const N: usize> RowsRef for FixedSlice<'a, T, N> {
    type Row = &'a [T; N];

    fn len(&self) -> usize {
        Self::len(self)
    }

    fn get(&self, i: usize) -> Option<&'a [T; N]> {
        Self::get(self, i)
    }

    fn window(&self, start: usize, end: usize) -> Option<Self> {
        Self::window(self, start, end)
    }
}

impl<'a, T: Copy, const N: usize> Realize<Vec<[T; N]>> for FixedSlice<'a, T, N> {
    fn realize(&self) -> Vec<[T; N]> {
        self.arrays.to_vec()
    }
}

impl<'a, 'b, T: PartialEq<U>, U, const N: usize> PartialEq<FixedSlice<'b, U, N>>
    for FixedSlice<'a, T, N>
{
    fn eq(&self, other: &FixedSlice<'b, U, N>) -> bool {
        self.arrays == other.arrays
    }
}

impl<'a, T: PartialEq<U>, U, const N: usize> PartialEq<[[U; N]]> for FixedSlice<'a, T, N> {
    fn eq(&self, other: &[[U; N]]) -> bool {
        self.arrays == other
    }
}

// =============================================================================
// OptionFixedSlice — a column of `Option<[T; N]>`
// =============================================================================

/// A column of `Option<[T; N]>`: the present arrays densely packed, plus one
/// validity bit per sample. Row `i` is `None` for a null, else a borrow of
/// its array.
#[derive(Debug)]
pub struct OptionFixedSlice<'a, T, const N: usize> {
    inner: OptionSlice<'a, [T; N]>,
}

impl<'a, T, const N: usize> Clone for OptionFixedSlice<'a, T, N> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T, const N: usize> Copy for OptionFixedSlice<'a, T, N> {}

impl<'a, T, const N: usize> From<OptionSlice<'a, [T; N]>> for OptionFixedSlice<'a, T, N> {
    fn from(inner: OptionSlice<'a, [T; N]>) -> Self {
        Self { inner }
    }
}

impl<'a, T, const N: usize> OptionFixedSlice<'a, T, N> {
    /// A view over `values` (the present arrays, densely packed) and
    /// `validity` (one bit per sample), which must have one set bit per
    /// array.
    pub fn new(values: &'a [[T; N]], validity: &'a BitSlice<u8, Lsb0>) -> Result<Self, WireError> {
        Ok(Self {
            inner: OptionSlice::new(values, validity)?,
        })
    }

    /// Samples, nulls included.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Sample `i`: `None` when out of bounds, `Some(None)` for a null, else
    /// a borrow of its array. Costs a popcount over the validity bits before
    /// `i`; use [`Self::iter`] to walk every sample.
    pub fn get(&self, i: usize) -> Option<Option<&'a [T; N]>> {
        let validity = self.inner.validity();
        if !*validity.get(i)? {
            return Some(None);
        }
        let rank = validity[..i].count_ones();
        Some(Some(self.inner.values().get(rank)?))
    }

    /// Samples `[start, end)`. `None` when out of bounds.
    pub fn window(&self, start: usize, end: usize) -> Option<Self> {
        Some(Self {
            inner: self.inner.window(start, end)?,
        })
    }

    /// Only the present arrays, densely packed, in order.
    pub fn values(&self) -> &'a [[T; N]] {
        self.inner.values()
    }

    /// One bit per sample, nulls included (`true` = present).
    pub fn validity(&self) -> &'a BitSlice<u8, Lsb0> {
        self.inner.validity()
    }

    /// Number of nulls.
    pub fn null_count(&self) -> usize {
        self.inner.null_count()
    }

    pub fn iter(&self) -> OptionFixedSliceIter<'a, T, N> {
        OptionFixedSliceIter {
            validity: self.inner.validity(),
            values: self.inner.values(),
            next: 0,
            dense: 0,
        }
    }
}

/// Iterator over an [`OptionFixedSlice`]'s samples; carries a running cursor
/// into the dense arrays instead of popcounting per item.
pub struct OptionFixedSliceIter<'a, T, const N: usize> {
    validity: &'a BitSlice<u8, Lsb0>,
    values: &'a [[T; N]],
    next: usize,
    /// Index into the values of the next present array.
    dense: usize,
}

impl<'a, T, const N: usize> Iterator for OptionFixedSliceIter<'a, T, N> {
    type Item = Option<&'a [T; N]>;

    fn next(&mut self) -> Option<Self::Item> {
        let present = *self.validity.get(self.next)?;
        self.next += 1;
        if !present {
            return Some(None);
        }
        let array = self.values.get(self.dense);
        self.dense += 1;
        Some(array)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = self.validity.len() - self.next;
        (left, Some(left))
    }
}

impl<'a, T, const N: usize> ExactSizeIterator for OptionFixedSliceIter<'a, T, N> {}

impl<'a, T, const N: usize> IntoIterator for OptionFixedSlice<'a, T, N> {
    type Item = Option<&'a [T; N]>;
    type IntoIter = OptionFixedSliceIter<'a, T, N>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T, const N: usize> RowsRef for OptionFixedSlice<'a, T, N> {
    type Row = Option<&'a [T; N]>;

    fn len(&self) -> usize {
        Self::len(self)
    }

    fn get(&self, i: usize) -> Option<Option<&'a [T; N]>> {
        Self::get(self, i)
    }

    fn window(&self, start: usize, end: usize) -> Option<Self> {
        Self::window(self, start, end)
    }
}

impl<'a, T: Copy, const N: usize> Realize<Vec<Option<[T; N]>>> for OptionFixedSlice<'a, T, N> {
    fn realize(&self) -> Vec<Option<[T; N]>> {
        self.iter().map(|a| a.copied()).collect()
    }
}

impl<'a, 'b, T: PartialEq<U>, U, const N: usize> PartialEq<OptionFixedSlice<'b, U, N>>
    for OptionFixedSlice<'a, T, N>
{
    fn eq(&self, other: &OptionFixedSlice<'b, U, N>) -> bool {
        self.inner == other.inner
    }
}
