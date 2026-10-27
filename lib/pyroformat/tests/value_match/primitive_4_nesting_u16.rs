//! Struct-vs-value match for `src/wire/tests/primitive_4_nesting_u16.rs` —
//! `List<List<List<List<T>>>>` against a derived struct with a native
//! `Vec<Vec<Vec<Vec<T>>>>` field. See `tests/value_match/primitive_u16.rs`'s
//! module doc for the shared harness rationale.

use half::f16;

use pyroformat::value::Time;
use pyroformat::wire::tests::primitive_4_nesting_u16::*;

use crate::support::bits::bits;
use crate::support::read_back::assert_roundtrip;
use pyroformat::{BitVec, Layout, Reader, Writer};

macro_rules! value_match_nested_4_test {
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
struct PNested4U8 {
    #[pyro(numeric)]
    a: Vec<Vec<Vec<Vec<u8>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4I8 {
    a: Vec<Vec<Vec<Vec<i8>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4U16 {
    a: Vec<Vec<Vec<Vec<u16>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4I16 {
    a: Vec<Vec<Vec<Vec<i16>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4U32 {
    a: Vec<Vec<Vec<Vec<u32>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4I32 {
    a: Vec<Vec<Vec<Vec<i32>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4U64 {
    a: Vec<Vec<Vec<Vec<u64>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4I64 {
    a: Vec<Vec<Vec<Vec<i64>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4F16 {
    a: Vec<Vec<Vec<Vec<f16>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4F32 {
    a: Vec<Vec<Vec<Vec<f32>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4F64 {
    a: Vec<Vec<Vec<Vec<f64>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4Bool {
    #[pyro(allow(vec_bool))]
    a: Vec<Vec<Vec<Vec<bool>>>>,
}

// `PNested4Bool` with the innermost `Vec<bool>` a packed `BitVec<u8, Lsb0>`:
// the same fixture either way.
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4BitVec {
    a: Vec<Vec<Vec<BitVec>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4Timestamp {
    a: Vec<Vec<Vec<Vec<Time>>>>,
}

value_match_nested_4_test!(
    value_match_nested_4_u8,
    PNested4U8,
    vec![
        vec![vec![vec![1, 2], vec![3]], vec![vec![4]]],
        vec![vec![vec![5, 6]]]
    ],
    NESTED_4_U8,
    NESTED_4_U8_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_i8,
    PNested4I8,
    vec![
        vec![vec![vec![-1, 2], vec![-3]], vec![vec![4]]],
        vec![vec![vec![-5, 6]]]
    ],
    NESTED_4_I8,
    NESTED_4_I8_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_u16,
    PNested4U16,
    vec![
        vec![vec![vec![1, 2], vec![3]], vec![vec![4]]],
        vec![vec![vec![5, 6]]]
    ],
    NESTED_4_U16,
    NESTED_4_U16_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_i16,
    PNested4I16,
    vec![
        vec![vec![vec![-1, 2], vec![-3]], vec![vec![4]]],
        vec![vec![vec![-5, 6]]]
    ],
    NESTED_4_I16,
    NESTED_4_I16_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_u32,
    PNested4U32,
    vec![
        vec![vec![vec![1, 2], vec![3]], vec![vec![4]]],
        vec![vec![vec![5, 6]]]
    ],
    NESTED_4_U32,
    NESTED_4_U32_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_i32,
    PNested4I32,
    vec![
        vec![vec![vec![-1, 2], vec![-3]], vec![vec![4]]],
        vec![vec![vec![-5, 6]]]
    ],
    NESTED_4_I32,
    NESTED_4_I32_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_u64,
    PNested4U64,
    vec![
        vec![vec![vec![1, 2], vec![3]], vec![vec![4]]],
        vec![vec![vec![5, 6]]]
    ],
    NESTED_4_U64,
    NESTED_4_U64_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_i64,
    PNested4I64,
    vec![
        vec![vec![vec![-1, 2], vec![-3]], vec![vec![4]]],
        vec![vec![vec![-5, 6]]]
    ],
    NESTED_4_I64,
    NESTED_4_I64_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_f16,
    PNested4F16,
    vec![
        vec![
            vec![
                vec![f16::from_f32(1.5), f16::from_f32(-2.0)],
                vec![f16::from_f32(3.25)]
            ],
            vec![vec![f16::from_f32(4.0)]]
        ],
        vec![vec![vec![f16::from_f32(-5.5), f16::from_f32(6.0)]]]
    ],
    NESTED_4_F16,
    NESTED_4_F16_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_f32,
    PNested4F32,
    vec![
        vec![vec![vec![1.5, -2.0], vec![3.25]], vec![vec![4.0]]],
        vec![vec![vec![-5.5, 6.0]]]
    ],
    NESTED_4_F32,
    NESTED_4_F32_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_f64,
    PNested4F64,
    vec![
        vec![vec![vec![1.5, -2.0], vec![3.25]], vec![vec![4.0]]],
        vec![vec![vec![-5.5, 6.0]]]
    ],
    NESTED_4_F64,
    NESTED_4_F64_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_bool,
    PNested4Bool,
    vec![
        vec![vec![vec![true, false], vec![true]], vec![vec![true]]],
        vec![vec![vec![true, false]]]
    ],
    NESTED_4_BOOL,
    NESTED_4_BOOL_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_bool_bitvec,
    PNested4BitVec,
    vec![
        vec![
            vec![bits(&[true, false]), bits(&[true])],
            vec![bits(&[true])]
        ],
        vec![vec![bits(&[true, false])]]
    ],
    NESTED_4_BOOL,
    NESTED_4_BOOL_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_timestamp,
    PNested4Timestamp,
    vec![
        vec![
            vec![
                vec![Time(1_000_001_000), Time(-2_000_002_000)],
                vec![Time(3_500_003_000)]
            ],
            vec![vec![Time(4_000_004_000)]]
        ],
        vec![vec![vec![
            Time(-5_000_005_000),
            Time(1_700_000_000_000_006_000)
        ]]]
    ],
    NESTED_4_TIMESTAMP,
    NESTED_4_TIMESTAMP_INODES
);

// ---------------------------------------------------------------------------
// Empty lists at every depth (`_EMPTY_d`: the list at depth `d` is empty)
// ---------------------------------------------------------------------------
value_match_nested_4_test!(
    value_match_nested_4_u8_empty_0,
    PNested4U8,
    vec![],
    NESTED_4_U8_EMPTY_0,
    NESTED_4_U8_EMPTY_0_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_u8_empty_1,
    PNested4U8,
    vec![vec![]],
    NESTED_4_U8_EMPTY_1,
    NESTED_4_U8_EMPTY_1_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_u8_empty_2,
    PNested4U8,
    vec![vec![vec![]]],
    NESTED_4_U8_EMPTY_2,
    NESTED_4_U8_EMPTY_2_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_u8_empty_3,
    PNested4U8,
    vec![vec![vec![vec![]]]],
    NESTED_4_U8_EMPTY_3,
    NESTED_4_U8_EMPTY_3_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_i8_empty_0,
    PNested4I8,
    vec![],
    NESTED_4_I8_EMPTY_0,
    NESTED_4_I8_EMPTY_0_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_i8_empty_1,
    PNested4I8,
    vec![vec![]],
    NESTED_4_I8_EMPTY_1,
    NESTED_4_I8_EMPTY_1_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_i8_empty_2,
    PNested4I8,
    vec![vec![vec![]]],
    NESTED_4_I8_EMPTY_2,
    NESTED_4_I8_EMPTY_2_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_i8_empty_3,
    PNested4I8,
    vec![vec![vec![vec![]]]],
    NESTED_4_I8_EMPTY_3,
    NESTED_4_I8_EMPTY_3_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_u16_empty_0,
    PNested4U16,
    vec![],
    NESTED_4_U16_EMPTY_0,
    NESTED_4_U16_EMPTY_0_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_u16_empty_1,
    PNested4U16,
    vec![vec![]],
    NESTED_4_U16_EMPTY_1,
    NESTED_4_U16_EMPTY_1_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_u16_empty_2,
    PNested4U16,
    vec![vec![vec![]]],
    NESTED_4_U16_EMPTY_2,
    NESTED_4_U16_EMPTY_2_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_u16_empty_3,
    PNested4U16,
    vec![vec![vec![vec![]]]],
    NESTED_4_U16_EMPTY_3,
    NESTED_4_U16_EMPTY_3_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_i16_empty_0,
    PNested4I16,
    vec![],
    NESTED_4_I16_EMPTY_0,
    NESTED_4_I16_EMPTY_0_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_i16_empty_1,
    PNested4I16,
    vec![vec![]],
    NESTED_4_I16_EMPTY_1,
    NESTED_4_I16_EMPTY_1_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_i16_empty_2,
    PNested4I16,
    vec![vec![vec![]]],
    NESTED_4_I16_EMPTY_2,
    NESTED_4_I16_EMPTY_2_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_i16_empty_3,
    PNested4I16,
    vec![vec![vec![vec![]]]],
    NESTED_4_I16_EMPTY_3,
    NESTED_4_I16_EMPTY_3_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_u32_empty_0,
    PNested4U32,
    vec![],
    NESTED_4_U32_EMPTY_0,
    NESTED_4_U32_EMPTY_0_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_u32_empty_1,
    PNested4U32,
    vec![vec![]],
    NESTED_4_U32_EMPTY_1,
    NESTED_4_U32_EMPTY_1_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_u32_empty_2,
    PNested4U32,
    vec![vec![vec![]]],
    NESTED_4_U32_EMPTY_2,
    NESTED_4_U32_EMPTY_2_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_u32_empty_3,
    PNested4U32,
    vec![vec![vec![vec![]]]],
    NESTED_4_U32_EMPTY_3,
    NESTED_4_U32_EMPTY_3_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_i32_empty_0,
    PNested4I32,
    vec![],
    NESTED_4_I32_EMPTY_0,
    NESTED_4_I32_EMPTY_0_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_i32_empty_1,
    PNested4I32,
    vec![vec![]],
    NESTED_4_I32_EMPTY_1,
    NESTED_4_I32_EMPTY_1_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_i32_empty_2,
    PNested4I32,
    vec![vec![vec![]]],
    NESTED_4_I32_EMPTY_2,
    NESTED_4_I32_EMPTY_2_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_i32_empty_3,
    PNested4I32,
    vec![vec![vec![vec![]]]],
    NESTED_4_I32_EMPTY_3,
    NESTED_4_I32_EMPTY_3_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_u64_empty_0,
    PNested4U64,
    vec![],
    NESTED_4_U64_EMPTY_0,
    NESTED_4_U64_EMPTY_0_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_u64_empty_1,
    PNested4U64,
    vec![vec![]],
    NESTED_4_U64_EMPTY_1,
    NESTED_4_U64_EMPTY_1_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_u64_empty_2,
    PNested4U64,
    vec![vec![vec![]]],
    NESTED_4_U64_EMPTY_2,
    NESTED_4_U64_EMPTY_2_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_u64_empty_3,
    PNested4U64,
    vec![vec![vec![vec![]]]],
    NESTED_4_U64_EMPTY_3,
    NESTED_4_U64_EMPTY_3_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_i64_empty_0,
    PNested4I64,
    vec![],
    NESTED_4_I64_EMPTY_0,
    NESTED_4_I64_EMPTY_0_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_i64_empty_1,
    PNested4I64,
    vec![vec![]],
    NESTED_4_I64_EMPTY_1,
    NESTED_4_I64_EMPTY_1_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_i64_empty_2,
    PNested4I64,
    vec![vec![vec![]]],
    NESTED_4_I64_EMPTY_2,
    NESTED_4_I64_EMPTY_2_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_i64_empty_3,
    PNested4I64,
    vec![vec![vec![vec![]]]],
    NESTED_4_I64_EMPTY_3,
    NESTED_4_I64_EMPTY_3_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_f16_empty_0,
    PNested4F16,
    vec![],
    NESTED_4_F16_EMPTY_0,
    NESTED_4_F16_EMPTY_0_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_f16_empty_1,
    PNested4F16,
    vec![vec![]],
    NESTED_4_F16_EMPTY_1,
    NESTED_4_F16_EMPTY_1_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_f16_empty_2,
    PNested4F16,
    vec![vec![vec![]]],
    NESTED_4_F16_EMPTY_2,
    NESTED_4_F16_EMPTY_2_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_f16_empty_3,
    PNested4F16,
    vec![vec![vec![vec![]]]],
    NESTED_4_F16_EMPTY_3,
    NESTED_4_F16_EMPTY_3_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_f32_empty_0,
    PNested4F32,
    vec![],
    NESTED_4_F32_EMPTY_0,
    NESTED_4_F32_EMPTY_0_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_f32_empty_1,
    PNested4F32,
    vec![vec![]],
    NESTED_4_F32_EMPTY_1,
    NESTED_4_F32_EMPTY_1_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_f32_empty_2,
    PNested4F32,
    vec![vec![vec![]]],
    NESTED_4_F32_EMPTY_2,
    NESTED_4_F32_EMPTY_2_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_f32_empty_3,
    PNested4F32,
    vec![vec![vec![vec![]]]],
    NESTED_4_F32_EMPTY_3,
    NESTED_4_F32_EMPTY_3_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_f64_empty_0,
    PNested4F64,
    vec![],
    NESTED_4_F64_EMPTY_0,
    NESTED_4_F64_EMPTY_0_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_f64_empty_1,
    PNested4F64,
    vec![vec![]],
    NESTED_4_F64_EMPTY_1,
    NESTED_4_F64_EMPTY_1_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_f64_empty_2,
    PNested4F64,
    vec![vec![vec![]]],
    NESTED_4_F64_EMPTY_2,
    NESTED_4_F64_EMPTY_2_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_f64_empty_3,
    PNested4F64,
    vec![vec![vec![vec![]]]],
    NESTED_4_F64_EMPTY_3,
    NESTED_4_F64_EMPTY_3_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_bool_empty_0,
    PNested4Bool,
    vec![],
    NESTED_4_BOOL_EMPTY_0,
    NESTED_4_BOOL_EMPTY_0_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_bool_bitvec_empty_0,
    PNested4BitVec,
    vec![],
    NESTED_4_BOOL_EMPTY_0,
    NESTED_4_BOOL_EMPTY_0_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_bool_empty_1,
    PNested4Bool,
    vec![vec![]],
    NESTED_4_BOOL_EMPTY_1,
    NESTED_4_BOOL_EMPTY_1_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_bool_bitvec_empty_1,
    PNested4BitVec,
    vec![vec![]],
    NESTED_4_BOOL_EMPTY_1,
    NESTED_4_BOOL_EMPTY_1_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_bool_empty_2,
    PNested4Bool,
    vec![vec![vec![]]],
    NESTED_4_BOOL_EMPTY_2,
    NESTED_4_BOOL_EMPTY_2_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_bool_bitvec_empty_2,
    PNested4BitVec,
    vec![vec![vec![]]],
    NESTED_4_BOOL_EMPTY_2,
    NESTED_4_BOOL_EMPTY_2_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_bool_empty_3,
    PNested4Bool,
    vec![vec![vec![vec![]]]],
    NESTED_4_BOOL_EMPTY_3,
    NESTED_4_BOOL_EMPTY_3_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_bool_bitvec_empty_3,
    PNested4BitVec,
    vec![vec![vec![bits(&[])]]],
    NESTED_4_BOOL_EMPTY_3,
    NESTED_4_BOOL_EMPTY_3_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_timestamp_empty_0,
    PNested4Timestamp,
    vec![],
    NESTED_4_TIMESTAMP_EMPTY_0,
    NESTED_4_TIMESTAMP_EMPTY_0_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_timestamp_empty_1,
    PNested4Timestamp,
    vec![vec![]],
    NESTED_4_TIMESTAMP_EMPTY_1,
    NESTED_4_TIMESTAMP_EMPTY_1_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_timestamp_empty_2,
    PNested4Timestamp,
    vec![vec![vec![]]],
    NESTED_4_TIMESTAMP_EMPTY_2,
    NESTED_4_TIMESTAMP_EMPTY_2_INODES
);

value_match_nested_4_test!(
    value_match_nested_4_timestamp_empty_3,
    PNested4Timestamp,
    vec![vec![vec![vec![]]]],
    NESTED_4_TIMESTAMP_EMPTY_3,
    NESTED_4_TIMESTAMP_EMPTY_3_INODES
);
