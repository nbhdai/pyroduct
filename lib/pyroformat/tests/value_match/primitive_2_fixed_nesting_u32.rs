//! Struct-vs-value match for `src/wire/tests/primitive_2_fixed_nesting_u32.rs` —
//! `List<List<[T; 2]>>` against a derived struct with a native
//! `Vec<Vec<[T; 2]>>` field. See `tests/value_match/primitive_u32.rs`'s
//! module doc for the shared harness rationale.

use half::f16;

use pyroformat::wire::tests::primitive_2_fixed_nesting_u32::*;

use crate::support::bits::lsb_bools;
use crate::support::read_back::assert_roundtrip;
use pyroformat::value::Time;
use pyroformat::{Layout, Reader, Writer};

macro_rules! value_match_nested_2_fixed_test {
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
struct PNested2FixedU8 {
    a: Vec<Vec<[u8; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2FixedI8 {
    a: Vec<Vec<[i8; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2FixedU16 {
    a: Vec<Vec<[u16; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2FixedI16 {
    a: Vec<Vec<[i16; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2FixedU32 {
    a: Vec<Vec<[u32; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2FixedI32 {
    a: Vec<Vec<[i32; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2FixedU64 {
    a: Vec<Vec<[u64; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2FixedI64 {
    a: Vec<Vec<[i64; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2FixedF16 {
    a: Vec<Vec<[f16; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2FixedF32 {
    a: Vec<Vec<[f32; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2FixedF64 {
    a: Vec<Vec<[f64; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2FixedTimestamp {
    a: Vec<Vec<[Time; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2FixedBool {
    a: Vec<Vec<[bool; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2FixedBool8 {
    a: Vec<Vec<[bool; 8]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2FixedBool16 {
    a: Vec<Vec<[bool; 16]>>,
}

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_u8,
    PNested2FixedU8,
    vec![
        vec![[1, 2], [3, 4]],
        vec![[5, 6]],
        vec![[7, 8], [9, 10], [11, 12]]
    ],
    NESTED_2_FIXED_U8,
    NESTED_2_FIXED_U8_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_i8,
    PNested2FixedI8,
    vec![
        vec![[-1, 2], [-3, 4]],
        vec![[-5, 6]],
        vec![[-7, 8], [-9, 10], [-11, 12]]
    ],
    NESTED_2_FIXED_I8,
    NESTED_2_FIXED_I8_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_u16,
    PNested2FixedU16,
    vec![
        vec![[1, 2], [3, 4]],
        vec![[5, 6]],
        vec![[7, 8], [9, 10], [11, 12]]
    ],
    NESTED_2_FIXED_U16,
    NESTED_2_FIXED_U16_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_i16,
    PNested2FixedI16,
    vec![
        vec![[-1, 2], [-3, 4]],
        vec![[-5, 6]],
        vec![[-7, 8], [-9, 10], [-11, 12]]
    ],
    NESTED_2_FIXED_I16,
    NESTED_2_FIXED_I16_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_u32,
    PNested2FixedU32,
    vec![
        vec![[1, 2], [3, 4]],
        vec![[5, 6]],
        vec![[7, 8], [9, 10], [11, 12]]
    ],
    NESTED_2_FIXED_U32,
    NESTED_2_FIXED_U32_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_i32,
    PNested2FixedI32,
    vec![
        vec![[-1, 2], [-3, 4]],
        vec![[-5, 6]],
        vec![[-7, 8], [-9, 10], [-11, 12]]
    ],
    NESTED_2_FIXED_I32,
    NESTED_2_FIXED_I32_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_u64,
    PNested2FixedU64,
    vec![
        vec![[1, 2], [3, 4]],
        vec![[5, 6]],
        vec![[7, 8], [9, 10], [11, 12]]
    ],
    NESTED_2_FIXED_U64,
    NESTED_2_FIXED_U64_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_i64,
    PNested2FixedI64,
    vec![
        vec![[-1, 2], [-3, 4]],
        vec![[-5, 6]],
        vec![[-7, 8], [-9, 10], [-11, 12]]
    ],
    NESTED_2_FIXED_I64,
    NESTED_2_FIXED_I64_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_f16,
    PNested2FixedF16,
    vec![
        vec![
            [f16::from_f32(1.5), f16::from_f32(-2.0)],
            [f16::from_f32(3.25), f16::from_f32(4.0)]
        ],
        vec![[f16::from_f32(-5.5), f16::from_f32(6.0)]],
        vec![
            [f16::from_f32(7.5), f16::from_f32(-8.0)],
            [f16::from_f32(9.25), f16::from_f32(10.0)],
            [f16::from_f32(-11.5), f16::from_f32(12.0)]
        ]
    ],
    NESTED_2_FIXED_F16,
    NESTED_2_FIXED_F16_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_f32,
    PNested2FixedF32,
    vec![
        vec![[1.5, -2.0], [3.25, 4.0]],
        vec![[-5.5, 6.0]],
        vec![[7.5, -8.0], [9.25, 10.0], [-11.5, 12.0]]
    ],
    NESTED_2_FIXED_F32,
    NESTED_2_FIXED_F32_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_f64,
    PNested2FixedF64,
    vec![
        vec![[1.5, -2.0], [3.25, 4.0]],
        vec![[-5.5, 6.0]],
        vec![[7.5, -8.0], [9.25, 10.0], [-11.5, 12.0]]
    ],
    NESTED_2_FIXED_F64,
    NESTED_2_FIXED_F64_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_bool,
    PNested2FixedBool,
    vec![
        vec![[true, false], [true, true]],
        vec![[true, false]],
        vec![[false, true], [true, false], [false, false]]
    ],
    NESTED_2_FIXED_BOOL,
    NESTED_2_FIXED_BOOL_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_bool_8,
    PNested2FixedBool8,
    vec![
        vec![lsb_bools::<8>(0x01), lsb_bools::<8>(0x80)],
        vec![lsb_bools::<8>(0xFF)],
        vec![
            lsb_bools::<8>(0x00),
            lsb_bools::<8>(0xA5),
            lsb_bools::<8>(0x3C)
        ]
    ],
    NESTED_2_FIXED_BOOL_8,
    NESTED_2_FIXED_BOOL_8_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_bool_16,
    PNested2FixedBool16,
    vec![
        vec![lsb_bools::<16>(0x0001), lsb_bools::<16>(0x8000)],
        vec![lsb_bools::<16>(0xFFFF)],
        vec![
            lsb_bools::<16>(0x0000),
            lsb_bools::<16>(0xA55A),
            lsb_bools::<16>(0x3CC3)
        ]
    ],
    NESTED_2_FIXED_BOOL_16,
    NESTED_2_FIXED_BOOL_16_INODES
);

// ---------------------------------------------------------------------------
// Empty lists at every depth (`_EMPTY_d`: the list at depth `d` is empty)
// ---------------------------------------------------------------------------
value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_u8_empty_0,
    PNested2FixedU8,
    vec![],
    NESTED_2_FIXED_U8_EMPTY_0,
    NESTED_2_FIXED_U8_EMPTY_0_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_u8_empty_1,
    PNested2FixedU8,
    vec![vec![]],
    NESTED_2_FIXED_U8_EMPTY_1,
    NESTED_2_FIXED_U8_EMPTY_1_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_i8_empty_0,
    PNested2FixedI8,
    vec![],
    NESTED_2_FIXED_I8_EMPTY_0,
    NESTED_2_FIXED_I8_EMPTY_0_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_i8_empty_1,
    PNested2FixedI8,
    vec![vec![]],
    NESTED_2_FIXED_I8_EMPTY_1,
    NESTED_2_FIXED_I8_EMPTY_1_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_u16_empty_0,
    PNested2FixedU16,
    vec![],
    NESTED_2_FIXED_U16_EMPTY_0,
    NESTED_2_FIXED_U16_EMPTY_0_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_u16_empty_1,
    PNested2FixedU16,
    vec![vec![]],
    NESTED_2_FIXED_U16_EMPTY_1,
    NESTED_2_FIXED_U16_EMPTY_1_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_i16_empty_0,
    PNested2FixedI16,
    vec![],
    NESTED_2_FIXED_I16_EMPTY_0,
    NESTED_2_FIXED_I16_EMPTY_0_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_i16_empty_1,
    PNested2FixedI16,
    vec![vec![]],
    NESTED_2_FIXED_I16_EMPTY_1,
    NESTED_2_FIXED_I16_EMPTY_1_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_u32_empty_0,
    PNested2FixedU32,
    vec![],
    NESTED_2_FIXED_U32_EMPTY_0,
    NESTED_2_FIXED_U32_EMPTY_0_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_u32_empty_1,
    PNested2FixedU32,
    vec![vec![]],
    NESTED_2_FIXED_U32_EMPTY_1,
    NESTED_2_FIXED_U32_EMPTY_1_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_i32_empty_0,
    PNested2FixedI32,
    vec![],
    NESTED_2_FIXED_I32_EMPTY_0,
    NESTED_2_FIXED_I32_EMPTY_0_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_i32_empty_1,
    PNested2FixedI32,
    vec![vec![]],
    NESTED_2_FIXED_I32_EMPTY_1,
    NESTED_2_FIXED_I32_EMPTY_1_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_u64_empty_0,
    PNested2FixedU64,
    vec![],
    NESTED_2_FIXED_U64_EMPTY_0,
    NESTED_2_FIXED_U64_EMPTY_0_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_u64_empty_1,
    PNested2FixedU64,
    vec![vec![]],
    NESTED_2_FIXED_U64_EMPTY_1,
    NESTED_2_FIXED_U64_EMPTY_1_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_i64_empty_0,
    PNested2FixedI64,
    vec![],
    NESTED_2_FIXED_I64_EMPTY_0,
    NESTED_2_FIXED_I64_EMPTY_0_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_i64_empty_1,
    PNested2FixedI64,
    vec![vec![]],
    NESTED_2_FIXED_I64_EMPTY_1,
    NESTED_2_FIXED_I64_EMPTY_1_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_f16_empty_0,
    PNested2FixedF16,
    vec![],
    NESTED_2_FIXED_F16_EMPTY_0,
    NESTED_2_FIXED_F16_EMPTY_0_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_f16_empty_1,
    PNested2FixedF16,
    vec![vec![]],
    NESTED_2_FIXED_F16_EMPTY_1,
    NESTED_2_FIXED_F16_EMPTY_1_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_f32_empty_0,
    PNested2FixedF32,
    vec![],
    NESTED_2_FIXED_F32_EMPTY_0,
    NESTED_2_FIXED_F32_EMPTY_0_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_f32_empty_1,
    PNested2FixedF32,
    vec![vec![]],
    NESTED_2_FIXED_F32_EMPTY_1,
    NESTED_2_FIXED_F32_EMPTY_1_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_f64_empty_0,
    PNested2FixedF64,
    vec![],
    NESTED_2_FIXED_F64_EMPTY_0,
    NESTED_2_FIXED_F64_EMPTY_0_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_f64_empty_1,
    PNested2FixedF64,
    vec![vec![]],
    NESTED_2_FIXED_F64_EMPTY_1,
    NESTED_2_FIXED_F64_EMPTY_1_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_bool_empty_0,
    PNested2FixedBool,
    vec![],
    NESTED_2_FIXED_BOOL_EMPTY_0,
    NESTED_2_FIXED_BOOL_EMPTY_0_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_bool_empty_1,
    PNested2FixedBool,
    vec![vec![]],
    NESTED_2_FIXED_BOOL_EMPTY_1,
    NESTED_2_FIXED_BOOL_EMPTY_1_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_bool_8_empty_0,
    PNested2FixedBool8,
    vec![],
    NESTED_2_FIXED_BOOL_8_EMPTY_0,
    NESTED_2_FIXED_BOOL_8_EMPTY_0_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_bool_8_empty_1,
    PNested2FixedBool8,
    vec![vec![]],
    NESTED_2_FIXED_BOOL_8_EMPTY_1,
    NESTED_2_FIXED_BOOL_8_EMPTY_1_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_bool_16_empty_0,
    PNested2FixedBool16,
    vec![],
    NESTED_2_FIXED_BOOL_16_EMPTY_0,
    NESTED_2_FIXED_BOOL_16_EMPTY_0_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_bool_16_empty_1,
    PNested2FixedBool16,
    vec![vec![]],
    NESTED_2_FIXED_BOOL_16_EMPTY_1,
    NESTED_2_FIXED_BOOL_16_EMPTY_1_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_timestamp,
    PNested2FixedTimestamp,
    vec![
        vec![
            [Time(1_500_000_000), Time(-2_000_000_000)],
            [Time(3_250_000_000), Time(4_000_000_000)]
        ],
        vec![[Time(-5_500_000_000), Time(6_000_000_000)]],
        vec![
            [Time(7_500_000_000), Time(-8_000_000_000)],
            [Time(9_250_000_000), Time(10_000_000_000)],
            [Time(-11_500_000_000), Time(12_000_000_000)]
        ]
    ],
    NESTED_2_FIXED_TIMESTAMP,
    NESTED_2_FIXED_TIMESTAMP_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_timestamp_empty_0,
    PNested2FixedTimestamp,
    vec![],
    NESTED_2_FIXED_TIMESTAMP_EMPTY_0,
    NESTED_2_FIXED_TIMESTAMP_EMPTY_0_INODES
);

value_match_nested_2_fixed_test!(
    value_match_nested_2_fixed_timestamp_empty_1,
    PNested2FixedTimestamp,
    vec![vec![]],
    NESTED_2_FIXED_TIMESTAMP_EMPTY_1,
    NESTED_2_FIXED_TIMESTAMP_EMPTY_1_INODES
);
