use crate::{
    Layout, Time,
    errors::WireError,
    offsets::InodeOffset,
    refs::{OptionBoolNested, OptionBoolSlice, OptionNested, OptionSlice},
    wire::readers::{ForeignReader, SplitSource},
};
use pyro_schema::StructureWord;

/// `Option<T>`, `[Option<T>]`, `Vec<Option<T>>`, and `[Vec<..Option<T>>]` /
/// `Vec<Vec<..Option<T>>>` up to six `Vec`s, for a primitive `T` whose wire
/// bytes are its LE native representation (zero-copy borrow). Only the leaf
/// is nullable, so every level above it is a plain topology level.
macro_rules! option_reader {
    ($ty:ty, $read:ident, $option_slice:ident, $option_nest:ident) => {
        impl ForeignReader for Option<$ty> {

            type Ref<'a, W: StructureWord> = Option<$ty>;

            type NestedRef<'a, W: StructureWord> = OptionSlice<'a, $ty>;

            fn foreign_read<
                Inodes: Layout,
                O: InodeOffset,
                const FIXED: usize,
            >(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
                if !buffer.read_nullable::<O>() {
                    return Ok(None);
                }
                Ok(Some(buffer.$read::<O>()))
            }

            fn foreign_iter_read<
                Inodes: Layout,
                O: InodeOffset,
                const FIXED: usize,
            >(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
                buffer.$option_slice::<O>()
            }
        }

        option_reader!(@list [Option<$ty>], $ty, $option_slice, $option_nest);
        option_reader!(@list Vec<Option<$ty>>, $ty, $option_slice, $option_nest);
        option_reader!(@nested [Vec<Option<$ty>>], $ty, 1, 2, $option_nest);
        option_reader!(@nested Vec<Vec<Option<$ty>>>, $ty, 1, 2, $option_nest);
        option_reader!(@nested [Vec<Vec<Option<$ty>>>], $ty, 2, 3, $option_nest);
        option_reader!(@nested Vec<Vec<Vec<Option<$ty>>>>, $ty, 2, 3, $option_nest);
        option_reader!(@nested [Vec<Vec<Vec<Option<$ty>>>>], $ty, 3, 4, $option_nest);
        option_reader!(@nested Vec<Vec<Vec<Vec<Option<$ty>>>>>, $ty, 3, 4, $option_nest);
        option_reader!(@nested [Vec<Vec<Vec<Vec<Option<$ty>>>>>], $ty, 4, 5, $option_nest);
        option_reader!(@nested Vec<Vec<Vec<Vec<Vec<Option<$ty>>>>>>, $ty, 4, 5, $option_nest);
        option_reader!(@nested [Vec<Vec<Vec<Vec<Vec<Option<$ty>>>>>>], $ty, 5, 6, $option_nest);
        option_reader!(@nested Vec<Vec<Vec<Vec<Vec<Vec<Option<$ty>>>>>>>, $ty, 5, 6, $option_nest);
    };

    (@list $self:ty, $ty:ty, $option_slice:ident, $option_nest:ident) => {
        impl ForeignReader for $self {

            type Ref<'a, W: StructureWord> = OptionSlice<'a, $ty>;

            type NestedRef<'a, W: StructureWord> = OptionNested<'a, W, $ty, 1>;

            fn foreign_read<
                Inodes: Layout,
                O: InodeOffset,
                const FIXED: usize,
            >(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
                buffer.$option_slice::<O>()
            }

            fn foreign_iter_read<
                Inodes: Layout,
                O: InodeOffset,
                const FIXED: usize,
            >(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
                buffer.$option_nest::<O, 1>()
            }
        }
    };

    (@nested $self:ty, $ty:ty, $ref_n:literal, $nest_n:literal, $option_nest:ident) => {
        impl ForeignReader for $self {

            type Ref<'a, W: StructureWord> = OptionNested<'a, W, $ty, $ref_n>;

            type NestedRef<'a, W: StructureWord> = OptionNested<'a, W, $ty, $nest_n>;

            fn foreign_read<
                Inodes: Layout,
                O: InodeOffset,
                const FIXED: usize,
            >(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
                buffer.$option_nest::<O, $ref_n>()
            }

            fn foreign_iter_read<
                Inodes: Layout,
                O: InodeOffset,
                const FIXED: usize,
            >(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
                buffer.$option_nest::<O, $nest_n>()
            }
        }
    };
}

option_reader!(u8, read_u8, read_u8_option_slice, read_u8_option_nest);
option_reader!(i8, read_i8, read_i8_option_slice, read_i8_option_nest);
option_reader!(u16, read_u16, read_u16_option_slice, read_u16_option_nest);
option_reader!(i16, read_i16, read_i16_option_slice, read_i16_option_nest);
option_reader!(u32, read_u32, read_u32_option_slice, read_u32_option_nest);
option_reader!(i32, read_i32, read_i32_option_slice, read_i32_option_nest);
option_reader!(u64, read_u64, read_u64_option_slice, read_u64_option_nest);
option_reader!(i64, read_i64, read_i64_option_slice, read_i64_option_nest);
option_reader!(
    half::f16,
    read_f16,
    read_f16_option_slice,
    read_f16_option_nest
);
option_reader!(f32, read_f32, read_f32_option_slice, read_f32_option_nest);
option_reader!(f64, read_f64, read_f64_option_slice, read_f64_option_nest);
option_reader!(
    Time,
    read_time,
    read_time_option_slice,
    read_time_option_nest
);

// Bool — a nullable scalar is one byte (0x00 = false) plus its validity bit;
// in a run both the present values and the bitmap are bit-packed.

impl ForeignReader for Option<bool> {
    type Ref<'a, W: StructureWord> = Option<bool>;

    type NestedRef<'a, W: StructureWord> = OptionBoolSlice<'a>;

    fn foreign_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
        buffer: SplitSource<'_, Inodes, FIXED>,
    ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
        if !buffer.read_nullable::<O>() {
            return Ok(None);
        }
        Ok(Some(buffer.read_bool::<O>()))
    }

    fn foreign_iter_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
        buffer: SplitSource<'_, Inodes, FIXED>,
    ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
        buffer.read_bool_option_slice::<O>()
    }
}

macro_rules! bool_option_reader {
    (@nested $self:ty, $ref_n:literal, $nest_n:literal) => {
        impl ForeignReader for $self {
            type Ref<'a, W: StructureWord> = OptionBoolNested<'a, W, $ref_n>;

            type NestedRef<'a, W: StructureWord> = OptionBoolNested<'a, W, $nest_n>;

            fn foreign_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
                buffer.read_bool_option_nest::<O, $ref_n>()
            }

            fn foreign_iter_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
                buffer.read_bool_option_nest::<O, $nest_n>()
            }
        }
    };

    (@list $self:ty) => {
        impl ForeignReader for $self {
            type Ref<'a, W: StructureWord> = OptionBoolSlice<'a>;

            type NestedRef<'a, W: StructureWord> = OptionBoolNested<'a, W, 1>;

            fn foreign_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
                buffer.read_bool_option_slice::<O>()
            }

            fn foreign_iter_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
                buffer.read_bool_option_nest::<O, 1>()
            }
        }
    };
}

bool_option_reader!(@list [Option<bool>]);
bool_option_reader!(@list Vec<Option<bool>>);
bool_option_reader!(@nested [Vec<Option<bool>>], 1, 2);
bool_option_reader!(@nested Vec<Vec<Option<bool>>>, 1, 2);
bool_option_reader!(@nested [Vec<Vec<Option<bool>>>], 2, 3);
bool_option_reader!(@nested Vec<Vec<Vec<Option<bool>>>>, 2, 3);
bool_option_reader!(@nested [Vec<Vec<Vec<Option<bool>>>>], 3, 4);
bool_option_reader!(@nested Vec<Vec<Vec<Vec<Option<bool>>>>>, 3, 4);
bool_option_reader!(@nested [Vec<Vec<Vec<Vec<Option<bool>>>>>], 4, 5);
bool_option_reader!(@nested Vec<Vec<Vec<Vec<Vec<Option<bool>>>>>>, 4, 5);
bool_option_reader!(@nested [Vec<Vec<Vec<Vec<Vec<Option<bool>>>>>>], 5, 6);
bool_option_reader!(@nested Vec<Vec<Vec<Vec<Vec<Vec<Option<bool>>>>>>>, 5, 6);
