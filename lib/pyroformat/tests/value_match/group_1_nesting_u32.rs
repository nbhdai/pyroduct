//! Struct-vs-value match for `src/wire/tests/group_1_nesting_u32.rs` — `Vec<..>` of a
//! derived inner struct standing in for each group. See
//! `tests/value_match/primitive_u32.rs`'s module doc for the shared rationale.

use pyroformat::wire::tests::group_1_nesting_u32::*;

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
struct GGroup1PrimitiveInner {
    a: u8,
    b: u32,
    c: Option<u16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup1Primitive {
    a: Vec<GGroup1PrimitiveInner>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup1BlobInner {
    s: String,
    n: u16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup1Blob {
    a: Vec<GGroup1BlobInner>,
}

value_match_group_test!(
    value_match_group_1_primitive,
    GGroup1Primitive,
    vec![
        GGroup1PrimitiveInner {
            a: 1,
            b: 1,
            c: Some(1)
        },
        GGroup1PrimitiveInner {
            a: 2,
            b: 2,
            c: None
        },
        GGroup1PrimitiveInner {
            a: 3,
            b: 3,
            c: Some(3)
        },
        GGroup1PrimitiveInner {
            a: 4,
            b: 4,
            c: Some(4)
        },
        GGroup1PrimitiveInner {
            a: 5,
            b: 5,
            c: None
        },
        GGroup1PrimitiveInner {
            a: 6,
            b: 6,
            c: Some(6)
        }
    ],
    GROUP_1_PRIMITIVE,
    GROUP_1_PRIMITIVE_INODES
);

value_match_group_test!(
    value_match_group_1_blob,
    GGroup1Blob,
    vec![
        GGroup1BlobInner {
            s: "ab".to_string(),
            n: 1
        },
        GGroup1BlobInner {
            s: "".to_string(),
            n: 2
        },
        GGroup1BlobInner {
            s: "cde".to_string(),
            n: 3
        },
        GGroup1BlobInner {
            s: "f".to_string(),
            n: 4
        },
        GGroup1BlobInner {
            s: "ghij".to_string(),
            n: 5
        },
        GGroup1BlobInner {
            s: "é".to_string(),
            n: 6
        }
    ],
    GROUP_1_BLOB,
    GROUP_1_BLOB_INODES
);

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup1MixedInner {
    a: bool,
    b: Option<f32>,
    c: String,
    d: u64,
    e: Option<i16>,
    f: [u8; 3],
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup1Mixed {
    a: Vec<GGroup1MixedInner>,
}

value_match_group_test!(
    value_match_group_1_mixed,
    GGroup1Mixed,
    vec![
        GGroup1MixedInner {
            a: true,
            b: Some(1.5),
            c: "ab".to_string(),
            d: 1,
            e: None,
            f: [1, 2, 3]
        },
        GGroup1MixedInner {
            a: false,
            b: None,
            c: "".to_string(),
            d: 1099511627779,
            e: Some(-2),
            f: [4, 5, 6]
        },
        GGroup1MixedInner {
            a: false,
            b: Some(-2.25),
            c: "cde".to_string(),
            d: 3,
            e: Some(3),
            f: [7, 8, 9]
        },
        GGroup1MixedInner {
            a: true,
            b: Some(8.0),
            c: "é".to_string(),
            d: 4,
            e: None,
            f: [10, 11, 12]
        },
        GGroup1MixedInner {
            a: true,
            b: None,
            c: "fghij".to_string(),
            d: 18446744073709551615,
            e: Some(-5),
            f: [13, 14, 15]
        },
        GGroup1MixedInner {
            a: false,
            b: Some(0.5),
            c: "k".to_string(),
            d: 6,
            e: Some(6),
            f: [16, 17, 18]
        }
    ],
    GROUP_1_MIXED,
    GROUP_1_MIXED_INODES
);
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup1NestedInner {
    a: u8,
    b: Vec<Vec<u64>>,
    c: Vec<u16>,
    d: Option<i32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GGroup1Nested {
    a: Vec<GGroup1NestedInner>,
}

value_match_group_test!(
    value_match_group_1_nested,
    GGroup1Nested,
    vec![
        GGroup1NestedInner {
            a: 1,
            b: vec![vec![1], vec![2, 3]],
            c: vec![1, 2, 3],
            d: Some(-1)
        },
        GGroup1NestedInner {
            a: 2,
            b: vec![],
            c: vec![],
            d: None
        },
        GGroup1NestedInner {
            a: 3,
            b: vec![vec![]],
            c: vec![4],
            d: Some(3)
        },
        GGroup1NestedInner {
            a: 4,
            b: vec![vec![4]],
            c: vec![5, 6],
            d: Some(-4)
        },
        GGroup1NestedInner {
            a: 5,
            b: vec![vec![5, 6], vec![], vec![7]],
            c: vec![],
            d: None
        },
        GGroup1NestedInner {
            a: 6,
            b: vec![vec![8]],
            c: vec![7],
            d: Some(6)
        }
    ],
    GROUP_1_NESTED,
    GROUP_1_NESTED_INODES
);
