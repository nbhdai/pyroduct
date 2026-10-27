//! Derived structs, as a field, nullable, or in a list.
//!
//! A group has no payload of its own: every pass goes to its fields, the
//! first at `Shift<O, 1>`, past the group's own inode. The derive implements
//! [`Group`] for `Foo` by handing each pass to each field in declaration
//! order; `Writer` and `Nest` for `Foo` and `Option<Foo>` are the blankets
//! here, the only ones in the pre writers.

use crate::{
    Layout,
    offsets::{InodeOffset, InodeWidth, Shift},
    schema::{StructureWord, write_word},
    wire::writers::{PreBuffer, Writer, nesting::Nest},
};

/// The passes of a struct's fields, `O` being the first field's.
pub trait Group: InodeWidth {
    /// The sum of the fields' [`Writer::foreign_plan`]: a lower bound.
    fn group_foreign_plan<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    /// [`Self::group_foreign_plan`] for an absent group.
    ///
    /// An absent group's fields are written as the value path writes the
    /// children of a null group it never ingests: every lane still gets its
    /// start pointer (and the padding before it), with nothing in it.
    fn group_foreign_plan_absent<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    fn group_write_structure_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    /// [`Self::group_write_structure_fixed`] for an absent group.
    fn group_write_structure_fixed_absent<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    fn group_write_16_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    /// [`Self::group_write_16_fixed`] for an absent group.
    fn group_write_16_fixed_absent<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    fn group_write_8_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    /// [`Self::group_write_8_fixed`] for an absent group.
    fn group_write_8_fixed_absent<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    fn group_write_4_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    /// [`Self::group_write_4_fixed`] for an absent group.
    fn group_write_4_fixed_absent<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    fn group_write_2_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    /// [`Self::group_write_2_fixed`] for an absent group.
    fn group_write_2_fixed_absent<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    fn group_write_1_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    /// [`Self::group_write_1_fixed`] for an absent group.
    fn group_write_1_fixed_absent<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    fn group_write_variable<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    );

    /// [`Self::group_write_variable`] for an absent group.
    fn group_write_variable_absent<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    );

    /// The sum of the fields' [`Writer::iter_foreign_plan`]: a lower bound.
    fn group_iter_foreign_plan<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    fn group_iter_write_structure_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    fn group_iter_write_16_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    fn group_iter_write_8_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    fn group_iter_write_4_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    fn group_iter_write_2_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    fn group_iter_write_1_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    fn group_iter_write_variable<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    );
}

impl<G: Group> Writer for G {
    #[inline]
    fn foreign_plan<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        self.group_foreign_plan::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(buffer)
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
        self.group_write_structure_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(
            byte_offset,
            buffer,
        )
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
        self.group_write_16_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer)
    }

    #[inline]
    fn write_8_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        self.group_write_8_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer)
    }

    #[inline]
    fn write_4_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        self.group_write_4_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer)
    }

    #[inline]
    fn write_2_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        self.group_write_2_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer)
    }

    #[inline]
    fn write_1_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        self.group_write_1_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer)
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
        self.group_write_variable::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(buffer);
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
        // Iterated, each group is one instance of its slot's count; it has
        // no bytes of its own past the fixed region.
        buffer.plan_data::<O>(1);
        self.group_iter_foreign_plan::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(buffer)
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
        // Iterated (a field of a repeated group, a map's key or value), the
        // group's slot is its own instance count.
        buffer.sizes[O::N].data.write(
            buffer.fixed,
            Inodes::INODES[O::N].topology_offset() as usize,
        );
        G::group_iter_write_structure_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(
            byte_offset,
            buffer,
        )
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
        G::group_iter_write_16_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer)
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
        G::group_iter_write_8_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer)
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
        G::group_iter_write_4_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer)
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
        G::group_iter_write_2_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer)
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
        G::group_iter_write_1_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer)
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
        self.group_iter_write_variable::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(buffer);
    }
}

impl<G: Group> Nest for G {
    const INODE_COUNT: usize = <G as InodeWidth>::INODE_COUNT;
    const DEPTH: usize = 0;

    #[inline]
    fn plan_slice<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        items: &[Self],
        _level: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        buffer.plan_data::<O>(items.len());
        let mut bytes = 0;
        for group in items {
            bytes += group.group_iter_foreign_plan::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(buffer);
        }
        bytes
    }

    #[inline]
    fn write_slice<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        items: &[Self],
        _level: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) {
        for group in items {
            group.group_iter_write_variable::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(buffer);
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
        let inode = Inodes::INODES[O::N];
        // A bare list of groups: the group count is the whole topology.
        if inode.nesting() == 1 && !inode.group_nest() {
            buffer.sizes[O::N]
                .data
                .write(buffer.fixed, inode.topology_offset() as usize);
        }
        G::group_iter_write_structure_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(
            byte_offset,
            buffer,
        )
    }

    #[inline]
    fn write_16_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        G::group_iter_write_16_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer)
    }

    #[inline]
    fn write_8_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        G::group_iter_write_8_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer)
    }

    #[inline]
    fn write_4_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        G::group_iter_write_4_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer)
    }

    #[inline]
    fn write_2_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        G::group_iter_write_2_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer)
    }

    #[inline]
    fn write_1_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        G::group_iter_write_1_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer)
    }
}

impl<G: Group> Writer for Option<G> {
    #[inline]
    fn foreign_plan<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        match self {
            Some(group) => {
                group.group_foreign_plan::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(buffer)
            }
            None => G::group_foreign_plan_absent::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(buffer),
        }
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
        buffer.set_validity_bit::<O>(self.is_some());
        match self {
            Some(group) => group
                .group_write_structure_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(
                    byte_offset,
                    buffer,
                ),
            None => G::group_write_structure_fixed_absent::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(
                byte_offset,
                buffer,
            ),
        }
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
        match self {
            Some(group) => group
                .group_write_16_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer),
            None => G::group_write_16_fixed_absent::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(
                byte_offset,
                buffer,
            ),
        }
    }

    #[inline]
    fn write_8_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        match self {
            Some(group) => group
                .group_write_8_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer),
            None => G::group_write_8_fixed_absent::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(
                byte_offset,
                buffer,
            ),
        }
    }

    #[inline]
    fn write_4_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        match self {
            Some(group) => group
                .group_write_4_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer),
            None => G::group_write_4_fixed_absent::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(
                byte_offset,
                buffer,
            ),
        }
    }

    #[inline]
    fn write_2_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        match self {
            Some(group) => group
                .group_write_2_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer),
            None => G::group_write_2_fixed_absent::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(
                byte_offset,
                buffer,
            ),
        }
    }

    #[inline]
    fn write_1_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        match self {
            Some(group) => group
                .group_write_1_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer),
            None => G::group_write_1_fixed_absent::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(
                byte_offset,
                buffer,
            ),
        }
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
        match self {
            Some(group) => {
                group.group_write_variable::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(buffer)
            }
            None => G::group_write_variable_absent::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(buffer),
        }
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
        // A whole byte of mask for its bit.
        1 + match self {
            Some(group) => {
                group.group_iter_foreign_plan::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(buffer)
            }
            None => 0,
        }
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
        // The mask, one bit per slot, under `(mask start, slots)`.
        let byte_offset = buffer.pad_to(byte_offset, Inodes::Word::SIZE);
        let slots = buffer.sizes[O::N].data.to_usize();
        let offset = Inodes::INODES[O::N].topology_offset() as usize;
        write_word::<Inodes::Word>(buffer.fixed, offset, byte_offset);
        write_word::<Inodes::Word>(buffer.fixed, offset + Inodes::Word::SIZE, slots);
        buffer.sizes[O::N].validity.start = Inodes::Word::from_usize(byte_offset);
        G::group_iter_write_structure_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(
            byte_offset + slots.div_ceil(8),
            buffer,
        )
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
        G::group_iter_write_16_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer)
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
        G::group_iter_write_8_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer)
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
        G::group_iter_write_4_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer)
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
        G::group_iter_write_2_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer)
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
        G::group_iter_write_1_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer)
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
        buffer.push_validity_bit::<O>(self.is_some());
        if let Some(group) = self {
            group.group_iter_write_variable::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(buffer);
        }
    }
}

impl<G: Group> Nest for Option<G> {
    const INODE_COUNT: usize = <G as InodeWidth>::INODE_COUNT;
    const DEPTH: usize = 0;

    #[inline]
    fn plan_slice<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        items: &[Self],
        _level: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        buffer.plan_data::<O>(items.len());
        // A whole byte of mask per slot.
        let mut bytes = items.len();
        for group in items.iter().flatten() {
            bytes += group.group_iter_foreign_plan::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(buffer);
        }
        bytes
    }

    #[inline]
    fn write_slice<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        items: &[Self],
        _level: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) {
        buffer.extend_validity_iter::<O>(items.iter().map(Option::is_some));
        for group in items.iter().flatten() {
            group.group_iter_write_variable::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(buffer);
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
        let inode = Inodes::INODES[O::N];
        let slots = buffer.sizes[O::N].data.to_usize();
        // With list tiers above, their header covers the mask, right after the levels.
        let byte_offset = if inode.nesting() as usize + inode.group_nest() as usize == 1 {
            let byte_offset = buffer.pad_to(byte_offset, Inodes::Word::SIZE);
            let offset = inode.topology_offset() as usize;
            write_word::<Inodes::Word>(buffer.fixed, offset, byte_offset);
            write_word::<Inodes::Word>(buffer.fixed, offset + Inodes::Word::SIZE, slots);
            byte_offset
        } else {
            byte_offset
        };
        buffer.sizes[O::N].validity.start = Inodes::Word::from_usize(byte_offset);
        G::group_iter_write_structure_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(
            byte_offset + slots.div_ceil(8),
            buffer,
        )
    }

    #[inline]
    fn write_16_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        G::group_iter_write_16_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer)
    }

    #[inline]
    fn write_8_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        G::group_iter_write_8_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer)
    }

    #[inline]
    fn write_4_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        G::group_iter_write_4_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer)
    }

    #[inline]
    fn write_2_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        G::group_iter_write_2_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer)
    }

    #[inline]
    fn write_1_slot<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize {
        G::group_iter_write_1_fixed::<Inodes, Shift<O, 1>, INODE_LEN, FIXED>(byte_offset, buffer)
    }
}
