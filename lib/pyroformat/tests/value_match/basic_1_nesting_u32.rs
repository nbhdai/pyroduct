//! Struct-vs-value match for `src/wire/tests/basic_1_nesting_u32.rs` —
//! `List<Str>` / `List<Bytes>` against derived structs
//! with native `Vec<String>` / `Vec<Vec<u8>>` fields. See
//! `tests/value_match/primitive_u32.rs`'s module doc for the shared rationale.

use pyroformat::wire::tests::basic_1_nesting_u32::*;

use crate::support::read_back::assert_roundtrip;
use pyroformat::{Layout, Reader, Writer};

macro_rules! value_match_basic_1_test {
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
struct PBasic1Str {
    a: Vec<String>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PBasic1Bytes {
    a: Vec<Vec<u8>>,
}

value_match_basic_1_test!(
    value_match_basic_1_str,
    PBasic1Str,
    vec![
        "ab".to_string(),
        "".to_string(),
        "cde".to_string(),
        "f".to_string(),
        "ghij".to_string(),
        "é".to_string()
    ],
    BASIC_1_STR,
    BASIC_1_STR_INODES
);

value_match_basic_1_test!(
    value_match_basic_1_bytes,
    PBasic1Bytes,
    vec![
        vec![1, 2],
        vec![] as Vec<u8>,
        vec![3, 4, 5],
        vec![255],
        vec![0, 6, 7, 8],
        vec![9]
    ],
    BASIC_1_BYTES,
    BASIC_1_BYTES_INODES
);
