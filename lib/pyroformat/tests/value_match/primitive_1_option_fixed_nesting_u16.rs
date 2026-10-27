//! Struct-vs-value match for `src/wire/tests/primitive_1_option_fixed_nesting_u16.rs` —
//! `List<Option<[T; 2]>>` against a derived struct with a native
//! `Vec<Option<[T; 2]>>` field. See `tests/value_match/primitive_u16.rs`'s
//! module doc for the shared harness rationale.

use half::f16;

use pyroformat::wire::tests::primitive_1_option_fixed_nesting_u16::*;

use crate::support::bits::lsb_bools;
use crate::support::read_back::assert_roundtrip;
use pyroformat::value::Time;
use pyroformat::{Layout, Reader, Writer};

macro_rules! value_match_nested_1_option_fixed_test {
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
struct PNested1OptionFixedU8 {
    a: Vec<Option<[u8; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionFixedI8 {
    a: Vec<Option<[i8; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionFixedU16 {
    a: Vec<Option<[u16; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionFixedI16 {
    a: Vec<Option<[i16; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionFixedU32 {
    a: Vec<Option<[u32; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionFixedI32 {
    a: Vec<Option<[i32; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionFixedU64 {
    a: Vec<Option<[u64; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionFixedI64 {
    a: Vec<Option<[i64; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionFixedF16 {
    a: Vec<Option<[f16; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionFixedF32 {
    a: Vec<Option<[f32; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionFixedF64 {
    a: Vec<Option<[f64; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionFixedTimestamp {
    a: Vec<Option<[Time; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionFixedBool {
    a: Vec<Option<[bool; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionFixedBool8 {
    a: Vec<Option<[bool; 8]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionFixedBool16 {
    a: Vec<Option<[bool; 16]>>,
}

value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_u8,
    PNested1OptionFixedU8,
    vec![
        Some([1, 2]),
        None,
        Some([5, 6]),
        Some([7, 8]),
        None,
        Some([11, 12])
    ],
    NESTED_1_OPTION_FIXED_U8,
    NESTED_1_OPTION_FIXED_U8_INODES
);

value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_i8,
    PNested1OptionFixedI8,
    vec![
        Some([-1, 2]),
        None,
        Some([-5, 6]),
        Some([-7, 8]),
        None,
        Some([-11, 12])
    ],
    NESTED_1_OPTION_FIXED_I8,
    NESTED_1_OPTION_FIXED_I8_INODES
);

value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_u16,
    PNested1OptionFixedU16,
    vec![
        Some([1, 2]),
        None,
        Some([5, 6]),
        Some([7, 8]),
        None,
        Some([11, 12])
    ],
    NESTED_1_OPTION_FIXED_U16,
    NESTED_1_OPTION_FIXED_U16_INODES
);

value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_i16,
    PNested1OptionFixedI16,
    vec![
        Some([-1, 2]),
        None,
        Some([-5, 6]),
        Some([-7, 8]),
        None,
        Some([-11, 12])
    ],
    NESTED_1_OPTION_FIXED_I16,
    NESTED_1_OPTION_FIXED_I16_INODES
);

value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_u32,
    PNested1OptionFixedU32,
    vec![
        Some([1, 2]),
        None,
        Some([5, 6]),
        Some([7, 8]),
        None,
        Some([11, 12])
    ],
    NESTED_1_OPTION_FIXED_U32,
    NESTED_1_OPTION_FIXED_U32_INODES
);

value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_i32,
    PNested1OptionFixedI32,
    vec![
        Some([-1, 2]),
        None,
        Some([-5, 6]),
        Some([-7, 8]),
        None,
        Some([-11, 12])
    ],
    NESTED_1_OPTION_FIXED_I32,
    NESTED_1_OPTION_FIXED_I32_INODES
);

value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_u64,
    PNested1OptionFixedU64,
    vec![
        Some([1, 2]),
        None,
        Some([5, 6]),
        Some([7, 8]),
        None,
        Some([11, 12])
    ],
    NESTED_1_OPTION_FIXED_U64,
    NESTED_1_OPTION_FIXED_U64_INODES
);

value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_i64,
    PNested1OptionFixedI64,
    vec![
        Some([-1, 2]),
        None,
        Some([-5, 6]),
        Some([-7, 8]),
        None,
        Some([-11, 12])
    ],
    NESTED_1_OPTION_FIXED_I64,
    NESTED_1_OPTION_FIXED_I64_INODES
);

value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_f16,
    PNested1OptionFixedF16,
    vec![
        Some([f16::from_f32(1.5), f16::from_f32(-2.0)]),
        None,
        Some([f16::from_f32(-5.5), f16::from_f32(6.0)]),
        Some([f16::from_f32(7.5), f16::from_f32(-8.0)]),
        None,
        Some([f16::from_f32(-11.5), f16::from_f32(12.0)])
    ],
    NESTED_1_OPTION_FIXED_F16,
    NESTED_1_OPTION_FIXED_F16_INODES
);

value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_f32,
    PNested1OptionFixedF32,
    vec![
        Some([1.5, -2.0]),
        None,
        Some([-5.5, 6.0]),
        Some([7.5, -8.0]),
        None,
        Some([-11.5, 12.0])
    ],
    NESTED_1_OPTION_FIXED_F32,
    NESTED_1_OPTION_FIXED_F32_INODES
);

value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_f64,
    PNested1OptionFixedF64,
    vec![
        Some([1.5, -2.0]),
        None,
        Some([-5.5, 6.0]),
        Some([7.5, -8.0]),
        None,
        Some([-11.5, 12.0])
    ],
    NESTED_1_OPTION_FIXED_F64,
    NESTED_1_OPTION_FIXED_F64_INODES
);

value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_bool,
    PNested1OptionFixedBool,
    vec![
        Some([true, false]),
        None,
        Some([true, false]),
        Some([false, true]),
        None,
        Some([false, false])
    ],
    NESTED_1_OPTION_FIXED_BOOL,
    NESTED_1_OPTION_FIXED_BOOL_INODES
);

value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_bool_8,
    PNested1OptionFixedBool8,
    vec![
        Some(lsb_bools::<8>(0x01)),
        None,
        Some(lsb_bools::<8>(0x80)),
        Some(lsb_bools::<8>(0xFF)),
        None,
        Some(lsb_bools::<8>(0x00))
    ],
    NESTED_1_OPTION_FIXED_BOOL_8,
    NESTED_1_OPTION_FIXED_BOOL_8_INODES
);

value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_bool_16,
    PNested1OptionFixedBool16,
    vec![
        Some(lsb_bools::<16>(0x0001)),
        None,
        Some(lsb_bools::<16>(0x8000)),
        Some(lsb_bools::<16>(0xFFFF)),
        None,
        Some(lsb_bools::<16>(0x0000))
    ],
    NESTED_1_OPTION_FIXED_BOOL_16,
    NESTED_1_OPTION_FIXED_BOOL_16_INODES
);

// ---------------------------------------------------------------------------
// Empty lists at every depth (`_EMPTY_d`: the list at depth `d` is empty)
// ---------------------------------------------------------------------------
value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_u8_empty_0,
    PNested1OptionFixedU8,
    vec![],
    NESTED_1_OPTION_FIXED_U8_EMPTY_0,
    NESTED_1_OPTION_FIXED_U8_EMPTY_0_INODES
);

value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_i8_empty_0,
    PNested1OptionFixedI8,
    vec![],
    NESTED_1_OPTION_FIXED_I8_EMPTY_0,
    NESTED_1_OPTION_FIXED_I8_EMPTY_0_INODES
);

value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_u16_empty_0,
    PNested1OptionFixedU16,
    vec![],
    NESTED_1_OPTION_FIXED_U16_EMPTY_0,
    NESTED_1_OPTION_FIXED_U16_EMPTY_0_INODES
);

value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_i16_empty_0,
    PNested1OptionFixedI16,
    vec![],
    NESTED_1_OPTION_FIXED_I16_EMPTY_0,
    NESTED_1_OPTION_FIXED_I16_EMPTY_0_INODES
);

value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_u32_empty_0,
    PNested1OptionFixedU32,
    vec![],
    NESTED_1_OPTION_FIXED_U32_EMPTY_0,
    NESTED_1_OPTION_FIXED_U32_EMPTY_0_INODES
);

value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_i32_empty_0,
    PNested1OptionFixedI32,
    vec![],
    NESTED_1_OPTION_FIXED_I32_EMPTY_0,
    NESTED_1_OPTION_FIXED_I32_EMPTY_0_INODES
);

value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_u64_empty_0,
    PNested1OptionFixedU64,
    vec![],
    NESTED_1_OPTION_FIXED_U64_EMPTY_0,
    NESTED_1_OPTION_FIXED_U64_EMPTY_0_INODES
);

value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_i64_empty_0,
    PNested1OptionFixedI64,
    vec![],
    NESTED_1_OPTION_FIXED_I64_EMPTY_0,
    NESTED_1_OPTION_FIXED_I64_EMPTY_0_INODES
);

value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_f16_empty_0,
    PNested1OptionFixedF16,
    vec![],
    NESTED_1_OPTION_FIXED_F16_EMPTY_0,
    NESTED_1_OPTION_FIXED_F16_EMPTY_0_INODES
);

value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_f32_empty_0,
    PNested1OptionFixedF32,
    vec![],
    NESTED_1_OPTION_FIXED_F32_EMPTY_0,
    NESTED_1_OPTION_FIXED_F32_EMPTY_0_INODES
);

value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_f64_empty_0,
    PNested1OptionFixedF64,
    vec![],
    NESTED_1_OPTION_FIXED_F64_EMPTY_0,
    NESTED_1_OPTION_FIXED_F64_EMPTY_0_INODES
);

value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_bool_empty_0,
    PNested1OptionFixedBool,
    vec![],
    NESTED_1_OPTION_FIXED_BOOL_EMPTY_0,
    NESTED_1_OPTION_FIXED_BOOL_EMPTY_0_INODES
);

value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_bool_8_empty_0,
    PNested1OptionFixedBool8,
    vec![],
    NESTED_1_OPTION_FIXED_BOOL_8_EMPTY_0,
    NESTED_1_OPTION_FIXED_BOOL_8_EMPTY_0_INODES
);

value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_bool_16_empty_0,
    PNested1OptionFixedBool16,
    vec![],
    NESTED_1_OPTION_FIXED_BOOL_16_EMPTY_0,
    NESTED_1_OPTION_FIXED_BOOL_16_EMPTY_0_INODES
);

value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_timestamp,
    PNested1OptionFixedTimestamp,
    vec![
        Some([Time(1_500_000_000), Time(-2_000_000_000)]),
        None,
        Some([Time(-5_500_000_000), Time(6_000_000_000)]),
        Some([Time(7_500_000_000), Time(-8_000_000_000)]),
        None,
        Some([Time(-11_500_000_000), Time(12_000_000_000)])
    ],
    NESTED_1_OPTION_FIXED_TIMESTAMP,
    NESTED_1_OPTION_FIXED_TIMESTAMP_INODES
);

value_match_nested_1_option_fixed_test!(
    value_match_nested_1_option_fixed_timestamp_empty_0,
    PNested1OptionFixedTimestamp,
    vec![],
    NESTED_1_OPTION_FIXED_TIMESTAMP_EMPTY_0,
    NESTED_1_OPTION_FIXED_TIMESTAMP_EMPTY_0_INODES
);
