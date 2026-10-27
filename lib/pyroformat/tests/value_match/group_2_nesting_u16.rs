//! Struct-vs-value match for `src/wire/tests/group_2_nesting_u16.rs` — `Vec<..>` of a
//! derived inner struct standing in for each group. See
//! `tests/value_match/primitive_u16.rs`'s module doc for the shared rationale.

use pyroformat::wire::tests::group_2_nesting_u16::*;

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
struct GGroup2PrimitiveInner {
    a: i16,
    b: bool,
    c: f64,
    d: Option<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup2Primitive {
    a: Vec<Vec<GGroup2PrimitiveInner>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup2BlobInner {
    b: Vec<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup2Blob {
    a: Vec<Vec<GGroup2BlobInner>>,
}

value_match_group_test!(
    value_match_group_2_primitive,
    GGroup2Primitive,
    vec![
        vec![
            GGroup2PrimitiveInner {
                a: -1,
                b: true,
                c: 1.5,
                d: Some(1)
            },
            GGroup2PrimitiveInner {
                a: 2,
                b: false,
                c: -2.0,
                d: None
            }
        ],
        vec![GGroup2PrimitiveInner {
            a: -3,
            b: true,
            c: 3.25,
            d: Some(3)
        }],
        vec![
            GGroup2PrimitiveInner {
                a: 4,
                b: true,
                c: 4.0,
                d: Some(4)
            },
            GGroup2PrimitiveInner {
                a: -5,
                b: true,
                c: -5.5,
                d: None
            },
            GGroup2PrimitiveInner {
                a: 6,
                b: false,
                c: 6.0,
                d: Some(6)
            }
        ]
    ],
    GROUP_2_PRIMITIVE,
    GROUP_2_PRIMITIVE_INODES
);

value_match_group_test!(
    value_match_group_2_blob,
    GGroup2Blob,
    vec![
        vec![
            GGroup2BlobInner { b: vec![1, 2] },
            GGroup2BlobInner { b: vec![] }
        ],
        vec![GGroup2BlobInner { b: vec![3, 4, 5] }],
        vec![
            GGroup2BlobInner { b: vec![255] },
            GGroup2BlobInner {
                b: vec![0, 6, 7, 8]
            },
            GGroup2BlobInner { b: vec![9] }
        ]
    ],
    GROUP_2_BLOB,
    GROUP_2_BLOB_INODES
);

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup2MixedInner {
    a: bool,
    b: Option<f32>,
    c: String,
    d: u64,
    e: Option<i16>,
    f: [u8; 3],
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup2Mixed {
    a: Vec<Vec<GGroup2MixedInner>>,
}

value_match_group_test!(
    value_match_group_2_mixed,
    GGroup2Mixed,
    vec![
        vec![
            GGroup2MixedInner {
                a: true,
                b: Some(1.5),
                c: "ab".to_string(),
                d: 1,
                e: None,
                f: [1, 2, 3]
            },
            GGroup2MixedInner {
                a: false,
                b: None,
                c: "".to_string(),
                d: 1099511627779,
                e: Some(-2),
                f: [4, 5, 6]
            }
        ],
        vec![GGroup2MixedInner {
            a: false,
            b: Some(-2.25),
            c: "cde".to_string(),
            d: 3,
            e: Some(3),
            f: [7, 8, 9]
        }],
        vec![
            GGroup2MixedInner {
                a: true,
                b: Some(8.0),
                c: "é".to_string(),
                d: 4,
                e: None,
                f: [10, 11, 12]
            },
            GGroup2MixedInner {
                a: true,
                b: None,
                c: "fghij".to_string(),
                d: 18446744073709551615,
                e: Some(-5),
                f: [13, 14, 15]
            },
            GGroup2MixedInner {
                a: false,
                b: Some(0.5),
                c: "k".to_string(),
                d: 6,
                e: Some(6),
                f: [16, 17, 18]
            }
        ]
    ],
    GROUP_2_MIXED,
    GROUP_2_MIXED_INODES
);
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup2NestedInner {
    a: u8,
    b: Vec<Vec<u64>>,
    c: Vec<u16>,
    d: Option<i32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GGroup2Nested {
    a: Vec<Vec<GGroup2NestedInner>>,
}

value_match_group_test!(
    value_match_group_2_nested,
    GGroup2Nested,
    vec![
        vec![
            GGroup2NestedInner {
                a: 1,
                b: vec![vec![1], vec![2, 3]],
                c: vec![1, 2, 3],
                d: Some(-1)
            },
            GGroup2NestedInner {
                a: 2,
                b: vec![],
                c: vec![],
                d: None
            }
        ],
        vec![GGroup2NestedInner {
            a: 3,
            b: vec![vec![]],
            c: vec![4],
            d: Some(3)
        }],
        vec![
            GGroup2NestedInner {
                a: 4,
                b: vec![vec![4]],
                c: vec![5, 6],
                d: Some(-4)
            },
            GGroup2NestedInner {
                a: 5,
                b: vec![vec![5, 6], vec![], vec![7]],
                c: vec![],
                d: None
            },
            GGroup2NestedInner {
                a: 6,
                b: vec![vec![8]],
                c: vec![7],
                d: Some(6)
            }
        ]
    ],
    GROUP_2_NESTED,
    GROUP_2_NESTED_INODES
);
