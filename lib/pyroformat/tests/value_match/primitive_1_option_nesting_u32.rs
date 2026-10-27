//! Struct-vs-value match for `src/wire/tests/primitive_1_option_nesting_u32.rs` —
//! `List<Option<T>>` against a derived struct with a native
//! `Vec<Option<T>>` field. See `tests/value_match/primitive_u32.rs`'s
//! module doc for the shared harness rationale.

use half::f16;

use pyroformat::value::Time;
use pyroformat::wire::tests::primitive_1_option_nesting_u32::*;

use crate::support::read_back::assert_roundtrip;
use pyroformat::{Layout, Reader, Writer};

macro_rules! value_match_nested_1_option_test {
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
struct PNested1OptionU8 {
    a: Vec<Option<u8>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionI8 {
    a: Vec<Option<i8>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionU16 {
    a: Vec<Option<u16>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionI16 {
    a: Vec<Option<i16>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionU32 {
    a: Vec<Option<u32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionI32 {
    a: Vec<Option<i32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionU64 {
    a: Vec<Option<u64>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionI64 {
    a: Vec<Option<i64>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionF16 {
    a: Vec<Option<f16>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionF32 {
    a: Vec<Option<f32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionF64 {
    a: Vec<Option<f64>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionBool {
    a: Vec<Option<bool>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionTimestamp {
    a: Vec<Option<Time>>,
}

value_match_nested_1_option_test!(
    value_match_nested_1_option_u8,
    PNested1OptionU8,
    vec![Some(1), None, Some(3), Some(4), None, Some(6)],
    NESTED_1_OPTION_U8,
    NESTED_1_OPTION_U8_INODES
);

value_match_nested_1_option_test!(
    value_match_nested_1_option_i8,
    PNested1OptionI8,
    vec![Some(-1), None, Some(-3), Some(4), None, Some(6)],
    NESTED_1_OPTION_I8,
    NESTED_1_OPTION_I8_INODES
);

value_match_nested_1_option_test!(
    value_match_nested_1_option_u16,
    PNested1OptionU16,
    vec![Some(1), None, Some(3), Some(4), None, Some(6)],
    NESTED_1_OPTION_U16,
    NESTED_1_OPTION_U16_INODES
);

value_match_nested_1_option_test!(
    value_match_nested_1_option_i16,
    PNested1OptionI16,
    vec![Some(-1), None, Some(-3), Some(4), None, Some(6)],
    NESTED_1_OPTION_I16,
    NESTED_1_OPTION_I16_INODES
);

value_match_nested_1_option_test!(
    value_match_nested_1_option_u32,
    PNested1OptionU32,
    vec![Some(1), None, Some(3), Some(4), None, Some(6)],
    NESTED_1_OPTION_U32,
    NESTED_1_OPTION_U32_INODES
);

value_match_nested_1_option_test!(
    value_match_nested_1_option_i32,
    PNested1OptionI32,
    vec![Some(-1), None, Some(-3), Some(4), None, Some(6)],
    NESTED_1_OPTION_I32,
    NESTED_1_OPTION_I32_INODES
);

value_match_nested_1_option_test!(
    value_match_nested_1_option_u64,
    PNested1OptionU64,
    vec![Some(1), None, Some(3), Some(4), None, Some(6)],
    NESTED_1_OPTION_U64,
    NESTED_1_OPTION_U64_INODES
);

value_match_nested_1_option_test!(
    value_match_nested_1_option_i64,
    PNested1OptionI64,
    vec![Some(-1), None, Some(-3), Some(4), None, Some(6)],
    NESTED_1_OPTION_I64,
    NESTED_1_OPTION_I64_INODES
);

value_match_nested_1_option_test!(
    value_match_nested_1_option_f16,
    PNested1OptionF16,
    vec![
        Some(f16::from_f32(1.5)),
        None,
        Some(f16::from_f32(3.25)),
        Some(f16::from_f32(4.0)),
        None,
        Some(f16::from_f32(6.0))
    ],
    NESTED_1_OPTION_F16,
    NESTED_1_OPTION_F16_INODES
);

value_match_nested_1_option_test!(
    value_match_nested_1_option_f32,
    PNested1OptionF32,
    vec![Some(1.5), None, Some(3.25), Some(4.0), None, Some(6.0)],
    NESTED_1_OPTION_F32,
    NESTED_1_OPTION_F32_INODES
);

value_match_nested_1_option_test!(
    value_match_nested_1_option_f64,
    PNested1OptionF64,
    vec![Some(1.5), None, Some(3.25), Some(4.0), None, Some(6.0)],
    NESTED_1_OPTION_F64,
    NESTED_1_OPTION_F64_INODES
);

value_match_nested_1_option_test!(
    value_match_nested_1_option_bool,
    PNested1OptionBool,
    vec![Some(true), None, Some(true), Some(true), None, Some(false)],
    NESTED_1_OPTION_BOOL,
    NESTED_1_OPTION_BOOL_INODES
);

value_match_nested_1_option_test!(
    value_match_nested_1_option_timestamp,
    PNested1OptionTimestamp,
    vec![
        Some(Time(1_000_001_000)),
        None,
        Some(Time(3_500_003_000)),
        Some(Time(4_000_004_000)),
        None,
        Some(Time(1_700_000_000_000_006_000))
    ],
    NESTED_1_OPTION_TIMESTAMP,
    NESTED_1_OPTION_TIMESTAMP_INODES
);

// ---------------------------------------------------------------------------
// Empty lists at every depth (`_EMPTY_d`: the list at depth `d` is empty)
// ---------------------------------------------------------------------------
value_match_nested_1_option_test!(
    value_match_nested_1_option_u8_empty_0,
    PNested1OptionU8,
    vec![],
    NESTED_1_OPTION_U8_EMPTY_0,
    NESTED_1_OPTION_U8_EMPTY_0_INODES
);

value_match_nested_1_option_test!(
    value_match_nested_1_option_i8_empty_0,
    PNested1OptionI8,
    vec![],
    NESTED_1_OPTION_I8_EMPTY_0,
    NESTED_1_OPTION_I8_EMPTY_0_INODES
);

value_match_nested_1_option_test!(
    value_match_nested_1_option_u16_empty_0,
    PNested1OptionU16,
    vec![],
    NESTED_1_OPTION_U16_EMPTY_0,
    NESTED_1_OPTION_U16_EMPTY_0_INODES
);

value_match_nested_1_option_test!(
    value_match_nested_1_option_i16_empty_0,
    PNested1OptionI16,
    vec![],
    NESTED_1_OPTION_I16_EMPTY_0,
    NESTED_1_OPTION_I16_EMPTY_0_INODES
);

value_match_nested_1_option_test!(
    value_match_nested_1_option_u32_empty_0,
    PNested1OptionU32,
    vec![],
    NESTED_1_OPTION_U32_EMPTY_0,
    NESTED_1_OPTION_U32_EMPTY_0_INODES
);

value_match_nested_1_option_test!(
    value_match_nested_1_option_i32_empty_0,
    PNested1OptionI32,
    vec![],
    NESTED_1_OPTION_I32_EMPTY_0,
    NESTED_1_OPTION_I32_EMPTY_0_INODES
);

value_match_nested_1_option_test!(
    value_match_nested_1_option_u64_empty_0,
    PNested1OptionU64,
    vec![],
    NESTED_1_OPTION_U64_EMPTY_0,
    NESTED_1_OPTION_U64_EMPTY_0_INODES
);

value_match_nested_1_option_test!(
    value_match_nested_1_option_i64_empty_0,
    PNested1OptionI64,
    vec![],
    NESTED_1_OPTION_I64_EMPTY_0,
    NESTED_1_OPTION_I64_EMPTY_0_INODES
);

value_match_nested_1_option_test!(
    value_match_nested_1_option_f16_empty_0,
    PNested1OptionF16,
    vec![],
    NESTED_1_OPTION_F16_EMPTY_0,
    NESTED_1_OPTION_F16_EMPTY_0_INODES
);

value_match_nested_1_option_test!(
    value_match_nested_1_option_f32_empty_0,
    PNested1OptionF32,
    vec![],
    NESTED_1_OPTION_F32_EMPTY_0,
    NESTED_1_OPTION_F32_EMPTY_0_INODES
);

value_match_nested_1_option_test!(
    value_match_nested_1_option_f64_empty_0,
    PNested1OptionF64,
    vec![],
    NESTED_1_OPTION_F64_EMPTY_0,
    NESTED_1_OPTION_F64_EMPTY_0_INODES
);

value_match_nested_1_option_test!(
    value_match_nested_1_option_bool_empty_0,
    PNested1OptionBool,
    vec![],
    NESTED_1_OPTION_BOOL_EMPTY_0,
    NESTED_1_OPTION_BOOL_EMPTY_0_INODES
);

value_match_nested_1_option_test!(
    value_match_nested_1_option_timestamp_empty_0,
    PNested1OptionTimestamp,
    vec![],
    NESTED_1_OPTION_TIMESTAMP_EMPTY_0,
    NESTED_1_OPTION_TIMESTAMP_EMPTY_0_INODES
);
