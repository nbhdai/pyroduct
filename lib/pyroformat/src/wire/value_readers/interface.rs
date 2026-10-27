use std::marker::PhantomData;

use bitvec::{order::Lsb0, slice::BitSlice};
use half::f16;

use crate::{
    MAX_TIERS,
    errors::WireError,
    refs::{
        BoolNested, Nested, OptionBoolNested, OptionBoolSlice, OptionNested, OptionSlice,
        StrNested, Topology, check_str,
        dynamic_fixed::{FixedSlice, OptionFixedSlice},
    },
    schema::{Inode, StructureWord, nullable_validity_start},
    value::Time,
};

/// Every scalar's `read_*`, run's `read_*_slice`, nest's `read_*_nest`,
/// bare array's `read_fixed_*` and run of arrays' `read_fixed_*_slice`, for
/// one fixed-width primitive: plain LE numerics (or a newtype over one),
/// valid at every bit pattern. `$from` decodes the scalar's LE bytes. `n` is
/// the inode; a nest's `levels` its topology levels, read into a
/// [`MAX_TIERS`] view; an array's `len` its element count, only known at
/// runtime.
macro_rules! primitive_reads {
    ($($read:ident, $option_read:ident, $slice:ident, $nest:ident, $fixed:ident, $option_fixed:ident, $fixed_slice:ident, $option_slice:ident, $option_nest:ident, $fixed_option_slice:ident: $ty:ty = $from:expr;)*) => {
        $(
            /// Inode `n`'s nullable run (`Option*Slice`).
            #[inline]
            pub fn $option_slice(
                &self,
                n: usize,
            ) -> Result<OptionSlice<'a, $ty>, WireError> {
                let data_offset: usize = self.inodes[n].data_offset() as usize;
                let (slice, option) = self.read_option_cast::<$ty>(data_offset, 1)?;
                OptionSlice::new(slice, option)
            }

            /// Inode `n`'s nullable nest, `levels` levels.
            #[inline]
            pub fn $option_nest(
                &self,
                n: usize,
                levels: usize,
            ) -> Result<OptionNested<'a, W, $ty, MAX_TIERS>, WireError> {
                let data_offset: usize = self.inodes[n].data_offset() as usize;
                let (slice, option) = self.read_option_cast::<$ty>(data_offset, 1)?;
                OptionNested::new(self.read_topology(n, levels)?, slice, option)
            }

            /// Inode `n`'s nullable run of `[T; len]`: the present arrays'
            /// `T`s back to back (`present * len`), and one bit per sample.
            #[inline]
            pub fn $fixed_option_slice(
                &self,
                n: usize,
                len: usize,
            ) -> Result<OptionFixedSlice<'a, $ty>, WireError> {
                let data_offset: usize = self.inodes[n].data_offset() as usize;
                let (dense, option) = self.read_option_cast::<$ty>(data_offset, len)?;
                check_fixed_option(dense.len(), len, option)?;
                Ok(OptionFixedSlice { dense, n: len, option })
            }

            /// Inode `n`'s run of `[T; len]`: every array's `T`s back to
            /// back, `count * len` of them. The slot's count is the array
            /// count.
            #[inline]
            pub fn $fixed_slice(&self, n: usize, len: usize) -> Result<FixedSlice<'a, $ty>, WireError> {
                let slice = self.read_fixed_arb_slice::<$ty>(n, len)?;
                if slice.len() % len != 0 {
                    return Err(WireError::InvalidFixedLen { expected: len });
                }
                Ok(FixedSlice { slice, n: len })
            }

            /// Inode `n`'s bare `[T; len]`, borrowed inline from its slot in
            /// the fixed region.
            #[inline]
            pub fn $fixed(&self, n: usize, len: usize) -> Result<&'a [$ty], WireError> {
                self.read_fixed_array::<$ty>(n, len)
            }

            #[inline]
            /// Inode `n`'s nullable bare `[T; len]`: `None` when its bit in
            /// the fixed region's bitmap is clear.
            pub fn $option_fixed(
                &self,
                n: usize,
                len: usize,
            ) -> Result<Option<&'a [$ty]>, WireError> {
                if self.read_nullable(n) {
                    Ok(Some(self.read_fixed_array::<$ty>(n, len)?))
                } else {
                    Ok(None)
                }
            }

            #[inline]
            pub fn $read(&self, n: usize) -> $ty {
                ($from)(self.read_scalar::<{ std::mem::size_of::<$ty>() }>(n))
            }

            #[inline]
            pub fn $option_read(&self, n: usize) -> Option<$ty> {
                if self.read_nullable(n) {
                    Some(($from)(self.read_scalar::<{ std::mem::size_of::<$ty>() }>(n)))
                } else {
                    None
                }
            }

            #[inline]
            pub fn $slice(&self, n: usize) -> Result<&'a [$ty], WireError> {
                self.read_arb_slice::<$ty>(n)
            }

            #[inline]
            pub fn $nest(
                &self,
                n: usize,
                levels: usize,
            ) -> Result<Nested<'a, W, $ty, MAX_TIERS>, WireError> {
                let slice = self.read_arb_slice::<$ty>(n)?;
                Nested::new(self.read_topology(n, levels)?, slice)
            }
        )*
    };
}

/// A nullable run of `[T; len]` holds one dense array per set bit of
/// `option`: `values` (elements or bits) must be `count_ones * len` long.
#[inline]
fn check_fixed_option(
    values: usize,
    len: usize,
    option: &BitSlice<u8, Lsb0>,
) -> Result<(), WireError> {
    if option.count_ones().checked_mul(len) != Some(values) {
        return Err(WireError::InvalidOptionLen {
            valid: option.count_ones(),
            data: values / len.max(1),
        });
    }
    Ok(())
}

/// [`crate::wire::readers::interface::SplitSource`] for a layout only known at runtime:
/// the inodes and the fixed region's length are values, so every inode `n`
/// is an argument rather than a const. A nest's `M` (its topology levels)
/// is still a const, since it is part of the view's type.
pub struct SplitSource<'a, W: StructureWord> {
    inodes: &'a [Inode],
    fixed_len: usize,
    /// Where the fixed region's nullable bitmap starts.
    validity_start: usize,
    fixed: &'a [u8],
    variable: &'a [u8],
    phantom: PhantomData<W>,
}

impl<'a, W: StructureWord> SplitSource<'a, W> {
    #[inline]
    pub fn new(source: &'a [u8], fixed_len: usize, inodes: &'a [Inode]) -> Result<Self, WireError> {
        // PyroView is guaranteed 16 byte aligned
        if source.len() < fixed_len {
            return Err(WireError::UnderSized {
                expected: fixed_len,
                actual: source.len(),
            });
        }
        let (fixed, variable) = source.split_at(fixed_len);
        Ok(Self {
            inodes,
            fixed_len,
            validity_start: nullable_validity_start::<W>(inodes),
            fixed,
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
        let start = W::read(self.fixed, offset).to_usize();
        let count = W::read(self.fixed, offset + W::SIZE).to_usize();
        (start, count)
    }

    /// Inode `n`'s scalar's `SIZE` bytes, from its slot in the fixed region.
    #[inline]
    fn read_scalar<const SIZE: usize>(&self, n: usize) -> [u8; SIZE] {
        let data_offset: usize = self.inodes[n].data_offset() as usize;
        std::array::from_fn(|i| self.fixed[data_offset + i])
    }

    /// Inode `n`'s bare `[T; len]`, borrowed inline from its slot in the
    /// fixed region. `T` as in [`Self::read_cast`].
    #[inline]
    fn read_fixed_array<T: Copy>(&self, n: usize, len: usize) -> Result<&'a [T], WireError> {
        let data_offset: usize = self.inodes[n].data_offset() as usize;
        let byte_len = len.checked_mul(std::mem::size_of::<T>());
        let end = byte_len.and_then(|b| data_offset.checked_add(b));
        let bytes: &'a [u8] = end.and_then(|end| self.fixed.get(data_offset..end)).ok_or(
            WireError::RawSliceOutOfRange(data_offset, end.unwrap_or(usize::MAX), self.fixed_len),
        )?;
        if bytes.is_empty() {
            return Ok(&[]);
        }
        if (bytes.as_ptr() as usize) % std::mem::align_of::<T>() != 0 {
            return Err(WireError::AlignmentViolation {
                expected: std::mem::align_of::<T>(),
                offset: data_offset,
            });
        }
        // SAFETY: `bytes` is in bounds, `len * size_of::<T>()` long and
        // aligned for `T`, which is valid at every bit pattern. It borrows
        // the view for `'a`.
        Ok(unsafe { std::slice::from_raw_parts(bytes.as_ptr() as *const T, len) })
    }

    // =========================================================================
    // Variable region
    // =========================================================================

    /// The `len` bytes at message offset `start` (pointers count from the
    /// start of the fixed region, so `start` is at least `fixed_len`).
    #[inline]
    fn read_bytes(&self, start: usize, len: usize) -> Result<&'a [u8], WireError> {
        if len == 0 {
            return Ok(&[]);
        }
        let total = self.variable.len() + self.fixed_len;
        let out_of_range =
            || WireError::RawSliceOutOfRange(start, start.saturating_add(len), total);
        let rel_start = start.checked_sub(self.fixed_len).ok_or_else(out_of_range)?;
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
        let total = self.variable.len() + self.fixed_len;
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

    /// Inode `n`'s topology words, from its `(start, count)` slot.
    #[inline]
    fn read_words(&self, n: usize) -> Result<&'a [W], WireError> {
        let topology_offset: usize = self.inodes[n].topology_offset() as usize;
        let (start, count) = self.read_ptr(topology_offset);
        self.read_cast::<W>(start, count)
    }

    /// Inode `n`'s topology, `levels` levels. No levels means none to read,
    /// and no words at all means every level is empty.
    #[inline]
    pub(crate) fn read_topology(
        &self,
        n: usize,
        levels: usize,
    ) -> Result<Topology<'a, W, MAX_TIERS>, WireError> {
        if levels == 0 {
            return Ok(Topology {
                topology: [&[]; MAX_TIERS],
            });
        }
        let words = self.read_words(n)?;
        if words.is_empty() {
            return Ok(Topology {
                topology: [&[]; MAX_TIERS],
            });
        }
        Topology::new_dynamic(words, levels)
    }

    /// Inode `n`'s run of `T`s, from its `(start, count)` slot.
    #[inline]
    fn read_arb_slice<T: Copy>(&self, n: usize) -> Result<&'a [T], WireError> {
        let data_offset: usize = self.inodes[n].data_offset() as usize;
        let (start, count) = self.read_ptr(data_offset);
        self.read_cast::<T>(start, count)
    }

    /// Inode `n`'s run of `[T; len]`, from its `(start, array count)` slot,
    /// as `count * len` `T`s.
    #[inline]
    fn read_fixed_arb_slice<T: Copy>(&self, n: usize, len: usize) -> Result<&'a [T], WireError> {
        let data_offset: usize = self.inodes[n].data_offset() as usize;
        let (start, count) = self.read_ptr(data_offset);
        let elements = count.checked_mul(len).ok_or(WireError::RawSliceOutOfRange(
            start,
            0,
            self.variable.len() + self.fixed_len,
        ))?;
        self.read_cast::<T>(start, elements)
    }

    // =========================================================================
    // Nullable runs: `(start, present, total)`, the present values back to
    // back, then the validity bitmap (`Lsb0`, one bit per sample, `true` =
    // present) straight after them.
    // =========================================================================

    /// The `(start, present, total)` slot at `offset`: three words.
    #[inline]
    fn read_ptr_option(&self, offset: usize) -> (usize, usize, usize) {
        let start = W::read(self.fixed, offset).to_usize();
        let present = W::read(self.fixed, offset + W::SIZE).to_usize();
        let total = W::read(self.fixed, offset + 2 * W::SIZE).to_usize();
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
        let out_of_range =
            || WireError::RawSliceOutOfRange(start, 0, self.variable.len() + self.fixed_len);
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
        let out_of_range =
            || WireError::RawSliceOutOfRange(start, 0, self.variable.len() + self.fixed_len);
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
        read_u8, read_option_u8, read_u8_slice, read_u8_nest, read_fixed_u8, read_option_fixed_u8, read_fixed_u8_slice, read_u8_option_slice, read_u8_option_nest, read_fixed_u8_option_slice: u8 = u8::from_le_bytes;
        read_i8, read_option_i8, read_i8_slice, read_i8_nest, read_fixed_i8, read_option_fixed_i8, read_fixed_i8_slice, read_i8_option_slice, read_i8_option_nest, read_fixed_i8_option_slice: i8 = i8::from_le_bytes;
        read_u16, read_option_u16, read_u16_slice, read_u16_nest, read_fixed_u16, read_option_fixed_u16, read_fixed_u16_slice, read_u16_option_slice, read_u16_option_nest, read_fixed_u16_option_slice: u16 = u16::from_le_bytes;
        read_i16, read_option_i16, read_i16_slice, read_i16_nest, read_fixed_i16, read_option_fixed_i16, read_fixed_i16_slice, read_i16_option_slice, read_i16_option_nest, read_fixed_i16_option_slice: i16 = i16::from_le_bytes;
        read_u32, read_option_u32, read_u32_slice, read_u32_nest, read_fixed_u32, read_option_fixed_u32, read_fixed_u32_slice, read_u32_option_slice, read_u32_option_nest, read_fixed_u32_option_slice: u32 = u32::from_le_bytes;
        read_i32, read_option_i32, read_i32_slice, read_i32_nest, read_fixed_i32, read_option_fixed_i32, read_fixed_i32_slice, read_i32_option_slice, read_i32_option_nest, read_fixed_i32_option_slice: i32 = i32::from_le_bytes;
        read_u64, read_option_u64, read_u64_slice, read_u64_nest, read_fixed_u64, read_option_fixed_u64, read_fixed_u64_slice, read_u64_option_slice, read_u64_option_nest, read_fixed_u64_option_slice: u64 = u64::from_le_bytes;
        read_i64, read_option_i64, read_i64_slice, read_i64_nest, read_fixed_i64, read_option_fixed_i64, read_fixed_i64_slice, read_i64_option_slice, read_i64_option_nest, read_fixed_i64_option_slice: i64 = i64::from_le_bytes;
        read_f16, read_option_f16, read_f16_slice, read_f16_nest, read_fixed_f16, read_option_fixed_f16, read_fixed_f16_slice, read_f16_option_slice, read_f16_option_nest, read_fixed_f16_option_slice: f16 = f16::from_le_bytes;
        read_f32, read_option_f32, read_f32_slice, read_f32_nest, read_fixed_f32, read_option_fixed_f32, read_fixed_f32_slice, read_f32_option_slice, read_f32_option_nest, read_fixed_f32_option_slice: f32 = f32::from_le_bytes;
        read_f64, read_option_f64, read_f64_slice, read_f64_nest, read_fixed_f64, read_option_fixed_f64, read_fixed_f64_slice, read_f64_option_slice, read_f64_option_nest, read_fixed_f64_option_slice: f64 = f64::from_le_bytes;
        read_time, read_option_time, read_time_slice, read_time_nest, read_fixed_time, read_option_fixed_time, read_fixed_time_slice, read_time_option_slice, read_time_option_nest, read_fixed_time_option_slice: Time = |b| Time(i128::from_le_bytes(b));
    }

    // =========================================================================
    // Bools: a scalar is one byte (0x00 = false), a run is bit-packed
    // (`Lsb0`) and its slot's count is the bit count.
    // =========================================================================

    /// Whether inode `n`'s nullable scalar is present: its bit in the fixed
    /// region's bitmap (`Lsb0`).
    #[inline]
    pub fn read_nullable(&self, n: usize) -> bool {
        let bit = self.inodes[n].nullable_bit_index() as usize;
        self.fixed[self.validity_start + bit / 8] & (1 << (bit % 8)) != 0
    }

    #[inline]
    pub fn read_bool(&self, n: usize) -> bool {
        self.read_scalar::<1>(n)[0] != 0
    }

    /// Inode `n`'s bare `[bool; len]`: its `len` bits packed inline at its
    /// data offset. A run of them is [`Self::read_fixed_bool_slice`].
    #[inline]
    pub fn read_fixed_bool(&self, n: usize, len: usize) -> Result<&'a BitSlice<u8>, WireError> {
        let data_offset: usize = self.inodes[n].data_offset() as usize;
        let end = data_offset.saturating_add(len.div_ceil(8));
        let bytes: &'a [u8] =
            self.fixed
                .get(data_offset..end)
                .ok_or(WireError::RawSliceOutOfRange(
                    data_offset,
                    end,
                    self.fixed_len,
                ))?;
        Ok(&BitSlice::from_slice(bytes)[..len])
    }

    /// Inode `n`'s run of `[bool; len]`: every array's bits back to back
    /// (`Lsb0`, arrays not byte-aligned), `count * len` bits in all. The
    /// slot's count is the array count, not the bit count.
    #[inline]
    pub fn read_fixed_bool_slice(
        &self,
        n: usize,
        len: usize,
    ) -> Result<&'a BitSlice<u8>, WireError> {
        let data_offset: usize = self.inodes[n].data_offset() as usize;
        let (start, count) = self.read_ptr(data_offset);
        let bits = count.checked_mul(len).ok_or(WireError::RawSliceOutOfRange(
            start,
            0,
            self.variable.len() + self.fixed_len,
        ))?;
        let bytes = self.read_bytes(start, bits.div_ceil(8))?;
        Ok(&BitSlice::from_slice(bytes)[..bits])
    }

    #[inline]
    pub fn read_bool_slice(&self, n: usize) -> Result<&'a BitSlice<u8>, WireError> {
        let data_offset: usize = self.inodes[n].data_offset() as usize;
        let (start, len) = self.read_ptr(data_offset);
        let bytes = self.read_bytes(start, len.div_ceil(8))?;
        Ok(&BitSlice::from_slice(bytes)[..len])
    }

    #[inline]
    pub fn read_bool_nest(
        &self,
        n: usize,
        levels: usize,
    ) -> Result<BoolNested<'a, W, MAX_TIERS>, WireError> {
        let slice = self.read_bool_slice(n)?;
        BoolNested::new(self.read_topology(n, levels)?, slice)
    }

    // =========================================================================
    // Nullable bools: the present values are bit-packed too; a fixed run's
    // `present` counts arrays, `len` bits each.
    // =========================================================================

    /// Inode `n`'s nullable bool run.
    #[inline]
    pub fn read_bool_option_slice(&self, n: usize) -> Result<OptionBoolSlice<'a>, WireError> {
        let data_offset: usize = self.inodes[n].data_offset() as usize;
        let (slice, option) = self.read_option_bits(data_offset, 1)?;
        OptionBoolSlice::new(slice, option)
    }

    /// Inode `n`'s nullable bool nest, `levels` levels.
    #[inline]
    pub fn read_bool_option_nest(
        &self,
        n: usize,
        levels: usize,
    ) -> Result<OptionBoolNested<'a, W, MAX_TIERS>, WireError> {
        let data_offset: usize = self.inodes[n].data_offset() as usize;
        let (slice, option) = self.read_option_bits(data_offset, 1)?;
        OptionBoolNested::new(self.read_topology(n, levels)?, slice, option)
    }

    /// Inode `n`'s nullable run of `[bool; len]`: the present arrays' bits
    /// back to back (`present * len`), and one bit per sample (`true` =
    /// present). No view takes a runtime array length over bits, so this
    /// hands back the parts, checked against each other.
    #[inline]
    pub fn read_fixed_bool_option_slice(
        &self,
        n: usize,
        len: usize,
    ) -> Result<(&'a BitSlice<u8, Lsb0>, &'a BitSlice<u8, Lsb0>), WireError> {
        let data_offset: usize = self.inodes[n].data_offset() as usize;
        let (bits, option) = self.read_option_bits(data_offset, len)?;
        check_fixed_option(bits.len(), len, option)?;
        Ok((bits, option))
    }

    // =========================================================================
    // Groups and maps: their slots sit at the inode's topology offset.
    // =========================================================================

    /// The one word in inode `n`'s slot: a bare map's entry count, or a
    /// list of groups' row count.
    #[inline]
    pub fn read_count(&self, n: usize) -> usize {
        let topology_offset: usize = self.inodes[n].topology_offset() as usize;
        W::read(self.fixed, topology_offset).to_usize()
    }

    /// A nullable group column with no tiers: the slot is
    /// `(mask start, slots)`.
    #[inline]
    pub fn read_group_mask(&self, n: usize) -> Result<&'a BitSlice<u8, Lsb0>, WireError> {
        let topology_offset: usize = self.inodes[n].topology_offset() as usize;
        let (start, slots) = self.read_ptr(topology_offset);
        self.read_mask(start, slots)
    }

    /// A nullable group column under `levels` tiers: the tiers from the
    /// `(start, words)` slot, then the mask right behind them over the last
    /// tier's total.
    #[inline]
    pub fn read_group_option_nest(
        &self,
        n: usize,
        levels: usize,
    ) -> Result<(Topology<'a, W, MAX_TIERS>, &'a BitSlice<u8, Lsb0>), WireError> {
        let topology = self.read_topology(n, levels)?;
        let topology_offset: usize = self.inodes[n].topology_offset() as usize;
        let (start, count) = self.read_ptr(topology_offset);
        let mask_start = count
            .checked_mul(W::SIZE)
            .and_then(|len| start.checked_add(len))
            .ok_or(WireError::RawSliceOutOfRange(
                start,
                0,
                self.variable.len() + self.fixed_len,
            ))?;
        let mask = self.read_mask(mask_start, topology.total())?;
        Ok((topology, mask))
    }

    // =========================================================================
    // Strings: the slot is `(start, byte count)`; loading checks UTF-8.
    // =========================================================================

    #[inline]
    pub fn read_str(&self, n: usize) -> Result<&'a str, WireError> {
        let data_offset: usize = self.inodes[n].data_offset() as usize;
        let (start, len) = self.read_ptr(data_offset);
        let bytes = self.read_bytes(start, len)?;
        check_str(start, bytes)
    }

    #[inline]
    pub fn read_str_nest(
        &self,
        n: usize,
        levels: usize,
    ) -> Result<StrNested<'a, W, MAX_TIERS>, WireError> {
        let data_offset: usize = self.inodes[n].data_offset() as usize;
        let (start, len) = self.read_ptr(data_offset);
        let bytes = self.read_bytes(start, len)?;
        let words = if levels == 0 {
            &[][..]
        } else {
            self.read_words(n)?
        };
        if levels != 0 && words.is_empty() {
            if !bytes.is_empty() {
                return Err(WireError::InvalidTopologyLen {
                    topo: 0,
                    data: bytes.len(),
                });
            }
            // SAFETY: no strings, no bytes: nothing to be UTF-8, and every
            // (empty) level's last boundary is `bytes.len()`, 0.
            return Ok(unsafe {
                StrNested::from_validated(
                    Topology {
                        topology: [&[]; MAX_TIERS],
                    },
                    bytes,
                )
            });
        }
        Topology::new_str_dynamic(start, words, bytes, levels)
    }
}
