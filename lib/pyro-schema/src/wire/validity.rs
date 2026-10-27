use crate::StructureWord;

use super::inode::Inode;
use super::layout::inode_fixed_data_len;

/// Byte offset where a struct's nullable bitmap begins — the end of the
/// fixed (non-bitmap) region, which is where
/// [`layout_fixed_data`](super::layout::layout_fixed_data) leaves off and
/// [`layout_in_place`](super::layout::layout_in_place) puts the bitmap. Used
/// by the derive to pass a real validity position to nullable fields'
/// `from_inode_nullable` constructors, instead of the placeholder
/// `from_inode` (`validity_start = 0`, which always reads back as non-null).
///
/// The region is the topology section, then the fixed values after it, so
/// this is the larger of:
///
/// - the end of the topology section, [`inode_topology_len`]: every slot in
///   `W` words — a list's `(start, count)`, a nullable list's or a
///   group-nested `Option` scalar's `(start, valid, total)`, a group-nested
///   scalar's pointer, a `Str`/`Bytes` pointer, and each group's and map's
///   own slot;
/// - the end of every fixed value: a scalar or fixed array, nullable or not,
///   with `nesting() == 0 && !group_nest()`, whose size doesn't depend on
///   `W`.
pub const fn nullable_validity_start<W: StructureWord>(inodes: &[Inode]) -> usize {
    let mut max = inode_fixed_data_len::<W>(inodes);
    let mut i = 0;
    while i < inodes.len() {
        let inode = inodes[i];
        // A group-nested `Option` scalar is a `(start, valid, total)` slot,
        // already in the topology section; only a bare one sits inline.
        if inode.is_nullable_fixed() && inode.nesting() == 0 && !inode.group_nest() {
            let end = inode.data_offset() as usize + inode.size::<W>();
            if end > max {
                max = end;
            }
        }
        i += 1;
    }
    max
}

/// The validity-bitmap bit index for the nullable fixed field at `inodes[idx]`.
///
/// After [`super::layout::layout_in_place`] has run, every nullable fixed
/// inode already carries its own `nullable_bit_index` (assigned inline
/// during the nullable placement passes, in strictly increasing
/// `data_offset` order). So this is a direct O(1) field read rather than a
/// whole-slice sort-and-search.
pub const fn nullable_bit_index_for(inodes: &[Inode], idx: usize) -> u16 {
    inodes[idx].nullable_bit_index()
}

#[cfg(test)]
mod tests {
    use super::nullable_validity_start;
    use crate::bridge::SchemaParts;
    use crate::types::{PrimitiveDataType, PyroField, PyroSchema, PyroType};
    use crate::wire::layout::inode_data_len;
    use crate::{Bow, Word};

    /// `{ m: Map<Str, Option<u64>>, z: Option<u8> }` under `u16`: the map's
    /// value column is a 6-byte `(start, valid, total)` slot at 10, not an
    /// inline `u64`, so `z` at 16 is the last field and the bitmap follows
    /// it at 17.
    #[test]
    fn group_nested_option_scalar_is_a_slot_not_inline() {
        let mut schema = PyroSchema::new(vec![
            PyroField::new(
                "m",
                PyroType::Map {
                    key: Bow::new(PyroType::Str),
                    value: Bow::new(PyroType::OptionPrimitiveScalar(PrimitiveDataType::U64)),
                },
            ),
            PyroField::new("z", PyroType::OptionPrimitiveScalar(PrimitiveDataType::U8)),
        ]);
        schema.set_word(Word::U16);
        let inodes = SchemaParts::from_schema(&schema).unwrap().inodes;
        assert_eq!(nullable_validity_start::<u16>(&inodes), 17);
        assert_eq!(inode_data_len::<u16>(&inodes), 18);
    }
}
