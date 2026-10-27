//! Struct-vs-value match for `src/wire/tests/primitive_slice_option_u16.rs` —
//! `Vec<Option<T>>` for every `PrimitiveDataType` variant. See
//! `tests/value_match/primitive_u16.rs`'s module doc for the shared rationale.

use half::f16;

use pyroformat::wire::tests::primitive_slice_option_u16::*;

use crate::support::read_back::assert_roundtrip;
use pyroformat::{Layout, Reader, Writer};

macro_rules! value_match_slice_option_test {
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
struct PVecOptBool {
    a: Vec<Option<bool>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecOptI8 {
    a: Vec<Option<i8>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecOptI16 {
    a: Vec<Option<i16>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecOptI32 {
    a: Vec<Option<i32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecOptI64 {
    a: Vec<Option<i64>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecOptU8 {
    a: Vec<Option<u8>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecOptU16 {
    a: Vec<Option<u16>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecOptU32 {
    a: Vec<Option<u32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecOptU64 {
    a: Vec<Option<u64>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecOptF16 {
    a: Vec<Option<f16>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecOptF32 {
    a: Vec<Option<f32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecOptF64 {
    a: Vec<Option<f64>>,
}

value_match_slice_option_test!(
    value_match_slice_option_bool,
    PVecOptBool,
    vec![Some(true), None, Some(false)],
    VALUE_WRITER_OPTION_BOOL_LIST,
    VALUE_WRITER_OPTION_BOOL_LIST_INODES
);
value_match_slice_option_test!(
    value_match_slice_option_i8,
    PVecOptI8,
    vec![Some(1i8), None, Some(-1)],
    VALUE_WRITER_OPTION_I8_LIST,
    VALUE_WRITER_OPTION_I8_LIST_INODES
);
value_match_slice_option_test!(
    value_match_slice_option_i16,
    PVecOptI16,
    vec![Some(1i16), None, Some(-1)],
    VALUE_WRITER_OPTION_I16_LIST,
    VALUE_WRITER_OPTION_I16_LIST_INODES
);
value_match_slice_option_test!(
    value_match_slice_option_i32,
    PVecOptI32,
    vec![Some(1i32), None, Some(-1)],
    VALUE_WRITER_OPTION_I32_LIST,
    VALUE_WRITER_OPTION_I32_LIST_INODES
);
value_match_slice_option_test!(
    value_match_slice_option_i64,
    PVecOptI64,
    vec![Some(1i64), None, Some(-1)],
    VALUE_WRITER_OPTION_I64_LIST,
    VALUE_WRITER_OPTION_I64_LIST_INODES
);
value_match_slice_option_test!(
    value_match_slice_option_u8,
    PVecOptU8,
    vec![Some(1u8), None, Some(3)],
    VALUE_WRITER_OPTION_U8_LIST,
    VALUE_WRITER_OPTION_U8_LIST_INODES
);
value_match_slice_option_test!(
    value_match_slice_option_u16,
    PVecOptU16,
    vec![Some(1u16), None, Some(3)],
    VALUE_WRITER_OPTION_U16_LIST,
    VALUE_WRITER_OPTION_U16_LIST_INODES
);
value_match_slice_option_test!(
    value_match_slice_option_u32,
    PVecOptU32,
    vec![Some(1u32), None, Some(3)],
    VALUE_WRITER_OPTION_U32_LIST,
    VALUE_WRITER_OPTION_U32_LIST_INODES
);
value_match_slice_option_test!(
    value_match_slice_option_u64,
    PVecOptU64,
    vec![Some(1u64), None, Some(3)],
    VALUE_WRITER_OPTION_U64_LIST,
    VALUE_WRITER_OPTION_U64_LIST_INODES
);
value_match_slice_option_test!(
    value_match_slice_option_f16,
    PVecOptF16,
    vec![Some(f16::from_f32(1.0)), None, Some(f16::from_f32(3.0))],
    VALUE_WRITER_OPTION_F16_LIST,
    VALUE_WRITER_OPTION_F16_LIST_INODES
);
value_match_slice_option_test!(
    value_match_slice_option_f32,
    PVecOptF32,
    vec![Some(1.0f32), None, Some(3.0)],
    VALUE_WRITER_OPTION_F32_LIST,
    VALUE_WRITER_OPTION_F32_LIST_INODES
);
value_match_slice_option_test!(
    value_match_slice_option_f64,
    PVecOptF64,
    vec![Some(1.0f64), None, Some(3.0)],
    VALUE_WRITER_OPTION_F64_LIST,
    VALUE_WRITER_OPTION_F64_LIST_INODES
);
value_match_slice_option_test!(
    value_match_slice_option_u32_all_none,
    PVecOptU32,
    vec![None, None, None],
    VALUE_WRITER_OPTION_U32_LIST_ALL_NONE,
    VALUE_WRITER_OPTION_U32_LIST_ALL_NONE_INODES
);
