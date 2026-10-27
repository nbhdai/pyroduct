use pyro_schema::StructureWord;

use crate::{
    Layout,
    errors::WireError,
    offsets::InodeOffset,
    refs::{FixedBoolNested, FixedBoolSlice, FixedSlice, Nested},
    value::Time,
    wire::readers::{ForeignReader, SplitSource},
};

/// `[T; N]`, `[[T; N]]`, `Vec<[T; N]>`, and `[Vec<..[T; N]>]` /
/// `Vec<Vec<..[T; N]>>` up to six `Vec`s, for a primitive `T` whose wire
/// bytes are its LE native representation. A bare array sits inline at its
/// data offset and borrows from there; a run of them is `(offset, array
/// count)` and borrows too — as a struct's column, a [`FixedSlice`], whose
/// rows are the same `&[T; N]` borrows.
macro_rules! fixed_reader {
    ($ty:ty, $read:ident, $slice:ident, $nest:ident) => {
        impl<const N: usize> ForeignReader for [$ty; N] {

            type Ref<'a, W: StructureWord> = &'a [$ty; N];

            type NestedRef<'a, W: StructureWord> = FixedSlice<'a, $ty, N>;

            fn foreign_read<
                Inodes: Layout,
                O: InodeOffset,
                const FIXED: usize,
            >(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
                buffer.$read::<O, N>()
            }

            fn foreign_iter_read<
                Inodes: Layout,
                O: InodeOffset,
                const FIXED: usize,
            >(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
                buffer.$slice::<O, N>().map(FixedSlice::new)
            }
        }

        fixed_reader!(@list [[$ty; N]], $ty, $slice, $nest);
        fixed_reader!(@list Vec<[$ty; N]>, $ty, $slice, $nest);
        fixed_reader!(@nested [Vec<[$ty; N]>], $ty, 1, 2, $nest);
        fixed_reader!(@nested Vec<Vec<[$ty; N]>>, $ty, 1, 2, $nest);
        fixed_reader!(@nested [Vec<Vec<[$ty; N]>>], $ty, 2, 3, $nest);
        fixed_reader!(@nested Vec<Vec<Vec<[$ty; N]>>>, $ty, 2, 3, $nest);
        fixed_reader!(@nested [Vec<Vec<Vec<[$ty; N]>>>], $ty, 3, 4, $nest);
        fixed_reader!(@nested Vec<Vec<Vec<Vec<[$ty; N]>>>>, $ty, 3, 4, $nest);
        fixed_reader!(@nested [Vec<Vec<Vec<Vec<[$ty; N]>>>>], $ty, 4, 5, $nest);
        fixed_reader!(@nested Vec<Vec<Vec<Vec<Vec<[$ty; N]>>>>>, $ty, 4, 5, $nest);
        fixed_reader!(@nested [Vec<Vec<Vec<Vec<Vec<[$ty; N]>>>>>], $ty, 5, 6, $nest);
        fixed_reader!(@nested Vec<Vec<Vec<Vec<Vec<Vec<[$ty; N]>>>>>>, $ty, 5, 6, $nest);
    };

    (@list $self:ty, $ty:ty, $slice:ident, $nest:ident) => {
        impl<const N: usize> ForeignReader for $self {

            type Ref<'a, W: StructureWord> = &'a [[$ty; N]];

            type NestedRef<'a, W: StructureWord> = Nested<'a, W, [$ty; N], 1>;

            fn foreign_read<
                Inodes: Layout,
                O: InodeOffset,
                const FIXED: usize,
            >(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
                buffer.$slice::<O, N>()
            }

            fn foreign_iter_read<
                Inodes: Layout,
                O: InodeOffset,
                const FIXED: usize,
            >(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
                buffer.$nest::<O, N, 1>()
            }
        }
    };

    (@nested $self:ty, $ty:ty, $ref_n:literal, $nest_n:literal, $nest:ident) => {
        impl<const N: usize> ForeignReader for $self {

            type Ref<'a, W: StructureWord> = Nested<'a, W, [$ty; N], $ref_n>;

            type NestedRef<'a, W: StructureWord> = Nested<'a, W, [$ty; N], $nest_n>;

            fn foreign_read<
                Inodes: Layout,
                O: InodeOffset,
                const FIXED: usize,
            >(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
                buffer.$nest::<O, N, $ref_n>()
            }

            fn foreign_iter_read<
                Inodes: Layout,
                O: InodeOffset,
                const FIXED: usize,
            >(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
                buffer.$nest::<O, N, $nest_n>()
            }
        }
    };
}

fixed_reader!(u8, read_fixed_u8, read_fixed_u8_slice, read_fixed_u8_nest);
fixed_reader!(i8, read_fixed_i8, read_fixed_i8_slice, read_fixed_i8_nest);
fixed_reader!(
    u16,
    read_fixed_u16,
    read_fixed_u16_slice,
    read_fixed_u16_nest
);
fixed_reader!(
    i16,
    read_fixed_i16,
    read_fixed_i16_slice,
    read_fixed_i16_nest
);
fixed_reader!(
    u32,
    read_fixed_u32,
    read_fixed_u32_slice,
    read_fixed_u32_nest
);
fixed_reader!(
    i32,
    read_fixed_i32,
    read_fixed_i32_slice,
    read_fixed_i32_nest
);
fixed_reader!(
    u64,
    read_fixed_u64,
    read_fixed_u64_slice,
    read_fixed_u64_nest
);
fixed_reader!(
    i64,
    read_fixed_i64,
    read_fixed_i64_slice,
    read_fixed_i64_nest
);
fixed_reader!(
    half::f16,
    read_fixed_f16,
    read_fixed_f16_slice,
    read_fixed_f16_nest
);
fixed_reader!(
    f32,
    read_fixed_f32,
    read_fixed_f32_slice,
    read_fixed_f32_nest
);
fixed_reader!(
    f64,
    read_fixed_f64,
    read_fixed_f64_slice,
    read_fixed_f64_nest
);
fixed_reader!(
    Time,
    read_fixed_time,
    read_fixed_time_slice,
    read_fixed_time_nest
);

// Bool — a bare `[bool; M]` is its `M` bits packed inline at the data offset
// and reads by value; a run of them is `count * M` bits back to back and
// borrows as a `FixedBoolSlice`.

impl<const M: usize> ForeignReader for [bool; M] {
    type Ref<'a, W: StructureWord> = [bool; M];

    type NestedRef<'a, W: StructureWord> = FixedBoolSlice<'a, M>;

    fn foreign_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
        buffer: SplitSource<'_, Inodes, FIXED>,
    ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
        let bits = buffer.read_fixed_bool::<O, M>();
        Ok(std::array::from_fn(|j| bits[j]))
    }

    fn foreign_iter_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
        buffer: SplitSource<'_, Inodes, FIXED>,
    ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
        buffer.read_fixed_bool_slice::<O, M>()
    }
}

macro_rules! bool_fixed_reader {
    (@list $self:ty) => {
        impl<const M: usize> ForeignReader for $self {
            type Ref<'a, W: StructureWord> = FixedBoolSlice<'a, M>;

            type NestedRef<'a, W: StructureWord> = FixedBoolNested<'a, W, M, 1>;

            fn foreign_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
                buffer.read_fixed_bool_slice::<O, M>()
            }

            fn foreign_iter_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
                buffer.read_fixed_bool_nest::<O, M, 1>()
            }
        }
    };

    (@nested $self:ty, $ref_n:literal, $nest_n:literal) => {
        impl<const M: usize> ForeignReader for $self {
            type Ref<'a, W: StructureWord> = FixedBoolNested<'a, W, M, $ref_n>;

            type NestedRef<'a, W: StructureWord> = FixedBoolNested<'a, W, M, $nest_n>;

            fn foreign_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::Ref<'_, Inodes::Word>, WireError> {
                buffer.read_fixed_bool_nest::<O, M, $ref_n>()
            }

            fn foreign_iter_read<Inodes: Layout, O: InodeOffset, const FIXED: usize>(
                buffer: SplitSource<'_, Inodes, FIXED>,
            ) -> Result<Self::NestedRef<'_, Inodes::Word>, WireError> {
                buffer.read_fixed_bool_nest::<O, M, $nest_n>()
            }
        }
    };
}

bool_fixed_reader!(@list [[bool; M]]);
bool_fixed_reader!(@list Vec<[bool; M]>);
bool_fixed_reader!(@nested [Vec<[bool; M]>], 1, 2);
bool_fixed_reader!(@nested Vec<Vec<[bool; M]>>, 1, 2);
bool_fixed_reader!(@nested [Vec<Vec<[bool; M]>>], 2, 3);
bool_fixed_reader!(@nested Vec<Vec<Vec<[bool; M]>>>, 2, 3);
bool_fixed_reader!(@nested [Vec<Vec<Vec<[bool; M]>>>], 3, 4);
bool_fixed_reader!(@nested Vec<Vec<Vec<Vec<[bool; M]>>>>, 3, 4);
bool_fixed_reader!(@nested [Vec<Vec<Vec<Vec<[bool; M]>>>>], 4, 5);
bool_fixed_reader!(@nested Vec<Vec<Vec<Vec<Vec<[bool; M]>>>>>, 4, 5);
bool_fixed_reader!(@nested [Vec<Vec<Vec<Vec<Vec<[bool; M]>>>>>], 5, 6);
bool_fixed_reader!(@nested Vec<Vec<Vec<Vec<Vec<Vec<[bool; M]>>>>>>, 5, 6);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::offsets::Root;
    use pyro_schema::{Inode, KindMeta, WireKind};

    struct L;
    impl Layout for L {
        type Word = u32;
        const RAW_LEN: usize = 1;
        const RAW: &'static [Inode] = Self::INODES;
        const INODES: &'static [Inode] = &[Inode {
            kind: WireKind::U8,
            kind_meta: KindMeta::Primitive {
                nesting: 3,
                data_offset: 8,
                topology_offset: 0,
                fixed_size: 2,
                nullable_bit_index: 0,
                group: false,
                group_nest: false,
            },
        }];
        const FIXED_SIZE: usize = 16;
        const VALIDITY_START: usize = 16;
        const DOC: &'static str = "";
        const FLAT_LEN: usize = 1;
        const FLAT_NAMES: &'static [&'static str] = &["a"];
        const FLAT_DOCS: &'static [&'static str] = &[""];
        const PYRO_TYPE: pyro_schema::PyroType = pyro_schema::const_pyro_type!(<L as Layout>::RAW);
        const SCHEMA_HASH: u64 = 0;
    }

    /// `NESTED_3_FIXED_U8` reads back as the arrays `writers/fixed.rs` wrote.
    #[test]
    fn triple_nested_fixed_reads_the_fixture() {
        let data: Vec<Vec<Vec<[u8; 2]>>> = vec![
            vec![vec![[1, 2], [3, 4]], vec![[5, 6]]],
            vec![vec![[7, 8], [9, 10], [11, 12]]],
        ];
        let buffer = crate::wire::tests::primitive_3_fixed_nesting_u32::NESTED_3_FIXED_U8.to_vec();
        let source = || SplitSource::<L, { L::FIXED_SIZE }>::from_payload(&buffer).unwrap();

        let read =
            <Vec<Vec<Vec<[u8; 2]>>>>::foreign_read::<L, Root, { L::FIXED_SIZE }>(source()).unwrap();
        assert_eq!(read, data);
        let read =
            <[Vec<Vec<[u8; 2]>>]>::foreign_read::<L, Root, { L::FIXED_SIZE }>(source()).unwrap();
        assert_eq!(read, data);
    }
}
