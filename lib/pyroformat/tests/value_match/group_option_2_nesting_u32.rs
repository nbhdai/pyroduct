//! Struct-vs-value match for `src/wire/tests/group_option_2_nesting_u32.rs` — `Vec<..>` of a
//! derived inner struct (`Option<Inner>`) standing in for each group. See
//! `tests/value_match/primitive_u32.rs`'s module doc for the shared rationale.

use pyroformat::wire::tests::group_option_2_nesting_u32::*;

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
struct GGroupOption2PrimitiveInner {
    a: i16,
    b: bool,
    c: f64,
    d: Option<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption2Primitive {
    a: Vec<Vec<Option<GGroupOption2PrimitiveInner>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption2BlobInner {
    b: Vec<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption2Blob {
    a: Vec<Vec<Option<GGroupOption2BlobInner>>>,
}

value_match_group_test!(
    value_match_group_option_2_primitive,
    GGroupOption2Primitive,
    vec![
        vec![
            Some(GGroupOption2PrimitiveInner {
                a: -1,
                b: true,
                c: 1.5,
                d: Some(1)
            }),
            Some(GGroupOption2PrimitiveInner {
                a: 2,
                b: false,
                c: -2.0,
                d: None
            })
        ],
        vec![None],
        vec![
            Some(GGroupOption2PrimitiveInner {
                a: 4,
                b: true,
                c: 4.0,
                d: Some(4)
            }),
            Some(GGroupOption2PrimitiveInner {
                a: -5,
                b: true,
                c: -5.5,
                d: None
            }),
            None
        ]
    ],
    GROUP_OPTION_2_PRIMITIVE,
    GROUP_OPTION_2_PRIMITIVE_INODES
);

value_match_group_test!(
    value_match_group_option_2_blob,
    GGroupOption2Blob,
    vec![
        vec![
            Some(GGroupOption2BlobInner { b: vec![1, 2] }),
            Some(GGroupOption2BlobInner { b: vec![] })
        ],
        vec![None],
        vec![
            Some(GGroupOption2BlobInner { b: vec![255] }),
            Some(GGroupOption2BlobInner {
                b: vec![0, 6, 7, 8]
            }),
            None
        ]
    ],
    GROUP_OPTION_2_BLOB,
    GROUP_OPTION_2_BLOB_INODES
);

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption2MixedInner {
    a: bool,
    b: Option<f32>,
    c: String,
    d: u64,
    e: Option<i16>,
    f: [u8; 3],
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption2Mixed {
    a: Vec<Vec<Option<GGroupOption2MixedInner>>>,
}

value_match_group_test!(
    value_match_group_option_2_mixed,
    GGroupOption2Mixed,
    vec![
        vec![
            Some(GGroupOption2MixedInner {
                a: true,
                b: Some(1.5),
                c: "ab".to_string(),
                d: 1,
                e: None,
                f: [1, 2, 3]
            }),
            Some(GGroupOption2MixedInner {
                a: false,
                b: None,
                c: "".to_string(),
                d: 1099511627779,
                e: Some(-2),
                f: [4, 5, 6]
            })
        ],
        vec![None],
        vec![
            Some(GGroupOption2MixedInner {
                a: true,
                b: Some(8.0),
                c: "é".to_string(),
                d: 4,
                e: None,
                f: [10, 11, 12]
            }),
            Some(GGroupOption2MixedInner {
                a: true,
                b: None,
                c: "fghij".to_string(),
                d: 18446744073709551615,
                e: Some(-5),
                f: [13, 14, 15]
            }),
            None
        ]
    ],
    GROUP_OPTION_2_MIXED,
    GROUP_OPTION_2_MIXED_INODES
);
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption2NestedInner {
    a: u8,
    b: Vec<Vec<u64>>,
    c: Vec<u16>,
    d: Option<i32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroupOption2Nested {
    a: Vec<Vec<Option<GGroupOption2NestedInner>>>,
}

value_match_group_test!(
    value_match_group_option_2_nested,
    GGroupOption2Nested,
    vec![
        vec![
            Some(GGroupOption2NestedInner {
                a: 1,
                b: vec![vec![1], vec![2, 3]],
                c: vec![1, 2, 3],
                d: Some(-1)
            }),
            Some(GGroupOption2NestedInner {
                a: 2,
                b: vec![],
                c: vec![],
                d: None
            })
        ],
        vec![None],
        vec![
            Some(GGroupOption2NestedInner {
                a: 4,
                b: vec![vec![4]],
                c: vec![5, 6],
                d: Some(-4)
            }),
            Some(GGroupOption2NestedInner {
                a: 5,
                b: vec![vec![5, 6], vec![], vec![7]],
                c: vec![],
                d: None
            }),
            None
        ]
    ],
    GROUP_OPTION_2_NESTED,
    GROUP_OPTION_2_NESTED_INODES
);
