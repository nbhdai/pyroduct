use std::marker::PhantomData;

use bitvec::{order::Lsb0, slice::BitSlice};
use half::f16;

use crate::{
    Layout,
    buf::PyroView,
    errors::WireError,
    offsets::InodeOffset,
    refs::{
        BoolNested, FixedBoolNested, FixedBoolSlice, Nested, OptionBoolNested, OptionBoolSlice,
        OptionFixedBoolNested, OptionFixedBoolSlice, OptionNested, OptionSlice, StrNested,
        Topology, check_str,
    },
    schema::{StructureWord, WireKind},
    value::Time,
};

pub struct SplitSource<'a, Inodes: Layout, const FIXED: usize> {
    fixed: &'a [u8; FIXED],
    variable: &'a [u8],
    phantom: PhantomData<Inodes>,
}

// Manual so `Inodes` needn't be `Clone`/`Copy` — only references are copied.
impl<Inodes: Layout, const FIXED: usize> Clone for SplitSource<'_, Inodes, FIXED> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Inodes: Layout, const FIXED: usize> Copy for SplitSource<'_, Inodes, FIXED> {}

/// Every scalar's `read_*`, run's `read_*_slice` and nest's `read_*_nest`,
/// for one fixed-width primitive: plain LE numerics (or a newtype over one),
/// valid at every bit pattern. `$from` decodes the scalar's LE bytes. `O` is
/// the inode offset; a nest's `M` its topology levels.
macro_rules! primitive_reads {
    ($($read:ident, $slice:ident, $nest:ident, $option_slice:ident, $option_nest:ident: $ty:ty = $from:expr;)*) => {
        $(
            /// Inode `O::N`'s nullable run (`Option*Slice`).
            #[inline]
            pub fn $option_slice<O: InodeOffset>(
                &self,
            ) -> Result<OptionSlice<'a, $ty>, WireError> {
                let data_offset: usize = Inodes::INODES[O::N].data_offset() as usize;
                let (slice, option) = self.read_option_cast::<$ty>(data_offset, 1)?;
                OptionSlice::new(slice, option)
            }

            /// Inode `O::N`'s nullable nest, `M` levels (`Option*Nested<_, M>`).
            #[inline]
            pub fn $option_nest<O: InodeOffset, const M: usize>(
                &self,
            ) -> Result<OptionNested<'a, Inodes::Word, $ty, M>, WireError> {
                let data_offset: usize = Inodes::INODES[O::N].data_offset() as usize;
                let (slice, option) = self.read_option_cast::<$ty>(data_offset, 1)?;
                OptionNested::new(self.read_topology::<O, M>()?, slice, option)
            }

            #[inline]
            pub fn $read<O: InodeOffset>(&self) -> $ty {
                ($from)(self.read_scalar::<O, { std::mem::size_of::<$ty>() }>())
            }

            #[inline]
            pub fn $slice<O: InodeOffset>(&self) -> Result<&'a [$ty], WireError> {
                self.read_arb_slice::<O, $ty>()
            }

            #[inline]
            pub fn $nest<O: InodeOffset, const M: usize>(
                &self,
            ) -> Result<Nested<'a, Inodes::Word, $ty, M>, WireError> {
                let slice = self.read_arb_slice::<O, $ty>()?;
                Nested::new(self.read_topology::<O, M>()?, slice)
            }
        )*
    };
}

/// Every bare array's `read_fixed_*`, run's `read_fixed_*_slice` and nest's
/// `read_fixed_*_nest`, for `[T; K]` of one fixed-width primitive `T` (as
/// [`primitive_reads`]). A bare array sits inline at its data offset; a run
/// of them is `(start, array count)`. `O` is the inode offset, `K` the array's
/// length; a nest's `M` its topology levels.
macro_rules! fixed_reads {
    ($($read:ident, $slice:ident, $nest:ident, $option_slice:ident, $option_nest:ident: $ty:ty;)*) => {
        $(
            /// Inode `O::N`'s nullable run of `[T; K]`.
            #[inline]
            pub fn $option_slice<O: InodeOffset, const K: usize>(
                &self,
            ) -> Result<OptionSlice<'a, [$ty; K]>, WireError> {
                let data_offset: usize = Inodes::INODES[O::N].data_offset() as usize;
                let (slice, option) = self.read_option_cast::<[$ty; K]>(data_offset, 1)?;
                OptionSlice::new(slice, option)
            }

            /// Inode `O::N`'s nullable nest of `[T; K]`, `M` levels.
            #[inline]
            pub fn $option_nest<O: InodeOffset, const K: usize, const M: usize>(
                &self,
            ) -> Result<OptionNested<'a, Inodes::Word, [$ty; K],  M>, WireError> {
                let data_offset: usize = Inodes::INODES[O::N].data_offset() as usize;
                let (slice, option) = self.read_option_cast::<[$ty; K]>(data_offset, 1)?;
                OptionNested::new(self.read_topology::<O, M>()?, slice, option)
            }

            #[inline]
            pub fn $read<O: InodeOffset, const K: usize>(
                &self,
            ) -> Result<&'a [$ty; K], WireError> {
                self.read_fixed_array::<O, $ty, K>()
            }

            #[inline]
            pub fn $slice<O: InodeOffset, const K: usize>(
                &self,
            ) -> Result<&'a [[$ty; K]], WireError> {
                self.read_arb_slice::<O, [$ty; K]>()
            }

            #[inline]
            pub fn $nest<O: InodeOffset, const K: usize, const M: usize>(
                &self,
            ) -> Result<Nested<'a, Inodes::Word, [$ty; K], M>, WireError> {
                let slice = self.read_arb_slice::<O, [$ty; K]>()?;
                Nested::new(self.read_topology::<O, M>()?, slice)
            }
        )*
    };
}

impl<'a, Inodes: Layout, const FIXED: usize> SplitSource<'a, Inodes, FIXED> {
    #[inline]
    pub fn new(source: &'a PyroView) -> Result<Self, WireError> {
        // PyroView is guaranteed 16 byte aligned
        Self::from_payload(source.payload())
    }

    /// Splits a bare payload (no header) at `FIXED`: the fixed region, then
    /// the variable region. Pointers count from the payload's start.
    #[inline]
    pub fn from_payload(payload: &'a [u8]) -> Result<Self, WireError> {
        if payload.len() < FIXED {
            return Err(WireError::UnderSized {
                expected: FIXED,
                actual: payload.len(),
            });
        }
        let (fixed, variable) = payload.split_at(FIXED);
        Ok(Self {
            fixed: fixed.try_into().unwrap(),
            variable,
            phantom: PhantomData,
        })
    }

    // =========================================================================
    // Fixed region
    // =========================================================================

    /// The `(start, count)` slot at `offset`: two words.
    #[inline]
    fn read_ptr(&self, offset: usize) -> (usize, usize) {
        let start = Inodes::Word::read(self.fixed, offset).to_usize();
        let count = Inodes::Word::read(self.fixed, offset + Inodes::Word::SIZE).to_usize();
        (start, count)
    }

    /// Inode `O::N`'s scalar's `SIZE` bytes, from its slot in the fixed region.
    #[inline]
    fn read_scalar<O: InodeOffset, const SIZE: usize>(&self) -> [u8; SIZE] {
        let data_offset: usize = Inodes::INODES[O::N].data_offset() as usize;
        std::array::from_fn(|i| self.fixed[data_offset + i])
    }

    /// Inode `O::N`'s bare `[T; K]`, borrowed inline from its slot in the fixed
    /// region. `T` as in [`Self::read_cast`].
    #[inline]
    fn read_fixed_array<O: InodeOffset, T: Copy, const K: usize>(
        &self,
    ) -> Result<&'a [T; K], WireError> {
        let data_offset: usize = Inodes::INODES[O::N].data_offset() as usize;
        let bytes: &'a [u8] = &self.fixed[data_offset..data_offset + std::mem::size_of::<[T; K]>()];
        if (bytes.as_ptr() as usize) % std::mem::align_of::<T>() != 0 {
            return Err(WireError::AlignmentViolation {
                expected: std::mem::align_of::<T>(),
                offset: data_offset,
            });
        }
        // SAFETY: `bytes` is `size_of::<[T; K]>()` long and aligned for `T`,
        // which is valid at every bit pattern. It borrows the view for `'a`.
        Ok(unsafe { &*(bytes.as_ptr() as *const [T; K]) })
    }

    // =========================================================================
    // Variable region
    // =========================================================================

    /// The `len` bytes at message offset `start` (pointers count from the
    /// start of the fixed region, so `start` is at least `FIXED`).
    #[inline]
    fn read_bytes(&self, start: usize, len: usize) -> Result<&'a [u8], WireError> {
        if len == 0 {
            return Ok(&[]);
        }
        let total = self.variable.len() + FIXED;
        let out_of_range =
            || WireError::RawSliceOutOfRange(start, start.saturating_add(len), total);
        let rel_start = start.checked_sub(FIXED).ok_or_else(out_of_range)?;
        let rel_end = rel_start.checked_add(len).ok_or_else(out_of_range)?;
        self.variable
            .get(rel_start..rel_end)
            .ok_or_else(out_of_range)
    }

    /// `count` `T`s at message offset `start`.
    ///
    /// Only called with [`primitive_reads`]' types or the message's
    /// [`StructureWord`]: plain LE numerics, valid at every bit pattern
    /// (the crate only builds for little-endian targets).
    #[inline]
    fn read_cast<T: Copy>(&self, start: usize, count: usize) -> Result<&'a [T], WireError> {
        let total = self.variable.len() + FIXED;
        let len = count
            .checked_mul(std::mem::size_of::<T>())
            .ok_or(WireError::RawSliceOutOfRange(start, 0, total))?;
        let bytes = self.read_bytes(start, len)?;
        if bytes.is_empty() {
            return Ok(&[]);
        }
        if (bytes.as_ptr() as usize) % std::mem::align_of::<T>() != 0 {
            return Err(WireError::AlignmentViolation {
                expected: std::mem::align_of::<T>(),
                offset: start,
            });
        }
        // SAFETY: `bytes` is in bounds, `count * size_of::<T>()` long and
        // aligned for `T`, which is valid at every bit pattern (see above).
        // It borrows the view for `'a`.
        Ok(unsafe { std::slice::from_raw_parts(bytes.as_ptr() as *const T, count) })
    }

    /// Inode `O::N`'s topology words, from its `(start, count)` slot.
    #[inline]
    fn read_words<O: InodeOffset>(&self) -> Result<&'a [Inodes::Word], WireError> {
        let topology_offset: usize = Inodes::INODES[O::N].topology_offset() as usize;
        let (start, count) = self.read_ptr(topology_offset);
        self.read_cast::<Inodes::Word>(start, count)
    }

    /// Inode `O::N`'s topology, `M` levels. A `M = 0` view has none to read,
    /// and no words at all means every level is empty.
    #[inline]
    pub fn read_topology<O: InodeOffset, const M: usize>(
        &self,
    ) -> Result<Topology<'a, Inodes::Word, M>, WireError> {
        if M == 0 {
            return Ok(Topology { topology: [&[]; M] });
        }
        let words = self.read_words::<O>()?;
        if words.is_empty() {
            return Ok(Topology { topology: [&[]; M] });
        }
        Topology::new(words)
    }

    /// Inode `O::N`'s run of `T`s, from its `(start, count)` slot.
    #[inline]
    fn read_arb_slice<O: InodeOffset, T: Copy>(&self) -> Result<&'a [T], WireError> {
        let data_offset: usize = Inodes::INODES[O::N].data_offset() as usize;
        let (start, count) = self.read_ptr(data_offset);
        self.read_cast::<T>(start, count)
    }

    // =========================================================================
    // Nullable runs: `(start, present, total)`, the present values back to
    // back, then the validity bitmap (`Lsb0`, one bit per sample, `true` =
    // present) straight after them.
    // =========================================================================

    /// The `(start, present, total)` slot at `offset`: three words.
    #[inline]
    fn read_ptr_option(&self, offset: usize) -> (usize, usize, usize) {
        let start = Inodes::Word::read(self.fixed, offset).to_usize();
        let present = Inodes::Word::read(self.fixed, offset + Inodes::Word::SIZE).to_usize();
        let total = Inodes::Word::read(self.fixed, offset + 2 * Inodes::Word::SIZE).to_usize();
        (start, present, total)
    }

    /// The `total`-bit validity bitmap at message offset `start`. Its set
    /// bits are checked against the present count by the view built on it.
    #[inline]
    fn read_mask(&self, start: usize, total: usize) -> Result<&'a BitSlice<u8, Lsb0>, WireError> {
        let bytes = self.read_bytes(start, total.div_ceil(8))?;
        Ok(&BitSlice::from_slice(bytes)[..total])
    }

    /// The nullable run at `data_offset`: `present * per` `T`s (`per` is an
    /// array's length, 1 for a plain run), then the bitmap. `T` as in
    /// [`Self::read_cast`].
    #[inline]
    fn read_option_cast<T: Copy>(
        &self,
        data_offset: usize,
        per: usize,
    ) -> Result<(&'a [T], &'a BitSlice<u8, Lsb0>), WireError> {
        let (start, present, total) = self.read_ptr_option(data_offset);
        let out_of_range = || WireError::RawSliceOutOfRange(start, 0, self.variable.len() + FIXED);
        let elements = present.checked_mul(per).ok_or_else(out_of_range)?;
        let values = self.read_cast::<T>(start, elements)?;
        let mask_start = elements
            .checked_mul(std::mem::size_of::<T>())
            .and_then(|len| start.checked_add(len))
            .ok_or_else(out_of_range)?;
        Ok((values, self.read_mask(mask_start, total)?))
    }

    /// The nullable bool run at `data_offset`: `present * per` bits (`per`
    /// is an array's length, 1 for a plain run), then the bitmap.
    #[inline]
    fn read_option_bits(
        &self,
        data_offset: usize,
        per: usize,
    ) -> Result<(&'a BitSlice<u8, Lsb0>, &'a BitSlice<u8, Lsb0>), WireError> {
        let (start, present, total) = self.read_ptr_option(data_offset);
        let out_of_range = || WireError::RawSliceOutOfRange(start, 0, self.variable.len() + FIXED);
        let bits = present.checked_mul(per).ok_or_else(out_of_range)?;
        let bytes = self.read_bytes(start, bits.div_ceil(8))?;
        let mask_start = start
            .checked_add(bits.div_ceil(8))
            .ok_or_else(out_of_range)?;
        Ok((
            &BitSlice::from_slice(bytes)[..bits],
            self.read_mask(mask_start, total)?,
        ))
    }

    // =========================================================================
    // Primitives
    // =========================================================================

    primitive_reads! {
        read_u8, read_u8_slice, read_u8_nest, read_u8_option_slice, read_u8_option_nest: u8 = u8::from_le_bytes;
        read_i8, read_i8_slice, read_i8_nest, read_i8_option_slice, read_i8_option_nest: i8 = i8::from_le_bytes;
        read_u16, read_u16_slice, read_u16_nest, read_u16_option_slice, read_u16_option_nest: u16 = u16::from_le_bytes;
        read_i16, read_i16_slice, read_i16_nest, read_i16_option_slice, read_i16_option_nest: i16 = i16::from_le_bytes;
        read_u32, read_u32_slice, read_u32_nest, read_u32_option_slice, read_u32_option_nest: u32 = u32::from_le_bytes;
        read_i32, read_i32_slice, read_i32_nest, read_i32_option_slice, read_i32_option_nest: i32 = i32::from_le_bytes;
        read_u64, read_u64_slice, read_u64_nest, read_u64_option_slice, read_u64_option_nest: u64 = u64::from_le_bytes;
        read_i64, read_i64_slice, read_i64_nest, read_i64_option_slice, read_i64_option_nest: i64 = i64::from_le_bytes;
        read_f16, read_f16_slice, read_f16_nest, read_f16_option_slice, read_f16_option_nest: f16 = f16::from_le_bytes;
        read_f32, read_f32_slice, read_f32_nest, read_f32_option_slice, read_f32_option_nest: f32 = f32::from_le_bytes;
        read_f64, read_f64_slice, read_f64_nest, read_f64_option_slice, read_f64_option_nest: f64 = f64::from_le_bytes;
        read_time, read_time_slice, read_time_nest, read_time_option_slice, read_time_option_nest: Time = |b| Time(i128::from_le_bytes(b));
    }

    fixed_reads! {
        read_fixed_u8, read_fixed_u8_slice, read_fixed_u8_nest, read_fixed_u8_option_slice, read_fixed_u8_option_nest: u8;
        read_fixed_i8, read_fixed_i8_slice, read_fixed_i8_nest, read_fixed_i8_option_slice, read_fixed_i8_option_nest: i8;
        read_fixed_u16, read_fixed_u16_slice, read_fixed_u16_nest, read_fixed_u16_option_slice, read_fixed_u16_option_nest: u16;
        read_fixed_i16, read_fixed_i16_slice, read_fixed_i16_nest, read_fixed_i16_option_slice, read_fixed_i16_option_nest: i16;
        read_fixed_u32, read_fixed_u32_slice, read_fixed_u32_nest, read_fixed_u32_option_slice, read_fixed_u32_option_nest: u32;
        read_fixed_i32, read_fixed_i32_slice, read_fixed_i32_nest, read_fixed_i32_option_slice, read_fixed_i32_option_nest: i32;
        read_fixed_u64, read_fixed_u64_slice, read_fixed_u64_nest, read_fixed_u64_option_slice, read_fixed_u64_option_nest: u64;
        read_fixed_i64, read_fixed_i64_slice, read_fixed_i64_nest, read_fixed_i64_option_slice, read_fixed_i64_option_nest: i64;
        read_fixed_f16, read_fixed_f16_slice, read_fixed_f16_nest, read_fixed_f16_option_slice, read_fixed_f16_option_nest: f16;
        read_fixed_f32, read_fixed_f32_slice, read_fixed_f32_nest, read_fixed_f32_option_slice, read_fixed_f32_option_nest: f32;
        read_fixed_f64, read_fixed_f64_slice, read_fixed_f64_nest, read_fixed_f64_option_slice, read_fixed_f64_option_nest: f64;
        read_fixed_time, read_fixed_time_slice, read_fixed_time_nest, read_fixed_time_option_slice, read_fixed_time_option_nest: Time;
    }

    // =========================================================================
    // Bools: a scalar is one byte (0x00 = false), a run is bit-packed
    // (`Lsb0`) and its slot's count is the bit count.
    // =========================================================================

    /// Whether Inode `O::N`'s nullable scalar is present: its bit in the fixed
    /// region's bitmap (`Lsb0`).
    #[inline]
    pub fn read_nullable<O: InodeOffset>(&self) -> bool {
        let bit = Inodes::INODES[O::N].nullable_bit_index() as usize;
        self.fixed[Inodes::VALIDITY_START + bit / 8] & (1 << (bit % 8)) != 0
    }

    #[inline]
    pub fn read_bool<O: InodeOffset>(&self) -> bool {
        self.read_scalar::<O, 1>()[0] != 0
    }

    #[inline]
    pub fn read_bool_slice<O: InodeOffset>(&self) -> Result<&'a BitSlice<u8, Lsb0>, WireError> {
        let data_offset: usize = Inodes::INODES[O::N].data_offset() as usize;
        let (start, len) = self.read_ptr(data_offset);
        let bytes = self.read_bytes(start, len.div_ceil(8))?;
        Ok(&BitSlice::from_slice(bytes)[..len])
    }

    #[inline]
    pub fn read_bool_nest<O: InodeOffset, const M: usize>(
        &self,
    ) -> Result<BoolNested<'a, Inodes::Word, M>, WireError> {
        let slice = self.read_bool_slice::<O>()?;
        BoolNested::new(self.read_topology::<O, M>()?, slice)
    }

    // =========================================================================
    // Fixed bools: a bare `[bool; K]` is its `K` bits packed inline at its
    // data offset; a run is `count * K` bits back to back, and its slot's
    // count is the array count.
    // =========================================================================

    #[inline]
    pub fn read_fixed_bool<O: InodeOffset, const K: usize>(&self) -> &'a BitSlice<u8, Lsb0> {
        let data_offset: usize = Inodes::INODES[O::N].data_offset() as usize;
        let bytes: &'a [u8] = &self.fixed[data_offset..data_offset + K.div_ceil(8)];
        &BitSlice::from_slice(bytes)[..K]
    }

    /// Every array's bits, back to back, and the array count.
    #[inline]
    fn read_fixed_bits<O: InodeOffset, const K: usize>(
        &self,
    ) -> Result<(&'a BitSlice<u8, Lsb0>, usize), WireError> {
        let data_offset: usize = Inodes::INODES[O::N].data_offset() as usize;
        let (start, count) = self.read_ptr(data_offset);
        let len = count.checked_mul(K).ok_or(WireError::RawSliceOutOfRange(
            start,
            0,
            self.variable.len() + FIXED,
        ))?;
        let bytes = self.read_bytes(start, len.div_ceil(8))?;
        Ok((&BitSlice::from_slice(bytes)[..len], count))
    }

    #[inline]
    pub fn read_fixed_bool_slice<O: InodeOffset, const K: usize>(
        &self,
    ) -> Result<FixedBoolSlice<'a, K>, WireError> {
        let (slice, count) = self.read_fixed_bits::<O, K>()?;
        Ok(FixedBoolSlice { slice, count })
    }

    #[inline]
    pub fn read_fixed_bool_nest<O: InodeOffset, const K: usize, const M: usize>(
        &self,
    ) -> Result<FixedBoolNested<'a, Inodes::Word, K, M>, WireError> {
        let (slice, count) = self.read_fixed_bits::<O, K>()?;
        let topology = self.read_topology::<O, M>()?;
        if M != 0 && topology.total() != count {
            return Err(WireError::InvalidTopologyLen {
                topo: topology.total(),
                data: count,
            });
        }
        Ok(FixedBoolNested {
            topology,
            slice,
            count,
        })
    }

    // =========================================================================
    // Nullable bools: the present values are bit-packed too; a fixed run's
    // `present` counts arrays, `K` bits each.
    // =========================================================================

    /// Inode `O::N`'s nullable bool run.
    #[inline]
    pub fn read_bool_option_slice<O: InodeOffset>(&self) -> Result<OptionBoolSlice<'a>, WireError> {
        let data_offset: usize = Inodes::INODES[O::N].data_offset() as usize;
        let (slice, option) = self.read_option_bits(data_offset, 1)?;
        OptionBoolSlice::new(slice, option)
    }

    /// Inode `O::N`'s nullable bool nest, `M` levels.
    #[inline]
    pub fn read_bool_option_nest<O: InodeOffset, const M: usize>(
        &self,
    ) -> Result<OptionBoolNested<'a, Inodes::Word, M>, WireError> {
        let data_offset: usize = Inodes::INODES[O::N].data_offset() as usize;
        let (slice, option) = self.read_option_bits(data_offset, 1)?;
        OptionBoolNested::new(self.read_topology::<O, M>()?, slice, option)
    }

    /// Inode `O::N`'s nullable run of `[bool; K]`.
    #[inline]
    pub fn read_fixed_bool_option_slice<O: InodeOffset, const K: usize>(
        &self,
    ) -> Result<OptionFixedBoolSlice<'a, K>, WireError> {
        let data_offset: usize = Inodes::INODES[O::N].data_offset() as usize;
        let (slice, option) = self.read_option_bits(data_offset, K)?;
        OptionFixedBoolSlice::new(slice, option)
    }

    /// Inode `O::N`'s nullable nest of `[bool; K]`, `M` levels: the present
    /// arrays' bits back to back, and one bit per sample.
    #[inline]
    pub fn read_fixed_bool_option_nest<O: InodeOffset, const K: usize, const M: usize>(
        &self,
    ) -> Result<OptionFixedBoolNested<'a, Inodes::Word, K, M>, WireError> {
        let data_offset: usize = Inodes::INODES[O::N].data_offset() as usize;
        let (slice, option) = self.read_option_bits(data_offset, K)?;
        let topology = self.read_topology::<O, M>()?;
        if M != 0 && topology.total() != option.len() {
            return Err(WireError::InvalidTopologyLen {
                topo: topology.total(),
                data: option.len(),
            });
        }
        if option.count_ones().checked_mul(K) != Some(slice.len()) {
            return Err(WireError::InvalidOptionLen {
                valid: option.count_ones(),
                data: slice.len() / K.max(1),
            });
        }
        Ok(OptionFixedBoolNested {
            topology,
            slice,
            option,
            count: option.len(),
        })
    }

    // =========================================================================
    // Groups and maps: their slots sit at the inode's topology offset.
    // =========================================================================

    /// A bare map's entry count: the one word in its slot.
    #[inline]
    pub fn read_entry_count<O: InodeOffset>(&self) -> usize {
        let topology_offset: usize = Inodes::INODES[O::N].topology_offset() as usize;
        Inodes::Word::read(self.fixed, topology_offset).to_usize()
    }

    /// How many instances of group inode `O::N` the message holds, from the
    /// group's own slot: the rows of each of its field columns. Only a
    /// struct with no fields needs it, having no column of its own to count.
    ///
    /// The inode's depth and nullability are constants, so this compiles to
    /// the one read its slot needs. With `tiers = nesting + group_nest - 1`
    /// (the value writer's depth):
    ///
    /// * `Group`, no tiers: the one-word count in its slot.
    /// * `Group` under tiers: the last tier's total.
    /// * `OptionGroup`: only present groups feed their fields, so the set
    ///   bits of its mask — behind `(mask start, slots)` with no tiers, or
    ///   behind the tiers' words otherwise.
    ///
    /// A group that isn't repeated at all (`nesting == 0` and not
    /// `group_nest`) is one instance.
    #[inline]
    pub fn read_group_rows<O: InodeOffset>(&self) -> Result<usize, WireError> {
        let lists = const {
            let inode = Inodes::INODES[O::N];
            inode.nesting() as usize + inode.group_nest() as usize
        };
        let nullable = const { matches!(Inodes::INODES[O::N].kind, WireKind::OptionGroup) };
        macro_rules! under_tiers {
            ($($tiers:literal),*) => {
                match lists - 1 {
                    0 if nullable => Ok(self.read_group_mask::<O>()?.count_ones()),
                    0 => Ok(self.read_entry_count::<O>()),
                    $(
                        $tiers if nullable => {
                            Ok(self.read_group_option_nest::<O, $tiers>()?.1.count_ones())
                        }
                        $tiers => Ok(self.read_topology::<O, $tiers>()?.total()),
                    )*
                    _ => Err(WireError::InvalidInodes),
                }
            };
        }
        if lists == 0 {
            return Ok(1);
        }
        under_tiers!(1, 2, 3, 4, 5, 6)
    }

    /// A nullable group column with no tiers: the slot is
    /// `(mask start, slots)`.
    #[inline]
    pub fn read_group_mask<O: InodeOffset>(&self) -> Result<&'a BitSlice<u8, Lsb0>, WireError> {
        let topology_offset: usize = Inodes::INODES[O::N].topology_offset() as usize;
        let (start, slots) = self.read_ptr(topology_offset);
        self.read_mask(start, slots)
    }

    /// A nullable group column under `M` tiers: the tiers from the
    /// `(start, words)` slot, then the mask right behind them over the last
    /// tier's total.
    #[inline]
    pub fn read_group_option_nest<O: InodeOffset, const M: usize>(
        &self,
    ) -> Result<(Topology<'a, Inodes::Word, M>, &'a BitSlice<u8, Lsb0>), WireError> {
        let topology = self.read_topology::<O, M>()?;
        let topology_offset: usize = Inodes::INODES[O::N].topology_offset() as usize;
        let (start, count) = self.read_ptr(topology_offset);
        let mask_start = count
            .checked_mul(Inodes::Word::SIZE)
            .and_then(|len| start.checked_add(len))
            .ok_or(WireError::RawSliceOutOfRange(
                start,
                0,
                self.variable.len() + FIXED,
            ))?;
        let mask = self.read_mask(mask_start, topology.total())?;
        Ok((topology, mask))
    }

    // =========================================================================
    // Strings: the slot is `(start, byte count)`; loading checks UTF-8.
    // =========================================================================

    #[inline]
    pub fn read_str<O: InodeOffset>(&self) -> Result<&'a str, WireError> {
        let data_offset: usize = Inodes::INODES[O::N].data_offset() as usize;
        let (start, len) = self.read_ptr(data_offset);
        let bytes = self.read_bytes(start, len)?;
        check_str(start, bytes)
    }

    #[inline]
    pub fn read_str_nest<O: InodeOffset, const M: usize>(
        &self,
    ) -> Result<StrNested<'a, Inodes::Word, M>, WireError> {
        let data_offset: usize = Inodes::INODES[O::N].data_offset() as usize;
        let (start, len) = self.read_ptr(data_offset);
        let bytes = self.read_bytes(start, len)?;
        let words = if M == 0 {
            &[][..]
        } else {
            self.read_words::<O>()?
        };
        if M != 0 && words.is_empty() {
            if !bytes.is_empty() {
                return Err(WireError::InvalidTopologyLen {
                    topo: 0,
                    data: bytes.len(),
                });
            }
            // SAFETY: no strings, no bytes: nothing to be UTF-8, and every
            // (empty) level's last boundary is `bytes.len()`, 0.
            return Ok(unsafe {
                StrNested::from_validated(Topology { topology: [&[]; M] }, bytes)
            });
        }
        Topology::new_str(start, words, bytes)
    }
}
