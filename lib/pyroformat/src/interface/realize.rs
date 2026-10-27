//! `Realize` and `RowsRef` impls for the borrowed view types in `std`/`bitvec`.
//! Trait definitions live in `super`.

use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;
use bitvec::vec::BitVec;

use super::{Realize, RowsRef};

macro_rules! impl_realize_scalar {
    ($($t:ty),* $(,)?) => {
        $(
            impl Realize<$t> for $t {
                #[inline]
                fn realize(&self) -> $t {
                    *self
                }
            }

            impl<'b> Realize<Vec<$t>> for &'b [$t] {
                fn realize(&self) -> Vec<$t> {
                    (*self).to_vec()
                }
            }

            impl<const N: usize> Realize<[$t; N]> for [$t; N] {
                #[inline]
                fn realize(&self) -> [$t; N] {
                    *self
                }
            }

            impl<'b, const N: usize> Realize<[$t; N]> for &'b [$t; N] {
                fn realize(&self) -> [$t; N] {
                    (*self).clone()
                }
            }

            impl<'b, const N: usize> Realize<Vec<[$t; N]>> for &'b [[$t; N]] {
                fn realize(&self) -> Vec<[$t; N]> {
                    (*self).to_vec()
                }
            }
        )*
    };
}

impl_realize_scalar!(
    bool,
    char,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    f32,
    f64,
    half::f16,
    crate::value::Time
);

impl<T: Realize<O>, O> Realize<Option<O>> for Option<T> {
    fn realize(&self) -> Option<O> {
        self.as_ref().map(Realize::realize)
    }
}

impl<T: Realize<O>, O> Realize<Vec<O>> for Vec<T> {
    fn realize(&self) -> Vec<O> {
        self.iter().map(Realize::realize).collect()
    }
}

impl<'b> Realize<String> for &'b str {
    fn realize(&self) -> String {
        (*self).to_string()
    }
}

// The owned blob types `wire::readers::basic` reads as `&[u8]`.
impl<'b> Realize<Box<[u8]>> for &'b [u8] {
    fn realize(&self) -> Box<[u8]> {
        Box::from(*self)
    }
}

impl<'b> Realize<bytes::Bytes> for &'b [u8] {
    fn realize(&self) -> bytes::Bytes {
        bytes::Bytes::copy_from_slice(self)
    }
}

impl<'b> Realize<bytes::BytesMut> for &'b [u8] {
    fn realize(&self) -> bytes::BytesMut {
        bytes::BytesMut::from(*self)
    }
}

impl<'b> Realize<BitVec<u8, Lsb0>> for &'b BitSlice<u8, Lsb0> {
    fn realize(&self) -> BitVec<u8, Lsb0> {
        (*self).to_bitvec()
    }
}

/// A packed bool run read back as a plain `Vec<bool>`, one bit per `bool`.
impl<'b> Realize<Vec<bool>> for &'b BitSlice<u8, Lsb0> {
    fn realize(&self) -> Vec<bool> {
        BitSlice::iter(self).by_vals().collect()
    }
}

impl<'b, T: Clone> RowsRef for &'b [T] {
    type Row = T;

    fn len(&self) -> usize {
        <[T]>::len(self)
    }

    fn get(&self, i: usize) -> Option<T> {
        <[T]>::get(self, i).cloned()
    }

    fn window(&self, start: usize, end: usize) -> Option<Self> {
        <[T]>::get(self, start..end)
    }
}

impl<'b> RowsRef for &'b BitSlice<u8, Lsb0> {
    type Row = bool;

    fn len(&self) -> usize {
        BitSlice::len(self)
    }

    fn get(&self, i: usize) -> Option<bool> {
        BitSlice::get(self, i).map(|b| *b)
    }

    fn window(&self, start: usize, end: usize) -> Option<Self> {
        BitSlice::get(self, start..end)
    }
}
