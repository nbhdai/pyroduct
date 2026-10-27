//! Struct-vs-value match for `src/wire/tests/primitive_2_nesting_u16.rs` —
//! `List<List<T>>` against a derived struct with a native
//! `Vec<Vec<T>>` field. See `tests/value_match/primitive_u16.rs`'s
//! module doc for the shared harness rationale.

use half::f16;

use pyroformat::value::Time;
use pyroformat::wire::tests::primitive_2_nesting_u16::*;

use crate::support::bits::bits;
use crate::support::read_back::assert_roundtrip;
use pyroformat::{BitVec, Layout, Reader, Writer};

macro_rules! value_match_nested_2_test {
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
struct PNested2U8 {
    #[pyro(numeric)]
    a: Vec<Vec<u8>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2I8 {
    a: Vec<Vec<i8>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2U16 {
    a: Vec<Vec<u16>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2I16 {
    a: Vec<Vec<i16>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2U32 {
    a: Vec<Vec<u32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2I32 {
    a: Vec<Vec<i32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2U64 {
    a: Vec<Vec<u64>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2I64 {
    a: Vec<Vec<i64>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2F16 {
    a: Vec<Vec<f16>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2F32 {
    a: Vec<Vec<f32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2F64 {
    a: Vec<Vec<f64>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2Bool {
    #[pyro(allow(vec_bool))]
    a: Vec<Vec<bool>>,
}

// `PNested2Bool` with the innermost `Vec<bool>` a packed `BitVec<u8, Lsb0>`:
// the same fixture either way.
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2BitVec {
    a: Vec<BitVec>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2Timestamp {
    a: Vec<Vec<Time>>,
}

value_match_nested_2_test!(
    value_match_nested_2_u8,
    PNested2U8,
    vec![vec![1, 2], vec![3], vec![4, 5, 6]],
    NESTED_2_U8,
    NESTED_2_U8_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_i8,
    PNested2I8,
    vec![vec![-1, 2], vec![-3], vec![4, -5, 6]],
    NESTED_2_I8,
    NESTED_2_I8_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_u16,
    PNested2U16,
    vec![vec![1, 2], vec![3], vec![4, 5, 6]],
    NESTED_2_U16,
    NESTED_2_U16_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_i16,
    PNested2I16,
    vec![vec![-1, 2], vec![-3], vec![4, -5, 6]],
    NESTED_2_I16,
    NESTED_2_I16_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_u32,
    PNested2U32,
    vec![vec![1, 2], vec![3], vec![4, 5, 6]],
    NESTED_2_U32,
    NESTED_2_U32_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_i32,
    PNested2I32,
    vec![vec![-1, 2], vec![-3], vec![4, -5, 6]],
    NESTED_2_I32,
    NESTED_2_I32_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_u64,
    PNested2U64,
    vec![vec![1, 2], vec![3], vec![4, 5, 6]],
    NESTED_2_U64,
    NESTED_2_U64_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_i64,
    PNested2I64,
    vec![vec![-1, 2], vec![-3], vec![4, -5, 6]],
    NESTED_2_I64,
    NESTED_2_I64_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_f16,
    PNested2F16,
    vec![
        vec![f16::from_f32(1.5), f16::from_f32(-2.0)],
        vec![f16::from_f32(3.25)],
        vec![f16::from_f32(4.0), f16::from_f32(-5.5), f16::from_f32(6.0)]
    ],
    NESTED_2_F16,
    NESTED_2_F16_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_f32,
    PNested2F32,
    vec![vec![1.5, -2.0], vec![3.25], vec![4.0, -5.5, 6.0]],
    NESTED_2_F32,
    NESTED_2_F32_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_f64,
    PNested2F64,
    vec![vec![1.5, -2.0], vec![3.25], vec![4.0, -5.5, 6.0]],
    NESTED_2_F64,
    NESTED_2_F64_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_bool,
    PNested2Bool,
    vec![vec![true, false], vec![true], vec![true, true, false]],
    NESTED_2_BOOL,
    NESTED_2_BOOL_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_bool_bitvec,
    PNested2BitVec,
    vec![
        bits(&[true, false]),
        bits(&[true]),
        bits(&[true, true, false])
    ],
    NESTED_2_BOOL,
    NESTED_2_BOOL_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_timestamp,
    PNested2Timestamp,
    vec![
        vec![Time(1_000_001_000), Time(-2_000_002_000)],
        vec![Time(3_500_003_000)],
        vec![
            Time(4_000_004_000),
            Time(-5_000_005_000),
            Time(1_700_000_000_000_006_000)
        ]
    ],
    NESTED_2_TIMESTAMP,
    NESTED_2_TIMESTAMP_INODES
);

// ---------------------------------------------------------------------------
// Empty lists at every depth (`_EMPTY_d`: the list at depth `d` is empty)
// ---------------------------------------------------------------------------
value_match_nested_2_test!(
    value_match_nested_2_u8_empty_0,
    PNested2U8,
    vec![],
    NESTED_2_U8_EMPTY_0,
    NESTED_2_U8_EMPTY_0_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_u8_empty_1,
    PNested2U8,
    vec![vec![]],
    NESTED_2_U8_EMPTY_1,
    NESTED_2_U8_EMPTY_1_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_i8_empty_0,
    PNested2I8,
    vec![],
    NESTED_2_I8_EMPTY_0,
    NESTED_2_I8_EMPTY_0_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_i8_empty_1,
    PNested2I8,
    vec![vec![]],
    NESTED_2_I8_EMPTY_1,
    NESTED_2_I8_EMPTY_1_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_u16_empty_0,
    PNested2U16,
    vec![],
    NESTED_2_U16_EMPTY_0,
    NESTED_2_U16_EMPTY_0_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_u16_empty_1,
    PNested2U16,
    vec![vec![]],
    NESTED_2_U16_EMPTY_1,
    NESTED_2_U16_EMPTY_1_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_i16_empty_0,
    PNested2I16,
    vec![],
    NESTED_2_I16_EMPTY_0,
    NESTED_2_I16_EMPTY_0_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_i16_empty_1,
    PNested2I16,
    vec![vec![]],
    NESTED_2_I16_EMPTY_1,
    NESTED_2_I16_EMPTY_1_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_u32_empty_0,
    PNested2U32,
    vec![],
    NESTED_2_U32_EMPTY_0,
    NESTED_2_U32_EMPTY_0_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_u32_empty_1,
    PNested2U32,
    vec![vec![]],
    NESTED_2_U32_EMPTY_1,
    NESTED_2_U32_EMPTY_1_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_i32_empty_0,
    PNested2I32,
    vec![],
    NESTED_2_I32_EMPTY_0,
    NESTED_2_I32_EMPTY_0_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_i32_empty_1,
    PNested2I32,
    vec![vec![]],
    NESTED_2_I32_EMPTY_1,
    NESTED_2_I32_EMPTY_1_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_u64_empty_0,
    PNested2U64,
    vec![],
    NESTED_2_U64_EMPTY_0,
    NESTED_2_U64_EMPTY_0_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_u64_empty_1,
    PNested2U64,
    vec![vec![]],
    NESTED_2_U64_EMPTY_1,
    NESTED_2_U64_EMPTY_1_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_i64_empty_0,
    PNested2I64,
    vec![],
    NESTED_2_I64_EMPTY_0,
    NESTED_2_I64_EMPTY_0_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_i64_empty_1,
    PNested2I64,
    vec![vec![]],
    NESTED_2_I64_EMPTY_1,
    NESTED_2_I64_EMPTY_1_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_f16_empty_0,
    PNested2F16,
    vec![],
    NESTED_2_F16_EMPTY_0,
    NESTED_2_F16_EMPTY_0_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_f16_empty_1,
    PNested2F16,
    vec![vec![]],
    NESTED_2_F16_EMPTY_1,
    NESTED_2_F16_EMPTY_1_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_f32_empty_0,
    PNested2F32,
    vec![],
    NESTED_2_F32_EMPTY_0,
    NESTED_2_F32_EMPTY_0_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_f32_empty_1,
    PNested2F32,
    vec![vec![]],
    NESTED_2_F32_EMPTY_1,
    NESTED_2_F32_EMPTY_1_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_f64_empty_0,
    PNested2F64,
    vec![],
    NESTED_2_F64_EMPTY_0,
    NESTED_2_F64_EMPTY_0_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_f64_empty_1,
    PNested2F64,
    vec![vec![]],
    NESTED_2_F64_EMPTY_1,
    NESTED_2_F64_EMPTY_1_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_bool_empty_0,
    PNested2Bool,
    vec![],
    NESTED_2_BOOL_EMPTY_0,
    NESTED_2_BOOL_EMPTY_0_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_bool_bitvec_empty_0,
    PNested2BitVec,
    vec![],
    NESTED_2_BOOL_EMPTY_0,
    NESTED_2_BOOL_EMPTY_0_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_bool_empty_1,
    PNested2Bool,
    vec![vec![]],
    NESTED_2_BOOL_EMPTY_1,
    NESTED_2_BOOL_EMPTY_1_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_bool_bitvec_empty_1,
    PNested2BitVec,
    vec![bits(&[])],
    NESTED_2_BOOL_EMPTY_1,
    NESTED_2_BOOL_EMPTY_1_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_timestamp_empty_0,
    PNested2Timestamp,
    vec![],
    NESTED_2_TIMESTAMP_EMPTY_0,
    NESTED_2_TIMESTAMP_EMPTY_0_INODES
);

value_match_nested_2_test!(
    value_match_nested_2_timestamp_empty_1,
    PNested2Timestamp,
    vec![vec![]],
    NESTED_2_TIMESTAMP_EMPTY_1,
    NESTED_2_TIMESTAMP_EMPTY_1_INODES
);
