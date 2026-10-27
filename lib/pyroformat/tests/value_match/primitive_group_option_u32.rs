use half::f16;

use pyroformat::value::Time;
use pyroformat::wire::tests::primitive_group_option_u32::*;

use crate::support::read_back::assert_roundtrip;
use pyroformat::{Layout, Reader, Writer};

macro_rules! group_option_test {
    (
        $test_name:ident,
        $group_ty:ident { $($body:tt)* },
        $value:expr,
        $buf_const:expr,
        $inodes_const:expr
    ) => {
        #[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
        struct $group_ty {
            $($body)*
        }

        #[test]
        fn $test_name() {
            let data = $value;
            assert_roundtrip(&data, &$buf_const, &$inodes_const);
        }
    };
}

// Nested `Option<Group>` shape: the source schema's top-level fields are
// `a, b, c`, so the derived shape needs two struct levels: the `Middle { a,
// b, c }` row holding the three fields, and `Middle`'s `c: Option<Inner>`
// (-> the nested `PyroType::OptionGroup`).
macro_rules! group_option_nested_test {
    (
        $test_name:ident,
        $middle_ty:ident { $($middle_body:tt)* },
        $inner_ty:ident { $($inner_body:tt)* },
        $value:expr,
        $buf_const:expr,
        $inodes_const:expr
    ) => {
        #[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
        struct $inner_ty {
            $($inner_body)*
        }

        #[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
        struct $middle_ty {
            $($middle_body)*
        }

        #[test]
        fn $test_name() {
            let data = $value;
            assert_roundtrip(&data, &$buf_const, &$inodes_const);
        }
    };
}

// ── Group { Option<I8>, F32, Option<U8> } ───────────────────────────────

group_option_test!(
    value_match_group_option_i8_f32_u8_some_some,
    GroupI8F32U8 {
        x: Option<i8>,
        y: f32,
        z: Option<u8>,
    },
    GroupI8F32U8 {
        x: Some(-100i8),
        y: 3.14f32,
        z: Some(200u8),
    },
    GROUP_OPTION_I8_F32_U8_SOME_SOME,
    GROUP_OPTION_I8_F32_U8_SOME_SOME_INODES

);

group_option_test!(
    value_match_group_option_i8_f32_u8_none_some,
    GroupI8F32U8NoneSome {
        x: Option<i8>,
        y: f32,
        z: Option<u8>,
    },
    GroupI8F32U8NoneSome {
        x: None::<i8>,
        y: 3.14f32,
        z: Some(200u8),
    },
    GROUP_OPTION_I8_F32_U8_NONE_SOME,
    GROUP_OPTION_I8_F32_U8_NONE_SOME_INODES

);

group_option_test!(
    value_match_group_option_i8_f32_u8_some_none,
    GroupI8F32U8SomeNone {
        x: Option<i8>,
        y: f32,
        z: Option<u8>,
    },
    GroupI8F32U8SomeNone {
        x: Some(-100i8),
        y: 3.14f32,
        z: None::<u8>,
    },
    GROUP_OPTION_I8_F32_U8_SOME_NONE,
    GROUP_OPTION_I8_F32_U8_SOME_NONE_INODES

);

group_option_test!(
    value_match_group_option_i8_f32_u8_none_none,
    GroupI8F32U8NoneNone {
        x: Option<i8>,
        y: f32,
        z: Option<u8>,
    },
    GroupI8F32U8NoneNone {
        x: None::<i8>,
        y: 3.14f32,
        z: None::<u8>,
    },
    GROUP_OPTION_I8_F32_U8_NONE_NONE,
    GROUP_OPTION_I8_F32_U8_NONE_NONE_INODES

);

// ── Group { Option<U32>, Option<U8>, Option<U16> } ──────────────────────

group_option_test!(
    value_match_group_option_u32_u8_u16_all_some,
    GroupU32U8U16AllSome {
        p: Option<u32>,
        q: Option<u8>,
        r: Option<u16>,
    },
    GroupU32U8U16AllSome {
        p: Some(123456u32),
        q: Some(200u8),
        r: Some(54321u16),
    },
    GROUP_OPTION_U32_U8_U16_ALL_SOME,
    GROUP_OPTION_U32_U8_U16_ALL_SOME_INODES

);

group_option_test!(
    value_match_group_option_u32_u8_u16_all_none,
    GroupU32U8U16AllNone {
        p: Option<u32>,
        q: Option<u8>,
        r: Option<u16>,
    },
    GroupU32U8U16AllNone {
        p: None::<u32>,
        q: None::<u8>,
        r: None::<u16>,
    },
    GROUP_OPTION_U32_U8_U16_ALL_NONE,
    GROUP_OPTION_U32_U8_U16_ALL_NONE_INODES

);

// ── Group { Option<Timestamp>, U8, Option<I16> } ────────────────────────

group_option_test!(
    value_match_group_option_timestamp_u8_i16_some_some,
    GroupTimestampU8I16SomeSome {
        t: Option<Time>,
        u: u8,
        v: Option<i16>,
    },
    GroupTimestampU8I16SomeSome {
        t: Some(Time(999_999_999)),
        u: 200u8,
        v: Some(-12345i16),
    },
    GROUP_OPTION_TIMESTAMP_U8_I16_SOME_SOME,
    GROUP_OPTION_TIMESTAMP_U8_I16_SOME_SOME_INODES

);

group_option_test!(
    value_match_group_option_timestamp_u8_i16_none_none,
    GroupTimestampU8I16NoneNone {
        t: Option<Time>,
        u: u8,
        v: Option<i16>,
    },
    GroupTimestampU8I16NoneNone {
        t: None::<Time>,
        u: 200u8,
        v: None::<i16>,
    },
    GROUP_OPTION_TIMESTAMP_U8_I16_NONE_NONE,
    GROUP_OPTION_TIMESTAMP_U8_I16_NONE_NONE_INODES

);

// ── Nested `Option<Group>`: Group { I8, Option<F16>, Option<Group {
// Option<I8>, F32, Option<U8> }> } ──────────────────────────────────────

group_option_nested_test!(
    value_match_group_option_nested_group_all_present,
    MiddleNestedGroupAllPresent {
        a: i8,
        b: Option<f16>,
        c: Option<InnerNestedGroupAllPresent>,
    },
    InnerNestedGroupAllPresent {
        x: Option<i8>,
        y: f32,
        z: Option<u8>,
    },
    MiddleNestedGroupAllPresent {
        a: -100i8,
        b: Some(f16::from_f32(1.5)),
        c: Some(InnerNestedGroupAllPresent {
            x: Some(-100i8),
            y: 3.14f32,
            z: Some(200u8),
        }),
    },
    GROUP_OPTION_NESTED_GROUP_ALL_PRESENT,
    GROUP_OPTION_NESTED_GROUP_ALL_PRESENT_INODES

);

group_option_nested_test!(
    value_match_group_option_nested_group_present_children_none,
    MiddleNestedGroupPresentChildrenNone {
        a: i8,
        b: Option<f16>,
        c: Option<InnerNestedGroupPresentChildrenNone>,
    },
    InnerNestedGroupPresentChildrenNone {
        x: Option<i8>,
        y: f32,
        z: Option<u8>,
    },
    MiddleNestedGroupPresentChildrenNone {
        a: -100i8,
        b: Some(f16::from_f32(1.5)),
        c: Some(InnerNestedGroupPresentChildrenNone {
            x: None::<i8>,
            y: 3.14f32,
            z: None::<u8>,
        }),
    },
    GROUP_OPTION_NESTED_GROUP_PRESENT_CHILDREN_NONE,
    GROUP_OPTION_NESTED_GROUP_PRESENT_CHILDREN_NONE_INODES

);

group_option_nested_test!(
    value_match_group_option_nested_group_absent,
    MiddleNestedGroupAbsent {
        a: i8,
        b: Option<f16>,
        c: Option<InnerNestedGroupAbsent>,
    },
    InnerNestedGroupAbsent {
        x: Option<i8>,
        y: f32,
        z: Option<u8>,
    },
    MiddleNestedGroupAbsent {
        a: -100i8,
        b: None::<f16>,
        c: None,
    },
    GROUP_OPTION_NESTED_GROUP_ABSENT,
    GROUP_OPTION_NESTED_GROUP_ABSENT_INODES

);

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupOptionGapsMasks {
    a: Vec<Option<u64>>,
    b: Vec<Option<u32>>,
    c: Vec<Option<u16>>,
    d: Vec<Option<u8>>,
    e: Vec<Option<Time>>,
    f: Option<u8>,
    g: Option<f64>,
}

#[test]
fn value_match_group_option_gaps_masks() {
    let data = GroupOptionGapsMasks {
        a: vec![Some(1), None, Some(3)],
        b: vec![None],
        c: vec![Some(1), Some(2), Some(3)],
        d: vec![None, Some(5), None, Some(7), Some(8)],
        e: vec![None, Some(Time(99))],
        f: Some(4),
        g: None,
    };
    assert_roundtrip(
        &data,
        &GROUP_OPTION_GAPS_MASKS,
        &GROUP_OPTION_GAPS_MASKS_INODES,
    );
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupOptionGapsMasksAllNone {
    a: Vec<Option<u64>>,
    b: Vec<Option<u32>>,
    c: Vec<Option<u16>>,
    d: Vec<Option<u8>>,
    e: Vec<Option<Time>>,
    f: Option<u8>,
    g: Option<f64>,
}

#[test]
fn value_match_group_option_gaps_masks_all_none() {
    let data = GroupOptionGapsMasksAllNone {
        a: vec![None, None],
        b: vec![None],
        c: vec![None, None, None],
        d: vec![None, None, None, None, None, None, None, None, None],
        e: vec![None],
        f: None,
        g: Some(2.5),
    };
    assert_roundtrip(
        &data,
        &GROUP_OPTION_GAPS_MASKS_ALL_NONE,
        &GROUP_OPTION_GAPS_MASKS_ALL_NONE_INODES,
    );
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupOptionGapsOptionGroupPresentA {
    x: Vec<Option<i16>>,
    y: Option<u32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupOptionGapsOptionGroupPresentB {
    p: u64,
    q: Option<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupOptionGapsOptionGroupPresent {
    a: Option<GroupOptionGapsOptionGroupPresentA>,
    b: Vec<Option<GroupOptionGapsOptionGroupPresentB>>,
    c: Vec<Vec<Option<f32>>>,
    d: u8,
}

#[test]
fn value_match_group_option_gaps_option_group_present() {
    let data = GroupOptionGapsOptionGroupPresent {
        a: Some(GroupOptionGapsOptionGroupPresentA {
            x: vec![None, Some(-2)],
            y: Some(5),
        }),
        b: vec![
            Some(GroupOptionGapsOptionGroupPresentB { p: 1, q: None }),
            None,
            Some(GroupOptionGapsOptionGroupPresentB { p: 3, q: Some(4) }),
        ],
        c: vec![vec![Some(1.5), None], vec![], vec![None]],
        d: 1,
    };
    assert_roundtrip(
        &data,
        &GROUP_OPTION_GAPS_OPTION_GROUP_PRESENT,
        &GROUP_OPTION_GAPS_OPTION_GROUP_PRESENT_INODES,
    );
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupOptionGapsOptionGroupAbsentA {
    x: Vec<Option<i16>>,
    y: Option<u32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupOptionGapsOptionGroupAbsentB {
    p: u64,
    q: Option<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupOptionGapsOptionGroupAbsent {
    a: Option<GroupOptionGapsOptionGroupAbsentA>,
    b: Vec<Option<GroupOptionGapsOptionGroupAbsentB>>,
    c: Vec<Vec<Option<f32>>>,
    d: u8,
}

#[test]
fn value_match_group_option_gaps_option_group_absent() {
    let data = GroupOptionGapsOptionGroupAbsent {
        a: None,
        b: vec![
            Some(GroupOptionGapsOptionGroupAbsentB { p: 1, q: None }),
            None,
            Some(GroupOptionGapsOptionGroupAbsentB { p: 3, q: Some(4) }),
        ],
        c: vec![vec![Some(1.5), None], vec![], vec![None]],
        d: 1,
    };
    assert_roundtrip(
        &data,
        &GROUP_OPTION_GAPS_OPTION_GROUP_ABSENT,
        &GROUP_OPTION_GAPS_OPTION_GROUP_ABSENT_INODES,
    );
}
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct GroupOptionGapsFixed {
    a: Vec<Option<[u16; 3]>>,
    b: Option<[u8; 5]>,
    c: Vec<Option<[f64; 2]>>,
    d: Vec<Option<[bool; 3]>>,
    e: Option<i8>,
}

#[test]
fn value_match_group_option_gaps_fixed() {
    let data = GroupOptionGapsFixed {
        a: vec![Some([1, 2, 3]), None, Some([4, 5, 6])],
        b: Some([1, 2, 3, 4, 5]),
        c: vec![None, Some([0.5, -0.5])],
        d: vec![Some([true, true, false]), None],
        e: Some(-1),
    };
    assert_roundtrip(
        &data,
        &GROUP_OPTION_GAPS_FIXED,
        &GROUP_OPTION_GAPS_FIXED_INODES,
    );
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OptionGroupAbsentStrA {
    s: String,
    n: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OptionGroupAbsentStr {
    a: Option<OptionGroupAbsentStrA>,
    b: String,
    c: u8,
}

#[test]
fn value_match_group_option_group_absent_str() {
    let data = OptionGroupAbsentStr {
        a: None,
        b: "xyz".to_string(),
        c: 7,
    };
    assert_roundtrip(
        &data,
        &GROUP_OPTION_GROUP_ABSENT_STR,
        &GROUP_OPTION_GROUP_ABSENT_STR_INODES,
    );
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OptionGroupAbsentWideListA {
    v: Vec<u64>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OptionGroupAbsentWideList {
    a: Option<OptionGroupAbsentWideListA>,
    b: Vec<u32>,
    d: u8,
}

#[test]
fn value_match_group_option_group_absent_wide_list() {
    let data = OptionGroupAbsentWideList {
        a: None,
        b: vec![1, 2, 3],
        d: 9,
    };
    assert_roundtrip(
        &data,
        &GROUP_OPTION_GROUP_ABSENT_WIDE_LIST,
        &GROUP_OPTION_GROUP_ABSENT_WIDE_LIST_INODES,
    );
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OptionGroupAbsentNestedListA {
    m: Vec<Vec<u16>>,
    k: u32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OptionGroupAbsentNestedList {
    a: Option<OptionGroupAbsentNestedListA>,
    #[pyro(numeric)]
    b: Vec<u8>,
}

#[test]
fn value_match_group_option_group_absent_nested_list() {
    let data = OptionGroupAbsentNestedList {
        a: None,
        b: vec![5, 6, 7],
    };
    assert_roundtrip(
        &data,
        &GROUP_OPTION_GROUP_ABSENT_NESTED_LIST,
        &GROUP_OPTION_GROUP_ABSENT_NESTED_LIST_INODES,
    );
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OptionGroupAbsentInsidePresentInner {
    v: Vec<i16>,
    w: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OptionGroupAbsentInsidePresentA {
    x: u8,
    inner: Option<OptionGroupAbsentInsidePresentInner>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OptionGroupAbsentInsidePresent {
    a: Option<OptionGroupAbsentInsidePresentA>,
    #[pyro(numeric)]
    b: Vec<u8>,
    c: u16,
}

#[test]
fn value_match_group_option_group_absent_inside_present() {
    let data = OptionGroupAbsentInsidePresent {
        a: Some(OptionGroupAbsentInsidePresentA { x: 3, inner: None }),
        b: vec![1],
        c: 513,
    };
    assert_roundtrip(
        &data,
        &GROUP_OPTION_GROUP_ABSENT_INSIDE_PRESENT,
        &GROUP_OPTION_GROUP_ABSENT_INSIDE_PRESENT_INODES,
    );
}
