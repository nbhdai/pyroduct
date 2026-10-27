//! Pure, `PyroType`-independent wire-layer primitives: the [`WireKind`]
//! discriminant, the [`Inode`] table entry, the const-evaluable layout
//! algorithm, and the nullable-bitmap helpers built on top of it.

mod inode;
mod kind;
mod layout;
mod meta;
mod validity;

pub use inode::{INODE_SIZE, Inode, InodeDecodeError};
pub use kind::{KindMeta, WireKind};
pub use layout::{
    MAX_GROUP_DEPTH, MAX_INODES, count_children, inode_data_len, layout_in_place,
    topology_reset_count, worst_padding,
};
pub use meta::{InodeMeta, InodeMetaDecodeError};
pub use validity::{nullable_bit_index_for, nullable_validity_start};
