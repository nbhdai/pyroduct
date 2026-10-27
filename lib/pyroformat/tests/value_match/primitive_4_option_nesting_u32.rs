//! Struct-vs-value match for `src/wire/tests/primitive_4_option_nesting_u32.rs` —
//! `List<List<List<List<Option<T>>>>>` against a derived struct with a native
//! `Vec<Vec<Vec<Vec<Option<T>>>>>` field. See `tests/value_match/primitive_u32.rs`'s
//! module doc for the shared harness rationale.

use half::f16;

use pyroformat::value::Time;
use pyroformat::wire::tests::primitive_4_option_nesting_u32::*;

use crate::support::read_back::assert_roundtrip;
use pyroformat::{Layout, Reader, Writer};

macro_rules! value_match_nested_4_option_test {
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
struct PNested4OptionU8 {
    a: Vec<Vec<Vec<Vec<Option<u8>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionI8 {
    a: Vec<Vec<Vec<Vec<Option<i8>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionU16 {
    a: Vec<Vec<Vec<Vec<Option<u16>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionI16 {
    a: Vec<Vec<Vec<Vec<Option<i16>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionU32 {
    a: Vec<Vec<Vec<Vec<Option<u32>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionI32 {
    a: Vec<Vec<Vec<Vec<Option<i32>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionU64 {
    a: Vec<Vec<Vec<Vec<Option<u64>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionI64 {
    a: Vec<Vec<Vec<Vec<Option<i64>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionF16 {
    a: Vec<Vec<Vec<Vec<Option<f16>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionF32 {
    a: Vec<Vec<Vec<Vec<Option<f32>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionF64 {
    a: Vec<Vec<Vec<Vec<Option<f64>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionBool {
    a: Vec<Vec<Vec<Vec<Option<bool>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionTimestamp {
    a: Vec<Vec<Vec<Vec<Option<Time>>>>>,
}

value_match_nested_4_option_test!(
    value_match_nested_4_option_u8,
    PNested4OptionU8,
    vec![
        vec![
            vec![vec![Some(1), None], vec![Some(3)]],
            vec![vec![Some(4)]]
        ],
        vec![vec![vec![None, Some(6)]]]
    ],
    NESTED_4_OPTION_U8,
    NESTED_4_OPTION_U8_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_i8,
    PNested4OptionI8,
    vec![
        vec![
            vec![vec![Some(-1), None], vec![Some(-3)]],
            vec![vec![Some(4)]]
        ],
        vec![vec![vec![None, Some(6)]]]
    ],
    NESTED_4_OPTION_I8,
    NESTED_4_OPTION_I8_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_u16,
    PNested4OptionU16,
    vec![
        vec![
            vec![vec![Some(1), None], vec![Some(3)]],
            vec![vec![Some(4)]]
        ],
        vec![vec![vec![None, Some(6)]]]
    ],
    NESTED_4_OPTION_U16,
    NESTED_4_OPTION_U16_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_i16,
    PNested4OptionI16,
    vec![
        vec![
            vec![vec![Some(-1), None], vec![Some(-3)]],
            vec![vec![Some(4)]]
        ],
        vec![vec![vec![None, Some(6)]]]
    ],
    NESTED_4_OPTION_I16,
    NESTED_4_OPTION_I16_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_u32,
    PNested4OptionU32,
    vec![
        vec![
            vec![vec![Some(1), None], vec![Some(3)]],
            vec![vec![Some(4)]]
        ],
        vec![vec![vec![None, Some(6)]]]
    ],
    NESTED_4_OPTION_U32,
    NESTED_4_OPTION_U32_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_i32,
    PNested4OptionI32,
    vec![
        vec![
            vec![vec![Some(-1), None], vec![Some(-3)]],
            vec![vec![Some(4)]]
        ],
        vec![vec![vec![None, Some(6)]]]
    ],
    NESTED_4_OPTION_I32,
    NESTED_4_OPTION_I32_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_u64,
    PNested4OptionU64,
    vec![
        vec![
            vec![vec![Some(1), None], vec![Some(3)]],
            vec![vec![Some(4)]]
        ],
        vec![vec![vec![None, Some(6)]]]
    ],
    NESTED_4_OPTION_U64,
    NESTED_4_OPTION_U64_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_i64,
    PNested4OptionI64,
    vec![
        vec![
            vec![vec![Some(-1), None], vec![Some(-3)]],
            vec![vec![Some(4)]]
        ],
        vec![vec![vec![None, Some(6)]]]
    ],
    NESTED_4_OPTION_I64,
    NESTED_4_OPTION_I64_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_f16,
    PNested4OptionF16,
    vec![
        vec![
            vec![
                vec![Some(f16::from_f32(1.5)), None],
                vec![Some(f16::from_f32(3.25))]
            ],
            vec![vec![Some(f16::from_f32(4.0))]]
        ],
        vec![vec![vec![None, Some(f16::from_f32(6.0))]]]
    ],
    NESTED_4_OPTION_F16,
    NESTED_4_OPTION_F16_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_f32,
    PNested4OptionF32,
    vec![
        vec![
            vec![vec![Some(1.5), None], vec![Some(3.25)]],
            vec![vec![Some(4.0)]]
        ],
        vec![vec![vec![None, Some(6.0)]]]
    ],
    NESTED_4_OPTION_F32,
    NESTED_4_OPTION_F32_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_f64,
    PNested4OptionF64,
    vec![
        vec![
            vec![vec![Some(1.5), None], vec![Some(3.25)]],
            vec![vec![Some(4.0)]]
        ],
        vec![vec![vec![None, Some(6.0)]]]
    ],
    NESTED_4_OPTION_F64,
    NESTED_4_OPTION_F64_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_bool,
    PNested4OptionBool,
    vec![
        vec![
            vec![vec![Some(true), None], vec![Some(true)]],
            vec![vec![Some(true)]]
        ],
        vec![vec![vec![None, Some(false)]]]
    ],
    NESTED_4_OPTION_BOOL,
    NESTED_4_OPTION_BOOL_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_timestamp,
    PNested4OptionTimestamp,
    vec![
        vec![
            vec![
                vec![Some(Time(1_000_001_000)), None],
                vec![Some(Time(3_500_003_000))]
            ],
            vec![vec![Some(Time(4_000_004_000))]]
        ],
        vec![vec![vec![None, Some(Time(1_700_000_000_000_006_000))]]]
    ],
    NESTED_4_OPTION_TIMESTAMP,
    NESTED_4_OPTION_TIMESTAMP_INODES
);

// ---------------------------------------------------------------------------
// Empty lists at every depth (`_EMPTY_d`: the list at depth `d` is empty)
// ---------------------------------------------------------------------------
value_match_nested_4_option_test!(
    value_match_nested_4_option_u8_empty_0,
    PNested4OptionU8,
    vec![],
    NESTED_4_OPTION_U8_EMPTY_0,
    NESTED_4_OPTION_U8_EMPTY_0_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_u8_empty_1,
    PNested4OptionU8,
    vec![vec![]],
    NESTED_4_OPTION_U8_EMPTY_1,
    NESTED_4_OPTION_U8_EMPTY_1_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_u8_empty_2,
    PNested4OptionU8,
    vec![vec![vec![]]],
    NESTED_4_OPTION_U8_EMPTY_2,
    NESTED_4_OPTION_U8_EMPTY_2_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_u8_empty_3,
    PNested4OptionU8,
    vec![vec![vec![vec![]]]],
    NESTED_4_OPTION_U8_EMPTY_3,
    NESTED_4_OPTION_U8_EMPTY_3_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_i8_empty_0,
    PNested4OptionI8,
    vec![],
    NESTED_4_OPTION_I8_EMPTY_0,
    NESTED_4_OPTION_I8_EMPTY_0_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_i8_empty_1,
    PNested4OptionI8,
    vec![vec![]],
    NESTED_4_OPTION_I8_EMPTY_1,
    NESTED_4_OPTION_I8_EMPTY_1_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_i8_empty_2,
    PNested4OptionI8,
    vec![vec![vec![]]],
    NESTED_4_OPTION_I8_EMPTY_2,
    NESTED_4_OPTION_I8_EMPTY_2_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_i8_empty_3,
    PNested4OptionI8,
    vec![vec![vec![vec![]]]],
    NESTED_4_OPTION_I8_EMPTY_3,
    NESTED_4_OPTION_I8_EMPTY_3_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_u16_empty_0,
    PNested4OptionU16,
    vec![],
    NESTED_4_OPTION_U16_EMPTY_0,
    NESTED_4_OPTION_U16_EMPTY_0_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_u16_empty_1,
    PNested4OptionU16,
    vec![vec![]],
    NESTED_4_OPTION_U16_EMPTY_1,
    NESTED_4_OPTION_U16_EMPTY_1_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_u16_empty_2,
    PNested4OptionU16,
    vec![vec![vec![]]],
    NESTED_4_OPTION_U16_EMPTY_2,
    NESTED_4_OPTION_U16_EMPTY_2_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_u16_empty_3,
    PNested4OptionU16,
    vec![vec![vec![vec![]]]],
    NESTED_4_OPTION_U16_EMPTY_3,
    NESTED_4_OPTION_U16_EMPTY_3_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_i16_empty_0,
    PNested4OptionI16,
    vec![],
    NESTED_4_OPTION_I16_EMPTY_0,
    NESTED_4_OPTION_I16_EMPTY_0_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_i16_empty_1,
    PNested4OptionI16,
    vec![vec![]],
    NESTED_4_OPTION_I16_EMPTY_1,
    NESTED_4_OPTION_I16_EMPTY_1_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_i16_empty_2,
    PNested4OptionI16,
    vec![vec![vec![]]],
    NESTED_4_OPTION_I16_EMPTY_2,
    NESTED_4_OPTION_I16_EMPTY_2_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_i16_empty_3,
    PNested4OptionI16,
    vec![vec![vec![vec![]]]],
    NESTED_4_OPTION_I16_EMPTY_3,
    NESTED_4_OPTION_I16_EMPTY_3_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_u32_empty_0,
    PNested4OptionU32,
    vec![],
    NESTED_4_OPTION_U32_EMPTY_0,
    NESTED_4_OPTION_U32_EMPTY_0_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_u32_empty_1,
    PNested4OptionU32,
    vec![vec![]],
    NESTED_4_OPTION_U32_EMPTY_1,
    NESTED_4_OPTION_U32_EMPTY_1_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_u32_empty_2,
    PNested4OptionU32,
    vec![vec![vec![]]],
    NESTED_4_OPTION_U32_EMPTY_2,
    NESTED_4_OPTION_U32_EMPTY_2_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_u32_empty_3,
    PNested4OptionU32,
    vec![vec![vec![vec![]]]],
    NESTED_4_OPTION_U32_EMPTY_3,
    NESTED_4_OPTION_U32_EMPTY_3_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_i32_empty_0,
    PNested4OptionI32,
    vec![],
    NESTED_4_OPTION_I32_EMPTY_0,
    NESTED_4_OPTION_I32_EMPTY_0_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_i32_empty_1,
    PNested4OptionI32,
    vec![vec![]],
    NESTED_4_OPTION_I32_EMPTY_1,
    NESTED_4_OPTION_I32_EMPTY_1_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_i32_empty_2,
    PNested4OptionI32,
    vec![vec![vec![]]],
    NESTED_4_OPTION_I32_EMPTY_2,
    NESTED_4_OPTION_I32_EMPTY_2_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_i32_empty_3,
    PNested4OptionI32,
    vec![vec![vec![vec![]]]],
    NESTED_4_OPTION_I32_EMPTY_3,
    NESTED_4_OPTION_I32_EMPTY_3_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_u64_empty_0,
    PNested4OptionU64,
    vec![],
    NESTED_4_OPTION_U64_EMPTY_0,
    NESTED_4_OPTION_U64_EMPTY_0_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_u64_empty_1,
    PNested4OptionU64,
    vec![vec![]],
    NESTED_4_OPTION_U64_EMPTY_1,
    NESTED_4_OPTION_U64_EMPTY_1_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_u64_empty_2,
    PNested4OptionU64,
    vec![vec![vec![]]],
    NESTED_4_OPTION_U64_EMPTY_2,
    NESTED_4_OPTION_U64_EMPTY_2_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_u64_empty_3,
    PNested4OptionU64,
    vec![vec![vec![vec![]]]],
    NESTED_4_OPTION_U64_EMPTY_3,
    NESTED_4_OPTION_U64_EMPTY_3_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_i64_empty_0,
    PNested4OptionI64,
    vec![],
    NESTED_4_OPTION_I64_EMPTY_0,
    NESTED_4_OPTION_I64_EMPTY_0_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_i64_empty_1,
    PNested4OptionI64,
    vec![vec![]],
    NESTED_4_OPTION_I64_EMPTY_1,
    NESTED_4_OPTION_I64_EMPTY_1_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_i64_empty_2,
    PNested4OptionI64,
    vec![vec![vec![]]],
    NESTED_4_OPTION_I64_EMPTY_2,
    NESTED_4_OPTION_I64_EMPTY_2_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_i64_empty_3,
    PNested4OptionI64,
    vec![vec![vec![vec![]]]],
    NESTED_4_OPTION_I64_EMPTY_3,
    NESTED_4_OPTION_I64_EMPTY_3_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_f16_empty_0,
    PNested4OptionF16,
    vec![],
    NESTED_4_OPTION_F16_EMPTY_0,
    NESTED_4_OPTION_F16_EMPTY_0_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_f16_empty_1,
    PNested4OptionF16,
    vec![vec![]],
    NESTED_4_OPTION_F16_EMPTY_1,
    NESTED_4_OPTION_F16_EMPTY_1_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_f16_empty_2,
    PNested4OptionF16,
    vec![vec![vec![]]],
    NESTED_4_OPTION_F16_EMPTY_2,
    NESTED_4_OPTION_F16_EMPTY_2_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_f16_empty_3,
    PNested4OptionF16,
    vec![vec![vec![vec![]]]],
    NESTED_4_OPTION_F16_EMPTY_3,
    NESTED_4_OPTION_F16_EMPTY_3_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_f32_empty_0,
    PNested4OptionF32,
    vec![],
    NESTED_4_OPTION_F32_EMPTY_0,
    NESTED_4_OPTION_F32_EMPTY_0_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_f32_empty_1,
    PNested4OptionF32,
    vec![vec![]],
    NESTED_4_OPTION_F32_EMPTY_1,
    NESTED_4_OPTION_F32_EMPTY_1_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_f32_empty_2,
    PNested4OptionF32,
    vec![vec![vec![]]],
    NESTED_4_OPTION_F32_EMPTY_2,
    NESTED_4_OPTION_F32_EMPTY_2_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_f32_empty_3,
    PNested4OptionF32,
    vec![vec![vec![vec![]]]],
    NESTED_4_OPTION_F32_EMPTY_3,
    NESTED_4_OPTION_F32_EMPTY_3_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_f64_empty_0,
    PNested4OptionF64,
    vec![],
    NESTED_4_OPTION_F64_EMPTY_0,
    NESTED_4_OPTION_F64_EMPTY_0_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_f64_empty_1,
    PNested4OptionF64,
    vec![vec![]],
    NESTED_4_OPTION_F64_EMPTY_1,
    NESTED_4_OPTION_F64_EMPTY_1_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_f64_empty_2,
    PNested4OptionF64,
    vec![vec![vec![]]],
    NESTED_4_OPTION_F64_EMPTY_2,
    NESTED_4_OPTION_F64_EMPTY_2_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_f64_empty_3,
    PNested4OptionF64,
    vec![vec![vec![vec![]]]],
    NESTED_4_OPTION_F64_EMPTY_3,
    NESTED_4_OPTION_F64_EMPTY_3_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_bool_empty_0,
    PNested4OptionBool,
    vec![],
    NESTED_4_OPTION_BOOL_EMPTY_0,
    NESTED_4_OPTION_BOOL_EMPTY_0_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_bool_empty_1,
    PNested4OptionBool,
    vec![vec![]],
    NESTED_4_OPTION_BOOL_EMPTY_1,
    NESTED_4_OPTION_BOOL_EMPTY_1_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_bool_empty_2,
    PNested4OptionBool,
    vec![vec![vec![]]],
    NESTED_4_OPTION_BOOL_EMPTY_2,
    NESTED_4_OPTION_BOOL_EMPTY_2_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_bool_empty_3,
    PNested4OptionBool,
    vec![vec![vec![vec![]]]],
    NESTED_4_OPTION_BOOL_EMPTY_3,
    NESTED_4_OPTION_BOOL_EMPTY_3_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_timestamp_empty_0,
    PNested4OptionTimestamp,
    vec![],
    NESTED_4_OPTION_TIMESTAMP_EMPTY_0,
    NESTED_4_OPTION_TIMESTAMP_EMPTY_0_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_timestamp_empty_1,
    PNested4OptionTimestamp,
    vec![vec![]],
    NESTED_4_OPTION_TIMESTAMP_EMPTY_1,
    NESTED_4_OPTION_TIMESTAMP_EMPTY_1_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_timestamp_empty_2,
    PNested4OptionTimestamp,
    vec![vec![vec![]]],
    NESTED_4_OPTION_TIMESTAMP_EMPTY_2,
    NESTED_4_OPTION_TIMESTAMP_EMPTY_2_INODES
);

value_match_nested_4_option_test!(
    value_match_nested_4_option_timestamp_empty_3,
    PNested4OptionTimestamp,
    vec![vec![vec![vec![]]]],
    NESTED_4_OPTION_TIMESTAMP_EMPTY_3,
    NESTED_4_OPTION_TIMESTAMP_EMPTY_3_INODES
);
