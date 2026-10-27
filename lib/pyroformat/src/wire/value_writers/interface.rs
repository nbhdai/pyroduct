use std::marker::PhantomData;

use bitvec::{order::Lsb0, slice::BitSlice, vec::BitVec};
use half::f16;
use pyro_schema::{Inode, nullable_validity_start, write_word};

use crate::{buf::PyroSink, errors::WireError, schema::StructureWord, value::Time};

fn as_le_bytes<T: Copy>(slice: &[T]) -> &[u8] {
    // SAFETY: `T` is `Copy` (no Drop glue to worry about) and, per the
    // doc comment above, one of the wire format's plain fixed-width numeric
    // types with no padding bytes. Reading its bytes can never be invalid,
    // and on the little-endian-only targets this crate compiles for, those
    // bytes are exactly the wire encoding `to_le_bytes()` would produce.
    unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const u8, std::mem::size_of_val(slice)) }
}

/// Every scalar's `write_*`, run's `write_*_slice` and nullable run's
/// `write_*_option_slice`, for one fixed-width primitive: plain LE numerics
/// (or a newtype over one). `$to` encodes the scalar as LE bytes. A run's
/// values are padded to the type's size; a nullable run's present values
/// are followed by its validity bitmap.
macro_rules! primitive_writes {
    ($($write:ident, $slice:ident, $option_slice:ident: $ty:ty = $to:expr;)*) => {
        $(
            #[inline]
            pub fn $write(&mut self, n: usize, data: $ty) {
                let data_offset = self.data_offset(n);
                self.write_scalar::<{ std::mem::size_of::<$ty>() }>(data_offset, ($to)(data));
            }

            #[inline]
            pub fn $slice(&mut self, n: usize, data: &[Leaf<'_, $ty>]) {
                let data_offset = self.data_offset(n);
                let (start, count) = self.write_leaves(data);
                write_word::<W>(self.fixed, data_offset, start);
                write_word::<W>(self.fixed, data_offset + W::SIZE, count);
            }

            #[inline]
            pub fn $option_slice(
                &mut self,
                n: usize,
                data: &[Leaf<'_, $ty>],
                mask: &BitVec<u8>,
            ) {
                let data_offset = self.data_offset(n);
                let (start, present) = self.write_leaves(data);
                self.write_option_slot(data_offset, start, present, mask);
            }
        )*
    };
}

/// Every bare array's `write_fixed_*`, run's `write_fixed_*_slice` and
/// nullable run's `write_fixed_*_option_slice`, for arrays of one
/// fixed-width primitive (as [`primitive_writes`]). A bare array sits inline
/// at its data offset; in a run every [`FixedLeaf`] is one array, written
/// back to back, and the slot's count is the array count.
macro_rules! fixed_writes {
    ($($write:ident, $slice:ident, $option_slice:ident: $ty:ty;)*) => {
        $(
            #[inline]
            pub fn $write(&mut self, n: usize, data: &[$ty]) {
                let data_offset = self.data_offset(n);
                self.write_fixed_array::<$ty>(data_offset, data);
            }

            #[inline]
            pub fn $slice(&mut self, n: usize, data: &[FixedLeaf<'_, $ty>]) {
                let data_offset = self.data_offset(n);
                let (start, count) = self.write_fixed_leaves(data);
                write_word::<W>(self.fixed, data_offset, start);
                write_word::<W>(self.fixed, data_offset + W::SIZE, count);
            }

            #[inline]
            pub fn $option_slice(
                &mut self,
                n: usize,
                data: &[FixedLeaf<'_, $ty>],
                mask: &BitVec<u8>,
            ) {
                let data_offset = self.data_offset(n);
                let (start, present) = self.write_fixed_leaves(data);
                self.write_option_slot(data_offset, start, present, mask);
            }
        )*
    };
}

/// The value path's view of a [`PyroSink`] being written, for a layout only
/// known at runtime: every write takes its inode `n`, and finds its slots
/// through `inodes`. The fixed region starts zeroed (a null leaves its slot
/// and validity bit as they are); every byte appended to the variable
/// region, padding included, is written here.
pub struct SplitSink<'a, W: StructureWord> {
    inodes: &'a [Inode],
    fixed_len: usize,
    /// Where the fixed region's nullable bitmap starts.
    validity_start: usize,
    fixed: &'a mut [u8],
    variable: &'a mut [u8],
    written: usize,
    phantom: PhantomData<W>,
}

#[derive(Debug, Clone)]
pub struct TopologyCache<W: StructureWord> {
    /// Each row assumes they start at 0
    pub topology: [Vec<W>; 5],
}

#[derive(Debug, Clone, Copy)]
pub struct Padding {
    at: usize,
    total: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum Leaf<'b, T> {
    Slice(&'b [T]),
    Entry(T),
}

#[derive(Debug, Clone)]
pub enum FixedLeaf<'b, T> {
    Slice(&'b [T]),
    Entry(Vec<T>),
}

impl<'a, W: StructureWord> SplitSink<'a, W> {
    /// # Panics
    /// If `sink`'s fixed region is not `FIXED` bytes.
    #[inline]
    pub fn new<S: PyroSink>(
        sink: &'a mut S,
        inodes: &'a [Inode],
        fixed_len: usize,
    ) -> Result<Self, WireError> {
        // PyroView is guaranteed 16 byte aligned

        if sink.len() < fixed_len {
            return Err(WireError::UnderSized {
                expected: fixed_len,
                actual: sink.len(),
            });
        }
        let (fixed, variable) = sink.payload_mut().split_at_mut(fixed_len);
        Ok(Self {
            inodes,
            validity_start: nullable_validity_start::<W>(inodes),
            fixed: fixed.try_into().unwrap(),
            variable,
            fixed_len,
            written: 0,
            phantom: PhantomData,
        })
    }

    pub fn len(&self) -> usize {
        self.written + self.fixed_len
    }

    /// Inode `n`'s data slot in the fixed region.
    #[inline]
    fn data_offset(&self, n: usize) -> usize {
        self.inodes[n].data_offset() as usize
    }

    /// Inode `n`'s topology slot in the fixed region.
    #[inline]
    fn topology_offset(&self, n: usize) -> usize {
        self.inodes[n].topology_offset() as usize
    }

    /// Marks inode `n`'s nullable scalar present: sets its bit in the fixed
    /// region's bitmap (`Lsb0`). A null is never written; the bit stays as
    /// the zeroed fixed region left it.
    #[inline]
    pub fn write_valid(&mut self, n: usize) {
        let bit = self.inodes[n].nullable_bit_index() as usize;
        self.fixed[self.validity_start + bit / 8] |= 1 << (bit % 8);
    }

    #[inline]
    pub fn extend_zeroed(&mut self, count: usize) {
        self.variable[self.written..self.written + count].fill(0);
        self.written += count;
    }

    /// `bytes` into the fixed region at `data_offset`.
    #[inline]
    fn write_scalar<const SIZE: usize>(&mut self, data_offset: usize, bytes: [u8; SIZE]) {
        self.fixed[data_offset..data_offset + SIZE].copy_from_slice(&bytes);
    }

    /// `bytes`, appended to the variable region.
    #[inline]
    fn extend_from_slice(&mut self, bytes: &[u8]) {
        self.variable[self.written..self.written + bytes.len()].copy_from_slice(bytes);
        self.written += bytes.len();
    }

    /// Every leaf's `T`s back to back, padded to `T`'s size first. Returns
    /// where they start and how many there are. `T` as in [`as_le_bytes`].
    #[inline]
    fn write_leaves<T: Copy>(&mut self, data: &[Leaf<'_, T>]) -> (usize, usize) {
        self.extend_zeroed(self.len().next_multiple_of(std::mem::size_of::<T>()) - self.len());
        let start = self.len();
        let mut count = 0;
        for leaf in data {
            let values: &[T] = match leaf {
                Leaf::Slice(s) => s,
                Leaf::Entry(v) => std::slice::from_ref(v),
            };
            self.extend_from_slice(as_le_bytes(values));
            count += values.len();
        }
        (start, count)
    }

    /// `bits`' packed bytes (`Lsb0`), appended. Returns where they start
    /// and the bit count.
    #[inline]
    fn write_bits(&mut self, bits: &BitVec<u8, Lsb0>) -> (usize, usize) {
        let start = self.len();
        self.extend_from_slice(bits.as_raw_slice());
        (start, bits.len())
    }

    /// A bare array's `T`s, inline into the fixed region at `data_offset`.
    /// `T` as in [`as_le_bytes`].
    #[inline]
    fn write_fixed_array<T: Copy>(&mut self, data_offset: usize, data: &[T]) {
        let bytes = as_le_bytes(data);
        self.fixed[data_offset..data_offset + bytes.len()].copy_from_slice(bytes);
    }

    /// Every array back to back, padded to `T`'s size first. Returns where
    /// they start and the array count. `T` as in [`as_le_bytes`].
    #[inline]
    fn write_fixed_leaves<T: Copy>(&mut self, data: &[FixedLeaf<'_, T>]) -> (usize, usize) {
        self.extend_zeroed(self.len().next_multiple_of(std::mem::size_of::<T>()) - self.len());
        let start = self.len();
        for leaf in data {
            match leaf {
                FixedLeaf::Slice(s) => self.extend_from_slice(as_le_bytes(s)),
                FixedLeaf::Entry(v) => self.extend_from_slice(as_le_bytes(v)),
            }
        }
        (start, data.len())
    }

    /// The validity bitmap (`Lsb0`, one bit per sample) after a nullable
    /// run's present values, and the run's `(start, present, total)` slot.
    #[inline]
    fn write_option_slot(
        &mut self,
        data_offset: usize,
        start: usize,
        present: usize,
        mask: &BitVec<u8>,
    ) {
        debug_assert_eq!(mask.count_ones(), present, "one set bit per present value");
        self.extend_from_slice(mask.as_raw_slice());
        write_word::<W>(self.fixed, data_offset, start);
        write_word::<W>(self.fixed, data_offset + W::SIZE, present);
        write_word::<W>(self.fixed, data_offset + 2 * W::SIZE, mask.len());
    }

    primitive_writes! {
        write_u8, write_u8_slice, write_u8_option_slice: u8 = u8::to_le_bytes;
        write_i8, write_i8_slice, write_i8_option_slice: i8 = i8::to_le_bytes;
        write_u16, write_u16_slice, write_u16_option_slice: u16 = u16::to_le_bytes;
        write_i16, write_i16_slice, write_i16_option_slice: i16 = i16::to_le_bytes;
        write_u32, write_u32_slice, write_u32_option_slice: u32 = u32::to_le_bytes;
        write_i32, write_i32_slice, write_i32_option_slice: i32 = i32::to_le_bytes;
        write_u64, write_u64_slice, write_u64_option_slice: u64 = u64::to_le_bytes;
        write_i64, write_i64_slice, write_i64_option_slice: i64 = i64::to_le_bytes;
        write_f16, write_f16_slice, write_f16_option_slice: f16 = f16::to_le_bytes;
        write_f32, write_f32_slice, write_f32_option_slice: f32 = f32::to_le_bytes;
        write_f64, write_f64_slice, write_f64_option_slice: f64 = f64::to_le_bytes;
        write_time, write_time_slice, write_time_option_slice: Time = |t: Time| t.0.to_le_bytes();
    }

    fixed_writes! {
        write_fixed_u8, write_fixed_u8_slice, write_fixed_u8_option_slice: u8;
        write_fixed_i8, write_fixed_i8_slice, write_fixed_i8_option_slice: i8;
        write_fixed_u16, write_fixed_u16_slice, write_fixed_u16_option_slice: u16;
        write_fixed_i16, write_fixed_i16_slice, write_fixed_i16_option_slice: i16;
        write_fixed_u32, write_fixed_u32_slice, write_fixed_u32_option_slice: u32;
        write_fixed_i32, write_fixed_i32_slice, write_fixed_i32_option_slice: i32;
        write_fixed_u64, write_fixed_u64_slice, write_fixed_u64_option_slice: u64;
        write_fixed_i64, write_fixed_i64_slice, write_fixed_i64_option_slice: i64;
        write_fixed_f16, write_fixed_f16_slice, write_fixed_f16_option_slice: f16;
        write_fixed_f32, write_fixed_f32_slice, write_fixed_f32_option_slice: f32;
        write_fixed_f64, write_fixed_f64_slice, write_fixed_f64_option_slice: f64;
        write_fixed_time, write_fixed_time_slice, write_fixed_time_option_slice: Time;
    }

    // Blobs: a bare string or byte blob is appended as-is, and its slot is
    // `(start, byte count)`.

    /// Inode `n`'s bare blob: its bytes appended, and the `(start, byte
    /// count)` slot pointing at them.
    #[inline]
    pub fn write_bytes(&mut self, n: usize, data: &[u8]) {
        let data_offset = self.data_offset(n);
        let start = self.len();
        self.extend_from_slice(data);
        write_word::<W>(self.fixed, data_offset, start);
        write_word::<W>(self.fixed, data_offset + W::SIZE, data.len());
    }

    /// Inode `n`'s bare string, as [`Self::write_bytes`].
    #[inline]
    pub fn write_str(&mut self, n: usize, data: &str) {
        self.write_bytes(n, data.as_bytes());
    }

    // Bools: a scalar is one byte (0x00 = false), a run is bit-packed
    // (`Lsb0`) and its slot's count is the bit count.

    #[inline]
    pub fn write_bool(&mut self, n: usize, data: bool) {
        let data_offset = self.data_offset(n);
        self.write_scalar::<1>(data_offset, [data as u8]);
    }

    #[inline]
    pub fn write_bool_slice(&mut self, n: usize, data: &BitVec<u8, Lsb0>) {
        let data_offset = self.data_offset(n);
        let (start, count) = self.write_bits(data);
        write_word::<W>(self.fixed, data_offset, start);
        write_word::<W>(self.fixed, data_offset + W::SIZE, count);
    }

    #[inline]
    pub fn write_bool_option_slice(
        &mut self,
        n: usize,
        data: &BitVec<u8, Lsb0>,
        mask: &BitVec<u8>,
    ) {
        let data_offset = self.data_offset(n);
        let (start, present) = self.write_bits(data);
        self.write_option_slot(data_offset, start, present, mask);
    }

    // Fixed bools: a bare `[bool; n]` is its `n` bits packed inline at its
    // data offset; a run is `arrays * n` bits back to back (`Lsb0`, arrays
    // not byte-aligned), and its slot's count is the array count.

    /// Inode `n`'s bare `[bool; len]`: its bits, packed inline into its slot
    /// in the fixed region. `data` need not start on a byte boundary.
    #[inline]
    pub fn write_fixed_bool(&mut self, n: usize, data: &BitSlice<u8, Lsb0>) {
        let data_offset = self.data_offset(n);
        let len = data.len();
        let dest = &mut self.fixed[data_offset..data_offset + len.div_ceil(8)];
        BitSlice::<u8, Lsb0>::from_slice_mut(dest)[..len].copy_from_bitslice(data);
    }

    /// A run of `[bool; len]`: every array's bits back to back in `data`.
    #[inline]
    pub fn write_fixed_bool_slice(&mut self, n: usize, len: usize, data: &BitVec<u8, Lsb0>) {
        let data_offset = self.data_offset(n);
        debug_assert_eq!(data.len() % len, 0, "whole arrays of {len} bits");
        let (start, bits) = self.write_bits(data);
        write_word::<W>(self.fixed, data_offset, start);
        write_word::<W>(self.fixed, data_offset + W::SIZE, bits / len);
    }

    /// A nullable run of `[bool; len]`: the present arrays' bits back to back
    /// in `data`, then `mask`, one bit per sample.
    #[inline]
    pub fn write_fixed_bool_option_slice(
        &mut self,
        n: usize,
        len: usize,
        data: &BitVec<u8, Lsb0>,
        mask: &BitVec<u8>,
    ) {
        let data_offset = self.data_offset(n);
        debug_assert_eq!(data.len() % len, 0, "whole arrays of {len} bits");
        let (start, bits) = self.write_bits(data);
        self.write_option_slot(data_offset, start, bits / len, mask);
    }

    /// `List<List<Group>>`, a `List<Group>` nested in another list, group
    /// or map, and every other leaf under levels: the sentinel-prefixed
    /// levels, padded to a word first, and inode `n`'s `(start, words)` slot.
    pub fn write_topology(&mut self, n: usize, data: &TopologyCache<W>) {
        let topology_offset = self.topology_offset(n);
        self.extend_zeroed(self.len().next_multiple_of(W::SIZE) - self.len());
        let start = self.len();
        let mut count = 0usize;
        for level in &data.topology {
            if !level.is_empty() {
                W::RESET.write(self.variable, self.written);
                self.written += W::SIZE;
                self.extend_from_slice(as_le_bytes(level));
                count += level.len() + 1;
            }
        }
        write_word::<W>(self.fixed, topology_offset, start);
        write_word::<W>(self.fixed, topology_offset + W::SIZE, count);
    }

    /// `List<Group>`: inode `n`'s row count, the one word in its slot. A
    /// bare root map's entry count takes the same slot.
    #[inline]
    pub fn write_count(&mut self, n: usize, count: usize) {
        let topology_offset = self.topology_offset(n);
        write_word::<W>(self.fixed, topology_offset, count);
    }

    /// `List<Option<Group>>`: the validity mask (one bit per group, nulls
    /// included), padded to a word first, and inode `n`'s
    /// `(mask start, groups)` slot.
    pub fn write_group_mask(&mut self, n: usize, mask: &BitVec<u8>) {
        let topology_offset = self.topology_offset(n);
        self.extend_zeroed(self.len().next_multiple_of(W::SIZE) - self.len());
        let start = self.len();
        self.extend_from_slice(mask.as_raw_slice());
        write_word::<W>(self.fixed, topology_offset, start);
        write_word::<W>(self.fixed, topology_offset + W::SIZE, mask.len());
    }

    /// Nullable groups under levels: [`Self::write_topology`]'s levels, then
    /// the validity mask over the last level's total right behind them.
    pub fn write_topology_option(&mut self, n: usize, data: &TopologyCache<W>, mask: &BitVec<u8>) {
        let topology_offset = self.topology_offset(n);
        self.extend_zeroed(self.len().next_multiple_of(W::SIZE) - self.len());
        let start = self.len();
        let mut count = 0usize;
        for level in &data.topology {
            if !level.is_empty() {
                W::RESET.write(self.variable, self.written);
                self.written += W::SIZE;
                self.extend_from_slice(as_le_bytes(level));
                count += level.len() + 1;
            }
        }
        self.extend_from_slice(mask.as_raw_slice());
        write_word::<W>(self.fixed, topology_offset, start);
        write_word::<W>(self.fixed, topology_offset + W::SIZE, count);
    }
}

impl<W: StructureWord> TopologyCache<W> {
    pub fn new() -> Self {
        TopologyCache {
            topology: std::array::from_fn(|_| Vec::new()),
        }
    }

    /// Advance `level`'s cumulative boundary by `n` — one call per element
    /// ingested at that level, in order.
    pub fn bound(&mut self, level: usize, n: usize) {
        let last = self.topology[level].last().map_or(0, |w| w.to_usize());
        self.topology[level].push(W::from_usize(last + n));
    }

    /// Every level's cumulative boundaries so far, without the leading
    /// sentinel — `levels()[..depth]` for a cache constructed with fewer
    /// than `N` levels, since the rest just stay empty and unused.
    pub fn levels(&self) -> &[Vec<W>] {
        &self.topology
    }

    /// Bytes [`Self::write`] will append: every non-empty level's sentinel
    /// plus its cumulative counts.
    pub fn variable_length(&self) -> usize {
        self.topology
            .iter()
            .filter(|l| !l.is_empty())
            .map(|l| (l.len() + 1) * W::SIZE)
            .sum()
    }

    /// [`Self::write_levels`]'s appends, dry: pad to `W::SIZE`, then the words.
    pub fn padding_levels(&self, padding: &mut Padding) {
        padding.align(W::SIZE);
        padding.skip(self.variable_length());
    }

    pub fn clear(&mut self) {
        for t in &mut self.topology {
            t.clear();
        }
    }
}

impl Padding {
    pub fn new(at: usize) -> Self {
        Self { at, total: 0 }
    }

    /// Mirror of `extend_zeroed(len.next_multiple_of(align) - len)`.
    #[inline]
    pub fn align(&mut self, align: usize) {
        let pad = self.at.next_multiple_of(align) - self.at;
        self.at += pad;
        self.total += pad;
    }

    /// Mirror of appending `n` payload bytes.
    #[inline]
    pub fn skip(&mut self, n: usize) {
        self.at += n;
    }

    #[inline]
    pub fn total(&self) -> usize {
        self.total
    }
}
