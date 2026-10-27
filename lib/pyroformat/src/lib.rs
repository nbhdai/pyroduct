#[cfg(not(target_endian = "little"))]
compile_error!("pyroformat requires a little-endian target");

// The derives share their names with the traits they implement, like
// serde's: `Writer` is both the trait and `#[derive(Writer)]`.
pub use pyroformat_derive::{Layout, Reader, Writer};

/// Test-only self-alias so shared test-support source files (e.g.
/// `tests/support/fixture_export.rs`, pulled into `src/wire/mod.rs`'s unit
/// tests via `#[path = ...]`) can refer to this crate as `pyroformat::...`
/// whether they're compiled as part of this crate's own unit tests or as a
/// dependent integration-test binary. See that file's module doc for why.
#[cfg(test)]
extern crate self as pyroformat;

pub mod value;
pub use value::{PyroRow, PyroValue, RowPrinter};
pub mod errors;
mod interface;
pub use interface::*;
pub mod map;
pub use map::Map;
pub mod buf;
pub mod refs;
pub mod wire;

/// Re-exports `pyro-schema`'s layout primitives (`Inode`, `WireKind`,
/// `KindMeta`, `layout_in_place`, `Layout`, ...) under the path
/// `pyroformat-derive`'s generated code names them by. The module used to be
/// `wire::inodes`; it moved out to its own crate so the derive crate could
/// depend on it directly (`docs/const-layout-plan.md`, Step 0).
pub mod schema {
    pub use pyro_schema::*;
}

/// Convenience alias for the only `bitvec` shape `#[derive(PyroCodec)]`
/// understands as a packed-bool field (`BitVec<u8, Lsb0>` matches the wire's
/// byte layout exactly — see `pyroformat-derive`'s `type_map::resolve_type_inner`).
/// Use `pyroformat::BitVec` as a field type to get a memcpy-fast write/read
/// path instead of `Vec<bool>`'s bit-by-bit packing loop.
pub type BitVec = bitvec::vec::BitVec<u8, bitvec::order::Lsb0>;
pub use bitvec::order::Lsb0;
pub use bitvec::slice::BitSlice;

// #[cfg(feature = "arrow")]
// pub mod arrow;

pub trait ToRow {
    /// Convert a borrowed reference to a PyroRow with borrowed data.
    /// Field names are static strings from compile time.
    fn to_row(&self) -> PyroRow<'_>;
}

/// Topology tiers an inode can have: the deepest iterated nesting.
pub const MAX_TIERS: usize = 5;
pub use value::Time;
