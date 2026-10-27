use pyro_schema::StructureWord;

use crate::{
    Layout,
    errors::WireError,
    offsets::InodeOffset,
    refs::{
        OptionFixedBoolNested, OptionFixedBoolSlice, OptionFixedSlice, OptionNested, OptionSlice,
    },
    value::Time,
    wire::readers::{ForeignReader, SplitSource},
};

/// `Option<[T; N]>`, `[Option<[T; N]>]`, `Vec<Option<[T; N]>>`, and
/// `[Vec<..Option<[T; N]>>]` / `Vec<Vec<..Option<[T; N]>>>` up to six `Vec`s,
/// for a primitive `T` whose wire bytes are its LE native representation
/// (zero-copy borrow, a bare array included; as a struct's column, an
/// [`OptionFixedSlice`], whose rows are the same `Option<&[T; N]>`). The layout
/// is `primitive_option.rs`'s with `[T; N]` as the element: a present count
/// of arrays, not of `T`s.
macro_rules! fixed_option_reader {
    ($ty:ty, $read:ident, $option_slice:ident, $option_nest:ident) => {
        impl<const N: usize> ForeignReader for Option<[$ty; N]> {

            type Ref<'a, W: StructureWord> = Option<&'a [$ty; N]>;

            type NestedRef<'a, W: StructureWord> = OptionFixedSlice<'a, $ty, N>;

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
                Ok(Some(buffer.$read::<O, N>()?))
            }

            fn foreign_iter_read<
                Inodes: Layout,
                O: InodeOffset,
                const FIXED: usize,
            >(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
                buffer.$option_slice::<O, N>().map(OptionFixedSlice::from)
            }
        }

        fixed_option_reader!(@list [Option<[$ty; N]>], $ty, $option_slice, $option_nest);
        fixed_option_reader!(@list Vec<Option<[$ty; N]>>, $ty, $option_slice, $option_nest);
        fixed_option_reader!(@nested [Vec<Option<[$ty; N]>>], $ty, 1, 2, $option_nest);
        fixed_option_reader!(@nested Vec<Vec<Option<[$ty; N]>>>, $ty, 1, 2, $option_nest);
        fixed_option_reader!(@nested [Vec<Vec<Option<[$ty; N]>>>], $ty, 2, 3, $option_nest);
        fixed_option_reader!(@nested Vec<Vec<Vec<Option<[$ty; N]>>>>, $ty, 2, 3, $option_nest);
        fixed_option_reader!(@nested [Vec<Vec<Vec<Option<[$ty; N]>>>>], $ty, 3, 4, $option_nest);
        fixed_option_reader!(@nested Vec<Vec<Vec<Vec<Option<[$ty; N]>>>>>, $ty, 3, 4, $option_nest);
        fixed_option_reader!(@nested [Vec<Vec<Vec<Vec<Option<[$ty; N]>>>>>], $ty, 4, 5, $option_nest);
        fixed_option_reader!(@nested Vec<Vec<Vec<Vec<Vec<Option<[$ty; N]>>>>>>, $ty, 4, 5, $option_nest);
        fixed_option_reader!(@nested [Vec<Vec<Vec<Vec<Vec<Option<[$ty; N]>>>>>>], $ty, 5, 6, $option_nest);
        fixed_option_reader!(@nested Vec<Vec<Vec<Vec<Vec<Vec<Option<[$ty; N]>>>>>>>, $ty, 5, 6, $option_nest);
    };

    (@list $self:ty, $ty:ty, $option_slice:ident, $option_nest:ident) => {
        impl<const N: usize> ForeignReader for $self {

            type Ref<'a, W: StructureWord> = OptionSlice<'a, [$ty; N]>;

            type NestedRef<'a, W: StructureWord> = OptionNested<'a, W, [$ty; N], 1>;

            fn foreign_read<
                Inodes: Layout,
                O: InodeOffset,
                const FIXED: usize,
            >(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
                buffer.$option_slice::<O, N>()
            }

            fn foreign_iter_read<
                Inodes: Layout,
                O: InodeOffset,
                const FIXED: usize,
            >(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
                buffer.$option_nest::<O, N, 1>()
            }
        }
    };

    (@nested $self:ty, $ty:ty, $ref_n:literal, $nest_n:literal, $option_nest:ident) => {
        impl<const N: usize> ForeignReader for $self {

            type Ref<'a, W: StructureWord> = OptionNested<'a, W, [$ty; N], $ref_n>;

            type NestedRef<'a, W: StructureWord> = OptionNested<'a, W, [$ty; N], $nest_n>;

            fn foreign_read<
                Inodes: Layout,
                O: InodeOffset,
                const FIXED: usize,
            >(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
                buffer.$option_nest::<O, N, $ref_n>()
            }

            fn foreign_iter_read<
                Inodes: Layout,
                O: InodeOffset,
                const FIXED: usize,
            >(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
                buffer.$option_nest::<O, N, $nest_n>()
            }
        }
    };
}

fixed_option_reader!(
    u8,
    read_fixed_u8,
    read_fixed_u8_option_slice,
    read_fixed_u8_option_nest
);
fixed_option_reader!(
    i8,
    read_fixed_i8,
    read_fixed_i8_option_slice,
    read_fixed_i8_option_nest
);
fixed_option_reader!(
    u16,
    read_fixed_u16,
    read_fixed_u16_option_slice,
    read_fixed_u16_option_nest
);
fixed_option_reader!(
    i16,
    read_fixed_i16,
    read_fixed_i16_option_slice,
    read_fixed_i16_option_nest
);
fixed_option_reader!(
    u32,
    read_fixed_u32,
    read_fixed_u32_option_slice,
    read_fixed_u32_option_nest
);
fixed_option_reader!(
    i32,
    read_fixed_i32,
    read_fixed_i32_option_slice,
    read_fixed_i32_option_nest
);
fixed_option_reader!(
    u64,
    read_fixed_u64,
    read_fixed_u64_option_slice,
    read_fixed_u64_option_nest
);
fixed_option_reader!(
    i64,
    read_fixed_i64,
    read_fixed_i64_option_slice,
    read_fixed_i64_option_nest
);
fixed_option_reader!(
    half::f16,
    read_fixed_f16,
    read_fixed_f16_option_slice,
    read_fixed_f16_option_nest
);
fixed_option_reader!(
    f32,
    read_fixed_f32,
    read_fixed_f32_option_slice,
    read_fixed_f32_option_nest
);
fixed_option_reader!(
    f64,
    read_fixed_f64,
    read_fixed_f64_option_slice,
    read_fixed_f64_option_nest
);
fixed_option_reader!(
    Time,
    read_fixed_time,
    read_fixed_time_option_slice,
    read_fixed_time_option_nest
);

// Bool — a nullable `[bool; M]` is its `M` bits inline at the data offset plus
// its validity bit; in a run the present arrays' bits and the bitmap are both
// bit-packed.

impl<const M: usize> ForeignReader for Option<[bool; M]> {
    type Ref<'a, W: StructureWord> = Option<[bool; M]>;

    type NestedRef<'a, W: StructureWord> = OptionFixedBoolSlice<'a, M>;

    fn foreign_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
        buffer: SplitSource<'_, Inodes, FIXED>,
    ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
        if !buffer.read_nullable::<O>() {
            return Ok(None);
        }
        let bits = buffer.read_fixed_bool::<O, M>();
        Ok(Some(std::array::from_fn(|j| bits[j])))
    }

    fn foreign_iter_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
        buffer: SplitSource<'_, Inodes, FIXED>,
    ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
        buffer.read_fixed_bool_option_slice::<O, M>()
    }
}

macro_rules! bool_fixed_option_reader {
    (@list $self:ty) => {
        impl<const M: usize> ForeignReader for $self {
            type Ref<'a, W: StructureWord> = OptionFixedBoolSlice<'a, M>;

            type NestedRef<'a, W: StructureWord> = OptionFixedBoolNested<'a, W, M, 1>;

            fn foreign_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
                buffer.read_fixed_bool_option_slice::<O, M>()
            }

            fn foreign_iter_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
                buffer.read_fixed_bool_option_nest::<O, M, 1>()
            }
        }
    };

    (@nested $self:ty, $ref_n:literal, $nest_n:literal) => {
        impl<const M: usize> ForeignReader for $self {
            type Ref<'a, W: StructureWord> = OptionFixedBoolNested<'a, W, M, $ref_n>;

            type NestedRef<'a, W: StructureWord> = OptionFixedBoolNested<'a, W, M, $nest_n>;

            fn foreign_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
                buffer.read_fixed_bool_option_nest::<O, M, $ref_n>()
            }

            fn foreign_iter_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
                buffer.read_fixed_bool_option_nest::<O, M, $nest_n>()
            }
        }
    };
}

bool_fixed_option_reader!(@list [Option<[bool; M]>]);
bool_fixed_option_reader!(@list Vec<Option<[bool; M]>>);
bool_fixed_option_reader!(@nested [Vec<Option<[bool; M]>>], 1, 2);
bool_fixed_option_reader!(@nested Vec<Vec<Option<[bool; M]>>>, 1, 2);
bool_fixed_option_reader!(@nested [Vec<Vec<Option<[bool; M]>>>], 2, 3);
bool_fixed_option_reader!(@nested Vec<Vec<Vec<Option<[bool; M]>>>>, 2, 3);
bool_fixed_option_reader!(@nested [Vec<Vec<Vec<Option<[bool; M]>>>>], 3, 4);
bool_fixed_option_reader!(@nested Vec<Vec<Vec<Vec<Option<[bool; M]>>>>>, 3, 4);
bool_fixed_option_reader!(@nested [Vec<Vec<Vec<Vec<Option<[bool; M]>>>>>], 4, 5);
bool_fixed_option_reader!(@nested Vec<Vec<Vec<Vec<Vec<Option<[bool; M]>>>>>>, 4, 5);
bool_fixed_option_reader!(@nested [Vec<Vec<Vec<Vec<Vec<Option<[bool; M]>>>>>>], 5, 6);
bool_fixed_option_reader!(@nested Vec<Vec<Vec<Vec<Vec<Vec<Option<[bool; M]>>>>>>>, 5, 6);
