//! `List<List<List<T>>>` for every `PrimitiveDataType`.
//!
//! Shared value shape: `[[[a, b], [c]], [[d, e, f]]]`.
//!
//! Layout: `[topology start, topology word count, payload start, leaf count]`,
//! then 2 topology levels of sentinel-prefixed cumulative boundaries
//! (level 0: `MAX, 2, 3`)
//! (level 1: `MAX, 2, 3, 6`)
//! then (after alignment padding) the payload.

#[cfg(test)]
use half::f16;
#[cfg(test)]
use pyro_schema::PrimitiveDataType;
#[cfg(test)]
use pyro_schema::human::{PyroField, PyroSchema, PyroType};

#[cfg(test)]
use crate::{
    PyroValue, value::PrimitiveValueList, value::Time,
    wire::tests::support::run_list_wrapped_roundtrip_u16,
};

use pyro_schema::{Inode, KindMeta, WireKind};

#[cfg(test)]
fn nested_3_schema(leaf: PyroType) -> PyroSchema {
    let mut ty = leaf;
    for _ in 0..3 {
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
fn primitive(primitive: PrimitiveDataType) -> PyroType {
    PyroType::PrimitiveScalar { primitive }
}

/// `[[[a, b], [c]], [[d, e, f]]]` from 3 leaf runs.
#[cfg(test)]
fn nested_3_value(leaves: [PyroValue<'static>; 3]) -> PyroValue<'static> {
    let [l0, l1, l2] = leaves;
    PyroValue::List(vec![
        PyroValue::List(vec![l0, l1]),
        PyroValue::List(vec![l2]),
    ])
}

/// `List<List<List<T>>>` at the root: one `Primitive` inode with `nesting: 3`,
/// its topology slot at 0 and its data slot at 4.
const fn nested_3_inode(kind: WireKind) -> [Inode; 1] {
    [Inode {
        kind,
        kind_meta: KindMeta::Primitive {
            nesting: 3,
            data_offset: 4,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    }]
}

// ---------------------------------------------------------------------------
// U8
// ---------------------------------------------------------------------------

pub const NESTED_3_U8: [u8; 28] = [
    8, 0, // topology start pointer
    7, 0, // topology word count (2 levels, sentinels included)
    22, 0, // payload start pointer
    6, 0, // leaf count
    255, 255, // level 0: sentinel
    2, 0, // level 0: boundary at 2 lists
    3, 0, // level 0: boundary at 3 lists
    255, 255, // level 1: sentinel
    2, 0, // level 1: boundary at 2 leaves
    3, 0, // level 1: boundary at 3 leaves
    6, 0, // level 1: boundary at 6 leaves
    1, // payload[0] = 1
    2, // payload[1] = 2
    3, // payload[2] = 3
    4, // payload[3] = 4
    5, // payload[4] = 5
    6, // payload[5] = 6
];
pub const NESTED_3_U8_INODES: [Inode; 1] = nested_3_inode(WireKind::U8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_u8() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::U8));
    let data = nested_3_value([
        PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![1, 2].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![3].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![4, 5, 6].into())),
    ]);

    run_list_wrapped_roundtrip_u16(schema, data, &NESTED_3_U8, &NESTED_3_U8_INODES);
}

// ---------------------------------------------------------------------------
// I8
// ---------------------------------------------------------------------------

pub const NESTED_3_I8: [u8; 28] = [
    8, 0, // topology start pointer
    7, 0, // topology word count (2 levels, sentinels included)
    22, 0, // payload start pointer
    6, 0, // leaf count
    255, 255, // level 0: sentinel
    2, 0, // level 0: boundary at 2 lists
    3, 0, // level 0: boundary at 3 lists
    255, 255, // level 1: sentinel
    2, 0, // level 1: boundary at 2 leaves
    3, 0, // level 1: boundary at 3 leaves
    6, 0,   // level 1: boundary at 6 leaves
    255, // payload[0] = -1
    2,   // payload[1] = 2
    253, // payload[2] = -3
    4,   // payload[3] = 4
    251, // payload[4] = -5
    6,   // payload[5] = 6
];
pub const NESTED_3_I8_INODES: [Inode; 1] = nested_3_inode(WireKind::I8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_i8() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::I8));
    let data = nested_3_value([
        PyroValue::PrimitiveList(PrimitiveValueList::I8(vec![-1, 2].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::I8(vec![-3].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::I8(vec![4, -5, 6].into())),
    ]);

    run_list_wrapped_roundtrip_u16(schema, data, &NESTED_3_I8, &NESTED_3_I8_INODES);
}

// ---------------------------------------------------------------------------
// U16
// ---------------------------------------------------------------------------

pub const NESTED_3_U16: [u8; 34] = [
    8, 0, // topology start pointer
    7, 0, // topology word count (2 levels, sentinels included)
    22, 0, // payload start pointer
    6, 0, // leaf count
    255, 255, // level 0: sentinel
    2, 0, // level 0: boundary at 2 lists
    3, 0, // level 0: boundary at 3 lists
    255, 255, // level 1: sentinel
    2, 0, // level 1: boundary at 2 leaves
    3, 0, // level 1: boundary at 3 leaves
    6, 0, // level 1: boundary at 6 leaves
    1, 0, // payload[0] = 1
    2, 0, // payload[1] = 2
    3, 0, // payload[2] = 3
    4, 0, // payload[3] = 4
    5, 0, // payload[4] = 5
    6, 0, // payload[5] = 6
];
pub const NESTED_3_U16_INODES: [Inode; 1] = nested_3_inode(WireKind::U16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_u16() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::U16));
    let data = nested_3_value([
        PyroValue::PrimitiveList(PrimitiveValueList::U16(vec![1, 2].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::U16(vec![3].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::U16(vec![4, 5, 6].into())),
    ]);

    run_list_wrapped_roundtrip_u16(schema, data, &NESTED_3_U16, &NESTED_3_U16_INODES);
}

// ---------------------------------------------------------------------------
// I16
// ---------------------------------------------------------------------------

pub const NESTED_3_I16: [u8; 34] = [
    8, 0, // topology start pointer
    7, 0, // topology word count (2 levels, sentinels included)
    22, 0, // payload start pointer
    6, 0, // leaf count
    255, 255, // level 0: sentinel
    2, 0, // level 0: boundary at 2 lists
    3, 0, // level 0: boundary at 3 lists
    255, 255, // level 1: sentinel
    2, 0, // level 1: boundary at 2 leaves
    3, 0, // level 1: boundary at 3 leaves
    6, 0, // level 1: boundary at 6 leaves
    255, 255, // payload[0] = -1
    2, 0, // payload[1] = 2
    253, 255, // payload[2] = -3
    4, 0, // payload[3] = 4
    251, 255, // payload[4] = -5
    6, 0, // payload[5] = 6
];
pub const NESTED_3_I16_INODES: [Inode; 1] = nested_3_inode(WireKind::I16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_i16() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::I16));
    let data = nested_3_value([
        PyroValue::PrimitiveList(PrimitiveValueList::I16(vec![-1, 2].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::I16(vec![-3].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::I16(vec![4, -5, 6].into())),
    ]);

    run_list_wrapped_roundtrip_u16(schema, data, &NESTED_3_I16, &NESTED_3_I16_INODES);
}

// ---------------------------------------------------------------------------
// U32
// ---------------------------------------------------------------------------

pub const NESTED_3_U32: [u8; 48] = [
    8, 0, // topology start pointer
    7, 0, // topology word count (2 levels, sentinels included)
    24, 0, // payload start pointer
    6, 0, // leaf count
    255, 255, // level 0: sentinel
    2, 0, // level 0: boundary at 2 lists
    3, 0, // level 0: boundary at 3 lists
    255, 255, // level 1: sentinel
    2, 0, // level 1: boundary at 2 leaves
    3, 0, // level 1: boundary at 3 leaves
    6, 0, // level 1: boundary at 6 leaves
    0, 0, // alignment padding (2 bytes)
    1, 0, 0, 0, // payload[0] = 1
    2, 0, 0, 0, // payload[1] = 2
    3, 0, 0, 0, // payload[2] = 3
    4, 0, 0, 0, // payload[3] = 4
    5, 0, 0, 0, // payload[4] = 5
    6, 0, 0, 0, // payload[5] = 6
];
pub const NESTED_3_U32_INODES: [Inode; 1] = nested_3_inode(WireKind::U32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_u32() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::U32));
    let data = nested_3_value([
        PyroValue::PrimitiveList(PrimitiveValueList::U32(vec![1, 2].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::U32(vec![3].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::U32(vec![4, 5, 6].into())),
    ]);

    run_list_wrapped_roundtrip_u16(schema, data, &NESTED_3_U32, &NESTED_3_U32_INODES);
}

// ---------------------------------------------------------------------------
// I32
// ---------------------------------------------------------------------------

pub const NESTED_3_I32: [u8; 48] = [
    8, 0, // topology start pointer
    7, 0, // topology word count (2 levels, sentinels included)
    24, 0, // payload start pointer
    6, 0, // leaf count
    255, 255, // level 0: sentinel
    2, 0, // level 0: boundary at 2 lists
    3, 0, // level 0: boundary at 3 lists
    255, 255, // level 1: sentinel
    2, 0, // level 1: boundary at 2 leaves
    3, 0, // level 1: boundary at 3 leaves
    6, 0, // level 1: boundary at 6 leaves
    0, 0, // alignment padding (2 bytes)
    255, 255, 255, 255, // payload[0] = -1
    2, 0, 0, 0, // payload[1] = 2
    253, 255, 255, 255, // payload[2] = -3
    4, 0, 0, 0, // payload[3] = 4
    251, 255, 255, 255, // payload[4] = -5
    6, 0, 0, 0, // payload[5] = 6
];
pub const NESTED_3_I32_INODES: [Inode; 1] = nested_3_inode(WireKind::I32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_i32() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::I32));
    let data = nested_3_value([
        PyroValue::PrimitiveList(PrimitiveValueList::I32(vec![-1, 2].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::I32(vec![-3].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::I32(vec![4, -5, 6].into())),
    ]);

    run_list_wrapped_roundtrip_u16(schema, data, &NESTED_3_I32, &NESTED_3_I32_INODES);
}

// ---------------------------------------------------------------------------
// U64
// ---------------------------------------------------------------------------

pub const NESTED_3_U64: [u8; 72] = [
    8, 0, // topology start pointer
    7, 0, // topology word count (2 levels, sentinels included)
    24, 0, // payload start pointer
    6, 0, // leaf count
    255, 255, // level 0: sentinel
    2, 0, // level 0: boundary at 2 lists
    3, 0, // level 0: boundary at 3 lists
    255, 255, // level 1: sentinel
    2, 0, // level 1: boundary at 2 leaves
    3, 0, // level 1: boundary at 3 leaves
    6, 0, // level 1: boundary at 6 leaves
    0, 0, // alignment padding to 8 bytes before payload
    1, 0, 0, 0, 0, 0, 0, 0, // payload[0] = 1
    2, 0, 0, 0, 0, 0, 0, 0, // payload[1] = 2
    3, 0, 0, 0, 0, 0, 0, 0, // payload[2] = 3
    4, 0, 0, 0, 0, 0, 0, 0, // payload[3] = 4
    5, 0, 0, 0, 0, 0, 0, 0, // payload[4] = 5
    6, 0, 0, 0, 0, 0, 0, 0, // payload[5] = 6
];
pub const NESTED_3_U64_INODES: [Inode; 1] = nested_3_inode(WireKind::U64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_u64() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::U64));
    let data = nested_3_value([
        PyroValue::PrimitiveList(PrimitiveValueList::U64(vec![1, 2].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::U64(vec![3].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::U64(vec![4, 5, 6].into())),
    ]);

    run_list_wrapped_roundtrip_u16(schema, data, &NESTED_3_U64, &NESTED_3_U64_INODES);
}

// ---------------------------------------------------------------------------
// I64
// ---------------------------------------------------------------------------

pub const NESTED_3_I64: [u8; 72] = [
    8, 0, // topology start pointer
    7, 0, // topology word count (2 levels, sentinels included)
    24, 0, // payload start pointer
    6, 0, // leaf count
    255, 255, // level 0: sentinel
    2, 0, // level 0: boundary at 2 lists
    3, 0, // level 0: boundary at 3 lists
    255, 255, // level 1: sentinel
    2, 0, // level 1: boundary at 2 leaves
    3, 0, // level 1: boundary at 3 leaves
    6, 0, // level 1: boundary at 6 leaves
    0, 0, // alignment padding to 8 bytes before payload
    255, 255, 255, 255, 255, 255, 255, 255, // payload[0] = -1
    2, 0, 0, 0, 0, 0, 0, 0, // payload[1] = 2
    253, 255, 255, 255, 255, 255, 255, 255, // payload[2] = -3
    4, 0, 0, 0, 0, 0, 0, 0, // payload[3] = 4
    251, 255, 255, 255, 255, 255, 255, 255, // payload[4] = -5
    6, 0, 0, 0, 0, 0, 0, 0, // payload[5] = 6
];
pub const NESTED_3_I64_INODES: [Inode; 1] = nested_3_inode(WireKind::I64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_i64() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::I64));
    let data = nested_3_value([
        PyroValue::PrimitiveList(PrimitiveValueList::I64(vec![-1, 2].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::I64(vec![-3].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::I64(vec![4, -5, 6].into())),
    ]);

    run_list_wrapped_roundtrip_u16(schema, data, &NESTED_3_I64, &NESTED_3_I64_INODES);
}

// ---------------------------------------------------------------------------
// F16
// ---------------------------------------------------------------------------

pub const NESTED_3_F16: [u8; 34] = [
    8, 0, // topology start pointer
    7, 0, // topology word count (2 levels, sentinels included)
    22, 0, // payload start pointer
    6, 0, // leaf count
    255, 255, // level 0: sentinel
    2, 0, // level 0: boundary at 2 lists
    3, 0, // level 0: boundary at 3 lists
    255, 255, // level 1: sentinel
    2, 0, // level 1: boundary at 2 leaves
    3, 0, // level 1: boundary at 3 leaves
    6, 0, // level 1: boundary at 6 leaves
    0, 62, // payload[0] = 1.5
    0, 192, // payload[1] = -2.0
    128, 66, // payload[2] = 3.25
    0, 68, // payload[3] = 4.0
    128, 197, // payload[4] = -5.5
    0, 70, // payload[5] = 6.0
];
pub const NESTED_3_F16_INODES: [Inode; 1] = nested_3_inode(WireKind::F16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_f16() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::F16));
    let data = nested_3_value([
        PyroValue::PrimitiveList(PrimitiveValueList::F16(
            vec![f16::from_f32(1.5), f16::from_f32(-2.0)].into(),
        )),
        PyroValue::PrimitiveList(PrimitiveValueList::F16(vec![f16::from_f32(3.25)].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::F16(
            vec![f16::from_f32(4.0), f16::from_f32(-5.5), f16::from_f32(6.0)].into(),
        )),
    ]);

    run_list_wrapped_roundtrip_u16(schema, data, &NESTED_3_F16, &NESTED_3_F16_INODES);
}

// ---------------------------------------------------------------------------
// F32
// ---------------------------------------------------------------------------

pub const NESTED_3_F32: [u8; 48] = [
    8, 0, // topology start pointer
    7, 0, // topology word count (2 levels, sentinels included)
    24, 0, // payload start pointer
    6, 0, // leaf count
    255, 255, // level 0: sentinel
    2, 0, // level 0: boundary at 2 lists
    3, 0, // level 0: boundary at 3 lists
    255, 255, // level 1: sentinel
    2, 0, // level 1: boundary at 2 leaves
    3, 0, // level 1: boundary at 3 leaves
    6, 0, // level 1: boundary at 6 leaves
    0, 0, // alignment padding (2 bytes)
    0, 0, 192, 63, // payload[0] = 1.5
    0, 0, 0, 192, // payload[1] = -2.0
    0, 0, 80, 64, // payload[2] = 3.25
    0, 0, 128, 64, // payload[3] = 4.0
    0, 0, 176, 192, // payload[4] = -5.5
    0, 0, 192, 64, // payload[5] = 6.0
];
pub const NESTED_3_F32_INODES: [Inode; 1] = nested_3_inode(WireKind::F32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_f32() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::F32));
    let data = nested_3_value([
        PyroValue::PrimitiveList(PrimitiveValueList::F32(vec![1.5, -2.0].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::F32(vec![3.25].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::F32(vec![4.0, -5.5, 6.0].into())),
    ]);

    run_list_wrapped_roundtrip_u16(schema, data, &NESTED_3_F32, &NESTED_3_F32_INODES);
}

// ---------------------------------------------------------------------------
// F64
// ---------------------------------------------------------------------------

pub const NESTED_3_F64: [u8; 72] = [
    8, 0, // topology start pointer
    7, 0, // topology word count (2 levels, sentinels included)
    24, 0, // payload start pointer
    6, 0, // leaf count
    255, 255, // level 0: sentinel
    2, 0, // level 0: boundary at 2 lists
    3, 0, // level 0: boundary at 3 lists
    255, 255, // level 1: sentinel
    2, 0, // level 1: boundary at 2 leaves
    3, 0, // level 1: boundary at 3 leaves
    6, 0, // level 1: boundary at 6 leaves
    0, 0, // alignment padding to 8 bytes before payload
    0, 0, 0, 0, 0, 0, 248, 63, // payload[0] = 1.5
    0, 0, 0, 0, 0, 0, 0, 192, // payload[1] = -2.0
    0, 0, 0, 0, 0, 0, 10, 64, // payload[2] = 3.25
    0, 0, 0, 0, 0, 0, 16, 64, // payload[3] = 4.0
    0, 0, 0, 0, 0, 0, 22, 192, // payload[4] = -5.5
    0, 0, 0, 0, 0, 0, 24, 64, // payload[5] = 6.0
];
pub const NESTED_3_F64_INODES: [Inode; 1] = nested_3_inode(WireKind::F64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_f64() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::F64));
    let data = nested_3_value([
        PyroValue::PrimitiveList(PrimitiveValueList::F64(vec![1.5, -2.0].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::F64(vec![3.25].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::F64(vec![4.0, -5.5, 6.0].into())),
    ]);

    run_list_wrapped_roundtrip_u16(schema, data, &NESTED_3_F64, &NESTED_3_F64_INODES);
}

// ---------------------------------------------------------------------------
// Bool
// ---------------------------------------------------------------------------

pub const NESTED_3_BOOL: [u8; 23] = [
    8, 0, // topology start pointer
    7, 0, // topology word count (2 levels, sentinels included)
    22, 0, // payload start pointer
    6, 0, // leaf count
    255, 255, // level 0: sentinel
    2, 0, // level 0: boundary at 2 lists
    3, 0, // level 0: boundary at 3 lists
    255, 255, // level 1: sentinel
    2, 0, // level 1: boundary at 2 leaves
    3, 0, // level 1: boundary at 3 leaves
    6, 0,  // level 1: boundary at 6 leaves
    29, // payload bits 0..6 (LSB first) = 1, 0, 1, 1, 1, 0
];
pub const NESTED_3_BOOL_INODES: [Inode; 1] = nested_3_inode(WireKind::Bool);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_bool() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::Bool));
    let data = nested_3_value([
        PyroValue::from(vec![true, false]),
        PyroValue::from(vec![true]),
        PyroValue::from(vec![true, true, false]),
    ]);

    run_list_wrapped_roundtrip_u16(schema, data, &NESTED_3_BOOL, &NESTED_3_BOOL_INODES);
}

// ---------------------------------------------------------------------------
// Timestamp
// ---------------------------------------------------------------------------

pub const NESTED_3_TIMESTAMP: [u8; 128] = [
    8, 0, // topology start pointer
    7, 0, // topology word count (2 levels, sentinels included)
    32, 0, // payload start pointer
    6, 0, // leaf count
    255, 255, // level 0: sentinel
    2, 0, // level 0: boundary at 2 lists
    3, 0, // level 0: boundary at 3 lists
    255, 255, // level 1: sentinel
    2, 0, // level 1: boundary at 2 leaves
    3, 0, // level 1: boundary at 3 leaves
    6, 0, // level 1: boundary at 6 leaves
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // alignment padding to 16 bytes before payload
    232, 205, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // payload[0] = Time(1000001000)
    48, 100, 202, 136, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, // payload[1] = Time(-2000002000)
    184, 206, 157, 208, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // payload[2] = Time(3500003000)
    160, 55, 107, 238, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // payload[3] = Time(4000004000)
    120, 250, 249, 213, 254, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, // payload[4] = Time(-5000005000)
    112, 23, 42, 54, 254, 156, 151, 23, 0, 0, 0, 0, 0, 0, 0,
    0, // payload[5] = Time(1700000000000006000)
];
pub const NESTED_3_TIMESTAMP_INODES: [Inode; 1] = nested_3_inode(WireKind::Timestamp);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_timestamp() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::Timestamp));
    let data = nested_3_value([
        PyroValue::from(vec![Time(1_000_001_000), Time(-2_000_002_000)]),
        PyroValue::from(vec![Time(3_500_003_000)]),
        PyroValue::from(vec![
            Time(4_000_004_000),
            Time(-5_000_005_000),
            Time(1_700_000_000_000_006_000),
        ]),
    ]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_TIMESTAMP,
        &NESTED_3_TIMESTAMP_INODES,
    );
}

// ===========================================================================
// Empty lists at every depth
// ===========================================================================
//
// `_EMPTY_d` is the list at depth `d` left empty, every shallower list
// holding exactly one element: `_EMPTY_0` = `[]`, `_EMPTY_1` = `[[]]`, `_EMPTY_2` = `[[[]]]`.
// No leaf is written, so each buffer is the fixed header, whatever
// topology levels still have entries (a level with no lists below it is
// omitted, sentinel included), then the padding up to the payload's
// alignment. Byte ranges are `[start..end)` offsets into the buffer.
// Captured from the value writer's output.

// ---------------------------------------------------------------------------
// U8: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_3_U8_EMPTY_0: [u8; 8] = [
    8, 0, // [0..2] topology start pointer
    0, 0, // [2..4] topology word count (0 levels, sentinels included)
    8, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
];
pub const NESTED_3_U8_EMPTY_0_INODES: [Inode; 1] = nested_3_inode(WireKind::U8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_u8_empty_0() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::U8));
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_U8_EMPTY_0,
        &NESTED_3_U8_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_3_U8_EMPTY_1: [u8; 12] = [
    8, 0, // [0..2] topology start pointer
    2, 0, // [2..4] topology word count (1 level, sentinels included)
    12, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
    255, 255, // [8..10] level 0: sentinel
    0, 0, // [10..12] level 0: boundary at 0 lists
];
pub const NESTED_3_U8_EMPTY_1_INODES: [Inode; 1] = nested_3_inode(WireKind::U8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_u8_empty_1() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::U8));
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_U8_EMPTY_1,
        &NESTED_3_U8_EMPTY_1_INODES,
    );
}

/// `[[[]]]`
pub const NESTED_3_U8_EMPTY_2: [u8; 16] = [
    8, 0, // [0..2] topology start pointer
    4, 0, // [2..4] topology word count (2 levels, sentinels included)
    16, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
    255, 255, // [8..10] level 0: sentinel
    1, 0, // [10..12] level 0: boundary at 1 lists
    255, 255, // [12..14] level 1: sentinel
    0, 0, // [14..16] level 1: boundary at 0 leaves
];
pub const NESTED_3_U8_EMPTY_2_INODES: [Inode; 1] = nested_3_inode(WireKind::U8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_u8_empty_2() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::U8));
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::PrimitiveList(
        PrimitiveValueList::U8(vec![].into()),
    )])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_U8_EMPTY_2,
        &NESTED_3_U8_EMPTY_2_INODES,
    );
}

// ---------------------------------------------------------------------------
// I8: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_3_I8_EMPTY_0: [u8; 8] = [
    8, 0, // [0..2] topology start pointer
    0, 0, // [2..4] topology word count (0 levels, sentinels included)
    8, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
];
pub const NESTED_3_I8_EMPTY_0_INODES: [Inode; 1] = nested_3_inode(WireKind::I8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_i8_empty_0() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::I8));
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_I8_EMPTY_0,
        &NESTED_3_I8_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_3_I8_EMPTY_1: [u8; 12] = [
    8, 0, // [0..2] topology start pointer
    2, 0, // [2..4] topology word count (1 level, sentinels included)
    12, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
    255, 255, // [8..10] level 0: sentinel
    0, 0, // [10..12] level 0: boundary at 0 lists
];
pub const NESTED_3_I8_EMPTY_1_INODES: [Inode; 1] = nested_3_inode(WireKind::I8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_i8_empty_1() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::I8));
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_I8_EMPTY_1,
        &NESTED_3_I8_EMPTY_1_INODES,
    );
}

/// `[[[]]]`
pub const NESTED_3_I8_EMPTY_2: [u8; 16] = [
    8, 0, // [0..2] topology start pointer
    4, 0, // [2..4] topology word count (2 levels, sentinels included)
    16, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
    255, 255, // [8..10] level 0: sentinel
    1, 0, // [10..12] level 0: boundary at 1 lists
    255, 255, // [12..14] level 1: sentinel
    0, 0, // [14..16] level 1: boundary at 0 leaves
];
pub const NESTED_3_I8_EMPTY_2_INODES: [Inode; 1] = nested_3_inode(WireKind::I8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_i8_empty_2() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::I8));
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::PrimitiveList(
        PrimitiveValueList::I8(vec![].into()),
    )])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_I8_EMPTY_2,
        &NESTED_3_I8_EMPTY_2_INODES,
    );
}

// ---------------------------------------------------------------------------
// U16: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_3_U16_EMPTY_0: [u8; 8] = [
    8, 0, // [0..2] topology start pointer
    0, 0, // [2..4] topology word count (0 levels, sentinels included)
    8, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
];
pub const NESTED_3_U16_EMPTY_0_INODES: [Inode; 1] = nested_3_inode(WireKind::U16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_u16_empty_0() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::U16));
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_U16_EMPTY_0,
        &NESTED_3_U16_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_3_U16_EMPTY_1: [u8; 12] = [
    8, 0, // [0..2] topology start pointer
    2, 0, // [2..4] topology word count (1 level, sentinels included)
    12, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
    255, 255, // [8..10] level 0: sentinel
    0, 0, // [10..12] level 0: boundary at 0 lists
];
pub const NESTED_3_U16_EMPTY_1_INODES: [Inode; 1] = nested_3_inode(WireKind::U16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_u16_empty_1() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::U16));
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_U16_EMPTY_1,
        &NESTED_3_U16_EMPTY_1_INODES,
    );
}

/// `[[[]]]`
pub const NESTED_3_U16_EMPTY_2: [u8; 16] = [
    8, 0, // [0..2] topology start pointer
    4, 0, // [2..4] topology word count (2 levels, sentinels included)
    16, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
    255, 255, // [8..10] level 0: sentinel
    1, 0, // [10..12] level 0: boundary at 1 lists
    255, 255, // [12..14] level 1: sentinel
    0, 0, // [14..16] level 1: boundary at 0 leaves
];
pub const NESTED_3_U16_EMPTY_2_INODES: [Inode; 1] = nested_3_inode(WireKind::U16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_u16_empty_2() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::U16));
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::PrimitiveList(
        PrimitiveValueList::U16(vec![].into()),
    )])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_U16_EMPTY_2,
        &NESTED_3_U16_EMPTY_2_INODES,
    );
}

// ---------------------------------------------------------------------------
// I16: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_3_I16_EMPTY_0: [u8; 8] = [
    8, 0, // [0..2] topology start pointer
    0, 0, // [2..4] topology word count (0 levels, sentinels included)
    8, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
];
pub const NESTED_3_I16_EMPTY_0_INODES: [Inode; 1] = nested_3_inode(WireKind::I16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_i16_empty_0() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::I16));
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_I16_EMPTY_0,
        &NESTED_3_I16_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_3_I16_EMPTY_1: [u8; 12] = [
    8, 0, // [0..2] topology start pointer
    2, 0, // [2..4] topology word count (1 level, sentinels included)
    12, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
    255, 255, // [8..10] level 0: sentinel
    0, 0, // [10..12] level 0: boundary at 0 lists
];
pub const NESTED_3_I16_EMPTY_1_INODES: [Inode; 1] = nested_3_inode(WireKind::I16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_i16_empty_1() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::I16));
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_I16_EMPTY_1,
        &NESTED_3_I16_EMPTY_1_INODES,
    );
}

/// `[[[]]]`
pub const NESTED_3_I16_EMPTY_2: [u8; 16] = [
    8, 0, // [0..2] topology start pointer
    4, 0, // [2..4] topology word count (2 levels, sentinels included)
    16, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
    255, 255, // [8..10] level 0: sentinel
    1, 0, // [10..12] level 0: boundary at 1 lists
    255, 255, // [12..14] level 1: sentinel
    0, 0, // [14..16] level 1: boundary at 0 leaves
];
pub const NESTED_3_I16_EMPTY_2_INODES: [Inode; 1] = nested_3_inode(WireKind::I16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_i16_empty_2() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::I16));
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::PrimitiveList(
        PrimitiveValueList::I16(vec![].into()),
    )])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_I16_EMPTY_2,
        &NESTED_3_I16_EMPTY_2_INODES,
    );
}

// ---------------------------------------------------------------------------
// U32: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_3_U32_EMPTY_0: [u8; 8] = [
    8, 0, // [0..2] topology start pointer
    0, 0, // [2..4] topology word count (0 levels, sentinels included)
    8, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
];
pub const NESTED_3_U32_EMPTY_0_INODES: [Inode; 1] = nested_3_inode(WireKind::U32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_u32_empty_0() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::U32));
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_U32_EMPTY_0,
        &NESTED_3_U32_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_3_U32_EMPTY_1: [u8; 12] = [
    8, 0, // [0..2] topology start pointer
    2, 0, // [2..4] topology word count (1 level, sentinels included)
    12, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
    255, 255, // [8..10] level 0: sentinel
    0, 0, // [10..12] level 0: boundary at 0 lists
];
pub const NESTED_3_U32_EMPTY_1_INODES: [Inode; 1] = nested_3_inode(WireKind::U32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_u32_empty_1() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::U32));
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_U32_EMPTY_1,
        &NESTED_3_U32_EMPTY_1_INODES,
    );
}

/// `[[[]]]`
pub const NESTED_3_U32_EMPTY_2: [u8; 16] = [
    8, 0, // [0..2] topology start pointer
    4, 0, // [2..4] topology word count (2 levels, sentinels included)
    16, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
    255, 255, // [8..10] level 0: sentinel
    1, 0, // [10..12] level 0: boundary at 1 lists
    255, 255, // [12..14] level 1: sentinel
    0, 0, // [14..16] level 1: boundary at 0 leaves
];
pub const NESTED_3_U32_EMPTY_2_INODES: [Inode; 1] = nested_3_inode(WireKind::U32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_u32_empty_2() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::U32));
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::PrimitiveList(
        PrimitiveValueList::U32(vec![].into()),
    )])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_U32_EMPTY_2,
        &NESTED_3_U32_EMPTY_2_INODES,
    );
}

// ---------------------------------------------------------------------------
// I32: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_3_I32_EMPTY_0: [u8; 8] = [
    8, 0, // [0..2] topology start pointer
    0, 0, // [2..4] topology word count (0 levels, sentinels included)
    8, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
];
pub const NESTED_3_I32_EMPTY_0_INODES: [Inode; 1] = nested_3_inode(WireKind::I32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_i32_empty_0() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::I32));
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_I32_EMPTY_0,
        &NESTED_3_I32_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_3_I32_EMPTY_1: [u8; 12] = [
    8, 0, // [0..2] topology start pointer
    2, 0, // [2..4] topology word count (1 level, sentinels included)
    12, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
    255, 255, // [8..10] level 0: sentinel
    0, 0, // [10..12] level 0: boundary at 0 lists
];
pub const NESTED_3_I32_EMPTY_1_INODES: [Inode; 1] = nested_3_inode(WireKind::I32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_i32_empty_1() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::I32));
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_I32_EMPTY_1,
        &NESTED_3_I32_EMPTY_1_INODES,
    );
}

/// `[[[]]]`
pub const NESTED_3_I32_EMPTY_2: [u8; 16] = [
    8, 0, // [0..2] topology start pointer
    4, 0, // [2..4] topology word count (2 levels, sentinels included)
    16, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
    255, 255, // [8..10] level 0: sentinel
    1, 0, // [10..12] level 0: boundary at 1 lists
    255, 255, // [12..14] level 1: sentinel
    0, 0, // [14..16] level 1: boundary at 0 leaves
];
pub const NESTED_3_I32_EMPTY_2_INODES: [Inode; 1] = nested_3_inode(WireKind::I32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_i32_empty_2() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::I32));
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::PrimitiveList(
        PrimitiveValueList::I32(vec![].into()),
    )])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_I32_EMPTY_2,
        &NESTED_3_I32_EMPTY_2_INODES,
    );
}

// ---------------------------------------------------------------------------
// U64: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_3_U64_EMPTY_0: [u8; 8] = [
    8, 0, // [0..2] topology start pointer
    0, 0, // [2..4] topology word count (0 levels, sentinels included)
    8, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
];
pub const NESTED_3_U64_EMPTY_0_INODES: [Inode; 1] = nested_3_inode(WireKind::U64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_u64_empty_0() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::U64));
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_U64_EMPTY_0,
        &NESTED_3_U64_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_3_U64_EMPTY_1: [u8; 16] = [
    8, 0, // [0..2] topology start pointer
    2, 0, // [2..4] topology word count (1 level, sentinels included)
    16, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
    255, 255, // [8..10] level 0: sentinel
    0, 0, // [10..12] level 0: boundary at 0 lists
    0, 0, 0, 0, // [12..16] alignment padding (4 bytes)
];
pub const NESTED_3_U64_EMPTY_1_INODES: [Inode; 1] = nested_3_inode(WireKind::U64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_u64_empty_1() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::U64));
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_U64_EMPTY_1,
        &NESTED_3_U64_EMPTY_1_INODES,
    );
}

/// `[[[]]]`
pub const NESTED_3_U64_EMPTY_2: [u8; 16] = [
    8, 0, // [0..2] topology start pointer
    4, 0, // [2..4] topology word count (2 levels, sentinels included)
    16, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
    255, 255, // [8..10] level 0: sentinel
    1, 0, // [10..12] level 0: boundary at 1 lists
    255, 255, // [12..14] level 1: sentinel
    0, 0, // [14..16] level 1: boundary at 0 leaves
];
pub const NESTED_3_U64_EMPTY_2_INODES: [Inode; 1] = nested_3_inode(WireKind::U64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_u64_empty_2() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::U64));
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::PrimitiveList(
        PrimitiveValueList::U64(vec![].into()),
    )])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_U64_EMPTY_2,
        &NESTED_3_U64_EMPTY_2_INODES,
    );
}

// ---------------------------------------------------------------------------
// I64: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_3_I64_EMPTY_0: [u8; 8] = [
    8, 0, // [0..2] topology start pointer
    0, 0, // [2..4] topology word count (0 levels, sentinels included)
    8, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
];
pub const NESTED_3_I64_EMPTY_0_INODES: [Inode; 1] = nested_3_inode(WireKind::I64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_i64_empty_0() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::I64));
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_I64_EMPTY_0,
        &NESTED_3_I64_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_3_I64_EMPTY_1: [u8; 16] = [
    8, 0, // [0..2] topology start pointer
    2, 0, // [2..4] topology word count (1 level, sentinels included)
    16, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
    255, 255, // [8..10] level 0: sentinel
    0, 0, // [10..12] level 0: boundary at 0 lists
    0, 0, 0, 0, // [12..16] alignment padding (4 bytes)
];
pub const NESTED_3_I64_EMPTY_1_INODES: [Inode; 1] = nested_3_inode(WireKind::I64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_i64_empty_1() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::I64));
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_I64_EMPTY_1,
        &NESTED_3_I64_EMPTY_1_INODES,
    );
}

/// `[[[]]]`
pub const NESTED_3_I64_EMPTY_2: [u8; 16] = [
    8, 0, // [0..2] topology start pointer
    4, 0, // [2..4] topology word count (2 levels, sentinels included)
    16, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
    255, 255, // [8..10] level 0: sentinel
    1, 0, // [10..12] level 0: boundary at 1 lists
    255, 255, // [12..14] level 1: sentinel
    0, 0, // [14..16] level 1: boundary at 0 leaves
];
pub const NESTED_3_I64_EMPTY_2_INODES: [Inode; 1] = nested_3_inode(WireKind::I64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_i64_empty_2() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::I64));
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::PrimitiveList(
        PrimitiveValueList::I64(vec![].into()),
    )])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_I64_EMPTY_2,
        &NESTED_3_I64_EMPTY_2_INODES,
    );
}

// ---------------------------------------------------------------------------
// F16: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_3_F16_EMPTY_0: [u8; 8] = [
    8, 0, // [0..2] topology start pointer
    0, 0, // [2..4] topology word count (0 levels, sentinels included)
    8, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
];
pub const NESTED_3_F16_EMPTY_0_INODES: [Inode; 1] = nested_3_inode(WireKind::F16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_f16_empty_0() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::F16));
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_F16_EMPTY_0,
        &NESTED_3_F16_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_3_F16_EMPTY_1: [u8; 12] = [
    8, 0, // [0..2] topology start pointer
    2, 0, // [2..4] topology word count (1 level, sentinels included)
    12, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
    255, 255, // [8..10] level 0: sentinel
    0, 0, // [10..12] level 0: boundary at 0 lists
];
pub const NESTED_3_F16_EMPTY_1_INODES: [Inode; 1] = nested_3_inode(WireKind::F16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_f16_empty_1() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::F16));
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_F16_EMPTY_1,
        &NESTED_3_F16_EMPTY_1_INODES,
    );
}

/// `[[[]]]`
pub const NESTED_3_F16_EMPTY_2: [u8; 16] = [
    8, 0, // [0..2] topology start pointer
    4, 0, // [2..4] topology word count (2 levels, sentinels included)
    16, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
    255, 255, // [8..10] level 0: sentinel
    1, 0, // [10..12] level 0: boundary at 1 lists
    255, 255, // [12..14] level 1: sentinel
    0, 0, // [14..16] level 1: boundary at 0 leaves
];
pub const NESTED_3_F16_EMPTY_2_INODES: [Inode; 1] = nested_3_inode(WireKind::F16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_f16_empty_2() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::F16));
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::PrimitiveList(
        PrimitiveValueList::F16(vec![].into()),
    )])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_F16_EMPTY_2,
        &NESTED_3_F16_EMPTY_2_INODES,
    );
}

// ---------------------------------------------------------------------------
// F32: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_3_F32_EMPTY_0: [u8; 8] = [
    8, 0, // [0..2] topology start pointer
    0, 0, // [2..4] topology word count (0 levels, sentinels included)
    8, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
];
pub const NESTED_3_F32_EMPTY_0_INODES: [Inode; 1] = nested_3_inode(WireKind::F32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_f32_empty_0() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::F32));
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_F32_EMPTY_0,
        &NESTED_3_F32_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_3_F32_EMPTY_1: [u8; 12] = [
    8, 0, // [0..2] topology start pointer
    2, 0, // [2..4] topology word count (1 level, sentinels included)
    12, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
    255, 255, // [8..10] level 0: sentinel
    0, 0, // [10..12] level 0: boundary at 0 lists
];
pub const NESTED_3_F32_EMPTY_1_INODES: [Inode; 1] = nested_3_inode(WireKind::F32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_f32_empty_1() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::F32));
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_F32_EMPTY_1,
        &NESTED_3_F32_EMPTY_1_INODES,
    );
}

/// `[[[]]]`
pub const NESTED_3_F32_EMPTY_2: [u8; 16] = [
    8, 0, // [0..2] topology start pointer
    4, 0, // [2..4] topology word count (2 levels, sentinels included)
    16, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
    255, 255, // [8..10] level 0: sentinel
    1, 0, // [10..12] level 0: boundary at 1 lists
    255, 255, // [12..14] level 1: sentinel
    0, 0, // [14..16] level 1: boundary at 0 leaves
];
pub const NESTED_3_F32_EMPTY_2_INODES: [Inode; 1] = nested_3_inode(WireKind::F32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_f32_empty_2() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::F32));
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::PrimitiveList(
        PrimitiveValueList::F32(vec![].into()),
    )])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_F32_EMPTY_2,
        &NESTED_3_F32_EMPTY_2_INODES,
    );
}

// ---------------------------------------------------------------------------
// F64: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_3_F64_EMPTY_0: [u8; 8] = [
    8, 0, // [0..2] topology start pointer
    0, 0, // [2..4] topology word count (0 levels, sentinels included)
    8, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
];
pub const NESTED_3_F64_EMPTY_0_INODES: [Inode; 1] = nested_3_inode(WireKind::F64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_f64_empty_0() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::F64));
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_F64_EMPTY_0,
        &NESTED_3_F64_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_3_F64_EMPTY_1: [u8; 16] = [
    8, 0, // [0..2] topology start pointer
    2, 0, // [2..4] topology word count (1 level, sentinels included)
    16, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
    255, 255, // [8..10] level 0: sentinel
    0, 0, // [10..12] level 0: boundary at 0 lists
    0, 0, 0, 0, // [12..16] alignment padding (4 bytes)
];
pub const NESTED_3_F64_EMPTY_1_INODES: [Inode; 1] = nested_3_inode(WireKind::F64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_f64_empty_1() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::F64));
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_F64_EMPTY_1,
        &NESTED_3_F64_EMPTY_1_INODES,
    );
}

/// `[[[]]]`
pub const NESTED_3_F64_EMPTY_2: [u8; 16] = [
    8, 0, // [0..2] topology start pointer
    4, 0, // [2..4] topology word count (2 levels, sentinels included)
    16, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
    255, 255, // [8..10] level 0: sentinel
    1, 0, // [10..12] level 0: boundary at 1 lists
    255, 255, // [12..14] level 1: sentinel
    0, 0, // [14..16] level 1: boundary at 0 leaves
];
pub const NESTED_3_F64_EMPTY_2_INODES: [Inode; 1] = nested_3_inode(WireKind::F64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_f64_empty_2() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::F64));
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::PrimitiveList(
        PrimitiveValueList::F64(vec![].into()),
    )])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_F64_EMPTY_2,
        &NESTED_3_F64_EMPTY_2_INODES,
    );
}

// ---------------------------------------------------------------------------
// Bool: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_3_BOOL_EMPTY_0: [u8; 8] = [
    8, 0, // [0..2] topology start pointer
    0, 0, // [2..4] topology word count (0 levels, sentinels included)
    8, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
];
pub const NESTED_3_BOOL_EMPTY_0_INODES: [Inode; 1] = nested_3_inode(WireKind::Bool);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_bool_empty_0() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::Bool));
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_BOOL_EMPTY_0,
        &NESTED_3_BOOL_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_3_BOOL_EMPTY_1: [u8; 12] = [
    8, 0, // [0..2] topology start pointer
    2, 0, // [2..4] topology word count (1 level, sentinels included)
    12, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
    255, 255, // [8..10] level 0: sentinel
    0, 0, // [10..12] level 0: boundary at 0 lists
];
pub const NESTED_3_BOOL_EMPTY_1_INODES: [Inode; 1] = nested_3_inode(WireKind::Bool);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_bool_empty_1() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::Bool));
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_BOOL_EMPTY_1,
        &NESTED_3_BOOL_EMPTY_1_INODES,
    );
}

/// `[[[]]]`
pub const NESTED_3_BOOL_EMPTY_2: [u8; 16] = [
    8, 0, // [0..2] topology start pointer
    4, 0, // [2..4] topology word count (2 levels, sentinels included)
    16, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
    255, 255, // [8..10] level 0: sentinel
    1, 0, // [10..12] level 0: boundary at 1 lists
    255, 255, // [12..14] level 1: sentinel
    0, 0, // [14..16] level 1: boundary at 0 leaves
];
pub const NESTED_3_BOOL_EMPTY_2_INODES: [Inode; 1] = nested_3_inode(WireKind::Bool);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_bool_empty_2() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::Bool));
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::from(
        Vec::<bool>::new(),
    )])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_BOOL_EMPTY_2,
        &NESTED_3_BOOL_EMPTY_2_INODES,
    );
}

// ---------------------------------------------------------------------------
// Timestamp: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_3_TIMESTAMP_EMPTY_0: [u8; 16] = [
    8, 0, // [0..2] topology start pointer
    0, 0, // [2..4] topology word count (0 levels, sentinels included)
    16, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
    0, 0, 0, 0, 0, 0, 0, 0, // [8..16] alignment padding (8 bytes)
];
pub const NESTED_3_TIMESTAMP_EMPTY_0_INODES: [Inode; 1] = nested_3_inode(WireKind::Timestamp);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_timestamp_empty_0() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::Timestamp));
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_TIMESTAMP_EMPTY_0,
        &NESTED_3_TIMESTAMP_EMPTY_0_INODES,
    );
}

/// `[[]]`
pub const NESTED_3_TIMESTAMP_EMPTY_1: [u8; 16] = [
    8, 0, // [0..2] topology start pointer
    2, 0, // [2..4] topology word count (1 level, sentinels included)
    16, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
    255, 255, // [8..10] level 0: sentinel
    0, 0, // [10..12] level 0: boundary at 0 lists
    0, 0, 0, 0, // [12..16] alignment padding to 16 bytes before payload
];
pub const NESTED_3_TIMESTAMP_EMPTY_1_INODES: [Inode; 1] = nested_3_inode(WireKind::Timestamp);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_timestamp_empty_1() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::Timestamp));
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_TIMESTAMP_EMPTY_1,
        &NESTED_3_TIMESTAMP_EMPTY_1_INODES,
    );
}

/// `[[[]]]`
pub const NESTED_3_TIMESTAMP_EMPTY_2: [u8; 16] = [
    8, 0, // [0..2] topology start pointer
    4, 0, // [2..4] topology word count (2 levels, sentinels included)
    16, 0, // [4..6] payload start pointer
    0, 0, // [6..8] leaf count
    255, 255, // [8..10] level 0: sentinel
    1, 0, // [10..12] level 0: boundary at 1 lists
    255, 255, // [12..14] level 1: sentinel
    0, 0, // [14..16] level 1: boundary at 0 leaves
];
pub const NESTED_3_TIMESTAMP_EMPTY_2_INODES: [Inode; 1] = nested_3_inode(WireKind::Timestamp);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_3_timestamp_empty_2() {
    let schema = nested_3_schema(primitive(PrimitiveDataType::Timestamp));
    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::PrimitiveList(
        PrimitiveValueList::Timestamp(vec![].into()),
    )])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &NESTED_3_TIMESTAMP_EMPTY_2,
        &NESTED_3_TIMESTAMP_EMPTY_2_INODES,
    );
}
