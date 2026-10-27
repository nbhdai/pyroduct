//! `List<T>` for every `PrimitiveDataType`.
//!
//! Shared value shape: `[a, b, c, d, e, f]`.
//!
//! Layout: `[payload start, leaf count]`, then (after alignment
//! padding) the payload. One list level needs no topology.

#[cfg(test)]
use half::f16;
#[cfg(test)]
use pyro_schema::PrimitiveDataType;
#[cfg(test)]
use pyro_schema::human::{PyroField, PyroSchema, PyroType};

#[cfg(test)]
use crate::{
    PyroValue, value::PrimitiveValueList, value::Time,
    wire::tests::support::run_list_wrapped_roundtrip_u32,
};

use pyro_schema::{Inode, KindMeta, WireKind};

#[cfg(test)]
fn nested_1_schema(leaf: PyroType) -> PyroSchema {
    let mut ty = leaf;
    for _ in 0..1 {
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

/// `[a, b, c, d, e, f]` from 1 leaf runs.
#[cfg(test)]
fn nested_1_value(leaves: [PyroValue<'static>; 1]) -> PyroValue<'static> {
    let [l0] = leaves;
    l0
}

/// `List<T>` at the root: one `Primitive` inode with `nesting: 1`,
/// its topology slot at 0 and its data slot at 0.
const fn nested_1_inode(kind: WireKind) -> [Inode; 1] {
    [Inode {
        kind,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 0,
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

pub const NESTED_1_U8: [u8; 14] = [
    8, 0, 0, 0, // payload start pointer
    6, 0, 0, 0, // leaf count
    1, // payload[0] = 1
    2, // payload[1] = 2
    3, // payload[2] = 3
    4, // payload[3] = 4
    5, // payload[4] = 5
    6, // payload[5] = 6
];
pub const NESTED_1_U8_INODES: [Inode; 1] = nested_1_inode(WireKind::U8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_u8() {
    let schema = nested_1_schema(primitive(PrimitiveDataType::U8));
    let data = nested_1_value([PyroValue::PrimitiveList(PrimitiveValueList::U8(
        vec![1, 2, 3, 4, 5, 6].into(),
    ))]);

    run_list_wrapped_roundtrip_u32(schema, data, &NESTED_1_U8, &NESTED_1_U8_INODES);
}

// ---------------------------------------------------------------------------
// I8
// ---------------------------------------------------------------------------

pub const NESTED_1_I8: [u8; 14] = [
    8, 0, 0, 0, // payload start pointer
    6, 0, 0, 0,   // leaf count
    255, // payload[0] = -1
    2,   // payload[1] = 2
    253, // payload[2] = -3
    4,   // payload[3] = 4
    251, // payload[4] = -5
    6,   // payload[5] = 6
];
pub const NESTED_1_I8_INODES: [Inode; 1] = nested_1_inode(WireKind::I8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_i8() {
    let schema = nested_1_schema(primitive(PrimitiveDataType::I8));
    let data = nested_1_value([PyroValue::PrimitiveList(PrimitiveValueList::I8(
        vec![-1, 2, -3, 4, -5, 6].into(),
    ))]);

    run_list_wrapped_roundtrip_u32(schema, data, &NESTED_1_I8, &NESTED_1_I8_INODES);
}

// ---------------------------------------------------------------------------
// U16
// ---------------------------------------------------------------------------

pub const NESTED_1_U16: [u8; 20] = [
    8, 0, 0, 0, // payload start pointer
    6, 0, 0, 0, // leaf count
    1, 0, // payload[0] = 1
    2, 0, // payload[1] = 2
    3, 0, // payload[2] = 3
    4, 0, // payload[3] = 4
    5, 0, // payload[4] = 5
    6, 0, // payload[5] = 6
];
pub const NESTED_1_U16_INODES: [Inode; 1] = nested_1_inode(WireKind::U16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_u16() {
    let schema = nested_1_schema(primitive(PrimitiveDataType::U16));
    let data = nested_1_value([PyroValue::PrimitiveList(PrimitiveValueList::U16(
        vec![1, 2, 3, 4, 5, 6].into(),
    ))]);

    run_list_wrapped_roundtrip_u32(schema, data, &NESTED_1_U16, &NESTED_1_U16_INODES);
}

// ---------------------------------------------------------------------------
// I16
// ---------------------------------------------------------------------------

pub const NESTED_1_I16: [u8; 20] = [
    8, 0, 0, 0, // payload start pointer
    6, 0, 0, 0, // leaf count
    255, 255, // payload[0] = -1
    2, 0, // payload[1] = 2
    253, 255, // payload[2] = -3
    4, 0, // payload[3] = 4
    251, 255, // payload[4] = -5
    6, 0, // payload[5] = 6
];
pub const NESTED_1_I16_INODES: [Inode; 1] = nested_1_inode(WireKind::I16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_i16() {
    let schema = nested_1_schema(primitive(PrimitiveDataType::I16));
    let data = nested_1_value([PyroValue::PrimitiveList(PrimitiveValueList::I16(
        vec![-1, 2, -3, 4, -5, 6].into(),
    ))]);

    run_list_wrapped_roundtrip_u32(schema, data, &NESTED_1_I16, &NESTED_1_I16_INODES);
}

// ---------------------------------------------------------------------------
// U32
// ---------------------------------------------------------------------------

pub const NESTED_1_U32: [u8; 32] = [
    8, 0, 0, 0, // payload start pointer
    6, 0, 0, 0, // leaf count
    1, 0, 0, 0, // payload[0] = 1
    2, 0, 0, 0, // payload[1] = 2
    3, 0, 0, 0, // payload[2] = 3
    4, 0, 0, 0, // payload[3] = 4
    5, 0, 0, 0, // payload[4] = 5
    6, 0, 0, 0, // payload[5] = 6
];
pub const NESTED_1_U32_INODES: [Inode; 1] = nested_1_inode(WireKind::U32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_u32() {
    let schema = nested_1_schema(primitive(PrimitiveDataType::U32));
    let data = nested_1_value([PyroValue::PrimitiveList(PrimitiveValueList::U32(
        vec![1, 2, 3, 4, 5, 6].into(),
    ))]);

    run_list_wrapped_roundtrip_u32(schema, data, &NESTED_1_U32, &NESTED_1_U32_INODES);
}

// ---------------------------------------------------------------------------
// I32
// ---------------------------------------------------------------------------

pub const NESTED_1_I32: [u8; 32] = [
    8, 0, 0, 0, // payload start pointer
    6, 0, 0, 0, // leaf count
    255, 255, 255, 255, // payload[0] = -1
    2, 0, 0, 0, // payload[1] = 2
    253, 255, 255, 255, // payload[2] = -3
    4, 0, 0, 0, // payload[3] = 4
    251, 255, 255, 255, // payload[4] = -5
    6, 0, 0, 0, // payload[5] = 6
];
pub const NESTED_1_I32_INODES: [Inode; 1] = nested_1_inode(WireKind::I32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_i32() {
    let schema = nested_1_schema(primitive(PrimitiveDataType::I32));
    let data = nested_1_value([PyroValue::PrimitiveList(PrimitiveValueList::I32(
        vec![-1, 2, -3, 4, -5, 6].into(),
    ))]);

    run_list_wrapped_roundtrip_u32(schema, data, &NESTED_1_I32, &NESTED_1_I32_INODES);
}

// ---------------------------------------------------------------------------
// U64
// ---------------------------------------------------------------------------

pub const NESTED_1_U64: [u8; 56] = [
    8, 0, 0, 0, // payload start pointer
    6, 0, 0, 0, // leaf count
    1, 0, 0, 0, 0, 0, 0, 0, // payload[0] = 1
    2, 0, 0, 0, 0, 0, 0, 0, // payload[1] = 2
    3, 0, 0, 0, 0, 0, 0, 0, // payload[2] = 3
    4, 0, 0, 0, 0, 0, 0, 0, // payload[3] = 4
    5, 0, 0, 0, 0, 0, 0, 0, // payload[4] = 5
    6, 0, 0, 0, 0, 0, 0, 0, // payload[5] = 6
];
pub const NESTED_1_U64_INODES: [Inode; 1] = nested_1_inode(WireKind::U64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_u64() {
    let schema = nested_1_schema(primitive(PrimitiveDataType::U64));
    let data = nested_1_value([PyroValue::PrimitiveList(PrimitiveValueList::U64(
        vec![1, 2, 3, 4, 5, 6].into(),
    ))]);

    run_list_wrapped_roundtrip_u32(schema, data, &NESTED_1_U64, &NESTED_1_U64_INODES);
}

// ---------------------------------------------------------------------------
// I64
// ---------------------------------------------------------------------------

pub const NESTED_1_I64: [u8; 56] = [
    8, 0, 0, 0, // payload start pointer
    6, 0, 0, 0, // leaf count
    255, 255, 255, 255, 255, 255, 255, 255, // payload[0] = -1
    2, 0, 0, 0, 0, 0, 0, 0, // payload[1] = 2
    253, 255, 255, 255, 255, 255, 255, 255, // payload[2] = -3
    4, 0, 0, 0, 0, 0, 0, 0, // payload[3] = 4
    251, 255, 255, 255, 255, 255, 255, 255, // payload[4] = -5
    6, 0, 0, 0, 0, 0, 0, 0, // payload[5] = 6
];
pub const NESTED_1_I64_INODES: [Inode; 1] = nested_1_inode(WireKind::I64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_i64() {
    let schema = nested_1_schema(primitive(PrimitiveDataType::I64));
    let data = nested_1_value([PyroValue::PrimitiveList(PrimitiveValueList::I64(
        vec![-1, 2, -3, 4, -5, 6].into(),
    ))]);

    run_list_wrapped_roundtrip_u32(schema, data, &NESTED_1_I64, &NESTED_1_I64_INODES);
}

// ---------------------------------------------------------------------------
// F16
// ---------------------------------------------------------------------------

pub const NESTED_1_F16: [u8; 20] = [
    8, 0, 0, 0, // payload start pointer
    6, 0, 0, 0, // leaf count
    0, 62, // payload[0] = 1.5
    0, 192, // payload[1] = -2.0
    128, 66, // payload[2] = 3.25
    0, 68, // payload[3] = 4.0
    128, 197, // payload[4] = -5.5
    0, 70, // payload[5] = 6.0
];
pub const NESTED_1_F16_INODES: [Inode; 1] = nested_1_inode(WireKind::F16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_f16() {
    let schema = nested_1_schema(primitive(PrimitiveDataType::F16));
    let data = nested_1_value([PyroValue::PrimitiveList(PrimitiveValueList::F16(
        vec![
            f16::from_f32(1.5),
            f16::from_f32(-2.0),
            f16::from_f32(3.25),
            f16::from_f32(4.0),
            f16::from_f32(-5.5),
            f16::from_f32(6.0),
        ]
        .into(),
    ))]);

    run_list_wrapped_roundtrip_u32(schema, data, &NESTED_1_F16, &NESTED_1_F16_INODES);
}

// ---------------------------------------------------------------------------
// F32
// ---------------------------------------------------------------------------

pub const NESTED_1_F32: [u8; 32] = [
    8, 0, 0, 0, // payload start pointer
    6, 0, 0, 0, // leaf count
    0, 0, 192, 63, // payload[0] = 1.5
    0, 0, 0, 192, // payload[1] = -2.0
    0, 0, 80, 64, // payload[2] = 3.25
    0, 0, 128, 64, // payload[3] = 4.0
    0, 0, 176, 192, // payload[4] = -5.5
    0, 0, 192, 64, // payload[5] = 6.0
];
pub const NESTED_1_F32_INODES: [Inode; 1] = nested_1_inode(WireKind::F32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_f32() {
    let schema = nested_1_schema(primitive(PrimitiveDataType::F32));
    let data = nested_1_value([PyroValue::PrimitiveList(PrimitiveValueList::F32(
        vec![1.5, -2.0, 3.25, 4.0, -5.5, 6.0].into(),
    ))]);

    run_list_wrapped_roundtrip_u32(schema, data, &NESTED_1_F32, &NESTED_1_F32_INODES);
}

// ---------------------------------------------------------------------------
// F64
// ---------------------------------------------------------------------------

pub const NESTED_1_F64: [u8; 56] = [
    8, 0, 0, 0, // payload start pointer
    6, 0, 0, 0, // leaf count
    0, 0, 0, 0, 0, 0, 248, 63, // payload[0] = 1.5
    0, 0, 0, 0, 0, 0, 0, 192, // payload[1] = -2.0
    0, 0, 0, 0, 0, 0, 10, 64, // payload[2] = 3.25
    0, 0, 0, 0, 0, 0, 16, 64, // payload[3] = 4.0
    0, 0, 0, 0, 0, 0, 22, 192, // payload[4] = -5.5
    0, 0, 0, 0, 0, 0, 24, 64, // payload[5] = 6.0
];
pub const NESTED_1_F64_INODES: [Inode; 1] = nested_1_inode(WireKind::F64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_f64() {
    let schema = nested_1_schema(primitive(PrimitiveDataType::F64));
    let data = nested_1_value([PyroValue::PrimitiveList(PrimitiveValueList::F64(
        vec![1.5, -2.0, 3.25, 4.0, -5.5, 6.0].into(),
    ))]);

    run_list_wrapped_roundtrip_u32(schema, data, &NESTED_1_F64, &NESTED_1_F64_INODES);
}

// ---------------------------------------------------------------------------
// Bool
// ---------------------------------------------------------------------------

pub const NESTED_1_BOOL: [u8; 9] = [
    8, 0, 0, 0, // payload start pointer
    6, 0, 0, 0,  // leaf count
    29, // payload bits 0..6 (LSB first) = 1, 0, 1, 1, 1, 0
];
pub const NESTED_1_BOOL_INODES: [Inode; 1] = nested_1_inode(WireKind::Bool);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_bool() {
    let schema = nested_1_schema(primitive(PrimitiveDataType::Bool));
    let data = nested_1_value([PyroValue::from(vec![true, false, true, true, true, false])]);

    run_list_wrapped_roundtrip_u32(schema, data, &NESTED_1_BOOL, &NESTED_1_BOOL_INODES);
}

// ---------------------------------------------------------------------------
// Timestamp
// ---------------------------------------------------------------------------

pub const NESTED_1_TIMESTAMP: [u8; 112] = [
    16, 0, 0, 0, // payload start pointer
    6, 0, 0, 0, // leaf count
    0, 0, 0, 0, 0, 0, 0, 0, // alignment padding to 16 bytes before payload
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
pub const NESTED_1_TIMESTAMP_INODES: [Inode; 1] = nested_1_inode(WireKind::Timestamp);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_timestamp() {
    let schema = nested_1_schema(primitive(PrimitiveDataType::Timestamp));
    let data = nested_1_value([PyroValue::from(vec![
        Time(1_000_001_000),
        Time(-2_000_002_000),
        Time(3_500_003_000),
        Time(4_000_004_000),
        Time(-5_000_005_000),
        Time(1_700_000_000_000_006_000),
    ])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_TIMESTAMP,
        &NESTED_1_TIMESTAMP_INODES,
    );
}

// ===========================================================================
// Empty lists at every depth
// ===========================================================================
//
// `_EMPTY_d` is the list at depth `d` left empty, every shallower list
// holding exactly one element: `_EMPTY_0` = `[]`.
// No leaf is written, so each buffer is the fixed header, whatever
// topology levels still have entries (a level with no lists below it is
// omitted, sentinel included), then the padding up to the payload's
// alignment. Byte ranges are `[start..end)` offsets into the buffer.
// Captured from the value writer's output.

// ---------------------------------------------------------------------------
// U8: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_1_U8_EMPTY_0: [u8; 8] = [
    8, 0, 0, 0, // [0..4] payload start pointer
    0, 0, 0, 0, // [4..8] leaf count
];
pub const NESTED_1_U8_EMPTY_0_INODES: [Inode; 1] = nested_1_inode(WireKind::U8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_u8_empty_0() {
    let schema = nested_1_schema(primitive(PrimitiveDataType::U8));
    let data = PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![].into()));

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_U8_EMPTY_0,
        &NESTED_1_U8_EMPTY_0_INODES,
    );
}

// ---------------------------------------------------------------------------
// I8: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_1_I8_EMPTY_0: [u8; 8] = [
    8, 0, 0, 0, // [0..4] payload start pointer
    0, 0, 0, 0, // [4..8] leaf count
];
pub const NESTED_1_I8_EMPTY_0_INODES: [Inode; 1] = nested_1_inode(WireKind::I8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_i8_empty_0() {
    let schema = nested_1_schema(primitive(PrimitiveDataType::I8));
    let data = PyroValue::PrimitiveList(PrimitiveValueList::I8(vec![].into()));

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_I8_EMPTY_0,
        &NESTED_1_I8_EMPTY_0_INODES,
    );
}

// ---------------------------------------------------------------------------
// U16: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_1_U16_EMPTY_0: [u8; 8] = [
    8, 0, 0, 0, // [0..4] payload start pointer
    0, 0, 0, 0, // [4..8] leaf count
];
pub const NESTED_1_U16_EMPTY_0_INODES: [Inode; 1] = nested_1_inode(WireKind::U16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_u16_empty_0() {
    let schema = nested_1_schema(primitive(PrimitiveDataType::U16));
    let data = PyroValue::PrimitiveList(PrimitiveValueList::U16(vec![].into()));

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_U16_EMPTY_0,
        &NESTED_1_U16_EMPTY_0_INODES,
    );
}

// ---------------------------------------------------------------------------
// I16: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_1_I16_EMPTY_0: [u8; 8] = [
    8, 0, 0, 0, // [0..4] payload start pointer
    0, 0, 0, 0, // [4..8] leaf count
];
pub const NESTED_1_I16_EMPTY_0_INODES: [Inode; 1] = nested_1_inode(WireKind::I16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_i16_empty_0() {
    let schema = nested_1_schema(primitive(PrimitiveDataType::I16));
    let data = PyroValue::PrimitiveList(PrimitiveValueList::I16(vec![].into()));

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_I16_EMPTY_0,
        &NESTED_1_I16_EMPTY_0_INODES,
    );
}

// ---------------------------------------------------------------------------
// U32: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_1_U32_EMPTY_0: [u8; 8] = [
    8, 0, 0, 0, // [0..4] payload start pointer
    0, 0, 0, 0, // [4..8] leaf count
];
pub const NESTED_1_U32_EMPTY_0_INODES: [Inode; 1] = nested_1_inode(WireKind::U32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_u32_empty_0() {
    let schema = nested_1_schema(primitive(PrimitiveDataType::U32));
    let data = PyroValue::PrimitiveList(PrimitiveValueList::U32(vec![].into()));

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_U32_EMPTY_0,
        &NESTED_1_U32_EMPTY_0_INODES,
    );
}

// ---------------------------------------------------------------------------
// I32: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_1_I32_EMPTY_0: [u8; 8] = [
    8, 0, 0, 0, // [0..4] payload start pointer
    0, 0, 0, 0, // [4..8] leaf count
];
pub const NESTED_1_I32_EMPTY_0_INODES: [Inode; 1] = nested_1_inode(WireKind::I32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_i32_empty_0() {
    let schema = nested_1_schema(primitive(PrimitiveDataType::I32));
    let data = PyroValue::PrimitiveList(PrimitiveValueList::I32(vec![].into()));

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_I32_EMPTY_0,
        &NESTED_1_I32_EMPTY_0_INODES,
    );
}

// ---------------------------------------------------------------------------
// U64: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_1_U64_EMPTY_0: [u8; 8] = [
    8, 0, 0, 0, // [0..4] payload start pointer
    0, 0, 0, 0, // [4..8] leaf count
];
pub const NESTED_1_U64_EMPTY_0_INODES: [Inode; 1] = nested_1_inode(WireKind::U64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_u64_empty_0() {
    let schema = nested_1_schema(primitive(PrimitiveDataType::U64));
    let data = PyroValue::PrimitiveList(PrimitiveValueList::U64(vec![].into()));

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_U64_EMPTY_0,
        &NESTED_1_U64_EMPTY_0_INODES,
    );
}

// ---------------------------------------------------------------------------
// I64: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_1_I64_EMPTY_0: [u8; 8] = [
    8, 0, 0, 0, // [0..4] payload start pointer
    0, 0, 0, 0, // [4..8] leaf count
];
pub const NESTED_1_I64_EMPTY_0_INODES: [Inode; 1] = nested_1_inode(WireKind::I64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_i64_empty_0() {
    let schema = nested_1_schema(primitive(PrimitiveDataType::I64));
    let data = PyroValue::PrimitiveList(PrimitiveValueList::I64(vec![].into()));

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_I64_EMPTY_0,
        &NESTED_1_I64_EMPTY_0_INODES,
    );
}

// ---------------------------------------------------------------------------
// F16: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_1_F16_EMPTY_0: [u8; 8] = [
    8, 0, 0, 0, // [0..4] payload start pointer
    0, 0, 0, 0, // [4..8] leaf count
];
pub const NESTED_1_F16_EMPTY_0_INODES: [Inode; 1] = nested_1_inode(WireKind::F16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_f16_empty_0() {
    let schema = nested_1_schema(primitive(PrimitiveDataType::F16));
    let data = PyroValue::PrimitiveList(PrimitiveValueList::F16(vec![].into()));

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_F16_EMPTY_0,
        &NESTED_1_F16_EMPTY_0_INODES,
    );
}

// ---------------------------------------------------------------------------
// F32: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_1_F32_EMPTY_0: [u8; 8] = [
    8, 0, 0, 0, // [0..4] payload start pointer
    0, 0, 0, 0, // [4..8] leaf count
];
pub const NESTED_1_F32_EMPTY_0_INODES: [Inode; 1] = nested_1_inode(WireKind::F32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_f32_empty_0() {
    let schema = nested_1_schema(primitive(PrimitiveDataType::F32));
    let data = PyroValue::PrimitiveList(PrimitiveValueList::F32(vec![].into()));

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_F32_EMPTY_0,
        &NESTED_1_F32_EMPTY_0_INODES,
    );
}

// ---------------------------------------------------------------------------
// F64: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_1_F64_EMPTY_0: [u8; 8] = [
    8, 0, 0, 0, // [0..4] payload start pointer
    0, 0, 0, 0, // [4..8] leaf count
];
pub const NESTED_1_F64_EMPTY_0_INODES: [Inode; 1] = nested_1_inode(WireKind::F64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_f64_empty_0() {
    let schema = nested_1_schema(primitive(PrimitiveDataType::F64));
    let data = PyroValue::PrimitiveList(PrimitiveValueList::F64(vec![].into()));

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_F64_EMPTY_0,
        &NESTED_1_F64_EMPTY_0_INODES,
    );
}

// ---------------------------------------------------------------------------
// Bool: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_1_BOOL_EMPTY_0: [u8; 8] = [
    8, 0, 0, 0, // [0..4] payload start pointer
    0, 0, 0, 0, // [4..8] leaf count
];
pub const NESTED_1_BOOL_EMPTY_0_INODES: [Inode; 1] = nested_1_inode(WireKind::Bool);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_bool_empty_0() {
    let schema = nested_1_schema(primitive(PrimitiveDataType::Bool));
    let data = PyroValue::from(Vec::<bool>::new());

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_BOOL_EMPTY_0,
        &NESTED_1_BOOL_EMPTY_0_INODES,
    );
}

// ---------------------------------------------------------------------------
// Timestamp: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_1_TIMESTAMP_EMPTY_0: [u8; 16] = [
    16, 0, 0, 0, // [0..4] payload start pointer
    0, 0, 0, 0, // [4..8] leaf count
    0, 0, 0, 0, 0, 0, 0, 0, // [8..16] alignment padding to 16 bytes before payload
];
pub const NESTED_1_TIMESTAMP_EMPTY_0_INODES: [Inode; 1] = nested_1_inode(WireKind::Timestamp);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_timestamp_empty_0() {
    let schema = nested_1_schema(primitive(PrimitiveDataType::Timestamp));
    let data = PyroValue::PrimitiveList(PrimitiveValueList::Timestamp(vec![].into()));

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_TIMESTAMP_EMPTY_0,
        &NESTED_1_TIMESTAMP_EMPTY_0_INODES,
    );
}
