//! Every wire shape a fixed-size array of primitives, `[P; N]`, can take.
//!
//! An array is one sample, not `N` of them: slots, tiers and counts are in
//! arrays, and only the payload is in elements. `N` is fixed by the type, so
//! nothing on the wire records it.
//!
//! * A **bare** array sits whole in the fixed region; a nullable one also sets
//!   its bit in the fixed region's bitmap.
//! * A **run** of arrays is every array's elements back to back under the
//!   `(offset, array count)` slot. Iterated, a bare array is a run.
//! * A **nullable run** is the present arrays' elements, then one validity bit
//!   per array, under the `(offset, present, total)` slot. The option is on
//!   the array, never on its elements.
//!
//! Each array is written in its element's lane. Every container of these, at
//! any depth, is handled by the blanket impls in `nesting.rs`. Bool arrays
//! are bit-packed and live in `bool.rs`.

use half::f16;
use std::mem::{align_of, size_of};

use crate::{
    Layout,
    schema::{StructureWord, write_word},
    value::Time,
    wire::writers::{InodeOffset, PreBuffer, Writer, nesting::Nest},
};

impl<'a, Inodes: Layout, const FIXED: usize, const INODE_LEN: usize>
    PreBuffer<'a, Inodes, FIXED, INODE_LEN>
{
    /// A bare array's elements in its slot in the fixed region, if this is its
    /// element's lane.
    #[inline]
    fn array_lane<T: Sized + Copy, const N: usize, const LANE: usize, O: InodeOffset>(
        &mut self,
        value: &[T; N],
        byte_offset: usize,
    ) -> usize {
        if align_of::<T>() == LANE {
            let offset = Inodes::INODES[O::N].data_offset() as usize;
            self.fixed[offset..offset + N * size_of::<T>()]
                .copy_from_slice(Self::as_le_bytes(value));
        }
        byte_offset
    }

    /// Same, nullable: the elements if present, and the array's bit in the fixed
    /// region's bitmap either way.
    #[inline]
    fn option_array_lane<T: Sized + Copy, const N: usize, const LANE: usize, O: InodeOffset>(
        &mut self,
        value: &Option<[T; N]>,
        byte_offset: usize,
    ) -> usize {
        if align_of::<T>() == LANE {
            let inode = Inodes::INODES[O::N];
            // A null leaves the slot's bytes as the layout zeroed them.
            if let Some(value) = value {
                let offset = inode.data_offset() as usize;
                self.fixed[offset..offset + N * size_of::<T>()]
                    .copy_from_slice(Self::as_le_bytes(value));
            }
            self.set_validity_bit::<O>(value.is_some());
        }
        byte_offset
    }

    /// Lays out a run of arrays and writes its `(start, array count)` slot, if
    /// this is the element's lane. Afterwards `data` is the offset of the first
    /// element.
    #[inline]
    fn array_run_lane<T: Sized + Copy, const N: usize, const LANE: usize, O: InodeOffset>(
        &mut self,
        byte_offset: usize,
    ) -> usize {
        if align_of::<T>() != LANE {
            return byte_offset;
        }
        let byte_offset = self.pad_to(byte_offset, align_of::<T>());
        let offset = Inodes::INODES[O::N].data_offset() as usize;
        let count = self.sizes[O::N].data.to_usize();
        write_word::<Inodes::Word>(self.fixed, offset, byte_offset);
        write_word::<Inodes::Word>(self.fixed, offset + Inodes::Word::SIZE, count);

        // Leave where we should start writing.
        self.sizes[O::N].data = Inodes::Word::from_usize(byte_offset);
        byte_offset + count * N * size_of::<T>()
    }

    /// Same, nullable: the present arrays' elements, then the mask, one bit per
    /// array, under the `(start, present, total)` slot. Afterwards `data` is the
    /// offset of the first element and `validity.start` the offset of the mask.
    /// The mask is not padded; the next writer pads to its own alignment.
    #[inline]
    fn option_array_run_lane<T: Sized + Copy, const N: usize, const LANE: usize, O: InodeOffset>(
        &mut self,
        byte_offset: usize,
    ) -> usize {
        if align_of::<T>() != LANE {
            return byte_offset;
        }
        let byte_offset = self.pad_to(byte_offset, align_of::<T>());
        let offset = Inodes::INODES[O::N].data_offset() as usize;
        let count = self.sizes[O::N].data.to_usize();
        let valid = self.sizes[O::N].validity.start.to_usize();
        write_word::<Inodes::Word>(self.fixed, offset, byte_offset);
        write_word::<Inodes::Word>(self.fixed, offset + Inodes::Word::SIZE, valid);
        write_word::<Inodes::Word>(self.fixed, offset + 2 * Inodes::Word::SIZE, count);

        // Leave where we should start writing.
        let mask_start = byte_offset + valid * N * align_of::<T>();
        self.sizes[O::N].data = Inodes::Word::from_usize(byte_offset);
        self.sizes[O::N].validity.start = Inodes::Word::from_usize(mask_start);
        mask_start + count.div_ceil(8)
    }

    /// Appends a run of arrays: every element, back to back, in one copy.
    #[inline]
    fn push_arrays<T: Sized + Copy, const N: usize, O: InodeOffset>(&mut self, arrays: &[[T; N]]) {
        let bytes = Self::as_le_bytes(arrays.as_flattened());
        let offset = self.sizes[O::N].data.to_usize() - FIXED;
        self.variable[offset..offset + bytes.len()].copy_from_slice(bytes);
        // Increment the sizes incase we need to do this more.
        self.sizes[O::N].data.add_usize(bytes.len());
    }

    /// Appends a nullable run: the present arrays' elements, and a bit per array
    /// to the mask.
    #[inline]
    fn push_option_arrays<T: Sized + Copy, const N: usize, O: InodeOffset>(
        &mut self,
        arrays: &[Option<[T; N]>],
    ) {
        for array in arrays.iter().flatten() {
            let bytes = Self::as_le_bytes(array);
            let offset = self.sizes[O::N].data.to_usize() - FIXED;
            self.variable[offset..offset + bytes.len()].copy_from_slice(bytes);
            self.sizes[O::N].data.add_usize(bytes.len());
        }
        self.extend_validity_iter::<O>(arrays.iter().map(Option::is_some));
    }
}
// ---- one set of impls per element type ----------------------------------------

/// `[$ty; N]`'s and `Option<[$ty; N]>`'s [`Writer`] and leaf [`Nest`],
/// generic over `N` only: the blankets are the derived groups'.
macro_rules! fixed_writer {
    ($ty:ty) => {
        impl<const N: usize> Writer for [$ty; N] {
            #[inline]
            fn foreign_plan<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
                &self,
                _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                // A bare array fills its slot in the fixed region and nothing more.
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
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.array_lane::<$ty, N, 16, O>(self, byte_offset)
            }

            #[inline]
            fn write_8_fixed<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
                &self,
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.array_lane::<$ty, N, 8, O>(self, byte_offset)
            }

            #[inline]
            fn write_4_fixed<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
                &self,
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.array_lane::<$ty, N, 4, O>(self, byte_offset)
            }

            #[inline]
            fn write_2_fixed<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
                &self,
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.array_lane::<$ty, N, 2, O>(self, byte_offset)
            }

            #[inline]
            fn write_1_fixed<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
                &self,
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.array_lane::<$ty, N, 1, O>(self, byte_offset)
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
                N * size_of::<$ty>()
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
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.array_run_lane::<$ty, N, 16, O>(byte_offset)
            }

            #[inline]
            fn iter_write_8_fixed<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.array_run_lane::<$ty, N, 8, O>(byte_offset)
            }

            #[inline]
            fn iter_write_4_fixed<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.array_run_lane::<$ty, N, 4, O>(byte_offset)
            }

            #[inline]
            fn iter_write_2_fixed<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.array_run_lane::<$ty, N, 2, O>(byte_offset)
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
                buffer.array_run_lane::<$ty, N, 1, O>(byte_offset)
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
                buffer.push_arrays::<_, _, O>(std::slice::from_ref(self));
            }
        }

        impl<const N: usize> Nest for [$ty; N] {
            const INODE_COUNT: usize = 1;
            const DEPTH: usize = 0;

            #[inline]
            fn plan_slice<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
                items: &[Self],
                _level: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.plan_data::<O>(items.len());
                items.len() * N * size_of::<$ty>()
            }

            #[inline]
            fn write_slice<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
                items: &[Self],
                _level: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) {
                buffer.push_arrays::<_, _, O>(items);
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
            fn write_16_slot<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.array_run_lane::<$ty, N, 16, O>(byte_offset)
            }

            #[inline]
            fn write_8_slot<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.array_run_lane::<$ty, N, 8, O>(byte_offset)
            }

            #[inline]
            fn write_4_slot<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.array_run_lane::<$ty, N, 4, O>(byte_offset)
            }

            #[inline]
            fn write_2_slot<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.array_run_lane::<$ty, N, 2, O>(byte_offset)
            }

            #[inline]
            fn write_1_slot<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.array_run_lane::<$ty, N, 1, O>(byte_offset)
            }
        }

        impl<const N: usize> Writer for Option<[$ty; N]> {
            #[inline]
            fn foreign_plan<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
                &self,
                _buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                // A bare array fills its slot in the fixed region and nothing more.
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
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.option_array_lane::<$ty, N, 16, O>(self, byte_offset)
            }

            #[inline]
            fn write_8_fixed<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
                &self,
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.option_array_lane::<$ty, N, 8, O>(self, byte_offset)
            }

            #[inline]
            fn write_4_fixed<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
                &self,
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.option_array_lane::<$ty, N, 4, O>(self, byte_offset)
            }

            #[inline]
            fn write_2_fixed<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
                &self,
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.option_array_lane::<$ty, N, 2, O>(self, byte_offset)
            }

            #[inline]
            fn write_1_fixed<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
                &self,
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.option_array_lane::<$ty, N, 1, O>(self, byte_offset)
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
                // The array, and a whole byte of mask for its bit.
                N * size_of::<$ty>() + 1
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
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.option_array_run_lane::<$ty, N, 16, O>(byte_offset)
            }

            #[inline]
            fn iter_write_8_fixed<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.option_array_run_lane::<$ty, N, 8, O>(byte_offset)
            }

            #[inline]
            fn iter_write_4_fixed<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.option_array_run_lane::<$ty, N, 4, O>(byte_offset)
            }

            #[inline]
            fn iter_write_2_fixed<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.option_array_run_lane::<$ty, N, 2, O>(byte_offset)
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
                buffer.option_array_run_lane::<$ty, N, 1, O>(byte_offset)
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
                buffer.push_option_arrays::<_, _, O>(std::slice::from_ref(self));
            }
        }

        impl<const N: usize> Nest for Option<[$ty; N]> {
            const INODE_COUNT: usize = 1;
            const DEPTH: usize = 0;

            #[inline]
            fn plan_slice<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
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
                // Every slot as if present, and a whole byte of mask each.
                items.len() * (N * size_of::<$ty>() + 1)
            }

            #[inline]
            fn write_slice<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
                items: &[Self],
                _level: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) {
                buffer.push_option_arrays::<_, _, O>(items);
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
            fn write_16_slot<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.option_array_run_lane::<$ty, N, 16, O>(byte_offset)
            }

            #[inline]
            fn write_8_slot<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.option_array_run_lane::<$ty, N, 8, O>(byte_offset)
            }

            #[inline]
            fn write_4_slot<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.option_array_run_lane::<$ty, N, 4, O>(byte_offset)
            }

            #[inline]
            fn write_2_slot<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.option_array_run_lane::<$ty, N, 2, O>(byte_offset)
            }

            #[inline]
            fn write_1_slot<
                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,
                const FIXED: usize,
            >(
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.option_array_run_lane::<$ty, N, 1, O>(byte_offset)
            }
        }
    };
}

fixed_writer!(Time);
fixed_writer!(f64);
fixed_writer!(i64);
fixed_writer!(u64);
fixed_writer!(f32);
fixed_writer!(i32);
fixed_writer!(u32);
fixed_writer!(f16);
fixed_writer!(i16);
fixed_writer!(u16);
fixed_writer!(i8);
fixed_writer!(u8);
