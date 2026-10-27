//! Every derived struct shape from `tests/value_match/primitive_group_option_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use half::f16;
use pyroformat::value::Time;
use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

macro_rules! group_option_test {
    (
        $word:ident,
        $test_name:ident,
        $outer_ty:ident,
        $group_ty:ident { $($body:tt)* },
        $value:expr,
        $buf_const:expr,
        $inodes_const:expr
    ) => {
        #[derive(Layout, Writer, Reader, Debug, PartialEq)]
        #[pyro(Word = $word)]
        struct $group_ty {
            $($body)*
        }

        // `#[pyro(skip_list_mode)]`: a nested (non-`Vec`) struct field's
        // own list-mode reader may or may not have been emitted for its
        // type — that's a separate, opaque derive invocation
        // (`pyroformat-derive/src/dynamic/read/list.rs`'s module doc).
        // When it wasn't (e.g. the group contains `Option<Time>`, which
        // `list_option_reader_type` in `dynamic/read/field_info.rs` has
        // no arm for), this struct's own speculative list-mode reader
        // would reference a `{Group}DynamicListReader` symbol that was
        // never generated — a hard compile error, not a skip. Applied
        // uniformly here since `assert_roundtrip` never exercises
        // list-mode reading anyway.
        #[derive(Layout, Writer, Reader, Debug, PartialEq)]
        #[pyro(Word = $word)]
        #[pyro(skip_list_mode)]
        struct $outer_ty {
            a: $group_ty,
        }

        const _: fn() = assert_pyro_traits::<$outer_ty>;
    };
}

// Nested `Option<Group>` shape: the source schema's single top-level field
// `a` is itself typed `Group { a, b, c }`, so the derived shape needs three
// struct levels: an outermost `Outer { a: Middle }` row wrapper (`a`
// bare-struct-typed -> `PyroType::Group`), the `Middle { a, b, c }` struct
// holding the three fields, and `Middle`'s `c: Option<Inner>` (-> the nested
// `PyroType::OptionGroup`).
macro_rules! group_option_nested_test {
    (
        $word:ident,
        $test_name:ident,
        $outer_ty:ident,
        $middle_ty:ident { $($middle_body:tt)* },
        $inner_ty:ident { $($inner_body:tt)* },
        $value:expr,
        $buf_const:expr,
        $inodes_const:expr
    ) => {
        #[derive(Layout, Writer, Reader, Debug, PartialEq)]
        #[pyro(Word = $word)]
        struct $inner_ty {
            $($inner_body)*
        }

        #[derive(Layout, Writer, Reader, Debug, PartialEq)]
        #[pyro(Word = $word)]
        struct $middle_ty {
            $($middle_body)*
        }

        #[derive(Layout, Writer, Reader, Debug, PartialEq)]
        #[pyro(Word = $word)]
        #[pyro(skip_list_mode)]
        struct $outer_ty {
            a: $middle_ty,
        }

        const _: fn() = assert_pyro_traits::<$outer_ty>;
    };
}

group_option_test!(
    u32,
    value_match_group_option_i8_f32_u8_some_some,
    OuterI8F32U8SomeSome32,
    GroupI8F32U8W32 {
        x: Option<i8>,
        y: f32,
        z: Option<u8>,
    },
    GroupI8F32U8W32 {
        x: Some(-100i8),
        y: 3.14f32,
        z: Some(200u8),
    },
    GROUP_OPTION_I8_F32_U8_SOME_SOME,
    GROUP_OPTION_I8_F32_U8_SOME_SOME_INODES
);

group_option_test!(
    u16,
    value_match_group_option_i8_f32_u8_some_some,
    OuterI8F32U8SomeSome16,
    GroupI8F32U8W16 {
        x: Option<i8>,
        y: f32,
        z: Option<u8>,
    },
    GroupI8F32U8W16 {
        x: Some(-100i8),
        y: 3.14f32,
        z: Some(200u8),
    },
    GROUP_OPTION_I8_F32_U8_SOME_SOME,
    GROUP_OPTION_I8_F32_U8_SOME_SOME_INODES
);

group_option_test!(
    u32,
    value_match_group_option_i8_f32_u8_none_some,
    OuterI8F32U8NoneSome32,
    GroupI8F32U8NoneSome32 {
        x: Option<i8>,
        y: f32,
        z: Option<u8>,
    },
    GroupI8F32U8NoneSome32 {
        x: None::<i8>,
        y: 3.14f32,
        z: Some(200u8),
    },
    GROUP_OPTION_I8_F32_U8_NONE_SOME,
    GROUP_OPTION_I8_F32_U8_NONE_SOME_INODES
);

group_option_test!(
    u16,
    value_match_group_option_i8_f32_u8_none_some,
    OuterI8F32U8NoneSome16,
    GroupI8F32U8NoneSome16 {
        x: Option<i8>,
        y: f32,
        z: Option<u8>,
    },
    GroupI8F32U8NoneSome16 {
        x: None::<i8>,
        y: 3.14f32,
        z: Some(200u8),
    },
    GROUP_OPTION_I8_F32_U8_NONE_SOME,
    GROUP_OPTION_I8_F32_U8_NONE_SOME_INODES
);

group_option_test!(
    u32,
    value_match_group_option_i8_f32_u8_some_none,
    OuterI8F32U8SomeNone32,
    GroupI8F32U8SomeNone32 {
        x: Option<i8>,
        y: f32,
        z: Option<u8>,
    },
    GroupI8F32U8SomeNone32 {
        x: Some(-100i8),
        y: 3.14f32,
        z: None::<u8>,
    },
    GROUP_OPTION_I8_F32_U8_SOME_NONE,
    GROUP_OPTION_I8_F32_U8_SOME_NONE_INODES
);

group_option_test!(
    u16,
    value_match_group_option_i8_f32_u8_some_none,
    OuterI8F32U8SomeNone16,
    GroupI8F32U8SomeNone16 {
        x: Option<i8>,
        y: f32,
        z: Option<u8>,
    },
    GroupI8F32U8SomeNone16 {
        x: Some(-100i8),
        y: 3.14f32,
        z: None::<u8>,
    },
    GROUP_OPTION_I8_F32_U8_SOME_NONE,
    GROUP_OPTION_I8_F32_U8_SOME_NONE_INODES
);

group_option_test!(
    u32,
    value_match_group_option_i8_f32_u8_none_none,
    OuterI8F32U8NoneNone32,
    GroupI8F32U8NoneNone32 {
        x: Option<i8>,
        y: f32,
        z: Option<u8>,
    },
    GroupI8F32U8NoneNone32 {
        x: None::<i8>,
        y: 3.14f32,
        z: None::<u8>,
    },
    GROUP_OPTION_I8_F32_U8_NONE_NONE,
    GROUP_OPTION_I8_F32_U8_NONE_NONE_INODES
);

group_option_test!(
    u16,
    value_match_group_option_i8_f32_u8_none_none,
    OuterI8F32U8NoneNone16,
    GroupI8F32U8NoneNone16 {
        x: Option<i8>,
        y: f32,
        z: Option<u8>,
    },
    GroupI8F32U8NoneNone16 {
        x: None::<i8>,
        y: 3.14f32,
        z: None::<u8>,
    },
    GROUP_OPTION_I8_F32_U8_NONE_NONE,
    GROUP_OPTION_I8_F32_U8_NONE_NONE_INODES
);

group_option_test!(
    u32,
    value_match_group_option_u32_u8_u16_all_some,
    OuterU32U8U16AllSome32,
    GroupU32U8U16AllSome32 {
        p: Option<u32>,
        q: Option<u8>,
        r: Option<u16>,
    },
    GroupU32U8U16AllSome32 {
        p: Some(123456u32),
        q: Some(200u8),
        r: Some(54321u16),
    },
    GROUP_OPTION_U32_U8_U16_ALL_SOME,
    GROUP_OPTION_U32_U8_U16_ALL_SOME_INODES
);

group_option_test!(
    u16,
    value_match_group_option_u32_u8_u16_all_some,
    OuterU32U8U16AllSome16,
    GroupU32U8U16AllSome16 {
        p: Option<u32>,
        q: Option<u8>,
        r: Option<u16>,
    },
    GroupU32U8U16AllSome16 {
        p: Some(123456u32),
        q: Some(200u8),
        r: Some(54321u16),
    },
    GROUP_OPTION_U32_U8_U16_ALL_SOME,
    GROUP_OPTION_U32_U8_U16_ALL_SOME_INODES
);

group_option_test!(
    u32,
    value_match_group_option_u32_u8_u16_all_none,
    OuterU32U8U16AllNone32,
    GroupU32U8U16AllNone32 {
        p: Option<u32>,
        q: Option<u8>,
        r: Option<u16>,
    },
    GroupU32U8U16AllNone32 {
        p: None::<u32>,
        q: None::<u8>,
        r: None::<u16>,
    },
    GROUP_OPTION_U32_U8_U16_ALL_NONE,
    GROUP_OPTION_U32_U8_U16_ALL_NONE_INODES
);

group_option_test!(
    u16,
    value_match_group_option_u32_u8_u16_all_none,
    OuterU32U8U16AllNone16,
    GroupU32U8U16AllNone16 {
        p: Option<u32>,
        q: Option<u8>,
        r: Option<u16>,
    },
    GroupU32U8U16AllNone16 {
        p: None::<u32>,
        q: None::<u8>,
        r: None::<u16>,
    },
    GROUP_OPTION_U32_U8_U16_ALL_NONE,
    GROUP_OPTION_U32_U8_U16_ALL_NONE_INODES
);

group_option_test!(
    u32,
    value_match_group_option_timestamp_u8_i16_some_some,
    OuterTimestampU8I16SomeSome32,
    GroupTimestampU8I16SomeSome32 {
        t: Option<Time>,
        u: u8,
        v: Option<i16>,
    },
    GroupTimestampU8I16SomeSome32 {
        t: Some(Time(999_999_999)),
        u: 200u8,
        v: Some(-12345i16),
    },
    GROUP_OPTION_TIMESTAMP_U8_I16_SOME_SOME,
    GROUP_OPTION_TIMESTAMP_U8_I16_SOME_SOME_INODES
);

group_option_test!(
    u16,
    value_match_group_option_timestamp_u8_i16_some_some,
    OuterTimestampU8I16SomeSome16,
    GroupTimestampU8I16SomeSome16 {
        t: Option<Time>,
        u: u8,
        v: Option<i16>,
    },
    GroupTimestampU8I16SomeSome16 {
        t: Some(Time(999_999_999)),
        u: 200u8,
        v: Some(-12345i16),
    },
    GROUP_OPTION_TIMESTAMP_U8_I16_SOME_SOME,
    GROUP_OPTION_TIMESTAMP_U8_I16_SOME_SOME_INODES
);

group_option_test!(
    u32,
    value_match_group_option_timestamp_u8_i16_none_none,
    OuterTimestampU8I16NoneNone32,
    GroupTimestampU8I16NoneNone32 {
        t: Option<Time>,
        u: u8,
        v: Option<i16>,
    },
    GroupTimestampU8I16NoneNone32 {
        t: None::<Time>,
        u: 200u8,
        v: None::<i16>,
    },
    GROUP_OPTION_TIMESTAMP_U8_I16_NONE_NONE,
    GROUP_OPTION_TIMESTAMP_U8_I16_NONE_NONE_INODES
);

group_option_test!(
    u16,
    value_match_group_option_timestamp_u8_i16_none_none,
    OuterTimestampU8I16NoneNone16,
    GroupTimestampU8I16NoneNone16 {
        t: Option<Time>,
        u: u8,
        v: Option<i16>,
    },
    GroupTimestampU8I16NoneNone16 {
        t: None::<Time>,
        u: 200u8,
        v: None::<i16>,
    },
    GROUP_OPTION_TIMESTAMP_U8_I16_NONE_NONE,
    GROUP_OPTION_TIMESTAMP_U8_I16_NONE_NONE_INODES
);

group_option_nested_test!(
    u32,
    value_match_group_option_nested_group_all_present,
    OuterNestedGroupAllPresent32,
    MiddleNestedGroupAllPresent32 {
        a: i8,
        b: Option<f16>,
        c: Option<InnerNestedGroupAllPresent32>,
    },
    InnerNestedGroupAllPresent32 {
        x: Option<i8>,
        y: f32,
        z: Option<u8>,
    },
    MiddleNestedGroupAllPresent32 {
        a: -100i8,
        b: Some(f16::from_f32(1.5)),
        c: Some(InnerNestedGroupAllPresent32 {
            x: Some(-100i8),
            y: 3.14f32,
            z: Some(200u8),
        }),
    },
    GROUP_OPTION_NESTED_GROUP_ALL_PRESENT,
    GROUP_OPTION_NESTED_GROUP_ALL_PRESENT_INODES
);

group_option_nested_test!(
    u16,
    value_match_group_option_nested_group_all_present,
    OuterNestedGroupAllPresent16,
    MiddleNestedGroupAllPresent16 {
        a: i8,
        b: Option<f16>,
        c: Option<InnerNestedGroupAllPresent16>,
    },
    InnerNestedGroupAllPresent16 {
        x: Option<i8>,
        y: f32,
        z: Option<u8>,
    },
    MiddleNestedGroupAllPresent16 {
        a: -100i8,
        b: Some(f16::from_f32(1.5)),
        c: Some(InnerNestedGroupAllPresent16 {
            x: Some(-100i8),
            y: 3.14f32,
            z: Some(200u8),
        }),
    },
    GROUP_OPTION_NESTED_GROUP_ALL_PRESENT,
    GROUP_OPTION_NESTED_GROUP_ALL_PRESENT_INODES
);

group_option_nested_test!(
    u32,
    value_match_group_option_nested_group_present_children_none,
    OuterNestedGroupPresentChildrenNone32,
    MiddleNestedGroupPresentChildrenNone32 {
        a: i8,
        b: Option<f16>,
        c: Option<InnerNestedGroupPresentChildrenNone32>,
    },
    InnerNestedGroupPresentChildrenNone32 {
        x: Option<i8>,
        y: f32,
        z: Option<u8>,
    },
    MiddleNestedGroupPresentChildrenNone32 {
        a: -100i8,
        b: Some(f16::from_f32(1.5)),
        c: Some(InnerNestedGroupPresentChildrenNone32 {
            x: None::<i8>,
            y: 3.14f32,
            z: None::<u8>,
        }),
    },
    GROUP_OPTION_NESTED_GROUP_PRESENT_CHILDREN_NONE,
    GROUP_OPTION_NESTED_GROUP_PRESENT_CHILDREN_NONE_INODES
);

group_option_nested_test!(
    u16,
    value_match_group_option_nested_group_present_children_none,
    OuterNestedGroupPresentChildrenNone16,
    MiddleNestedGroupPresentChildrenNone16 {
        a: i8,
        b: Option<f16>,
        c: Option<InnerNestedGroupPresentChildrenNone16>,
    },
    InnerNestedGroupPresentChildrenNone16 {
        x: Option<i8>,
        y: f32,
        z: Option<u8>,
    },
    MiddleNestedGroupPresentChildrenNone16 {
        a: -100i8,
        b: Some(f16::from_f32(1.5)),
        c: Some(InnerNestedGroupPresentChildrenNone16 {
            x: None::<i8>,
            y: 3.14f32,
            z: None::<u8>,
        }),
    },
    GROUP_OPTION_NESTED_GROUP_PRESENT_CHILDREN_NONE,
    GROUP_OPTION_NESTED_GROUP_PRESENT_CHILDREN_NONE_INODES
);

group_option_nested_test!(
    u32,
    value_match_group_option_nested_group_absent,
    OuterNestedGroupAbsent32,
    MiddleNestedGroupAbsent32 {
        a: i8,
        b: Option<f16>,
        c: Option<InnerNestedGroupAbsent32>,
    },
    InnerNestedGroupAbsent32 {
        x: Option<i8>,
        y: f32,
        z: Option<u8>,
    },
    MiddleNestedGroupAbsent32 {
        a: -100i8,
        b: None::<f16>,
        c: None,
    },
    GROUP_OPTION_NESTED_GROUP_ABSENT,
    GROUP_OPTION_NESTED_GROUP_ABSENT_INODES
);

group_option_nested_test!(
    u16,
    value_match_group_option_nested_group_absent,
    OuterNestedGroupAbsent16,
    MiddleNestedGroupAbsent16 {
        a: i8,
        b: Option<f16>,
        c: Option<InnerNestedGroupAbsent16>,
    },
    InnerNestedGroupAbsent16 {
        x: Option<i8>,
        y: f32,
        z: Option<u8>,
    },
    MiddleNestedGroupAbsent16 {
        a: -100i8,
        b: None::<f16>,
        c: None,
    },
    GROUP_OPTION_NESTED_GROUP_ABSENT,
    GROUP_OPTION_NESTED_GROUP_ABSENT_INODES
);

fn main() {
}
