//! Every wire shape a `bool` can take. Bools align to one byte, so
//! everything is written in the 1-byte lane and every other lane writes
//! nothing.
//!
//! * A scalar is one byte in the fixed region (`0x00` = false); a nullable
//!   one also sets its bit in the fixed region's bitmap.
//! * A run is bit-packed, least significant bit first: the `(offset, bit
//!   count)` slot addresses the bits. Runs at the bottom of a nesting are one
//!   run, back to back, with no padding between them.
//! * A nullable run is the present values bit-packed, then one validity bit
//!   per sample (nulls included) starting on the next whole byte, under the
//!   `(offset, present, total)` slot.
//!
//! * A `[bool; N]` is one sample of `N` bits: bare, its bits are packed
//!   inline at the data offset; in a run, every array's bits are back to back
//!   with no padding between arrays, and the slots count arrays. A nullable
//!   run packs only the present arrays' bits, then one validity bit per
//!   array.
//!
//! Every container of these, at any depth, is handled by the blanket impls in
//! `nesting.rs`.

use bitvec::{order::Lsb0, slice::BitSlice};

use crate::{
    Layout,
    schema::{StructureWord, write_word},
    wire::writers::{InodeOffset, PreBuffer, ValidityWriter, Writer, nesting::Nest},
};

impl<'a, Inodes: Layout, const FIXED: usize, const INODE_LEN: usize>
    PreBuffer<'a, Inodes, FIXED, INODE_LEN>
{
    /// Appends one bit to the leaf's packed run, at `data`'s byte and
    /// `data_bit`'s bit.
    ///
    /// A fresh byte is overwritten whole and a partial one keeps its low bits, so
    /// the buffer needn't start zeroed.
    #[inline]
    fn push_bit<O: InodeOffset>(&mut self, value: bool) {
        let size = &mut self.sizes[O::N];
        let at = size.data.to_usize() - FIXED;
        let byte = if size.data_bit == 0 {
            0
        } else {
            self.variable[at]
        };
        self.variable[at] = byte | ((value as u8) << size.data_bit);
        size.data_bit += 1;
        if size.data_bit == 8 {
            size.data_bit = 0;
            size.data.add_usize(1);
        }
    }

    /// Appends to the leaf's packed run in bulk, through the validity
    /// pushers: `data`'s byte and `data_bit`'s bit stand in for the inode's
    /// [`ValidityWriter`] while `write` runs, then move past what it pushed.
    ///
    /// Both keep a partial byte already in the buffer, so it is picked up from
    /// there.
    #[inline]
    fn push_data_bits<O: InodeOffset>(&mut self, write: impl FnOnce(&mut Self)) {
        let size = &self.sizes[O::N];
        let byte = if size.data_bit == 0 {
            0
        } else {
            self.variable[size.data.to_usize() - FIXED]
        };
        let bits = ValidityWriter {
            start: size.data,
            byte,
            bit: size.data_bit,
        };
        let validity = std::mem::replace(&mut self.sizes[O::N].validity, bits);
        write(self);
        let bits = std::mem::replace(&mut self.sizes[O::N].validity, validity);
        self.sizes[O::N].data = bits.start;
        self.sizes[O::N].data_bit = bits.bit;
    }

    /// Appends every bit of `bits` to the leaf's packed run, in order, as
    /// [`Self::extend_validity`]: since its copy stops at the last whole byte,
    /// whatever a `BitVec` keeps in its last byte past its length never reaches
    /// the wire.
    #[inline]
    fn push_bits<O: InodeOffset>(&mut self, bits: &BitSlice<u8, Lsb0>) {
        self.push_data_bits::<O>(|buffer| buffer.extend_validity::<O>(bits));
    }

    /// Lays out this inode's packed run and writes its `(start, sample count)`
    /// slot, each sample `BITS` bits: `1` for a `bool`, `N` for a `[bool; N]`.
    /// Afterwards `data` is the offset of the first byte, at bit `0`.
    #[inline]
    fn write_bit_run_slot<const BITS: usize, O: InodeOffset>(
        &mut self,
        byte_offset: usize,
    ) -> usize {
        let inode = Inodes::INODES[O::N];
        let offset = inode.data_offset() as usize;
        let count = self.sizes[O::N].data.to_usize();
        write_word::<Inodes::Word>(self.fixed, offset, byte_offset);
        write_word::<Inodes::Word>(self.fixed, offset + Inodes::Word::SIZE, count);

        // Leave where we should start writing.
        self.sizes[O::N].data = Inodes::Word::from_usize(byte_offset);
        self.sizes[O::N].data_bit = 0;
        byte_offset + (count * BITS).div_ceil(8)
    }

    /// Lays out this inode's nullable packed run and writes its `(start, valid,
    /// count)` slot: the present samples' bits, `BITS` each, then the mask from
    /// the next whole byte, one bit per slot. Afterwards `data` is the offset of the first value
    /// byte and `validity.start` the offset of the mask. The mask is not padded;
    /// the next writer pads to its own alignment.
    #[inline]
    fn write_option_bit_run_slot<const BITS: usize, O: InodeOffset>(
        &mut self,
        byte_offset: usize,
    ) -> usize {
        let inode = Inodes::INODES[O::N];
        let offset = inode.data_offset() as usize;
        let count = self.sizes[O::N].data.to_usize();
        let valid = self.sizes[O::N].validity.start.to_usize();
        write_word::<Inodes::Word>(self.fixed, offset, byte_offset);
        write_word::<Inodes::Word>(self.fixed, offset + Inodes::Word::SIZE, valid);
        write_word::<Inodes::Word>(self.fixed, offset + 2 * Inodes::Word::SIZE, count);

        // Leave where we should start writing.
        let mask_start = byte_offset + (valid * BITS).div_ceil(8);
        self.sizes[O::N].data = Inodes::Word::from_usize(byte_offset);
        self.sizes[O::N].data_bit = 0;
        self.sizes[O::N].validity.start = Inodes::Word::from_usize(mask_start);
        mask_start + count.div_ceil(8)
    }

    /// Appends one nullable sample: its bit to the values if present, and its
    /// validity bit either way.
    #[inline]
    fn push_option_bit<O: InodeOffset>(&mut self, value: Option<bool>) {
        if let Some(value) = value {
            self.push_bit::<O>(value);
        }
        self.push_validity_bit::<O>(value.is_some());
    }

    /// A bare array's bits, packed LSB first into `N.div_ceil(8)` bytes at the
    /// data offset.
    #[inline]
    fn write_packed_array<const N: usize, O: InodeOffset>(&mut self, value: &[bool; N]) {
        let offset = Inodes::INODES[O::N].data_offset() as usize;
        for (i, byte) in self.fixed[offset..offset + N.div_ceil(8)]
            .iter_mut()
            .enumerate()
        {
            *byte = value[i * 8..N.min(i * 8 + 8)]
                .iter()
                .enumerate()
                .fold(0, |byte, (bit, value)| byte | ((*value as u8) << bit));
        }
    }

    /// Appends a run of arrays' bits, back to back.
    #[inline]
    fn push_bit_arrays<const N: usize, O: InodeOffset>(&mut self, arrays: &[[bool; N]]) {
        self.push_data_bits::<O>(|buffer| buffer.extend_validity_slice::<O>(arrays.as_flattened()));
    }

    /// Appends a nullable run: the present arrays' bits, and a bit per array to
    /// the mask.
    #[inline]
    fn push_option_bit_arrays<const N: usize, O: InodeOffset>(
        &mut self,
        arrays: &[Option<[bool; N]>],
    ) {
        self.push_data_bits::<O>(|buffer| {
            buffer.extend_validity_iter::<O>(arrays.iter().flatten().flatten().copied())
        });
        self.extend_validity_iter::<O>(arrays.iter().map(Option::is_some));
    }
}

impl Writer for bool {
    #[inline]
    fn foreign_plan<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        // Fixed region only.
        0
    }

    #[inline]
    fn write_structure_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_16_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_8_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_4_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_2_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_1_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        let offset = Inodes::INODES[O::N].data_offset() as usize;
        buffer.fixed[offset] = *self as u8;
        byte_offset
    }

    #[inline]
    fn write_variable<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) {
    }

    #[inline]
    fn iter_foreign_plan<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        buffer.plan_data::<O>(1);
        // A whole byte for its bit.
        1
    }

    #[inline]
    fn iter_write_structure_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn iter_write_16_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn iter_write_8_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn iter_write_4_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn iter_write_2_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn iter_write_1_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        // The iterated scalar is a run: same slot as `Vec<bool>`.
        buffer.write_bit_run_slot::<1, O>(byte_offset)
    }

    #[inline]
    fn iter_write_variable<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) {
        buffer.push_bit::<O>(*self);
    }
}

impl Writer for Option<bool> {
    #[inline]
    fn foreign_plan<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        // Fixed region only.
        0
    }

    #[inline]
    fn write_structure_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_16_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_8_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_4_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_2_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_1_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        let inode = Inodes::INODES[O::N];
        // A null leaves the slot's byte as the layout zeroed it.
        if let Some(value) = self {
            buffer.fixed[inode.data_offset() as usize] = *value as u8;
        }
        buffer.set_validity_bit::<O>(self.is_some());
        byte_offset
    }

    #[inline]
    fn write_variable<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) {
    }

    #[inline]
    fn iter_foreign_plan<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        let size = &mut buffer.sizes[O::N];
        size.data.add_planned(1);
        if self.is_some() {
            size.validity.start.add_planned(1);
        }
        2
    }

    #[inline]
    fn iter_write_structure_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn iter_write_16_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn iter_write_8_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn iter_write_4_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn iter_write_2_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn iter_write_1_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        // The iterated nullable scalar is a nullable run: same slot as
        // `Vec<Option<bool>>`.
        buffer.write_option_bit_run_slot::<1, O>(byte_offset)
    }

    #[inline]
    fn iter_write_variable<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) {
        buffer.push_option_bit::<O>(*self);
    }
}

// ---- leaves ------------------------------------------------------------------

impl Nest for bool {
    const INODE_COUNT: usize = 1;
    const DEPTH: usize = 0;

    #[inline]
    fn plan_slice<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        items: &[Self],
        _level: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        buffer.plan_data::<O>(items.len());
        items.len().div_ceil(8)
    }

    #[inline]
    fn write_slice<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        items: &[Self],
        _level: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) {
        buffer.push_data_bits::<O>(|buffer| buffer.extend_validity_slice::<O>(items));
    }

    #[inline]
    fn write_structure_slot<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_16_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_8_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_4_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_2_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_1_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        buffer.write_bit_run_slot::<1, O>(byte_offset)
    }
}

impl Nest for Option<bool> {
    const INODE_COUNT: usize = 1;
    const DEPTH: usize = 0;

    #[inline]
    fn plan_slice<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        items: &[Self],
        _level: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        let size = &mut buffer.sizes[O::N];
        size.data.add_planned(items.len());
        // Present values never outnumber the slots, which are counted above.
        size.validity
            .start
            .add_planned(items.iter().filter(|v| v.is_some()).count());
        2 * items.len().div_ceil(8)
    }

    #[inline]
    fn write_slice<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        items: &[Self],
        _level: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) {
        buffer.push_data_bits::<O>(|buffer| {
            buffer.extend_validity_iter::<O>(items.iter().flatten().copied())
        });
        buffer.extend_validity_iter::<O>(items.iter().map(Option::is_some));
    }

    #[inline]
    fn write_structure_slot<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_16_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_8_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_4_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_2_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_1_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        buffer.write_option_bit_run_slot::<1, O>(byte_offset)
    }
}

// ---- fixed-size arrays ---------------------------------------------------------

impl<const N: usize> Writer for [bool; N] {
    #[inline]
    fn foreign_plan<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        // Fixed region only.
        0
    }

    #[inline]
    fn write_structure_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_16_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_8_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_4_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_2_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_1_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        buffer.write_packed_array::<N, O>(self);
        byte_offset
    }

    #[inline]
    fn write_variable<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) {
    }

    #[inline]
    fn iter_foreign_plan<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        buffer.plan_data::<O>(1);
        N.div_ceil(8)
    }

    #[inline]
    fn iter_write_structure_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn iter_write_16_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn iter_write_8_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn iter_write_4_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn iter_write_2_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn iter_write_1_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        // The iterated array is a run: same slot as `Vec<[bool; N]>`.
        buffer.write_bit_run_slot::<N, O>(byte_offset)
    }

    #[inline]
    fn iter_write_variable<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) {
        buffer.push_bit_arrays::<N, O>(std::slice::from_ref(self));
    }
}

impl<const N: usize> Writer for Option<[bool; N]> {
    #[inline]
    fn foreign_plan<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        // Fixed region only.
        0
    }

    #[inline]
    fn write_structure_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_16_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_8_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_4_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_2_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_1_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        // A null leaves the slot's bytes as the layout zeroed them.
        if let Some(value) = self {
            buffer.write_packed_array::<N, O>(value);
        }
        buffer.set_validity_bit::<O>(self.is_some());
        byte_offset
    }

    #[inline]
    fn write_variable<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) {
    }

    #[inline]
    fn iter_foreign_plan<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        let size = &mut buffer.sizes[O::N];
        size.data.add_planned(1);
        if self.is_some() {
            size.validity.start.add_planned(1);
        }
        // Its bits, and a whole byte of mask for its bit.
        N.div_ceil(8) + 1
    }

    #[inline]
    fn iter_write_structure_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn iter_write_16_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn iter_write_8_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn iter_write_4_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn iter_write_2_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn iter_write_1_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        // The iterated nullable array is a nullable run: same slot as
        // `Vec<Option<[bool; N]>>`.
        buffer.write_option_bit_run_slot::<N, O>(byte_offset)
    }

    #[inline]
    fn iter_write_variable<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) {
        buffer.push_option_bit_arrays::<N, O>(std::slice::from_ref(self));
    }
}

impl<const N: usize> Nest for [bool; N] {
    const INODE_COUNT: usize = 1;
    const DEPTH: usize = 0;

    #[inline]
    fn plan_slice<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        items: &[Self],
        _level: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        buffer.plan_data::<O>(items.len());
        items.len() * N.div_ceil(8)
    }

    #[inline]
    fn write_slice<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        items: &[Self],
        _level: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) {
        buffer.push_bit_arrays::<N, O>(items);
    }

    #[inline]
    fn write_structure_slot<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_16_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_8_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_4_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_2_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_1_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        buffer.write_bit_run_slot::<N, O>(byte_offset)
    }
}

impl<const N: usize> Nest for Option<[bool; N]> {
    const INODE_COUNT: usize = 1;
    const DEPTH: usize = 0;

    #[inline]
    fn plan_slice<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        items: &[Self],
        _level: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        let size = &mut buffer.sizes[O::N];
        size.data.add_planned(items.len());
        // Present arrays never outnumber the slots, which are counted above.
        size.validity
            .start
            .add_planned(items.iter().filter(|v| v.is_some()).count());
        items.len() * (N.div_ceil(8) + 1)
    }

    #[inline]
    fn write_slice<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        items: &[Self],
        _level: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) {
        buffer.push_option_bit_arrays::<N, O>(items);
    }

    #[inline]
    fn write_structure_slot<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_16_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_8_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_4_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_2_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_1_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        buffer.write_option_bit_run_slot::<N, O>(byte_offset)
    }
}

// ---- `pyroformat::BitVec` ----------------------------------------------------
//
// A `BitVec` is a `Vec<bool>` already packed: the same wire shape, one level
// over `bool` leaves, but its run is a `BitSlice` rather than a `&[bool]`, so
// it can't go through `nesting.rs`'s `Run`-based containers. These are those
// containers' impls with `bool`'s leaf methods inlined, and `push_bits` in
// place of a `write_slice` over the items.

impl Nest for crate::BitVec {
    const INODE_COUNT: usize = 1;
    const DEPTH: usize = 1;

    #[inline]
    fn plan_slice<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        items: &[Self],
        level: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        buffer.sizes[O::N].topo[level].add_planned(items.len());
        let mut bytes = items.len() * Inodes::Word::SIZE;
        for item in items {
            buffer.plan_data::<O>(item.len());
            bytes += item.len().div_ceil(8);
        }
        bytes
    }

    #[inline]
    fn write_slice<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        items: &[Self],
        level: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) {
        for item in items {
            buffer.push_topology_word::<O>(level, item.len());
            buffer.push_bits::<O>(item);
        }
    }

    #[inline]
    fn write_structure_slot<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_16_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_8_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_4_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_2_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_1_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        buffer.write_bit_run_slot::<1, O>(byte_offset)
    }
}

/// As a single value the bits are the run itself, with no tiers; iterated,
/// each value is one node of tier `0` and its bits sit below it.
impl Writer for crate::BitVec {
    #[inline]
    fn foreign_plan<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        buffer.plan_data::<O>(self.len());
        // No tiers as a single value, and bits have no alignment.
        self.len().div_ceil(8)
    }

    #[inline]
    fn write_structure_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_16_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_8_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_4_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_2_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn write_1_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        buffer.write_bit_run_slot::<1, O>(byte_offset)
    }

    #[inline]
    fn write_variable<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) {
        buffer.push_bits::<O>(self);
    }

    #[inline]
    fn iter_foreign_plan<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        buffer.sizes[O::N].topo[0].add_planned(1);
        buffer.plan_data::<O>(self.len());
        Inodes::Word::SIZE + self.len().div_ceil(8)
    }

    #[inline]
    fn iter_write_structure_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        buffer.write_topology_structure::<O>(1, byte_offset)
    }

    #[inline]
    fn iter_write_16_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn iter_write_8_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn iter_write_4_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn iter_write_2_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset
    }

    #[inline]
    fn iter_write_1_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        buffer.write_bit_run_slot::<1, O>(byte_offset)
    }

    #[inline]
    fn iter_write_variable<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) {
        buffer.push_topology_word::<O>(0, self.len());
        buffer.push_bits::<O>(self);
    }
}
