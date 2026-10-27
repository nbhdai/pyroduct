//! Struct-vs-value match for `src/wire/tests/primitive_4_option_fixed_nesting_u32.rs` —
//! `List<List<List<List<Option<[T; 2]>>>>>` against a derived struct with a native
//! `Vec<Vec<Vec<Vec<Option<[T; 2]>>>>>` field. See `tests/value_match/primitive_u32.rs`'s
//! module doc for the shared harness rationale.

use half::f16;

use pyroformat::wire::tests::primitive_4_option_fixed_nesting_u32::*;

use crate::support::bits::lsb_bools;
use crate::support::read_back::assert_roundtrip;
use pyroformat::value::Time;
use pyroformat::{Layout, Reader, Writer};

macro_rules! value_match_nested_4_option_fixed_test {
    ($test_name:ident, $struct_ty:ident, $field:expr, $buf_const:expr, $inodes_const:expr) => {
        #[test]
        fn $test_name() {
            let data = $struct_ty { a: $field };
            assert_roundtrip(&data, &$buf_const, &$inodes_const);
        }
    };
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionFixedU8 {
    a: Vec<Vec<Vec<Vec<Option<[u8; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionFixedI8 {
    a: Vec<Vec<Vec<Vec<Option<[i8; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionFixedU16 {
    a: Vec<Vec<Vec<Vec<Option<[u16; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionFixedI16 {
    a: Vec<Vec<Vec<Vec<Option<[i16; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionFixedU32 {
    a: Vec<Vec<Vec<Vec<Option<[u32; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionFixedI32 {
    a: Vec<Vec<Vec<Vec<Option<[i32; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionFixedU64 {
    a: Vec<Vec<Vec<Vec<Option<[u64; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionFixedI64 {
    a: Vec<Vec<Vec<Vec<Option<[i64; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionFixedF16 {
    a: Vec<Vec<Vec<Vec<Option<[f16; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionFixedF32 {
    a: Vec<Vec<Vec<Vec<Option<[f32; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionFixedF64 {
    a: Vec<Vec<Vec<Vec<Option<[f64; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionFixedTimestamp {
    a: Vec<Vec<Vec<Vec<Option<[Time; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionFixedBool {
    a: Vec<Vec<Vec<Vec<Option<[bool; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionFixedBool8 {
    a: Vec<Vec<Vec<Vec<Option<[bool; 8]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionFixedBool16 {
    a: Vec<Vec<Vec<Vec<Option<[bool; 16]>>>>>,
}

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_u8,
    PNested4OptionFixedU8,
    vec![
        vec![
            vec![vec![Some([1, 2]), None], vec![Some([5, 6])]],
            vec![vec![Some([7, 8])]]
        ],
        vec![vec![vec![None, Some([11, 12])]]]
    ],
    NESTED_4_OPTION_FIXED_U8,
    NESTED_4_OPTION_FIXED_U8_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_i8,
    PNested4OptionFixedI8,
    vec![
        vec![
            vec![vec![Some([-1, 2]), None], vec![Some([-5, 6])]],
            vec![vec![Some([-7, 8])]]
        ],
        vec![vec![vec![None, Some([-11, 12])]]]
    ],
    NESTED_4_OPTION_FIXED_I8,
    NESTED_4_OPTION_FIXED_I8_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_u16,
    PNested4OptionFixedU16,
    vec![
        vec![
            vec![vec![Some([1, 2]), None], vec![Some([5, 6])]],
            vec![vec![Some([7, 8])]]
        ],
        vec![vec![vec![None, Some([11, 12])]]]
    ],
    NESTED_4_OPTION_FIXED_U16,
    NESTED_4_OPTION_FIXED_U16_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_i16,
    PNested4OptionFixedI16,
    vec![
        vec![
            vec![vec![Some([-1, 2]), None], vec![Some([-5, 6])]],
            vec![vec![Some([-7, 8])]]
        ],
        vec![vec![vec![None, Some([-11, 12])]]]
    ],
    NESTED_4_OPTION_FIXED_I16,
    NESTED_4_OPTION_FIXED_I16_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_u32,
    PNested4OptionFixedU32,
    vec![
        vec![
            vec![vec![Some([1, 2]), None], vec![Some([5, 6])]],
            vec![vec![Some([7, 8])]]
        ],
        vec![vec![vec![None, Some([11, 12])]]]
    ],
    NESTED_4_OPTION_FIXED_U32,
    NESTED_4_OPTION_FIXED_U32_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_i32,
    PNested4OptionFixedI32,
    vec![
        vec![
            vec![vec![Some([-1, 2]), None], vec![Some([-5, 6])]],
            vec![vec![Some([-7, 8])]]
        ],
        vec![vec![vec![None, Some([-11, 12])]]]
    ],
    NESTED_4_OPTION_FIXED_I32,
    NESTED_4_OPTION_FIXED_I32_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_u64,
    PNested4OptionFixedU64,
    vec![
        vec![
            vec![vec![Some([1, 2]), None], vec![Some([5, 6])]],
            vec![vec![Some([7, 8])]]
        ],
        vec![vec![vec![None, Some([11, 12])]]]
    ],
    NESTED_4_OPTION_FIXED_U64,
    NESTED_4_OPTION_FIXED_U64_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_i64,
    PNested4OptionFixedI64,
    vec![
        vec![
            vec![vec![Some([-1, 2]), None], vec![Some([-5, 6])]],
            vec![vec![Some([-7, 8])]]
        ],
        vec![vec![vec![None, Some([-11, 12])]]]
    ],
    NESTED_4_OPTION_FIXED_I64,
    NESTED_4_OPTION_FIXED_I64_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_f16,
    PNested4OptionFixedF16,
    vec![
        vec![
            vec![
                vec![Some([f16::from_f32(1.5), f16::from_f32(-2.0)]), None],
                vec![Some([f16::from_f32(-5.5), f16::from_f32(6.0)])]
            ],
            vec![vec![Some([f16::from_f32(7.5), f16::from_f32(-8.0)])]]
        ],
        vec![vec![vec![
            None,
            Some([f16::from_f32(-11.5), f16::from_f32(12.0)])
        ]]]
    ],
    NESTED_4_OPTION_FIXED_F16,
    NESTED_4_OPTION_FIXED_F16_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_f32,
    PNested4OptionFixedF32,
    vec![
        vec![
            vec![vec![Some([1.5, -2.0]), None], vec![Some([-5.5, 6.0])]],
            vec![vec![Some([7.5, -8.0])]]
        ],
        vec![vec![vec![None, Some([-11.5, 12.0])]]]
    ],
    NESTED_4_OPTION_FIXED_F32,
    NESTED_4_OPTION_FIXED_F32_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_f64,
    PNested4OptionFixedF64,
    vec![
        vec![
            vec![vec![Some([1.5, -2.0]), None], vec![Some([-5.5, 6.0])]],
            vec![vec![Some([7.5, -8.0])]]
        ],
        vec![vec![vec![None, Some([-11.5, 12.0])]]]
    ],
    NESTED_4_OPTION_FIXED_F64,
    NESTED_4_OPTION_FIXED_F64_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_bool,
    PNested4OptionFixedBool,
    vec![
        vec![
            vec![vec![Some([true, false]), None], vec![Some([true, false])]],
            vec![vec![Some([false, true])]]
        ],
        vec![vec![vec![None, Some([false, false])]]]
    ],
    NESTED_4_OPTION_FIXED_BOOL,
    NESTED_4_OPTION_FIXED_BOOL_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_bool_8,
    PNested4OptionFixedBool8,
    vec![
        vec![
            vec![
                vec![Some(lsb_bools::<8>(0x01)), None],
                vec![Some(lsb_bools::<8>(0x80))]
            ],
            vec![vec![Some(lsb_bools::<8>(0xFF))]]
        ],
        vec![vec![vec![None, Some(lsb_bools::<8>(0x00))]]]
    ],
    NESTED_4_OPTION_FIXED_BOOL_8,
    NESTED_4_OPTION_FIXED_BOOL_8_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_bool_16,
    PNested4OptionFixedBool16,
    vec![
        vec![
            vec![
                vec![Some(lsb_bools::<16>(0x0001)), None],
                vec![Some(lsb_bools::<16>(0x8000))]
            ],
            vec![vec![Some(lsb_bools::<16>(0xFFFF))]]
        ],
        vec![vec![vec![None, Some(lsb_bools::<16>(0x0000))]]]
    ],
    NESTED_4_OPTION_FIXED_BOOL_16,
    NESTED_4_OPTION_FIXED_BOOL_16_INODES
);

// ---------------------------------------------------------------------------
// Empty lists at every depth (`_EMPTY_d`: the list at depth `d` is empty)
// ---------------------------------------------------------------------------
value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_u8_empty_0,
    PNested4OptionFixedU8,
    vec![],
    NESTED_4_OPTION_FIXED_U8_EMPTY_0,
    NESTED_4_OPTION_FIXED_U8_EMPTY_0_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_u8_empty_1,
    PNested4OptionFixedU8,
    vec![vec![]],
    NESTED_4_OPTION_FIXED_U8_EMPTY_1,
    NESTED_4_OPTION_FIXED_U8_EMPTY_1_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_u8_empty_2,
    PNested4OptionFixedU8,
    vec![vec![vec![]]],
    NESTED_4_OPTION_FIXED_U8_EMPTY_2,
    NESTED_4_OPTION_FIXED_U8_EMPTY_2_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_u8_empty_3,
    PNested4OptionFixedU8,
    vec![vec![vec![vec![]]]],
    NESTED_4_OPTION_FIXED_U8_EMPTY_3,
    NESTED_4_OPTION_FIXED_U8_EMPTY_3_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_i8_empty_0,
    PNested4OptionFixedI8,
    vec![],
    NESTED_4_OPTION_FIXED_I8_EMPTY_0,
    NESTED_4_OPTION_FIXED_I8_EMPTY_0_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_i8_empty_1,
    PNested4OptionFixedI8,
    vec![vec![]],
    NESTED_4_OPTION_FIXED_I8_EMPTY_1,
    NESTED_4_OPTION_FIXED_I8_EMPTY_1_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_i8_empty_2,
    PNested4OptionFixedI8,
    vec![vec![vec![]]],
    NESTED_4_OPTION_FIXED_I8_EMPTY_2,
    NESTED_4_OPTION_FIXED_I8_EMPTY_2_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_i8_empty_3,
    PNested4OptionFixedI8,
    vec![vec![vec![vec![]]]],
    NESTED_4_OPTION_FIXED_I8_EMPTY_3,
    NESTED_4_OPTION_FIXED_I8_EMPTY_3_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_u16_empty_0,
    PNested4OptionFixedU16,
    vec![],
    NESTED_4_OPTION_FIXED_U16_EMPTY_0,
    NESTED_4_OPTION_FIXED_U16_EMPTY_0_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_u16_empty_1,
    PNested4OptionFixedU16,
    vec![vec![]],
    NESTED_4_OPTION_FIXED_U16_EMPTY_1,
    NESTED_4_OPTION_FIXED_U16_EMPTY_1_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_u16_empty_2,
    PNested4OptionFixedU16,
    vec![vec![vec![]]],
    NESTED_4_OPTION_FIXED_U16_EMPTY_2,
    NESTED_4_OPTION_FIXED_U16_EMPTY_2_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_u16_empty_3,
    PNested4OptionFixedU16,
    vec![vec![vec![vec![]]]],
    NESTED_4_OPTION_FIXED_U16_EMPTY_3,
    NESTED_4_OPTION_FIXED_U16_EMPTY_3_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_i16_empty_0,
    PNested4OptionFixedI16,
    vec![],
    NESTED_4_OPTION_FIXED_I16_EMPTY_0,
    NESTED_4_OPTION_FIXED_I16_EMPTY_0_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_i16_empty_1,
    PNested4OptionFixedI16,
    vec![vec![]],
    NESTED_4_OPTION_FIXED_I16_EMPTY_1,
    NESTED_4_OPTION_FIXED_I16_EMPTY_1_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_i16_empty_2,
    PNested4OptionFixedI16,
    vec![vec![vec![]]],
    NESTED_4_OPTION_FIXED_I16_EMPTY_2,
    NESTED_4_OPTION_FIXED_I16_EMPTY_2_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_i16_empty_3,
    PNested4OptionFixedI16,
    vec![vec![vec![vec![]]]],
    NESTED_4_OPTION_FIXED_I16_EMPTY_3,
    NESTED_4_OPTION_FIXED_I16_EMPTY_3_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_u32_empty_0,
    PNested4OptionFixedU32,
    vec![],
    NESTED_4_OPTION_FIXED_U32_EMPTY_0,
    NESTED_4_OPTION_FIXED_U32_EMPTY_0_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_u32_empty_1,
    PNested4OptionFixedU32,
    vec![vec![]],
    NESTED_4_OPTION_FIXED_U32_EMPTY_1,
    NESTED_4_OPTION_FIXED_U32_EMPTY_1_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_u32_empty_2,
    PNested4OptionFixedU32,
    vec![vec![vec![]]],
    NESTED_4_OPTION_FIXED_U32_EMPTY_2,
    NESTED_4_OPTION_FIXED_U32_EMPTY_2_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_u32_empty_3,
    PNested4OptionFixedU32,
    vec![vec![vec![vec![]]]],
    NESTED_4_OPTION_FIXED_U32_EMPTY_3,
    NESTED_4_OPTION_FIXED_U32_EMPTY_3_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_i32_empty_0,
    PNested4OptionFixedI32,
    vec![],
    NESTED_4_OPTION_FIXED_I32_EMPTY_0,
    NESTED_4_OPTION_FIXED_I32_EMPTY_0_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_i32_empty_1,
    PNested4OptionFixedI32,
    vec![vec![]],
    NESTED_4_OPTION_FIXED_I32_EMPTY_1,
    NESTED_4_OPTION_FIXED_I32_EMPTY_1_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_i32_empty_2,
    PNested4OptionFixedI32,
    vec![vec![vec![]]],
    NESTED_4_OPTION_FIXED_I32_EMPTY_2,
    NESTED_4_OPTION_FIXED_I32_EMPTY_2_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_i32_empty_3,
    PNested4OptionFixedI32,
    vec![vec![vec![vec![]]]],
    NESTED_4_OPTION_FIXED_I32_EMPTY_3,
    NESTED_4_OPTION_FIXED_I32_EMPTY_3_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_u64_empty_0,
    PNested4OptionFixedU64,
    vec![],
    NESTED_4_OPTION_FIXED_U64_EMPTY_0,
    NESTED_4_OPTION_FIXED_U64_EMPTY_0_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_u64_empty_1,
    PNested4OptionFixedU64,
    vec![vec![]],
    NESTED_4_OPTION_FIXED_U64_EMPTY_1,
    NESTED_4_OPTION_FIXED_U64_EMPTY_1_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_u64_empty_2,
    PNested4OptionFixedU64,
    vec![vec![vec![]]],
    NESTED_4_OPTION_FIXED_U64_EMPTY_2,
    NESTED_4_OPTION_FIXED_U64_EMPTY_2_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_u64_empty_3,
    PNested4OptionFixedU64,
    vec![vec![vec![vec![]]]],
    NESTED_4_OPTION_FIXED_U64_EMPTY_3,
    NESTED_4_OPTION_FIXED_U64_EMPTY_3_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_i64_empty_0,
    PNested4OptionFixedI64,
    vec![],
    NESTED_4_OPTION_FIXED_I64_EMPTY_0,
    NESTED_4_OPTION_FIXED_I64_EMPTY_0_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_i64_empty_1,
    PNested4OptionFixedI64,
    vec![vec![]],
    NESTED_4_OPTION_FIXED_I64_EMPTY_1,
    NESTED_4_OPTION_FIXED_I64_EMPTY_1_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_i64_empty_2,
    PNested4OptionFixedI64,
    vec![vec![vec![]]],
    NESTED_4_OPTION_FIXED_I64_EMPTY_2,
    NESTED_4_OPTION_FIXED_I64_EMPTY_2_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_i64_empty_3,
    PNested4OptionFixedI64,
    vec![vec![vec![vec![]]]],
    NESTED_4_OPTION_FIXED_I64_EMPTY_3,
    NESTED_4_OPTION_FIXED_I64_EMPTY_3_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_f16_empty_0,
    PNested4OptionFixedF16,
    vec![],
    NESTED_4_OPTION_FIXED_F16_EMPTY_0,
    NESTED_4_OPTION_FIXED_F16_EMPTY_0_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_f16_empty_1,
    PNested4OptionFixedF16,
    vec![vec![]],
    NESTED_4_OPTION_FIXED_F16_EMPTY_1,
    NESTED_4_OPTION_FIXED_F16_EMPTY_1_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_f16_empty_2,
    PNested4OptionFixedF16,
    vec![vec![vec![]]],
    NESTED_4_OPTION_FIXED_F16_EMPTY_2,
    NESTED_4_OPTION_FIXED_F16_EMPTY_2_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_f16_empty_3,
    PNested4OptionFixedF16,
    vec![vec![vec![vec![]]]],
    NESTED_4_OPTION_FIXED_F16_EMPTY_3,
    NESTED_4_OPTION_FIXED_F16_EMPTY_3_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_f32_empty_0,
    PNested4OptionFixedF32,
    vec![],
    NESTED_4_OPTION_FIXED_F32_EMPTY_0,
    NESTED_4_OPTION_FIXED_F32_EMPTY_0_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_f32_empty_1,
    PNested4OptionFixedF32,
    vec![vec![]],
    NESTED_4_OPTION_FIXED_F32_EMPTY_1,
    NESTED_4_OPTION_FIXED_F32_EMPTY_1_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_f32_empty_2,
    PNested4OptionFixedF32,
    vec![vec![vec![]]],
    NESTED_4_OPTION_FIXED_F32_EMPTY_2,
    NESTED_4_OPTION_FIXED_F32_EMPTY_2_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_f32_empty_3,
    PNested4OptionFixedF32,
    vec![vec![vec![vec![]]]],
    NESTED_4_OPTION_FIXED_F32_EMPTY_3,
    NESTED_4_OPTION_FIXED_F32_EMPTY_3_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_f64_empty_0,
    PNested4OptionFixedF64,
    vec![],
    NESTED_4_OPTION_FIXED_F64_EMPTY_0,
    NESTED_4_OPTION_FIXED_F64_EMPTY_0_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_f64_empty_1,
    PNested4OptionFixedF64,
    vec![vec![]],
    NESTED_4_OPTION_FIXED_F64_EMPTY_1,
    NESTED_4_OPTION_FIXED_F64_EMPTY_1_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_f64_empty_2,
    PNested4OptionFixedF64,
    vec![vec![vec![]]],
    NESTED_4_OPTION_FIXED_F64_EMPTY_2,
    NESTED_4_OPTION_FIXED_F64_EMPTY_2_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_f64_empty_3,
    PNested4OptionFixedF64,
    vec![vec![vec![vec![]]]],
    NESTED_4_OPTION_FIXED_F64_EMPTY_3,
    NESTED_4_OPTION_FIXED_F64_EMPTY_3_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_bool_empty_0,
    PNested4OptionFixedBool,
    vec![],
    NESTED_4_OPTION_FIXED_BOOL_EMPTY_0,
    NESTED_4_OPTION_FIXED_BOOL_EMPTY_0_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_bool_empty_1,
    PNested4OptionFixedBool,
    vec![vec![]],
    NESTED_4_OPTION_FIXED_BOOL_EMPTY_1,
    NESTED_4_OPTION_FIXED_BOOL_EMPTY_1_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_bool_empty_2,
    PNested4OptionFixedBool,
    vec![vec![vec![]]],
    NESTED_4_OPTION_FIXED_BOOL_EMPTY_2,
    NESTED_4_OPTION_FIXED_BOOL_EMPTY_2_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_bool_empty_3,
    PNested4OptionFixedBool,
    vec![vec![vec![vec![]]]],
    NESTED_4_OPTION_FIXED_BOOL_EMPTY_3,
    NESTED_4_OPTION_FIXED_BOOL_EMPTY_3_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_bool_8_empty_0,
    PNested4OptionFixedBool8,
    vec![],
    NESTED_4_OPTION_FIXED_BOOL_8_EMPTY_0,
    NESTED_4_OPTION_FIXED_BOOL_8_EMPTY_0_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_bool_8_empty_1,
    PNested4OptionFixedBool8,
    vec![vec![]],
    NESTED_4_OPTION_FIXED_BOOL_8_EMPTY_1,
    NESTED_4_OPTION_FIXED_BOOL_8_EMPTY_1_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_bool_8_empty_2,
    PNested4OptionFixedBool8,
    vec![vec![vec![]]],
    NESTED_4_OPTION_FIXED_BOOL_8_EMPTY_2,
    NESTED_4_OPTION_FIXED_BOOL_8_EMPTY_2_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_bool_8_empty_3,
    PNested4OptionFixedBool8,
    vec![vec![vec![vec![]]]],
    NESTED_4_OPTION_FIXED_BOOL_8_EMPTY_3,
    NESTED_4_OPTION_FIXED_BOOL_8_EMPTY_3_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_bool_16_empty_0,
    PNested4OptionFixedBool16,
    vec![],
    NESTED_4_OPTION_FIXED_BOOL_16_EMPTY_0,
    NESTED_4_OPTION_FIXED_BOOL_16_EMPTY_0_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_bool_16_empty_1,
    PNested4OptionFixedBool16,
    vec![vec![]],
    NESTED_4_OPTION_FIXED_BOOL_16_EMPTY_1,
    NESTED_4_OPTION_FIXED_BOOL_16_EMPTY_1_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_bool_16_empty_2,
    PNested4OptionFixedBool16,
    vec![vec![vec![]]],
    NESTED_4_OPTION_FIXED_BOOL_16_EMPTY_2,
    NESTED_4_OPTION_FIXED_BOOL_16_EMPTY_2_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_bool_16_empty_3,
    PNested4OptionFixedBool16,
    vec![vec![vec![vec![]]]],
    NESTED_4_OPTION_FIXED_BOOL_16_EMPTY_3,
    NESTED_4_OPTION_FIXED_BOOL_16_EMPTY_3_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_timestamp,
    PNested4OptionFixedTimestamp,
    vec![
        vec![
            vec![
                vec![Some([Time(1_500_000_000), Time(-2_000_000_000)]), None],
                vec![Some([Time(-5_500_000_000), Time(6_000_000_000)])]
            ],
            vec![vec![Some([Time(7_500_000_000), Time(-8_000_000_000)])]]
        ],
        vec![vec![vec![
            None,
            Some([Time(-11_500_000_000), Time(12_000_000_000)])
        ]]]
    ],
    NESTED_4_OPTION_FIXED_TIMESTAMP,
    NESTED_4_OPTION_FIXED_TIMESTAMP_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_timestamp_empty_0,
    PNested4OptionFixedTimestamp,
    vec![],
    NESTED_4_OPTION_FIXED_TIMESTAMP_EMPTY_0,
    NESTED_4_OPTION_FIXED_TIMESTAMP_EMPTY_0_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_timestamp_empty_1,
    PNested4OptionFixedTimestamp,
    vec![vec![]],
    NESTED_4_OPTION_FIXED_TIMESTAMP_EMPTY_1,
    NESTED_4_OPTION_FIXED_TIMESTAMP_EMPTY_1_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_timestamp_empty_2,
    PNested4OptionFixedTimestamp,
    vec![vec![vec![]]],
    NESTED_4_OPTION_FIXED_TIMESTAMP_EMPTY_2,
    NESTED_4_OPTION_FIXED_TIMESTAMP_EMPTY_2_INODES
);

value_match_nested_4_option_fixed_test!(
    value_match_nested_4_option_fixed_timestamp_empty_3,
    PNested4OptionFixedTimestamp,
    vec![vec![vec![vec![]]]],
    NESTED_4_OPTION_FIXED_TIMESTAMP_EMPTY_3,
    NESTED_4_OPTION_FIXED_TIMESTAMP_EMPTY_3_INODES
);
