//! Struct-vs-value match for `src/wire/tests/group_option_1_nesting_u16.rs` — `Vec<..>` of a
//! derived inner struct (`Option<Inner>`) standing in for each group. See
//! `tests/value_match/primitive_u16.rs`'s module doc for the shared rationale.

use pyroformat::wire::tests::group_option_1_nesting_u16::*;

use crate::support::read_back::assert_roundtrip;
use pyroformat::{Layout, Reader, Writer};

macro_rules! value_match_group_test {
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
struct GGroupOption1PrimitiveInner {
    a: u8,
    b: u32,
    c: Option<u16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption1Primitive {
    a: Vec<Option<GGroupOption1PrimitiveInner>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption1BlobInner {
    s: String,
    n: u16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption1Blob {
    a: Vec<Option<GGroupOption1BlobInner>>,
}

value_match_group_test!(
    value_match_group_option_1_primitive,
    GGroupOption1Primitive,
    vec![
        Some(GGroupOption1PrimitiveInner {
            a: 1,
            b: 1,
            c: Some(1)
        }),
        Some(GGroupOption1PrimitiveInner {
            a: 2,
            b: 2,
            c: None
        }),
        None,
        Some(GGroupOption1PrimitiveInner {
            a: 4,
            b: 4,
            c: Some(4)
        }),
        Some(GGroupOption1PrimitiveInner {
            a: 5,
            b: 5,
            c: None
        }),
        None
    ],
    GROUP_OPTION_1_PRIMITIVE,
    GROUP_OPTION_1_PRIMITIVE_INODES
);

value_match_group_test!(
    value_match_group_option_1_blob,
    GGroupOption1Blob,
    vec![
        Some(GGroupOption1BlobInner {
            s: "ab".to_string(),
            n: 1
        }),
        Some(GGroupOption1BlobInner {
            s: "".to_string(),
            n: 2
        }),
        None,
        Some(GGroupOption1BlobInner {
            s: "f".to_string(),
            n: 4
        }),
        Some(GGroupOption1BlobInner {
            s: "ghij".to_string(),
            n: 5
        }),
        None
    ],
    GROUP_OPTION_1_BLOB,
    GROUP_OPTION_1_BLOB_INODES
);

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption1MixedInner {
    a: bool,
    b: Option<f32>,
    c: String,
    d: u64,
    e: Option<i16>,
    f: [u8; 3],
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption1Mixed {
    a: Vec<Option<GGroupOption1MixedInner>>,
}

value_match_group_test!(
    value_match_group_option_1_mixed,
    GGroupOption1Mixed,
    vec![
        Some(GGroupOption1MixedInner {
            a: true,
            b: Some(1.5),
            c: "ab".to_string(),
            d: 1,
            e: None,
            f: [1, 2, 3]
        }),
        Some(GGroupOption1MixedInner {
            a: false,
            b: None,
            c: "".to_string(),
            d: 1099511627779,
            e: Some(-2),
            f: [4, 5, 6]
        }),
        None,
        Some(GGroupOption1MixedInner {
            a: true,
            b: Some(8.0),
            c: "é".to_string(),
            d: 4,
            e: None,
            f: [10, 11, 12]
        }),
        Some(GGroupOption1MixedInner {
            a: true,
            b: None,
            c: "fghij".to_string(),
            d: 18446744073709551615,
            e: Some(-5),
            f: [13, 14, 15]
        }),
        None
    ],
    GROUP_OPTION_1_MIXED,
    GROUP_OPTION_1_MIXED_INODES
);
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption1NestedInner {
    a: u8,
    b: Vec<Vec<u64>>,
    c: Vec<u16>,
    d: Option<i32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption1Nested {
    a: Vec<Option<GGroupOption1NestedInner>>,
}

value_match_group_test!(
    value_match_group_option_1_nested,
    GGroupOption1Nested,
    vec![
        Some(GGroupOption1NestedInner {
            a: 1,
            b: vec![vec![1], vec![2, 3]],
            c: vec![1, 2, 3],
            d: Some(-1)
        }),
        Some(GGroupOption1NestedInner {
            a: 2,
            b: vec![],
            c: vec![],
            d: None
        }),
        None,
        Some(GGroupOption1NestedInner {
            a: 4,
            b: vec![vec![4]],
            c: vec![5, 6],
            d: Some(-4)
        }),
        Some(GGroupOption1NestedInner {
            a: 5,
            b: vec![vec![5, 6], vec![], vec![7]],
            c: vec![],
            d: None
        }),
        None
    ],
    GROUP_OPTION_1_NESTED,
    GROUP_OPTION_1_NESTED_INODES
);
