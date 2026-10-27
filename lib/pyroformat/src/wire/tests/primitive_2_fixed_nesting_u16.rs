//! `List<List<[T; 2]>>` for every `PrimitiveDataType`. The same list shapes as
//! `primitive_2_nesting.rs`, except every leaf sample is a `[T; 2]`
//! array, so the deepest topology level and the trailing header word
//! count **arrays**, not raw elements. `Bool` arrays are bit-packed
//! back to back into one LSB-first run.
//!
//! Shared value shape: `[[a, b], [c], [d, e, f]]` (each letter a `[T; 2]`).
//!
//! Layout: `[topology start, topology word count, payload start, array count]`,
//! then 1 topology level of sentinel-prefixed cumulative boundaries
//! (level 0: `MAX, 2, 3, 6`)
//! then (after alignment padding) the payload.

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
    wire::tests::support::run_list_wrapped_roundtrip_u16,
};

use pyro_schema::{Inode, KindMeta, WireKind};

#[cfg(test)]
fn nested_2_fixed_sized_schema(primitive: PrimitiveDataType, size: usize) -> PyroSchema {
    let mut ty = PyroType::PrimitiveFixedList {
        element: primitive,
        size,
    };
    for _ in 0..2 {
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
fn nested_2_fixed_schema(primitive: PrimitiveDataType) -> PyroSchema {
    nested_2_fixed_sized_schema(primitive, 2)
}

/// `[[a, b], [c], [d, e, f]]` from 3 leaf runs of `[T; 2]` arrays.
#[cfg(test)]
fn nested_2_fixed_value(leaves: [PyroValue<'static>; 3]) -> PyroValue<'static> {
    let [l0, l1, l2] = leaves;
    PyroValue::List(vec![l0, l1, l2])
}

/// Same as [`nested_2_fixed_inode`], for `[T; fixed_size]` leaves.
const fn nested_2_fixed_sized_inode(kind: WireKind, fixed_size: u16) -> [Inode; 1] {
    [Inode {
        kind,
        kind_meta: KindMeta::Primitive {
            nesting: 2,
            data_offset: 4,
            topology_offset: 0,
            fixed_size,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    }]
}

/// `List<List<[T; 2]>>` at the root: one `Primitive` inode with `nesting: 2`, `fixed_size: 2`,
/// its topology slot at 0 and its data slot at 4.
const fn nested_2_fixed_inode(kind: WireKind) -> [Inode; 1] {
    nested_2_fixed_sized_inode(kind, 2)
}

// ---------------------------------------------------------------------------
// U8
// ---------------------------------------------------------------------------

pub const NESTED_2_FIXED_U8: [u8; 28] = [
    8, 0, // topology start pointer
    4, 0, // topology word count (1 level, sentinels included)
    16, 0, // payload start pointer
    6, 0, // array count
    255, 255, // level 0: sentinel
    2, 0, // level 0: boundary at 2 arrays
    3, 0, // level 0: boundary at 3 arrays
    6, 0,  // level 0: boundary at 6 arrays
    1,  // payload[0] = 1
    2,  // payload[1] = 2
    3,  // payload[2] = 3
    4,  // payload[3] = 4
    5,  // payload[4] = 5
    6,  // payload[5] = 6
    7,  // payload[6] = 7
    8,  // payload[7] = 8
    9,  // payload[8] = 9
    10, // payload[9] = 10
    11, // payload[10] = 11
    12, // payload[11] = 12
];
pub const NESTED_2_FIXED_U8_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::U8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_u8() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::U8);
    let data = nested_2_fixed_value([
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![1, 2].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![3, 4].into())),
        ]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::U8(
            vec![5, 6].into(),
        ))]),
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![7, 8].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![9, 10].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![11, 12].into())),
        ]),
    ]);

    run_list_wrapped_roundtrip_u16(schema, data, &NESTED_2_FIXED_U8, &NESTED_2_FIXED_U8_INODES);
}

// ---------------------------------------------------------------------------
// I8
// ---------------------------------------------------------------------------

pub const NESTED_2_FIXED_I8: [u8; 28] = [
    8, 0, // topology start pointer
    4, 0, // topology word count (1 level, sentinels included)
    16, 0, // payload start pointer
    6, 0, // array count
    255, 255, // level 0: sentinel
    2, 0, // level 0: boundary at 2 arrays
    3, 0, // level 0: boundary at 3 arrays
    6, 0,   // level 0: boundary at 6 arrays
    255, // payload[0] = -1
    2,   // payload[1] = 2
    253, // payload[2] = -3
    4,   // payload[3] = 4
    251, // payload[4] = -5
    6,   // payload[5] = 6
    249, // payload[6] = -7
    8,   // payload[7] = 8
    247, // payload[8] = -9
    10,  // payload[9] = 10
    245, // payload[10] = -11
    12,  // payload[11] = 12
];
pub const NESTED_2_FIXED_I8_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::I8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_i8() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::I8);
    let data = nested_2_fixed_value([
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::I8(vec![-1, 2].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::I8(vec![-3, 4].into())),
        ]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::I8(
            vec![-5, 6].into(),
        ))]),
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::I8(vec![-7, 8].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::I8(vec![-9, 10].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::I8(vec![-11, 12].into())),
        ]),
    ]);

    run_list_wrapped_roundtrip_u16(schema, data, &NESTED_2_FIXED_I8, &NESTED_2_FIXED_I8_INODES);
}

// ---------------------------------------------------------------------------
// U16
// ---------------------------------------------------------------------------

pub const NESTED_2_FIXED_U16: [u8; 40] = [
    8, 0, // topology start pointer
    4, 0, // topology word count (1 level, sentinels included)
    16, 0, // payload start pointer
    6, 0, // array count
    255, 255, // level 0: sentinel
    2, 0, // level 0: boundary at 2 arrays
    3, 0, // level 0: boundary at 3 arrays
    6, 0, // level 0: boundary at 6 arrays
    1, 0, // payload[0] = 1
    2, 0, // payload[1] = 2
    3, 0, // payload[2] = 3
    4, 0, // payload[3] = 4
    5, 0, // payload[4] = 5
    6, 0, // payload[5] = 6
    7, 0, // payload[6] = 7
    8, 0, // payload[7] = 8
    9, 0, // payload[8] = 9
    10, 0, // payload[9] = 10
    11, 0, // payload[10] = 11
    12, 0, // payload[11] = 12
];
pub const NESTED_2_FIXED_U16_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::U16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_u16() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::U16);
    let data = nested_2_fixed_value([
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::U16(vec![1, 2].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::U16(vec![3, 4].into())),
        ]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::U16(
            vec![5, 6].into(),
        ))]),
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::U16(vec![7, 8].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::U16(vec![9, 10].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::U16(vec![11, 12].into())),
        ]),
    ]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_U16,
        &NESTED_2_FIXED_U16_INODES,
    );
}

// ---------------------------------------------------------------------------
// I16
// ---------------------------------------------------------------------------

pub const NESTED_2_FIXED_I16: [u8; 40] = [
    8, 0, // topology start pointer
    4, 0, // topology word count (1 level, sentinels included)
    16, 0, // payload start pointer
    6, 0, // array count
    255, 255, // level 0: sentinel
    2, 0, // level 0: boundary at 2 arrays
    3, 0, // level 0: boundary at 3 arrays
    6, 0, // level 0: boundary at 6 arrays
    255, 255, // payload[0] = -1
    2, 0, // payload[1] = 2
    253, 255, // payload[2] = -3
    4, 0, // payload[3] = 4
    251, 255, // payload[4] = -5
    6, 0, // payload[5] = 6
    249, 255, // payload[6] = -7
    8, 0, // payload[7] = 8
    247, 255, // payload[8] = -9
    10, 0, // payload[9] = 10
    245, 255, // payload[10] = -11
    12, 0, // payload[11] = 12
];
pub const NESTED_2_FIXED_I16_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::I16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_i16() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::I16);
    let data = nested_2_fixed_value([
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::I16(vec![-1, 2].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::I16(vec![-3, 4].into())),
        ]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::I16(
            vec![-5, 6].into(),
        ))]),
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::I16(vec![-7, 8].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::I16(vec![-9, 10].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::I16(vec![-11, 12].into())),
        ]),
    ]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_I16,
        &NESTED_2_FIXED_I16_INODES,
    );
}

// ---------------------------------------------------------------------------
// U32
// ---------------------------------------------------------------------------

pub const NESTED_2_FIXED_U32: [u8; 64] = [
    8, 0, // topology start pointer
    4, 0, // topology word count (1 level, sentinels included)
    16, 0, // payload start pointer
    6, 0, // array count
    255, 255, // level 0: sentinel
    2, 0, // level 0: boundary at 2 arrays
    3, 0, // level 0: boundary at 3 arrays
    6, 0, // level 0: boundary at 6 arrays
    1, 0, 0, 0, // payload[0] = 1
    2, 0, 0, 0, // payload[1] = 2
    3, 0, 0, 0, // payload[2] = 3
    4, 0, 0, 0, // payload[3] = 4
    5, 0, 0, 0, // payload[4] = 5
    6, 0, 0, 0, // payload[5] = 6
    7, 0, 0, 0, // payload[6] = 7
    8, 0, 0, 0, // payload[7] = 8
    9, 0, 0, 0, // payload[8] = 9
    10, 0, 0, 0, // payload[9] = 10
    11, 0, 0, 0, // payload[10] = 11
    12, 0, 0, 0, // payload[11] = 12
];
pub const NESTED_2_FIXED_U32_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::U32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_u32() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::U32);
    let data = nested_2_fixed_value([
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::U32(vec![1, 2].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::U32(vec![3, 4].into())),
        ]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::U32(
            vec![5, 6].into(),
        ))]),
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::U32(vec![7, 8].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::U32(vec![9, 10].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::U32(vec![11, 12].into())),
        ]),
    ]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_U32,
        &NESTED_2_FIXED_U32_INODES,
    );
}

// ---------------------------------------------------------------------------
// I32
// ---------------------------------------------------------------------------

pub const NESTED_2_FIXED_I32: [u8; 64] = [
    8, 0, // topology start pointer
    4, 0, // topology word count (1 level, sentinels included)
    16, 0, // payload start pointer
    6, 0, // array count
    255, 255, // level 0: sentinel
    2, 0, // level 0: boundary at 2 arrays
    3, 0, // level 0: boundary at 3 arrays
    6, 0, // level 0: boundary at 6 arrays
    255, 255, 255, 255, // payload[0] = -1
    2, 0, 0, 0, // payload[1] = 2
    253, 255, 255, 255, // payload[2] = -3
    4, 0, 0, 0, // payload[3] = 4
    251, 255, 255, 255, // payload[4] = -5
    6, 0, 0, 0, // payload[5] = 6
    249, 255, 255, 255, // payload[6] = -7
    8, 0, 0, 0, // payload[7] = 8
    247, 255, 255, 255, // payload[8] = -9
    10, 0, 0, 0, // payload[9] = 10
    245, 255, 255, 255, // payload[10] = -11
    12, 0, 0, 0, // payload[11] = 12
];
pub const NESTED_2_FIXED_I32_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::I32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_i32() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::I32);
    let data = nested_2_fixed_value([
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::I32(vec![-1, 2].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::I32(vec![-3, 4].into())),
        ]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::I32(
            vec![-5, 6].into(),
        ))]),
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::I32(vec![-7, 8].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::I32(vec![-9, 10].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::I32(vec![-11, 12].into())),
        ]),
    ]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_I32,
        &NESTED_2_FIXED_I32_INODES,
    );
}

// ---------------------------------------------------------------------------
// U64
// ---------------------------------------------------------------------------

pub const NESTED_2_FIXED_U64: [u8; 112] = [
    8, 0, // topology start pointer
    4, 0, // topology word count (1 level, sentinels included)
    16, 0, // payload start pointer
    6, 0, // array count
    255, 255, // level 0: sentinel
    2, 0, // level 0: boundary at 2 arrays
    3, 0, // level 0: boundary at 3 arrays
    6, 0, // level 0: boundary at 6 arrays
    1, 0, 0, 0, 0, 0, 0, 0, // payload[0] = 1
    2, 0, 0, 0, 0, 0, 0, 0, // payload[1] = 2
    3, 0, 0, 0, 0, 0, 0, 0, // payload[2] = 3
    4, 0, 0, 0, 0, 0, 0, 0, // payload[3] = 4
    5, 0, 0, 0, 0, 0, 0, 0, // payload[4] = 5
    6, 0, 0, 0, 0, 0, 0, 0, // payload[5] = 6
    7, 0, 0, 0, 0, 0, 0, 0, // payload[6] = 7
    8, 0, 0, 0, 0, 0, 0, 0, // payload[7] = 8
    9, 0, 0, 0, 0, 0, 0, 0, // payload[8] = 9
    10, 0, 0, 0, 0, 0, 0, 0, // payload[9] = 10
    11, 0, 0, 0, 0, 0, 0, 0, // payload[10] = 11
    12, 0, 0, 0, 0, 0, 0, 0, // payload[11] = 12
];
pub const NESTED_2_FIXED_U64_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::U64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_u64() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::U64);
    let data = nested_2_fixed_value([
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::U64(vec![1, 2].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::U64(vec![3, 4].into())),
        ]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::U64(
            vec![5, 6].into(),
        ))]),
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::U64(vec![7, 8].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::U64(vec![9, 10].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::U64(vec![11, 12].into())),
        ]),
    ]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_U64,
        &NESTED_2_FIXED_U64_INODES,
    );
}

// ---------------------------------------------------------------------------
// I64
// ---------------------------------------------------------------------------

pub const NESTED_2_FIXED_I64: [u8; 112] = [
    8, 0, // topology start pointer
    4, 0, // topology word count (1 level, sentinels included)
    16, 0, // payload start pointer
    6, 0, // array count
    255, 255, // level 0: sentinel
    2, 0, // level 0: boundary at 2 arrays
    3, 0, // level 0: boundary at 3 arrays
    6, 0, // level 0: boundary at 6 arrays
    255, 255, 255, 255, 255, 255, 255, 255, // payload[0] = -1
    2, 0, 0, 0, 0, 0, 0, 0, // payload[1] = 2
    253, 255, 255, 255, 255, 255, 255, 255, // payload[2] = -3
    4, 0, 0, 0, 0, 0, 0, 0, // payload[3] = 4
    251, 255, 255, 255, 255, 255, 255, 255, // payload[4] = -5
    6, 0, 0, 0, 0, 0, 0, 0, // payload[5] = 6
    249, 255, 255, 255, 255, 255, 255, 255, // payload[6] = -7
    8, 0, 0, 0, 0, 0, 0, 0, // payload[7] = 8
    247, 255, 255, 255, 255, 255, 255, 255, // payload[8] = -9
    10, 0, 0, 0, 0, 0, 0, 0, // payload[9] = 10
    245, 255, 255, 255, 255, 255, 255, 255, // payload[10] = -11
    12, 0, 0, 0, 0, 0, 0, 0, // payload[11] = 12
];
pub const NESTED_2_FIXED_I64_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::I64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_i64() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::I64);
    let data = nested_2_fixed_value([
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::I64(vec![-1, 2].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::I64(vec![-3, 4].into())),
        ]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::I64(
            vec![-5, 6].into(),
        ))]),
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::I64(vec![-7, 8].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::I64(vec![-9, 10].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::I64(vec![-11, 12].into())),
        ]),
    ]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_I64,
        &NESTED_2_FIXED_I64_INODES,
    );
}

// ---------------------------------------------------------------------------
// F16
// ---------------------------------------------------------------------------

pub const NESTED_2_FIXED_F16: [u8; 40] = [
    8, 0, // topology start pointer
    4, 0, // topology word count (1 level, sentinels included)
    16, 0, // payload start pointer
    6, 0, // array count
    255, 255, // level 0: sentinel
    2, 0, // level 0: boundary at 2 arrays
    3, 0, // level 0: boundary at 3 arrays
    6, 0, // level 0: boundary at 6 arrays
    0, 62, // payload[0] = 1.5
    0, 192, // payload[1] = -2.0
    128, 66, // payload[2] = 3.25
    0, 68, // payload[3] = 4.0
    128, 197, // payload[4] = -5.5
    0, 70, // payload[5] = 6.0
    128, 71, // payload[6] = 7.5
    0, 200, // payload[7] = -8.0
    160, 72, // payload[8] = 9.25
    0, 73, // payload[9] = 10.0
    192, 201, // payload[10] = -11.5
    0, 74, // payload[11] = 12.0
];
pub const NESTED_2_FIXED_F16_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::F16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_f16() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::F16);
    let data = nested_2_fixed_value([
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::F16(
                vec![f16::from_f32(1.5), f16::from_f32(-2.0)].into(),
            )),
            PyroValue::PrimitiveList(PrimitiveValueList::F16(
                vec![f16::from_f32(3.25), f16::from_f32(4.0)].into(),
            )),
        ]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::F16(
            vec![f16::from_f32(-5.5), f16::from_f32(6.0)].into(),
        ))]),
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::F16(
                vec![f16::from_f32(7.5), f16::from_f32(-8.0)].into(),
            )),
            PyroValue::PrimitiveList(PrimitiveValueList::F16(
                vec![f16::from_f32(9.25), f16::from_f32(10.0)].into(),
            )),
            PyroValue::PrimitiveList(PrimitiveValueList::F16(
                vec![f16::from_f32(-11.5), f16::from_f32(12.0)].into(),
            )),
        ]),
    ]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_F16,
        &NESTED_2_FIXED_F16_INODES,
    );
}

// ---------------------------------------------------------------------------
// F32
// ---------------------------------------------------------------------------

pub const NESTED_2_FIXED_F32: [u8; 64] = [
    8, 0, // topology start pointer
    4, 0, // topology word count (1 level, sentinels included)
    16, 0, // payload start pointer
    6, 0, // array count
    255, 255, // level 0: sentinel
    2, 0, // level 0: boundary at 2 arrays
    3, 0, // level 0: boundary at 3 arrays
    6, 0, // level 0: boundary at 6 arrays
    0, 0, 192, 63, // payload[0] = 1.5
    0, 0, 0, 192, // payload[1] = -2.0
    0, 0, 80, 64, // payload[2] = 3.25
    0, 0, 128, 64, // payload[3] = 4.0
    0, 0, 176, 192, // payload[4] = -5.5
    0, 0, 192, 64, // payload[5] = 6.0
    0, 0, 240, 64, // payload[6] = 7.5
    0, 0, 0, 193, // payload[7] = -8.0
    0, 0, 20, 65, // payload[8] = 9.25
    0, 0, 32, 65, // payload[9] = 10.0
    0, 0, 56, 193, // payload[10] = -11.5
    0, 0, 64, 65, // payload[11] = 12.0
];
pub const NESTED_2_FIXED_F32_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::F32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_f32() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::F32);
    let data = nested_2_fixed_value([
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::F32(vec![1.5, -2.0].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::F32(vec![3.25, 4.0].into())),
        ]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::F32(
            vec![-5.5, 6.0].into(),
        ))]),
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::F32(vec![7.5, -8.0].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::F32(vec![9.25, 10.0].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::F32(vec![-11.5, 12.0].into())),
        ]),
    ]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_F32,
        &NESTED_2_FIXED_F32_INODES,
    );
}

// ---------------------------------------------------------------------------
// F64
// ---------------------------------------------------------------------------

pub const NESTED_2_FIXED_F64: [u8; 112] = [
    8, 0, // topology start pointer
    4, 0, // topology word count (1 level, sentinels included)
    16, 0, // payload start pointer
    6, 0, // array count
    255, 255, // level 0: sentinel
    2, 0, // level 0: boundary at 2 arrays
    3, 0, // level 0: boundary at 3 arrays
    6, 0, // level 0: boundary at 6 arrays
    0, 0, 0, 0, 0, 0, 248, 63, // payload[0] = 1.5
    0, 0, 0, 0, 0, 0, 0, 192, // payload[1] = -2.0
    0, 0, 0, 0, 0, 0, 10, 64, // payload[2] = 3.25
    0, 0, 0, 0, 0, 0, 16, 64, // payload[3] = 4.0
    0, 0, 0, 0, 0, 0, 22, 192, // payload[4] = -5.5
    0, 0, 0, 0, 0, 0, 24, 64, // payload[5] = 6.0
    0, 0, 0, 0, 0, 0, 30, 64, // payload[6] = 7.5
    0, 0, 0, 0, 0, 0, 32, 192, // payload[7] = -8.0
    0, 0, 0, 0, 0, 128, 34, 64, // payload[8] = 9.25
    0, 0, 0, 0, 0, 0, 36, 64, // payload[9] = 10.0
    0, 0, 0, 0, 0, 0, 39, 192, // payload[10] = -11.5
    0, 0, 0, 0, 0, 0, 40, 64, // payload[11] = 12.0
];
pub const NESTED_2_FIXED_F64_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::F64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_f64() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::F64);
    let data = nested_2_fixed_value([
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::F64(vec![1.5, -2.0].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::F64(vec![3.25, 4.0].into())),
        ]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::F64(
            vec![-5.5, 6.0].into(),
        ))]),
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::F64(vec![7.5, -8.0].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::F64(vec![9.25, 10.0].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::F64(vec![-11.5, 12.0].into())),
        ]),
    ]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_F64,
        &NESTED_2_FIXED_F64_INODES,
    );
}

// ---------------------------------------------------------------------------
// Timestamp
// ---------------------------------------------------------------------------

pub const NESTED_2_FIXED_TIMESTAMP: [u8; 208] = [
    8, 0, // topology start pointer
    4, 0, // topology word count (1 level, sentinels included)
    16, 0, // payload start pointer
    6, 0, // array count
    255, 255, // level 0: sentinel
    2, 0, // level 0: boundary at 2 arrays
    3, 0, // level 0: boundary at 3 arrays
    6, 0, // level 0: boundary at 6 arrays
    0, 47, 104, 89, 0, 0, 0, 0, // payload[0] = Time(1_500_000_000), low 8 bytes
    0, 0, 0, 0, 0, 0, 0, 0, // payload[0], high 8 bytes
    0, 108, 202, 136, 255, 255, 255, 255, // payload[1] = Time(-2_000_000_000), low 8 bytes
    255, 255, 255, 255, 255, 255, 255, 255, // payload[1], high 8 bytes
    128, 16, 183, 193, 0, 0, 0, 0, // payload[2] = Time(3_250_000_000), low 8 bytes
    0, 0, 0, 0, 0, 0, 0, 0, // payload[2], high 8 bytes
    0, 40, 107, 238, 0, 0, 0, 0, // payload[3] = Time(4_000_000_000), low 8 bytes
    0, 0, 0, 0, 0, 0, 0, 0, // payload[3], high 8 bytes
    0, 169, 44, 184, 254, 255, 255, 255, // payload[4] = Time(-5_500_000_000), low 8 bytes
    255, 255, 255, 255, 255, 255, 255, 255, // payload[4], high 8 bytes
    0, 188, 160, 101, 1, 0, 0, 0, // payload[5] = Time(6_000_000_000), low 8 bytes
    0, 0, 0, 0, 0, 0, 0, 0, // payload[5], high 8 bytes
    0, 235, 8, 191, 1, 0, 0, 0, // payload[6] = Time(7_500_000_000), low 8 bytes
    0, 0, 0, 0, 0, 0, 0, 0, // payload[6], high 8 bytes
    0, 176, 41, 35, 254, 255, 255, 255, // payload[7] = Time(-8_000_000_000), low 8 bytes
    255, 255, 255, 255, 255, 255, 255, 255, // payload[7], high 8 bytes
    128, 204, 87, 39, 2, 0, 0, 0, // payload[8] = Time(9_250_000_000), low 8 bytes
    0, 0, 0, 0, 0, 0, 0, 0, // payload[8], high 8 bytes
    0, 228, 11, 84, 2, 0, 0, 0, // payload[9] = Time(10_000_000_000), low 8 bytes
    0, 0, 0, 0, 0, 0, 0, 0, // payload[9], high 8 bytes
    0, 237, 139, 82, 253, 255, 255, 255, // payload[10] = Time(-11_500_000_000), low 8 bytes
    255, 255, 255, 255, 255, 255, 255, 255, // payload[10], high 8 bytes
    0, 120, 65, 203, 2, 0, 0, 0, // payload[11] = Time(12_000_000_000), low 8 bytes
    0, 0, 0, 0, 0, 0, 0, 0, // payload[11], high 8 bytes
];
pub const NESTED_2_FIXED_TIMESTAMP_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::Timestamp);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_timestamp() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::Timestamp);
    let data = nested_2_fixed_value([
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::Timestamp(
                vec![Time(1_500_000_000), Time(-2_000_000_000)].into(),
            )),
            PyroValue::PrimitiveList(PrimitiveValueList::Timestamp(
                vec![Time(3_250_000_000), Time(4_000_000_000)].into(),
            )),
        ]),
        PyroValue::List(vec![PyroValue::PrimitiveList(
            PrimitiveValueList::Timestamp(vec![Time(-5_500_000_000), Time(6_000_000_000)].into()),
        )]),
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::Timestamp(
                vec![Time(7_500_000_000), Time(-8_000_000_000)].into(),
            )),
            PyroValue::PrimitiveList(PrimitiveValueList::Timestamp(
                vec![Time(9_250_000_000), Time(10_000_000_000)].into(),
            )),
            PyroValue::PrimitiveList(PrimitiveValueList::Timestamp(
                vec![Time(-11_500_000_000), Time(12_000_000_000)].into(),
            )),
        ]),
    ]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_TIMESTAMP,
        &NESTED_2_FIXED_TIMESTAMP_INODES,
    );
}

// ---------------------------------------------------------------------------
// Bool
// ---------------------------------------------------------------------------

pub const NESTED_2_FIXED_BOOL: [u8; 18] = [
    8, 0, // topology start pointer
    4, 0, // topology word count (1 level, sentinels included)
    16, 0, // payload start pointer
    6, 0, // array count
    255, 255, // level 0: sentinel
    2, 0, // level 0: boundary at 2 arrays
    3, 0, // level 0: boundary at 3 arrays
    6, 0,   // level 0: boundary at 6 arrays
    157, // payload bits 0..8 (LSB first) = 1, 0, 1, 1, 1, 0, 0, 1
    1,   // payload bits 8..12 (LSB first) = 1, 0, 0, 0
];
pub const NESTED_2_FIXED_BOOL_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::Bool);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_bool() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::Bool);
    let data = nested_2_fixed_value([
        PyroValue::List(vec![
            PyroValue::from(vec![true, false]),
            PyroValue::from(vec![true, true]),
        ]),
        PyroValue::List(vec![PyroValue::from(vec![true, false])]),
        PyroValue::List(vec![
            PyroValue::from(vec![false, true]),
            PyroValue::from(vec![true, false]),
            PyroValue::from(vec![false, false]),
        ]),
    ]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_BOOL,
        &NESTED_2_FIXED_BOOL_INODES,
    );
}

// `[bool; 8]`: the same shape as `[bool; 2]` above, but every array
// fills a whole byte, so the payload is the arrays' bytes back to back.
pub const NESTED_2_FIXED_BOOL_8: [u8; 22] = [
    8, 0, // topology start pointer
    4, 0, // topology word count (1 level, sentinels included)
    16, 0, // payload start pointer
    6, 0, // array count
    255, 255, // level 0: sentinel
    2, 0, // level 0: boundary at 2 arrays
    3, 0, // level 0: boundary at 3 arrays
    6, 0,   // level 0: boundary at 6 arrays
    1,   // payload: array 0 (LSB first)
    128, // payload: array 1 (LSB first)
    255, // payload: array 2 (LSB first)
    0,   // payload: array 3 (LSB first)
    165, // payload: array 4 (LSB first)
    60,  // payload: array 5 (LSB first)
];
pub const NESTED_2_FIXED_BOOL_8_INODES: [Inode; 1] = nested_2_fixed_sized_inode(WireKind::Bool, 8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_bool_8() {
    let schema = nested_2_fixed_sized_schema(PrimitiveDataType::Bool, 8);
    let data = nested_2_fixed_value([
        PyroValue::List(vec![
            PyroValue::from(vec![true, false, false, false, false, false, false, false]),
            PyroValue::from(vec![false, false, false, false, false, false, false, true]),
        ]),
        PyroValue::List(vec![PyroValue::from(vec![
            true, true, true, true, true, true, true, true,
        ])]),
        PyroValue::List(vec![
            PyroValue::from(vec![false, false, false, false, false, false, false, false]),
            PyroValue::from(vec![true, false, true, false, false, true, false, true]),
            PyroValue::from(vec![false, false, true, true, true, true, false, false]),
        ]),
    ]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_BOOL_8,
        &NESTED_2_FIXED_BOOL_8_INODES,
    );
}

// `[bool; 16]`: the same shape as `[bool; 2]` above, but every array
// fills two whole bytes, so the payload is the arrays' bytes back to back.
pub const NESTED_2_FIXED_BOOL_16: [u8; 28] = [
    8, 0, // topology start pointer
    4, 0, // topology word count (1 level, sentinels included)
    16, 0, // payload start pointer
    6, 0, // array count
    255, 255, // level 0: sentinel
    2, 0, // level 0: boundary at 2 arrays
    3, 0, // level 0: boundary at 3 arrays
    6, 0,   // level 0: boundary at 6 arrays
    1,   // payload: array 0, bits 0..8 (LSB first)
    0,   // payload: array 0, bits 8..16 (LSB first)
    0,   // payload: array 1, bits 0..8 (LSB first)
    128, // payload: array 1, bits 8..16 (LSB first)
    255, // payload: array 2, bits 0..8 (LSB first)
    255, // payload: array 2, bits 8..16 (LSB first)
    0,   // payload: array 3, bits 0..8 (LSB first)
    0,   // payload: array 3, bits 8..16 (LSB first)
    90,  // payload: array 4, bits 0..8 (LSB first)
    165, // payload: array 4, bits 8..16 (LSB first)
    195, // payload: array 5, bits 0..8 (LSB first)
    60,  // payload: array 5, bits 8..16 (LSB first)
];
pub const NESTED_2_FIXED_BOOL_16_INODES: [Inode; 1] =
    nested_2_fixed_sized_inode(WireKind::Bool, 16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_bool_16() {
    let schema = nested_2_fixed_sized_schema(PrimitiveDataType::Bool, 16);
    let data = nested_2_fixed_value([
        PyroValue::List(vec![
            PyroValue::from(vec![
                true, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false, false,
            ]),
            PyroValue::from(vec![
                false, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false, true,
            ]),
        ]),
        PyroValue::List(vec![PyroValue::from(vec![
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true,
        ])]),
        PyroValue::List(vec![
            PyroValue::from(vec![
                false, false, false, false, false, false, false, false, false, false, false, false,
                false, false, false, false,
            ]),
            PyroValue::from(vec![
                false, true, false, true, true, false, true, false, true, false, true, false,
                false, true, false, true,
            ]),
            PyroValue::from(vec![
                true, true, false, false, false, false, true, true, false, false, true, true, true,
                true, false, false,
            ]),
        ]),
    ]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_BOOL_16,
        &NESTED_2_FIXED_BOOL_16_INODES,
    );
}

// ===========================================================================
// Empty lists at every depth
// ===========================================================================
//
// `_EMPTY_d` is the list at depth `d` left empty, every shallower list
// holding exactly one element: `_EMPTY_0` = `[]`, `_EMPTY_1` = `[[]]`.
// No leaf is written, so each buffer is the fixed header, whatever
// topology levels still have entries (a level with no lists below it is
// omitted, sentinel included), then the padding up to the payload's
// alignment. Byte ranges are `[start..end)` offsets into the buffer.
// Captured from the value writer's output.

// ---------------------------------------------------------------------------
// U8, [T; 2]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_2_FIXED_U8_EMPTY_0: [u8; 8] = [
    8, 0, // [0..2] topology start pointer
    0, 0, // [2..4] topology word count (0 levels, sentinels included)
    8, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
];
pub const NESTED_2_FIXED_U8_EMPTY_0_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::U8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_u8_empty_0() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::U8);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_U8_EMPTY_0,
        &NESTED_2_FIXED_U8_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_2_FIXED_U8_EMPTY_1: [u8; 12] = [
    8, 0, // [0..2] topology start pointer
    2, 0, // [2..4] topology word count (1 level, sentinels included)
    12, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
    255, 255, // [8..10] level 0: sentinel
    0, 0, // [10..12] level 0: boundary at 0 arrays
];
pub const NESTED_2_FIXED_U8_EMPTY_1_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::U8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_u8_empty_1() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::U8);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_U8_EMPTY_1,
        &NESTED_2_FIXED_U8_EMPTY_1_INODES,
    );
}

// ---------------------------------------------------------------------------
// I8, [T; 2]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_2_FIXED_I8_EMPTY_0: [u8; 8] = [
    8, 0, // [0..2] topology start pointer
    0, 0, // [2..4] topology word count (0 levels, sentinels included)
    8, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
];
pub const NESTED_2_FIXED_I8_EMPTY_0_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::I8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_i8_empty_0() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::I8);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_I8_EMPTY_0,
        &NESTED_2_FIXED_I8_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_2_FIXED_I8_EMPTY_1: [u8; 12] = [
    8, 0, // [0..2] topology start pointer
    2, 0, // [2..4] topology word count (1 level, sentinels included)
    12, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
    255, 255, // [8..10] level 0: sentinel
    0, 0, // [10..12] level 0: boundary at 0 arrays
];
pub const NESTED_2_FIXED_I8_EMPTY_1_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::I8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_i8_empty_1() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::I8);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_I8_EMPTY_1,
        &NESTED_2_FIXED_I8_EMPTY_1_INODES,
    );
}

// ---------------------------------------------------------------------------
// U16, [T; 2]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_2_FIXED_U16_EMPTY_0: [u8; 8] = [
    8, 0, // [0..2] topology start pointer
    0, 0, // [2..4] topology word count (0 levels, sentinels included)
    8, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
];
pub const NESTED_2_FIXED_U16_EMPTY_0_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::U16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_u16_empty_0() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::U16);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_U16_EMPTY_0,
        &NESTED_2_FIXED_U16_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_2_FIXED_U16_EMPTY_1: [u8; 12] = [
    8, 0, // [0..2] topology start pointer
    2, 0, // [2..4] topology word count (1 level, sentinels included)
    12, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
    255, 255, // [8..10] level 0: sentinel
    0, 0, // [10..12] level 0: boundary at 0 arrays
];
pub const NESTED_2_FIXED_U16_EMPTY_1_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::U16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_u16_empty_1() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::U16);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_U16_EMPTY_1,
        &NESTED_2_FIXED_U16_EMPTY_1_INODES,
    );
}

// ---------------------------------------------------------------------------
// I16, [T; 2]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_2_FIXED_I16_EMPTY_0: [u8; 8] = [
    8, 0, // [0..2] topology start pointer
    0, 0, // [2..4] topology word count (0 levels, sentinels included)
    8, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
];
pub const NESTED_2_FIXED_I16_EMPTY_0_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::I16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_i16_empty_0() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::I16);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_I16_EMPTY_0,
        &NESTED_2_FIXED_I16_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_2_FIXED_I16_EMPTY_1: [u8; 12] = [
    8, 0, // [0..2] topology start pointer
    2, 0, // [2..4] topology word count (1 level, sentinels included)
    12, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
    255, 255, // [8..10] level 0: sentinel
    0, 0, // [10..12] level 0: boundary at 0 arrays
];
pub const NESTED_2_FIXED_I16_EMPTY_1_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::I16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_i16_empty_1() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::I16);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_I16_EMPTY_1,
        &NESTED_2_FIXED_I16_EMPTY_1_INODES,
    );
}

// ---------------------------------------------------------------------------
// U32, [T; 2]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_2_FIXED_U32_EMPTY_0: [u8; 8] = [
    8, 0, // [0..2] topology start pointer
    0, 0, // [2..4] topology word count (0 levels, sentinels included)
    8, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
];
pub const NESTED_2_FIXED_U32_EMPTY_0_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::U32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_u32_empty_0() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::U32);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_U32_EMPTY_0,
        &NESTED_2_FIXED_U32_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_2_FIXED_U32_EMPTY_1: [u8; 12] = [
    8, 0, // [0..2] topology start pointer
    2, 0, // [2..4] topology word count (1 level, sentinels included)
    12, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
    255, 255, // [8..10] level 0: sentinel
    0, 0, // [10..12] level 0: boundary at 0 arrays
];
pub const NESTED_2_FIXED_U32_EMPTY_1_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::U32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_u32_empty_1() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::U32);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_U32_EMPTY_1,
        &NESTED_2_FIXED_U32_EMPTY_1_INODES,
    );
}

// ---------------------------------------------------------------------------
// I32, [T; 2]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_2_FIXED_I32_EMPTY_0: [u8; 8] = [
    8, 0, // [0..2] topology start pointer
    0, 0, // [2..4] topology word count (0 levels, sentinels included)
    8, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
];
pub const NESTED_2_FIXED_I32_EMPTY_0_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::I32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_i32_empty_0() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::I32);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_I32_EMPTY_0,
        &NESTED_2_FIXED_I32_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_2_FIXED_I32_EMPTY_1: [u8; 12] = [
    8, 0, // [0..2] topology start pointer
    2, 0, // [2..4] topology word count (1 level, sentinels included)
    12, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
    255, 255, // [8..10] level 0: sentinel
    0, 0, // [10..12] level 0: boundary at 0 arrays
];
pub const NESTED_2_FIXED_I32_EMPTY_1_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::I32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_i32_empty_1() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::I32);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_I32_EMPTY_1,
        &NESTED_2_FIXED_I32_EMPTY_1_INODES,
    );
}

// ---------------------------------------------------------------------------
// U64, [T; 2]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_2_FIXED_U64_EMPTY_0: [u8; 8] = [
    8, 0, // [0..2] topology start pointer
    0, 0, // [2..4] topology word count (0 levels, sentinels included)
    8, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
];
pub const NESTED_2_FIXED_U64_EMPTY_0_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::U64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_u64_empty_0() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::U64);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_U64_EMPTY_0,
        &NESTED_2_FIXED_U64_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_2_FIXED_U64_EMPTY_1: [u8; 16] = [
    8, 0, // [0..2] topology start pointer
    2, 0, // [2..4] topology word count (1 level, sentinels included)
    16, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
    255, 255, // [8..10] level 0: sentinel
    0, 0, // [10..12] level 0: boundary at 0 arrays
    0, 0, 0, 0, // [12..16] alignment padding (4 bytes)
];
pub const NESTED_2_FIXED_U64_EMPTY_1_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::U64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_u64_empty_1() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::U64);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_U64_EMPTY_1,
        &NESTED_2_FIXED_U64_EMPTY_1_INODES,
    );
}

// ---------------------------------------------------------------------------
// I64, [T; 2]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_2_FIXED_I64_EMPTY_0: [u8; 8] = [
    8, 0, // [0..2] topology start pointer
    0, 0, // [2..4] topology word count (0 levels, sentinels included)
    8, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
];
pub const NESTED_2_FIXED_I64_EMPTY_0_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::I64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_i64_empty_0() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::I64);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_I64_EMPTY_0,
        &NESTED_2_FIXED_I64_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_2_FIXED_I64_EMPTY_1: [u8; 16] = [
    8, 0, // [0..2] topology start pointer
    2, 0, // [2..4] topology word count (1 level, sentinels included)
    16, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
    255, 255, // [8..10] level 0: sentinel
    0, 0, // [10..12] level 0: boundary at 0 arrays
    0, 0, 0, 0, // [12..16] alignment padding (4 bytes)
];
pub const NESTED_2_FIXED_I64_EMPTY_1_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::I64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_i64_empty_1() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::I64);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_I64_EMPTY_1,
        &NESTED_2_FIXED_I64_EMPTY_1_INODES,
    );
}

// ---------------------------------------------------------------------------
// F16, [T; 2]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_2_FIXED_F16_EMPTY_0: [u8; 8] = [
    8, 0, // [0..2] topology start pointer
    0, 0, // [2..4] topology word count (0 levels, sentinels included)
    8, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
];
pub const NESTED_2_FIXED_F16_EMPTY_0_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::F16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_f16_empty_0() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::F16);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_F16_EMPTY_0,
        &NESTED_2_FIXED_F16_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_2_FIXED_F16_EMPTY_1: [u8; 12] = [
    8, 0, // [0..2] topology start pointer
    2, 0, // [2..4] topology word count (1 level, sentinels included)
    12, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
    255, 255, // [8..10] level 0: sentinel
    0, 0, // [10..12] level 0: boundary at 0 arrays
];
pub const NESTED_2_FIXED_F16_EMPTY_1_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::F16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_f16_empty_1() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::F16);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_F16_EMPTY_1,
        &NESTED_2_FIXED_F16_EMPTY_1_INODES,
    );
}

// ---------------------------------------------------------------------------
// F32, [T; 2]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_2_FIXED_F32_EMPTY_0: [u8; 8] = [
    8, 0, // [0..2] topology start pointer
    0, 0, // [2..4] topology word count (0 levels, sentinels included)
    8, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
];
pub const NESTED_2_FIXED_F32_EMPTY_0_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::F32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_f32_empty_0() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::F32);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_F32_EMPTY_0,
        &NESTED_2_FIXED_F32_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_2_FIXED_F32_EMPTY_1: [u8; 12] = [
    8, 0, // [0..2] topology start pointer
    2, 0, // [2..4] topology word count (1 level, sentinels included)
    12, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
    255, 255, // [8..10] level 0: sentinel
    0, 0, // [10..12] level 0: boundary at 0 arrays
];
pub const NESTED_2_FIXED_F32_EMPTY_1_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::F32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_f32_empty_1() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::F32);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_F32_EMPTY_1,
        &NESTED_2_FIXED_F32_EMPTY_1_INODES,
    );
}

// ---------------------------------------------------------------------------
// F64, [T; 2]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_2_FIXED_F64_EMPTY_0: [u8; 8] = [
    8, 0, // [0..2] topology start pointer
    0, 0, // [2..4] topology word count (0 levels, sentinels included)
    8, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
];
pub const NESTED_2_FIXED_F64_EMPTY_0_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::F64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_f64_empty_0() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::F64);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_F64_EMPTY_0,
        &NESTED_2_FIXED_F64_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_2_FIXED_F64_EMPTY_1: [u8; 16] = [
    8, 0, // [0..2] topology start pointer
    2, 0, // [2..4] topology word count (1 level, sentinels included)
    16, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
    255, 255, // [8..10] level 0: sentinel
    0, 0, // [10..12] level 0: boundary at 0 arrays
    0, 0, 0, 0, // [12..16] alignment padding (4 bytes)
];
pub const NESTED_2_FIXED_F64_EMPTY_1_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::F64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_f64_empty_1() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::F64);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_F64_EMPTY_1,
        &NESTED_2_FIXED_F64_EMPTY_1_INODES,
    );
}

// ---------------------------------------------------------------------------
// Timestamp, [T; 2]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_2_FIXED_TIMESTAMP_EMPTY_0: [u8; 16] = [
    8, 0, // [0..2] topology start pointer
    0, 0, // [2..4] topology word count (0 levels, sentinels included)
    16, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
    0, 0, 0, 0, 0, 0, 0, 0, // [8..16] alignment padding to 16 bytes before payload
];
pub const NESTED_2_FIXED_TIMESTAMP_EMPTY_0_INODES: [Inode; 1] =
    nested_2_fixed_inode(WireKind::Timestamp);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_timestamp_empty_0() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::Timestamp);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_TIMESTAMP_EMPTY_0,
        &NESTED_2_FIXED_TIMESTAMP_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_2_FIXED_TIMESTAMP_EMPTY_1: [u8; 16] = [
    8, 0, // [0..2] topology start pointer
    2, 0, // [2..4] topology word count (1 level, sentinels included)
    16, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
    255, 255, // [8..10] level 0: sentinel
    0, 0, // [10..12] level 0: boundary at 0 arrays
    0, 0, 0, 0, // [12..16] alignment padding to 16 bytes before payload
];
pub const NESTED_2_FIXED_TIMESTAMP_EMPTY_1_INODES: [Inode; 1] =
    nested_2_fixed_inode(WireKind::Timestamp);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_timestamp_empty_1() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::Timestamp);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_TIMESTAMP_EMPTY_1,
        &NESTED_2_FIXED_TIMESTAMP_EMPTY_1_INODES,
    );
}

// ---------------------------------------------------------------------------
// Bool, [T; 2]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_2_FIXED_BOOL_EMPTY_0: [u8; 8] = [
    8, 0, // [0..2] topology start pointer
    0, 0, // [2..4] topology word count (0 levels, sentinels included)
    8, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
];
pub const NESTED_2_FIXED_BOOL_EMPTY_0_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::Bool);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_bool_empty_0() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::Bool);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_BOOL_EMPTY_0,
        &NESTED_2_FIXED_BOOL_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_2_FIXED_BOOL_EMPTY_1: [u8; 12] = [
    8, 0, // [0..2] topology start pointer
    2, 0, // [2..4] topology word count (1 level, sentinels included)
    12, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
    255, 255, // [8..10] level 0: sentinel
    0, 0, // [10..12] level 0: boundary at 0 arrays
];
pub const NESTED_2_FIXED_BOOL_EMPTY_1_INODES: [Inode; 1] = nested_2_fixed_inode(WireKind::Bool);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_bool_empty_1() {
    let schema = nested_2_fixed_schema(PrimitiveDataType::Bool);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_BOOL_EMPTY_1,
        &NESTED_2_FIXED_BOOL_EMPTY_1_INODES,
    );
}

// ---------------------------------------------------------------------------
// Bool, [T; 8]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_2_FIXED_BOOL_8_EMPTY_0: [u8; 8] = [
    8, 0, // [0..2] topology start pointer
    0, 0, // [2..4] topology word count (0 levels, sentinels included)
    8, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
];
pub const NESTED_2_FIXED_BOOL_8_EMPTY_0_INODES: [Inode; 1] =
    nested_2_fixed_sized_inode(WireKind::Bool, 8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_bool_8_empty_0() {
    let schema = nested_2_fixed_sized_schema(PrimitiveDataType::Bool, 8);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_BOOL_8_EMPTY_0,
        &NESTED_2_FIXED_BOOL_8_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_2_FIXED_BOOL_8_EMPTY_1: [u8; 12] = [
    8, 0, // [0..2] topology start pointer
    2, 0, // [2..4] topology word count (1 level, sentinels included)
    12, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
    255, 255, // [8..10] level 0: sentinel
    0, 0, // [10..12] level 0: boundary at 0 arrays
];
pub const NESTED_2_FIXED_BOOL_8_EMPTY_1_INODES: [Inode; 1] =
    nested_2_fixed_sized_inode(WireKind::Bool, 8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_bool_8_empty_1() {
    let schema = nested_2_fixed_sized_schema(PrimitiveDataType::Bool, 8);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_BOOL_8_EMPTY_1,
        &NESTED_2_FIXED_BOOL_8_EMPTY_1_INODES,
    );
}

// ---------------------------------------------------------------------------
// Bool, [T; 16]: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_2_FIXED_BOOL_16_EMPTY_0: [u8; 8] = [
    8, 0, // [0..2] topology start pointer
    0, 0, // [2..4] topology word count (0 levels, sentinels included)
    8, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
];
pub const NESTED_2_FIXED_BOOL_16_EMPTY_0_INODES: [Inode; 1] =
    nested_2_fixed_sized_inode(WireKind::Bool, 16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_bool_16_empty_0() {
    let schema = nested_2_fixed_sized_schema(PrimitiveDataType::Bool, 16);
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_BOOL_16_EMPTY_0,
        &NESTED_2_FIXED_BOOL_16_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_2_FIXED_BOOL_16_EMPTY_1: [u8; 12] = [
    8, 0, // [0..2] topology start pointer
    2, 0, // [2..4] topology word count (1 level, sentinels included)
    12, 0, // [4..6] payload start pointer
    0, 0, // [6..8] array count
    255, 255, // [8..10] level 0: sentinel
    0, 0, // [10..12] level 0: boundary at 0 arrays
];
pub const NESTED_2_FIXED_BOOL_16_EMPTY_1_INODES: [Inode; 1] =
    nested_2_fixed_sized_inode(WireKind::Bool, 16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_2_fixed_bool_16_empty_1() {
    let schema = nested_2_fixed_sized_schema(PrimitiveDataType::Bool, 16);
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_2_FIXED_BOOL_16_EMPTY_1,
        &NESTED_2_FIXED_BOOL_16_EMPTY_1_INODES,
    );
}
