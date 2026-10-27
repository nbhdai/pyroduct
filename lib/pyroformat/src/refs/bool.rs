//! Required `bool` leaves: [`BoolNested`] for lists of `bool`, and
//! [`FixedBoolSlice`] / [`FixedBoolNested`] for `[bool; M]`.
//!
//! These are [`Nested`](super::Nested)'s shape over a bit-packed leaf: the
//! leaf run is a `BitSlice` rather than a `&[bool]`, so rows are
//! `&BitSlice<u8, Lsb0>` where `Nested` hands out `&[T]`. A bare
//! `Vec<bool>` has no topology and is the plain `&BitSlice<u8, Lsb0>`.

use bitvec::{order::Lsb0, slice::BitSlice, vec::BitVec};

use crate::PyroValue;
use crate::errors::WireError;
use crate::interface::{Realize, RowsRef};
use crate::refs::Topology;
use pyro_schema::StructureWord;

// =============================================================================
// BoolNested — `Vec<…<Vec<bool>>…>` over a bit-packed leaf
// =============================================================================

/// A zero-copy `Vec<…<Vec<bool>>…>`: [`Nested`](super::Nested)'s shape over a
/// bit-packed leaf.
///
/// | view                     | behaves like        | `get(i)` / iter item        |
/// |--------------------------|---------------------|-----------------------------|
/// | `BoolNested<W, 1>`    | `Vec<Vec<bool>>`    | `&'a BitSlice<u8, Lsb0>`    |
/// | `BoolNested<W, 2>`    | `Vec<Vec<Vec<bool>>>` | `BoolNested<W, 1>`     |
/// | …up to `N = 5`           |                     |                             |
///
/// `slice` holds exactly the logical bools, with the wire's byte padding
/// trimmed off.
///
/// ```
/// use pyroformat::refs::{BoolNested, Topology};
/// use pyroformat::schema::StructureWord;
/// const R: u32 = <u32 as StructureWord>::RESET;
/// use pyroformat::{BitSlice, Lsb0, Realize};
///
/// // [[true, false], [], [true]] packed LSB-first into one byte: 0b101.
/// let bytes = [0b0000_0101u8];
/// let rows = [R, 2, 2, 3];
/// let nest = BoolNested::new(
///     Topology { topology: [&rows[..]] },
///     &BitSlice::<u8, Lsb0>::from_slice(&bytes)[..3],
/// )
/// .unwrap();
///
/// assert_eq!(nest.len(), 3);
/// assert!(nest.get(1).unwrap().is_empty());
/// assert_eq!(nest.get(0).unwrap()[0], true);
/// let realized: Vec<Vec<bool>> = nest.realize();
/// assert_eq!(realized, vec![vec![true, false], vec![], vec![true]]);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct BoolNested<'a, W: StructureWord, const N: usize> {
    topology: Topology<'a, W, N>,
    slice: &'a BitSlice<u8, Lsb0>,
}

impl<'a, W: StructureWord, const N: usize> BoolNested<'a, W, N> {
    pub fn new(
        topology: Topology<'a, W, N>,
        slice: &'a BitSlice<u8, Lsb0>,
    ) -> Result<Self, WireError> {
        if topology.total() != slice.len() {
            return Err(WireError::InvalidTopologyLen {
                topo: topology.total(),
                data: slice.len(),
            });
        }
        Ok(Self { topology, slice })
    }

    /// Number of lists at this view's own (outermost) level.
    pub fn len(&self) -> usize {
        self.topology.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Elements `[start, end)`, same depth, `O(1)`. `None` when out of
    /// bounds.
    pub fn window(&self, start: usize, end: usize) -> Option<Self> {
        Some(BoolNested {
            topology: self.topology.window(start, end)?,
            slice: self.slice,
        })
    }

    pub fn iter(&self) -> BoolNestedIter<'a, W, N> {
        BoolNestedIter {
            nest: *self,
            front: 0,
            back: self.len(),
        }
    }
}

impl<'a, W: StructureWord> BoolNested<'a, W, 1> {
    /// Row `i` as its bits.
    pub fn get(&self, i: usize) -> Option<&'a BitSlice<u8, Lsb0>> {
        let (start, end) = self.topology.leaf_range(i)?;
        self.slice.get(start..end)
    }

    /// Alias for [`Self::get`].
    pub fn descend(&self, i: usize) -> Option<&'a BitSlice<u8, Lsb0>> {
        self.get(i)
    }
}

impl<'a, W: StructureWord> Realize<Vec<BitVec<u8, Lsb0>>> for BoolNested<'a, W, 1> {
    fn realize(&self) -> Vec<BitVec<u8, Lsb0>> {
        self.iter().map(|row| row.to_bitvec()).collect()
    }
}

impl<'a, W: StructureWord> Realize<Vec<Vec<bool>>> for BoolNested<'a, W, 1> {
    fn realize(&self) -> Vec<Vec<bool>> {
        self.iter()
            .map(|row| row.iter().by_vals().collect())
            .collect()
    }
}

macro_rules! bool_nest_level {
    ($n:literal => $m:literal) => {
        impl<'a, W: StructureWord> BoolNested<'a, W, $n> {
            /// Element `i`, one level down.
            pub fn get(&self, i: usize) -> Option<BoolNested<'a, W, $m>> {
                Some(BoolNested {
                    topology: self.topology.descend(i)?,
                    slice: self.slice,
                })
            }

            /// Alias for [`Self::get`].
            pub fn descend(&self, i: usize) -> Option<BoolNested<'a, W, $m>> {
                self.get(i)
            }
        }

        impl<'a, W: StructureWord, O> Realize<Vec<O>> for BoolNested<'a, W, $n>
        where
            BoolNested<'a, W, $m>: Realize<O>,
        {
            fn realize(&self) -> Vec<O> {
                self.iter().map(|child| child.realize()).collect()
            }
        }
    };
}

bool_nest_level!(2 => 1);
bool_nest_level!(3 => 2);
bool_nest_level!(4 => 3);
bool_nest_level!(5 => 4);

/// Iterator over a [`BoolNested`]'s outermost level; yields what
/// `get(i)` would, front to back (or back to front).
pub struct BoolNestedIter<'a, W: StructureWord, const N: usize> {
    nest: BoolNested<'a, W, N>,
    front: usize,
    back: usize,
}

macro_rules! bool_nest_iter {
    ($n:literal => $item:ty) => {
        impl<'a, W: StructureWord> Iterator for BoolNestedIter<'a, W, $n> {
            type Item = $item;

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

        impl<'a, W: StructureWord> DoubleEndedIterator for BoolNestedIter<'a, W, $n> {
            fn next_back(&mut self) -> Option<Self::Item> {
                if self.front >= self.back {
                    return None;
                }
                self.back -= 1;
                self.nest.get(self.back)
            }
        }

        impl<'a, W: StructureWord> ExactSizeIterator for BoolNestedIter<'a, W, $n> {}

        impl<'a, W: StructureWord> IntoIterator for BoolNested<'a, W, $n> {
            type Item = $item;
            type IntoIter = BoolNestedIter<'a, W, $n>;

            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
    };
}

bool_nest_iter!(1 => &'a BitSlice<u8, Lsb0>);
bool_nest_iter!(2 => BoolNested<'a, W, 1>);
bool_nest_iter!(3 => BoolNested<'a, W, 2>);
bool_nest_iter!(4 => BoolNested<'a, W, 3>);
bool_nest_iter!(5 => BoolNested<'a, W, 4>);

// =============================================================================
// BoolNested equality — compares like the `Vec<…<Vec<bool>>…>` it
// stands for, against another view or against `Vec`s (both directions)
// =============================================================================

/// One row against an owned `[bool]`, bit by bit.
fn bits_eq(bits: &BitSlice<u8, Lsb0>, other: &[bool]) -> bool {
    bits.len() == other.len() && bits.iter().by_vals().zip(other).all(|(a, b)| a == *b)
}

impl<'a, 'b, W: StructureWord> PartialEq<BoolNested<'b, W, 1>> for BoolNested<'a, W, 1> {
    fn eq(&self, other: &BoolNested<'b, W, 1>) -> bool {
        self.len() == other.len() && self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}

impl<'a, W: StructureWord> Eq for BoolNested<'a, W, 1> {}

impl<'a, W: StructureWord> PartialEq<[Vec<bool>]> for BoolNested<'a, W, 1> {
    fn eq(&self, other: &[Vec<bool>]) -> bool {
        self.len() == other.len() && self.iter().zip(other).all(|(a, b)| bits_eq(a, b))
    }
}

impl<'a, W: StructureWord> PartialEq<Vec<Vec<bool>>> for BoolNested<'a, W, 1> {
    fn eq(&self, other: &Vec<Vec<bool>>) -> bool {
        *self == *other.as_slice()
    }
}

impl<'a, W: StructureWord> PartialEq<BoolNested<'a, W, 1>> for Vec<Vec<bool>> {
    fn eq(&self, other: &BoolNested<'a, W, 1>) -> bool {
        *other == *self.as_slice()
    }
}

macro_rules! bool_nest_eq {
    ($($n:literal => $m:literal),* $(,)?) => {
        $(
            impl<'a, 'b, W: StructureWord> PartialEq<BoolNested<'b, W, $n>> for BoolNested<'a, W, $n> {
                fn eq(&self, other: &BoolNested<'b, W, $n>) -> bool {
                    self.len() == other.len()
                        && (0..self.len()).all(|i| self.descend(i) == other.descend(i))
                }
            }

            impl<'a, W: StructureWord> Eq for BoolNested<'a, W, $n> {}

            impl<'a, W: StructureWord, V> PartialEq<[V]> for BoolNested<'a, W, $n>
            where
                BoolNested<'a, W, $m>: PartialEq<V>,
            {
                fn eq(&self, other: &[V]) -> bool {
                    self.len() == other.len()
                        && other
                            .iter()
                            .enumerate()
                            .all(|(i, b)| self.descend(i).is_some_and(|a| a == *b))
                }
            }

            impl<'a, W: StructureWord, V> PartialEq<Vec<V>> for BoolNested<'a, W, $n>
            where
                BoolNested<'a, W, $m>: PartialEq<V>,
            {
                fn eq(&self, other: &Vec<V>) -> bool {
                    *self == *other.as_slice()
                }
            }

            impl<'a, W: StructureWord, V> PartialEq<BoolNested<'a, W, $n>> for Vec<V>
            where
                BoolNested<'a, W, $m>: PartialEq<V>,
            {
                fn eq(&self, other: &BoolNested<'a, W, $n>) -> bool {
                    *other == *self.as_slice()
                }
            }
        )*
    };
}

bool_nest_eq!(2 => 1, 3 => 2, 4 => 3, 5 => 4);

macro_rules! bool_nest_rows_ref {
    ($($n:literal => $row:ty),* $(,)?) => {
        $(
            impl<'a, W: StructureWord> RowsRef for BoolNested<'a, W, $n> {
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

bool_nest_rows_ref!(
    1 => &'a BitSlice<u8, Lsb0>,
    2 => BoolNested<'a, W, 1>,
    3 => BoolNested<'a, W, 2>,
    4 => BoolNested<'a, W, 3>,
    5 => BoolNested<'a, W, 4>,
);

impl<'a, W: StructureWord, const N: usize> BoolNested<'a, W, N> {
    /// The elements of this view's outermost level as nested
    /// `PyroValue::List`s, each innermost list a borrowed bool
    /// `PrimitiveList`.
    ///
    /// `levels` is how many of the view's `N` levels the data uses
    /// (`1 <= levels <= N`): a view read at [`crate::MAX_TIERS`] levels for a
    /// schema only known at runtime uses fewer.
    pub fn into_value(&self, levels: usize) -> Vec<PyroValue<'a>> {
        let slice = self.slice;
        self.topology.fold_rows(levels, |leaf| {
            BoolNested::<'a, W, 1> {
                topology: leaf,
                slice,
            }
            .iter()
            .map(PyroValue::from)
        })
    }
}

// =============================================================================
// FixedBoolSlice — `Vec<[bool; M]>` as one bit-packed run
// =============================================================================

/// Bits `[start * M, end * M)` of `slice`: arrays `[start, end)`.
fn array_bits<const M: usize>(
    slice: &BitSlice<u8, Lsb0>,
    start: usize,
    end: usize,
) -> Option<&BitSlice<u8, Lsb0>> {
    slice.get(start.checked_mul(M)?..end.checked_mul(M)?)
}

/// Array `i` of a packed run, by value.
fn array_at<const M: usize>(slice: &BitSlice<u8, Lsb0>, i: usize) -> Option<[bool; M]> {
    let bits = array_bits::<M>(slice, i, i + 1)?;
    Some(std::array::from_fn(|j| bits[j]))
}

/// A zero-copy `Vec<[bool; M]>`: every array's `M` bits back to back. No
/// topology, so no word type.
///
/// `count` is the number of *arrays*; `slice` may run past `count * M` bits
/// (the wire pads it to whole bytes), and that padding is never exposed.
/// Rows are `[bool; M]` by value: a `BitArray` of exactly `M` bits needs
/// `M.div_ceil(8)` bytes of storage, which stable Rust can't name for a
/// generic `M`. [`Self::get_bits`] borrows the bits instead.
///
/// ```
/// use pyroformat::refs::FixedBoolSlice;
/// use pyroformat::{BitSlice, Lsb0, Realize};
///
/// // [[true, false], [false, true]], two bits per array: 01 10.
/// let bytes = [0b0000_1001u8];
/// let arrays = FixedBoolSlice::<2> {
///     slice: BitSlice::<u8, Lsb0>::from_slice(&bytes),
///     count: 2,
/// };
/// assert_eq!(arrays.get(1), Some([false, true]));
/// assert_eq!(arrays.realize(), vec![[true, false], [false, true]]);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct FixedBoolSlice<'a, const M: usize> {
    pub slice: &'a BitSlice<u8, Lsb0>,
    pub count: usize,
}

impl<'a, const M: usize> FixedBoolSlice<'a, M> {
    /// Number of arrays.
    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Array `i`. `None` when out of bounds.
    pub fn get(&self, i: usize) -> Option<[bool; M]> {
        if i < self.count {
            array_at::<M>(self.slice, i)
        } else {
            None
        }
    }

    /// Array `i` as its `M` bits, without copying them out.
    pub fn get_bits(&self, i: usize) -> Option<&'a BitSlice<u8, Lsb0>> {
        if i < self.count {
            array_bits::<M>(self.slice, i, i + 1)
        } else {
            None
        }
    }

    /// Every array's bits back to back, with the wire's byte padding
    /// trimmed off.
    pub fn as_bits(&self) -> &'a BitSlice<u8, Lsb0> {
        &self.slice[..self.count.saturating_mul(M).min(self.slice.len())]
    }

    /// Arrays `[start, end)`, `O(1)`. `None` when out of bounds.
    pub fn window(&self, start: usize, end: usize) -> Option<Self> {
        if start > end || end > self.count {
            return None;
        }
        Some(FixedBoolSlice {
            slice: array_bits::<M>(self.slice, start, end)?,
            count: end - start,
        })
    }

    pub fn iter(&self) -> FixedBoolSliceIter<'a, M> {
        FixedBoolSliceIter {
            slice: *self,
            front: 0,
            back: self.len(),
        }
    }
}

/// Iterator over a [`FixedBoolSlice`]'s arrays, front to back (or back to
/// front).
pub struct FixedBoolSliceIter<'a, const M: usize> {
    slice: FixedBoolSlice<'a, M>,
    front: usize,
    back: usize,
}

impl<'a, const M: usize> Iterator for FixedBoolSliceIter<'a, M> {
    type Item = [bool; M];

    fn next(&mut self) -> Option<Self::Item> {
        if self.front >= self.back {
            return None;
        }
        let i = self.front;
        self.front += 1;
        self.slice.get(i)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.back - self.front;
        (n, Some(n))
    }
}

impl<'a, const M: usize> DoubleEndedIterator for FixedBoolSliceIter<'a, M> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.front >= self.back {
            return None;
        }
        self.back -= 1;
        self.slice.get(self.back)
    }
}

impl<'a, const M: usize> ExactSizeIterator for FixedBoolSliceIter<'a, M> {}

impl<'a, const M: usize> IntoIterator for FixedBoolSlice<'a, M> {
    type Item = [bool; M];
    type IntoIter = FixedBoolSliceIter<'a, M>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, 'n, const M: usize> IntoIterator for &'n FixedBoolSlice<'a, M> {
    type Item = [bool; M];
    type IntoIter = FixedBoolSliceIter<'a, M>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, const M: usize> RowsRef for FixedBoolSlice<'a, M> {
    type Row = [bool; M];

    fn len(&self) -> usize {
        Self::len(self)
    }

    fn get(&self, i: usize) -> Option<[bool; M]> {
        Self::get(self, i)
    }

    fn window(&self, start: usize, end: usize) -> Option<Self> {
        Self::window(self, start, end)
    }
}

impl<'a, const M: usize> FixedBoolSlice<'a, M> {
    /// The arrays as a `PyroValue::List`, each array a bool `PrimitiveList`.
    pub fn into_value(&self) -> PyroValue<'a> {
        PyroValue::List(
            self.iter()
                .map(|array| -> PyroValue<'a> { array.into() })
                .collect(),
        )
    }
}

impl<'a, const M: usize> Realize<Vec<[bool; M]>> for FixedBoolSlice<'a, M> {
    fn realize(&self) -> Vec<[bool; M]> {
        self.iter().collect()
    }
}

impl<'a, 'b, const M: usize> PartialEq<FixedBoolSlice<'b, M>> for FixedBoolSlice<'a, M> {
    fn eq(&self, other: &FixedBoolSlice<'b, M>) -> bool {
        self.count == other.count && self.as_bits() == other.as_bits()
    }
}

impl<'a, const M: usize> Eq for FixedBoolSlice<'a, M> {}

impl<'a, const M: usize> PartialEq<[[bool; M]]> for FixedBoolSlice<'a, M> {
    fn eq(&self, other: &[[bool; M]]) -> bool {
        self.len() == other.len() && self.iter().zip(other).all(|(a, b)| a == *b)
    }
}

impl<'a, const M: usize> PartialEq<Vec<[bool; M]>> for FixedBoolSlice<'a, M> {
    fn eq(&self, other: &Vec<[bool; M]>) -> bool {
        *self == *other.as_slice()
    }
}

impl<'a, const M: usize> PartialEq<FixedBoolSlice<'a, M>> for Vec<[bool; M]> {
    fn eq(&self, other: &FixedBoolSlice<'a, M>) -> bool {
        *other == *self.as_slice()
    }
}

// =============================================================================
// FixedBoolNested — `Vec<…<Vec<[bool; M]>>…>` over a bit-packed leaf
// =============================================================================

/// A zero-copy `Vec<…<Vec<[bool; M]>>…>`: [`BoolNested`]'s shape,
/// with every leaf element `M` bits wide.
///
/// | view                           | behaves like              | `get(i)` / iter item          |
/// |--------------------------------|---------------------------|-------------------------------|
/// | `FixedBoolNested<W, M, 1>`  | `Vec<Vec<[bool; M]>>`     | `FixedBoolSlice<M>`        |
/// | `FixedBoolNested<W, M, 2>`  | `Vec<Vec<Vec<[bool; M]>>>` | `FixedBoolNested<W, M, 1>` |
/// | …up to `N = 5`                 |                           |                               |
///
/// `count` is the number of *arrays* in the whole leaf run; the topology,
/// which counts arrays, decides every length, and `count` is carried along
/// unchanged. A bare `Vec<[bool; M]>` is a [`FixedBoolSlice`].
///
/// ```
/// use pyroformat::refs::{FixedBoolNested, Topology};
/// use pyroformat::schema::StructureWord;
/// const R: u32 = <u32 as StructureWord>::RESET;
/// use pyroformat::{BitSlice, Lsb0, Realize};
///
/// // [[[true, false], [false, true]], [], [[true, true]]], two bits per
/// // array, packed LSB-first: 01 10 11.
/// let bytes = [0b0011_1001u8];
/// let rows = [R, 2, 2, 3];
/// let nest = FixedBoolNested::<u32, 2, 1> {
///     topology: Topology { topology: [&rows[..]] },
///     slice: BitSlice::<u8, Lsb0>::from_slice(&bytes),
///     count: 3,
/// };
///
/// assert_eq!(nest.len(), 3);
/// assert!(nest.get(1).unwrap().is_empty());
/// assert_eq!(nest.get(0).unwrap().get(1), Some([false, true]));
/// assert_eq!(
///     nest.realize(),
///     vec![vec![[true, false], [false, true]], vec![], vec![[true, true]]]
/// );
/// ```
#[derive(Debug, Clone, Copy)]
pub struct FixedBoolNested<'a, W: StructureWord, const M: usize, const N: usize> {
    pub topology: Topology<'a, W, N>,
    pub slice: &'a BitSlice<u8, Lsb0>,
    pub count: usize,
}

impl<'a, W: StructureWord, const M: usize, const N: usize> FixedBoolNested<'a, W, M, N> {
    /// Number of lists at this view's own (outermost) level.
    pub fn len(&self) -> usize {
        self.topology.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Lists `[start, end)`, same depth, `O(1)`. `None` when out of bounds.
    pub fn window(&self, start: usize, end: usize) -> Option<Self> {
        Some(FixedBoolNested {
            topology: self.topology.window(start, end)?,
            slice: self.slice,
            count: self.count,
        })
    }

    pub fn iter(&self) -> FixedBoolNestedIter<'a, W, M, N> {
        FixedBoolNestedIter {
            nest: *self,
            front: 0,
            back: self.len(),
        }
    }
}

impl<'a, W: StructureWord, const M: usize> FixedBoolNested<'a, W, M, 1> {
    /// List `i` as a [`FixedBoolSlice`]. `None` when out of bounds.
    pub fn get(&self, i: usize) -> Option<FixedBoolSlice<'a, M>> {
        let (start, end) = self.topology.leaf_range(i)?;
        Some(FixedBoolSlice {
            slice: array_bits::<M>(self.slice, start, end)?,
            count: end - start,
        })
    }

    /// Alias for [`Self::get`].
    pub fn descend(&self, i: usize) -> Option<FixedBoolSlice<'a, M>> {
        self.get(i)
    }
}

impl<'a, W: StructureWord, const M: usize, const N: usize> FixedBoolNested<'a, W, M, N> {
    /// The elements of this view's outermost level as nested
    /// `PyroValue::List`s down to the arrays (see
    /// [`FixedBoolSlice::into_value`]).
    ///
    /// `levels` is how many of the view's `N` levels the data uses
    /// (`1 <= levels <= N`): a view read at [`crate::MAX_TIERS`] levels for a
    /// schema only known at runtime uses fewer.
    pub fn into_value(&self, levels: usize) -> Vec<PyroValue<'a>> {
        let (slice, count) = (self.slice, self.count);
        self.topology.fold_rows(levels, |leaf| {
            FixedBoolNested::<'a, W, M, 1> {
                topology: leaf,
                slice,
                count,
            }
            .iter()
            .map(|row| row.into_value())
        })
    }
}

impl<'a, W: StructureWord, const M: usize> Realize<Vec<Vec<[bool; M]>>>
    for FixedBoolNested<'a, W, M, 1>
{
    fn realize(&self) -> Vec<Vec<[bool; M]>> {
        self.iter().map(|row| row.realize()).collect()
    }
}

macro_rules! fixed_bool_nest_level {
    ($n:literal => $m:literal) => {
        impl<'a, W: StructureWord, const M: usize> FixedBoolNested<'a, W, M, $n> {
            /// Element `i`, one level down, `O(1)`.
            pub fn get(&self, i: usize) -> Option<FixedBoolNested<'a, W, M, $m>> {
                Some(FixedBoolNested {
                    topology: self.topology.descend(i)?,
                    slice: self.slice,
                    count: self.count,
                })
            }

            /// Alias for [`Self::get`].
            pub fn descend(&self, i: usize) -> Option<FixedBoolNested<'a, W, M, $m>> {
                self.get(i)
            }
        }

        impl<'a, W: StructureWord, const M: usize, O> Realize<Vec<O>>
            for FixedBoolNested<'a, W, M, $n>
        where
            FixedBoolNested<'a, W, M, $m>: Realize<O>,
        {
            fn realize(&self) -> Vec<O> {
                self.iter().map(|child| child.realize()).collect()
            }
        }
    };
}

fixed_bool_nest_level!(2 => 1);
fixed_bool_nest_level!(3 => 2);
fixed_bool_nest_level!(4 => 3);
fixed_bool_nest_level!(5 => 4);

/// Iterator over a [`FixedBoolNested`]'s outermost level; yields
/// what `get(i)` would, front to back (or back to front).
pub struct FixedBoolNestedIter<'a, W: StructureWord, const M: usize, const N: usize> {
    nest: FixedBoolNested<'a, W, M, N>,
    front: usize,
    back: usize,
}

macro_rules! fixed_bool_nest_iter {
    ($($n:literal => $item:ty),* $(,)?) => {
        $(
            impl<'a, W: StructureWord, const M: usize> Iterator for FixedBoolNestedIter<'a, W, M, $n> {
                type Item = $item;

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

            impl<'a, W: StructureWord, const M: usize> DoubleEndedIterator for FixedBoolNestedIter<'a, W, M, $n> {
                fn next_back(&mut self) -> Option<Self::Item> {
                    if self.front >= self.back {
                        return None;
                    }
                    self.back -= 1;
                    self.nest.get(self.back)
                }
            }

            impl<'a, W: StructureWord, const M: usize> ExactSizeIterator for FixedBoolNestedIter<'a, W, M, $n> {}

            impl<'a, W: StructureWord, const M: usize> IntoIterator for FixedBoolNested<'a, W, M, $n> {
                type Item = $item;
                type IntoIter = FixedBoolNestedIter<'a, W, M, $n>;

                fn into_iter(self) -> Self::IntoIter {
                    self.iter()
                }
            }

            impl<'a, 'n, W: StructureWord, const M: usize> IntoIterator for &'n FixedBoolNested<'a, W, M, $n> {
                type Item = $item;
                type IntoIter = FixedBoolNestedIter<'a, W, M, $n>;

                fn into_iter(self) -> Self::IntoIter {
                    self.iter()
                }
            }

            impl<'a, W: StructureWord, const M: usize> RowsRef for FixedBoolNested<'a, W, M, $n> {
                type Row = $item;

                fn len(&self) -> usize {
                    Self::len(self)
                }

                fn get(&self, i: usize) -> Option<$item> {
                    Self::get(self, i)
                }

                fn window(&self, start: usize, end: usize) -> Option<Self> {
                    Self::window(self, start, end)
                }
            }
        )*
    };
}

fixed_bool_nest_iter!(
    1 => FixedBoolSlice<'a, M>,
    2 => FixedBoolNested<'a, W, M, 1>,
    3 => FixedBoolNested<'a, W, M, 2>,
    4 => FixedBoolNested<'a, W, M, 3>,
    5 => FixedBoolNested<'a, W, M, 4>,
);

// =============================================================================
// FixedBoolNested equality — compares like the
// `Vec<…<Vec<[bool; M]>>…>` it stands for, against another view or against
// `Vec`s (both directions)
// =============================================================================

macro_rules! fixed_bool_nest_eq {
    ($($n:literal => $child:ty),* $(,)?) => {
        $(
            impl<'a, 'b, W: StructureWord, const M: usize> PartialEq<FixedBoolNested<'b, W, M, $n>>
                for FixedBoolNested<'a, W, M, $n>
            {
                fn eq(&self, other: &FixedBoolNested<'b, W, M, $n>) -> bool {
                    self.len() == other.len()
                        && self.iter().zip(other.iter()).all(|(a, b)| a == b)
                }
            }

            impl<'a, W: StructureWord, const M: usize> Eq for FixedBoolNested<'a, W, M, $n> {}

            impl<'a, W: StructureWord, V, const M: usize> PartialEq<[V]> for FixedBoolNested<'a, W, M, $n>
            where
                $child: PartialEq<V>,
            {
                fn eq(&self, other: &[V]) -> bool {
                    self.len() == other.len() && self.iter().zip(other).all(|(a, b)| a == *b)
                }
            }

            impl<'a, W: StructureWord, V, const M: usize> PartialEq<Vec<V>> for FixedBoolNested<'a, W, M, $n>
            where
                $child: PartialEq<V>,
            {
                fn eq(&self, other: &Vec<V>) -> bool {
                    *self == *other.as_slice()
                }
            }

            impl<'a, W: StructureWord, V, const M: usize> PartialEq<FixedBoolNested<'a, W, M, $n>> for Vec<V>
            where
                $child: PartialEq<V>,
            {
                fn eq(&self, other: &FixedBoolNested<'a, W, M, $n>) -> bool {
                    *other == *self.as_slice()
                }
            }
        )*
    };
}

fixed_bool_nest_eq!(
    1 => FixedBoolSlice<'a, M>,
    2 => FixedBoolNested<'a, W, M, 1>,
    3 => FixedBoolNested<'a, W, M, 2>,
    4 => FixedBoolNested<'a, W, M, 3>,
    5 => FixedBoolNested<'a, W, M, 4>,
);

#[cfg(test)]
mod tests {
    use super::*;
    use pyro_schema::StructureWord;

    const R: u32 = <u32 as StructureWord>::RESET;

    #[test]
    fn bool_nest_levels() {
        // [[[true], []], [], [[false, true]]]
        let l0 = [R, 2, 2, 3];
        let l1 = [R, 1, 1, 3];
        let leaf = [0b0000_0101u8];
        let nest: BoolNested<'_, u32, 2> = BoolNested {
            topology: Topology {
                topology: [&l0, &l1],
            },
            slice: &BitSlice::from_slice(&leaf)[..3],
        };
        let owned = vec![vec![vec![true], vec![]], vec![], vec![vec![false, true]]];
        assert_eq!(Realize::<Vec<Vec<Vec<bool>>>>::realize(&nest), owned);
        assert_eq!(
            Realize::<Vec<Vec<Vec<bool>>>>::realize(&nest.window(2, 3).unwrap()),
            owned[2..].to_vec()
        );
        assert_eq!(
            nest.iter()
                .rev()
                .next()
                .unwrap()
                .descend(0)
                .unwrap()
                .get(1)
                .as_deref(),
            Some(&true)
        );
    }

    #[test]
    fn bool_nest_equality() {
        let l0 = [R, 2, 2, 3];
        let l1 = [R, 1, 1, 3];
        let leaf = [0b0000_0101u8];
        let nest: BoolNested<'_, u32, 2> = BoolNested {
            topology: Topology {
                topology: [&l0, &l1],
            },
            slice: &BitSlice::from_slice(&leaf)[..3],
        };
        let owned = vec![vec![vec![true], vec![]], vec![], vec![vec![false, true]]];
        assert_eq!(nest, owned);
        assert_eq!(owned, nest);
        assert_eq!(nest, nest);
        assert_ne!(nest.get(0).unwrap(), nest.get(2).unwrap());
        assert_eq!(nest.get(2).unwrap(), vec![vec![false, true]]);
    }

    fn pack(pattern: &[bool]) -> Vec<u8> {
        let mut out = vec![0u8; pattern.len().div_ceil(8)];
        for (i, &b) in pattern.iter().enumerate() {
            out[i / 8] |= (b as u8) << (i % 8);
        }
        out
    }

    // [[[[T, F, T]], [[F, F, T], [T, T, T]]], [], [[]]] — arrays of 3 bits.
    fn fixed_bool_owned() -> Vec<Vec<Vec<[bool; 3]>>> {
        vec![
            vec![
                vec![[true, false, true]],
                vec![[false, false, true], [true, true, true]],
            ],
            vec![],
            vec![vec![]],
        ]
    }

    #[test]
    fn fixed_bool_nest_behaves_like_nested_vecs_of_arrays() {
        let owned = fixed_bool_owned();
        let flat: Vec<bool> = owned
            .iter()
            .flatten()
            .flatten()
            .flatten()
            .copied()
            .collect();
        let bytes = pack(&flat);
        let l0 = [R, 2, 2, 3];
        let l1 = [R, 1, 3, 3];
        let nest: FixedBoolNested<'_, u32, 3, 2> = FixedBoolNested {
            topology: Topology {
                topology: [&l0, &l1],
            },
            slice: BitSlice::from_slice(&bytes),
            count: 3,
        };

        assert_eq!(nest.len(), 3);
        assert_eq!(nest, owned);
        assert_eq!(owned, nest);
        assert_eq!(nest.realize(), owned);
        assert_eq!(nest, nest);
        assert!(nest.get(3).is_none());
        assert_eq!(nest.iter().len(), 3);
        assert_eq!(nest.iter().rev().next().unwrap(), owned[2]);
        assert_eq!(nest.window(1, 3).unwrap(), owned[1..].to_vec());

        let second = nest.get(0).unwrap().get(1).unwrap();
        assert_eq!(second, owned[0][1]);
        assert_eq!(second.len(), 2);
        assert_eq!(second.get(1), Some([true, true, true]));
        assert!(second.get(2).is_none());
        assert_eq!(second.get_bits(0).unwrap().len(), 3);
        assert_eq!(second.as_bits().len(), 6);
        assert_eq!(second.window(1, 2).unwrap(), vec![[true, true, true]]);
        assert!(second.window(1, 3).is_none());
        assert_eq!(
            second.iter().rev().collect::<Vec<_>>(),
            [[true, true, true], [false, false, true]]
        );
        assert_ne!(second, nest.get(0).unwrap().get(0).unwrap());

        // `RowsRef` forwards to the inherent `get`.
        assert_eq!(RowsRef::get(&second, 0), second.get(0));
        let rows: Vec<Vec<[bool; 3]>> = RowsRef::iter(&nest.get(0).unwrap())
            .map(|row| row.realize())
            .collect();
        assert_eq!(rows, owned[0]);
    }

    #[test]
    fn fixed_bool_nest_padding_is_not_data() {
        // Two 3-bit arrays in one byte; the top two bits are padding.
        let bytes = [0b1111_0101u8];
        let nest = FixedBoolSlice::<3> {
            slice: BitSlice::from_slice(&bytes),
            count: 2,
        };
        assert_eq!(nest, vec![[true, false, true], [false, true, true]]);
        assert_eq!(nest.as_bits().len(), 6);
    }
}
