use pyro_schema::StructureWord;

use crate::{
    Layout,
    errors::WireError,
    offsets::{InodeOffset, InodeWidth, Shift},
    refs::{GroupNested, OptionList, OptionNest},
    wire::readers::{ForeignReader, SplitSource},
};

/// The reader twin of [`crate::wire::writers::group::GroupForeignWriter`]:
/// a struct's fields. [`Self::group_read`] reads one row starting at `O`,
/// with no inode of its own; [`Self::group_iter_read`] reads the column
/// under group inode `O`.
///
/// A root row has no group inode — its first field is inode zero — so this
/// is what [`crate::Reader`] reads at [`crate::wire::writers::Root`]. The
/// blanket [`ForeignReader`] below is the same struct as a field of another
/// one, where `O` is its own group inode and the fields start one past it.
pub trait GroupForeignReader: InodeWidth {
    /// One row: every field's own [`ForeignReader::Ref`].
    type Ref<'a, W: StructureWord>;

    /// A column of rows: every field's own [`ForeignReader::NestedRef`].
    type Slice<'a, W: StructureWord>: crate::ColumnLen;

    fn group_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
        buffer: SplitSource<'_, Inodes, FIXED>,
    ) -> Result<Self::Ref<'_, Inodes::Word>, WireError>;

    /// The column, at the group's own inode `O` — a column only exists
    /// under a group inode, unlike a root row — with the fields at
    /// `Shift<O, 1>` on.
    fn group_iter_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
        buffer: SplitSource<'_, Inodes, FIXED>,
    ) -> Result<Self::Slice<'_, Inodes::Word>, WireError>;
}

impl<T: GroupForeignReader> ForeignReader for T {
    type Ref<'a, W: StructureWord> = <T as GroupForeignReader>::Ref<'a, W>;

    type NestedRef<'a, W: StructureWord> = <T as GroupForeignReader>::Slice<'a, W>;

    fn foreign_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
        buffer: SplitSource<'_, Inodes, FIXED>,
    ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
        T::group_read::<Inodes, Shift<O, 1>, FIXED>(buffer)
    }

    fn foreign_iter_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
        buffer: SplitSource<'_, Inodes, FIXED>,
    ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
        T::group_iter_read::<Inodes, O, FIXED>(buffer)
    }
}

// =============================================================================
// Nullable and repeated groups
// =============================================================================
//
// Each impl reads what the matching `ForeignWriter` in
// `wire::writers::group` wrote. The group's own inode is at `O`, its fields
// start at `Shift<O, 1>`, and the fields are always read as whole columns
// (`group_iter_read`, handed `O` itself): however deep the lists above them, a group's fields
// hold every row in the buffer, and the lists are only topology on top of
// that column (`refs::list`'s "everything stays flat").
//
// * `Option<T>` is a bit in the fixed region's bitmap and the fields in
//   place. Iterated, it is a `(start, slots)` slot addressing a mask, with
//   only the present rows in the columns below.
// * `Vec<T>` keeps only the row count in its slot, so it is the column
//   itself. Every deeper list is a `(start, words)` slot over its tiers.
// * `Vec<Option<T>>` is a `(start, slots)` slot addressing its mask. Every
//   deeper one is a `(start, words)` slot over its tiers with the mask
//   right behind them, covering the last tier's total.

impl<T: GroupForeignReader> ForeignReader for Option<T> {
    type Ref<'a, W: StructureWord> = Option<<T as GroupForeignReader>::Ref<'a, W>>;

    type NestedRef<'a, W: StructureWord> = OptionList<'a, <T as GroupForeignReader>::Slice<'a, W>>;

    fn foreign_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
        buffer: SplitSource<'_, Inodes, FIXED>,
    ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
        if buffer.read_nullable::<O>() {
            Ok(Some(T::group_read::<Inodes, Shift<O, 1>, FIXED>(buffer)?))
        } else {
            Ok(None)
        }
    }

    fn foreign_iter_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
        buffer: SplitSource<'_, Inodes, FIXED>,
    ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
        let validity = buffer.read_group_mask::<O>()?;
        Ok(OptionList {
            len: validity.len(),
            validity,
            prior_valid: 0,
            data: T::group_iter_read::<Inodes, O, FIXED>(buffer)?,
        })
    }
}

impl<T: GroupForeignReader> ForeignReader for Vec<T> {
    type Ref<'a, W: StructureWord> = <T as GroupForeignReader>::Slice<'a, W>;

    type NestedRef<'a, W: StructureWord> =
        GroupNested<'a, W, <T as GroupForeignReader>::Slice<'a, W>, 1>;

    fn foreign_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
        buffer: SplitSource<'_, Inodes, FIXED>,
    ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
        // The slot's row count is the column's own length, so the column is
        // the whole list.
        T::group_iter_read::<Inodes, O, FIXED>(buffer)
    }

    fn foreign_iter_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
        buffer: SplitSource<'_, Inodes, FIXED>,
    ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
        GroupNested::new(
            buffer.read_topology::<O, 1>()?,
            T::group_iter_read::<Inodes, O, FIXED>(buffer)?,
        )
    }
}

impl<T: GroupForeignReader> ForeignReader for Vec<Option<T>> {
    type Ref<'a, W: StructureWord> = OptionList<'a, <T as GroupForeignReader>::Slice<'a, W>>;

    type NestedRef<'a, W: StructureWord> =
        OptionNest<'a, W, <T as GroupForeignReader>::Slice<'a, W>, 1>;

    fn foreign_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
        buffer: SplitSource<'_, Inodes, FIXED>,
    ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
        // No tiers: the slot is `(mask start, slots)`.
        let validity = buffer.read_group_mask::<O>()?;
        Ok(OptionList {
            len: validity.len(),
            validity,
            prior_valid: 0,
            data: T::group_iter_read::<Inodes, O, FIXED>(buffer)?,
        })
    }

    fn foreign_iter_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
        buffer: SplitSource<'_, Inodes, FIXED>,
    ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
        read_option_nest::<T, Inodes, O, FIXED, 1>(buffer)
    }
}

/// A nullable group column under `N` tiers: the tiers from the
/// `(start, words)` slot, then the mask right behind them over the last
/// tier's total.
fn read_option_nest<
    T: GroupForeignReader,
    Inodes: Layout,
    O: InodeOffset,
    const FIXED: usize,
    const N: usize,
>(
    buffer: SplitSource<'_, Inodes, FIXED>,
) -> Result<
    OptionNest<'_, Inodes::Word, <T as GroupForeignReader>::Slice<'_, Inodes::Word>, N>,
    WireError,
> {
    let (topology, validity) = buffer.read_group_option_nest::<O, N>()?;
    Ok(OptionNest {
        topology,
        len: validity.len(),
        validity,
        prior_valid: 0,
        data: T::group_iter_read::<Inodes, O, FIXED>(buffer)?,
    })
}

/// `Vec<Vec<..T>>` read as a root with `$ref_n` tiers, and inside a
/// group's column with one more; the same for `Vec<Vec<..Option<T>>>`.
macro_rules! group_list_reader {
    (@plain $self:ty, $ref_n:literal, $nest_n:literal) => {
        impl<T: GroupForeignReader> ForeignReader for $self {
            type Ref<'a, W: StructureWord> =
                GroupNested<'a, W, <T as GroupForeignReader>::Slice<'a, W>, $ref_n>;

            type NestedRef<'a, W: StructureWord> =
                GroupNested<'a, W, <T as GroupForeignReader>::Slice<'a, W>, $nest_n>;

            fn foreign_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
                GroupNested::new(
                    buffer.read_topology::<O, $ref_n>()?,
                    T::group_iter_read::<Inodes, O, FIXED>(buffer)?,
                )
            }

            fn foreign_iter_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
                GroupNested::new(
                    buffer.read_topology::<O, $nest_n>()?,
                    T::group_iter_read::<Inodes, O, FIXED>(buffer)?,
                )
            }
        }
    };

    (@option $self:ty, $ref_n:literal, $nest_n:literal) => {
        impl<T: GroupForeignReader> ForeignReader for $self {
            type Ref<'a, W: StructureWord> =
                OptionNest<'a, W, <T as GroupForeignReader>::Slice<'a, W>, $ref_n>;

            type NestedRef<'a, W: StructureWord> =
                OptionNest<'a, W, <T as GroupForeignReader>::Slice<'a, W>, $nest_n>;

            fn foreign_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
                read_option_nest::<T, Inodes, O, FIXED, $ref_n>(buffer)
            }

            fn foreign_iter_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
                read_option_nest::<T, Inodes, O, FIXED, $nest_n>(buffer)
            }
        }
    };
}

group_list_reader!(@plain Vec<Vec<T>>, 1, 2);
group_list_reader!(@plain Vec<Vec<Vec<T>>>, 2, 3);
group_list_reader!(@plain Vec<Vec<Vec<Vec<T>>>>, 3, 4);
group_list_reader!(@plain Vec<Vec<Vec<Vec<Vec<T>>>>>, 4, 5);
group_list_reader!(@plain Vec<Vec<Vec<Vec<Vec<Vec<T>>>>>>, 5, 6);

group_list_reader!(@option Vec<Vec<Option<T>>>, 1, 2);
group_list_reader!(@option Vec<Vec<Vec<Option<T>>>>, 2, 3);
group_list_reader!(@option Vec<Vec<Vec<Vec<Option<T>>>>>, 3, 4);
group_list_reader!(@option Vec<Vec<Vec<Vec<Vec<Option<T>>>>>>, 4, 5);
group_list_reader!(@option Vec<Vec<Vec<Vec<Vec<Vec<Option<T>>>>>>>, 5, 6);
