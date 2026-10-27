//! Struct-vs-value match for `src/wire/tests/primitive_slice_u16.rs` —
//! `Vec<T>` for every `PrimitiveDataType` variant. See
//! `tests/value_match/primitive_u16.rs`'s module doc for the shared rationale.

use half::f16;

use pyroformat::wire::tests::primitive_slice_u16::*;

use crate::support::read_back::assert_roundtrip;
use pyroformat::{Layout, Reader, Writer};

macro_rules! value_match_slice_test {
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
struct PVecBool {
    #[pyro(allow(vec_bool))]
    a: Vec<bool>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecI8 {
    a: Vec<i8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecI16 {
    a: Vec<i16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecI32 {
    a: Vec<i32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecI64 {
    a: Vec<i64>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecU8 {
    #[pyro(numeric)]
    a: Vec<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecU16 {
    a: Vec<u16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecU32 {
    a: Vec<u32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecU64 {
    a: Vec<u64>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecF16 {
    a: Vec<f16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecF32 {
    a: Vec<f32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecF64 {
    a: Vec<f64>,
}

value_match_slice_test!(
    value_match_slice_bool,
    PVecBool,
    vec![true, false, true],
    BASIC_BOOL_LIST,
    BASIC_BOOL_LIST_INODES
);
value_match_slice_test!(
    value_match_slice_i8,
    PVecI8,
    vec![-1i8, 0, 1],
    BASIC_I8_LIST,
    BASIC_I8_LIST_INODES
);
value_match_slice_test!(
    value_match_slice_u8,
    PVecU8,
    vec![1u8, 2, 3],
    BASIC_U8_LIST,
    BASIC_U8_LIST_INODES
);
value_match_slice_test!(
    value_match_slice_i16,
    PVecI16,
    vec![-1i16, 0, 1],
    BASIC_I16_LIST,
    BASIC_I16_LIST_INODES
);
value_match_slice_test!(
    value_match_slice_i32,
    PVecI32,
    vec![-1i32, 0, 1],
    BASIC_I32_LIST,
    BASIC_I32_LIST_INODES
);
value_match_slice_test!(
    value_match_slice_i64,
    PVecI64,
    vec![-1i64, 0, 1],
    BASIC_I64_LIST,
    BASIC_I64_LIST_INODES
);
value_match_slice_test!(
    value_match_slice_u16,
    PVecU16,
    vec![1u16, 2, 3],
    BASIC_U16_LIST,
    BASIC_U16_LIST_INODES
);
value_match_slice_test!(
    value_match_slice_u32,
    PVecU32,
    vec![1u32, 2, 3],
    BASIC_U32_LIST,
    BASIC_U32_LIST_INODES
);
value_match_slice_test!(
    value_match_slice_u64,
    PVecU64,
    vec![1u64, 2, 3],
    BASIC_U64_LIST,
    BASIC_U64_LIST_INODES
);
value_match_slice_test!(
    value_match_slice_f16,
    PVecF16,
    vec![f16::from_f32(1.0), f16::from_f32(2.0), f16::from_f32(3.0)],
    BASIC_F16_LIST,
    BASIC_F16_LIST_INODES
);
value_match_slice_test!(
    value_match_slice_f32,
    PVecF32,
    vec![1.0f32, 2.0, 3.0],
    BASIC_F32_LIST,
    BASIC_F32_LIST_INODES
);
value_match_slice_test!(
    value_match_slice_f64,
    PVecF64,
    vec![1.0f64, 2.0, 3.0],
    BASIC_F64_LIST,
    BASIC_F64_LIST_INODES
);
