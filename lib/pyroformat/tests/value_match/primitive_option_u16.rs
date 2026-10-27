//! Struct-vs-value match for `src/wire/tests/primitive_option_u16.rs` —
//! `Option<T>` for every `PrimitiveDataType` variant, `Some` and `None`.
//! See `tests/value_match/primitive_u16.rs`'s module doc for the shared
//! rationale.

use half::f16;

use pyroformat::value::Time;
use pyroformat::wire::tests::primitive_option_u16::*;

use crate::support::read_back::assert_roundtrip;
use pyroformat::{Layout, Reader, Writer};

macro_rules! value_match_option_test {
    ($some_name:ident, $none_name:ident, $struct_ty:ident, $inner_ty:ty, $inner_value:expr, $some_buf_const:expr, $none_buf_const:expr, $inodes_const:expr) => {
        #[test]
        fn $some_name() {
            let data = $struct_ty {
                a: Some($inner_value),
            };
            assert_roundtrip(&data, &$some_buf_const, &$inodes_const);
        }

        #[test]
        fn $none_name() {
            let data = $struct_ty {
                a: None::<$inner_ty>,
            };
            assert_roundtrip(&data, &$none_buf_const, &$inodes_const);
        }
    };
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct POptBool {
    a: Option<bool>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct POptI8 {
    a: Option<i8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct POptI16 {
    a: Option<i16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct POptI32 {
    a: Option<i32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct POptI64 {
    a: Option<i64>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct POptU8 {
    a: Option<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct POptU16 {
    a: Option<u16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct POptU32 {
    a: Option<u32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct POptU64 {
    a: Option<u64>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct POptF16 {
    a: Option<f16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct POptF32 {
    a: Option<f32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct POptF64 {
    a: Option<f64>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct POptTimestamp {
    a: Option<Time>,
}

value_match_option_test!(
    value_match_option_bool_some,
    value_match_option_bool_none,
    POptBool,
    bool,
    true,
    OPTION_BOOL_SOME,
    OPTION_BOOL_NONE,
    OPTION_BOOL_INODES
);

value_match_option_test!(
    value_match_option_i8_some,
    value_match_option_i8_none,
    POptI8,
    i8,
    -100i8,
    OPTION_I8_SOME,
    OPTION_I8_NONE,
    OPTION_I8_INODES
);

value_match_option_test!(
    value_match_option_i16_some,
    value_match_option_i16_none,
    POptI16,
    i16,
    -12345i16,
    OPTION_I16_SOME,
    OPTION_I16_NONE,
    OPTION_I16_INODES
);

value_match_option_test!(
    value_match_option_i32_some,
    value_match_option_i32_none,
    POptI32,
    i32,
    -123456i32,
    OPTION_I32_SOME,
    OPTION_I32_NONE,
    OPTION_I32_INODES
);

value_match_option_test!(
    value_match_option_i64_some,
    value_match_option_i64_none,
    POptI64,
    i64,
    -1234567890123i64,
    OPTION_I64_SOME,
    OPTION_I64_NONE,
    OPTION_I64_INODES
);

value_match_option_test!(
    value_match_option_u8_some,
    value_match_option_u8_none,
    POptU8,
    u8,
    5u8,
    OPTION_U8_SOME,
    OPTION_U8_NONE,
    OPTION_U8_INODES
);

value_match_option_test!(
    value_match_option_u16_some,
    value_match_option_u16_none,
    POptU16,
    u16,
    54321u16,
    OPTION_U16_SOME,
    OPTION_U16_NONE,
    OPTION_U16_INODES
);

value_match_option_test!(
    value_match_option_u32_some,
    value_match_option_u32_none,
    POptU32,
    u32,
    123456u32,
    OPTION_U32_SOME,
    OPTION_U32_NONE,
    OPTION_U32_INODES
);

value_match_option_test!(
    value_match_option_u64_some,
    value_match_option_u64_none,
    POptU64,
    u64,
    12345678901234567890u64,
    OPTION_U64_SOME,
    OPTION_U64_NONE,
    OPTION_U64_INODES
);

value_match_option_test!(
    value_match_option_f16_some,
    value_match_option_f16_none,
    POptF16,
    f16,
    f16::from_f32(1.5),
    OPTION_F16_SOME,
    OPTION_F16_NONE,
    OPTION_F16_INODES
);

value_match_option_test!(
    value_match_option_f32_some,
    value_match_option_f32_none,
    POptF32,
    f32,
    3.14f32,
    OPTION_F32_SOME,
    OPTION_F32_NONE,
    OPTION_F32_INODES
);

value_match_option_test!(
    value_match_option_f64_some,
    value_match_option_f64_none,
    POptF64,
    f64,
    2.718281828f64,
    OPTION_F64_SOME,
    OPTION_F64_NONE,
    OPTION_F64_INODES
);

value_match_option_test!(
    value_match_option_timestamp_some,
    value_match_option_timestamp_none,
    POptTimestamp,
    Time,
    Time(999_999_999),
    OPTION_TIMESTAMP_SOME,
    OPTION_TIMESTAMP_NONE,
    OPTION_TIMESTAMP_INODES
);
