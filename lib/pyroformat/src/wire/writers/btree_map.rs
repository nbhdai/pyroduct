use std::collections::BTreeMap;

use crate::{
    Layout,
    offsets::{InodeOffset, InodeWidth, Past, Shift},
    schema::StructureWord,
    wire::writers::{PreBuffer, Writer, nesting::Nest},
};

impl<K: Writer, V: Writer> Nest for BTreeMap<K, V> {
    const INODE_COUNT: usize = 1 + <K as InodeWidth>::INODE_COUNT + <V as InodeWidth>::INODE_COUNT;
    const DEPTH: usize = 1;

    #[inline]
    fn plan_slice<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        items: &[Self],
        level: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        buffer.sizes[O::N].topo[level].add_planned(items.len());
        // A tier word per map.
        let mut bytes = items.len() * Inodes::Word::SIZE;
        for item in items {
            bytes += item.plan_entries::<Inodes, O, INODE_LEN, FIXED>(buffer);
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
            item.write_entries::<Inodes, O, INODE_LEN, FIXED>(buffer);
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
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        // The containers above wrote this map's tier with their own.
        Self::entries_structure::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
    }

    #[inline]
    fn write_16_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        <Self as Writer>::iter_write_16_fixed::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
    }

    #[inline]
    fn write_8_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        <Self as Writer>::iter_write_8_fixed::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
    }

    #[inline]
    fn write_4_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        <Self as Writer>::iter_write_4_fixed::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
    }

    #[inline]
    fn write_2_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        <Self as Writer>::iter_write_2_fixed::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
    }

    #[inline]
    fn write_1_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        <Self as Writer>::iter_write_1_fixed::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
    }
}

impl<K: Writer, V: Writer> Writer for BTreeMap<K, V> {
    #[inline]
    fn foreign_plan<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        // The entry count is a fixed-region slot.
        buffer.plan_data::<O>(self.len());
        self.plan_entries::<Inodes, O, INODE_LEN, FIXED>(buffer)
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
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        // A bare map's slot is its entry count.
        buffer.sizes[O::N].data.write(
            buffer.fixed,
            Inodes::INODES[O::N].topology_offset() as usize,
        );
        Self::entries_structure::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
    }

    #[inline]
    fn write_16_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        mut byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset = <K as Writer>::iter_write_16_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(
            byte_offset,
            buffer,
        );
        byte_offset =
            <V as Writer>::iter_write_16_fixed::<Inodes, Past<Shift<O, 1>, K>, INODE_LEN, FIXED>(
                byte_offset,
                buffer,
            );
        byte_offset
    }

    #[inline]
    fn write_8_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        mut byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset = <K as Writer>::iter_write_8_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(
            byte_offset,
            buffer,
        );
        byte_offset =
            <V as Writer>::iter_write_8_fixed::<Inodes, Past<Shift<O, 1>, K>, INODE_LEN, FIXED>(
                byte_offset,
                buffer,
            );
        byte_offset
    }

    #[inline]
    fn write_4_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        mut byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset = <K as Writer>::iter_write_4_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(
            byte_offset,
            buffer,
        );
        byte_offset =
            <V as Writer>::iter_write_4_fixed::<Inodes, Past<Shift<O, 1>, K>, INODE_LEN, FIXED>(
                byte_offset,
                buffer,
            );
        byte_offset
    }

    #[inline]
    fn write_2_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        mut byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset = <K as Writer>::iter_write_2_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(
            byte_offset,
            buffer,
        );
        byte_offset =
            <V as Writer>::iter_write_2_fixed::<Inodes, Past<Shift<O, 1>, K>, INODE_LEN, FIXED>(
                byte_offset,
                buffer,
            );
        byte_offset
    }

    #[inline]
    fn write_1_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        mut byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset = <K as Writer>::iter_write_1_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(
            byte_offset,
            buffer,
        );
        byte_offset =
            <V as Writer>::iter_write_1_fixed::<Inodes, Past<Shift<O, 1>, K>, INODE_LEN, FIXED>(
                byte_offset,
                buffer,
            );
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
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) {
        self.write_entries::<Inodes, O, INODE_LEN, FIXED>(buffer);
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
        // Iterated, each map is one node of tier `0`, counting its entries.
        buffer.sizes[O::N].topo[0].add_planned(1);
        // Its tier word, then its entries.
        Inodes::Word::SIZE + self.plan_entries::<Inodes, O, INODE_LEN, FIXED>(buffer)
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
        // Iterated, the map's slot points at its tier: one entry count per map.
        let byte_offset = buffer.write_topology_structure::<O>(1, byte_offset);
        Self::entries_structure::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
    }

    #[inline]
    fn iter_write_16_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        mut byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset = <K as Writer>::iter_write_16_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(
            byte_offset,
            buffer,
        );
        byte_offset =
            <V as Writer>::iter_write_16_fixed::<Inodes, Past<Shift<O, 1>, K>, INODE_LEN, FIXED>(
                byte_offset,
                buffer,
            );
        byte_offset
    }

    #[inline]
    fn iter_write_8_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        mut byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset = <K as Writer>::iter_write_8_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(
            byte_offset,
            buffer,
        );
        byte_offset =
            <V as Writer>::iter_write_8_fixed::<Inodes, Past<Shift<O, 1>, K>, INODE_LEN, FIXED>(
                byte_offset,
                buffer,
            );
        byte_offset
    }

    #[inline]
    fn iter_write_4_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        mut byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset = <K as Writer>::iter_write_4_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(
            byte_offset,
            buffer,
        );
        byte_offset =
            <V as Writer>::iter_write_4_fixed::<Inodes, Past<Shift<O, 1>, K>, INODE_LEN, FIXED>(
                byte_offset,
                buffer,
            );
        byte_offset
    }

    #[inline]
    fn iter_write_2_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        mut byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset = <K as Writer>::iter_write_2_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(
            byte_offset,
            buffer,
        );
        byte_offset =
            <V as Writer>::iter_write_2_fixed::<Inodes, Past<Shift<O, 1>, K>, INODE_LEN, FIXED>(
                byte_offset,
                buffer,
            );
        byte_offset
    }

    #[inline]
    fn iter_write_1_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        mut byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        byte_offset = <K as Writer>::iter_write_1_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(
            byte_offset,
            buffer,
        );
        byte_offset =
            <V as Writer>::iter_write_1_fixed::<Inodes, Past<Shift<O, 1>, K>, INODE_LEN, FIXED>(
                byte_offset,
                buffer,
            );
        byte_offset
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
        self.write_entries::<Inodes, O, INODE_LEN, FIXED>(buffer);
    }
}

/// The entries of one map, without the map's own count or tier: every key
/// and value is one iterated item of its column. The map's slot is its
/// caller's: a bare map's entry count, an iterated map's tier, or the tiers
/// of the containers above it.
trait Entries {
    fn plan_entries<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    fn entries_structure<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    fn write_entries<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    );
}

impl<K: Writer, V: Writer> Entries for BTreeMap<K, V> {
    #[inline]
    fn plan_entries<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        let mut bytes = 0;
        for (k, v) in self.iter() {
            bytes += k.iter_foreign_plan::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(buffer);
            bytes += v.iter_foreign_plan::<Inodes, Past<Shift<O, 1>, K>, INODE_LEN, FIXED>(buffer);
        }
        bytes
    }

    #[inline]
    fn entries_structure<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        let byte_offset =
            <K as Writer>::iter_write_structure_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(
                byte_offset,
                buffer,
            );
        <V as Writer>::iter_write_structure_fixed::<Inodes, Past<Shift<O, 1>, K>, INODE_LEN, FIXED>(
            byte_offset,
            buffer,
        )
    }

    #[inline]
    fn write_entries<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) {
        for (k, v) in self.iter() {
            k.iter_write_variable::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(buffer);
            v.iter_write_variable::<Inodes, Past<Shift<O, 1>, K>, INODE_LEN, FIXED>(buffer);
        }
    }
}
