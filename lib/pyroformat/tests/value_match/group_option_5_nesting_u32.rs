//! Struct-vs-value match for `src/wire/tests/group_option_5_nesting_u32.rs` — `Vec<..>` of a
//! derived inner struct (`Option<Inner>`) standing in for each group. See
//! `tests/value_match/primitive_u32.rs`'s module doc for the shared rationale.

use pyroformat::wire::tests::group_option_5_nesting_u32::*;

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
#[pyro(Word = u32)]
struct GGroupOption5PrimitiveInner {
    a: bool,
    b: bool,
    c: i32,
    d: Option<bool>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption5Primitive {
    a: Vec<Vec<Vec<Vec<Vec<Option<GGroupOption5PrimitiveInner>>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption5BlobInner {
    s: String,
    t: Vec<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption5Blob {
    a: Vec<Vec<Vec<Vec<Vec<Option<GGroupOption5BlobInner>>>>>>,
}

value_match_group_test!(
    value_match_group_option_5_primitive,
    GGroupOption5Primitive,
    vec![
        vec![
            vec![
                vec![vec![
                    Some(GGroupOption5PrimitiveInner {
                        a: true,
                        b: true,
                        c: -1,
                        d: Some(true)
                    }),
                    Some(GGroupOption5PrimitiveInner {
                        a: false,
                        b: false,
                        c: 2,
                        d: None
                    })
                ]],
                vec![vec![None]]
            ],
            vec![vec![vec![Some(GGroupOption5PrimitiveInner {
                a: true,
                b: true,
                c: 4,
                d: Some(true)
            })]]]
        ],
        vec![vec![vec![vec![
            Some(GGroupOption5PrimitiveInner {
                a: true,
                b: true,
                c: -5,
                d: None
            }),
            None
        ]]]]
    ],
    GROUP_OPTION_5_PRIMITIVE,
    GROUP_OPTION_5_PRIMITIVE_INODES
);

value_match_group_test!(
    value_match_group_option_5_blob,
    GGroupOption5Blob,
    vec![
        vec![
            vec![
                vec![vec![
                    Some(GGroupOption5BlobInner {
                        s: "ab".to_string(),
                        t: vec![1, 2]
                    }),
                    Some(GGroupOption5BlobInner {
                        s: "".to_string(),
                        t: vec![]
                    })
                ]],
                vec![vec![None]]
            ],
            vec![vec![vec![Some(GGroupOption5BlobInner {
                s: "f".to_string(),
                t: vec![255]
            })]]]
        ],
        vec![vec![vec![vec![
            Some(GGroupOption5BlobInner {
                s: "ghij".to_string(),
                t: vec![0, 6, 7, 8]
            }),
            None
        ]]]]
    ],
    GROUP_OPTION_5_BLOB,
    GROUP_OPTION_5_BLOB_INODES
);

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption5MixedInner {
    a: bool,
    b: Option<f32>,
    c: String,
    d: u64,
    e: Option<i16>,
    f: [u8; 3],
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption5Mixed {
    a: Vec<Vec<Vec<Vec<Vec<Option<GGroupOption5MixedInner>>>>>>,
}

value_match_group_test!(
    value_match_group_option_5_mixed,
    GGroupOption5Mixed,
    vec![
        vec![
            vec![
                vec![vec![
                    Some(GGroupOption5MixedInner {
                        a: true,
                        b: Some(1.5),
                        c: "ab".to_string(),
                        d: 1,
                        e: None,
                        f: [1, 2, 3]
                    }),
                    Some(GGroupOption5MixedInner {
                        a: false,
                        b: None,
                        c: "".to_string(),
                        d: 1099511627779,
                        e: Some(-2),
                        f: [4, 5, 6]
                    })
                ]],
                vec![vec![None]]
            ],
            vec![vec![vec![Some(GGroupOption5MixedInner {
                a: true,
                b: Some(8.0),
                c: "é".to_string(),
                d: 4,
                e: None,
                f: [10, 11, 12]
            })]]]
        ],
        vec![vec![vec![vec![
            Some(GGroupOption5MixedInner {
                a: true,
                b: None,
                c: "fghij".to_string(),
                d: 18446744073709551615,
                e: Some(-5),
                f: [13, 14, 15]
            }),
            None
        ]]]]
    ],
    GROUP_OPTION_5_MIXED,
    GROUP_OPTION_5_MIXED_INODES
);
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption5NestedInner {
    a: u8,
    b: Vec<Vec<u64>>,
    c: Vec<u16>,
    d: Option<i32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption5Nested {
    a: Vec<Vec<Vec<Vec<Vec<Option<GGroupOption5NestedInner>>>>>>,
}

value_match_group_test!(
    value_match_group_option_5_nested,
    GGroupOption5Nested,
    vec![
        vec![
            vec![
                vec![vec![
                    Some(GGroupOption5NestedInner {
                        a: 1,
                        b: vec![vec![1], vec![2, 3]],
                        c: vec![1, 2, 3],
                        d: Some(-1)
                    }),
                    Some(GGroupOption5NestedInner {
                        a: 2,
                        b: vec![],
                        c: vec![],
                        d: None
                    })
                ]],
                vec![vec![None]]
            ],
            vec![vec![vec![Some(GGroupOption5NestedInner {
                a: 4,
                b: vec![vec![4]],
                c: vec![5, 6],
                d: Some(-4)
            })]]]
        ],
        vec![vec![vec![vec![
            Some(GGroupOption5NestedInner {
                a: 5,
                b: vec![vec![5, 6], vec![], vec![7]],
                c: vec![],
                d: None
            }),
            None
        ]]]]
    ],
    GROUP_OPTION_5_NESTED,
    GROUP_OPTION_5_NESTED_INODES
);
