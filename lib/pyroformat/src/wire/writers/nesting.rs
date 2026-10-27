use std::{borrow::Cow, rc::Rc, sync::Arc};

use crate::{
    Layout, MAX_TIERS,
    schema::{StructureWord, write_word},
    wire::writers::{InodeOffset, PreBuffer, Writer, run::Run},
};

impl<'a, Inodes: Layout, const FIXED: usize, const INODE_LEN: usize>
    PreBuffer<'a, Inodes, FIXED, INODE_LEN>
{
    /// Lays out tiers `0..levels` of this inode's topology back to back, outermost
    /// first, and writes its `(start, words)` slot.
    ///
    /// Each written tier opens with a [`StructureWord::RESET`] and holds one word per
    /// node. A tier with no nodes is not written, and nor is anything below it —
    /// nothing can have been planned there. Afterwards `topo[level]` is the
    /// offset of that tier's first node word.
    #[inline]
    pub fn write_topology_structure<O: InodeOffset>(
        &mut self,
        levels: usize,
        mut byte_offset: usize,
    ) -> usize {
        byte_offset = self.pad_to(byte_offset, Inodes::Word::SIZE);
        let inode = Inodes::INODES[O::N];
        let offset = inode.topology_offset() as usize;
        let size = &mut self.sizes[O::N];

        let mut level_byte_offset = byte_offset;
        let mut words = 0;
        for level in 0..levels {
            let count = size.topo[level];
            if count.to_usize() == 0 {
                break;
            }
            Inodes::Word::RESET.write(self.variable, level_byte_offset - FIXED);
            size.topo[level] = Inodes::Word::from_usize(level_byte_offset + Inodes::Word::SIZE);
            level_byte_offset += (count.to_usize() + 1) * Inodes::Word::SIZE;
            words += count.to_usize() + 1;
        }

        write_word::<Inodes::Word>(self.fixed, offset, byte_offset);
        write_word::<Inodes::Word>(self.fixed, offset + Inodes::Word::SIZE, words);
        level_byte_offset
    }

    /// Appends one node to tier `level`. Tier words are cumulative, and run on
    /// across every value written to this inode, so the new word is the previous
    /// one (the reset standing in for `0`) plus this node's `extent`.
    #[inline]
    pub fn push_topology_word<O: InodeOffset>(&mut self, level: usize, extent: usize) {
        // Move the tier's end first: the word below is written through
        // `self.variable`, which the compiler can't tell apart from
        // `self.sizes`, so updating after it would reload the count.
        let at = self.sizes[O::N].topo[level].to_usize();
        self.sizes[O::N].topo[level] = Inodes::Word::from_usize(at + Inodes::Word::SIZE);
        let previous = Inodes::Word::read(self.variable, at - Inodes::Word::SIZE - FIXED);
        let base = if previous == Inodes::Word::RESET {
            0
        } else {
            previous.to_usize()
        };
        write_word::<Inodes::Word>(self.variable, at - FIXED, base + extent);
    }
}
/// One level of the nesting, walked over a slice of siblings.
///
/// A leaf (`f32`, `Option<f32>`, `u8`, …) is depth `0`: its slice is the
/// run at the bottom, written to the data lane. Every container of a `Nest`
/// is a `Nest` one deeper, through the blanket impl below, so the depth and
/// the containers come from the type and nothing here is written per depth.
///
/// The methods take the whole `sizes` array and `O` rather than one
/// [`InodeSize`], so an item spanning several inodes (a group, a map) can
/// reach its own.
pub trait Nest: Sized {
    const INODE_COUNT: usize;
    /// `0` for a leaf, one more than its items for a container.
    const DEPTH: usize;

    /// Counts `items`, which sit at tier `level`, and everything below them.
    ///
    /// Returns the bytes they take past the fixed region, less each tier's
    /// reset and alignment: whoever runs the plan adds
    /// [`topology_reset_count`](crate::schema::topology_reset_count) and
    /// [`worst_padding`](crate::schema::worst_padding) once, for the whole
    /// message.
    fn plan_slice<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        items: &[Self],
        level: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    /// Writes `items`, which sit at tier `level`, and everything below them.
    fn write_slice<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        items: &[Self],
        level: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    );

    /// Lays out and writes whatever this level has in the structure pass: an item's own structure (a group's topology
    /// and validity, say), after the tiers of the containers above it.
    /// Returns where the lane continues.
    fn write_structure_slot<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    /// Lays out and writes whatever this level has in the 16-byte lane.
    /// Returns where the lane continues.
    fn write_16_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    /// Lays out and writes whatever this level has in the 8-byte lane.
    /// Returns where the lane continues.
    fn write_8_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    /// Lays out and writes whatever this level has in the 4-byte lane.
    /// Returns where the lane continues.
    fn write_4_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    /// Lays out and writes whatever this level has in the 2-byte lane.
    /// Returns where the lane continues.
    fn write_2_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    /// Lays out and writes whatever this level has in the 1-byte lane.
    /// Returns where the lane continues.
    fn write_1_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;
}

/// A container's [`Nest`], one level over its items: a node per item at
/// `level`, extent its length, and its items one tier down.
macro_rules! container_nest {
    ([$($generics:tt)*] $ty:ty) => {
        impl<$($generics)*> Nest for $ty
        where
            <$ty as Run>::Item: Nest,
        {
            const INODE_COUNT: usize = <<$ty as Run>::Item as Nest>::INODE_COUNT;
            const DEPTH: usize = <$ty as Run>::Item::DEPTH + 1;

            #[inline]
            fn plan_slice< Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
                items: &[Self],
                level: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.sizes[O::N].topo[level].add_planned(items.len());
                // A tier word per item.
                let mut bytes = items.len() * Inodes::Word::SIZE;
                for item in items {
                    bytes += <$ty as Run>::Item::plan_slice::<Inodes, O, INODE_LEN, FIXED>(item.as_run(), level + 1, buffer);
                }
                bytes
            }

            #[inline]
            fn write_slice< Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
                items: &[Self],
                level: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) {
                for item in items {
                    let run = item.as_run();
                    buffer.push_topology_word::<O>(level, run.len());
                    <$ty as Run>::Item::write_slice::<Inodes, O, INODE_LEN, FIXED>(run, level + 1, buffer);
                }
            }

            #[inline]
            fn write_structure_slot<

                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize, const FIXED: usize,
            >(
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                <$ty as Run>::Item::write_structure_slot::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
            }

            #[inline]
            fn write_16_slot< Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                <$ty as Run>::Item::write_16_slot::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
            }

            #[inline]
            fn write_8_slot< Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                <$ty as Run>::Item::write_8_slot::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
            }

            #[inline]
            fn write_4_slot< Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                <$ty as Run>::Item::write_4_slot::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
            }

            #[inline]
            fn write_2_slot< Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                <$ty as Run>::Item::write_2_slot::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
            }

            #[inline]
            fn write_1_slot< Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                <$ty as Run>::Item::write_1_slot::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
            }
        }
    };
}

/// A container's [`Writer`]. As a single value its items are tier `0`, so a
/// run of leaves has no tiers at all; iterated, each value is one node of tier
/// `0` and its items are tier `1`.
macro_rules! container_pre_writer {
    ([$($generics:tt)*] $ty:ty) => {
        impl<$($generics)*> Writer for $ty
        where
            <$ty as Run>::Item: Nest + Writer,
        {

            #[inline]
            fn foreign_plan< Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
                &self,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                <$ty as Run>::Item::plan_slice::<Inodes, O, INODE_LEN, FIXED>(self.as_run(), 0, buffer)
            }

            #[inline]
            fn write_structure_fixed<

                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize, const FIXED: usize,
            >(
                &self,
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                const { assert!(<$ty as Run>::Item::DEPTH <= MAX_TIERS, "nested deeper than MAX_TIERS") };
                // A single run has no tiers, and no topology slot to write.
                let byte_offset = if <$ty as Run>::Item::DEPTH == 0 {
                    byte_offset
                } else {
                    buffer.write_topology_structure::<O>(
                        <$ty as Run>::Item::DEPTH,
                        byte_offset,
                    )
                };
                // Then whatever structure the items have of their own.
                <$ty as Run>::Item::write_structure_slot::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
            }

            #[inline]
            fn write_16_fixed< Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
                &self,
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                <$ty as Run>::Item::write_16_slot::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
            }

            #[inline]
            fn write_8_fixed< Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
                &self,
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                <$ty as Run>::Item::write_8_slot::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
            }

            #[inline]
            fn write_4_fixed< Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
                &self,
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                <$ty as Run>::Item::write_4_slot::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
            }

            #[inline]
            fn write_2_fixed< Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
                &self,
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                <$ty as Run>::Item::write_2_slot::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
            }

            #[inline]
            fn write_1_fixed< Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
                &self,
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                <$ty as Run>::Item::write_1_slot::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
            }

            #[inline]
            fn write_variable< Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
                &self,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) {
                <$ty as Run>::Item::write_slice::<Inodes, O, INODE_LEN, FIXED>(self.as_run(), 0, buffer);
            }

            #[inline]
            fn iter_foreign_plan<

                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize,const FIXED: usize
            >(
                &self,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                buffer.sizes[O::N].topo[0].add_planned(1);
                // Its tier word, then its items.
                Inodes::Word::SIZE + <$ty as Run>::Item::plan_slice::<Inodes, O, INODE_LEN, FIXED>(self.as_run(), 1, buffer)
            }

            #[inline]
            fn iter_write_structure_fixed<

                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize, const FIXED: usize,
            >(
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                const { assert!(<$ty as Run>::Item::DEPTH < MAX_TIERS, "nested deeper than MAX_TIERS") };
                let byte_offset = buffer.write_topology_structure::<O>(
                    <$ty as Run>::Item::DEPTH + 1,
                    byte_offset,
                );
                // Then whatever structure the items have of their own.
                <$ty as Run>::Item::write_structure_slot::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
            }

            #[inline]
            fn iter_write_16_fixed<

                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize, const FIXED: usize,
            >(
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                <$ty as Run>::Item::write_16_slot::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
            }

            #[inline]
            fn iter_write_8_fixed<

                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize, const FIXED: usize,
            >(
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                <$ty as Run>::Item::write_8_slot::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
            }

            #[inline]
            fn iter_write_4_fixed<

                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize, const FIXED: usize,
            >(
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                <$ty as Run>::Item::write_4_slot::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
            }

            #[inline]
            fn iter_write_2_fixed<

                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize, const FIXED: usize,
            >(
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                <$ty as Run>::Item::write_2_slot::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
            }

            #[inline]
            fn iter_write_1_fixed<

                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize, const FIXED: usize,
            >(
                byte_offset: usize,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) -> usize {
                <$ty as Run>::Item::write_1_slot::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
            }

            #[inline]
            fn iter_write_variable<

                Inodes: Layout,
                O: InodeOffset,
                const INODE_LEN: usize, const FIXED: usize,
            >(
                &self,
                buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
            ) {
                let run = self.as_run();
                buffer.push_topology_word::<O>(0, run.len());
                <$ty as Run>::Item::write_slice::<Inodes, O, INODE_LEN, FIXED>(run, 1, buffer);
            }
        }
    };
}

// One impl per container rather than one `impl<C: Run>`: the blankets are the
// derived groups' (`impl<G: Group> … for G`).
container_nest!([T] Vec<T>);
container_nest!([T] Box<[T]>);
container_nest!([T] Arc<[T]>);
container_nest!([T] Rc<[T]>);
container_nest!(['a, T: Clone] Cow<'a, [T]>);
container_nest!(['a, T] &'a [T]);
container_nest!([] String);
container_nest!([] Box<str>);
container_nest!([] Arc<str>);
container_nest!([] Rc<str>);
container_nest!(['a] Cow<'a, str>);
container_nest!(['a] &'a str);
container_nest!([] bytes::Bytes);
container_nest!([] bytes::BytesMut);

container_pre_writer!([T][T]);
container_pre_writer!([T] Vec<T>);
container_pre_writer!([T] Box<[T]>);
container_pre_writer!([T] Arc<[T]>);
container_pre_writer!([T] Rc<[T]>);
container_pre_writer!(['a, T: Clone] Cow<'a, [T]>);
container_pre_writer!(['a, T] &'a [T]);
container_pre_writer!([] str);
container_pre_writer!([] String);
container_pre_writer!([] Box<str>);
container_pre_writer!([] Arc<str>);
container_pre_writer!([] Rc<str>);
container_pre_writer!(['a] Cow<'a, str>);
container_pre_writer!(['a] &'a str);
container_pre_writer!([] bytes::Bytes);
container_pre_writer!([] bytes::BytesMut);
