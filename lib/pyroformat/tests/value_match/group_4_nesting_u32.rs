//! Struct-vs-value match for `src/wire/tests/group_4_nesting_u32.rs` — `Vec<..>` of a
//! derived inner struct standing in for each group. See
//! `tests/value_match/primitive_u32.rs`'s module doc for the shared rationale.

use pyroformat::wire::tests::group_4_nesting_u32::*;

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
struct GGroup4PrimitiveInner {
    a: f32,
    b: i64,
    c: u16,
    d: Option<f64>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup4Primitive {
    a: Vec<Vec<Vec<Vec<GGroup4PrimitiveInner>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup4BlobInner {
    b: Vec<u8>,
    f: f64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup4Blob {
    a: Vec<Vec<Vec<Vec<GGroup4BlobInner>>>>,
}

value_match_group_test!(
    value_match_group_4_primitive,
    GGroup4Primitive,
    vec![
        vec![
            vec![
                vec![
                    GGroup4PrimitiveInner {
                        a: 1.5,
                        b: -1,
                        c: 1,
                        d: Some(1.5)
                    },
                    GGroup4PrimitiveInner {
                        a: -2.0,
                        b: 2,
                        c: 2,
                        d: None
                    }
                ],
                vec![GGroup4PrimitiveInner {
                    a: 3.25,
                    b: -3,
                    c: 3,
                    d: Some(3.25)
                }]
            ],
            vec![vec![GGroup4PrimitiveInner {
                a: 4.0,
                b: 4,
                c: 4,
                d: Some(4.0)
            }]]
        ],
        vec![vec![vec![
            GGroup4PrimitiveInner {
                a: -5.5,
                b: -5,
                c: 5,
                d: None
            },
            GGroup4PrimitiveInner {
                a: 6.0,
                b: 6,
                c: 6,
                d: Some(6.0)
            }
        ]]]
    ],
    GROUP_4_PRIMITIVE,
    GROUP_4_PRIMITIVE_INODES
);

value_match_group_test!(
    value_match_group_4_blob,
    GGroup4Blob,
    vec![
        vec![
            vec![
                vec![
                    GGroup4BlobInner {
                        b: vec![1, 2],
                        f: 1.5
                    },
                    GGroup4BlobInner { b: vec![], f: -2.0 }
                ],
                vec![GGroup4BlobInner {
                    b: vec![3, 4, 5],
                    f: 3.25
                }]
            ],
            vec![vec![GGroup4BlobInner {
                b: vec![255],
                f: 4.0
            }]]
        ],
        vec![vec![vec![
            GGroup4BlobInner {
                b: vec![0, 6, 7, 8],
                f: -5.5
            },
            GGroup4BlobInner { b: vec![9], f: 6.0 }
        ]]]
    ],
    GROUP_4_BLOB,
    GROUP_4_BLOB_INODES
);

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup4MixedInner {
    a: bool,
    b: Option<f32>,
    c: String,
    d: u64,
    e: Option<i16>,
    f: [u8; 3],
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup4Mixed {
    a: Vec<Vec<Vec<Vec<GGroup4MixedInner>>>>,
}

value_match_group_test!(
    value_match_group_4_mixed,
    GGroup4Mixed,
    vec![
        vec![
            vec![
                vec![
                    GGroup4MixedInner {
                        a: true,
                        b: Some(1.5),
                        c: "ab".to_string(),
                        d: 1,
                        e: None,
                        f: [1, 2, 3]
                    },
                    GGroup4MixedInner {
                        a: false,
                        b: None,
                        c: "".to_string(),
                        d: 1099511627779,
                        e: Some(-2),
                        f: [4, 5, 6]
                    }
                ],
                vec![GGroup4MixedInner {
                    a: false,
                    b: Some(-2.25),
                    c: "cde".to_string(),
                    d: 3,
                    e: Some(3),
                    f: [7, 8, 9]
                }]
            ],
            vec![vec![GGroup4MixedInner {
                a: true,
                b: Some(8.0),
                c: "é".to_string(),
                d: 4,
                e: None,
                f: [10, 11, 12]
            }]]
        ],
        vec![vec![vec![
            GGroup4MixedInner {
                a: true,
                b: None,
                c: "fghij".to_string(),
                d: 18446744073709551615,
                e: Some(-5),
                f: [13, 14, 15]
            },
            GGroup4MixedInner {
                a: false,
                b: Some(0.5),
                c: "k".to_string(),
                d: 6,
                e: Some(6),
                f: [16, 17, 18]
            }
        ]]]
    ],
    GROUP_4_MIXED,
    GROUP_4_MIXED_INODES
);
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup4NestedInner {
    a: u8,
    b: Vec<Vec<u64>>,
    c: Vec<u16>,
    d: Option<i32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup4Nested {
    a: Vec<Vec<Vec<Vec<GGroup4NestedInner>>>>,
}

value_match_group_test!(
    value_match_group_4_nested,
    GGroup4Nested,
    vec![
        vec![
            vec![
                vec![
                    GGroup4NestedInner {
                        a: 1,
                        b: vec![vec![1], vec![2, 3]],
                        c: vec![1, 2, 3],
                        d: Some(-1)
                    },
                    GGroup4NestedInner {
                        a: 2,
                        b: vec![],
                        c: vec![],
                        d: None
                    }
                ],
                vec![GGroup4NestedInner {
                    a: 3,
                    b: vec![vec![]],
                    c: vec![4],
                    d: Some(3)
                }]
            ],
            vec![vec![GGroup4NestedInner {
                a: 4,
                b: vec![vec![4]],
                c: vec![5, 6],
                d: Some(-4)
            }]]
        ],
        vec![vec![vec![
            GGroup4NestedInner {
                a: 5,
                b: vec![vec![5, 6], vec![], vec![7]],
                c: vec![],
                d: None
            },
            GGroup4NestedInner {
                a: 6,
                b: vec![vec![8]],
                c: vec![7],
                d: Some(6)
            }
        ]]]
    ],
    GROUP_4_NESTED,
    GROUP_4_NESTED_INODES
);
