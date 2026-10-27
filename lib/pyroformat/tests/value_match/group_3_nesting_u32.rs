//! Struct-vs-value match for `src/wire/tests/group_3_nesting_u32.rs` — `Vec<..>` of a
//! derived inner struct standing in for each group. See
//! `tests/value_match/primitive_u32.rs`'s module doc for the shared rationale.

use pyroformat::value::Time;
use pyroformat::wire::tests::group_3_nesting_u32::*;

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
struct GGroup3PrimitiveInner {
    a: Time,
    b: u8,
    c: Option<i32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup3Primitive {
    a: Vec<Vec<Vec<GGroup3PrimitiveInner>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup3BlobInner {
    id: u32,
    s: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup3Blob {
    a: Vec<Vec<Vec<GGroup3BlobInner>>>,
}

value_match_group_test!(
    value_match_group_3_primitive,
    GGroup3Primitive,
    vec![
        vec![
            vec![
                GGroup3PrimitiveInner {
                    a: Time(1_000_001_000),
                    b: 1,
                    c: Some(-1)
                },
                GGroup3PrimitiveInner {
                    a: Time(-2_000_002_000),
                    b: 2,
                    c: None
                }
            ],
            vec![GGroup3PrimitiveInner {
                a: Time(3_500_003_000),
                b: 3,
                c: Some(-3)
            }]
        ],
        vec![vec![
            GGroup3PrimitiveInner {
                a: Time(4_000_004_000),
                b: 4,
                c: Some(4)
            },
            GGroup3PrimitiveInner {
                a: Time(-5_000_005_000),
                b: 5,
                c: None
            },
            GGroup3PrimitiveInner {
                a: Time(1_700_000_000_000_006_000),
                b: 6,
                c: Some(6)
            }
        ]]
    ],
    GROUP_3_PRIMITIVE,
    GROUP_3_PRIMITIVE_INODES
);

value_match_group_test!(
    value_match_group_3_blob,
    GGroup3Blob,
    vec![
        vec![
            vec![
                GGroup3BlobInner {
                    id: 1,
                    s: "ab".to_string()
                },
                GGroup3BlobInner {
                    id: 2,
                    s: "".to_string()
                }
            ],
            vec![GGroup3BlobInner {
                id: 3,
                s: "cde".to_string()
            }]
        ],
        vec![vec![
            GGroup3BlobInner {
                id: 4,
                s: "f".to_string()
            },
            GGroup3BlobInner {
                id: 5,
                s: "ghij".to_string()
            },
            GGroup3BlobInner {
                id: 6,
                s: "é".to_string()
            }
        ]]
    ],
    GROUP_3_BLOB,
    GROUP_3_BLOB_INODES
);

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup3MixedInner {
    a: bool,
    b: Option<f32>,
    c: String,
    d: u64,
    e: Option<i16>,
    f: [u8; 3],
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup3Mixed {
    a: Vec<Vec<Vec<GGroup3MixedInner>>>,
}

value_match_group_test!(
    value_match_group_3_mixed,
    GGroup3Mixed,
    vec![
        vec![
            vec![
                GGroup3MixedInner {
                    a: true,
                    b: Some(1.5),
                    c: "ab".to_string(),
                    d: 1,
                    e: None,
                    f: [1, 2, 3]
                },
                GGroup3MixedInner {
                    a: false,
                    b: None,
                    c: "".to_string(),
                    d: 1099511627779,
                    e: Some(-2),
                    f: [4, 5, 6]
                }
            ],
            vec![GGroup3MixedInner {
                a: false,
                b: Some(-2.25),
                c: "cde".to_string(),
                d: 3,
                e: Some(3),
                f: [7, 8, 9]
            }]
        ],
        vec![vec![
            GGroup3MixedInner {
                a: true,
                b: Some(8.0),
                c: "é".to_string(),
                d: 4,
                e: None,
                f: [10, 11, 12]
            },
            GGroup3MixedInner {
                a: true,
                b: None,
                c: "fghij".to_string(),
                d: 18446744073709551615,
                e: Some(-5),
                f: [13, 14, 15]
            },
            GGroup3MixedInner {
                a: false,
                b: Some(0.5),
                c: "k".to_string(),
                d: 6,
                e: Some(6),
                f: [16, 17, 18]
            }
        ]]
    ],
    GROUP_3_MIXED,
    GROUP_3_MIXED_INODES
);
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup3NestedInner {
    a: u8,
    b: Vec<Vec<u64>>,
    c: Vec<u16>,
    d: Option<i32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup3Nested {
    a: Vec<Vec<Vec<GGroup3NestedInner>>>,
}

value_match_group_test!(
    value_match_group_3_nested,
    GGroup3Nested,
    vec![
        vec![
            vec![
                GGroup3NestedInner {
                    a: 1,
                    b: vec![vec![1], vec![2, 3]],
                    c: vec![1, 2, 3],
                    d: Some(-1)
                },
                GGroup3NestedInner {
                    a: 2,
                    b: vec![],
                    c: vec![],
                    d: None
                }
            ],
            vec![GGroup3NestedInner {
                a: 3,
                b: vec![vec![]],
                c: vec![4],
                d: Some(3)
            }]
        ],
        vec![vec![
            GGroup3NestedInner {
                a: 4,
                b: vec![vec![4]],
                c: vec![5, 6],
                d: Some(-4)
            },
            GGroup3NestedInner {
                a: 5,
                b: vec![vec![5, 6], vec![], vec![7]],
                c: vec![],
                d: None
            },
            GGroup3NestedInner {
                a: 6,
                b: vec![vec![8]],
                c: vec![7],
                d: Some(6)
            }
        ]]
    ],
    GROUP_3_NESTED,
    GROUP_3_NESTED_INODES
);
