//! Struct-vs-value match for `src/wire/tests/primitive_5_nesting_u32.rs` —
//! `List<List<List<List<List<T>>>>>` against a derived struct with a native
//! `Vec<Vec<Vec<Vec<Vec<T>>>>>` field. See `tests/value_match/primitive_u32.rs`'s
//! module doc for the shared harness rationale.

use half::f16;

use pyroformat::value::Time;
use pyroformat::wire::tests::primitive_5_nesting_u32::*;

use crate::support::bits::bits;
use crate::support::read_back::assert_roundtrip;
use pyroformat::{BitVec, Layout, Reader, Writer};

macro_rules! value_match_nested_5_test {
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
struct PNested5U8 {
    #[pyro(numeric)]
    a: Vec<Vec<Vec<Vec<Vec<u8>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested5I8 {
    a: Vec<Vec<Vec<Vec<Vec<i8>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested5U16 {
    a: Vec<Vec<Vec<Vec<Vec<u16>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested5I16 {
    a: Vec<Vec<Vec<Vec<Vec<i16>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested5U32 {
    a: Vec<Vec<Vec<Vec<Vec<u32>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested5I32 {
    a: Vec<Vec<Vec<Vec<Vec<i32>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested5U64 {
    a: Vec<Vec<Vec<Vec<Vec<u64>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested5I64 {
    a: Vec<Vec<Vec<Vec<Vec<i64>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested5F16 {
    a: Vec<Vec<Vec<Vec<Vec<f16>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested5F32 {
    a: Vec<Vec<Vec<Vec<Vec<f32>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested5F64 {
    a: Vec<Vec<Vec<Vec<Vec<f64>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested5Bool {
    #[pyro(allow(vec_bool))]
    a: Vec<Vec<Vec<Vec<Vec<bool>>>>>,
}

// `PNested5Bool` with the innermost `Vec<bool>` a packed `BitVec<u8, Lsb0>`:
// the same fixture either way.
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested5BitVec {
    a: Vec<Vec<Vec<Vec<BitVec>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested5Timestamp {
    a: Vec<Vec<Vec<Vec<Vec<Time>>>>>,
}

value_match_nested_5_test!(
    value_match_nested_5_u8,
    PNested5U8,
    vec![
        vec![vec![vec![vec![1, 2]], vec![vec![3]]], vec![vec![vec![4]]]],
        vec![vec![vec![vec![5, 6]]]]
    ],
    NESTED_5_U8,
    NESTED_5_U8_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_i8,
    PNested5I8,
    vec![
        vec![vec![vec![vec![-1, 2]], vec![vec![-3]]], vec![vec![vec![4]]]],
        vec![vec![vec![vec![-5, 6]]]]
    ],
    NESTED_5_I8,
    NESTED_5_I8_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_u16,
    PNested5U16,
    vec![
        vec![vec![vec![vec![1, 2]], vec![vec![3]]], vec![vec![vec![4]]]],
        vec![vec![vec![vec![5, 6]]]]
    ],
    NESTED_5_U16,
    NESTED_5_U16_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_i16,
    PNested5I16,
    vec![
        vec![vec![vec![vec![-1, 2]], vec![vec![-3]]], vec![vec![vec![4]]]],
        vec![vec![vec![vec![-5, 6]]]]
    ],
    NESTED_5_I16,
    NESTED_5_I16_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_u32,
    PNested5U32,
    vec![
        vec![vec![vec![vec![1, 2]], vec![vec![3]]], vec![vec![vec![4]]]],
        vec![vec![vec![vec![5, 6]]]]
    ],
    NESTED_5_U32,
    NESTED_5_U32_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_i32,
    PNested5I32,
    vec![
        vec![vec![vec![vec![-1, 2]], vec![vec![-3]]], vec![vec![vec![4]]]],
        vec![vec![vec![vec![-5, 6]]]]
    ],
    NESTED_5_I32,
    NESTED_5_I32_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_u64,
    PNested5U64,
    vec![
        vec![vec![vec![vec![1, 2]], vec![vec![3]]], vec![vec![vec![4]]]],
        vec![vec![vec![vec![5, 6]]]]
    ],
    NESTED_5_U64,
    NESTED_5_U64_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_i64,
    PNested5I64,
    vec![
        vec![vec![vec![vec![-1, 2]], vec![vec![-3]]], vec![vec![vec![4]]]],
        vec![vec![vec![vec![-5, 6]]]]
    ],
    NESTED_5_I64,
    NESTED_5_I64_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_f16,
    PNested5F16,
    vec![
        vec![
            vec![
                vec![vec![f16::from_f32(1.5), f16::from_f32(-2.0)]],
                vec![vec![f16::from_f32(3.25)]]
            ],
            vec![vec![vec![f16::from_f32(4.0)]]]
        ],
        vec![vec![vec![vec![f16::from_f32(-5.5), f16::from_f32(6.0)]]]]
    ],
    NESTED_5_F16,
    NESTED_5_F16_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_f32,
    PNested5F32,
    vec![
        vec![
            vec![vec![vec![1.5, -2.0]], vec![vec![3.25]]],
            vec![vec![vec![4.0]]]
        ],
        vec![vec![vec![vec![-5.5, 6.0]]]]
    ],
    NESTED_5_F32,
    NESTED_5_F32_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_f64,
    PNested5F64,
    vec![
        vec![
            vec![vec![vec![1.5, -2.0]], vec![vec![3.25]]],
            vec![vec![vec![4.0]]]
        ],
        vec![vec![vec![vec![-5.5, 6.0]]]]
    ],
    NESTED_5_F64,
    NESTED_5_F64_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_bool,
    PNested5Bool,
    vec![
        vec![
            vec![vec![vec![true, false]], vec![vec![true]]],
            vec![vec![vec![true]]]
        ],
        vec![vec![vec![vec![true, false]]]]
    ],
    NESTED_5_BOOL,
    NESTED_5_BOOL_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_bool_bitvec,
    PNested5BitVec,
    vec![
        vec![
            vec![vec![bits(&[true, false])], vec![bits(&[true])]],
            vec![vec![bits(&[true])]]
        ],
        vec![vec![vec![bits(&[true, false])]]]
    ],
    NESTED_5_BOOL,
    NESTED_5_BOOL_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_timestamp,
    PNested5Timestamp,
    vec![
        vec![
            vec![
                vec![vec![Time(1_000_001_000), Time(-2_000_002_000)]],
                vec![vec![Time(3_500_003_000)]]
            ],
            vec![vec![vec![Time(4_000_004_000)]]]
        ],
        vec![vec![vec![vec![
            Time(-5_000_005_000),
            Time(1_700_000_000_000_006_000)
        ]]]]
    ],
    NESTED_5_TIMESTAMP,
    NESTED_5_TIMESTAMP_INODES
);

// ---------------------------------------------------------------------------
// Empty lists at every depth (`_EMPTY_d`: the list at depth `d` is empty)
// ---------------------------------------------------------------------------
value_match_nested_5_test!(
    value_match_nested_5_u8_empty_0,
    PNested5U8,
    vec![],
    NESTED_5_U8_EMPTY_0,
    NESTED_5_U8_EMPTY_0_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_u8_empty_1,
    PNested5U8,
    vec![vec![]],
    NESTED_5_U8_EMPTY_1,
    NESTED_5_U8_EMPTY_1_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_u8_empty_2,
    PNested5U8,
    vec![vec![vec![]]],
    NESTED_5_U8_EMPTY_2,
    NESTED_5_U8_EMPTY_2_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_u8_empty_3,
    PNested5U8,
    vec![vec![vec![vec![]]]],
    NESTED_5_U8_EMPTY_3,
    NESTED_5_U8_EMPTY_3_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_u8_empty_4,
    PNested5U8,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_U8_EMPTY_4,
    NESTED_5_U8_EMPTY_4_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_i8_empty_0,
    PNested5I8,
    vec![],
    NESTED_5_I8_EMPTY_0,
    NESTED_5_I8_EMPTY_0_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_i8_empty_1,
    PNested5I8,
    vec![vec![]],
    NESTED_5_I8_EMPTY_1,
    NESTED_5_I8_EMPTY_1_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_i8_empty_2,
    PNested5I8,
    vec![vec![vec![]]],
    NESTED_5_I8_EMPTY_2,
    NESTED_5_I8_EMPTY_2_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_i8_empty_3,
    PNested5I8,
    vec![vec![vec![vec![]]]],
    NESTED_5_I8_EMPTY_3,
    NESTED_5_I8_EMPTY_3_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_i8_empty_4,
    PNested5I8,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_I8_EMPTY_4,
    NESTED_5_I8_EMPTY_4_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_u16_empty_0,
    PNested5U16,
    vec![],
    NESTED_5_U16_EMPTY_0,
    NESTED_5_U16_EMPTY_0_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_u16_empty_1,
    PNested5U16,
    vec![vec![]],
    NESTED_5_U16_EMPTY_1,
    NESTED_5_U16_EMPTY_1_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_u16_empty_2,
    PNested5U16,
    vec![vec![vec![]]],
    NESTED_5_U16_EMPTY_2,
    NESTED_5_U16_EMPTY_2_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_u16_empty_3,
    PNested5U16,
    vec![vec![vec![vec![]]]],
    NESTED_5_U16_EMPTY_3,
    NESTED_5_U16_EMPTY_3_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_u16_empty_4,
    PNested5U16,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_U16_EMPTY_4,
    NESTED_5_U16_EMPTY_4_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_i16_empty_0,
    PNested5I16,
    vec![],
    NESTED_5_I16_EMPTY_0,
    NESTED_5_I16_EMPTY_0_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_i16_empty_1,
    PNested5I16,
    vec![vec![]],
    NESTED_5_I16_EMPTY_1,
    NESTED_5_I16_EMPTY_1_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_i16_empty_2,
    PNested5I16,
    vec![vec![vec![]]],
    NESTED_5_I16_EMPTY_2,
    NESTED_5_I16_EMPTY_2_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_i16_empty_3,
    PNested5I16,
    vec![vec![vec![vec![]]]],
    NESTED_5_I16_EMPTY_3,
    NESTED_5_I16_EMPTY_3_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_i16_empty_4,
    PNested5I16,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_I16_EMPTY_4,
    NESTED_5_I16_EMPTY_4_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_u32_empty_0,
    PNested5U32,
    vec![],
    NESTED_5_U32_EMPTY_0,
    NESTED_5_U32_EMPTY_0_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_u32_empty_1,
    PNested5U32,
    vec![vec![]],
    NESTED_5_U32_EMPTY_1,
    NESTED_5_U32_EMPTY_1_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_u32_empty_2,
    PNested5U32,
    vec![vec![vec![]]],
    NESTED_5_U32_EMPTY_2,
    NESTED_5_U32_EMPTY_2_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_u32_empty_3,
    PNested5U32,
    vec![vec![vec![vec![]]]],
    NESTED_5_U32_EMPTY_3,
    NESTED_5_U32_EMPTY_3_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_u32_empty_4,
    PNested5U32,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_U32_EMPTY_4,
    NESTED_5_U32_EMPTY_4_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_i32_empty_0,
    PNested5I32,
    vec![],
    NESTED_5_I32_EMPTY_0,
    NESTED_5_I32_EMPTY_0_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_i32_empty_1,
    PNested5I32,
    vec![vec![]],
    NESTED_5_I32_EMPTY_1,
    NESTED_5_I32_EMPTY_1_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_i32_empty_2,
    PNested5I32,
    vec![vec![vec![]]],
    NESTED_5_I32_EMPTY_2,
    NESTED_5_I32_EMPTY_2_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_i32_empty_3,
    PNested5I32,
    vec![vec![vec![vec![]]]],
    NESTED_5_I32_EMPTY_3,
    NESTED_5_I32_EMPTY_3_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_i32_empty_4,
    PNested5I32,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_I32_EMPTY_4,
    NESTED_5_I32_EMPTY_4_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_u64_empty_0,
    PNested5U64,
    vec![],
    NESTED_5_U64_EMPTY_0,
    NESTED_5_U64_EMPTY_0_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_u64_empty_1,
    PNested5U64,
    vec![vec![]],
    NESTED_5_U64_EMPTY_1,
    NESTED_5_U64_EMPTY_1_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_u64_empty_2,
    PNested5U64,
    vec![vec![vec![]]],
    NESTED_5_U64_EMPTY_2,
    NESTED_5_U64_EMPTY_2_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_u64_empty_3,
    PNested5U64,
    vec![vec![vec![vec![]]]],
    NESTED_5_U64_EMPTY_3,
    NESTED_5_U64_EMPTY_3_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_u64_empty_4,
    PNested5U64,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_U64_EMPTY_4,
    NESTED_5_U64_EMPTY_4_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_i64_empty_0,
    PNested5I64,
    vec![],
    NESTED_5_I64_EMPTY_0,
    NESTED_5_I64_EMPTY_0_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_i64_empty_1,
    PNested5I64,
    vec![vec![]],
    NESTED_5_I64_EMPTY_1,
    NESTED_5_I64_EMPTY_1_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_i64_empty_2,
    PNested5I64,
    vec![vec![vec![]]],
    NESTED_5_I64_EMPTY_2,
    NESTED_5_I64_EMPTY_2_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_i64_empty_3,
    PNested5I64,
    vec![vec![vec![vec![]]]],
    NESTED_5_I64_EMPTY_3,
    NESTED_5_I64_EMPTY_3_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_i64_empty_4,
    PNested5I64,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_I64_EMPTY_4,
    NESTED_5_I64_EMPTY_4_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_f16_empty_0,
    PNested5F16,
    vec![],
    NESTED_5_F16_EMPTY_0,
    NESTED_5_F16_EMPTY_0_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_f16_empty_1,
    PNested5F16,
    vec![vec![]],
    NESTED_5_F16_EMPTY_1,
    NESTED_5_F16_EMPTY_1_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_f16_empty_2,
    PNested5F16,
    vec![vec![vec![]]],
    NESTED_5_F16_EMPTY_2,
    NESTED_5_F16_EMPTY_2_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_f16_empty_3,
    PNested5F16,
    vec![vec![vec![vec![]]]],
    NESTED_5_F16_EMPTY_3,
    NESTED_5_F16_EMPTY_3_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_f16_empty_4,
    PNested5F16,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_F16_EMPTY_4,
    NESTED_5_F16_EMPTY_4_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_f32_empty_0,
    PNested5F32,
    vec![],
    NESTED_5_F32_EMPTY_0,
    NESTED_5_F32_EMPTY_0_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_f32_empty_1,
    PNested5F32,
    vec![vec![]],
    NESTED_5_F32_EMPTY_1,
    NESTED_5_F32_EMPTY_1_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_f32_empty_2,
    PNested5F32,
    vec![vec![vec![]]],
    NESTED_5_F32_EMPTY_2,
    NESTED_5_F32_EMPTY_2_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_f32_empty_3,
    PNested5F32,
    vec![vec![vec![vec![]]]],
    NESTED_5_F32_EMPTY_3,
    NESTED_5_F32_EMPTY_3_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_f32_empty_4,
    PNested5F32,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_F32_EMPTY_4,
    NESTED_5_F32_EMPTY_4_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_f64_empty_0,
    PNested5F64,
    vec![],
    NESTED_5_F64_EMPTY_0,
    NESTED_5_F64_EMPTY_0_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_f64_empty_1,
    PNested5F64,
    vec![vec![]],
    NESTED_5_F64_EMPTY_1,
    NESTED_5_F64_EMPTY_1_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_f64_empty_2,
    PNested5F64,
    vec![vec![vec![]]],
    NESTED_5_F64_EMPTY_2,
    NESTED_5_F64_EMPTY_2_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_f64_empty_3,
    PNested5F64,
    vec![vec![vec![vec![]]]],
    NESTED_5_F64_EMPTY_3,
    NESTED_5_F64_EMPTY_3_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_f64_empty_4,
    PNested5F64,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_F64_EMPTY_4,
    NESTED_5_F64_EMPTY_4_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_bool_empty_0,
    PNested5Bool,
    vec![],
    NESTED_5_BOOL_EMPTY_0,
    NESTED_5_BOOL_EMPTY_0_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_bool_bitvec_empty_0,
    PNested5BitVec,
    vec![],
    NESTED_5_BOOL_EMPTY_0,
    NESTED_5_BOOL_EMPTY_0_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_bool_empty_1,
    PNested5Bool,
    vec![vec![]],
    NESTED_5_BOOL_EMPTY_1,
    NESTED_5_BOOL_EMPTY_1_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_bool_bitvec_empty_1,
    PNested5BitVec,
    vec![vec![]],
    NESTED_5_BOOL_EMPTY_1,
    NESTED_5_BOOL_EMPTY_1_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_bool_empty_2,
    PNested5Bool,
    vec![vec![vec![]]],
    NESTED_5_BOOL_EMPTY_2,
    NESTED_5_BOOL_EMPTY_2_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_bool_bitvec_empty_2,
    PNested5BitVec,
    vec![vec![vec![]]],
    NESTED_5_BOOL_EMPTY_2,
    NESTED_5_BOOL_EMPTY_2_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_bool_empty_3,
    PNested5Bool,
    vec![vec![vec![vec![]]]],
    NESTED_5_BOOL_EMPTY_3,
    NESTED_5_BOOL_EMPTY_3_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_bool_bitvec_empty_3,
    PNested5BitVec,
    vec![vec![vec![vec![]]]],
    NESTED_5_BOOL_EMPTY_3,
    NESTED_5_BOOL_EMPTY_3_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_bool_empty_4,
    PNested5Bool,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_BOOL_EMPTY_4,
    NESTED_5_BOOL_EMPTY_4_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_bool_bitvec_empty_4,
    PNested5BitVec,
    vec![vec![vec![vec![bits(&[])]]]],
    NESTED_5_BOOL_EMPTY_4,
    NESTED_5_BOOL_EMPTY_4_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_timestamp_empty_0,
    PNested5Timestamp,
    vec![],
    NESTED_5_TIMESTAMP_EMPTY_0,
    NESTED_5_TIMESTAMP_EMPTY_0_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_timestamp_empty_1,
    PNested5Timestamp,
    vec![vec![]],
    NESTED_5_TIMESTAMP_EMPTY_1,
    NESTED_5_TIMESTAMP_EMPTY_1_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_timestamp_empty_2,
    PNested5Timestamp,
    vec![vec![vec![]]],
    NESTED_5_TIMESTAMP_EMPTY_2,
    NESTED_5_TIMESTAMP_EMPTY_2_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_timestamp_empty_3,
    PNested5Timestamp,
    vec![vec![vec![vec![]]]],
    NESTED_5_TIMESTAMP_EMPTY_3,
    NESTED_5_TIMESTAMP_EMPTY_3_INODES
);

value_match_nested_5_test!(
    value_match_nested_5_timestamp_empty_4,
    PNested5Timestamp,
    vec![vec![vec![vec![vec![]]]]],
    NESTED_5_TIMESTAMP_EMPTY_4,
    NESTED_5_TIMESTAMP_EMPTY_4_INODES
);
