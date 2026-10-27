//! Struct-vs-value match for `src/wire/tests/basic_3_nesting_u32.rs` —
//! `List<List<List<Str>>>` / `List<List<List<Bytes>>>` against derived structs
//! with native `Vec<Vec<Vec<String>>>` / `Vec<Vec<Vec<Vec<u8>>>>` fields. See
//! `tests/value_match/primitive_u32.rs`'s module doc for the shared rationale.

use pyroformat::wire::tests::basic_3_nesting_u32::*;

use crate::support::read_back::assert_roundtrip;
use pyroformat::{Layout, Reader, Writer};

macro_rules! value_match_basic_3_test {
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
struct PBasic3Str {
    a: Vec<Vec<Vec<String>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PBasic3Bytes {
    a: Vec<Vec<Vec<Vec<u8>>>>,
}

value_match_basic_3_test!(
    value_match_basic_3_str,
    PBasic3Str,
    vec![
        vec![
            vec!["ab".to_string(), "".to_string()],
            vec!["cde".to_string()]
        ],
        vec![vec!["f".to_string(), "ghij".to_string(), "é".to_string()]]
    ],
    BASIC_3_STR,
    BASIC_3_STR_INODES
);

value_match_basic_3_test!(
    value_match_basic_3_bytes,
    PBasic3Bytes,
    vec![
        vec![vec![vec![1, 2], vec![] as Vec<u8>], vec![vec![3, 4, 5]]],
        vec![vec![vec![255], vec![0, 6, 7, 8], vec![9]]]
    ],
    BASIC_3_BYTES,
    BASIC_3_BYTES_INODES
);
