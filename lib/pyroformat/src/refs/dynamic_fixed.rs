//! `List<[T; n]>` / `List<Option<[T; n]>>` value-path leaves, where `n`
//! (elements per array) is only known at runtime — from the schema's
//! `fixed_size` — unlike [`super::primitive`]'s `Fixed{Variant}Nested`
//! aliases (`Nested<'a, W, [T; M], N>`), whose `M` is a compile-time
//! const generic coming from the derived Rust type. Hence "dynamic": same
//! shape, runtime stride instead of a const one.
//!
//! # Why no `Topology` here at all
//!
//! Every other nested leaf in this crate (`primitive::Nested`,
//! `primitive_option::OptionNested`) pairs its dense payload with a
//! [`Topology`](crate::refs::Topology) level, because their atomic
//! unit (a scalar `T`) is smaller than a topology boundary: several scalars
//! can share one leaf index, so `Topology::split` peels the deepest level
//! off to merge them into one `&'a [T]` chunk per leaf before `eat_values`
//! sees them (see `wire::value_readers::slices`'s module-level reasoning).
//!
//! A fixed-size array has no such merging to do: every wire array is
//! exactly `n` elements, always, so "one array" *is* the atomic unit a
//! `List<[T; n]>`'s own topology level already counts — chunking `n`
//! elements at a time reproduces exactly what a topology boundary would
//! have said anyway. That means the *whole, unsplit* multi-level
//! `Topology` from `wire::value_readers::fixed_nested`/`fixed_nested_option`
//! can be fed `[Self::iter]`'s output directly: at whatever depth its
//! deepest populated level turns out to be terminal, it pulls exactly the
//! right number of arrays off this iterator, one `PyroValue::PrimitiveList`
//! (or `PyroValue::Null`) per array, no splitting required.

use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;

use crate::PyroValue;

// =============================================================================
// FixedSlice — required `[T; n]` arrays, flat and dense
// =============================================================================

/// A flat run of `[T; n]` arrays, packed back to back (`slice.len() ==
/// array_count * n`). `iter` hands out one whole array at a time.
#[derive(Debug, Clone, Copy)]
pub struct FixedSlice<'a, T> {
    pub slice: &'a [T],
    /// Elements per array.
    pub n: usize,
}

impl<'a, T: Copy> FixedSlice<'a, T> {
    pub fn iter(&self) -> std::slice::ChunksExact<'a, T> {
        self.slice.chunks_exact(self.n)
    }
}

impl<'a, T: Copy> FixedSlice<'a, T>
where
    PyroValue<'a>: From<&'a [T]>,
{
    /// The arrays as a `PyroValue::List`, each array a borrowed
    /// `PrimitiveList`.
    pub fn into_value(&self) -> PyroValue<'a> {
        PyroValue::List(self.iter().map(PyroValue::from).collect())
    }
}

// =============================================================================
// OptionFixedSlice — nullable twin: dense arrays + a validity mask
// (one bit per array, nulls included)
// =============================================================================

/// [`FixedSlice`]'s nullable twin: `dense` holds only the *valid*
/// arrays, packed back to back; `option` has one bit per array, nulls
/// included, so `option.len()` is the total (valid + null) array count.
#[derive(Debug, Clone, Copy)]
pub struct OptionFixedSlice<'a, T> {
    pub dense: &'a [T],
    /// Elements per array.
    pub n: usize,
    pub option: &'a BitSlice<u8, Lsb0>,
}

impl<'a, T: Copy> OptionFixedSlice<'a, T>
where
    PyroValue<'a>: From<&'a [T]>,
{
    /// The samples as a `PyroValue::List`: each `Some` array a borrowed
    /// `PrimitiveList`, each `None` as `PyroValue::Null`.
    pub fn into_value(&self) -> PyroValue<'a> {
        PyroValue::List(
            self.iter()
                .map(|v| v.map_or(PyroValue::Null, PyroValue::from))
                .collect(),
        )
    }
}

impl<'a, T: Copy> OptionFixedSlice<'a, T> {
    pub fn iter(&self) -> OptionFixedSliceIter<'a, T> {
        OptionFixedSliceIter {
            dense: self.dense,
            n: self.n,
            option: self.option,
            opt_cursor: 0,
            dense_cursor: 0,
        }
    }
}

pub struct OptionFixedSliceIter<'a, T> {
    dense: &'a [T],
    n: usize,
    option: &'a BitSlice<u8, Lsb0>,
    opt_cursor: usize,
    dense_cursor: usize,
}

impl<'a, T: Copy> Iterator for OptionFixedSliceIter<'a, T> {
    type Item = Option<&'a [T]>;

    #[cfg_attr(
        feature = "trace-logging",
        tracing::instrument(level = "trace", skip_all)
    )]
    fn next(&mut self) -> Option<Self::Item> {
        if self.opt_cursor < self.option.len() {
            let result = if self.option[self.opt_cursor] {
                let start = self.dense_cursor * self.n;
                let val = Some(&self.dense[start..start + self.n]);
                self.dense_cursor += 1;
                val
            } else {
                None
            };
            self.opt_cursor += 1;
            Some(result)
        } else {
            None
        }
    }
}
