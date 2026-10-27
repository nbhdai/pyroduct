//! Struct-vs-value match for `src/wire/tests/primitive_5_option_nesting_u16.rs` —
//! `List<List<List<List<List<Option<T>>>>>>` against a derived struct with a native
//! `Vec<Vec<Vec<Vec<Vec<Option<T>>>>>>` field. See `tests/value_match/primitive_u16.rs`'s
//! module doc for the shared harness rationale.

use half::f16;

use pyroformat::value::Time;
use pyroformat::wire::tests::primitive_5_option_nesting_u16::*;

use crate::support::read_back::assert_roundtrip;
use pyroformat::{Layout, Reader, Writer};

macro_rules! value_match_nested_5_option_test {
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
struct PNested5OptionU8 {
    a: Vec<Vec<Vec<Vec<Vec<Option<u8>>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5OptionI8 {
    a: Vec<Vec<Vec<Vec<Vec<Option<i8>>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5OptionU16 {
    a: Vec<Vec<Vec<Vec<Vec<Option<u16>>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5OptionI16 {
    a: Vec<Vec<Vec<Vec<Vec<Option<i16>>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5OptionU32 {
    a: Vec<Vec<Vec<Vec<Vec<Option<u32>>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5OptionI32 {
    a: Vec<Vec<Vec<Vec<Vec<Option<i32>>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5OptionU64 {
    a: Vec<Vec<Vec<Vec<Vec<Option<u64>>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5OptionI64 {
    a: Vec<Vec<Vec<Vec<Vec<Option<i64>>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5OptionF16 {
    a: Vec<Vec<Vec<Vec<Vec<Option<f16>>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5OptionF32 {
    a: Vec<Vec<Vec<Vec<Vec<Option<f32>>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5OptionF64 {
    a: Vec<Vec<Vec<Vec<Vec<Option<f64>>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5OptionBool {
    a: Vec<Vec<Vec<Vec<Vec<Option<bool>>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested5OptionTimestamp {
    a: Vec<Vec<Vec<Vec<Vec<Option<Time>>>>>>,
}

value_match_nested_5_option_test!(
    value_match_nested_5_option_u8,
    PNested5OptionU8,
    vec![
        vec![
            vec![vec![vec![Some(1), None]], vec![vec![Some(3)]]],
            vec![vec![vec![Some(4)]]]
        ],
        vec![vec![vec![vec![None, Some(6)]]]]
    ],
    NESTED_5_OPTION_U8,
    NESTED_5_OPTION_U8_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_i8,
    PNested5OptionI8,
    vec![
        vec![
            vec![vec![vec![Some(-1), None]], vec![vec![Some(-3)]]],
            vec![vec![vec![Some(4)]]]
        ],
        vec![vec![vec![vec![None, Some(6)]]]]
    ],
    NESTED_5_OPTION_I8,
    NESTED_5_OPTION_I8_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_u16,
    PNested5OptionU16,
    vec![
        vec![
            vec![vec![vec![Some(1), None]], vec![vec![Some(3)]]],
            vec![vec![vec![Some(4)]]]
        ],
        vec![vec![vec![vec![None, Some(6)]]]]
    ],
    NESTED_5_OPTION_U16,
    NESTED_5_OPTION_U16_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_i16,
    PNested5OptionI16,
    vec![
        vec![
            vec![vec![vec![Some(-1), None]], vec![vec![Some(-3)]]],
            vec![vec![vec![Some(4)]]]
        ],
        vec![vec![vec![vec![None, Some(6)]]]]
    ],
    NESTED_5_OPTION_I16,
    NESTED_5_OPTION_I16_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_u32,
    PNested5OptionU32,
    vec![
        vec![
            vec![vec![vec![Some(1), None]], vec![vec![Some(3)]]],
            vec![vec![vec![Some(4)]]]
        ],
        vec![vec![vec![vec![None, Some(6)]]]]
    ],
    NESTED_5_OPTION_U32,
    NESTED_5_OPTION_U32_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_i32,
    PNested5OptionI32,
    vec![
        vec![
            vec![vec![vec![Some(-1), None]], vec![vec![Some(-3)]]],
            vec![vec![vec![Some(4)]]]
        ],
        vec![vec![vec![vec![None, Some(6)]]]]
    ],
    NESTED_5_OPTION_I32,
    NESTED_5_OPTION_I32_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_u64,
    PNested5OptionU64,
    vec![
        vec![
            vec![vec![vec![Some(1), None]], vec![vec![Some(3)]]],
            vec![vec![vec![Some(4)]]]
        ],
        vec![vec![vec![vec![None, Some(6)]]]]
    ],
    NESTED_5_OPTION_U64,
    NESTED_5_OPTION_U64_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_i64,
    PNested5OptionI64,
    vec![
        vec![
            vec![vec![vec![Some(-1), None]], vec![vec![Some(-3)]]],
            vec![vec![vec![Some(4)]]]
        ],
        vec![vec![vec![vec![None, Some(6)]]]]
    ],
    NESTED_5_OPTION_I64,
    NESTED_5_OPTION_I64_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_f16,
    PNested5OptionF16,
    vec![
        vec![
            vec![
                vec![vec![Some(f16::from_f32(1.5)), None]],
                vec![vec![Some(f16::from_f32(3.25))]]
            ],
            vec![vec![vec![Some(f16::from_f32(4.0))]]]
        ],
        vec![vec![vec![vec![None, Some(f16::from_f32(6.0))]]]]
    ],
    NESTED_5_OPTION_F16,
    NESTED_5_OPTION_F16_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_f32,
    PNested5OptionF32,
    vec![
        vec![
            vec![vec![vec![Some(1.5), None]], vec![vec![Some(3.25)]]],
            vec![vec![vec![Some(4.0)]]]
        ],
        vec![vec![vec![vec![None, Some(6.0)]]]]
    ],
    NESTED_5_OPTION_F32,
    NESTED_5_OPTION_F32_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_f64,
    PNested5OptionF64,
    vec![
        vec![
            vec![vec![vec![Some(1.5), None]], vec![vec![Some(3.25)]]],
            vec![vec![vec![Some(4.0)]]]
        ],
        vec![vec![vec![vec![None, Some(6.0)]]]]
    ],
    NESTED_5_OPTION_F64,
    NESTED_5_OPTION_F64_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_bool,
    PNested5OptionBool,
    vec![
        vec![
            vec![vec![vec![Some(true), None]], vec![vec![Some(true)]]],
            vec![vec![vec![Some(true)]]]
        ],
        vec![vec![vec![vec![None, Some(false)]]]]
    ],
    NESTED_5_OPTION_BOOL,
    NESTED_5_OPTION_BOOL_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_timestamp,
    PNested5OptionTimestamp,
    vec![
        vec![
            vec![
                vec![vec![Some(Time(1_000_001_000)), None]],
                vec![vec![Some(Time(3_500_003_000))]]
            ],
            vec![vec![vec![Some(Time(4_000_004_000))]]]
        ],
        vec![vec![vec![vec![
            None,
            Some(Time(1_700_000_000_000_006_000))
        ]]]]
    ],
    NESTED_5_OPTION_TIMESTAMP,
    NESTED_5_OPTION_TIMESTAMP_INODES
);

// ---------------------------------------------------------------------------
// Empty lists at every depth (`_EMPTY_d`: the list at depth `d` is empty)
// ---------------------------------------------------------------------------
value_match_nested_5_option_test!(
    value_match_nested_5_option_u8_empty_0,
    PNested5OptionU8,
    vec![],
    NESTED_5_OPTION_U8_EMPTY_0,
    NESTED_5_OPTION_U8_EMPTY_0_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_u8_empty_1,
    PNested5OptionU8,
    vec![vec![]],
    NESTED_5_OPTION_U8_EMPTY_1,
    NESTED_5_OPTION_U8_EMPTY_1_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_u8_empty_2,
    PNested5OptionU8,
    vec![vec![vec![]]],
    NESTED_5_OPTION_U8_EMPTY_2,
    NESTED_5_OPTION_U8_EMPTY_2_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_u8_empty_3,
    PNested5OptionU8,
    vec![vec![vec![vec![]]]],
    NESTED_5_OPTION_U8_EMPTY_3,
    NESTED_5_OPTION_U8_EMPTY_3_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_u8_empty_4,
    PNested5OptionU8,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_OPTION_U8_EMPTY_4,
    NESTED_5_OPTION_U8_EMPTY_4_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_i8_empty_0,
    PNested5OptionI8,
    vec![],
    NESTED_5_OPTION_I8_EMPTY_0,
    NESTED_5_OPTION_I8_EMPTY_0_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_i8_empty_1,
    PNested5OptionI8,
    vec![vec![]],
    NESTED_5_OPTION_I8_EMPTY_1,
    NESTED_5_OPTION_I8_EMPTY_1_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_i8_empty_2,
    PNested5OptionI8,
    vec![vec![vec![]]],
    NESTED_5_OPTION_I8_EMPTY_2,
    NESTED_5_OPTION_I8_EMPTY_2_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_i8_empty_3,
    PNested5OptionI8,
    vec![vec![vec![vec![]]]],
    NESTED_5_OPTION_I8_EMPTY_3,
    NESTED_5_OPTION_I8_EMPTY_3_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_i8_empty_4,
    PNested5OptionI8,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_OPTION_I8_EMPTY_4,
    NESTED_5_OPTION_I8_EMPTY_4_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_u16_empty_0,
    PNested5OptionU16,
    vec![],
    NESTED_5_OPTION_U16_EMPTY_0,
    NESTED_5_OPTION_U16_EMPTY_0_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_u16_empty_1,
    PNested5OptionU16,
    vec![vec![]],
    NESTED_5_OPTION_U16_EMPTY_1,
    NESTED_5_OPTION_U16_EMPTY_1_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_u16_empty_2,
    PNested5OptionU16,
    vec![vec![vec![]]],
    NESTED_5_OPTION_U16_EMPTY_2,
    NESTED_5_OPTION_U16_EMPTY_2_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_u16_empty_3,
    PNested5OptionU16,
    vec![vec![vec![vec![]]]],
    NESTED_5_OPTION_U16_EMPTY_3,
    NESTED_5_OPTION_U16_EMPTY_3_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_u16_empty_4,
    PNested5OptionU16,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_OPTION_U16_EMPTY_4,
    NESTED_5_OPTION_U16_EMPTY_4_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_i16_empty_0,
    PNested5OptionI16,
    vec![],
    NESTED_5_OPTION_I16_EMPTY_0,
    NESTED_5_OPTION_I16_EMPTY_0_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_i16_empty_1,
    PNested5OptionI16,
    vec![vec![]],
    NESTED_5_OPTION_I16_EMPTY_1,
    NESTED_5_OPTION_I16_EMPTY_1_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_i16_empty_2,
    PNested5OptionI16,
    vec![vec![vec![]]],
    NESTED_5_OPTION_I16_EMPTY_2,
    NESTED_5_OPTION_I16_EMPTY_2_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_i16_empty_3,
    PNested5OptionI16,
    vec![vec![vec![vec![]]]],
    NESTED_5_OPTION_I16_EMPTY_3,
    NESTED_5_OPTION_I16_EMPTY_3_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_i16_empty_4,
    PNested5OptionI16,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_OPTION_I16_EMPTY_4,
    NESTED_5_OPTION_I16_EMPTY_4_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_u32_empty_0,
    PNested5OptionU32,
    vec![],
    NESTED_5_OPTION_U32_EMPTY_0,
    NESTED_5_OPTION_U32_EMPTY_0_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_u32_empty_1,
    PNested5OptionU32,
    vec![vec![]],
    NESTED_5_OPTION_U32_EMPTY_1,
    NESTED_5_OPTION_U32_EMPTY_1_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_u32_empty_2,
    PNested5OptionU32,
    vec![vec![vec![]]],
    NESTED_5_OPTION_U32_EMPTY_2,
    NESTED_5_OPTION_U32_EMPTY_2_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_u32_empty_3,
    PNested5OptionU32,
    vec![vec![vec![vec![]]]],
    NESTED_5_OPTION_U32_EMPTY_3,
    NESTED_5_OPTION_U32_EMPTY_3_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_u32_empty_4,
    PNested5OptionU32,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_OPTION_U32_EMPTY_4,
    NESTED_5_OPTION_U32_EMPTY_4_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_i32_empty_0,
    PNested5OptionI32,
    vec![],
    NESTED_5_OPTION_I32_EMPTY_0,
    NESTED_5_OPTION_I32_EMPTY_0_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_i32_empty_1,
    PNested5OptionI32,
    vec![vec![]],
    NESTED_5_OPTION_I32_EMPTY_1,
    NESTED_5_OPTION_I32_EMPTY_1_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_i32_empty_2,
    PNested5OptionI32,
    vec![vec![vec![]]],
    NESTED_5_OPTION_I32_EMPTY_2,
    NESTED_5_OPTION_I32_EMPTY_2_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_i32_empty_3,
    PNested5OptionI32,
    vec![vec![vec![vec![]]]],
    NESTED_5_OPTION_I32_EMPTY_3,
    NESTED_5_OPTION_I32_EMPTY_3_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_i32_empty_4,
    PNested5OptionI32,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_OPTION_I32_EMPTY_4,
    NESTED_5_OPTION_I32_EMPTY_4_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_u64_empty_0,
    PNested5OptionU64,
    vec![],
    NESTED_5_OPTION_U64_EMPTY_0,
    NESTED_5_OPTION_U64_EMPTY_0_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_u64_empty_1,
    PNested5OptionU64,
    vec![vec![]],
    NESTED_5_OPTION_U64_EMPTY_1,
    NESTED_5_OPTION_U64_EMPTY_1_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_u64_empty_2,
    PNested5OptionU64,
    vec![vec![vec![]]],
    NESTED_5_OPTION_U64_EMPTY_2,
    NESTED_5_OPTION_U64_EMPTY_2_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_u64_empty_3,
    PNested5OptionU64,
    vec![vec![vec![vec![]]]],
    NESTED_5_OPTION_U64_EMPTY_3,
    NESTED_5_OPTION_U64_EMPTY_3_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_u64_empty_4,
    PNested5OptionU64,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_OPTION_U64_EMPTY_4,
    NESTED_5_OPTION_U64_EMPTY_4_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_i64_empty_0,
    PNested5OptionI64,
    vec![],
    NESTED_5_OPTION_I64_EMPTY_0,
    NESTED_5_OPTION_I64_EMPTY_0_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_i64_empty_1,
    PNested5OptionI64,
    vec![vec![]],
    NESTED_5_OPTION_I64_EMPTY_1,
    NESTED_5_OPTION_I64_EMPTY_1_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_i64_empty_2,
    PNested5OptionI64,
    vec![vec![vec![]]],
    NESTED_5_OPTION_I64_EMPTY_2,
    NESTED_5_OPTION_I64_EMPTY_2_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_i64_empty_3,
    PNested5OptionI64,
    vec![vec![vec![vec![]]]],
    NESTED_5_OPTION_I64_EMPTY_3,
    NESTED_5_OPTION_I64_EMPTY_3_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_i64_empty_4,
    PNested5OptionI64,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_OPTION_I64_EMPTY_4,
    NESTED_5_OPTION_I64_EMPTY_4_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_f16_empty_0,
    PNested5OptionF16,
    vec![],
    NESTED_5_OPTION_F16_EMPTY_0,
    NESTED_5_OPTION_F16_EMPTY_0_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_f16_empty_1,
    PNested5OptionF16,
    vec![vec![]],
    NESTED_5_OPTION_F16_EMPTY_1,
    NESTED_5_OPTION_F16_EMPTY_1_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_f16_empty_2,
    PNested5OptionF16,
    vec![vec![vec![]]],
    NESTED_5_OPTION_F16_EMPTY_2,
    NESTED_5_OPTION_F16_EMPTY_2_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_f16_empty_3,
    PNested5OptionF16,
    vec![vec![vec![vec![]]]],
    NESTED_5_OPTION_F16_EMPTY_3,
    NESTED_5_OPTION_F16_EMPTY_3_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_f16_empty_4,
    PNested5OptionF16,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_OPTION_F16_EMPTY_4,
    NESTED_5_OPTION_F16_EMPTY_4_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_f32_empty_0,
    PNested5OptionF32,
    vec![],
    NESTED_5_OPTION_F32_EMPTY_0,
    NESTED_5_OPTION_F32_EMPTY_0_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_f32_empty_1,
    PNested5OptionF32,
    vec![vec![]],
    NESTED_5_OPTION_F32_EMPTY_1,
    NESTED_5_OPTION_F32_EMPTY_1_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_f32_empty_2,
    PNested5OptionF32,
    vec![vec![vec![]]],
    NESTED_5_OPTION_F32_EMPTY_2,
    NESTED_5_OPTION_F32_EMPTY_2_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_f32_empty_3,
    PNested5OptionF32,
    vec![vec![vec![vec![]]]],
    NESTED_5_OPTION_F32_EMPTY_3,
    NESTED_5_OPTION_F32_EMPTY_3_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_f32_empty_4,
    PNested5OptionF32,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_OPTION_F32_EMPTY_4,
    NESTED_5_OPTION_F32_EMPTY_4_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_f64_empty_0,
    PNested5OptionF64,
    vec![],
    NESTED_5_OPTION_F64_EMPTY_0,
    NESTED_5_OPTION_F64_EMPTY_0_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_f64_empty_1,
    PNested5OptionF64,
    vec![vec![]],
    NESTED_5_OPTION_F64_EMPTY_1,
    NESTED_5_OPTION_F64_EMPTY_1_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_f64_empty_2,
    PNested5OptionF64,
    vec![vec![vec![]]],
    NESTED_5_OPTION_F64_EMPTY_2,
    NESTED_5_OPTION_F64_EMPTY_2_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_f64_empty_3,
    PNested5OptionF64,
    vec![vec![vec![vec![]]]],
    NESTED_5_OPTION_F64_EMPTY_3,
    NESTED_5_OPTION_F64_EMPTY_3_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_f64_empty_4,
    PNested5OptionF64,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_OPTION_F64_EMPTY_4,
    NESTED_5_OPTION_F64_EMPTY_4_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_bool_empty_0,
    PNested5OptionBool,
    vec![],
    NESTED_5_OPTION_BOOL_EMPTY_0,
    NESTED_5_OPTION_BOOL_EMPTY_0_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_bool_empty_1,
    PNested5OptionBool,
    vec![vec![]],
    NESTED_5_OPTION_BOOL_EMPTY_1,
    NESTED_5_OPTION_BOOL_EMPTY_1_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_bool_empty_2,
    PNested5OptionBool,
    vec![vec![vec![]]],
    NESTED_5_OPTION_BOOL_EMPTY_2,
    NESTED_5_OPTION_BOOL_EMPTY_2_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_bool_empty_3,
    PNested5OptionBool,
    vec![vec![vec![vec![]]]],
    NESTED_5_OPTION_BOOL_EMPTY_3,
    NESTED_5_OPTION_BOOL_EMPTY_3_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_bool_empty_4,
    PNested5OptionBool,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_OPTION_BOOL_EMPTY_4,
    NESTED_5_OPTION_BOOL_EMPTY_4_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_timestamp_empty_0,
    PNested5OptionTimestamp,
    vec![],
    NESTED_5_OPTION_TIMESTAMP_EMPTY_0,
    NESTED_5_OPTION_TIMESTAMP_EMPTY_0_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_timestamp_empty_1,
    PNested5OptionTimestamp,
    vec![vec![]],
    NESTED_5_OPTION_TIMESTAMP_EMPTY_1,
    NESTED_5_OPTION_TIMESTAMP_EMPTY_1_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_timestamp_empty_2,
    PNested5OptionTimestamp,
    vec![vec![vec![]]],
    NESTED_5_OPTION_TIMESTAMP_EMPTY_2,
    NESTED_5_OPTION_TIMESTAMP_EMPTY_2_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_timestamp_empty_3,
    PNested5OptionTimestamp,
    vec![vec![vec![vec![]]]],
    NESTED_5_OPTION_TIMESTAMP_EMPTY_3,
    NESTED_5_OPTION_TIMESTAMP_EMPTY_3_INODES
);

value_match_nested_5_option_test!(
    value_match_nested_5_option_timestamp_empty_4,
    PNested5OptionTimestamp,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_OPTION_TIMESTAMP_EMPTY_4,
    NESTED_5_OPTION_TIMESTAMP_EMPTY_4_INODES
);
