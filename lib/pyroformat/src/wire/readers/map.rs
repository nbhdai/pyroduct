//! `ForeignReader` for `BTreeMap` and [`crate::Map`], and for up to five
//! levels of `Vec`/slice nesting on top of either — the read side of
//! `writers/map.rs`. Both read back as the same views.
//!
//! A map owns one inode, immediately followed by its key type's inodes and
//! then its value type's: hence `Shift<O, 1>` for the keys and
//! `Past<Shift<O, 1>, K>` for the values.
//!
//! Keys and values are each one flat column across every map, so both are
//! always read in their iterated form (`foreign_iter_read`) whatever the
//! map's own depth, and come back as `K::NestedRef` / `V::NestedRef`.
//!
//! The map's slot mirrors the `nesting` field on its inode. At nesting zero
//! it holds the entry count outright, and the map reads as a [`MapRef`].
//! Deeper than that it is a `(start, count)` pair pointing at `nesting`
//! topology levels — the outer list levels first, the per-map entry counts
//! last — and the map reads as a [`NestedMapRef`] with that many levels. See
//! `src/wire/tests/map_nested.rs` for the byte-level truth.

use std::collections::BTreeMap;

use pyro_schema::StructureWord;

use crate::{
    Layout, Map,
    errors::WireError,
    offsets::{InodeOffset, Past, Shift},
    refs::{MapRef, NestedMapRef},
    wire::readers::{ForeignReader, SplitSource},
};

/// Every wire shape a map can take, for one map type: the bare map, whose
/// slot holds its entry count, then lists of maps, `$ref_n` topology levels
/// deep read as a root and `$nest_n` inside a group's slice.
macro_rules! map_reader {
    (@bare $map:ty) => {
        impl<K, V> ForeignReader for $map
        where
            K: ForeignReader,
            V: ForeignReader,
        {

            type Ref<'a, W: StructureWord> = MapRef<K::NestedRef<'a, W>, V::NestedRef<'a, W>>;

            type NestedRef<'a, W: StructureWord> = NestedMapRef<'a, W, K::NestedRef<'a, W>, V::NestedRef<'a, W>, 1>;

            fn foreign_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
                let len = buffer.read_entry_count::<O>();
                let keys = K::foreign_iter_read::<Inodes, Shift<O, 1>, FIXED>(buffer)?;
                let values = V::foreign_iter_read::<Inodes, Past<Shift<O, 1>, K>, FIXED>(buffer)?;
                MapRef::new(len, keys, values)
            }

            fn foreign_iter_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
                let keys = K::foreign_iter_read::<Inodes, Shift<O, 1>, FIXED>(buffer)?;
                let values = V::foreign_iter_read::<Inodes, Past<Shift<O, 1>, K>, FIXED>(buffer)?;
                let topology = buffer.read_topology::<O, 1>()?;
                NestedMapRef::new(topology, keys, values)
            }
        }
    };

    ($self:ty, $ref_n:literal, $nest_n:literal) => {
        impl<K, V> ForeignReader for $self
        where
            K: ForeignReader,
            V: ForeignReader,
        {

            type Ref<'a, W: StructureWord> = NestedMapRef<'a, W, K::NestedRef<'a, W>, V::NestedRef<'a, W>, $ref_n>;

            type NestedRef<'a, W: StructureWord> = NestedMapRef<'a, W, K::NestedRef<'a, W>, V::NestedRef<'a, W>, $nest_n>;

            fn foreign_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
                let keys = K::foreign_iter_read::<Inodes, Shift<O, 1>, FIXED>(buffer)?;
                let values = V::foreign_iter_read::<Inodes, Past<Shift<O, 1>, K>, FIXED>(buffer)?;
                let topology = buffer.read_topology::<O, $ref_n>()?;
                NestedMapRef::new(topology, keys, values)
            }

            fn foreign_iter_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
                let keys = K::foreign_iter_read::<Inodes, Shift<O, 1>, FIXED>(buffer)?;
                let values = V::foreign_iter_read::<Inodes, Past<Shift<O, 1>, K>, FIXED>(buffer)?;
                let topology = buffer.read_topology::<O, $nest_n>()?;
                NestedMapRef::new(topology, keys, values)
            }
        }
    };

    (@all $map:ident) => {
        map_reader!(@bare $map<K, V>);
        map_reader!([$map<K, V>], 1, 2);
        map_reader!(Vec<$map<K, V>>, 1, 2);
        map_reader!([Vec<$map<K, V>>], 2, 3);
        map_reader!(Vec<Vec<$map<K, V>>>, 2, 3);
        map_reader!([Vec<Vec<$map<K, V>>>], 3, 4);
        map_reader!(Vec<Vec<Vec<$map<K, V>>>>, 3, 4);
        map_reader!([Vec<Vec<Vec<$map<K, V>>>>], 4, 5);
        map_reader!(Vec<Vec<Vec<Vec<$map<K, V>>>>>, 4, 5);
        map_reader!([Vec<Vec<Vec<Vec<$map<K, V>>>>>], 5, 6);
        map_reader!(Vec<Vec<Vec<Vec<Vec<$map<K, V>>>>>>, 5, 6);
    };
}

map_reader!(@all BTreeMap);
map_reader!(@all Map);
