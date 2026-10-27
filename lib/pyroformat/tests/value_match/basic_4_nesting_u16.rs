//! Struct-vs-value match for `src/wire/tests/basic_4_nesting_u16.rs` —
//! `List<List<List<List<Str>>>>` / `List<List<List<List<Bytes>>>>` against derived structs
//! with native `Vec<Vec<Vec<Vec<String>>>>` / `Vec<Vec<Vec<Vec<Vec<u8>>>>>` fields. See
//! `tests/value_match/primitive_u16.rs`'s module doc for the shared rationale.

use pyroformat::wire::tests::basic_4_nesting_u16::*;

use crate::support::read_back::assert_roundtrip;
use pyroformat::{Layout, Reader, Writer};

macro_rules! value_match_basic_4_test {
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
struct PBasic4Str {
    a: Vec<Vec<Vec<Vec<String>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PBasic4Bytes {
    a: Vec<Vec<Vec<Vec<Vec<u8>>>>>,
}

value_match_basic_4_test!(
    value_match_basic_4_str,
    PBasic4Str,
    vec![
        vec![
            vec![
                vec!["ab".to_string(), "".to_string()],
                vec!["cde".to_string()]
            ],
            vec![vec!["f".to_string()]]
        ],
        vec![vec![vec!["ghij".to_string(), "é".to_string()]]]
    ],
    BASIC_4_STR,
    BASIC_4_STR_INODES
);

value_match_basic_4_test!(
    value_match_basic_4_bytes,
    PBasic4Bytes,
    vec![
        vec![
            vec![vec![vec![1, 2], vec![] as Vec<u8>], vec![vec![3, 4, 5]]],
            vec![vec![vec![255]]]
        ],
        vec![vec![vec![vec![0, 6, 7, 8], vec![9]]]]
    ],
    BASIC_4_BYTES,
    BASIC_4_BYTES_INODES
);
