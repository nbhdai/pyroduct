//! Struct-vs-value match for `src/wire/tests/primitive_1_fixed_nesting_u16.rs` —
//! `List<[T; 2]>` against a derived struct with a native
//! `Vec<[T; 2]>` field. See `tests/value_match/primitive_u16.rs`'s
//! module doc for the shared harness rationale.

use half::f16;

use pyroformat::wire::tests::primitive_1_fixed_nesting_u16::*;

use crate::support::bits::lsb_bools;
use crate::support::read_back::assert_roundtrip;
use pyroformat::value::Time;
use pyroformat::{Layout, Reader, Writer};

macro_rules! value_match_nested_1_fixed_test {
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
struct PNested1FixedU8 {
    a: Vec<[u8; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1FixedI8 {
    a: Vec<[i8; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1FixedU16 {
    a: Vec<[u16; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1FixedI16 {
    a: Vec<[i16; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1FixedU32 {
    a: Vec<[u32; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1FixedI32 {
    a: Vec<[i32; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1FixedU64 {
    a: Vec<[u64; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1FixedI64 {
    a: Vec<[i64; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1FixedF16 {
    a: Vec<[f16; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1FixedF32 {
    a: Vec<[f32; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1FixedF64 {
    a: Vec<[f64; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1FixedTimestamp {
    a: Vec<[Time; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1FixedBool {
    a: Vec<[bool; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1FixedBool8 {
    a: Vec<[bool; 8]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1FixedBool16 {
    a: Vec<[bool; 16]>,
}

value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_u8,
    PNested1FixedU8,
    vec![[1, 2], [3, 4], [5, 6], [7, 8], [9, 10], [11, 12]],
    NESTED_1_FIXED_U8,
    NESTED_1_FIXED_U8_INODES
);

value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_i8,
    PNested1FixedI8,
    vec![[-1, 2], [-3, 4], [-5, 6], [-7, 8], [-9, 10], [-11, 12]],
    NESTED_1_FIXED_I8,
    NESTED_1_FIXED_I8_INODES
);

value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_u16,
    PNested1FixedU16,
    vec![[1, 2], [3, 4], [5, 6], [7, 8], [9, 10], [11, 12]],
    NESTED_1_FIXED_U16,
    NESTED_1_FIXED_U16_INODES
);

value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_i16,
    PNested1FixedI16,
    vec![[-1, 2], [-3, 4], [-5, 6], [-7, 8], [-9, 10], [-11, 12]],
    NESTED_1_FIXED_I16,
    NESTED_1_FIXED_I16_INODES
);

value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_u32,
    PNested1FixedU32,
    vec![[1, 2], [3, 4], [5, 6], [7, 8], [9, 10], [11, 12]],
    NESTED_1_FIXED_U32,
    NESTED_1_FIXED_U32_INODES
);

value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_i32,
    PNested1FixedI32,
    vec![[-1, 2], [-3, 4], [-5, 6], [-7, 8], [-9, 10], [-11, 12]],
    NESTED_1_FIXED_I32,
    NESTED_1_FIXED_I32_INODES
);

value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_u64,
    PNested1FixedU64,
    vec![[1, 2], [3, 4], [5, 6], [7, 8], [9, 10], [11, 12]],
    NESTED_1_FIXED_U64,
    NESTED_1_FIXED_U64_INODES
);

value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_i64,
    PNested1FixedI64,
    vec![[-1, 2], [-3, 4], [-5, 6], [-7, 8], [-9, 10], [-11, 12]],
    NESTED_1_FIXED_I64,
    NESTED_1_FIXED_I64_INODES
);

value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_f16,
    PNested1FixedF16,
    vec![
        [f16::from_f32(1.5), f16::from_f32(-2.0)],
        [f16::from_f32(3.25), f16::from_f32(4.0)],
        [f16::from_f32(-5.5), f16::from_f32(6.0)],
        [f16::from_f32(7.5), f16::from_f32(-8.0)],
        [f16::from_f32(9.25), f16::from_f32(10.0)],
        [f16::from_f32(-11.5), f16::from_f32(12.0)]
    ],
    NESTED_1_FIXED_F16,
    NESTED_1_FIXED_F16_INODES
);

value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_f32,
    PNested1FixedF32,
    vec![
        [1.5, -2.0],
        [3.25, 4.0],
        [-5.5, 6.0],
        [7.5, -8.0],
        [9.25, 10.0],
        [-11.5, 12.0]
    ],
    NESTED_1_FIXED_F32,
    NESTED_1_FIXED_F32_INODES
);

value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_f64,
    PNested1FixedF64,
    vec![
        [1.5, -2.0],
        [3.25, 4.0],
        [-5.5, 6.0],
        [7.5, -8.0],
        [9.25, 10.0],
        [-11.5, 12.0]
    ],
    NESTED_1_FIXED_F64,
    NESTED_1_FIXED_F64_INODES
);

value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_bool,
    PNested1FixedBool,
    vec![
        [true, false],
        [true, true],
        [true, false],
        [false, true],
        [true, false],
        [false, false]
    ],
    NESTED_1_FIXED_BOOL,
    NESTED_1_FIXED_BOOL_INODES
);

value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_bool_8,
    PNested1FixedBool8,
    vec![
        lsb_bools::<8>(0x01),
        lsb_bools::<8>(0x80),
        lsb_bools::<8>(0xFF),
        lsb_bools::<8>(0x00),
        lsb_bools::<8>(0xA5),
        lsb_bools::<8>(0x3C)
    ],
    NESTED_1_FIXED_BOOL_8,
    NESTED_1_FIXED_BOOL_8_INODES
);

value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_bool_16,
    PNested1FixedBool16,
    vec![
        lsb_bools::<16>(0x0001),
        lsb_bools::<16>(0x8000),
        lsb_bools::<16>(0xFFFF),
        lsb_bools::<16>(0x0000),
        lsb_bools::<16>(0xA55A),
        lsb_bools::<16>(0x3CC3)
    ],
    NESTED_1_FIXED_BOOL_16,
    NESTED_1_FIXED_BOOL_16_INODES
);

// ---------------------------------------------------------------------------
// Empty lists at every depth (`_EMPTY_d`: the list at depth `d` is empty)
// ---------------------------------------------------------------------------
value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_u8_empty_0,
    PNested1FixedU8,
    vec![],
    NESTED_1_FIXED_U8_EMPTY_0,
    NESTED_1_FIXED_U8_EMPTY_0_INODES
);

value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_i8_empty_0,
    PNested1FixedI8,
    vec![],
    NESTED_1_FIXED_I8_EMPTY_0,
    NESTED_1_FIXED_I8_EMPTY_0_INODES
);

value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_u16_empty_0,
    PNested1FixedU16,
    vec![],
    NESTED_1_FIXED_U16_EMPTY_0,
    NESTED_1_FIXED_U16_EMPTY_0_INODES
);

value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_i16_empty_0,
    PNested1FixedI16,
    vec![],
    NESTED_1_FIXED_I16_EMPTY_0,
    NESTED_1_FIXED_I16_EMPTY_0_INODES
);

value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_u32_empty_0,
    PNested1FixedU32,
    vec![],
    NESTED_1_FIXED_U32_EMPTY_0,
    NESTED_1_FIXED_U32_EMPTY_0_INODES
);

value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_i32_empty_0,
    PNested1FixedI32,
    vec![],
    NESTED_1_FIXED_I32_EMPTY_0,
    NESTED_1_FIXED_I32_EMPTY_0_INODES
);

value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_u64_empty_0,
    PNested1FixedU64,
    vec![],
    NESTED_1_FIXED_U64_EMPTY_0,
    NESTED_1_FIXED_U64_EMPTY_0_INODES
);

value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_i64_empty_0,
    PNested1FixedI64,
    vec![],
    NESTED_1_FIXED_I64_EMPTY_0,
    NESTED_1_FIXED_I64_EMPTY_0_INODES
);

value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_f16_empty_0,
    PNested1FixedF16,
    vec![],
    NESTED_1_FIXED_F16_EMPTY_0,
    NESTED_1_FIXED_F16_EMPTY_0_INODES
);

value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_f32_empty_0,
    PNested1FixedF32,
    vec![],
    NESTED_1_FIXED_F32_EMPTY_0,
    NESTED_1_FIXED_F32_EMPTY_0_INODES
);

value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_f64_empty_0,
    PNested1FixedF64,
    vec![],
    NESTED_1_FIXED_F64_EMPTY_0,
    NESTED_1_FIXED_F64_EMPTY_0_INODES
);

value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_bool_empty_0,
    PNested1FixedBool,
    vec![],
    NESTED_1_FIXED_BOOL_EMPTY_0,
    NESTED_1_FIXED_BOOL_EMPTY_0_INODES
);

value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_bool_8_empty_0,
    PNested1FixedBool8,
    vec![],
    NESTED_1_FIXED_BOOL_8_EMPTY_0,
    NESTED_1_FIXED_BOOL_8_EMPTY_0_INODES
);

value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_bool_16_empty_0,
    PNested1FixedBool16,
    vec![],
    NESTED_1_FIXED_BOOL_16_EMPTY_0,
    NESTED_1_FIXED_BOOL_16_EMPTY_0_INODES
);

value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_timestamp,
    PNested1FixedTimestamp,
    vec![
        [Time(1_500_000_000), Time(-2_000_000_000)],
        [Time(3_250_000_000), Time(4_000_000_000)],
        [Time(-5_500_000_000), Time(6_000_000_000)],
        [Time(7_500_000_000), Time(-8_000_000_000)],
        [Time(9_250_000_000), Time(10_000_000_000)],
        [Time(-11_500_000_000), Time(12_000_000_000)]
    ],
    NESTED_1_FIXED_TIMESTAMP,
    NESTED_1_FIXED_TIMESTAMP_INODES
);

value_match_nested_1_fixed_test!(
    value_match_nested_1_fixed_timestamp_empty_0,
    PNested1FixedTimestamp,
    vec![],
    NESTED_1_FIXED_TIMESTAMP_EMPTY_0,
    NESTED_1_FIXED_TIMESTAMP_EMPTY_0_INODES
);
