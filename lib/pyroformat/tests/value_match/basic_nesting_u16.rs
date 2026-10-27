//! Struct-vs-value match for `src/wire/tests/basic_nesting_u16.rs` — the
//! `Str`/`Bytes` counterpart to `primitive_nesting.rs`: `List<Str>`/
//! `List<Bytes>` (`Vec<String>`/`Vec<Vec<u8>>`) and `List<List<Str>>`/
//! `List<List<Bytes>>` (`Vec<Vec<String>>`/`Vec<Vec<Vec<u8>>>`). See
//! `tests/value_match/primitive_u16.rs`'s module doc for the shared rationale.

use pyroformat::wire::tests::basic_nesting_u16::*;

use crate::support::read_back::assert_roundtrip;
use pyroformat::{Layout, Reader, Writer};

macro_rules! value_match_basic_nesting_test {
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
struct PVecStr {
    a: Vec<String>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecBytes {
    a: Vec<Vec<u8>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecVecStr {
    a: Vec<Vec<String>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecVecBytes {
    a: Vec<Vec<Vec<u8>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GroupStr {
    s: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PGroupStr {
    a: Option<GroupStr>,
}

value_match_basic_nesting_test!(
    value_match_list_of_str,
    PVecStr,
    vec!["ab".to_string(), "".to_string(), "cde".to_string()],
    LIST_OF_STR,
    LIST_OF_STR_INODES
);

value_match_basic_nesting_test!(
    value_match_list_of_bytes,
    PVecBytes,
    vec![vec![10u8, 20, 30], vec![40u8, 50]],
    LIST_OF_BYTES,
    LIST_OF_BYTES_INODES
);

value_match_basic_nesting_test!(
    value_match_list_of_list_str,
    PVecVecStr,
    vec![
        vec!["x".to_string(), "yz".to_string()],
        vec!["abc".to_string()],
    ],
    LIST_OF_LIST_STR,
    LIST_OF_LIST_STR_INODES
);

value_match_basic_nesting_test!(
    value_match_list_of_list_bytes,
    PVecVecBytes,
    vec![vec![vec![1u8], vec![2u8, 3]], vec![vec![4u8, 5, 6]]],
    LIST_OF_LIST_BYTES,
    LIST_OF_LIST_BYTES_INODES
);

value_match_basic_nesting_test!(
    value_match_group_option_str_some,
    PGroupStr,
    Some(GroupStr {
        s: "hello".to_string(),
    }),
    GROUP_OPTION_STR_SOME,
    GROUP_OPTION_STR_INODES
);
