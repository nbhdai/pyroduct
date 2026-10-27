use half::f16;
use std::mem::{align_of, size_of};

use crate::{
    Layout,
    schema::{StructureWord, write_word},
    value::Time,
    wire::writers::{InodeOffset, PreBuffer, nesting::Nest},
};

use super::Writer;

impl<'a, Inodes: Layout, const FIXED: usize, const INODE_LEN: usize>
    PreBuffer<'a, Inodes, FIXED, INODE_LEN>
{
    /// A scalar's value in its slot in the fixed region, if this is its lane.
    #[inline]
    fn scalar_lane<T: Sized + Copy, const LANE: usize, O: InodeOffset>(
        &mut self,
        value: &T,
        byte_offset: usize,
    ) -> usize {
        if align_of::<T>() != LANE {
            return byte_offset;
        }
        let offset = Inodes::INODES[O::N].data_offset() as usize;
        self.fixed[offset..offset + size_of::<T>()]
            .copy_from_slice(Self::as_le_bytes(std::slice::from_ref(value)));

        byte_offset
    }

    /// Same, nullable: the value if present, and its bit in the fixed region's
    /// bitmap either way.
    #[inline]
    fn option_scalar_lane<T: Sized + Copy, const LANE: usize, O: InodeOffset>(
        &mut self,
        value: &Option<T>,
        byte_offset: usize,
    ) -> usize {
        if align_of::<T>() != LANE {
            return byte_offset;
        }
        let inode = Inodes::INODES[O::N];
        // A null leaves the slot's bytes as the layout zeroed them.
        if let Some(value) = value {
            let offset = inode.data_offset() as usize;
            self.fixed[offset..offset + size_of::<T>()]
                .copy_from_slice(Self::as_le_bytes(std::slice::from_ref(value)));
        }
        self.set_validity_bit::<O>(value.is_some());
        byte_offset
    }

    /// A run's slot, if this is its lane.
    #[inline]
    fn run_lane<T: Sized + Copy, const LANE: usize, O: InodeOffset>(
        &mut self,
        mut byte_offset: usize,
    ) -> usize {
        if align_of::<T>() != LANE {
            return byte_offset;
        }
        byte_offset = self.pad_to(byte_offset, std::mem::align_of::<T>());
        let inode = Inodes::INODES[O::N];
        let offset = inode.data_offset() as usize;
        let count = self.sizes[O::N].data;
        write_word::<Inodes::Word>(self.fixed, offset, byte_offset);
        write_word::<Inodes::Word>(self.fixed, offset + Inodes::Word::SIZE, count.to_usize());

        // Leave where we should start writing.
        self.sizes[O::N].data = Inodes::Word::from_usize(byte_offset);
        byte_offset + count.to_usize() * size_of::<T>()
    }

    /// A nullable run's slot, if this is its lane.
    #[inline]
    fn option_run_lane<T: Sized + Copy, const LANE: usize, O: InodeOffset>(
        &mut self,
        mut byte_offset: usize,
    ) -> usize {
        if align_of::<T>() != LANE {
            return byte_offset;
        }
        byte_offset = self.pad_to(byte_offset, align_of::<T>());
        let inode = Inodes::INODES[O::N];
        let offset = inode.data_offset() as usize;
        let count = self.sizes[O::N].data;
        let valid = self.sizes[O::N].validity.start;
        write_word::<Inodes::Word>(self.fixed, offset, byte_offset);
        write_word::<Inodes::Word>(self.fixed, offset + Inodes::Word::SIZE, valid.to_usize());
        write_word::<Inodes::Word>(
            self.fixed,
            offset + 2 * Inodes::Word::SIZE,
            count.to_usize(),
        );

        // Leave where we should start writing.
        let mask_start = byte_offset + valid.to_usize() * size_of::<T>();
        self.sizes[O::N].data = Inodes::Word::from_usize(byte_offset);
        self.sizes[O::N].validity.start = Inodes::Word::from_usize(mask_start);
        mask_start + count.to_usize().div_ceil(8)
    }

    /// Appends one run to the leaf lane.
    #[inline]
    fn push_run<T: Sized + Copy, O: InodeOffset>(&mut self, run: &[T]) {
        // Move the lane's end first: the copy below goes through
        // `self.variable`, which the compiler can't tell apart from
        // `self.sizes`, so updating after it would reload the count.
        let start = self.sizes[O::N].data.to_usize();
        let byte_len = run.len() * size_of::<T>();
        self.sizes[O::N].data = Inodes::Word::from_usize(start + byte_len);
        let offset = start - FIXED;
        self.variable[offset..offset + byte_len].copy_from_slice(Self::as_le_bytes(run));
    }

    /// Appends one nullable run: its present values to the leaf lane, a bit per
    /// slot to the mask.
    #[inline]
    fn push_option_run<T: Sized + Copy, O: InodeOffset>(&mut self, run: &[Option<T>]) {
        for value in run.iter().flatten() {
            let offset = self.sizes[O::N].data.to_usize() - FIXED;
            let byte_len = size_of::<T>();
            self.variable[offset..offset + byte_len]
                .copy_from_slice(Self::as_le_bytes(std::slice::from_ref(value)));
            // Increment the sizes incase we need to do this more.
            self.sizes[O::N].data.add_usize(byte_len);
        }
        self.extend_validity_iter::<O>(run.iter().map(Option::is_some));
    }
}

// ---- one set of impls per primitive, each a call into the helpers -----------

/// The primitive `$ty`, written in lane `$align`: its [`Primitive`] impl, and
/// the [`Writer`] and leaf [`Nest`] of both `$ty` and `Option<$ty>`.
///
/// `Option<$ty>` is concrete per type rather than one `impl<T: Sized + Copy>`:
/// that would overlap the `impl<G> … for Option<G>` a derived group needs,
/// which has to live in this crate since the derive's crate can't implement
/// anything for `Option<Foo>`.
macro_rules! primitive_writer {
    ($ty:ty, $align:expr) => {
        impl Writer for $ty {
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
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.scalar_lane::<$ty, 16, O>(self, byte_offset)
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
                buffer.scalar_lane::<$ty, 8, O>(self, byte_offset)
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
                buffer.scalar_lane::<$ty, 4, O>(self, byte_offset)
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
                buffer.scalar_lane::<$ty, 2, O>(self, byte_offset)
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
                buffer.scalar_lane::<$ty, 1, O>(self, byte_offset)
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
                $align
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
                // The iterated scalar is a run, which has no tiers.
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
                buffer.run_lane::<$ty, 16, O>(byte_offset)
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
                buffer.run_lane::<$ty, 8, O>(byte_offset)
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
                buffer.run_lane::<$ty, 4, O>(byte_offset)
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
                buffer.run_lane::<$ty, 2, O>(byte_offset)
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
                buffer.run_lane::<$ty, 1, O>(byte_offset)
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
                buffer.push_run::<_, O>(std::slice::from_ref(self));
            }
        }

        impl Nest for $ty {
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
                items.len() * align_of::<$ty>()
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
                buffer.push_run::<_, O>(items);
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
                buffer.run_lane::<$ty, 16, O>(byte_offset)
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
                buffer.run_lane::<$ty, 8, O>(byte_offset)
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
                buffer.run_lane::<$ty, 4, O>(byte_offset)
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
                buffer.run_lane::<$ty, 2, O>(byte_offset)
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
                buffer.run_lane::<$ty, 1, O>(byte_offset)
            }
        }

        impl Writer for Option<$ty> {
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
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.option_scalar_lane::<$ty, 16, O>(self, byte_offset)
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
                buffer.option_scalar_lane::<$ty, 8, O>(self, byte_offset)
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
                buffer.option_scalar_lane::<$ty, 4, O>(self, byte_offset)
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
                buffer.option_scalar_lane::<$ty, 2, O>(self, byte_offset)
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
                buffer.option_scalar_lane::<$ty, 1, O>(self, byte_offset)
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
                // The value, and a whole byte of mask for its bit.
                $align + 1
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
                // The iterated scalar is a run, which has no tiers.
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
                buffer.option_run_lane::<$ty, 16, O>(byte_offset)
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
                buffer.option_run_lane::<$ty, 8, O>(byte_offset)
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
                buffer.option_run_lane::<$ty, 4, O>(byte_offset)
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
                buffer.option_run_lane::<$ty, 2, O>(byte_offset)
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
                buffer.option_run_lane::<$ty, 1, O>(byte_offset)
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
                buffer.push_option_run::<_, O>(std::slice::from_ref(self));
            }
        }

        impl Nest for Option<$ty> {
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
                // Present values never outnumber the slots, which are counted above.
                size.validity
                    .start
                    .add_planned(items.iter().filter(|v| v.is_some()).count());
                // Every slot as if present, and a whole byte of mask each.
                items.len() * ($align + 1)
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
                buffer.push_option_run::<_, O>(items);
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
                buffer.option_run_lane::<$ty, 16, O>(byte_offset)
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
                buffer.option_run_lane::<$ty, 8, O>(byte_offset)
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
                buffer.option_run_lane::<$ty, 4, O>(byte_offset)
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
                buffer.option_run_lane::<$ty, 2, O>(byte_offset)
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
                buffer.option_run_lane::<$ty, 1, O>(byte_offset)
            }
        }
    };
}

primitive_writer!(Time, 16);
primitive_writer!(f64, 8);
primitive_writer!(i64, 8);
primitive_writer!(u64, 8);
primitive_writer!(f32, 4);
primitive_writer!(i32, 4);
primitive_writer!(u32, 4);
primitive_writer!(f16, 2);
primitive_writer!(i16, 2);
primitive_writer!(u16, 2);
primitive_writer!(i8, 1);
primitive_writer!(u8, 1);
