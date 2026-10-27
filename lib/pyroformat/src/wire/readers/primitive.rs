use bitvec::{order::Lsb0, slice::BitSlice};
use pyro_schema::StructureWord;

use crate::{
    Layout,
    errors::WireError,
    offsets::InodeOffset,
    refs::{BoolNested, Nested},
    value::Time,
    wire::readers::{ForeignReader, SplitSource},
};

/// `T`, `[T]`, `Vec<T>`, and `[Vec<..T>]` / `Vec<Vec<..T>>` up to six `Vec`s, for a primitive `T`
/// whose wire bytes are its LE native representation (zero-copy borrow).
macro_rules! primitive_reader {
    ($ty:ty, $read:ident, $slice:ident, $nest:ident) => {
        impl ForeignReader for $ty {

            type Ref<'a, W: StructureWord> = $ty;

            type NestedRef<'a, W: StructureWord> = &'a [$ty];

            fn foreign_read<
                Inodes: Layout,
                O: InodeOffset,
                const FIXED: usize,
            >(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
                Ok(buffer.$read::<O>())
            }

            fn foreign_iter_read<
                Inodes: Layout,
                O: InodeOffset,
                const FIXED: usize,
            >(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
                buffer.$slice::<O>()
            }
        }

        primitive_reader!(@list [$ty], $ty, $read, $slice, $nest);
        primitive_reader!(@list Vec<$ty>, $ty, $read, $slice, $nest);
        primitive_reader!(@nested [Vec<$ty>], $ty, 1, 2, $read, $slice, $nest);
        primitive_reader!(@nested Vec<Vec<$ty>>, $ty, 1, 2, $read, $slice, $nest);
        primitive_reader!(@nested [Vec<Vec<$ty>>], $ty, 2, 3, $read, $slice, $nest);
        primitive_reader!(@nested Vec<Vec<Vec<$ty>>>, $ty, 2, 3, $read, $slice, $nest);
        primitive_reader!(@nested [Vec<Vec<Vec<$ty>>>], $ty, 3, 4, $read, $slice, $nest);
        primitive_reader!(@nested Vec<Vec<Vec<Vec<$ty>>>>, $ty, 3, 4, $read, $slice, $nest);
        primitive_reader!(@nested [Vec<Vec<Vec<Vec<$ty>>>>], $ty, 4, 5, $read, $slice, $nest);
        primitive_reader!(@nested Vec<Vec<Vec<Vec<Vec<$ty>>>>>, $ty, 4, 5, $read, $slice, $nest);
        primitive_reader!(@nested [Vec<Vec<Vec<Vec<Vec<$ty>>>>>], $ty, 5, 6, $read, $slice, $nest);
        primitive_reader!(@nested Vec<Vec<Vec<Vec<Vec<Vec<$ty>>>>>>, $ty, 5, 6, $read, $slice, $nest);
    };

    (@list $self:ty, $ty:ty, $read:ident, $slice:ident, $nest:ident) => {
        impl ForeignReader for $self {

            type Ref<'a, W: StructureWord> = &'a [$ty];

            type NestedRef<'a, W: StructureWord> = Nested<'a, W, $ty, 1>;

            fn foreign_read<
                Inodes: Layout,
                O: InodeOffset,
                const FIXED: usize,
            >(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
                buffer.$slice::<O>()
            }

            fn foreign_iter_read<
                Inodes: Layout,
                O: InodeOffset,
                const FIXED: usize,
            >(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
                buffer.$nest::<O, 1>()
            }
        }
    };

    (@nested $self:ty, $ty:ty, $ref_n:literal, $nest_n:literal, $read:ident, $slice:ident, $nest:ident) => {
        impl ForeignReader for $self {

            type Ref<'a, W: StructureWord> = Nested<'a, W, $ty, $ref_n>;

            type NestedRef<'a, W: StructureWord> = Nested<'a, W, $ty, $nest_n>;

            fn foreign_read<
                Inodes: Layout,
                O: InodeOffset,
                const FIXED: usize,
            >(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
                buffer.$nest::<O, $ref_n>()
            }

            fn foreign_iter_read<
                Inodes: Layout,
                O: InodeOffset,
                const FIXED: usize,
            >(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
                buffer.$nest::<O, $nest_n>()

            }
        }
    };
}

primitive_reader!(u8, read_u8, read_u8_slice, read_u8_nest);
primitive_reader!(i8, read_i8, read_i8_slice, read_i8_nest);
primitive_reader!(u16, read_u16, read_u16_slice, read_u16_nest);
primitive_reader!(i16, read_i16, read_i16_slice, read_i16_nest);
primitive_reader!(u32, read_u32, read_u32_slice, read_u32_nest);
primitive_reader!(i32, read_i32, read_i32_slice, read_i32_nest);
primitive_reader!(u64, read_u64, read_u64_slice, read_u64_nest);
primitive_reader!(i64, read_i64, read_i64_slice, read_i64_nest);
primitive_reader!(half::f16, read_f16, read_f16_slice, read_f16_nest);
primitive_reader!(f32, read_f32, read_f32_slice, read_f32_nest);
primitive_reader!(f64, read_f64, read_f64_slice, read_f64_nest);
primitive_reader!(Time, read_time, read_time_slice, read_time_nest);

// Bool — a scalar is one byte (0x00 = false), but a run of them is
// bit-packed, so lists borrow as a `BitSlice` rather than `&[bool]`.

impl ForeignReader for bool {
    type Ref<'a, W: StructureWord> = bool;

    type NestedRef<'a, W: StructureWord> = &'a BitSlice<u8, Lsb0>;

    fn foreign_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
        buffer: SplitSource<'_, Inodes, FIXED>,
    ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
        Ok(buffer.read_bool::<O>())
    }

    fn foreign_iter_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
        buffer: SplitSource<'_, Inodes, FIXED>,
    ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
        buffer.read_bool_slice::<O>()
    }
}

macro_rules! bool_reader {
    (@list $self:ty) => {
        impl ForeignReader for $self {
            type Ref<'a, W: StructureWord> = &'a BitSlice<u8, Lsb0>;

            type NestedRef<'a, W: StructureWord> = BoolNested<'a, W, 1>;

            fn foreign_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
                buffer.read_bool_slice::<O>()
            }

            fn foreign_iter_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
                buffer.read_bool_nest::<O, 1>()
            }
        }
    };

    (@nested $self:ty, $ref_n:literal, $nest_n:literal) => {
        impl ForeignReader for $self {
            type Ref<'a, W: StructureWord> = BoolNested<'a, W, $ref_n>;

            type NestedRef<'a, W: StructureWord> = BoolNested<'a, W, $nest_n>;

            fn foreign_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
                buffer.read_bool_nest::<O, $ref_n>()
            }

            fn foreign_iter_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
                buffer.read_bool_nest::<O, $nest_n>()
            }
        }
    };
}

bool_reader!(@list [bool]);
bool_reader!(@list Vec<bool>);
bool_reader!(@nested [Vec<bool>], 1, 2);
bool_reader!(@nested Vec<Vec<bool>>, 1, 2);
bool_reader!(@nested [Vec<Vec<bool>>], 2, 3);
bool_reader!(@nested Vec<Vec<Vec<bool>>>, 2, 3);
bool_reader!(@nested [Vec<Vec<Vec<bool>>>], 3, 4);
bool_reader!(@nested Vec<Vec<Vec<Vec<bool>>>>, 3, 4);
bool_reader!(@nested [Vec<Vec<Vec<Vec<bool>>>>], 4, 5);
bool_reader!(@nested Vec<Vec<Vec<Vec<Vec<bool>>>>>, 4, 5);
bool_reader!(@nested [Vec<Vec<Vec<Vec<Vec<bool>>>>>], 5, 6);
bool_reader!(@nested Vec<Vec<Vec<Vec<Vec<Vec<bool>>>>>>, 5, 6);

// `pyroformat::BitVec` is a `Vec<bool>` already packed, so it reads back
// through the same views: each shape is the `Vec<bool>` one above with the
// innermost `Vec<bool>` swapped for it.
bool_reader!(@list crate::BitVec);
bool_reader!(@nested [crate::BitVec], 1, 2);
bool_reader!(@nested Vec<crate::BitVec>, 1, 2);
bool_reader!(@nested [Vec<crate::BitVec>], 2, 3);
bool_reader!(@nested Vec<Vec<crate::BitVec>>, 2, 3);
bool_reader!(@nested [Vec<Vec<crate::BitVec>>], 3, 4);
bool_reader!(@nested Vec<Vec<Vec<crate::BitVec>>>, 3, 4);
bool_reader!(@nested [Vec<Vec<Vec<crate::BitVec>>>], 4, 5);
bool_reader!(@nested Vec<Vec<Vec<Vec<crate::BitVec>>>>, 4, 5);
bool_reader!(@nested [Vec<Vec<Vec<Vec<crate::BitVec>>>>], 5, 6);
bool_reader!(@nested Vec<Vec<Vec<Vec<Vec<crate::BitVec>>>>>, 5, 6);
