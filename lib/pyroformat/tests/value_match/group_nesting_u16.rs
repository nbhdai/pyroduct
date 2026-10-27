//! Struct-vs-value match for `src/wire/tests/group_nesting_u16.rs` — hybrid
//! group-inside-list and list-inside-group shapes. See
//! `tests/value_match/primitive_u16.rs`'s module doc for the shared rationale.
//!
//! Per the "how Group maps to a struct field" unlock: a bare, non-`Vec`
//! nested-struct field is `PyroType::Group`, `Vec<Inner>` is
//! `List<Group>`. Each case below reproduces the source schema's `Group`/
//! `List` nesting (and field names) as a chain of derive structs.

use pyroformat::wire::tests::group_nesting_u16::*;

use crate::support::read_back::assert_roundtrip;
use pyroformat::{Layout, Reader, Writer};

macro_rules! value_match_group_nesting_test {
    ($test_name:ident, $struct_ty:ident, $field:expr, $buf_const:expr, $inodes_const:expr) => {
        #[test]
        fn $test_name() {
            let data = $struct_ty { a: $field };
            assert_roundtrip(&data, &$buf_const, &$inodes_const);
        }
    };
}

// ── `List<Group{y: u8}>` ──────────────────────────────────────────────

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GYInner {
    y: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GListGroupY {
    a: Vec<GYInner>,
}

// ── `List<List<Group{a: Str, b: List<Group{c: Str}>}>>` ──────────────

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GNestC {
    c: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GNestOuter {
    a: String,
    b: Vec<GNestC>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GDoubleListGroupStr {
    a: Vec<Vec<GNestOuter>>,
}

// ── `List<Group{b1: List<List<f64>>, b2: List<List<i64>>}>` ─────────

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GLgF64I64Inner {
    b1: Vec<Vec<f64>>,
    b2: Vec<Vec<i64>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GListGroupF64I64 {
    a: Vec<GLgF64I64Inner>,
}

// ── `List<Group{b: Group{c: Group{d: u32}}}>` ────────────────────────

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GTripleD {
    d: u32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GTripleC {
    c: GTripleD,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GTripleB {
    b: GTripleC,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GListGroupTripleU32 {
    a: Vec<GTripleB>,
}

// ── `List<Group{a: Bytes, b: List<Bytes>}>` ──────────────────────────

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GBytesInner {
    a: Vec<u8>,
    b: Vec<Vec<u8>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GListGroupBytes {
    a: Vec<GBytesInner>,
}

// ── `List<Group{a: [u8; 16], b: List<[u16; 8]>}>` ────────────────────

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GFixedInner {
    a: [u8; 16],
    b: Vec<[u16; 8]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GListGroupFixedArrays {
    a: Vec<GFixedInner>,
}

// ── `List<List<Group{a: u8, b: u8}>>` ────────────────────────────────

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GTwoU8Inner {
    a: u8,
    b: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GDoubleListGroupTwoU8 {
    a: Vec<Vec<GTwoU8Inner>>,
}

// ── `List<Group{b: List<Group{c: List<List<u32>>}>}>` ───────────────

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GTn7C {
    c: Vec<Vec<u32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GTn7B {
    b: Vec<GTn7C>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GTripleNestedListGroupU32 {
    a: Vec<GTn7B>,
}

// ── `List<Group{a: bool, b: List<List<bool>>}>` ──────────────────────

// `b: Vec<Vec<bool>>` is nested (vec_depth 2), not the plain `Vec<bool>`
// the `#[pyro(allow(vec_bool))]` deprecation-suppression attribute
// targets (`pyroformat-derive/src/parse/structs.rs`'s `is_plain_vec_bool`
// check rejects it on this field) — and it isn't needed here since that
// warning only fires for the plain depth-1 shape anyway.
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GBoolInner {
    a: bool,
    #[pyro(allow(vec_bool))]
    b: Vec<Vec<bool>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GListGroupBool {
    a: Vec<GBoolInner>,
}

// ── `List<Group{a: Str, b: Str}>` ─────────────────────────────────────

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GTwoStrInner {
    a: String,
    b: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GListGroupTwoStr {
    a: Vec<GTwoStrInner>,
}

value_match_group_nesting_test!(
    value_match_list_of_group,
    GListGroupY,
    vec![GYInner { y: 100 }, GYInner { y: 42 }],
    LIST_GROUP,
    LIST_GROUP_INODE
);

value_match_group_nesting_test!(
    value_match_double_list_group_with_str_and_inner_list_group,
    GDoubleListGroupStr,
    vec![vec![GNestOuter {
        a: "world".to_string(),
        b: vec![GNestC {
            c: "hello".to_string(),
        }],
    }]],
    DOUBLE_LIST_GROUP_WITH_STR_AND_INNER_LIST_GROUP,
    DOUBLE_LIST_GROUP_WITH_STR_AND_INNER_LIST_GROUP_INODES
);

value_match_group_nesting_test!(
    value_match_list_group_with_list_list_f64_and_list_list_i64,
    GListGroupF64I64,
    vec![GLgF64I64Inner {
        b1: vec![vec![1.0]],
        b2: vec![vec![-1]],
    }],
    LIST_GROUP_WITH_LIST_LIST_F64_AND_LIST_LIST_I64,
    LIST_GROUP_WITH_LIST_LIST_F64_AND_LIST_LIST_I64_INODES
);

value_match_group_nesting_test!(
    value_match_list_group_triple_nested_u32,
    GListGroupTripleU32,
    vec![GTripleB {
        b: GTripleC {
            c: GTripleD { d: 99 },
        },
    }],
    LIST_GROUP_TRIPLE_NESTED_U32,
    LIST_GROUP_TRIPLE_NESTED_U32_INODES
);

value_match_group_nesting_test!(
    value_match_list_group_bytes_and_list_bytes,
    GListGroupBytes,
    vec![GBytesInner {
        a: vec![0xAA, 0xBB],
        b: vec![vec![0xCC]],
    }],
    LIST_GROUP_BYTES_AND_LIST_BYTES,
    LIST_GROUP_BYTES_AND_LIST_BYTES_INODES
);

value_match_group_nesting_test!(
    value_match_list_group_fixed_u8_array_and_list_fixed_u16_array,
    GListGroupFixedArrays,
    vec![GFixedInner {
        a: {
            let mut arr = [0u8; 16];
            arr[0] = 42;
            arr
        },
        b: vec![{
            let mut arr = [0u16; 8];
            arr[0] = 1234;
            arr
        }],
    }],
    LIST_GROUP_FIXED_U8_ARRAY_AND_LIST_FIXED_U16_ARRAY,
    LIST_GROUP_FIXED_U8_ARRAY_AND_LIST_FIXED_U16_ARRAY_INODES
);

value_match_group_nesting_test!(
    value_match_triple_nested_list_group_list_list_u32,
    GTripleNestedListGroupU32,
    vec![GTn7B {
        b: vec![GTn7C { c: vec![vec![42]] }],
    }],
    TRIPLE_NESTED_LIST_GROUP_LIST_LIST_U32,
    TRIPLE_NESTED_LIST_GROUP_LIST_LIST_U32_INODES
);

value_match_group_nesting_test!(
    value_match_list_group_bool_and_list_list_bool,
    GListGroupBool,
    vec![GBoolInner {
        a: true,
        b: vec![vec![true, false, true]],
    }],
    LIST_GROUP_BOOL_AND_LIST_LIST_BOOL,
    LIST_GROUP_BOOL_AND_LIST_LIST_BOOL_INODES
);

value_match_group_nesting_test!(
    value_match_list_group_two_str_fields,
    GListGroupTwoStr,
    vec![GTwoStrInner {
        a: "two".to_string(),
        b: "four".to_string(),
    }],
    LIST_GROUP_TWO_STR_FIELDS,
    LIST_GROUP_TWO_STR_FIELDS_INODES
);

value_match_group_nesting_test!(
    value_match_double_list_group_two_u8_fields,
    GDoubleListGroupTwoU8,
    vec![vec![GTwoU8Inner { a: 1, b: 2 }, GTwoU8Inner { a: 1, b: 2 },]],
    DOUBLE_LIST_GROUP_TWO_U8_FIELDS,
    DOUBLE_LIST_GROUP_TWO_U8_FIELDS_INODES
);

value_match_group_nesting_test!(
    value_match_double_list_group_two_u8_fields_uneven,
    GDoubleListGroupTwoU8,
    vec![
        vec![GTwoU8Inner { a: 1, b: 2 }],
        vec![GTwoU8Inner { a: 3, b: 4 }, GTwoU8Inner { a: 5, b: 6 }],
    ],
    DOUBLE_LIST_GROUP_TWO_U8_FIELDS_UNEVEN,
    DOUBLE_LIST_GROUP_TWO_U8_FIELDS_INODES
);

value_match_group_nesting_test!(
    value_match_double_list_group_two_u8_fields_empty_inner,
    GDoubleListGroupTwoU8,
    vec![
        vec![GTwoU8Inner { a: 1, b: 2 }],
        vec![],
        vec![GTwoU8Inner { a: 3, b: 4 }],
    ],
    DOUBLE_LIST_GROUP_TWO_U8_FIELDS_EMPTY_INNER,
    DOUBLE_LIST_GROUP_TWO_U8_FIELDS_INODES
);

value_match_group_nesting_test!(
    value_match_double_list_group_two_u8_fields_one_empty_inner,
    GDoubleListGroupTwoU8,
    vec![vec![]],
    DOUBLE_LIST_GROUP_TWO_U8_FIELDS_ONE_EMPTY_INNER,
    DOUBLE_LIST_GROUP_TWO_U8_FIELDS_INODES
);

value_match_group_nesting_test!(
    value_match_double_list_group_two_u8_fields_empty_outer,
    GDoubleListGroupTwoU8,
    Vec::<Vec<GTwoU8Inner>>::new(),
    DOUBLE_LIST_GROUP_TWO_U8_FIELDS_EMPTY_OUTER,
    DOUBLE_LIST_GROUP_TWO_U8_FIELDS_INODES
);
