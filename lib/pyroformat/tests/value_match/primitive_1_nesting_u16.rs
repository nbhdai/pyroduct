//! Struct-vs-value match for `src/wire/tests/primitive_1_nesting_u16.rs` —
//! `List<T>` against a derived struct with a native
//! `Vec<T>` field. See `tests/value_match/primitive_u16.rs`'s
//! module doc for the shared harness rationale.

use half::f16;

use pyroformat::value::Time;
use pyroformat::wire::tests::primitive_1_nesting_u16::*;

use crate::support::bits::bits;
use crate::support::read_back::assert_roundtrip;
use pyroformat::{BitVec, Layout, Reader, Writer};

macro_rules! value_match_nested_1_test {
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
struct PNested1U8 {
    #[pyro(numeric)]
    a: Vec<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1I8 {
    a: Vec<i8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1U16 {
    a: Vec<u16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1I16 {
    a: Vec<i16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1U32 {
    a: Vec<u32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1I32 {
    a: Vec<i32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1U64 {
    a: Vec<u64>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1I64 {
    a: Vec<i64>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1F16 {
    a: Vec<f16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1F32 {
    a: Vec<f32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1F64 {
    a: Vec<f64>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1Bool {
    #[pyro(allow(vec_bool))]
    a: Vec<bool>,
}

// `PNested1Bool` with the innermost `Vec<bool>` a packed `BitVec<u8, Lsb0>`:
// the same fixture either way.
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1BitVec {
    a: BitVec,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1Timestamp {
    a: Vec<Time>,
}

value_match_nested_1_test!(
    value_match_nested_1_u8,
    PNested1U8,
    vec![1, 2, 3, 4, 5, 6],
    NESTED_1_U8,
    NESTED_1_U8_INODES
);

value_match_nested_1_test!(
    value_match_nested_1_i8,
    PNested1I8,
    vec![-1, 2, -3, 4, -5, 6],
    NESTED_1_I8,
    NESTED_1_I8_INODES
);

value_match_nested_1_test!(
    value_match_nested_1_u16,
    PNested1U16,
    vec![1, 2, 3, 4, 5, 6],
    NESTED_1_U16,
    NESTED_1_U16_INODES
);

value_match_nested_1_test!(
    value_match_nested_1_i16,
    PNested1I16,
    vec![-1, 2, -3, 4, -5, 6],
    NESTED_1_I16,
    NESTED_1_I16_INODES
);

value_match_nested_1_test!(
    value_match_nested_1_u32,
    PNested1U32,
    vec![1, 2, 3, 4, 5, 6],
    NESTED_1_U32,
    NESTED_1_U32_INODES
);

value_match_nested_1_test!(
    value_match_nested_1_i32,
    PNested1I32,
    vec![-1, 2, -3, 4, -5, 6],
    NESTED_1_I32,
    NESTED_1_I32_INODES
);

value_match_nested_1_test!(
    value_match_nested_1_u64,
    PNested1U64,
    vec![1, 2, 3, 4, 5, 6],
    NESTED_1_U64,
    NESTED_1_U64_INODES
);

value_match_nested_1_test!(
    value_match_nested_1_i64,
    PNested1I64,
    vec![-1, 2, -3, 4, -5, 6],
    NESTED_1_I64,
    NESTED_1_I64_INODES
);

value_match_nested_1_test!(
    value_match_nested_1_f16,
    PNested1F16,
    vec![
        f16::from_f32(1.5),
        f16::from_f32(-2.0),
        f16::from_f32(3.25),
        f16::from_f32(4.0),
        f16::from_f32(-5.5),
        f16::from_f32(6.0)
    ],
    NESTED_1_F16,
    NESTED_1_F16_INODES
);

value_match_nested_1_test!(
    value_match_nested_1_f32,
    PNested1F32,
    vec![1.5, -2.0, 3.25, 4.0, -5.5, 6.0],
    NESTED_1_F32,
    NESTED_1_F32_INODES
);

value_match_nested_1_test!(
    value_match_nested_1_f64,
    PNested1F64,
    vec![1.5, -2.0, 3.25, 4.0, -5.5, 6.0],
    NESTED_1_F64,
    NESTED_1_F64_INODES
);

value_match_nested_1_test!(
    value_match_nested_1_bool,
    PNested1Bool,
    vec![true, false, true, true, true, false],
    NESTED_1_BOOL,
    NESTED_1_BOOL_INODES
);

value_match_nested_1_test!(
    value_match_nested_1_bool_bitvec,
    PNested1BitVec,
    bits(&[true, false, true, true, true, false]),
    NESTED_1_BOOL,
    NESTED_1_BOOL_INODES
);

value_match_nested_1_test!(
    value_match_nested_1_timestamp,
    PNested1Timestamp,
    vec![
        Time(1_000_001_000),
        Time(-2_000_002_000),
        Time(3_500_003_000),
        Time(4_000_004_000),
        Time(-5_000_005_000),
        Time(1_700_000_000_000_006_000)
    ],
    NESTED_1_TIMESTAMP,
    NESTED_1_TIMESTAMP_INODES
);

// ---------------------------------------------------------------------------
// Empty lists at every depth (`_EMPTY_d`: the list at depth `d` is empty)
// ---------------------------------------------------------------------------
value_match_nested_1_test!(
    value_match_nested_1_u8_empty_0,
    PNested1U8,
    vec![],
    NESTED_1_U8_EMPTY_0,
    NESTED_1_U8_EMPTY_0_INODES
);

value_match_nested_1_test!(
    value_match_nested_1_i8_empty_0,
    PNested1I8,
    vec![],
    NESTED_1_I8_EMPTY_0,
    NESTED_1_I8_EMPTY_0_INODES
);

value_match_nested_1_test!(
    value_match_nested_1_u16_empty_0,
    PNested1U16,
    vec![],
    NESTED_1_U16_EMPTY_0,
    NESTED_1_U16_EMPTY_0_INODES
);

value_match_nested_1_test!(
    value_match_nested_1_i16_empty_0,
    PNested1I16,
    vec![],
    NESTED_1_I16_EMPTY_0,
    NESTED_1_I16_EMPTY_0_INODES
);

value_match_nested_1_test!(
    value_match_nested_1_u32_empty_0,
    PNested1U32,
    vec![],
    NESTED_1_U32_EMPTY_0,
    NESTED_1_U32_EMPTY_0_INODES
);

value_match_nested_1_test!(
    value_match_nested_1_i32_empty_0,
    PNested1I32,
    vec![],
    NESTED_1_I32_EMPTY_0,
    NESTED_1_I32_EMPTY_0_INODES
);

value_match_nested_1_test!(
    value_match_nested_1_u64_empty_0,
    PNested1U64,
    vec![],
    NESTED_1_U64_EMPTY_0,
    NESTED_1_U64_EMPTY_0_INODES
);

value_match_nested_1_test!(
    value_match_nested_1_i64_empty_0,
    PNested1I64,
    vec![],
    NESTED_1_I64_EMPTY_0,
    NESTED_1_I64_EMPTY_0_INODES
);

value_match_nested_1_test!(
    value_match_nested_1_f16_empty_0,
    PNested1F16,
    vec![],
    NESTED_1_F16_EMPTY_0,
    NESTED_1_F16_EMPTY_0_INODES
);

value_match_nested_1_test!(
    value_match_nested_1_f32_empty_0,
    PNested1F32,
    vec![],
    NESTED_1_F32_EMPTY_0,
    NESTED_1_F32_EMPTY_0_INODES
);

value_match_nested_1_test!(
    value_match_nested_1_f64_empty_0,
    PNested1F64,
    vec![],
    NESTED_1_F64_EMPTY_0,
    NESTED_1_F64_EMPTY_0_INODES
);

value_match_nested_1_test!(
    value_match_nested_1_bool_empty_0,
    PNested1Bool,
    vec![],
    NESTED_1_BOOL_EMPTY_0,
    NESTED_1_BOOL_EMPTY_0_INODES
);

value_match_nested_1_test!(
    value_match_nested_1_bool_bitvec_empty_0,
    PNested1BitVec,
    bits(&[]),
    NESTED_1_BOOL_EMPTY_0,
    NESTED_1_BOOL_EMPTY_0_INODES
);

value_match_nested_1_test!(
    value_match_nested_1_timestamp_empty_0,
    PNested1Timestamp,
    vec![],
    NESTED_1_TIMESTAMP_EMPTY_0,
    NESTED_1_TIMESTAMP_EMPTY_0_INODES
);
