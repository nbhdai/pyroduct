//! Struct-vs-value match for `src/wire/tests/group_option_3_nesting_u16.rs` — `Vec<..>` of a
//! derived inner struct (`Option<Inner>`) standing in for each group. See
//! `tests/value_match/primitive_u16.rs`'s module doc for the shared rationale.

use pyroformat::value::Time;
use pyroformat::wire::tests::group_option_3_nesting_u16::*;

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
struct GGroupOption3PrimitiveInner {
    a: Time,
    b: u8,
    c: Option<i32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption3Primitive {
    a: Vec<Vec<Vec<Option<GGroupOption3PrimitiveInner>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption3BlobInner {
    id: u32,
    s: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption3Blob {
    a: Vec<Vec<Vec<Option<GGroupOption3BlobInner>>>>,
}

value_match_group_test!(
    value_match_group_option_3_primitive,
    GGroupOption3Primitive,
    vec![
        vec![
            vec![
                Some(GGroupOption3PrimitiveInner {
                    a: Time(1_000_001_000),
                    b: 1,
                    c: Some(-1)
                }),
                Some(GGroupOption3PrimitiveInner {
                    a: Time(-2_000_002_000),
                    b: 2,
                    c: None
                })
            ],
            vec![None]
        ],
        vec![vec![
            Some(GGroupOption3PrimitiveInner {
                a: Time(4_000_004_000),
                b: 4,
                c: Some(4)
            }),
            Some(GGroupOption3PrimitiveInner {
                a: Time(-5_000_005_000),
                b: 5,
                c: None
            }),
            None
        ]]
    ],
    GROUP_OPTION_3_PRIMITIVE,
    GROUP_OPTION_3_PRIMITIVE_INODES
);

value_match_group_test!(
    value_match_group_option_3_blob,
    GGroupOption3Blob,
    vec![
        vec![
            vec![
                Some(GGroupOption3BlobInner {
                    id: 1,
                    s: "ab".to_string()
                }),
                Some(GGroupOption3BlobInner {
                    id: 2,
                    s: "".to_string()
                })
            ],
            vec![None]
        ],
        vec![vec![
            Some(GGroupOption3BlobInner {
                id: 4,
                s: "f".to_string()
            }),
            Some(GGroupOption3BlobInner {
                id: 5,
                s: "ghij".to_string()
            }),
            None
        ]]
    ],
    GROUP_OPTION_3_BLOB,
    GROUP_OPTION_3_BLOB_INODES
);

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption3MixedInner {
    a: bool,
    b: Option<f32>,
    c: String,
    d: u64,
    e: Option<i16>,
    f: [u8; 3],
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption3Mixed {
    a: Vec<Vec<Vec<Option<GGroupOption3MixedInner>>>>,
}

value_match_group_test!(
    value_match_group_option_3_mixed,
    GGroupOption3Mixed,
    vec![
        vec![
            vec![
                Some(GGroupOption3MixedInner {
                    a: true,
                    b: Some(1.5),
                    c: "ab".to_string(),
                    d: 1,
                    e: None,
                    f: [1, 2, 3]
                }),
                Some(GGroupOption3MixedInner {
                    a: false,
                    b: None,
                    c: "".to_string(),
                    d: 1099511627779,
                    e: Some(-2),
                    f: [4, 5, 6]
                })
            ],
            vec![None]
        ],
        vec![vec![
            Some(GGroupOption3MixedInner {
                a: true,
                b: Some(8.0),
                c: "é".to_string(),
                d: 4,
                e: None,
                f: [10, 11, 12]
            }),
            Some(GGroupOption3MixedInner {
                a: true,
                b: None,
                c: "fghij".to_string(),
                d: 18446744073709551615,
                e: Some(-5),
                f: [13, 14, 15]
            }),
            None
        ]]
    ],
    GROUP_OPTION_3_MIXED,
    GROUP_OPTION_3_MIXED_INODES
);
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption3NestedInner {
    a: u8,
    b: Vec<Vec<u64>>,
    c: Vec<u16>,
    d: Option<i32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroupOption3Nested {
    a: Vec<Vec<Vec<Option<GGroupOption3NestedInner>>>>,
}

value_match_group_test!(
    value_match_group_option_3_nested,
    GGroupOption3Nested,
    vec![
        vec![
            vec![
                Some(GGroupOption3NestedInner {
                    a: 1,
                    b: vec![vec![1], vec![2, 3]],
                    c: vec![1, 2, 3],
                    d: Some(-1)
                }),
                Some(GGroupOption3NestedInner {
                    a: 2,
                    b: vec![],
                    c: vec![],
                    d: None
                })
            ],
            vec![None]
        ],
        vec![vec![
            Some(GGroupOption3NestedInner {
                a: 4,
                b: vec![vec![4]],
                c: vec![5, 6],
                d: Some(-4)
            }),
            Some(GGroupOption3NestedInner {
                a: 5,
                b: vec![vec![5, 6], vec![], vec![7]],
                c: vec![],
                d: None
            }),
            None
        ]]
    ],
    GROUP_OPTION_3_NESTED,
    GROUP_OPTION_3_NESTED_INODES
);
