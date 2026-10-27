//! `List<List<List<List<List<Option<[T; 2]>>>>>>` for every `PrimitiveDataType`. The same list shapes as
//! `primitive_5_fixed_nesting.rs`, with arrays 1 and 4 (of 6) null:
//! the option is on the array, never on its elements.
//!
//! Shared value shape: `[[[[[a, b]], [[c]]], [[[d]]]], [[[[e, f]]]]]` (each letter an `Option<[T; 2]>`), `b` and `e` null.
//!
//! Layout: `[topology start, topology word count]`, `[payload start, valid
//! arrays, total arrays]`, then 4 topology levels of sentinel-prefixed
//! cumulative boundaries (the deepest counts arrays, nulls included)
//! (level 0: `MAX, 2, 3`)
//! (level 1: `MAX, 2, 3, 4`)
//! (level 2: `MAX, 1, 2, 3, 4`)
//! (level 3: `MAX, 2, 3, 4, 6`)
//! then (after alignment padding) the valid payload, then the validity
//! mask (one bit per array, LSB first, zero-padded up to the element
//! alignment).

#[cfg(test)]
use half::f16;
#[cfg(test)]
use pyro_schema::PrimitiveDataType;
#[cfg(test)]
use pyro_schema::human::{PyroField, PyroSchema, PyroType};

#[cfg(test)]
use crate::{
    PyroValue,
    value::{PrimitiveValueList, Time},
    wire::tests::support::run_list_wrapped_roundtrip_u32,
};

use pyro_schema::{Inode, KindMeta, WireKind};

#[cfg(test)]
fn nested_5_option_fixed_sized_schema(primitive: PrimitiveDataType, size: usize) -> PyroSchema {
    let mut ty = PyroType::OptionPrimitiveFixedList {
        element: primitive,
        size,
    };
    for _ in 0..5 {
        ty = PyroType::List {
            element: Box::new(ty),
        };
    }
    PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: ty,
        }],
    }
}

#[cfg(test)]
fn nested_5_option_fixed_schema(primitive: PrimitiveDataType) -> PyroSchema {
    nested_5_option_fixed_sized_schema(primitive, 2)
}

/// `[[[[[a, b]], [[c]]], [[[d]]]], [[[[e, f]]]]]` from 4 leaf runs.
#[cfg(test)]
fn nested_5_option_fixed_value(leaves: [PyroValue<'static>; 4]) -> PyroValue<'static> {
    let [l0, l1, l2, l3] = leaves;
    PyroValue::List(vec![
        PyroValue::List(vec![
            PyroValue::List(vec![PyroValue::List(vec![l0]), PyroValue::List(vec![l1])]),
            PyroValue::List(vec![PyroValue::List(vec![l2])]),
        ]),
        PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![l3])])]),
    ])
}

/// Same as [`nested_5_option_fixed_inode`], for `[T; fixed_size]` leaves.
const fn nested_5_option_fixed_sized_inode(kind: WireKind, fixed_size: u16) -> [Inode; 1] {
    [Inode {
        kind,
        kind_meta: KindMeta::Primitive {
            nesting: 5,
            data_offset: 8,
            topology_offset: 0,
            fixed_size,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    }]
}

/// `List<List<List<List<List<Option<[T; 2]>>>>>>` at the root: one `Primitive` inode with `nesting: 5`, `fixed_size: 2`,
/// its topology slot at 0 and its data slot at 8.
const fn nested_5_option_fixed_inode(kind: WireKind) -> [Inode; 1] {
    nested_5_option_fixed_sized_inode(kind, 2)
}

// ---------------------------------------------------------------------------
// OptionU8
// ---------------------------------------------------------------------------

pub const NESTED_5_OPTION_FIXED_U8: [u8; 97] = [
    20, 0, 0, 0, // topology start pointer
    17, 0, 0, 0, // topology word count (4 levels, sentinels included)
    88, 0, 0, 0, // payload start pointer
    4, 0, 0, 0, // valid arrays
    6, 0, 0, 0, // total arrays (nulls included)
    255, 255, 255, 255, // level 0: sentinel
    2, 0, 0, 0, // level 0: boundary at 2 lists
    3, 0, 0, 0, // level 0: boundary at 3 lists
    255, 255, 255, 255, // level 1: sentinel
    2, 0, 0, 0, // level 1: boundary at 2 lists
    3, 0, 0, 0, // level 1: boundary at 3 lists
    4, 0, 0, 0, // level 1: boundary at 4 lists
    255, 255, 255, 255, // level 2: sentinel
    1, 0, 0, 0, // level 2: boundary at 1 lists
    2, 0, 0, 0, // level 2: boundary at 2 lists
    3, 0, 0, 0, // level 2: boundary at 3 lists
    4, 0, 0, 0, // level 2: boundary at 4 lists
    255, 255, 255, 255, // level 3: sentinel
    2, 0, 0, 0, // level 3: boundary at 2 arrays (nulls included)
    3, 0, 0, 0, // level 3: boundary at 3 arrays (nulls included)
    4, 0, 0, 0, // level 3: boundary at 4 arrays (nulls included)
    6, 0, 0, 0,  // level 3: boundary at 6 arrays (nulls included)
    1,  // valid payload[0] = 1
    2,  // valid payload[1] = 2
    5,  // valid payload[2] = 5
    6,  // valid payload[3] = 6
    7,  // valid payload[4] = 7
    8,  // valid payload[5] = 8
    11, // valid payload[6] = 11
    12, // valid payload[7] = 12
    45, // validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
];
pub const NESTED_5_OPTION_FIXED_U8_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionU8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_u8() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::U8);
    let data = nested_5_option_fixed_value([
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![1, 2].into())),
            PyroValue::Null,
        ]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::U8(
            vec![5, 6].into(),
        ))]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::U8(
            vec![7, 8].into(),
        ))]),
        PyroValue::List(vec![
            PyroValue::Null,
            PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![11, 12].into())),
        ]),
    ]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_U8,
        &NESTED_5_OPTION_FIXED_U8_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionI8
// ---------------------------------------------------------------------------

pub const NESTED_5_OPTION_FIXED_I8: [u8; 97] = [
    20, 0, 0, 0, // topology start pointer
    17, 0, 0, 0, // topology word count (4 levels, sentinels included)
    88, 0, 0, 0, // payload start pointer
    4, 0, 0, 0, // valid arrays
    6, 0, 0, 0, // total arrays (nulls included)
    255, 255, 255, 255, // level 0: sentinel
    2, 0, 0, 0, // level 0: boundary at 2 lists
    3, 0, 0, 0, // level 0: boundary at 3 lists
    255, 255, 255, 255, // level 1: sentinel
    2, 0, 0, 0, // level 1: boundary at 2 lists
    3, 0, 0, 0, // level 1: boundary at 3 lists
    4, 0, 0, 0, // level 1: boundary at 4 lists
    255, 255, 255, 255, // level 2: sentinel
    1, 0, 0, 0, // level 2: boundary at 1 lists
    2, 0, 0, 0, // level 2: boundary at 2 lists
    3, 0, 0, 0, // level 2: boundary at 3 lists
    4, 0, 0, 0, // level 2: boundary at 4 lists
    255, 255, 255, 255, // level 3: sentinel
    2, 0, 0, 0, // level 3: boundary at 2 arrays (nulls included)
    3, 0, 0, 0, // level 3: boundary at 3 arrays (nulls included)
    4, 0, 0, 0, // level 3: boundary at 4 arrays (nulls included)
    6, 0, 0, 0,   // level 3: boundary at 6 arrays (nulls included)
    255, // valid payload[0] = -1
    2,   // valid payload[1] = 2
    251, // valid payload[2] = -5
    6,   // valid payload[3] = 6
    249, // valid payload[4] = -7
    8,   // valid payload[5] = 8
    245, // valid payload[6] = -11
    12,  // valid payload[7] = 12
    45,  // validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
];
pub const NESTED_5_OPTION_FIXED_I8_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionI8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_i8() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::I8);
    let data = nested_5_option_fixed_value([
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::I8(vec![-1, 2].into())),
            PyroValue::Null,
        ]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::I8(
            vec![-5, 6].into(),
        ))]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::I8(
            vec![-7, 8].into(),
        ))]),
        PyroValue::List(vec![
            PyroValue::Null,
            PyroValue::PrimitiveList(PrimitiveValueList::I8(vec![-11, 12].into())),
        ]),
    ]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_I8,
        &NESTED_5_OPTION_FIXED_I8_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionU16
// ---------------------------------------------------------------------------

pub const NESTED_5_OPTION_FIXED_U16: [u8; 105] = [
    20, 0, 0, 0, // topology start pointer
    17, 0, 0, 0, // topology word count (4 levels, sentinels included)
    88, 0, 0, 0, // payload start pointer
    4, 0, 0, 0, // valid arrays
    6, 0, 0, 0, // total arrays (nulls included)
    255, 255, 255, 255, // level 0: sentinel
    2, 0, 0, 0, // level 0: boundary at 2 lists
    3, 0, 0, 0, // level 0: boundary at 3 lists
    255, 255, 255, 255, // level 1: sentinel
    2, 0, 0, 0, // level 1: boundary at 2 lists
    3, 0, 0, 0, // level 1: boundary at 3 lists
    4, 0, 0, 0, // level 1: boundary at 4 lists
    255, 255, 255, 255, // level 2: sentinel
    1, 0, 0, 0, // level 2: boundary at 1 lists
    2, 0, 0, 0, // level 2: boundary at 2 lists
    3, 0, 0, 0, // level 2: boundary at 3 lists
    4, 0, 0, 0, // level 2: boundary at 4 lists
    255, 255, 255, 255, // level 3: sentinel
    2, 0, 0, 0, // level 3: boundary at 2 arrays (nulls included)
    3, 0, 0, 0, // level 3: boundary at 3 arrays (nulls included)
    4, 0, 0, 0, // level 3: boundary at 4 arrays (nulls included)
    6, 0, 0, 0, // level 3: boundary at 6 arrays (nulls included)
    1, 0, // valid payload[0] = 1
    2, 0, // valid payload[1] = 2
    5, 0, // valid payload[2] = 5
    6, 0, // valid payload[3] = 6
    7, 0, // valid payload[4] = 7
    8, 0, // valid payload[5] = 8
    11, 0, // valid payload[6] = 11
    12, 0,  // valid payload[7] = 12
    45, // validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
];
pub const NESTED_5_OPTION_FIXED_U16_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionU16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_u16() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::U16);
    let data = nested_5_option_fixed_value([
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::U16(vec![1, 2].into())),
            PyroValue::Null,
        ]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::U16(
            vec![5, 6].into(),
        ))]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::U16(
            vec![7, 8].into(),
        ))]),
        PyroValue::List(vec![
            PyroValue::Null,
            PyroValue::PrimitiveList(PrimitiveValueList::U16(vec![11, 12].into())),
        ]),
    ]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_U16,
        &NESTED_5_OPTION_FIXED_U16_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionI16
// ---------------------------------------------------------------------------

pub const NESTED_5_OPTION_FIXED_I16: [u8; 105] = [
    20, 0, 0, 0, // topology start pointer
    17, 0, 0, 0, // topology word count (4 levels, sentinels included)
    88, 0, 0, 0, // payload start pointer
    4, 0, 0, 0, // valid arrays
    6, 0, 0, 0, // total arrays (nulls included)
    255, 255, 255, 255, // level 0: sentinel
    2, 0, 0, 0, // level 0: boundary at 2 lists
    3, 0, 0, 0, // level 0: boundary at 3 lists
    255, 255, 255, 255, // level 1: sentinel
    2, 0, 0, 0, // level 1: boundary at 2 lists
    3, 0, 0, 0, // level 1: boundary at 3 lists
    4, 0, 0, 0, // level 1: boundary at 4 lists
    255, 255, 255, 255, // level 2: sentinel
    1, 0, 0, 0, // level 2: boundary at 1 lists
    2, 0, 0, 0, // level 2: boundary at 2 lists
    3, 0, 0, 0, // level 2: boundary at 3 lists
    4, 0, 0, 0, // level 2: boundary at 4 lists
    255, 255, 255, 255, // level 3: sentinel
    2, 0, 0, 0, // level 3: boundary at 2 arrays (nulls included)
    3, 0, 0, 0, // level 3: boundary at 3 arrays (nulls included)
    4, 0, 0, 0, // level 3: boundary at 4 arrays (nulls included)
    6, 0, 0, 0, // level 3: boundary at 6 arrays (nulls included)
    255, 255, // valid payload[0] = -1
    2, 0, // valid payload[1] = 2
    251, 255, // valid payload[2] = -5
    6, 0, // valid payload[3] = 6
    249, 255, // valid payload[4] = -7
    8, 0, // valid payload[5] = 8
    245, 255, // valid payload[6] = -11
    12, 0,  // valid payload[7] = 12
    45, // validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
];
pub const NESTED_5_OPTION_FIXED_I16_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionI16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_i16() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::I16);
    let data = nested_5_option_fixed_value([
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::I16(vec![-1, 2].into())),
            PyroValue::Null,
        ]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::I16(
            vec![-5, 6].into(),
        ))]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::I16(
            vec![-7, 8].into(),
        ))]),
        PyroValue::List(vec![
            PyroValue::Null,
            PyroValue::PrimitiveList(PrimitiveValueList::I16(vec![-11, 12].into())),
        ]),
    ]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_I16,
        &NESTED_5_OPTION_FIXED_I16_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionU32
// ---------------------------------------------------------------------------

pub const NESTED_5_OPTION_FIXED_U32: [u8; 121] = [
    20, 0, 0, 0, // topology start pointer
    17, 0, 0, 0, // topology word count (4 levels, sentinels included)
    88, 0, 0, 0, // payload start pointer
    4, 0, 0, 0, // valid arrays
    6, 0, 0, 0, // total arrays (nulls included)
    255, 255, 255, 255, // level 0: sentinel
    2, 0, 0, 0, // level 0: boundary at 2 lists
    3, 0, 0, 0, // level 0: boundary at 3 lists
    255, 255, 255, 255, // level 1: sentinel
    2, 0, 0, 0, // level 1: boundary at 2 lists
    3, 0, 0, 0, // level 1: boundary at 3 lists
    4, 0, 0, 0, // level 1: boundary at 4 lists
    255, 255, 255, 255, // level 2: sentinel
    1, 0, 0, 0, // level 2: boundary at 1 lists
    2, 0, 0, 0, // level 2: boundary at 2 lists
    3, 0, 0, 0, // level 2: boundary at 3 lists
    4, 0, 0, 0, // level 2: boundary at 4 lists
    255, 255, 255, 255, // level 3: sentinel
    2, 0, 0, 0, // level 3: boundary at 2 arrays (nulls included)
    3, 0, 0, 0, // level 3: boundary at 3 arrays (nulls included)
    4, 0, 0, 0, // level 3: boundary at 4 arrays (nulls included)
    6, 0, 0, 0, // level 3: boundary at 6 arrays (nulls included)
    1, 0, 0, 0, // valid payload[0] = 1
    2, 0, 0, 0, // valid payload[1] = 2
    5, 0, 0, 0, // valid payload[2] = 5
    6, 0, 0, 0, // valid payload[3] = 6
    7, 0, 0, 0, // valid payload[4] = 7
    8, 0, 0, 0, // valid payload[5] = 8
    11, 0, 0, 0, // valid payload[6] = 11
    12, 0, 0, 0,  // valid payload[7] = 12
    45, // validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
];
pub const NESTED_5_OPTION_FIXED_U32_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionU32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_u32() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::U32);
    let data = nested_5_option_fixed_value([
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::U32(vec![1, 2].into())),
            PyroValue::Null,
        ]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::U32(
            vec![5, 6].into(),
        ))]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::U32(
            vec![7, 8].into(),
        ))]),
        PyroValue::List(vec![
            PyroValue::Null,
            PyroValue::PrimitiveList(PrimitiveValueList::U32(vec![11, 12].into())),
        ]),
    ]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_U32,
        &NESTED_5_OPTION_FIXED_U32_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionI32
// ---------------------------------------------------------------------------

pub const NESTED_5_OPTION_FIXED_I32: [u8; 121] = [
    20, 0, 0, 0, // topology start pointer
    17, 0, 0, 0, // topology word count (4 levels, sentinels included)
    88, 0, 0, 0, // payload start pointer
    4, 0, 0, 0, // valid arrays
    6, 0, 0, 0, // total arrays (nulls included)
    255, 255, 255, 255, // level 0: sentinel
    2, 0, 0, 0, // level 0: boundary at 2 lists
    3, 0, 0, 0, // level 0: boundary at 3 lists
    255, 255, 255, 255, // level 1: sentinel
    2, 0, 0, 0, // level 1: boundary at 2 lists
    3, 0, 0, 0, // level 1: boundary at 3 lists
    4, 0, 0, 0, // level 1: boundary at 4 lists
    255, 255, 255, 255, // level 2: sentinel
    1, 0, 0, 0, // level 2: boundary at 1 lists
    2, 0, 0, 0, // level 2: boundary at 2 lists
    3, 0, 0, 0, // level 2: boundary at 3 lists
    4, 0, 0, 0, // level 2: boundary at 4 lists
    255, 255, 255, 255, // level 3: sentinel
    2, 0, 0, 0, // level 3: boundary at 2 arrays (nulls included)
    3, 0, 0, 0, // level 3: boundary at 3 arrays (nulls included)
    4, 0, 0, 0, // level 3: boundary at 4 arrays (nulls included)
    6, 0, 0, 0, // level 3: boundary at 6 arrays (nulls included)
    255, 255, 255, 255, // valid payload[0] = -1
    2, 0, 0, 0, // valid payload[1] = 2
    251, 255, 255, 255, // valid payload[2] = -5
    6, 0, 0, 0, // valid payload[3] = 6
    249, 255, 255, 255, // valid payload[4] = -7
    8, 0, 0, 0, // valid payload[5] = 8
    245, 255, 255, 255, // valid payload[6] = -11
    12, 0, 0, 0,  // valid payload[7] = 12
    45, // validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
];
pub const NESTED_5_OPTION_FIXED_I32_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionI32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_i32() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::I32);
    let data = nested_5_option_fixed_value([
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::I32(vec![-1, 2].into())),
            PyroValue::Null,
        ]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::I32(
            vec![-5, 6].into(),
        ))]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::I32(
            vec![-7, 8].into(),
        ))]),
        PyroValue::List(vec![
            PyroValue::Null,
            PyroValue::PrimitiveList(PrimitiveValueList::I32(vec![-11, 12].into())),
        ]),
    ]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_I32,
        &NESTED_5_OPTION_FIXED_I32_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionU64
// ---------------------------------------------------------------------------

pub const NESTED_5_OPTION_FIXED_U64: [u8; 153] = [
    20, 0, 0, 0, // topology start pointer
    17, 0, 0, 0, // topology word count (4 levels, sentinels included)
    88, 0, 0, 0, // payload start pointer
    4, 0, 0, 0, // valid arrays
    6, 0, 0, 0, // total arrays (nulls included)
    255, 255, 255, 255, // level 0: sentinel
    2, 0, 0, 0, // level 0: boundary at 2 lists
    3, 0, 0, 0, // level 0: boundary at 3 lists
    255, 255, 255, 255, // level 1: sentinel
    2, 0, 0, 0, // level 1: boundary at 2 lists
    3, 0, 0, 0, // level 1: boundary at 3 lists
    4, 0, 0, 0, // level 1: boundary at 4 lists
    255, 255, 255, 255, // level 2: sentinel
    1, 0, 0, 0, // level 2: boundary at 1 lists
    2, 0, 0, 0, // level 2: boundary at 2 lists
    3, 0, 0, 0, // level 2: boundary at 3 lists
    4, 0, 0, 0, // level 2: boundary at 4 lists
    255, 255, 255, 255, // level 3: sentinel
    2, 0, 0, 0, // level 3: boundary at 2 arrays (nulls included)
    3, 0, 0, 0, // level 3: boundary at 3 arrays (nulls included)
    4, 0, 0, 0, // level 3: boundary at 4 arrays (nulls included)
    6, 0, 0, 0, // level 3: boundary at 6 arrays (nulls included)
    1, 0, 0, 0, 0, 0, 0, 0, // valid payload[0] = 1
    2, 0, 0, 0, 0, 0, 0, 0, // valid payload[1] = 2
    5, 0, 0, 0, 0, 0, 0, 0, // valid payload[2] = 5
    6, 0, 0, 0, 0, 0, 0, 0, // valid payload[3] = 6
    7, 0, 0, 0, 0, 0, 0, 0, // valid payload[4] = 7
    8, 0, 0, 0, 0, 0, 0, 0, // valid payload[5] = 8
    11, 0, 0, 0, 0, 0, 0, 0, // valid payload[6] = 11
    12, 0, 0, 0, 0, 0, 0, 0,  // valid payload[7] = 12
    45, // validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
];
pub const NESTED_5_OPTION_FIXED_U64_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionU64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_u64() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::U64);
    let data = nested_5_option_fixed_value([
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::U64(vec![1, 2].into())),
            PyroValue::Null,
        ]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::U64(
            vec![5, 6].into(),
        ))]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::U64(
            vec![7, 8].into(),
        ))]),
        PyroValue::List(vec![
            PyroValue::Null,
            PyroValue::PrimitiveList(PrimitiveValueList::U64(vec![11, 12].into())),
        ]),
    ]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_U64,
        &NESTED_5_OPTION_FIXED_U64_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionI64
// ---------------------------------------------------------------------------

pub const NESTED_5_OPTION_FIXED_I64: [u8; 153] = [
    20, 0, 0, 0, // topology start pointer
    17, 0, 0, 0, // topology word count (4 levels, sentinels included)
    88, 0, 0, 0, // payload start pointer
    4, 0, 0, 0, // valid arrays
    6, 0, 0, 0, // total arrays (nulls included)
    255, 255, 255, 255, // level 0: sentinel
    2, 0, 0, 0, // level 0: boundary at 2 lists
    3, 0, 0, 0, // level 0: boundary at 3 lists
    255, 255, 255, 255, // level 1: sentinel
    2, 0, 0, 0, // level 1: boundary at 2 lists
    3, 0, 0, 0, // level 1: boundary at 3 lists
    4, 0, 0, 0, // level 1: boundary at 4 lists
    255, 255, 255, 255, // level 2: sentinel
    1, 0, 0, 0, // level 2: boundary at 1 lists
    2, 0, 0, 0, // level 2: boundary at 2 lists
    3, 0, 0, 0, // level 2: boundary at 3 lists
    4, 0, 0, 0, // level 2: boundary at 4 lists
    255, 255, 255, 255, // level 3: sentinel
    2, 0, 0, 0, // level 3: boundary at 2 arrays (nulls included)
    3, 0, 0, 0, // level 3: boundary at 3 arrays (nulls included)
    4, 0, 0, 0, // level 3: boundary at 4 arrays (nulls included)
    6, 0, 0, 0, // level 3: boundary at 6 arrays (nulls included)
    255, 255, 255, 255, 255, 255, 255, 255, // valid payload[0] = -1
    2, 0, 0, 0, 0, 0, 0, 0, // valid payload[1] = 2
    251, 255, 255, 255, 255, 255, 255, 255, // valid payload[2] = -5
    6, 0, 0, 0, 0, 0, 0, 0, // valid payload[3] = 6
    249, 255, 255, 255, 255, 255, 255, 255, // valid payload[4] = -7
    8, 0, 0, 0, 0, 0, 0, 0, // valid payload[5] = 8
    245, 255, 255, 255, 255, 255, 255, 255, // valid payload[6] = -11
    12, 0, 0, 0, 0, 0, 0, 0,  // valid payload[7] = 12
    45, // validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
];
pub const NESTED_5_OPTION_FIXED_I64_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionI64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_i64() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::I64);
    let data = nested_5_option_fixed_value([
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::I64(vec![-1, 2].into())),
            PyroValue::Null,
        ]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::I64(
            vec![-5, 6].into(),
        ))]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::I64(
            vec![-7, 8].into(),
        ))]),
        PyroValue::List(vec![
            PyroValue::Null,
            PyroValue::PrimitiveList(PrimitiveValueList::I64(vec![-11, 12].into())),
        ]),
    ]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_I64,
        &NESTED_5_OPTION_FIXED_I64_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionF16
// ---------------------------------------------------------------------------

pub const NESTED_5_OPTION_FIXED_F16: [u8; 105] = [
    20, 0, 0, 0, // topology start pointer
    17, 0, 0, 0, // topology word count (4 levels, sentinels included)
    88, 0, 0, 0, // payload start pointer
    4, 0, 0, 0, // valid arrays
    6, 0, 0, 0, // total arrays (nulls included)
    255, 255, 255, 255, // level 0: sentinel
    2, 0, 0, 0, // level 0: boundary at 2 lists
    3, 0, 0, 0, // level 0: boundary at 3 lists
    255, 255, 255, 255, // level 1: sentinel
    2, 0, 0, 0, // level 1: boundary at 2 lists
    3, 0, 0, 0, // level 1: boundary at 3 lists
    4, 0, 0, 0, // level 1: boundary at 4 lists
    255, 255, 255, 255, // level 2: sentinel
    1, 0, 0, 0, // level 2: boundary at 1 lists
    2, 0, 0, 0, // level 2: boundary at 2 lists
    3, 0, 0, 0, // level 2: boundary at 3 lists
    4, 0, 0, 0, // level 2: boundary at 4 lists
    255, 255, 255, 255, // level 3: sentinel
    2, 0, 0, 0, // level 3: boundary at 2 arrays (nulls included)
    3, 0, 0, 0, // level 3: boundary at 3 arrays (nulls included)
    4, 0, 0, 0, // level 3: boundary at 4 arrays (nulls included)
    6, 0, 0, 0, // level 3: boundary at 6 arrays (nulls included)
    0, 62, // valid payload[0] = 1.5
    0, 192, // valid payload[1] = -2.0
    128, 197, // valid payload[2] = -5.5
    0, 70, // valid payload[3] = 6.0
    128, 71, // valid payload[4] = 7.5
    0, 200, // valid payload[5] = -8.0
    192, 201, // valid payload[6] = -11.5
    0, 74, // valid payload[7] = 12.0
    45, // validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
];
pub const NESTED_5_OPTION_FIXED_F16_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionF16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_f16() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::F16);
    let data = nested_5_option_fixed_value([
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::F16(
                vec![f16::from_f32(1.5), f16::from_f32(-2.0)].into(),
            )),
            PyroValue::Null,
        ]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::F16(
            vec![f16::from_f32(-5.5), f16::from_f32(6.0)].into(),
        ))]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::F16(
            vec![f16::from_f32(7.5), f16::from_f32(-8.0)].into(),
        ))]),
        PyroValue::List(vec![
            PyroValue::Null,
            PyroValue::PrimitiveList(PrimitiveValueList::F16(
                vec![f16::from_f32(-11.5), f16::from_f32(12.0)].into(),
            )),
        ]),
    ]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_F16,
        &NESTED_5_OPTION_FIXED_F16_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionF32
// ---------------------------------------------------------------------------

pub const NESTED_5_OPTION_FIXED_F32: [u8; 121] = [
    20, 0, 0, 0, // topology start pointer
    17, 0, 0, 0, // topology word count (4 levels, sentinels included)
    88, 0, 0, 0, // payload start pointer
    4, 0, 0, 0, // valid arrays
    6, 0, 0, 0, // total arrays (nulls included)
    255, 255, 255, 255, // level 0: sentinel
    2, 0, 0, 0, // level 0: boundary at 2 lists
    3, 0, 0, 0, // level 0: boundary at 3 lists
    255, 255, 255, 255, // level 1: sentinel
    2, 0, 0, 0, // level 1: boundary at 2 lists
    3, 0, 0, 0, // level 1: boundary at 3 lists
    4, 0, 0, 0, // level 1: boundary at 4 lists
    255, 255, 255, 255, // level 2: sentinel
    1, 0, 0, 0, // level 2: boundary at 1 lists
    2, 0, 0, 0, // level 2: boundary at 2 lists
    3, 0, 0, 0, // level 2: boundary at 3 lists
    4, 0, 0, 0, // level 2: boundary at 4 lists
    255, 255, 255, 255, // level 3: sentinel
    2, 0, 0, 0, // level 3: boundary at 2 arrays (nulls included)
    3, 0, 0, 0, // level 3: boundary at 3 arrays (nulls included)
    4, 0, 0, 0, // level 3: boundary at 4 arrays (nulls included)
    6, 0, 0, 0, // level 3: boundary at 6 arrays (nulls included)
    0, 0, 192, 63, // valid payload[0] = 1.5
    0, 0, 0, 192, // valid payload[1] = -2.0
    0, 0, 176, 192, // valid payload[2] = -5.5
    0, 0, 192, 64, // valid payload[3] = 6.0
    0, 0, 240, 64, // valid payload[4] = 7.5
    0, 0, 0, 193, // valid payload[5] = -8.0
    0, 0, 56, 193, // valid payload[6] = -11.5
    0, 0, 64, 65, // valid payload[7] = 12.0
    45, // validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
];
pub const NESTED_5_OPTION_FIXED_F32_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionF32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_f32() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::F32);
    let data = nested_5_option_fixed_value([
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::F32(vec![1.5, -2.0].into())),
            PyroValue::Null,
        ]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::F32(
            vec![-5.5, 6.0].into(),
        ))]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::F32(
            vec![7.5, -8.0].into(),
        ))]),
        PyroValue::List(vec![
            PyroValue::Null,
            PyroValue::PrimitiveList(PrimitiveValueList::F32(vec![-11.5, 12.0].into())),
        ]),
    ]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_F32,
        &NESTED_5_OPTION_FIXED_F32_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionF64
// ---------------------------------------------------------------------------

pub const NESTED_5_OPTION_FIXED_F64: [u8; 153] = [
    20, 0, 0, 0, // topology start pointer
    17, 0, 0, 0, // topology word count (4 levels, sentinels included)
    88, 0, 0, 0, // payload start pointer
    4, 0, 0, 0, // valid arrays
    6, 0, 0, 0, // total arrays (nulls included)
    255, 255, 255, 255, // level 0: sentinel
    2, 0, 0, 0, // level 0: boundary at 2 lists
    3, 0, 0, 0, // level 0: boundary at 3 lists
    255, 255, 255, 255, // level 1: sentinel
    2, 0, 0, 0, // level 1: boundary at 2 lists
    3, 0, 0, 0, // level 1: boundary at 3 lists
    4, 0, 0, 0, // level 1: boundary at 4 lists
    255, 255, 255, 255, // level 2: sentinel
    1, 0, 0, 0, // level 2: boundary at 1 lists
    2, 0, 0, 0, // level 2: boundary at 2 lists
    3, 0, 0, 0, // level 2: boundary at 3 lists
    4, 0, 0, 0, // level 2: boundary at 4 lists
    255, 255, 255, 255, // level 3: sentinel
    2, 0, 0, 0, // level 3: boundary at 2 arrays (nulls included)
    3, 0, 0, 0, // level 3: boundary at 3 arrays (nulls included)
    4, 0, 0, 0, // level 3: boundary at 4 arrays (nulls included)
    6, 0, 0, 0, // level 3: boundary at 6 arrays (nulls included)
    0, 0, 0, 0, 0, 0, 248, 63, // valid payload[0] = 1.5
    0, 0, 0, 0, 0, 0, 0, 192, // valid payload[1] = -2.0
    0, 0, 0, 0, 0, 0, 22, 192, // valid payload[2] = -5.5
    0, 0, 0, 0, 0, 0, 24, 64, // valid payload[3] = 6.0
    0, 0, 0, 0, 0, 0, 30, 64, // valid payload[4] = 7.5
    0, 0, 0, 0, 0, 0, 32, 192, // valid payload[5] = -8.0
    0, 0, 0, 0, 0, 0, 39, 192, // valid payload[6] = -11.5
    0, 0, 0, 0, 0, 0, 40, 64, // valid payload[7] = 12.0
    45, // validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
];
pub const NESTED_5_OPTION_FIXED_F64_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionF64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_f64() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::F64);
    let data = nested_5_option_fixed_value([
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::F64(vec![1.5, -2.0].into())),
            PyroValue::Null,
        ]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::F64(
            vec![-5.5, 6.0].into(),
        ))]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::F64(
            vec![7.5, -8.0].into(),
        ))]),
        PyroValue::List(vec![
            PyroValue::Null,
            PyroValue::PrimitiveList(PrimitiveValueList::F64(vec![-11.5, 12.0].into())),
        ]),
    ]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_F64,
        &NESTED_5_OPTION_FIXED_F64_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionTimestamp
// ---------------------------------------------------------------------------

pub const NESTED_5_OPTION_FIXED_TIMESTAMP: [u8; 225] = [
    20, 0, 0, 0, // topology start pointer
    17, 0, 0, 0, // topology word count (4 levels, sentinels included)
    96, 0, 0, 0, // payload start pointer
    4, 0, 0, 0, // valid arrays
    6, 0, 0, 0, // total arrays (nulls included)
    255, 255, 255, 255, // level 0: sentinel
    2, 0, 0, 0, // level 0: boundary at 2 lists
    3, 0, 0, 0, // level 0: boundary at 3 lists
    255, 255, 255, 255, // level 1: sentinel
    2, 0, 0, 0, // level 1: boundary at 2 lists
    3, 0, 0, 0, // level 1: boundary at 3 lists
    4, 0, 0, 0, // level 1: boundary at 4 lists
    255, 255, 255, 255, // level 2: sentinel
    1, 0, 0, 0, // level 2: boundary at 1 lists
    2, 0, 0, 0, // level 2: boundary at 2 lists
    3, 0, 0, 0, // level 2: boundary at 3 lists
    4, 0, 0, 0, // level 2: boundary at 4 lists
    255, 255, 255, 255, // level 3: sentinel
    2, 0, 0, 0, // level 3: boundary at 2 arrays (nulls included)
    3, 0, 0, 0, // level 3: boundary at 3 arrays (nulls included)
    4, 0, 0, 0, // level 3: boundary at 4 arrays (nulls included)
    6, 0, 0, 0, // level 3: boundary at 6 arrays (nulls included)
    0, 0, 0, 0, 0, 0, 0, 0, // alignment padding to 16 bytes before payload
    0, 47, 104, 89, 0, 0, 0, 0, // valid payload[0] = Time(1_500_000_000), low 8 bytes
    0, 0, 0, 0, 0, 0, 0, 0, // valid payload[0], high 8 bytes
    0, 108, 202, 136, 255, 255, 255,
    255, // valid payload[1] = Time(-2_000_000_000), low 8 bytes
    255, 255, 255, 255, 255, 255, 255, 255, // valid payload[1], high 8 bytes
    0, 169, 44, 184, 254, 255, 255,
    255, // valid payload[2] = Time(-5_500_000_000), low 8 bytes
    255, 255, 255, 255, 255, 255, 255, 255, // valid payload[2], high 8 bytes
    0, 188, 160, 101, 1, 0, 0, 0, // valid payload[3] = Time(6_000_000_000), low 8 bytes
    0, 0, 0, 0, 0, 0, 0, 0, // valid payload[3], high 8 bytes
    0, 235, 8, 191, 1, 0, 0, 0, // valid payload[4] = Time(7_500_000_000), low 8 bytes
    0, 0, 0, 0, 0, 0, 0, 0, // valid payload[4], high 8 bytes
    0, 176, 41, 35, 254, 255, 255,
    255, // valid payload[5] = Time(-8_000_000_000), low 8 bytes
    255, 255, 255, 255, 255, 255, 255, 255, // valid payload[5], high 8 bytes
    0, 237, 139, 82, 253, 255, 255,
    255, // valid payload[6] = Time(-11_500_000_000), low 8 bytes
    255, 255, 255, 255, 255, 255, 255, 255, // valid payload[6], high 8 bytes
    0, 120, 65, 203, 2, 0, 0, 0, // valid payload[7] = Time(12_000_000_000), low 8 bytes
    0, 0, 0, 0, 0, 0, 0, 0,  // valid payload[7], high 8 bytes
    45, // validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
];
pub const NESTED_5_OPTION_FIXED_TIMESTAMP_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionTimestamp);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_timestamp() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::Timestamp);
    let data = nested_5_option_fixed_value([
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::Timestamp(
                vec![Time(1_500_000_000), Time(-2_000_000_000)].into(),
            )),
            PyroValue::Null,
        ]),
        PyroValue::List(vec![PyroValue::PrimitiveList(
            PrimitiveValueList::Timestamp(vec![Time(-5_500_000_000), Time(6_000_000_000)].into()),
        )]),
        PyroValue::List(vec![PyroValue::PrimitiveList(
            PrimitiveValueList::Timestamp(vec![Time(7_500_000_000), Time(-8_000_000_000)].into()),
        )]),
        PyroValue::List(vec![
            PyroValue::Null,
            PyroValue::PrimitiveList(PrimitiveValueList::Timestamp(
                vec![Time(-11_500_000_000), Time(12_000_000_000)].into(),
            )),
        ]),
    ]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_TIMESTAMP,
        &NESTED_5_OPTION_FIXED_TIMESTAMP_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionBool
// ---------------------------------------------------------------------------

pub const NESTED_5_OPTION_FIXED_BOOL: [u8; 90] = [
    20, 0, 0, 0, // topology start pointer
    17, 0, 0, 0, // topology word count (4 levels, sentinels included)
    88, 0, 0, 0, // payload start pointer
    4, 0, 0, 0, // valid arrays
    6, 0, 0, 0, // total arrays (nulls included)
    255, 255, 255, 255, // level 0: sentinel
    2, 0, 0, 0, // level 0: boundary at 2 lists
    3, 0, 0, 0, // level 0: boundary at 3 lists
    255, 255, 255, 255, // level 1: sentinel
    2, 0, 0, 0, // level 1: boundary at 2 lists
    3, 0, 0, 0, // level 1: boundary at 3 lists
    4, 0, 0, 0, // level 1: boundary at 4 lists
    255, 255, 255, 255, // level 2: sentinel
    1, 0, 0, 0, // level 2: boundary at 1 lists
    2, 0, 0, 0, // level 2: boundary at 2 lists
    3, 0, 0, 0, // level 2: boundary at 3 lists
    4, 0, 0, 0, // level 2: boundary at 4 lists
    255, 255, 255, 255, // level 3: sentinel
    2, 0, 0, 0, // level 3: boundary at 2 arrays (nulls included)
    3, 0, 0, 0, // level 3: boundary at 3 arrays (nulls included)
    4, 0, 0, 0, // level 3: boundary at 4 arrays (nulls included)
    6, 0, 0, 0,  // level 3: boundary at 6 arrays (nulls included)
    37, // valid payload bits 0..8 (LSB first) = 1, 0, 1, 0, 0, 1, 0, 0
    45, // validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
];
pub const NESTED_5_OPTION_FIXED_BOOL_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionBool);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_bool() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::Bool);
    let data = nested_5_option_fixed_value([
        PyroValue::List(vec![PyroValue::from(vec![true, false]), PyroValue::Null]),
        PyroValue::List(vec![PyroValue::from(vec![true, false])]),
        PyroValue::List(vec![PyroValue::from(vec![false, true])]),
        PyroValue::List(vec![PyroValue::Null, PyroValue::from(vec![false, false])]),
    ]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_BOOL,
        &NESTED_5_OPTION_FIXED_BOOL_INODES,
    );
}

// `[bool; 8]`: the same shape as `[bool; 2]` above, but every array
// fills a whole byte, so the payload is the arrays' bytes back to back.
pub const NESTED_5_OPTION_FIXED_BOOL_8: [u8; 93] = [
    20, 0, 0, 0, // topology start pointer
    17, 0, 0, 0, // topology word count (4 levels, sentinels included)
    88, 0, 0, 0, // payload start pointer
    4, 0, 0, 0, // valid arrays
    6, 0, 0, 0, // total arrays (nulls included)
    255, 255, 255, 255, // level 0: sentinel
    2, 0, 0, 0, // level 0: boundary at 2 lists
    3, 0, 0, 0, // level 0: boundary at 3 lists
    255, 255, 255, 255, // level 1: sentinel
    2, 0, 0, 0, // level 1: boundary at 2 lists
    3, 0, 0, 0, // level 1: boundary at 3 lists
    4, 0, 0, 0, // level 1: boundary at 4 lists
    255, 255, 255, 255, // level 2: sentinel
    1, 0, 0, 0, // level 2: boundary at 1 lists
    2, 0, 0, 0, // level 2: boundary at 2 lists
    3, 0, 0, 0, // level 2: boundary at 3 lists
    4, 0, 0, 0, // level 2: boundary at 4 lists
    255, 255, 255, 255, // level 3: sentinel
    2, 0, 0, 0, // level 3: boundary at 2 arrays (nulls included)
    3, 0, 0, 0, // level 3: boundary at 3 arrays (nulls included)
    4, 0, 0, 0, // level 3: boundary at 4 arrays (nulls included)
    6, 0, 0, 0,   // level 3: boundary at 6 arrays (nulls included)
    1,   // payload: array 0 (LSB first)
    128, // payload: array 1 (LSB first)
    255, // payload: array 2 (LSB first)
    0,   // payload: array 3 (LSB first)
    45,  // validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
];
pub const NESTED_5_OPTION_FIXED_BOOL_8_INODES: [Inode; 1] =
    nested_5_option_fixed_sized_inode(WireKind::OptionBool, 8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_bool_8() {
    let schema = nested_5_option_fixed_sized_schema(PrimitiveDataType::Bool, 8);
    let data = nested_5_option_fixed_value([
        PyroValue::List(vec![
            PyroValue::from(vec![true, false, false, false, false, false, false, false]),
            PyroValue::Null,
        ]),
        PyroValue::List(vec![PyroValue::from(vec![
            false, false, false, false, false, false, false, true,
        ])]),
        PyroValue::List(vec![PyroValue::from(vec![
            true, true, true, true, true, true, true, true,
        ])]),
        PyroValue::List(vec![
            PyroValue::Null,
            PyroValue::from(vec![false, false, false, false, false, false, false, false]),
        ]),
    ]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_BOOL_8,
        &NESTED_5_OPTION_FIXED_BOOL_8_INODES,
    );
}

// `[bool; 16]`: the same shape as `[bool; 2]` above, but every array
// fills two whole bytes, so the payload is the arrays' bytes back to back.
pub const NESTED_5_OPTION_FIXED_BOOL_16: [u8; 97] = [
    20, 0, 0, 0, // topology start pointer
    17, 0, 0, 0, // topology word count (4 levels, sentinels included)
    88, 0, 0, 0, // payload start pointer
    4, 0, 0, 0, // valid arrays
    6, 0, 0, 0, // total arrays (nulls included)
    255, 255, 255, 255, // level 0: sentinel
    2, 0, 0, 0, // level 0: boundary at 2 lists
    3, 0, 0, 0, // level 0: boundary at 3 lists
    255, 255, 255, 255, // level 1: sentinel
    2, 0, 0, 0, // level 1: boundary at 2 lists
    3, 0, 0, 0, // level 1: boundary at 3 lists
    4, 0, 0, 0, // level 1: boundary at 4 lists
    255, 255, 255, 255, // level 2: sentinel
    1, 0, 0, 0, // level 2: boundary at 1 lists
    2, 0, 0, 0, // level 2: boundary at 2 lists
    3, 0, 0, 0, // level 2: boundary at 3 lists
    4, 0, 0, 0, // level 2: boundary at 4 lists
    255, 255, 255, 255, // level 3: sentinel
    2, 0, 0, 0, // level 3: boundary at 2 arrays (nulls included)
    3, 0, 0, 0, // level 3: boundary at 3 arrays (nulls included)
    4, 0, 0, 0, // level 3: boundary at 4 arrays (nulls included)
    6, 0, 0, 0,   // level 3: boundary at 6 arrays (nulls included)
    1,   // payload: array 0, bits 0..8 (LSB first)
    0,   // payload: array 0, bits 8..16 (LSB first)
    0,   // payload: array 1, bits 0..8 (LSB first)
    128, // payload: array 1, bits 8..16 (LSB first)
    255, // payload: array 2, bits 0..8 (LSB first)
    255, // payload: array 2, bits 8..16 (LSB first)
    0,   // payload: array 3, bits 0..8 (LSB first)
    0,   // payload: array 3, bits 8..16 (LSB first)
    45,  // validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
];
pub const NESTED_5_OPTION_FIXED_BOOL_16_INODES: [Inode; 1] =
    nested_5_option_fixed_sized_inode(WireKind::OptionBool, 16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_bool_16() {
    let schema = nested_5_option_fixed_sized_schema(PrimitiveDataType::Bool, 16);
    let data = nested_5_option_fixed_value([
        PyroValue::List(vec![
            PyroValue::from(vec![
                true, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false, false,
            ]),
            PyroValue::Null,
        ]),
        PyroValue::List(vec![PyroValue::from(vec![
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, true,
        ])]),
        PyroValue::List(vec![PyroValue::from(vec![
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true,
        ])]),
        PyroValue::List(vec![
            PyroValue::Null,
            PyroValue::from(vec![
                false, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false, false,
            ]),
        ]),
    ]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_BOOL_16,
        &NESTED_5_OPTION_FIXED_BOOL_16_INODES,
    );
}

// ===========================================================================
// Empty lists at every depth
// ===========================================================================
//
// `_EMPTY_d` is the list at depth `d` left empty, every shallower list
// holding exactly one element: `_EMPTY_0` = `[]`, `_EMPTY_1` = `[[]]`, `_EMPTY_2` = `[[[]]]`, `_EMPTY_3` = `[[[[]]]]`, `_EMPTY_4` = `[[[[[]]]]]`.
// No leaf is written, so each buffer is the fixed header, whatever
// topology levels still have entries (a level with no lists below it is
// omitted, sentinel included), then the padding up to the payload's
// alignment. Byte ranges are `[start..end)` offsets into the buffer.
// Captured from the value writer's output.

// ---------------------------------------------------------------------------
// OptionU8, [T; 2]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_5_OPTION_FIXED_U8_EMPTY_0: [u8; 20] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    0, 0, 0, 0, // [4..8] topology word count (0 levels, sentinels included)
    20, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
];
pub const NESTED_5_OPTION_FIXED_U8_EMPTY_0_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionU8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_u8_empty_0() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::U8);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_U8_EMPTY_0,
        &NESTED_5_OPTION_FIXED_U8_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_5_OPTION_FIXED_U8_EMPTY_1: [u8; 28] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    2, 0, 0, 0, // [4..8] topology word count (1 level, sentinels included)
    28, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    0, 0, 0, 0, // [24..28] level 0: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_U8_EMPTY_1_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionU8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_u8_empty_1() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::U8);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_U8_EMPTY_1,
        &NESTED_5_OPTION_FIXED_U8_EMPTY_1_INODES,
    );
}

/// `[[[]]]`
pub const NESTED_5_OPTION_FIXED_U8_EMPTY_2: [u8; 36] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    4, 0, 0, 0, // [4..8] topology word count (2 levels, sentinels included)
    36, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    0, 0, 0, 0, // [32..36] level 1: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_U8_EMPTY_2_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionU8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_u8_empty_2() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::U8);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_U8_EMPTY_2,
        &NESTED_5_OPTION_FIXED_U8_EMPTY_2_INODES,
    );
}

/// `[[[[]]]]`
pub const NESTED_5_OPTION_FIXED_U8_EMPTY_3: [u8; 44] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    6, 0, 0, 0, // [4..8] topology word count (3 levels, sentinels included)
    44, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    0, 0, 0, 0, // [40..44] level 2: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_U8_EMPTY_3_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionU8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_u8_empty_3() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::U8);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_U8_EMPTY_3,
        &NESTED_5_OPTION_FIXED_U8_EMPTY_3_INODES,
    );
}

/// `[[[[[]]]]]`
pub const NESTED_5_OPTION_FIXED_U8_EMPTY_4: [u8; 52] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    8, 0, 0, 0, // [4..8] topology word count (4 levels, sentinels included)
    52, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    1, 0, 0, 0, // [40..44] level 2: boundary at 1 lists
    255, 255, 255, 255, // [44..48] level 3: sentinel
    0, 0, 0, 0, // [48..52] level 3: boundary at 0 arrays (nulls included)
];
pub const NESTED_5_OPTION_FIXED_U8_EMPTY_4_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionU8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_u8_empty_4() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::U8);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![PyroValue::List(vec![])]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_U8_EMPTY_4,
        &NESTED_5_OPTION_FIXED_U8_EMPTY_4_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionI8, [T; 2]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_5_OPTION_FIXED_I8_EMPTY_0: [u8; 20] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    0, 0, 0, 0, // [4..8] topology word count (0 levels, sentinels included)
    20, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
];
pub const NESTED_5_OPTION_FIXED_I8_EMPTY_0_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionI8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_i8_empty_0() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::I8);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_I8_EMPTY_0,
        &NESTED_5_OPTION_FIXED_I8_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_5_OPTION_FIXED_I8_EMPTY_1: [u8; 28] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    2, 0, 0, 0, // [4..8] topology word count (1 level, sentinels included)
    28, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    0, 0, 0, 0, // [24..28] level 0: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_I8_EMPTY_1_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionI8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_i8_empty_1() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::I8);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_I8_EMPTY_1,
        &NESTED_5_OPTION_FIXED_I8_EMPTY_1_INODES,
    );
}

/// `[[[]]]`
pub const NESTED_5_OPTION_FIXED_I8_EMPTY_2: [u8; 36] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    4, 0, 0, 0, // [4..8] topology word count (2 levels, sentinels included)
    36, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    0, 0, 0, 0, // [32..36] level 1: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_I8_EMPTY_2_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionI8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_i8_empty_2() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::I8);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_I8_EMPTY_2,
        &NESTED_5_OPTION_FIXED_I8_EMPTY_2_INODES,
    );
}

/// `[[[[]]]]`
pub const NESTED_5_OPTION_FIXED_I8_EMPTY_3: [u8; 44] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    6, 0, 0, 0, // [4..8] topology word count (3 levels, sentinels included)
    44, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    0, 0, 0, 0, // [40..44] level 2: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_I8_EMPTY_3_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionI8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_i8_empty_3() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::I8);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_I8_EMPTY_3,
        &NESTED_5_OPTION_FIXED_I8_EMPTY_3_INODES,
    );
}

/// `[[[[[]]]]]`
pub const NESTED_5_OPTION_FIXED_I8_EMPTY_4: [u8; 52] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    8, 0, 0, 0, // [4..8] topology word count (4 levels, sentinels included)
    52, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    1, 0, 0, 0, // [40..44] level 2: boundary at 1 lists
    255, 255, 255, 255, // [44..48] level 3: sentinel
    0, 0, 0, 0, // [48..52] level 3: boundary at 0 arrays (nulls included)
];
pub const NESTED_5_OPTION_FIXED_I8_EMPTY_4_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionI8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_i8_empty_4() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::I8);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![PyroValue::List(vec![])]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_I8_EMPTY_4,
        &NESTED_5_OPTION_FIXED_I8_EMPTY_4_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionU16, [T; 2]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_5_OPTION_FIXED_U16_EMPTY_0: [u8; 20] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    0, 0, 0, 0, // [4..8] topology word count (0 levels, sentinels included)
    20, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
];
pub const NESTED_5_OPTION_FIXED_U16_EMPTY_0_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionU16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_u16_empty_0() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::U16);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_U16_EMPTY_0,
        &NESTED_5_OPTION_FIXED_U16_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_5_OPTION_FIXED_U16_EMPTY_1: [u8; 28] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    2, 0, 0, 0, // [4..8] topology word count (1 level, sentinels included)
    28, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    0, 0, 0, 0, // [24..28] level 0: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_U16_EMPTY_1_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionU16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_u16_empty_1() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::U16);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_U16_EMPTY_1,
        &NESTED_5_OPTION_FIXED_U16_EMPTY_1_INODES,
    );
}

/// `[[[]]]`
pub const NESTED_5_OPTION_FIXED_U16_EMPTY_2: [u8; 36] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    4, 0, 0, 0, // [4..8] topology word count (2 levels, sentinels included)
    36, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    0, 0, 0, 0, // [32..36] level 1: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_U16_EMPTY_2_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionU16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_u16_empty_2() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::U16);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_U16_EMPTY_2,
        &NESTED_5_OPTION_FIXED_U16_EMPTY_2_INODES,
    );
}

/// `[[[[]]]]`
pub const NESTED_5_OPTION_FIXED_U16_EMPTY_3: [u8; 44] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    6, 0, 0, 0, // [4..8] topology word count (3 levels, sentinels included)
    44, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    0, 0, 0, 0, // [40..44] level 2: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_U16_EMPTY_3_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionU16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_u16_empty_3() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::U16);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_U16_EMPTY_3,
        &NESTED_5_OPTION_FIXED_U16_EMPTY_3_INODES,
    );
}

/// `[[[[[]]]]]`
pub const NESTED_5_OPTION_FIXED_U16_EMPTY_4: [u8; 52] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    8, 0, 0, 0, // [4..8] topology word count (4 levels, sentinels included)
    52, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    1, 0, 0, 0, // [40..44] level 2: boundary at 1 lists
    255, 255, 255, 255, // [44..48] level 3: sentinel
    0, 0, 0, 0, // [48..52] level 3: boundary at 0 arrays (nulls included)
];
pub const NESTED_5_OPTION_FIXED_U16_EMPTY_4_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionU16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_u16_empty_4() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::U16);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![PyroValue::List(vec![])]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_U16_EMPTY_4,
        &NESTED_5_OPTION_FIXED_U16_EMPTY_4_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionI16, [T; 2]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_5_OPTION_FIXED_I16_EMPTY_0: [u8; 20] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    0, 0, 0, 0, // [4..8] topology word count (0 levels, sentinels included)
    20, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
];
pub const NESTED_5_OPTION_FIXED_I16_EMPTY_0_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionI16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_i16_empty_0() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::I16);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_I16_EMPTY_0,
        &NESTED_5_OPTION_FIXED_I16_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_5_OPTION_FIXED_I16_EMPTY_1: [u8; 28] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    2, 0, 0, 0, // [4..8] topology word count (1 level, sentinels included)
    28, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    0, 0, 0, 0, // [24..28] level 0: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_I16_EMPTY_1_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionI16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_i16_empty_1() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::I16);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_I16_EMPTY_1,
        &NESTED_5_OPTION_FIXED_I16_EMPTY_1_INODES,
    );
}

/// `[[[]]]`
pub const NESTED_5_OPTION_FIXED_I16_EMPTY_2: [u8; 36] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    4, 0, 0, 0, // [4..8] topology word count (2 levels, sentinels included)
    36, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    0, 0, 0, 0, // [32..36] level 1: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_I16_EMPTY_2_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionI16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_i16_empty_2() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::I16);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_I16_EMPTY_2,
        &NESTED_5_OPTION_FIXED_I16_EMPTY_2_INODES,
    );
}

/// `[[[[]]]]`
pub const NESTED_5_OPTION_FIXED_I16_EMPTY_3: [u8; 44] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    6, 0, 0, 0, // [4..8] topology word count (3 levels, sentinels included)
    44, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    0, 0, 0, 0, // [40..44] level 2: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_I16_EMPTY_3_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionI16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_i16_empty_3() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::I16);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_I16_EMPTY_3,
        &NESTED_5_OPTION_FIXED_I16_EMPTY_3_INODES,
    );
}

/// `[[[[[]]]]]`
pub const NESTED_5_OPTION_FIXED_I16_EMPTY_4: [u8; 52] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    8, 0, 0, 0, // [4..8] topology word count (4 levels, sentinels included)
    52, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    1, 0, 0, 0, // [40..44] level 2: boundary at 1 lists
    255, 255, 255, 255, // [44..48] level 3: sentinel
    0, 0, 0, 0, // [48..52] level 3: boundary at 0 arrays (nulls included)
];
pub const NESTED_5_OPTION_FIXED_I16_EMPTY_4_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionI16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_i16_empty_4() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::I16);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![PyroValue::List(vec![])]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_I16_EMPTY_4,
        &NESTED_5_OPTION_FIXED_I16_EMPTY_4_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionU32, [T; 2]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_5_OPTION_FIXED_U32_EMPTY_0: [u8; 20] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    0, 0, 0, 0, // [4..8] topology word count (0 levels, sentinels included)
    20, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
];
pub const NESTED_5_OPTION_FIXED_U32_EMPTY_0_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionU32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_u32_empty_0() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::U32);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_U32_EMPTY_0,
        &NESTED_5_OPTION_FIXED_U32_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_5_OPTION_FIXED_U32_EMPTY_1: [u8; 28] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    2, 0, 0, 0, // [4..8] topology word count (1 level, sentinels included)
    28, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    0, 0, 0, 0, // [24..28] level 0: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_U32_EMPTY_1_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionU32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_u32_empty_1() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::U32);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_U32_EMPTY_1,
        &NESTED_5_OPTION_FIXED_U32_EMPTY_1_INODES,
    );
}

/// `[[[]]]`
pub const NESTED_5_OPTION_FIXED_U32_EMPTY_2: [u8; 36] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    4, 0, 0, 0, // [4..8] topology word count (2 levels, sentinels included)
    36, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    0, 0, 0, 0, // [32..36] level 1: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_U32_EMPTY_2_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionU32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_u32_empty_2() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::U32);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_U32_EMPTY_2,
        &NESTED_5_OPTION_FIXED_U32_EMPTY_2_INODES,
    );
}

/// `[[[[]]]]`
pub const NESTED_5_OPTION_FIXED_U32_EMPTY_3: [u8; 44] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    6, 0, 0, 0, // [4..8] topology word count (3 levels, sentinels included)
    44, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    0, 0, 0, 0, // [40..44] level 2: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_U32_EMPTY_3_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionU32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_u32_empty_3() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::U32);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_U32_EMPTY_3,
        &NESTED_5_OPTION_FIXED_U32_EMPTY_3_INODES,
    );
}

/// `[[[[[]]]]]`
pub const NESTED_5_OPTION_FIXED_U32_EMPTY_4: [u8; 52] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    8, 0, 0, 0, // [4..8] topology word count (4 levels, sentinels included)
    52, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    1, 0, 0, 0, // [40..44] level 2: boundary at 1 lists
    255, 255, 255, 255, // [44..48] level 3: sentinel
    0, 0, 0, 0, // [48..52] level 3: boundary at 0 arrays (nulls included)
];
pub const NESTED_5_OPTION_FIXED_U32_EMPTY_4_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionU32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_u32_empty_4() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::U32);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![PyroValue::List(vec![])]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_U32_EMPTY_4,
        &NESTED_5_OPTION_FIXED_U32_EMPTY_4_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionI32, [T; 2]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_5_OPTION_FIXED_I32_EMPTY_0: [u8; 20] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    0, 0, 0, 0, // [4..8] topology word count (0 levels, sentinels included)
    20, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
];
pub const NESTED_5_OPTION_FIXED_I32_EMPTY_0_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionI32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_i32_empty_0() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::I32);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_I32_EMPTY_0,
        &NESTED_5_OPTION_FIXED_I32_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_5_OPTION_FIXED_I32_EMPTY_1: [u8; 28] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    2, 0, 0, 0, // [4..8] topology word count (1 level, sentinels included)
    28, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    0, 0, 0, 0, // [24..28] level 0: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_I32_EMPTY_1_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionI32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_i32_empty_1() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::I32);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_I32_EMPTY_1,
        &NESTED_5_OPTION_FIXED_I32_EMPTY_1_INODES,
    );
}

/// `[[[]]]`
pub const NESTED_5_OPTION_FIXED_I32_EMPTY_2: [u8; 36] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    4, 0, 0, 0, // [4..8] topology word count (2 levels, sentinels included)
    36, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    0, 0, 0, 0, // [32..36] level 1: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_I32_EMPTY_2_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionI32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_i32_empty_2() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::I32);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_I32_EMPTY_2,
        &NESTED_5_OPTION_FIXED_I32_EMPTY_2_INODES,
    );
}

/// `[[[[]]]]`
pub const NESTED_5_OPTION_FIXED_I32_EMPTY_3: [u8; 44] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    6, 0, 0, 0, // [4..8] topology word count (3 levels, sentinels included)
    44, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    0, 0, 0, 0, // [40..44] level 2: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_I32_EMPTY_3_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionI32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_i32_empty_3() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::I32);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_I32_EMPTY_3,
        &NESTED_5_OPTION_FIXED_I32_EMPTY_3_INODES,
    );
}

/// `[[[[[]]]]]`
pub const NESTED_5_OPTION_FIXED_I32_EMPTY_4: [u8; 52] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    8, 0, 0, 0, // [4..8] topology word count (4 levels, sentinels included)
    52, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    1, 0, 0, 0, // [40..44] level 2: boundary at 1 lists
    255, 255, 255, 255, // [44..48] level 3: sentinel
    0, 0, 0, 0, // [48..52] level 3: boundary at 0 arrays (nulls included)
];
pub const NESTED_5_OPTION_FIXED_I32_EMPTY_4_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionI32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_i32_empty_4() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::I32);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![PyroValue::List(vec![])]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_I32_EMPTY_4,
        &NESTED_5_OPTION_FIXED_I32_EMPTY_4_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionU64, [T; 2]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_5_OPTION_FIXED_U64_EMPTY_0: [u8; 24] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    0, 0, 0, 0, // [4..8] topology word count (0 levels, sentinels included)
    24, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    0, 0, 0, 0, // [20..24] alignment padding to 8 bytes before payload
];
pub const NESTED_5_OPTION_FIXED_U64_EMPTY_0_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionU64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_u64_empty_0() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::U64);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_U64_EMPTY_0,
        &NESTED_5_OPTION_FIXED_U64_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_5_OPTION_FIXED_U64_EMPTY_1: [u8; 32] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    2, 0, 0, 0, // [4..8] topology word count (1 level, sentinels included)
    32, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    0, 0, 0, 0, // [24..28] level 0: boundary at 0 lists
    0, 0, 0, 0, // [28..32] alignment padding to 8 bytes before payload
];
pub const NESTED_5_OPTION_FIXED_U64_EMPTY_1_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionU64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_u64_empty_1() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::U64);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_U64_EMPTY_1,
        &NESTED_5_OPTION_FIXED_U64_EMPTY_1_INODES,
    );
}

/// `[[[]]]`
pub const NESTED_5_OPTION_FIXED_U64_EMPTY_2: [u8; 40] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    4, 0, 0, 0, // [4..8] topology word count (2 levels, sentinels included)
    40, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    0, 0, 0, 0, // [32..36] level 1: boundary at 0 lists
    0, 0, 0, 0, // [36..40] alignment padding to 8 bytes before payload
];
pub const NESTED_5_OPTION_FIXED_U64_EMPTY_2_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionU64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_u64_empty_2() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::U64);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_U64_EMPTY_2,
        &NESTED_5_OPTION_FIXED_U64_EMPTY_2_INODES,
    );
}

/// `[[[[]]]]`
pub const NESTED_5_OPTION_FIXED_U64_EMPTY_3: [u8; 48] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    6, 0, 0, 0, // [4..8] topology word count (3 levels, sentinels included)
    48, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    0, 0, 0, 0, // [40..44] level 2: boundary at 0 lists
    0, 0, 0, 0, // [44..48] alignment padding to 8 bytes before payload
];
pub const NESTED_5_OPTION_FIXED_U64_EMPTY_3_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionU64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_u64_empty_3() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::U64);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_U64_EMPTY_3,
        &NESTED_5_OPTION_FIXED_U64_EMPTY_3_INODES,
    );
}

/// `[[[[[]]]]]`
pub const NESTED_5_OPTION_FIXED_U64_EMPTY_4: [u8; 56] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    8, 0, 0, 0, // [4..8] topology word count (4 levels, sentinels included)
    56, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    1, 0, 0, 0, // [40..44] level 2: boundary at 1 lists
    255, 255, 255, 255, // [44..48] level 3: sentinel
    0, 0, 0, 0, // [48..52] level 3: boundary at 0 arrays (nulls included)
    0, 0, 0, 0, // [52..56] alignment padding to 8 bytes before payload
];
pub const NESTED_5_OPTION_FIXED_U64_EMPTY_4_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionU64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_u64_empty_4() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::U64);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![PyroValue::List(vec![])]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_U64_EMPTY_4,
        &NESTED_5_OPTION_FIXED_U64_EMPTY_4_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionI64, [T; 2]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_5_OPTION_FIXED_I64_EMPTY_0: [u8; 24] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    0, 0, 0, 0, // [4..8] topology word count (0 levels, sentinels included)
    24, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    0, 0, 0, 0, // [20..24] alignment padding to 8 bytes before payload
];
pub const NESTED_5_OPTION_FIXED_I64_EMPTY_0_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionI64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_i64_empty_0() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::I64);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_I64_EMPTY_0,
        &NESTED_5_OPTION_FIXED_I64_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_5_OPTION_FIXED_I64_EMPTY_1: [u8; 32] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    2, 0, 0, 0, // [4..8] topology word count (1 level, sentinels included)
    32, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    0, 0, 0, 0, // [24..28] level 0: boundary at 0 lists
    0, 0, 0, 0, // [28..32] alignment padding to 8 bytes before payload
];
pub const NESTED_5_OPTION_FIXED_I64_EMPTY_1_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionI64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_i64_empty_1() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::I64);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_I64_EMPTY_1,
        &NESTED_5_OPTION_FIXED_I64_EMPTY_1_INODES,
    );
}

/// `[[[]]]`
pub const NESTED_5_OPTION_FIXED_I64_EMPTY_2: [u8; 40] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    4, 0, 0, 0, // [4..8] topology word count (2 levels, sentinels included)
    40, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    0, 0, 0, 0, // [32..36] level 1: boundary at 0 lists
    0, 0, 0, 0, // [36..40] alignment padding to 8 bytes before payload
];
pub const NESTED_5_OPTION_FIXED_I64_EMPTY_2_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionI64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_i64_empty_2() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::I64);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_I64_EMPTY_2,
        &NESTED_5_OPTION_FIXED_I64_EMPTY_2_INODES,
    );
}

/// `[[[[]]]]`
pub const NESTED_5_OPTION_FIXED_I64_EMPTY_3: [u8; 48] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    6, 0, 0, 0, // [4..8] topology word count (3 levels, sentinels included)
    48, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    0, 0, 0, 0, // [40..44] level 2: boundary at 0 lists
    0, 0, 0, 0, // [44..48] alignment padding to 8 bytes before payload
];
pub const NESTED_5_OPTION_FIXED_I64_EMPTY_3_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionI64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_i64_empty_3() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::I64);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_I64_EMPTY_3,
        &NESTED_5_OPTION_FIXED_I64_EMPTY_3_INODES,
    );
}

/// `[[[[[]]]]]`
pub const NESTED_5_OPTION_FIXED_I64_EMPTY_4: [u8; 56] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    8, 0, 0, 0, // [4..8] topology word count (4 levels, sentinels included)
    56, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    1, 0, 0, 0, // [40..44] level 2: boundary at 1 lists
    255, 255, 255, 255, // [44..48] level 3: sentinel
    0, 0, 0, 0, // [48..52] level 3: boundary at 0 arrays (nulls included)
    0, 0, 0, 0, // [52..56] alignment padding to 8 bytes before payload
];
pub const NESTED_5_OPTION_FIXED_I64_EMPTY_4_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionI64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_i64_empty_4() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::I64);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![PyroValue::List(vec![])]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_I64_EMPTY_4,
        &NESTED_5_OPTION_FIXED_I64_EMPTY_4_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionF16, [T; 2]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_5_OPTION_FIXED_F16_EMPTY_0: [u8; 20] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    0, 0, 0, 0, // [4..8] topology word count (0 levels, sentinels included)
    20, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
];
pub const NESTED_5_OPTION_FIXED_F16_EMPTY_0_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionF16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_f16_empty_0() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::F16);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_F16_EMPTY_0,
        &NESTED_5_OPTION_FIXED_F16_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_5_OPTION_FIXED_F16_EMPTY_1: [u8; 28] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    2, 0, 0, 0, // [4..8] topology word count (1 level, sentinels included)
    28, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    0, 0, 0, 0, // [24..28] level 0: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_F16_EMPTY_1_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionF16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_f16_empty_1() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::F16);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_F16_EMPTY_1,
        &NESTED_5_OPTION_FIXED_F16_EMPTY_1_INODES,
    );
}

/// `[[[]]]`
pub const NESTED_5_OPTION_FIXED_F16_EMPTY_2: [u8; 36] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    4, 0, 0, 0, // [4..8] topology word count (2 levels, sentinels included)
    36, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    0, 0, 0, 0, // [32..36] level 1: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_F16_EMPTY_2_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionF16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_f16_empty_2() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::F16);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_F16_EMPTY_2,
        &NESTED_5_OPTION_FIXED_F16_EMPTY_2_INODES,
    );
}

/// `[[[[]]]]`
pub const NESTED_5_OPTION_FIXED_F16_EMPTY_3: [u8; 44] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    6, 0, 0, 0, // [4..8] topology word count (3 levels, sentinels included)
    44, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    0, 0, 0, 0, // [40..44] level 2: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_F16_EMPTY_3_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionF16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_f16_empty_3() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::F16);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_F16_EMPTY_3,
        &NESTED_5_OPTION_FIXED_F16_EMPTY_3_INODES,
    );
}

/// `[[[[[]]]]]`
pub const NESTED_5_OPTION_FIXED_F16_EMPTY_4: [u8; 52] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    8, 0, 0, 0, // [4..8] topology word count (4 levels, sentinels included)
    52, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    1, 0, 0, 0, // [40..44] level 2: boundary at 1 lists
    255, 255, 255, 255, // [44..48] level 3: sentinel
    0, 0, 0, 0, // [48..52] level 3: boundary at 0 arrays (nulls included)
];
pub const NESTED_5_OPTION_FIXED_F16_EMPTY_4_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionF16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_f16_empty_4() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::F16);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![PyroValue::List(vec![])]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_F16_EMPTY_4,
        &NESTED_5_OPTION_FIXED_F16_EMPTY_4_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionF32, [T; 2]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_5_OPTION_FIXED_F32_EMPTY_0: [u8; 20] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    0, 0, 0, 0, // [4..8] topology word count (0 levels, sentinels included)
    20, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
];
pub const NESTED_5_OPTION_FIXED_F32_EMPTY_0_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionF32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_f32_empty_0() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::F32);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_F32_EMPTY_0,
        &NESTED_5_OPTION_FIXED_F32_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_5_OPTION_FIXED_F32_EMPTY_1: [u8; 28] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    2, 0, 0, 0, // [4..8] topology word count (1 level, sentinels included)
    28, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    0, 0, 0, 0, // [24..28] level 0: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_F32_EMPTY_1_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionF32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_f32_empty_1() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::F32);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_F32_EMPTY_1,
        &NESTED_5_OPTION_FIXED_F32_EMPTY_1_INODES,
    );
}

/// `[[[]]]`
pub const NESTED_5_OPTION_FIXED_F32_EMPTY_2: [u8; 36] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    4, 0, 0, 0, // [4..8] topology word count (2 levels, sentinels included)
    36, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    0, 0, 0, 0, // [32..36] level 1: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_F32_EMPTY_2_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionF32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_f32_empty_2() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::F32);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_F32_EMPTY_2,
        &NESTED_5_OPTION_FIXED_F32_EMPTY_2_INODES,
    );
}

/// `[[[[]]]]`
pub const NESTED_5_OPTION_FIXED_F32_EMPTY_3: [u8; 44] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    6, 0, 0, 0, // [4..8] topology word count (3 levels, sentinels included)
    44, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    0, 0, 0, 0, // [40..44] level 2: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_F32_EMPTY_3_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionF32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_f32_empty_3() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::F32);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_F32_EMPTY_3,
        &NESTED_5_OPTION_FIXED_F32_EMPTY_3_INODES,
    );
}

/// `[[[[[]]]]]`
pub const NESTED_5_OPTION_FIXED_F32_EMPTY_4: [u8; 52] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    8, 0, 0, 0, // [4..8] topology word count (4 levels, sentinels included)
    52, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    1, 0, 0, 0, // [40..44] level 2: boundary at 1 lists
    255, 255, 255, 255, // [44..48] level 3: sentinel
    0, 0, 0, 0, // [48..52] level 3: boundary at 0 arrays (nulls included)
];
pub const NESTED_5_OPTION_FIXED_F32_EMPTY_4_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionF32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_f32_empty_4() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::F32);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![PyroValue::List(vec![])]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_F32_EMPTY_4,
        &NESTED_5_OPTION_FIXED_F32_EMPTY_4_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionF64, [T; 2]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_5_OPTION_FIXED_F64_EMPTY_0: [u8; 24] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    0, 0, 0, 0, // [4..8] topology word count (0 levels, sentinels included)
    24, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    0, 0, 0, 0, // [20..24] alignment padding to 8 bytes before payload
];
pub const NESTED_5_OPTION_FIXED_F64_EMPTY_0_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionF64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_f64_empty_0() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::F64);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_F64_EMPTY_0,
        &NESTED_5_OPTION_FIXED_F64_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_5_OPTION_FIXED_F64_EMPTY_1: [u8; 32] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    2, 0, 0, 0, // [4..8] topology word count (1 level, sentinels included)
    32, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    0, 0, 0, 0, // [24..28] level 0: boundary at 0 lists
    0, 0, 0, 0, // [28..32] alignment padding to 8 bytes before payload
];
pub const NESTED_5_OPTION_FIXED_F64_EMPTY_1_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionF64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_f64_empty_1() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::F64);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_F64_EMPTY_1,
        &NESTED_5_OPTION_FIXED_F64_EMPTY_1_INODES,
    );
}

/// `[[[]]]`
pub const NESTED_5_OPTION_FIXED_F64_EMPTY_2: [u8; 40] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    4, 0, 0, 0, // [4..8] topology word count (2 levels, sentinels included)
    40, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    0, 0, 0, 0, // [32..36] level 1: boundary at 0 lists
    0, 0, 0, 0, // [36..40] alignment padding to 8 bytes before payload
];
pub const NESTED_5_OPTION_FIXED_F64_EMPTY_2_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionF64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_f64_empty_2() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::F64);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_F64_EMPTY_2,
        &NESTED_5_OPTION_FIXED_F64_EMPTY_2_INODES,
    );
}

/// `[[[[]]]]`
pub const NESTED_5_OPTION_FIXED_F64_EMPTY_3: [u8; 48] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    6, 0, 0, 0, // [4..8] topology word count (3 levels, sentinels included)
    48, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    0, 0, 0, 0, // [40..44] level 2: boundary at 0 lists
    0, 0, 0, 0, // [44..48] alignment padding to 8 bytes before payload
];
pub const NESTED_5_OPTION_FIXED_F64_EMPTY_3_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionF64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_f64_empty_3() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::F64);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_F64_EMPTY_3,
        &NESTED_5_OPTION_FIXED_F64_EMPTY_3_INODES,
    );
}

/// `[[[[[]]]]]`
pub const NESTED_5_OPTION_FIXED_F64_EMPTY_4: [u8; 56] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    8, 0, 0, 0, // [4..8] topology word count (4 levels, sentinels included)
    56, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    1, 0, 0, 0, // [40..44] level 2: boundary at 1 lists
    255, 255, 255, 255, // [44..48] level 3: sentinel
    0, 0, 0, 0, // [48..52] level 3: boundary at 0 arrays (nulls included)
    0, 0, 0, 0, // [52..56] alignment padding to 8 bytes before payload
];
pub const NESTED_5_OPTION_FIXED_F64_EMPTY_4_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionF64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_f64_empty_4() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::F64);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![PyroValue::List(vec![])]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_F64_EMPTY_4,
        &NESTED_5_OPTION_FIXED_F64_EMPTY_4_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionTimestamp, [T; 2]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_5_OPTION_FIXED_TIMESTAMP_EMPTY_0: [u8; 32] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    0, 0, 0, 0, // [4..8] topology word count (0 levels, sentinels included)
    32, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, // [20..32] alignment padding to 16 bytes before payload
];
pub const NESTED_5_OPTION_FIXED_TIMESTAMP_EMPTY_0_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionTimestamp);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_timestamp_empty_0() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::Timestamp);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_TIMESTAMP_EMPTY_0,
        &NESTED_5_OPTION_FIXED_TIMESTAMP_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_5_OPTION_FIXED_TIMESTAMP_EMPTY_1: [u8; 32] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    2, 0, 0, 0, // [4..8] topology word count (1 level, sentinels included)
    32, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    0, 0, 0, 0, // [24..28] level 0: boundary at 0 lists
    0, 0, 0, 0, // [28..32] alignment padding to 16 bytes before payload
];
pub const NESTED_5_OPTION_FIXED_TIMESTAMP_EMPTY_1_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionTimestamp);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_timestamp_empty_1() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::Timestamp);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_TIMESTAMP_EMPTY_1,
        &NESTED_5_OPTION_FIXED_TIMESTAMP_EMPTY_1_INODES,
    );
}

/// `[[[]]]`
pub const NESTED_5_OPTION_FIXED_TIMESTAMP_EMPTY_2: [u8; 48] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    4, 0, 0, 0, // [4..8] topology word count (2 levels, sentinels included)
    48, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    0, 0, 0, 0, // [32..36] level 1: boundary at 0 lists
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, // [36..48] alignment padding to 16 bytes before payload
];
pub const NESTED_5_OPTION_FIXED_TIMESTAMP_EMPTY_2_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionTimestamp);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_timestamp_empty_2() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::Timestamp);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_TIMESTAMP_EMPTY_2,
        &NESTED_5_OPTION_FIXED_TIMESTAMP_EMPTY_2_INODES,
    );
}

/// `[[[[]]]]`
pub const NESTED_5_OPTION_FIXED_TIMESTAMP_EMPTY_3: [u8; 48] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    6, 0, 0, 0, // [4..8] topology word count (3 levels, sentinels included)
    48, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    0, 0, 0, 0, // [40..44] level 2: boundary at 0 lists
    0, 0, 0, 0, // [44..48] alignment padding to 16 bytes before payload
];
pub const NESTED_5_OPTION_FIXED_TIMESTAMP_EMPTY_3_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionTimestamp);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_timestamp_empty_3() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::Timestamp);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_TIMESTAMP_EMPTY_3,
        &NESTED_5_OPTION_FIXED_TIMESTAMP_EMPTY_3_INODES,
    );
}

/// `[[[[[]]]]]`
pub const NESTED_5_OPTION_FIXED_TIMESTAMP_EMPTY_4: [u8; 64] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    8, 0, 0, 0, // [4..8] topology word count (4 levels, sentinels included)
    64, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    1, 0, 0, 0, // [40..44] level 2: boundary at 1 lists
    255, 255, 255, 255, // [44..48] level 3: sentinel
    0, 0, 0, 0, // [48..52] level 3: boundary at 0 arrays (nulls included)
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, // [52..64] alignment padding to 16 bytes before payload
];
pub const NESTED_5_OPTION_FIXED_TIMESTAMP_EMPTY_4_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionTimestamp);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_timestamp_empty_4() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::Timestamp);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![PyroValue::List(vec![])]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_TIMESTAMP_EMPTY_4,
        &NESTED_5_OPTION_FIXED_TIMESTAMP_EMPTY_4_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionBool, [T; 2]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_5_OPTION_FIXED_BOOL_EMPTY_0: [u8; 20] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    0, 0, 0, 0, // [4..8] topology word count (0 levels, sentinels included)
    20, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
];
pub const NESTED_5_OPTION_FIXED_BOOL_EMPTY_0_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionBool);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_bool_empty_0() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::Bool);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_BOOL_EMPTY_0,
        &NESTED_5_OPTION_FIXED_BOOL_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_5_OPTION_FIXED_BOOL_EMPTY_1: [u8; 28] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    2, 0, 0, 0, // [4..8] topology word count (1 level, sentinels included)
    28, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    0, 0, 0, 0, // [24..28] level 0: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_BOOL_EMPTY_1_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionBool);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_bool_empty_1() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::Bool);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_BOOL_EMPTY_1,
        &NESTED_5_OPTION_FIXED_BOOL_EMPTY_1_INODES,
    );
}

/// `[[[]]]`
pub const NESTED_5_OPTION_FIXED_BOOL_EMPTY_2: [u8; 36] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    4, 0, 0, 0, // [4..8] topology word count (2 levels, sentinels included)
    36, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    0, 0, 0, 0, // [32..36] level 1: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_BOOL_EMPTY_2_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionBool);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_bool_empty_2() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::Bool);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_BOOL_EMPTY_2,
        &NESTED_5_OPTION_FIXED_BOOL_EMPTY_2_INODES,
    );
}

/// `[[[[]]]]`
pub const NESTED_5_OPTION_FIXED_BOOL_EMPTY_3: [u8; 44] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    6, 0, 0, 0, // [4..8] topology word count (3 levels, sentinels included)
    44, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    0, 0, 0, 0, // [40..44] level 2: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_BOOL_EMPTY_3_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionBool);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_bool_empty_3() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::Bool);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_BOOL_EMPTY_3,
        &NESTED_5_OPTION_FIXED_BOOL_EMPTY_3_INODES,
    );
}

/// `[[[[[]]]]]`
pub const NESTED_5_OPTION_FIXED_BOOL_EMPTY_4: [u8; 52] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    8, 0, 0, 0, // [4..8] topology word count (4 levels, sentinels included)
    52, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    1, 0, 0, 0, // [40..44] level 2: boundary at 1 lists
    255, 255, 255, 255, // [44..48] level 3: sentinel
    0, 0, 0, 0, // [48..52] level 3: boundary at 0 arrays (nulls included)
];
pub const NESTED_5_OPTION_FIXED_BOOL_EMPTY_4_INODES: [Inode; 1] =
    nested_5_option_fixed_inode(WireKind::OptionBool);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_bool_empty_4() {
    let schema = nested_5_option_fixed_schema(PrimitiveDataType::Bool);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![PyroValue::List(vec![])]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_BOOL_EMPTY_4,
        &NESTED_5_OPTION_FIXED_BOOL_EMPTY_4_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionBool, [T; 8]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_5_OPTION_FIXED_BOOL_8_EMPTY_0: [u8; 20] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    0, 0, 0, 0, // [4..8] topology word count (0 levels, sentinels included)
    20, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
];
pub const NESTED_5_OPTION_FIXED_BOOL_8_EMPTY_0_INODES: [Inode; 1] =
    nested_5_option_fixed_sized_inode(WireKind::OptionBool, 8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_bool_8_empty_0() {
    let schema = nested_5_option_fixed_sized_schema(PrimitiveDataType::Bool, 8);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_BOOL_8_EMPTY_0,
        &NESTED_5_OPTION_FIXED_BOOL_8_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_5_OPTION_FIXED_BOOL_8_EMPTY_1: [u8; 28] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    2, 0, 0, 0, // [4..8] topology word count (1 level, sentinels included)
    28, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    0, 0, 0, 0, // [24..28] level 0: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_BOOL_8_EMPTY_1_INODES: [Inode; 1] =
    nested_5_option_fixed_sized_inode(WireKind::OptionBool, 8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_bool_8_empty_1() {
    let schema = nested_5_option_fixed_sized_schema(PrimitiveDataType::Bool, 8);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_BOOL_8_EMPTY_1,
        &NESTED_5_OPTION_FIXED_BOOL_8_EMPTY_1_INODES,
    );
}

/// `[[[]]]`
pub const NESTED_5_OPTION_FIXED_BOOL_8_EMPTY_2: [u8; 36] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    4, 0, 0, 0, // [4..8] topology word count (2 levels, sentinels included)
    36, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    0, 0, 0, 0, // [32..36] level 1: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_BOOL_8_EMPTY_2_INODES: [Inode; 1] =
    nested_5_option_fixed_sized_inode(WireKind::OptionBool, 8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_bool_8_empty_2() {
    let schema = nested_5_option_fixed_sized_schema(PrimitiveDataType::Bool, 8);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_BOOL_8_EMPTY_2,
        &NESTED_5_OPTION_FIXED_BOOL_8_EMPTY_2_INODES,
    );
}

/// `[[[[]]]]`
pub const NESTED_5_OPTION_FIXED_BOOL_8_EMPTY_3: [u8; 44] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    6, 0, 0, 0, // [4..8] topology word count (3 levels, sentinels included)
    44, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    0, 0, 0, 0, // [40..44] level 2: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_BOOL_8_EMPTY_3_INODES: [Inode; 1] =
    nested_5_option_fixed_sized_inode(WireKind::OptionBool, 8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_bool_8_empty_3() {
    let schema = nested_5_option_fixed_sized_schema(PrimitiveDataType::Bool, 8);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_BOOL_8_EMPTY_3,
        &NESTED_5_OPTION_FIXED_BOOL_8_EMPTY_3_INODES,
    );
}

/// `[[[[[]]]]]`
pub const NESTED_5_OPTION_FIXED_BOOL_8_EMPTY_4: [u8; 52] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    8, 0, 0, 0, // [4..8] topology word count (4 levels, sentinels included)
    52, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    1, 0, 0, 0, // [40..44] level 2: boundary at 1 lists
    255, 255, 255, 255, // [44..48] level 3: sentinel
    0, 0, 0, 0, // [48..52] level 3: boundary at 0 arrays (nulls included)
];
pub const NESTED_5_OPTION_FIXED_BOOL_8_EMPTY_4_INODES: [Inode; 1] =
    nested_5_option_fixed_sized_inode(WireKind::OptionBool, 8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_bool_8_empty_4() {
    let schema = nested_5_option_fixed_sized_schema(PrimitiveDataType::Bool, 8);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![PyroValue::List(vec![])]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_BOOL_8_EMPTY_4,
        &NESTED_5_OPTION_FIXED_BOOL_8_EMPTY_4_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionBool, [T; 16]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_5_OPTION_FIXED_BOOL_16_EMPTY_0: [u8; 20] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    0, 0, 0, 0, // [4..8] topology word count (0 levels, sentinels included)
    20, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
];
pub const NESTED_5_OPTION_FIXED_BOOL_16_EMPTY_0_INODES: [Inode; 1] =
    nested_5_option_fixed_sized_inode(WireKind::OptionBool, 16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_bool_16_empty_0() {
    let schema = nested_5_option_fixed_sized_schema(PrimitiveDataType::Bool, 16);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_BOOL_16_EMPTY_0,
        &NESTED_5_OPTION_FIXED_BOOL_16_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_5_OPTION_FIXED_BOOL_16_EMPTY_1: [u8; 28] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    2, 0, 0, 0, // [4..8] topology word count (1 level, sentinels included)
    28, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    0, 0, 0, 0, // [24..28] level 0: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_BOOL_16_EMPTY_1_INODES: [Inode; 1] =
    nested_5_option_fixed_sized_inode(WireKind::OptionBool, 16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_bool_16_empty_1() {
    let schema = nested_5_option_fixed_sized_schema(PrimitiveDataType::Bool, 16);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_BOOL_16_EMPTY_1,
        &NESTED_5_OPTION_FIXED_BOOL_16_EMPTY_1_INODES,
    );
}

/// `[[[]]]`
pub const NESTED_5_OPTION_FIXED_BOOL_16_EMPTY_2: [u8; 36] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    4, 0, 0, 0, // [4..8] topology word count (2 levels, sentinels included)
    36, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    0, 0, 0, 0, // [32..36] level 1: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_BOOL_16_EMPTY_2_INODES: [Inode; 1] =
    nested_5_option_fixed_sized_inode(WireKind::OptionBool, 16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_bool_16_empty_2() {
    let schema = nested_5_option_fixed_sized_schema(PrimitiveDataType::Bool, 16);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_BOOL_16_EMPTY_2,
        &NESTED_5_OPTION_FIXED_BOOL_16_EMPTY_2_INODES,
    );
}

/// `[[[[]]]]`
pub const NESTED_5_OPTION_FIXED_BOOL_16_EMPTY_3: [u8; 44] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    6, 0, 0, 0, // [4..8] topology word count (3 levels, sentinels included)
    44, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    0, 0, 0, 0, // [40..44] level 2: boundary at 0 lists
];
pub const NESTED_5_OPTION_FIXED_BOOL_16_EMPTY_3_INODES: [Inode; 1] =
    nested_5_option_fixed_sized_inode(WireKind::OptionBool, 16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_bool_16_empty_3() {
    let schema = nested_5_option_fixed_sized_schema(PrimitiveDataType::Bool, 16);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_BOOL_16_EMPTY_3,
        &NESTED_5_OPTION_FIXED_BOOL_16_EMPTY_3_INODES,
    );
}

/// `[[[[[]]]]]`
pub const NESTED_5_OPTION_FIXED_BOOL_16_EMPTY_4: [u8; 52] = [
    20, 0, 0, 0, // [0..4] topology start pointer
    8, 0, 0, 0, // [4..8] topology word count (4 levels, sentinels included)
    52, 0, 0, 0, // [8..12] payload start pointer
    0, 0, 0, 0, // [12..16] valid arrays
    0, 0, 0, 0, // [16..20] total arrays (nulls included)
    255, 255, 255, 255, // [20..24] level 0: sentinel
    1, 0, 0, 0, // [24..28] level 0: boundary at 1 lists
    255, 255, 255, 255, // [28..32] level 1: sentinel
    1, 0, 0, 0, // [32..36] level 1: boundary at 1 lists
    255, 255, 255, 255, // [36..40] level 2: sentinel
    1, 0, 0, 0, // [40..44] level 2: boundary at 1 lists
    255, 255, 255, 255, // [44..48] level 3: sentinel
    0, 0, 0, 0, // [48..52] level 3: boundary at 0 arrays (nulls included)
];
pub const NESTED_5_OPTION_FIXED_BOOL_16_EMPTY_4_INODES: [Inode; 1] =
    nested_5_option_fixed_sized_inode(WireKind::OptionBool, 16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_5_option_fixed_bool_16_empty_4() {
    let schema = nested_5_option_fixed_sized_schema(PrimitiveDataType::Bool, 16);
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::List(vec![PyroValue::List(vec![])]),
    ])])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_5_OPTION_FIXED_BOOL_16_EMPTY_4,
        &NESTED_5_OPTION_FIXED_BOOL_16_EMPTY_4_INODES,
    );
}
