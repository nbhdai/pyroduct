//! Struct-vs-value match for `src/wire/tests/basic_2_nesting_u32.rs` —
//! `List<List<Str>>` / `List<List<Bytes>>` against derived structs
//! with native `Vec<Vec<String>>` / `Vec<Vec<Vec<u8>>>` fields. See
//! `tests/value_match/primitive_u32.rs`'s module doc for the shared rationale.

use pyroformat::wire::tests::basic_2_nesting_u32::*;

use crate::support::read_back::assert_roundtrip;
use pyroformat::{Layout, Reader, Writer};

macro_rules! value_match_basic_2_test {
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
struct PBasic2Str {
    a: Vec<Vec<String>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PBasic2Bytes {
    a: Vec<Vec<Vec<u8>>>,
}

value_match_basic_2_test!(
    value_match_basic_2_str,
    PBasic2Str,
    vec![
        vec!["ab".to_string(), "".to_string()],
        vec!["cde".to_string()],
        vec!["f".to_string(), "ghij".to_string(), "é".to_string()]
    ],
    BASIC_2_STR,
    BASIC_2_STR_INODES
);

value_match_basic_2_test!(
    value_match_basic_2_bytes,
    PBasic2Bytes,
    vec![
        vec![vec![1, 2], vec![] as Vec<u8>],
        vec![vec![3, 4, 5]],
        vec![vec![255], vec![0, 6, 7, 8], vec![9]]
    ],
    BASIC_2_BYTES,
    BASIC_2_BYTES_INODES
);
