pub mod basic;
pub mod fixed;
pub mod fixed_option;
pub mod group;
mod interface;
pub mod map;
pub mod primitive;
pub mod primitive_option;

use pyro_schema::StructureWord;

use crate::{
    Layout,
    errors::WireError,
    offsets::{InodeOffset, InodeWidth},
};

pub use interface::SplitSource;

/// Core writer for a thing that can have topology, like Vec<f32> can have topology inside of a Vec<Group { v: Vec<f32> }>
pub trait ForeignReader: InodeWidth {
    type Ref<'a, W: StructureWord>;
    type NestedRef<'a, W: StructureWord>: crate::ColumnLen;

    /// For reading into a root group
    fn foreign_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
        buffer: SplitSource<'_, Inodes, FIXED>,
    ) -> Result<Self::Ref<'_, Inodes::Word>, WireError>;

    /// For reading into a group's slice
    fn foreign_iter_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
        buffer: SplitSource<'_, Inodes, FIXED>,
    ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError>;
}
