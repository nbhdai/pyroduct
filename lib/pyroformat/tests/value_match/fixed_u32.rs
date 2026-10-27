//! Struct-vs-value match for `src/wire/tests/fixed_u32.rs` — fixed-size
//! primitive arrays (`[T; N]`/`Option<[T; N]>`) against a derived struct
//! with a native Rust array field. See `tests/value_match/primitive_u32.rs`'s
//! module doc for the shared harness rationale.

use pyroformat::wire::tests::fixed_u32::*;

use crate::support::read_back::assert_roundtrip;
use pyroformat::value::Time;
use pyroformat::{Layout, Reader, Writer};

macro_rules! value_match_fixed_test {
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
struct PFixedU32x4 {
    a: [u32; 4],
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PFixedF64x2 {
    a: [f64; 2],
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct POptionFixedF32x3 {
    a: Option<[f32; 3]>,
}

value_match_fixed_test!(
    value_match_fixed_u32,
    PFixedU32x4,
    [10u32, 20, 30, 40],
    FIXED_U32_4,
    FIXED_U32_4_INODES
);

value_match_fixed_test!(
    value_match_fixed_f64,
    PFixedF64x2,
    [1.0f64, 2.5],
    FIXED_F64_2,
    FIXED_F64_2_INODES
);

value_match_fixed_test!(
    value_match_option_fixed_f32_some,
    POptionFixedF32x3,
    Some([1.0f32, 2.0, 3.0]),
    OPTION_FIXED_F32_SOME,
    OPTION_FIXED_F32_INODES
);

value_match_fixed_test!(
    value_match_option_fixed_f32_none,
    POptionFixedF32x3,
    None,
    OPTION_FIXED_F32_NONE,
    OPTION_FIXED_F32_INODES
);

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PFixedTimestampx2 {
    a: [Time; 2],
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct POptionFixedTimestampx2 {
    a: Option<[Time; 2]>,
}

value_match_fixed_test!(
    value_match_fixed_timestamp,
    PFixedTimestampx2,
    [Time(1_000_000_001), Time(-2_000_000_002)],
    FIXED_TIMESTAMP_2,
    FIXED_TIMESTAMP_2_INODES
);

value_match_fixed_test!(
    value_match_option_fixed_timestamp_some,
    POptionFixedTimestampx2,
    Some([Time(1_000_000_001), Time(-2_000_000_002)]),
    OPTION_FIXED_TIMESTAMP_SOME,
    OPTION_FIXED_TIMESTAMP_INODES
);

value_match_fixed_test!(
    value_match_option_fixed_timestamp_none,
    POptionFixedTimestampx2,
    None,
    OPTION_FIXED_TIMESTAMP_NONE,
    OPTION_FIXED_TIMESTAMP_INODES
);
