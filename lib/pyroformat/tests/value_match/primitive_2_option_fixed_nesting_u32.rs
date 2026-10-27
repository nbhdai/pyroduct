//! Struct-vs-value match for `src/wire/tests/primitive_2_option_fixed_nesting_u32.rs` —
//! `List<List<Option<[T; 2]>>>` against a derived struct with a native
//! `Vec<Vec<Option<[T; 2]>>>` field. See `tests/value_match/primitive_u32.rs`'s
//! module doc for the shared harness rationale.

use half::f16;

use pyroformat::wire::tests::primitive_2_option_fixed_nesting_u32::*;

use crate::support::bits::lsb_bools;
use crate::support::read_back::assert_roundtrip;
use pyroformat::value::Time;
use pyroformat::{Layout, Reader, Writer};

macro_rules! value_match_nested_2_option_fixed_test {
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
struct PNested2OptionFixedU8 {
    a: Vec<Vec<Option<[u8; 2]>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2OptionFixedI8 {
    a: Vec<Vec<Option<[i8; 2]>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2OptionFixedU16 {
    a: Vec<Vec<Option<[u16; 2]>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2OptionFixedI16 {
    a: Vec<Vec<Option<[i16; 2]>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2OptionFixedU32 {
    a: Vec<Vec<Option<[u32; 2]>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2OptionFixedI32 {
    a: Vec<Vec<Option<[i32; 2]>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2OptionFixedU64 {
    a: Vec<Vec<Option<[u64; 2]>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2OptionFixedI64 {
    a: Vec<Vec<Option<[i64; 2]>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2OptionFixedF16 {
    a: Vec<Vec<Option<[f16; 2]>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2OptionFixedF32 {
    a: Vec<Vec<Option<[f32; 2]>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2OptionFixedF64 {
    a: Vec<Vec<Option<[f64; 2]>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2OptionFixedTimestamp {
    a: Vec<Vec<Option<[Time; 2]>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2OptionFixedBool {
    a: Vec<Vec<Option<[bool; 2]>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2OptionFixedBool8 {
    a: Vec<Vec<Option<[bool; 8]>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2OptionFixedBool16 {
    a: Vec<Vec<Option<[bool; 16]>>>,
}

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_u8,
    PNested2OptionFixedU8,
    vec![
        vec![Some([1, 2]), None],
        vec![Some([5, 6])],
        vec![Some([7, 8]), None, Some([11, 12])]
    ],
    NESTED_2_OPTION_FIXED_U8,
    NESTED_2_OPTION_FIXED_U8_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_i8,
    PNested2OptionFixedI8,
    vec![
        vec![Some([-1, 2]), None],
        vec![Some([-5, 6])],
        vec![Some([-7, 8]), None, Some([-11, 12])]
    ],
    NESTED_2_OPTION_FIXED_I8,
    NESTED_2_OPTION_FIXED_I8_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_u16,
    PNested2OptionFixedU16,
    vec![
        vec![Some([1, 2]), None],
        vec![Some([5, 6])],
        vec![Some([7, 8]), None, Some([11, 12])]
    ],
    NESTED_2_OPTION_FIXED_U16,
    NESTED_2_OPTION_FIXED_U16_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_i16,
    PNested2OptionFixedI16,
    vec![
        vec![Some([-1, 2]), None],
        vec![Some([-5, 6])],
        vec![Some([-7, 8]), None, Some([-11, 12])]
    ],
    NESTED_2_OPTION_FIXED_I16,
    NESTED_2_OPTION_FIXED_I16_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_u32,
    PNested2OptionFixedU32,
    vec![
        vec![Some([1, 2]), None],
        vec![Some([5, 6])],
        vec![Some([7, 8]), None, Some([11, 12])]
    ],
    NESTED_2_OPTION_FIXED_U32,
    NESTED_2_OPTION_FIXED_U32_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_i32,
    PNested2OptionFixedI32,
    vec![
        vec![Some([-1, 2]), None],
        vec![Some([-5, 6])],
        vec![Some([-7, 8]), None, Some([-11, 12])]
    ],
    NESTED_2_OPTION_FIXED_I32,
    NESTED_2_OPTION_FIXED_I32_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_u64,
    PNested2OptionFixedU64,
    vec![
        vec![Some([1, 2]), None],
        vec![Some([5, 6])],
        vec![Some([7, 8]), None, Some([11, 12])]
    ],
    NESTED_2_OPTION_FIXED_U64,
    NESTED_2_OPTION_FIXED_U64_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_i64,
    PNested2OptionFixedI64,
    vec![
        vec![Some([-1, 2]), None],
        vec![Some([-5, 6])],
        vec![Some([-7, 8]), None, Some([-11, 12])]
    ],
    NESTED_2_OPTION_FIXED_I64,
    NESTED_2_OPTION_FIXED_I64_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_f16,
    PNested2OptionFixedF16,
    vec![
        vec![Some([f16::from_f32(1.5), f16::from_f32(-2.0)]), None],
        vec![Some([f16::from_f32(-5.5), f16::from_f32(6.0)])],
        vec![
            Some([f16::from_f32(7.5), f16::from_f32(-8.0)]),
            None,
            Some([f16::from_f32(-11.5), f16::from_f32(12.0)])
        ]
    ],
    NESTED_2_OPTION_FIXED_F16,
    NESTED_2_OPTION_FIXED_F16_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_f32,
    PNested2OptionFixedF32,
    vec![
        vec![Some([1.5, -2.0]), None],
        vec![Some([-5.5, 6.0])],
        vec![Some([7.5, -8.0]), None, Some([-11.5, 12.0])]
    ],
    NESTED_2_OPTION_FIXED_F32,
    NESTED_2_OPTION_FIXED_F32_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_f64,
    PNested2OptionFixedF64,
    vec![
        vec![Some([1.5, -2.0]), None],
        vec![Some([-5.5, 6.0])],
        vec![Some([7.5, -8.0]), None, Some([-11.5, 12.0])]
    ],
    NESTED_2_OPTION_FIXED_F64,
    NESTED_2_OPTION_FIXED_F64_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_bool,
    PNested2OptionFixedBool,
    vec![
        vec![Some([true, false]), None],
        vec![Some([true, false])],
        vec![Some([false, true]), None, Some([false, false])]
    ],
    NESTED_2_OPTION_FIXED_BOOL,
    NESTED_2_OPTION_FIXED_BOOL_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_bool_8,
    PNested2OptionFixedBool8,
    vec![
        vec![Some(lsb_bools::<8>(0x01)), None],
        vec![Some(lsb_bools::<8>(0x80))],
        vec![Some(lsb_bools::<8>(0xFF)), None, Some(lsb_bools::<8>(0x00))]
    ],
    NESTED_2_OPTION_FIXED_BOOL_8,
    NESTED_2_OPTION_FIXED_BOOL_8_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_bool_16,
    PNested2OptionFixedBool16,
    vec![
        vec![Some(lsb_bools::<16>(0x0001)), None],
        vec![Some(lsb_bools::<16>(0x8000))],
        vec![
            Some(lsb_bools::<16>(0xFFFF)),
            None,
            Some(lsb_bools::<16>(0x0000))
        ]
    ],
    NESTED_2_OPTION_FIXED_BOOL_16,
    NESTED_2_OPTION_FIXED_BOOL_16_INODES
);

// ---------------------------------------------------------------------------
// Empty lists at every depth (`_EMPTY_d`: the list at depth `d` is empty)
// ---------------------------------------------------------------------------
value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_u8_empty_0,
    PNested2OptionFixedU8,
    vec![],
    NESTED_2_OPTION_FIXED_U8_EMPTY_0,
    NESTED_2_OPTION_FIXED_U8_EMPTY_0_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_u8_empty_1,
    PNested2OptionFixedU8,
    vec![vec![]],
    NESTED_2_OPTION_FIXED_U8_EMPTY_1,
    NESTED_2_OPTION_FIXED_U8_EMPTY_1_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_i8_empty_0,
    PNested2OptionFixedI8,
    vec![],
    NESTED_2_OPTION_FIXED_I8_EMPTY_0,
    NESTED_2_OPTION_FIXED_I8_EMPTY_0_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_i8_empty_1,
    PNested2OptionFixedI8,
    vec![vec![]],
    NESTED_2_OPTION_FIXED_I8_EMPTY_1,
    NESTED_2_OPTION_FIXED_I8_EMPTY_1_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_u16_empty_0,
    PNested2OptionFixedU16,
    vec![],
    NESTED_2_OPTION_FIXED_U16_EMPTY_0,
    NESTED_2_OPTION_FIXED_U16_EMPTY_0_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_u16_empty_1,
    PNested2OptionFixedU16,
    vec![vec![]],
    NESTED_2_OPTION_FIXED_U16_EMPTY_1,
    NESTED_2_OPTION_FIXED_U16_EMPTY_1_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_i16_empty_0,
    PNested2OptionFixedI16,
    vec![],
    NESTED_2_OPTION_FIXED_I16_EMPTY_0,
    NESTED_2_OPTION_FIXED_I16_EMPTY_0_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_i16_empty_1,
    PNested2OptionFixedI16,
    vec![vec![]],
    NESTED_2_OPTION_FIXED_I16_EMPTY_1,
    NESTED_2_OPTION_FIXED_I16_EMPTY_1_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_u32_empty_0,
    PNested2OptionFixedU32,
    vec![],
    NESTED_2_OPTION_FIXED_U32_EMPTY_0,
    NESTED_2_OPTION_FIXED_U32_EMPTY_0_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_u32_empty_1,
    PNested2OptionFixedU32,
    vec![vec![]],
    NESTED_2_OPTION_FIXED_U32_EMPTY_1,
    NESTED_2_OPTION_FIXED_U32_EMPTY_1_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_i32_empty_0,
    PNested2OptionFixedI32,
    vec![],
    NESTED_2_OPTION_FIXED_I32_EMPTY_0,
    NESTED_2_OPTION_FIXED_I32_EMPTY_0_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_i32_empty_1,
    PNested2OptionFixedI32,
    vec![vec![]],
    NESTED_2_OPTION_FIXED_I32_EMPTY_1,
    NESTED_2_OPTION_FIXED_I32_EMPTY_1_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_u64_empty_0,
    PNested2OptionFixedU64,
    vec![],
    NESTED_2_OPTION_FIXED_U64_EMPTY_0,
    NESTED_2_OPTION_FIXED_U64_EMPTY_0_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_u64_empty_1,
    PNested2OptionFixedU64,
    vec![vec![]],
    NESTED_2_OPTION_FIXED_U64_EMPTY_1,
    NESTED_2_OPTION_FIXED_U64_EMPTY_1_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_i64_empty_0,
    PNested2OptionFixedI64,
    vec![],
    NESTED_2_OPTION_FIXED_I64_EMPTY_0,
    NESTED_2_OPTION_FIXED_I64_EMPTY_0_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_i64_empty_1,
    PNested2OptionFixedI64,
    vec![vec![]],
    NESTED_2_OPTION_FIXED_I64_EMPTY_1,
    NESTED_2_OPTION_FIXED_I64_EMPTY_1_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_f16_empty_0,
    PNested2OptionFixedF16,
    vec![],
    NESTED_2_OPTION_FIXED_F16_EMPTY_0,
    NESTED_2_OPTION_FIXED_F16_EMPTY_0_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_f16_empty_1,
    PNested2OptionFixedF16,
    vec![vec![]],
    NESTED_2_OPTION_FIXED_F16_EMPTY_1,
    NESTED_2_OPTION_FIXED_F16_EMPTY_1_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_f32_empty_0,
    PNested2OptionFixedF32,
    vec![],
    NESTED_2_OPTION_FIXED_F32_EMPTY_0,
    NESTED_2_OPTION_FIXED_F32_EMPTY_0_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_f32_empty_1,
    PNested2OptionFixedF32,
    vec![vec![]],
    NESTED_2_OPTION_FIXED_F32_EMPTY_1,
    NESTED_2_OPTION_FIXED_F32_EMPTY_1_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_f64_empty_0,
    PNested2OptionFixedF64,
    vec![],
    NESTED_2_OPTION_FIXED_F64_EMPTY_0,
    NESTED_2_OPTION_FIXED_F64_EMPTY_0_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_f64_empty_1,
    PNested2OptionFixedF64,
    vec![vec![]],
    NESTED_2_OPTION_FIXED_F64_EMPTY_1,
    NESTED_2_OPTION_FIXED_F64_EMPTY_1_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_bool_empty_0,
    PNested2OptionFixedBool,
    vec![],
    NESTED_2_OPTION_FIXED_BOOL_EMPTY_0,
    NESTED_2_OPTION_FIXED_BOOL_EMPTY_0_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_bool_empty_1,
    PNested2OptionFixedBool,
    vec![vec![]],
    NESTED_2_OPTION_FIXED_BOOL_EMPTY_1,
    NESTED_2_OPTION_FIXED_BOOL_EMPTY_1_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_bool_8_empty_0,
    PNested2OptionFixedBool8,
    vec![],
    NESTED_2_OPTION_FIXED_BOOL_8_EMPTY_0,
    NESTED_2_OPTION_FIXED_BOOL_8_EMPTY_0_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_bool_8_empty_1,
    PNested2OptionFixedBool8,
    vec![vec![]],
    NESTED_2_OPTION_FIXED_BOOL_8_EMPTY_1,
    NESTED_2_OPTION_FIXED_BOOL_8_EMPTY_1_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_bool_16_empty_0,
    PNested2OptionFixedBool16,
    vec![],
    NESTED_2_OPTION_FIXED_BOOL_16_EMPTY_0,
    NESTED_2_OPTION_FIXED_BOOL_16_EMPTY_0_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_bool_16_empty_1,
    PNested2OptionFixedBool16,
    vec![vec![]],
    NESTED_2_OPTION_FIXED_BOOL_16_EMPTY_1,
    NESTED_2_OPTION_FIXED_BOOL_16_EMPTY_1_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_timestamp,
    PNested2OptionFixedTimestamp,
    vec![
        vec![Some([Time(1_500_000_000), Time(-2_000_000_000)]), None],
        vec![Some([Time(-5_500_000_000), Time(6_000_000_000)])],
        vec![
            Some([Time(7_500_000_000), Time(-8_000_000_000)]),
            None,
            Some([Time(-11_500_000_000), Time(12_000_000_000)])
        ]
    ],
    NESTED_2_OPTION_FIXED_TIMESTAMP,
    NESTED_2_OPTION_FIXED_TIMESTAMP_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_timestamp_empty_0,
    PNested2OptionFixedTimestamp,
    vec![],
    NESTED_2_OPTION_FIXED_TIMESTAMP_EMPTY_0,
    NESTED_2_OPTION_FIXED_TIMESTAMP_EMPTY_0_INODES
);

value_match_nested_2_option_fixed_test!(
    value_match_nested_2_option_fixed_timestamp_empty_1,
    PNested2OptionFixedTimestamp,
    vec![vec![]],
    NESTED_2_OPTION_FIXED_TIMESTAMP_EMPTY_1,
    NESTED_2_OPTION_FIXED_TIMESTAMP_EMPTY_1_INODES
);
