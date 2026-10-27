use crate::{
    Layout,
    offsets::{InodeOffset, InodeWidth},
};

mod basic;
mod bool;
mod btree_map;
mod fixed;
mod group;
mod interface;
mod map;
mod nesting;
mod primitive;
mod run;

pub use group::*;
pub use interface::*;
pub use nesting::*;
pub use run::*;

/// `byte_offset` padded up to `align`, with the bytes it skips zeroed.
///
/// The variable region is not zeroed ahead of time, and nothing else writes
/// the gaps the layout leaves for alignment, so whoever lays out past one
/// fills it here. The buffer is already grown to the plan's bound by then.

/// `W` is the word the structure is written in — topology tiers and the fixed
/// region's pointers. It is picked per message: `u16` for packets, `u32` for
/// ones too big for `u16` offsets.
pub trait Writer: InodeWidth {
    /// This just increments the counts of everything.
    ///
    /// Every raw length counted is also added to `byte_offset`, which is
    /// returned. Counts are stored as `W` and wrap if too large; the caller
    /// checks the returned total against `W::RESET` instead.
    ///
    /// Returns an upper bound on how much this will take.
    fn foreign_plan<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    /// This writes all the fixed data, resolving the offsets of the sizes and replacing the
    /// count with their offset. We're left with byte offsets that each variable sized writer
    /// needs to write to.
    ///
    /// buf is the entire slice we're writing to.
    fn write_structure_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    fn write_16_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    fn write_8_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    fn write_4_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    fn write_2_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    fn write_1_fixed<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    /// Writes the variable data
    ///
    fn write_variable<Inodes: Layout, O: InodeOffset, const INODE_LEN: usize, const FIXED: usize>(
        &self,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    );

    /// This just increments the counts of everything.
    ///
    /// Every raw length counted is also added to `byte_offset`, which is
    /// returned. Counts are stored as `W` and wrap if too large; the caller
    /// checks the returned total against `W::RESET` instead.
    ///
    /// Returns an estimate for how much this will take.
    fn iter_foreign_plan<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    /// This writes all the fixed data, resolving the offsets of the sizes and replacing the
    /// count with their offset. We're left with byte offsets that each variable sized writer
    /// needs to write to.
    ///
    /// buf is the entire slice we're writing to.
    fn iter_write_structure_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    fn iter_write_16_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    fn iter_write_8_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    fn iter_write_4_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    fn iter_write_2_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    fn iter_write_1_fixed<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        byte_offset: usize,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    ) -> usize;

    /// Writes the variable data
    ///
    fn iter_write_variable<
        Inodes: Layout,
        O: InodeOffset,
        const INODE_LEN: usize,
        const FIXED: usize,
    >(
        &self,
        buffer: &mut PreBuffer<'_, Inodes, FIXED, INODE_LEN>,
    );
}
