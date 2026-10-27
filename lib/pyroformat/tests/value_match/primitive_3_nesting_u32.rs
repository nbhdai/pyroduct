//! Struct-vs-value match for `src/wire/tests/primitive_3_nesting_u32.rs` —
//! `List<List<List<T>>>` against a derived struct with a native
//! `Vec<Vec<Vec<T>>>` field. See `tests/value_match/primitive_u32.rs`'s
//! module doc for the shared harness rationale.

use half::f16;

use pyroformat::value::Time;
use pyroformat::wire::tests::primitive_3_nesting_u32::*;

use crate::support::bits::bits;
use crate::support::read_back::assert_roundtrip;
use pyroformat::{BitVec, Layout, Reader, Writer};

macro_rules! value_match_nested_3_test {
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
struct PNested3U8 {
    #[pyro(numeric)]
    a: Vec<Vec<Vec<u8>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3I8 {
    a: Vec<Vec<Vec<i8>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3U16 {
    a: Vec<Vec<Vec<u16>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3I16 {
    a: Vec<Vec<Vec<i16>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3U32 {
    a: Vec<Vec<Vec<u32>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3I32 {
    a: Vec<Vec<Vec<i32>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3U64 {
    a: Vec<Vec<Vec<u64>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3I64 {
    a: Vec<Vec<Vec<i64>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3F16 {
    a: Vec<Vec<Vec<f16>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3F32 {
    a: Vec<Vec<Vec<f32>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3F64 {
    a: Vec<Vec<Vec<f64>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3Bool {
    #[pyro(allow(vec_bool))]
    a: Vec<Vec<Vec<bool>>>,
}

// `PNested3Bool` with the innermost `Vec<bool>` a packed `BitVec<u8, Lsb0>`:
// the same fixture either way.
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3BitVec {
    a: Vec<Vec<BitVec>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3Timestamp {
    a: Vec<Vec<Vec<Time>>>,
}

value_match_nested_3_test!(
    value_match_nested_3_u8,
    PNested3U8,
    vec![vec![vec![1, 2], vec![3]], vec![vec![4, 5, 6]]],
    NESTED_3_U8,
    NESTED_3_U8_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_i8,
    PNested3I8,
    vec![vec![vec![-1, 2], vec![-3]], vec![vec![4, -5, 6]]],
    NESTED_3_I8,
    NESTED_3_I8_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_u16,
    PNested3U16,
    vec![vec![vec![1, 2], vec![3]], vec![vec![4, 5, 6]]],
    NESTED_3_U16,
    NESTED_3_U16_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_i16,
    PNested3I16,
    vec![vec![vec![-1, 2], vec![-3]], vec![vec![4, -5, 6]]],
    NESTED_3_I16,
    NESTED_3_I16_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_u32,
    PNested3U32,
    vec![vec![vec![1, 2], vec![3]], vec![vec![4, 5, 6]]],
    NESTED_3_U32,
    NESTED_3_U32_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_i32,
    PNested3I32,
    vec![vec![vec![-1, 2], vec![-3]], vec![vec![4, -5, 6]]],
    NESTED_3_I32,
    NESTED_3_I32_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_u64,
    PNested3U64,
    vec![vec![vec![1, 2], vec![3]], vec![vec![4, 5, 6]]],
    NESTED_3_U64,
    NESTED_3_U64_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_i64,
    PNested3I64,
    vec![vec![vec![-1, 2], vec![-3]], vec![vec![4, -5, 6]]],
    NESTED_3_I64,
    NESTED_3_I64_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_f16,
    PNested3F16,
    vec![
        vec![
            vec![f16::from_f32(1.5), f16::from_f32(-2.0)],
            vec![f16::from_f32(3.25)]
        ],
        vec![vec![
            f16::from_f32(4.0),
            f16::from_f32(-5.5),
            f16::from_f32(6.0)
        ]]
    ],
    NESTED_3_F16,
    NESTED_3_F16_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_f32,
    PNested3F32,
    vec![
        vec![vec![1.5, -2.0], vec![3.25]],
        vec![vec![4.0, -5.5, 6.0]]
    ],
    NESTED_3_F32,
    NESTED_3_F32_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_f64,
    PNested3F64,
    vec![
        vec![vec![1.5, -2.0], vec![3.25]],
        vec![vec![4.0, -5.5, 6.0]]
    ],
    NESTED_3_F64,
    NESTED_3_F64_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_bool,
    PNested3Bool,
    vec![
        vec![vec![true, false], vec![true]],
        vec![vec![true, true, false]]
    ],
    NESTED_3_BOOL,
    NESTED_3_BOOL_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_bool_bitvec,
    PNested3BitVec,
    vec![
        vec![bits(&[true, false]), bits(&[true])],
        vec![bits(&[true, true, false])]
    ],
    NESTED_3_BOOL,
    NESTED_3_BOOL_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_timestamp,
    PNested3Timestamp,
    vec![
        vec![
            vec![Time(1_000_001_000), Time(-2_000_002_000)],
            vec![Time(3_500_003_000)]
        ],
        vec![vec![
            Time(4_000_004_000),
            Time(-5_000_005_000),
            Time(1_700_000_000_000_006_000)
        ]]
    ],
    NESTED_3_TIMESTAMP,
    NESTED_3_TIMESTAMP_INODES
);

// ---------------------------------------------------------------------------
// Empty lists at every depth (`_EMPTY_d`: the list at depth `d` is empty)
// ---------------------------------------------------------------------------
value_match_nested_3_test!(
    value_match_nested_3_u8_empty_0,
    PNested3U8,
    vec![],
    NESTED_3_U8_EMPTY_0,
    NESTED_3_U8_EMPTY_0_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_u8_empty_1,
    PNested3U8,
    vec![vec![]],
    NESTED_3_U8_EMPTY_1,
    NESTED_3_U8_EMPTY_1_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_u8_empty_2,
    PNested3U8,
    vec![vec![vec![]]],
    NESTED_3_U8_EMPTY_2,
    NESTED_3_U8_EMPTY_2_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_i8_empty_0,
    PNested3I8,
    vec![],
    NESTED_3_I8_EMPTY_0,
    NESTED_3_I8_EMPTY_0_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_i8_empty_1,
    PNested3I8,
    vec![vec![]],
    NESTED_3_I8_EMPTY_1,
    NESTED_3_I8_EMPTY_1_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_i8_empty_2,
    PNested3I8,
    vec![vec![vec![]]],
    NESTED_3_I8_EMPTY_2,
    NESTED_3_I8_EMPTY_2_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_u16_empty_0,
    PNested3U16,
    vec![],
    NESTED_3_U16_EMPTY_0,
    NESTED_3_U16_EMPTY_0_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_u16_empty_1,
    PNested3U16,
    vec![vec![]],
    NESTED_3_U16_EMPTY_1,
    NESTED_3_U16_EMPTY_1_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_u16_empty_2,
    PNested3U16,
    vec![vec![vec![]]],
    NESTED_3_U16_EMPTY_2,
    NESTED_3_U16_EMPTY_2_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_i16_empty_0,
    PNested3I16,
    vec![],
    NESTED_3_I16_EMPTY_0,
    NESTED_3_I16_EMPTY_0_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_i16_empty_1,
    PNested3I16,
    vec![vec![]],
    NESTED_3_I16_EMPTY_1,
    NESTED_3_I16_EMPTY_1_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_i16_empty_2,
    PNested3I16,
    vec![vec![vec![]]],
    NESTED_3_I16_EMPTY_2,
    NESTED_3_I16_EMPTY_2_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_u32_empty_0,
    PNested3U32,
    vec![],
    NESTED_3_U32_EMPTY_0,
    NESTED_3_U32_EMPTY_0_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_u32_empty_1,
    PNested3U32,
    vec![vec![]],
    NESTED_3_U32_EMPTY_1,
    NESTED_3_U32_EMPTY_1_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_u32_empty_2,
    PNested3U32,
    vec![vec![vec![]]],
    NESTED_3_U32_EMPTY_2,
    NESTED_3_U32_EMPTY_2_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_i32_empty_0,
    PNested3I32,
    vec![],
    NESTED_3_I32_EMPTY_0,
    NESTED_3_I32_EMPTY_0_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_i32_empty_1,
    PNested3I32,
    vec![vec![]],
    NESTED_3_I32_EMPTY_1,
    NESTED_3_I32_EMPTY_1_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_i32_empty_2,
    PNested3I32,
    vec![vec![vec![]]],
    NESTED_3_I32_EMPTY_2,
    NESTED_3_I32_EMPTY_2_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_u64_empty_0,
    PNested3U64,
    vec![],
    NESTED_3_U64_EMPTY_0,
    NESTED_3_U64_EMPTY_0_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_u64_empty_1,
    PNested3U64,
    vec![vec![]],
    NESTED_3_U64_EMPTY_1,
    NESTED_3_U64_EMPTY_1_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_u64_empty_2,
    PNested3U64,
    vec![vec![vec![]]],
    NESTED_3_U64_EMPTY_2,
    NESTED_3_U64_EMPTY_2_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_i64_empty_0,
    PNested3I64,
    vec![],
    NESTED_3_I64_EMPTY_0,
    NESTED_3_I64_EMPTY_0_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_i64_empty_1,
    PNested3I64,
    vec![vec![]],
    NESTED_3_I64_EMPTY_1,
    NESTED_3_I64_EMPTY_1_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_i64_empty_2,
    PNested3I64,
    vec![vec![vec![]]],
    NESTED_3_I64_EMPTY_2,
    NESTED_3_I64_EMPTY_2_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_f16_empty_0,
    PNested3F16,
    vec![],
    NESTED_3_F16_EMPTY_0,
    NESTED_3_F16_EMPTY_0_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_f16_empty_1,
    PNested3F16,
    vec![vec![]],
    NESTED_3_F16_EMPTY_1,
    NESTED_3_F16_EMPTY_1_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_f16_empty_2,
    PNested3F16,
    vec![vec![vec![]]],
    NESTED_3_F16_EMPTY_2,
    NESTED_3_F16_EMPTY_2_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_f32_empty_0,
    PNested3F32,
    vec![],
    NESTED_3_F32_EMPTY_0,
    NESTED_3_F32_EMPTY_0_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_f32_empty_1,
    PNested3F32,
    vec![vec![]],
    NESTED_3_F32_EMPTY_1,
    NESTED_3_F32_EMPTY_1_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_f32_empty_2,
    PNested3F32,
    vec![vec![vec![]]],
    NESTED_3_F32_EMPTY_2,
    NESTED_3_F32_EMPTY_2_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_f64_empty_0,
    PNested3F64,
    vec![],
    NESTED_3_F64_EMPTY_0,
    NESTED_3_F64_EMPTY_0_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_f64_empty_1,
    PNested3F64,
    vec![vec![]],
    NESTED_3_F64_EMPTY_1,
    NESTED_3_F64_EMPTY_1_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_f64_empty_2,
    PNested3F64,
    vec![vec![vec![]]],
    NESTED_3_F64_EMPTY_2,
    NESTED_3_F64_EMPTY_2_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_bool_empty_0,
    PNested3Bool,
    vec![],
    NESTED_3_BOOL_EMPTY_0,
    NESTED_3_BOOL_EMPTY_0_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_bool_bitvec_empty_0,
    PNested3BitVec,
    vec![],
    NESTED_3_BOOL_EMPTY_0,
    NESTED_3_BOOL_EMPTY_0_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_bool_empty_1,
    PNested3Bool,
    vec![vec![]],
    NESTED_3_BOOL_EMPTY_1,
    NESTED_3_BOOL_EMPTY_1_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_bool_bitvec_empty_1,
    PNested3BitVec,
    vec![vec![]],
    NESTED_3_BOOL_EMPTY_1,
    NESTED_3_BOOL_EMPTY_1_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_bool_empty_2,
    PNested3Bool,
    vec![vec![vec![]]],
    NESTED_3_BOOL_EMPTY_2,
    NESTED_3_BOOL_EMPTY_2_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_bool_bitvec_empty_2,
    PNested3BitVec,
    vec![vec![bits(&[])]],
    NESTED_3_BOOL_EMPTY_2,
    NESTED_3_BOOL_EMPTY_2_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_timestamp_empty_0,
    PNested3Timestamp,
    vec![],
    NESTED_3_TIMESTAMP_EMPTY_0,
    NESTED_3_TIMESTAMP_EMPTY_0_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_timestamp_empty_1,
    PNested3Timestamp,
    vec![vec![]],
    NESTED_3_TIMESTAMP_EMPTY_1,
    NESTED_3_TIMESTAMP_EMPTY_1_INODES
);

value_match_nested_3_test!(
    value_match_nested_3_timestamp_empty_2,
    PNested3Timestamp,
    vec![vec![vec![]]],
    NESTED_3_TIMESTAMP_EMPTY_2,
    NESTED_3_TIMESTAMP_EMPTY_2_INODES
);
