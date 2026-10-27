//! Struct-vs-value match for `src/wire/tests/group_5_nesting_u16.rs` — `Vec<..>` of a
//! derived inner struct standing in for each group. See
//! `tests/value_match/primitive_u16.rs`'s module doc for the shared rationale.

use pyroformat::wire::tests::group_5_nesting_u16::*;

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
struct GGroup5PrimitiveInner {
    a: bool,
    b: bool,
    c: i32,
    d: Option<bool>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup5Primitive {
    a: Vec<Vec<Vec<Vec<Vec<GGroup5PrimitiveInner>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup5BlobInner {
    s: String,
    t: Vec<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup5Blob {
    a: Vec<Vec<Vec<Vec<Vec<GGroup5BlobInner>>>>>,
}

value_match_group_test!(
    value_match_group_5_primitive,
    GGroup5Primitive,
    vec![
        vec![
            vec![
                vec![vec![
                    GGroup5PrimitiveInner {
                        a: true,
                        b: true,
                        c: -1,
                        d: Some(true)
                    },
                    GGroup5PrimitiveInner {
                        a: false,
                        b: false,
                        c: 2,
                        d: None
                    }
                ]],
                vec![vec![GGroup5PrimitiveInner {
                    a: true,
                    b: true,
                    c: -3,
                    d: Some(true)
                }]]
            ],
            vec![vec![vec![GGroup5PrimitiveInner {
                a: true,
                b: true,
                c: 4,
                d: Some(true)
            }]]]
        ],
        vec![vec![vec![vec![
            GGroup5PrimitiveInner {
                a: true,
                b: true,
                c: -5,
                d: None
            },
            GGroup5PrimitiveInner {
                a: false,
                b: false,
                c: 6,
                d: Some(false)
            }
        ]]]]
    ],
    GROUP_5_PRIMITIVE,
    GROUP_5_PRIMITIVE_INODES
);

value_match_group_test!(
    value_match_group_5_blob,
    GGroup5Blob,
    vec![
        vec![
            vec![
                vec![vec![
                    GGroup5BlobInner {
                        s: "ab".to_string(),
                        t: vec![1, 2]
                    },
                    GGroup5BlobInner {
                        s: "".to_string(),
                        t: vec![]
                    }
                ]],
                vec![vec![GGroup5BlobInner {
                    s: "cde".to_string(),
                    t: vec![3, 4, 5]
                }]]
            ],
            vec![vec![vec![GGroup5BlobInner {
                s: "f".to_string(),
                t: vec![255]
            }]]]
        ],
        vec![vec![vec![vec![
            GGroup5BlobInner {
                s: "ghij".to_string(),
                t: vec![0, 6, 7, 8]
            },
            GGroup5BlobInner {
                s: "é".to_string(),
                t: vec![9]
            }
        ]]]]
    ],
    GROUP_5_BLOB,
    GROUP_5_BLOB_INODES
);

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup5MixedInner {
    a: bool,
    b: Option<f32>,
    c: String,
    d: u64,
    e: Option<i16>,
    f: [u8; 3],
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup5Mixed {
    a: Vec<Vec<Vec<Vec<Vec<GGroup5MixedInner>>>>>,
}

value_match_group_test!(
    value_match_group_5_mixed,
    GGroup5Mixed,
    vec![
        vec![
            vec![
                vec![vec![
                    GGroup5MixedInner {
                        a: true,
                        b: Some(1.5),
                        c: "ab".to_string(),
                        d: 1,
                        e: None,
                        f: [1, 2, 3]
                    },
                    GGroup5MixedInner {
                        a: false,
                        b: None,
                        c: "".to_string(),
                        d: 1099511627779,
                        e: Some(-2),
                        f: [4, 5, 6]
                    }
                ]],
                vec![vec![GGroup5MixedInner {
                    a: false,
                    b: Some(-2.25),
                    c: "cde".to_string(),
                    d: 3,
                    e: Some(3),
                    f: [7, 8, 9]
                }]]
            ],
            vec![vec![vec![GGroup5MixedInner {
                a: true,
                b: Some(8.0),
                c: "é".to_string(),
                d: 4,
                e: None,
                f: [10, 11, 12]
            }]]]
        ],
        vec![vec![vec![vec![
            GGroup5MixedInner {
                a: true,
                b: None,
                c: "fghij".to_string(),
                d: 18446744073709551615,
                e: Some(-5),
                f: [13, 14, 15]
            },
            GGroup5MixedInner {
                a: false,
                b: Some(0.5),
                c: "k".to_string(),
                d: 6,
                e: Some(6),
                f: [16, 17, 18]
            }
        ]]]]
    ],
    GROUP_5_MIXED,
    GROUP_5_MIXED_INODES
);
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup5NestedInner {
    a: u8,
    b: Vec<Vec<u64>>,
    c: Vec<u16>,
    d: Option<i32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup5Nested {
    a: Vec<Vec<Vec<Vec<Vec<GGroup5NestedInner>>>>>,
}

value_match_group_test!(
    value_match_group_5_nested,
    GGroup5Nested,
    vec![
        vec![
            vec![
                vec![vec![
                    GGroup5NestedInner {
                        a: 1,
                        b: vec![vec![1], vec![2, 3]],
                        c: vec![1, 2, 3],
                        d: Some(-1)
                    },
                    GGroup5NestedInner {
                        a: 2,
                        b: vec![],
                        c: vec![],
                        d: None
                    }
                ]],
                vec![vec![GGroup5NestedInner {
                    a: 3,
                    b: vec![vec![]],
                    c: vec![4],
                    d: Some(3)
                }]]
            ],
            vec![vec![vec![GGroup5NestedInner {
                a: 4,
                b: vec![vec![4]],
                c: vec![5, 6],
                d: Some(-4)
            }]]]
        ],
        vec![vec![vec![vec![
            GGroup5NestedInner {
                a: 5,
                b: vec![vec![5, 6], vec![], vec![7]],
                c: vec![],
                d: None
            },
            GGroup5NestedInner {
                a: 6,
                b: vec![vec![8]],
                c: vec![7],
                d: Some(6)
            }
        ]]]]
    ],
    GROUP_5_NESTED,
    GROUP_5_NESTED_INODES
);
