//! Pure, `PyroType`-independent layout primitives for `pyroformat`'s wire
//! format, extracted into their own crate (Step 0 of
//! `docs/const-layout-plan.md`'s `pyro_optimize!` work) so that
//! `pyroformat-derive` can depend on and run [`layout_in_place`] directly at
//! proc-macro expansion time, on real [`Inode`] values, instead of
//! re-deriving a const-eval token-soup version.
//!
//! Everything here is pure over `Inode`/`KindMeta`/`WireKind` values — no
//! dependency on `pyroformat`'s `PyroType`/`PrimitiveDataType` spec types, no
//! heap allocation in the `const fn`s, and no dependency on any other crate
//! in this workspace. `pyroformat::wire::inodes` and
//! `pyroformat::integration` re-export everything below so every existing
//! call site keeps compiling unchanged.

#[cfg(feature = "arrow")]
mod arrow;
mod bow;
mod bridge;
mod error;
#[cfg(feature = "interface")]
pub mod interface;
mod types;
mod wire;
mod word;

pub use bow::Bow;
pub use bridge::{
    SchemaParts, arena_root, decode_metadata_header, decode_metadata_string, encode_metadata,
    inodes_to_type, pyro_type_to_wire_kind, type_node_count, wire_kind_as_primitive_data_type,
};
mod compat;
pub use compat::{SchemaCheck, check, layout_eq, schema_eq, schema_hash, signature_eq};
pub use error::{SchemaError, WireKindError};
#[cfg(feature = "human")]
pub use types::human;
pub use types::{
    ChildField, ChildIter, FieldName, FieldNames, PrimitiveDataType, PyroField, PyroSchema,
    PyroType, coerce_pyro_types,
};
pub use wire::{
    INODE_SIZE, Inode, InodeDecodeError, InodeMeta, InodeMetaDecodeError, KindMeta,
    MAX_GROUP_DEPTH, MAX_INODES, WireKind, count_children, inode_data_len, layout_in_place,
    nullable_bit_index_for, nullable_validity_start, topology_reset_count, worst_padding,
};
pub use word::{StructureWord, Word, write_word};
