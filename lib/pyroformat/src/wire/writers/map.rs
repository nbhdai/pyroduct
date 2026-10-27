//! [`Map`]: the same wire bytes as a `BTreeMap` with the same entries, but its
//! keys and values are each one slice, so each column is written through its
//! [`Nest`] over the whole slice at once — one copy for a primitive column —
//! rather than an entry at a time.

use crate::{
    Layout,
    map::Map,
    offsets::{InodeOffset, Past, Shift},
    schema::StructureWord,
    wire::writers::{PreBuffer, Writer, nesting::Nest},
};

impl<K: Writer + Nest, V: Writer + Nest> Writer for Map<K, V> {
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
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        Self::iter_write_16_fixed::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
    }

    #[inline]
    fn write_8_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        Self::iter_write_8_fixed::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
    }

    #[inline]
    fn write_4_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        Self::iter_write_4_fixed::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
    }

    #[inline]
    fn write_2_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        Self::iter_write_2_fixed::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
    }

    #[inline]
    fn write_1_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        Self::iter_write_1_fixed::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
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
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        let byte_offset =
            K::write_16_slot::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer);
        V::write_16_slot::<Inodes, Past<Shift<O, 1>, K>, INODE_LEN, FIXED>(byte_offset, buffer)
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
        let byte_offset =
            K::write_8_slot::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer);
        V::write_8_slot::<Inodes, Past<Shift<O, 1>, K>, INODE_LEN, FIXED>(byte_offset, buffer)
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
        let byte_offset =
            K::write_4_slot::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer);
        V::write_4_slot::<Inodes, Past<Shift<O, 1>, K>, INODE_LEN, FIXED>(byte_offset, buffer)
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
        let byte_offset =
            K::write_2_slot::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer);
        V::write_2_slot::<Inodes, Past<Shift<O, 1>, K>, INODE_LEN, FIXED>(byte_offset, buffer)
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
        let byte_offset =
            K::write_1_slot::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer);
        V::write_1_slot::<Inodes, Past<Shift<O, 1>, K>, INODE_LEN, FIXED>(byte_offset, buffer)
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

/// The entries of one map, without the map's own count or tier: the keys
/// and the values are each one run of their column. The map's slot is its
/// caller's: a bare map's entry count, an iterated map's tier, or the tiers
/// of the containers above it.
impl<K: Writer + Nest, V: Writer + Nest> Map<K, V> {
    #[inline]
    fn plan_entries<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        K::plan_slice::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(&self.keys, 0, buffer)
            + V::plan_slice::<Inodes, Past<Shift<O, 1>, K>, INODE_LEN, FIXED>(
                &self.values,
                0,
                buffer,
            )
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
        let mut byte_offset = byte_offset;
        if K::DEPTH > 0 {
            byte_offset = buffer.write_topology_structure::<Shift<O, 1>>(K::DEPTH, byte_offset);
        }
        byte_offset =
            K::write_structure_slot::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer);
        if V::DEPTH > 0 {
            byte_offset =
                buffer.write_topology_structure::<Past<Shift<O, 1>, K>>(V::DEPTH, byte_offset);
        }
        V::write_structure_slot::<Inodes, Past<Shift<O, 1>, K>, INODE_LEN, FIXED>(
            byte_offset,
            buffer,
        )
    }

    #[inline]
    fn write_entries<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) {
        K::write_slice::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(&self.keys, 0, buffer);
        V::write_slice::<Inodes, Past<Shift<O, 1>, K>, INODE_LEN, FIXED>(&self.values, 0, buffer);
    }
}

impl<K: Writer + Nest, V: Writer + Nest> Nest for Map<K, V> {
    const INODE_COUNT: usize = 1 + <K as Nest>::INODE_COUNT + <V as Nest>::INODE_COUNT;
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
        for map in items {
            bytes += map.plan_entries::<Inodes, O, INODE_LEN, FIXED>(buffer);
        }
        bytes
    }

    #[inline]
    fn write_slice<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        items: &[Self],
        level: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) {
        for map in items {
            buffer.push_topology_word::<O>(level, map.len());
            map.write_entries::<Inodes, O, INODE_LEN, FIXED>(buffer);
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
        Self::iter_write_16_fixed::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
    }

    #[inline]
    fn write_8_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        Self::iter_write_8_fixed::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
    }

    #[inline]
    fn write_4_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        Self::iter_write_4_fixed::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
    }

    #[inline]
    fn write_2_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        Self::iter_write_2_fixed::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
    }

    #[inline]
    fn write_1_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        Self::iter_write_1_fixed::<Inodes, O, INODE_LEN, FIXED>(byte_offset, buffer)
    }
}
