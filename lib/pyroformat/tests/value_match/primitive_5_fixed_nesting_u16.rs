//! Struct-vs-value match for `src/wire/tests/primitive_5_fixed_nesting_u16.rs` —
//! `List<List<List<List<List<[T; 2]>>>>>` against a derived struct with a native
//! `Vec<Vec<Vec<Vec<Vec<[T; 2]>>>>>` field. See `tests/value_match/primitive_u16.rs`'s
//! module doc for the shared harness rationale.

use half::f16;

use pyroformat::wire::tests::primitive_5_fixed_nesting_u16::*;

use crate::support::bits::lsb_bools;
use crate::support::read_back::assert_roundtrip;
use pyroformat::value::Time;
use pyroformat::{Layout, Reader, Writer};

macro_rules! value_match_nested_5_fixed_test {
    ($test_name:ident, $struct_ty:ident, $field:expr, $buf_const:expr, $inodes_const:expr) => {
        #[test]
        fn $test_name() {
            let data = $struct_ty { a: $field };
            assert_roundtrip(&data, &$buf_const, &$inodes_const);
        }
    };
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5FixedU8 {
    a: Vec<Vec<Vec<Vec<Vec<[u8; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5FixedI8 {
    a: Vec<Vec<Vec<Vec<Vec<[i8; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5FixedU16 {
    a: Vec<Vec<Vec<Vec<Vec<[u16; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5FixedI16 {
    a: Vec<Vec<Vec<Vec<Vec<[i16; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5FixedU32 {
    a: Vec<Vec<Vec<Vec<Vec<[u32; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5FixedI32 {
    a: Vec<Vec<Vec<Vec<Vec<[i32; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5FixedU64 {
    a: Vec<Vec<Vec<Vec<Vec<[u64; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5FixedI64 {
    a: Vec<Vec<Vec<Vec<Vec<[i64; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5FixedF16 {
    a: Vec<Vec<Vec<Vec<Vec<[f16; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5FixedF32 {
    a: Vec<Vec<Vec<Vec<Vec<[f32; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5FixedF64 {
    a: Vec<Vec<Vec<Vec<Vec<[f64; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5FixedTimestamp {
    a: Vec<Vec<Vec<Vec<Vec<[Time; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5FixedBool {
    a: Vec<Vec<Vec<Vec<Vec<[bool; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5FixedBool8 {
    a: Vec<Vec<Vec<Vec<Vec<[bool; 8]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5FixedBool16 {
    a: Vec<Vec<Vec<Vec<Vec<[bool; 16]>>>>>,
}

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_u8,
    PNested5FixedU8,
    vec![
        vec![
            vec![vec![vec![[1, 2], [3, 4]]], vec![vec![[5, 6]]]],
            vec![vec![vec![[7, 8]]]]
        ],
        vec![vec![vec![vec![[9, 10], [11, 12]]]]]
    ],
    NESTED_5_FIXED_U8,
    NESTED_5_FIXED_U8_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_i8,
    PNested5FixedI8,
    vec![
        vec![
            vec![vec![vec![[-1, 2], [-3, 4]]], vec![vec![[-5, 6]]]],
            vec![vec![vec![[-7, 8]]]]
        ],
        vec![vec![vec![vec![[-9, 10], [-11, 12]]]]]
    ],
    NESTED_5_FIXED_I8,
    NESTED_5_FIXED_I8_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_u16,
    PNested5FixedU16,
    vec![
        vec![
            vec![vec![vec![[1, 2], [3, 4]]], vec![vec![[5, 6]]]],
            vec![vec![vec![[7, 8]]]]
        ],
        vec![vec![vec![vec![[9, 10], [11, 12]]]]]
    ],
    NESTED_5_FIXED_U16,
    NESTED_5_FIXED_U16_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_i16,
    PNested5FixedI16,
    vec![
        vec![
            vec![vec![vec![[-1, 2], [-3, 4]]], vec![vec![[-5, 6]]]],
            vec![vec![vec![[-7, 8]]]]
        ],
        vec![vec![vec![vec![[-9, 10], [-11, 12]]]]]
    ],
    NESTED_5_FIXED_I16,
    NESTED_5_FIXED_I16_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_u32,
    PNested5FixedU32,
    vec![
        vec![
            vec![vec![vec![[1, 2], [3, 4]]], vec![vec![[5, 6]]]],
            vec![vec![vec![[7, 8]]]]
        ],
        vec![vec![vec![vec![[9, 10], [11, 12]]]]]
    ],
    NESTED_5_FIXED_U32,
    NESTED_5_FIXED_U32_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_i32,
    PNested5FixedI32,
    vec![
        vec![
            vec![vec![vec![[-1, 2], [-3, 4]]], vec![vec![[-5, 6]]]],
            vec![vec![vec![[-7, 8]]]]
        ],
        vec![vec![vec![vec![[-9, 10], [-11, 12]]]]]
    ],
    NESTED_5_FIXED_I32,
    NESTED_5_FIXED_I32_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_u64,
    PNested5FixedU64,
    vec![
        vec![
            vec![vec![vec![[1, 2], [3, 4]]], vec![vec![[5, 6]]]],
            vec![vec![vec![[7, 8]]]]
        ],
        vec![vec![vec![vec![[9, 10], [11, 12]]]]]
    ],
    NESTED_5_FIXED_U64,
    NESTED_5_FIXED_U64_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_i64,
    PNested5FixedI64,
    vec![
        vec![
            vec![vec![vec![[-1, 2], [-3, 4]]], vec![vec![[-5, 6]]]],
            vec![vec![vec![[-7, 8]]]]
        ],
        vec![vec![vec![vec![[-9, 10], [-11, 12]]]]]
    ],
    NESTED_5_FIXED_I64,
    NESTED_5_FIXED_I64_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_f16,
    PNested5FixedF16,
    vec![
        vec![
            vec![
                vec![vec![
                    [f16::from_f32(1.5), f16::from_f32(-2.0)],
                    [f16::from_f32(3.25), f16::from_f32(4.0)]
                ]],
                vec![vec![[f16::from_f32(-5.5), f16::from_f32(6.0)]]]
            ],
            vec![vec![vec![[f16::from_f32(7.5), f16::from_f32(-8.0)]]]]
        ],
        vec![vec![vec![vec![
            [f16::from_f32(9.25), f16::from_f32(10.0)],
            [f16::from_f32(-11.5), f16::from_f32(12.0)]
        ]]]]
    ],
    NESTED_5_FIXED_F16,
    NESTED_5_FIXED_F16_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_f32,
    PNested5FixedF32,
    vec![
        vec![
            vec![
                vec![vec![[1.5, -2.0], [3.25, 4.0]]],
                vec![vec![[-5.5, 6.0]]]
            ],
            vec![vec![vec![[7.5, -8.0]]]]
        ],
        vec![vec![vec![vec![[9.25, 10.0], [-11.5, 12.0]]]]]
    ],
    NESTED_5_FIXED_F32,
    NESTED_5_FIXED_F32_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_f64,
    PNested5FixedF64,
    vec![
        vec![
            vec![
                vec![vec![[1.5, -2.0], [3.25, 4.0]]],
                vec![vec![[-5.5, 6.0]]]
            ],
            vec![vec![vec![[7.5, -8.0]]]]
        ],
        vec![vec![vec![vec![[9.25, 10.0], [-11.5, 12.0]]]]]
    ],
    NESTED_5_FIXED_F64,
    NESTED_5_FIXED_F64_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_bool,
    PNested5FixedBool,
    vec![
        vec![
            vec![
                vec![vec![[true, false], [true, true]]],
                vec![vec![[true, false]]]
            ],
            vec![vec![vec![[false, true]]]]
        ],
        vec![vec![vec![vec![[true, false], [false, false]]]]]
    ],
    NESTED_5_FIXED_BOOL,
    NESTED_5_FIXED_BOOL_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_bool_8,
    PNested5FixedBool8,
    vec![
        vec![
            vec![
                vec![vec![lsb_bools::<8>(0x01), lsb_bools::<8>(0x80)]],
                vec![vec![lsb_bools::<8>(0xFF)]]
            ],
            vec![vec![vec![lsb_bools::<8>(0x00)]]]
        ],
        vec![vec![vec![vec![lsb_bools::<8>(0xA5), lsb_bools::<8>(0x3C)]]]]
    ],
    NESTED_5_FIXED_BOOL_8,
    NESTED_5_FIXED_BOOL_8_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_bool_16,
    PNested5FixedBool16,
    vec![
        vec![
            vec![
                vec![vec![lsb_bools::<16>(0x0001), lsb_bools::<16>(0x8000)]],
                vec![vec![lsb_bools::<16>(0xFFFF)]]
            ],
            vec![vec![vec![lsb_bools::<16>(0x0000)]]]
        ],
        vec![vec![vec![vec![
            lsb_bools::<16>(0xA55A),
            lsb_bools::<16>(0x3CC3)
        ]]]]
    ],
    NESTED_5_FIXED_BOOL_16,
    NESTED_5_FIXED_BOOL_16_INODES
);

// ---------------------------------------------------------------------------
// Empty lists at every depth (`_EMPTY_d`: the list at depth `d` is empty)
// ---------------------------------------------------------------------------
value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_u8_empty_0,
    PNested5FixedU8,
    vec![],
    NESTED_5_FIXED_U8_EMPTY_0,
    NESTED_5_FIXED_U8_EMPTY_0_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_u8_empty_1,
    PNested5FixedU8,
    vec![vec![]],
    NESTED_5_FIXED_U8_EMPTY_1,
    NESTED_5_FIXED_U8_EMPTY_1_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_u8_empty_2,
    PNested5FixedU8,
    vec![vec![vec![]]],
    NESTED_5_FIXED_U8_EMPTY_2,
    NESTED_5_FIXED_U8_EMPTY_2_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_u8_empty_3,
    PNested5FixedU8,
    vec![vec![vec![vec![]]]],
    NESTED_5_FIXED_U8_EMPTY_3,
    NESTED_5_FIXED_U8_EMPTY_3_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_u8_empty_4,
    PNested5FixedU8,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_FIXED_U8_EMPTY_4,
    NESTED_5_FIXED_U8_EMPTY_4_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_i8_empty_0,
    PNested5FixedI8,
    vec![],
    NESTED_5_FIXED_I8_EMPTY_0,
    NESTED_5_FIXED_I8_EMPTY_0_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_i8_empty_1,
    PNested5FixedI8,
    vec![vec![]],
    NESTED_5_FIXED_I8_EMPTY_1,
    NESTED_5_FIXED_I8_EMPTY_1_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_i8_empty_2,
    PNested5FixedI8,
    vec![vec![vec![]]],
    NESTED_5_FIXED_I8_EMPTY_2,
    NESTED_5_FIXED_I8_EMPTY_2_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_i8_empty_3,
    PNested5FixedI8,
    vec![vec![vec![vec![]]]],
    NESTED_5_FIXED_I8_EMPTY_3,
    NESTED_5_FIXED_I8_EMPTY_3_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_i8_empty_4,
    PNested5FixedI8,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_FIXED_I8_EMPTY_4,
    NESTED_5_FIXED_I8_EMPTY_4_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_u16_empty_0,
    PNested5FixedU16,
    vec![],
    NESTED_5_FIXED_U16_EMPTY_0,
    NESTED_5_FIXED_U16_EMPTY_0_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_u16_empty_1,
    PNested5FixedU16,
    vec![vec![]],
    NESTED_5_FIXED_U16_EMPTY_1,
    NESTED_5_FIXED_U16_EMPTY_1_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_u16_empty_2,
    PNested5FixedU16,
    vec![vec![vec![]]],
    NESTED_5_FIXED_U16_EMPTY_2,
    NESTED_5_FIXED_U16_EMPTY_2_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_u16_empty_3,
    PNested5FixedU16,
    vec![vec![vec![vec![]]]],
    NESTED_5_FIXED_U16_EMPTY_3,
    NESTED_5_FIXED_U16_EMPTY_3_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_u16_empty_4,
    PNested5FixedU16,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_FIXED_U16_EMPTY_4,
    NESTED_5_FIXED_U16_EMPTY_4_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_i16_empty_0,
    PNested5FixedI16,
    vec![],
    NESTED_5_FIXED_I16_EMPTY_0,
    NESTED_5_FIXED_I16_EMPTY_0_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_i16_empty_1,
    PNested5FixedI16,
    vec![vec![]],
    NESTED_5_FIXED_I16_EMPTY_1,
    NESTED_5_FIXED_I16_EMPTY_1_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_i16_empty_2,
    PNested5FixedI16,
    vec![vec![vec![]]],
    NESTED_5_FIXED_I16_EMPTY_2,
    NESTED_5_FIXED_I16_EMPTY_2_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_i16_empty_3,
    PNested5FixedI16,
    vec![vec![vec![vec![]]]],
    NESTED_5_FIXED_I16_EMPTY_3,
    NESTED_5_FIXED_I16_EMPTY_3_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_i16_empty_4,
    PNested5FixedI16,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_FIXED_I16_EMPTY_4,
    NESTED_5_FIXED_I16_EMPTY_4_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_u32_empty_0,
    PNested5FixedU32,
    vec![],
    NESTED_5_FIXED_U32_EMPTY_0,
    NESTED_5_FIXED_U32_EMPTY_0_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_u32_empty_1,
    PNested5FixedU32,
    vec![vec![]],
    NESTED_5_FIXED_U32_EMPTY_1,
    NESTED_5_FIXED_U32_EMPTY_1_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_u32_empty_2,
    PNested5FixedU32,
    vec![vec![vec![]]],
    NESTED_5_FIXED_U32_EMPTY_2,
    NESTED_5_FIXED_U32_EMPTY_2_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_u32_empty_3,
    PNested5FixedU32,
    vec![vec![vec![vec![]]]],
    NESTED_5_FIXED_U32_EMPTY_3,
    NESTED_5_FIXED_U32_EMPTY_3_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_u32_empty_4,
    PNested5FixedU32,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_FIXED_U32_EMPTY_4,
    NESTED_5_FIXED_U32_EMPTY_4_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_i32_empty_0,
    PNested5FixedI32,
    vec![],
    NESTED_5_FIXED_I32_EMPTY_0,
    NESTED_5_FIXED_I32_EMPTY_0_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_i32_empty_1,
    PNested5FixedI32,
    vec![vec![]],
    NESTED_5_FIXED_I32_EMPTY_1,
    NESTED_5_FIXED_I32_EMPTY_1_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_i32_empty_2,
    PNested5FixedI32,
    vec![vec![vec![]]],
    NESTED_5_FIXED_I32_EMPTY_2,
    NESTED_5_FIXED_I32_EMPTY_2_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_i32_empty_3,
    PNested5FixedI32,
    vec![vec![vec![vec![]]]],
    NESTED_5_FIXED_I32_EMPTY_3,
    NESTED_5_FIXED_I32_EMPTY_3_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_i32_empty_4,
    PNested5FixedI32,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_FIXED_I32_EMPTY_4,
    NESTED_5_FIXED_I32_EMPTY_4_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_u64_empty_0,
    PNested5FixedU64,
    vec![],
    NESTED_5_FIXED_U64_EMPTY_0,
    NESTED_5_FIXED_U64_EMPTY_0_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_u64_empty_1,
    PNested5FixedU64,
    vec![vec![]],
    NESTED_5_FIXED_U64_EMPTY_1,
    NESTED_5_FIXED_U64_EMPTY_1_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_u64_empty_2,
    PNested5FixedU64,
    vec![vec![vec![]]],
    NESTED_5_FIXED_U64_EMPTY_2,
    NESTED_5_FIXED_U64_EMPTY_2_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_u64_empty_3,
    PNested5FixedU64,
    vec![vec![vec![vec![]]]],
    NESTED_5_FIXED_U64_EMPTY_3,
    NESTED_5_FIXED_U64_EMPTY_3_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_u64_empty_4,
    PNested5FixedU64,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_FIXED_U64_EMPTY_4,
    NESTED_5_FIXED_U64_EMPTY_4_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_i64_empty_0,
    PNested5FixedI64,
    vec![],
    NESTED_5_FIXED_I64_EMPTY_0,
    NESTED_5_FIXED_I64_EMPTY_0_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_i64_empty_1,
    PNested5FixedI64,
    vec![vec![]],
    NESTED_5_FIXED_I64_EMPTY_1,
    NESTED_5_FIXED_I64_EMPTY_1_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_i64_empty_2,
    PNested5FixedI64,
    vec![vec![vec![]]],
    NESTED_5_FIXED_I64_EMPTY_2,
    NESTED_5_FIXED_I64_EMPTY_2_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_i64_empty_3,
    PNested5FixedI64,
    vec![vec![vec![vec![]]]],
    NESTED_5_FIXED_I64_EMPTY_3,
    NESTED_5_FIXED_I64_EMPTY_3_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_i64_empty_4,
    PNested5FixedI64,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_FIXED_I64_EMPTY_4,
    NESTED_5_FIXED_I64_EMPTY_4_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_f16_empty_0,
    PNested5FixedF16,
    vec![],
    NESTED_5_FIXED_F16_EMPTY_0,
    NESTED_5_FIXED_F16_EMPTY_0_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_f16_empty_1,
    PNested5FixedF16,
    vec![vec![]],
    NESTED_5_FIXED_F16_EMPTY_1,
    NESTED_5_FIXED_F16_EMPTY_1_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_f16_empty_2,
    PNested5FixedF16,
    vec![vec![vec![]]],
    NESTED_5_FIXED_F16_EMPTY_2,
    NESTED_5_FIXED_F16_EMPTY_2_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_f16_empty_3,
    PNested5FixedF16,
    vec![vec![vec![vec![]]]],
    NESTED_5_FIXED_F16_EMPTY_3,
    NESTED_5_FIXED_F16_EMPTY_3_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_f16_empty_4,
    PNested5FixedF16,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_FIXED_F16_EMPTY_4,
    NESTED_5_FIXED_F16_EMPTY_4_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_f32_empty_0,
    PNested5FixedF32,
    vec![],
    NESTED_5_FIXED_F32_EMPTY_0,
    NESTED_5_FIXED_F32_EMPTY_0_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_f32_empty_1,
    PNested5FixedF32,
    vec![vec![]],
    NESTED_5_FIXED_F32_EMPTY_1,
    NESTED_5_FIXED_F32_EMPTY_1_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_f32_empty_2,
    PNested5FixedF32,
    vec![vec![vec![]]],
    NESTED_5_FIXED_F32_EMPTY_2,
    NESTED_5_FIXED_F32_EMPTY_2_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_f32_empty_3,
    PNested5FixedF32,
    vec![vec![vec![vec![]]]],
    NESTED_5_FIXED_F32_EMPTY_3,
    NESTED_5_FIXED_F32_EMPTY_3_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_f32_empty_4,
    PNested5FixedF32,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_FIXED_F32_EMPTY_4,
    NESTED_5_FIXED_F32_EMPTY_4_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_f64_empty_0,
    PNested5FixedF64,
    vec![],
    NESTED_5_FIXED_F64_EMPTY_0,
    NESTED_5_FIXED_F64_EMPTY_0_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_f64_empty_1,
    PNested5FixedF64,
    vec![vec![]],
    NESTED_5_FIXED_F64_EMPTY_1,
    NESTED_5_FIXED_F64_EMPTY_1_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_f64_empty_2,
    PNested5FixedF64,
    vec![vec![vec![]]],
    NESTED_5_FIXED_F64_EMPTY_2,
    NESTED_5_FIXED_F64_EMPTY_2_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_f64_empty_3,
    PNested5FixedF64,
    vec![vec![vec![vec![]]]],
    NESTED_5_FIXED_F64_EMPTY_3,
    NESTED_5_FIXED_F64_EMPTY_3_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_f64_empty_4,
    PNested5FixedF64,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_FIXED_F64_EMPTY_4,
    NESTED_5_FIXED_F64_EMPTY_4_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_bool_empty_0,
    PNested5FixedBool,
    vec![],
    NESTED_5_FIXED_BOOL_EMPTY_0,
    NESTED_5_FIXED_BOOL_EMPTY_0_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_bool_empty_1,
    PNested5FixedBool,
    vec![vec![]],
    NESTED_5_FIXED_BOOL_EMPTY_1,
    NESTED_5_FIXED_BOOL_EMPTY_1_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_bool_empty_2,
    PNested5FixedBool,
    vec![vec![vec![]]],
    NESTED_5_FIXED_BOOL_EMPTY_2,
    NESTED_5_FIXED_BOOL_EMPTY_2_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_bool_empty_3,
    PNested5FixedBool,
    vec![vec![vec![vec![]]]],
    NESTED_5_FIXED_BOOL_EMPTY_3,
    NESTED_5_FIXED_BOOL_EMPTY_3_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_bool_empty_4,
    PNested5FixedBool,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_FIXED_BOOL_EMPTY_4,
    NESTED_5_FIXED_BOOL_EMPTY_4_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_bool_8_empty_0,
    PNested5FixedBool8,
    vec![],
    NESTED_5_FIXED_BOOL_8_EMPTY_0,
    NESTED_5_FIXED_BOOL_8_EMPTY_0_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_bool_8_empty_1,
    PNested5FixedBool8,
    vec![vec![]],
    NESTED_5_FIXED_BOOL_8_EMPTY_1,
    NESTED_5_FIXED_BOOL_8_EMPTY_1_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_bool_8_empty_2,
    PNested5FixedBool8,
    vec![vec![vec![]]],
    NESTED_5_FIXED_BOOL_8_EMPTY_2,
    NESTED_5_FIXED_BOOL_8_EMPTY_2_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_bool_8_empty_3,
    PNested5FixedBool8,
    vec![vec![vec![vec![]]]],
    NESTED_5_FIXED_BOOL_8_EMPTY_3,
    NESTED_5_FIXED_BOOL_8_EMPTY_3_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_bool_8_empty_4,
    PNested5FixedBool8,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_FIXED_BOOL_8_EMPTY_4,
    NESTED_5_FIXED_BOOL_8_EMPTY_4_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_bool_16_empty_0,
    PNested5FixedBool16,
    vec![],
    NESTED_5_FIXED_BOOL_16_EMPTY_0,
    NESTED_5_FIXED_BOOL_16_EMPTY_0_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_bool_16_empty_1,
    PNested5FixedBool16,
    vec![vec![]],
    NESTED_5_FIXED_BOOL_16_EMPTY_1,
    NESTED_5_FIXED_BOOL_16_EMPTY_1_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_bool_16_empty_2,
    PNested5FixedBool16,
    vec![vec![vec![]]],
    NESTED_5_FIXED_BOOL_16_EMPTY_2,
    NESTED_5_FIXED_BOOL_16_EMPTY_2_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_bool_16_empty_3,
    PNested5FixedBool16,
    vec![vec![vec![vec![]]]],
    NESTED_5_FIXED_BOOL_16_EMPTY_3,
    NESTED_5_FIXED_BOOL_16_EMPTY_3_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_bool_16_empty_4,
    PNested5FixedBool16,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_FIXED_BOOL_16_EMPTY_4,
    NESTED_5_FIXED_BOOL_16_EMPTY_4_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_timestamp,
    PNested5FixedTimestamp,
    vec![
        vec![
            vec![
                vec![vec![
                    [Time(1_500_000_000), Time(-2_000_000_000)],
                    [Time(3_250_000_000), Time(4_000_000_000)]
                ]],
                vec![vec![[Time(-5_500_000_000), Time(6_000_000_000)]]]
            ],
            vec![vec![vec![[Time(7_500_000_000), Time(-8_000_000_000)]]]]
        ],
        vec![vec![vec![vec![
            [Time(9_250_000_000), Time(10_000_000_000)],
            [Time(-11_500_000_000), Time(12_000_000_000)]
        ]]]]
    ],
    NESTED_5_FIXED_TIMESTAMP,
    NESTED_5_FIXED_TIMESTAMP_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_timestamp_empty_0,
    PNested5FixedTimestamp,
    vec![],
    NESTED_5_FIXED_TIMESTAMP_EMPTY_0,
    NESTED_5_FIXED_TIMESTAMP_EMPTY_0_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_timestamp_empty_1,
    PNested5FixedTimestamp,
    vec![vec![]],
    NESTED_5_FIXED_TIMESTAMP_EMPTY_1,
    NESTED_5_FIXED_TIMESTAMP_EMPTY_1_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_timestamp_empty_2,
    PNested5FixedTimestamp,
    vec![vec![vec![]]],
    NESTED_5_FIXED_TIMESTAMP_EMPTY_2,
    NESTED_5_FIXED_TIMESTAMP_EMPTY_2_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_timestamp_empty_3,
    PNested5FixedTimestamp,
    vec![vec![vec![vec![]]]],
    NESTED_5_FIXED_TIMESTAMP_EMPTY_3,
    NESTED_5_FIXED_TIMESTAMP_EMPTY_3_INODES
);

value_match_nested_5_fixed_test!(
    value_match_nested_5_fixed_timestamp_empty_4,
    PNested5FixedTimestamp,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_FIXED_TIMESTAMP_EMPTY_4,
    NESTED_5_FIXED_TIMESTAMP_EMPTY_4_INODES
);
