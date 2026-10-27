//! Struct-vs-value match for `src/wire/tests/group_option_nesting_u16.rs` —
//! `Group`/`OptionGroup` nesting combined with `Option` fields, always
//! itself wrapped in exactly one list layer (`List<Group>` /
//! `List<OptionGroup>`). See `tests/value_match/primitive_u16.rs`'s module doc
//! for the shared harness rationale.

use pyroformat::wire::tests::group_option_nesting_u16::*;

use crate::support::read_back::assert_roundtrip;
use pyroformat::{Layout, Reader, Writer};

macro_rules! value_match_group_option_nesting_test {
    ($test_name:ident, $struct_ty:ident, $field:expr, $buf_const:expr, $inodes_const:expr) => {
        #[test]
        fn $test_name() {
            let data = $struct_ty { a: $field };
            assert_roundtrip(&data, &$buf_const, &$inodes_const);
        }
    };
}

// ── `List<OptionGroup { y: PrimitiveScalar<U8> }>` ──────────────────

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct LOptGroupY {
    y: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PListOptGroupY {
    a: Vec<Option<LOptGroupY>>,
}

// ── `List<Group { x: OptionPrimitiveScalar<I8>, y: PrimitiveScalar<F32> }>` ──

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GroupOptPrim {
    x: Option<i8>,
    y: f32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PListGroupOptPrim {
    a: Vec<GroupOptPrim>,
}

// ── `List<Group { a: U8, c: OptionGroup { x: OptionI8, y: F32 } }>` ──

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GroupOptGroupInner {
    x: Option<i8>,
    y: f32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GroupOptGroupElem {
    a: u8,
    c: Option<GroupOptGroupInner>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PListGroupOptionGroup {
    a: Vec<GroupOptGroupElem>,
}

// ── `List<Group { a: U8, b: OptionGroup { x: OptionI8, y: OptionGroup { p: U16, q: OptionBool } } }>` ──

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct DoubleNestedY {
    p: u16,
    q: Option<bool>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct DoubleNestedB {
    x: Option<i8>,
    y: Option<DoubleNestedY>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct DoubleNestedElem {
    a: u8,
    b: Option<DoubleNestedB>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PListGroupDoubleNestedOptionGroup {
    a: Vec<DoubleNestedElem>,
}

// ── `List<Group { t: OptionPrimitiveScalar<F64>, u: PrimitiveScalar<Bool> }>` ──

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GroupOptF64BoolElem {
    t: Option<f64>,
    u: bool,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PListGroupOptionF64Bool {
    a: Vec<Option<GroupOptF64BoolElem>>,
}

// ── `List<Group { x: OptionPrimitiveScalar<U8> }>` ──

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GroupOptU8Elem {
    x: Option<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PEmptyListGroupOption {
    a: Vec<Option<GroupOptU8Elem>>,
}

// ── `List<List<OptionGroup { x: OptionPrimitiveScalar<I8>, y: OptionPrimitiveScalar<F32> }>>` ──

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GroupOptI8OptF32 {
    x: Option<i8>,
    y: Option<f32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PDoubleListGroupOption {
    a: Vec<Vec<Option<GroupOptI8OptF32>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct GroupOptU8U16Elem {
    a: Option<u8>,
    b: Option<u16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PTripleListGroupOption {
    a: Vec<Vec<Vec<Option<GroupOptU8U16Elem>>>>,
}

value_match_group_option_nesting_test!(
    value_match_list_of_opt_group,
    PListOptGroupY,
    vec![
        Some(LOptGroupY { y: 100u8 }),
        None,
        Some(LOptGroupY { y: 42u8 }),
    ],
    LIST_OPT_GROUP,
    LIST_OPT_GROUP_INODE
);

value_match_group_option_nesting_test!(
    value_match_list_of_opt_group_two_validity_words,
    PListOptGroupY,
    (0..40u8)
        .map(|i| if i % 5 == 4 {
            None
        } else {
            Some(LOptGroupY { y: i })
        })
        .collect::<Vec<_>>(),
    LIST_OPT_GROUP_TWO_VALIDITY_WORDS,
    LIST_OPT_GROUP_INODE
);

value_match_group_option_nesting_test!(
    value_match_list_of_group_option_primitive_all_some,
    PListGroupOptPrim,
    vec![
        GroupOptPrim {
            x: Some(-100i8),
            y: 3.14f32,
        },
        GroupOptPrim {
            x: Some(42i8),
            y: 1.5f32,
        },
    ],
    LIST_GROUP_OPT_PRIM_ALL_SOME,
    LIST_GROUP_OPT_PRIM_ALL_SOME_INODES
);

value_match_group_option_nesting_test!(
    value_match_list_of_group_option_primitive_mixed_none,
    PListGroupOptPrim,
    vec![
        GroupOptPrim {
            x: Some(-100i8),
            y: 3.14f32,
        },
        GroupOptPrim {
            x: None::<i8>,
            y: 1.5f32,
        },
    ],
    LIST_GROUP_OPT_PRIM_MIXED_NONE,
    LIST_GROUP_OPT_PRIM_MIXED_NONE_INODES
);

value_match_group_option_nesting_test!(
    value_match_list_of_group_option_group_present,
    PListGroupOptionGroup,
    vec![GroupOptGroupElem {
        a: 200u8,
        c: Some(GroupOptGroupInner {
            x: Some(-100i8),
            y: 3.14f32,
        }),
    }],
    LIST_GROUP_OPTION_GROUP_PRESENT,
    LIST_GROUP_OPTION_GROUP_PRESENT_INODES
);

value_match_group_option_nesting_test!(
    value_match_list_of_group_option_group_absent,
    PListGroupOptionGroup,
    vec![GroupOptGroupElem { a: 200u8, c: None }],
    LIST_GROUP_OPTION_GROUP_ABSENT,
    LIST_GROUP_OPTION_GROUP_ABSENT_INODES
);

value_match_group_option_nesting_test!(
    value_match_list_of_group_option_group_mixed_present_absent,
    PListGroupOptionGroup,
    vec![
        GroupOptGroupElem {
            a: 200u8,
            c: Some(GroupOptGroupInner {
                x: None::<i8>,
                y: 3.14f32,
            }),
        },
        GroupOptGroupElem { a: 1u8, c: None },
    ],
    LIST_GROUP_OPTION_GROUP_MIXED,
    LIST_GROUP_OPTION_GROUP_MIXED_INODES
);

value_match_group_option_nesting_test!(
    value_match_list_of_group_double_nested_option_group,
    PListGroupDoubleNestedOptionGroup,
    vec![
        DoubleNestedElem {
            a: 10u8,
            b: Some(DoubleNestedB {
                x: Some(-5i8),
                y: Some(DoubleNestedY {
                    p: 300u16,
                    q: Some(true),
                }),
            }),
        },
        DoubleNestedElem { a: 30u8, b: None },
    ],
    LIST_GROUP_DOUBLE_NESTED_OPTION_GROUP,
    LIST_GROUP_DOUBLE_NESTED_OPTION_GROUP_INODES
);

value_match_group_option_nesting_test!(
    value_match_list_of_group_option_f64_bool_some_none,
    PListGroupOptionF64Bool,
    vec![
        Some(GroupOptF64BoolElem {
            t: Some(2.718281828f64),
            u: false,
        }),
        None,
        Some(GroupOptF64BoolElem {
            t: None::<f64>,
            u: true,
        }),
    ],
    LIST_GROUP_OPTION_F64_BOOL,
    LIST_GROUP_OPTION_F64_BOOL_INODES
);

value_match_group_option_nesting_test!(
    value_match_empty_list_of_group_option,
    PEmptyListGroupOption,
    vec![],
    EMPTY_LIST_GROUP_OPTION,
    EMPTY_LIST_GROUP_OPTION_INODES
);

value_match_group_option_nesting_test!(
    value_match_double_list_of_group_option,
    PDoubleListGroupOption,
    vec![vec![
        Some(GroupOptI8OptF32 {
            x: Some(-100i8),
            y: Some(3.14f32),
        }),
        None,
        Some(GroupOptI8OptF32 {
            x: None::<i8>,
            y: Some(1.5f32),
        }),
    ]],
    DOUBLE_LIST_GROUP_OPTION,
    DOUBLE_LIST_GROUP_OPTION_INODES
);

value_match_group_option_nesting_test!(
    value_match_triple_list_of_group_option,
    PTripleListGroupOption,
    vec![vec![vec![
        Some(GroupOptU8U16Elem {
            a: None::<u8>,
            b: None::<u16>,
        }),
        None,
        Some(GroupOptU8U16Elem {
            a: Some(10u8),
            b: Some(300u16),
        }),
    ]]],
    TRIPLE_LIST_GROUP_OPTION,
    TRIPLE_LIST_GROUP_OPTION_INODES
);
