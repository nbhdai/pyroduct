//! `List<Option<T>>` for every `PrimitiveDataType`. The
//! same list shapes as `primitive_1_nesting.rs`, with samples 1 and 4
//! (of 6) null.
//!
//! Shared value shape: `[a, b, c, d, e, f]`, `b` and `e` null.
//!
//! Layout: `[payload start, valid samples, total samples]`, then (after
//! alignment padding) the valid payload, then the validity mask (one bit
//! per sample, LSB first, zero-padded up to the element alignment). One
//! list level needs no topology.

#[cfg(test)]
use half::f16;
#[cfg(test)]
use pyro_schema::PrimitiveDataType;
#[cfg(test)]
use pyro_schema::human::{PyroField, PyroSchema, PyroType};

#[cfg(test)]
use crate::{PyroValue, value::Time, wire::tests::support::run_list_wrapped_roundtrip_u32};

use pyro_schema::{Inode, KindMeta, WireKind};

#[cfg(test)]
fn nested_1_option_schema(leaf: PyroType) -> PyroSchema {
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
fn option_primitive(primitive: PrimitiveDataType) -> PyroType {
    PyroType::OptionPrimitiveScalar { primitive }
}

/// `[a, b, c, d, e, f]` from 1 leaf run.
#[cfg(test)]
fn nested_1_option_value(leaves: [PyroValue<'static>; 1]) -> PyroValue<'static> {
    let [l0] = leaves;
    l0
}

/// `List<Option<T>>` at the root: one `Primitive` inode with `nesting: 1`,
/// its topology slot at 0 and its data slot at 0.
const fn nested_1_option_inode(kind: WireKind) -> [Inode; 1] {
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
// OptionU8
// ---------------------------------------------------------------------------

pub const NESTED_1_OPTION_U8: [u8; 17] = [
    12, 0, 0, 0, // payload start pointer
    4, 0, 0, 0, // valid samples
    6, 0, 0, 0,  // total samples (nulls included)
    1,  // valid payload[0] = 1
    3,  // valid payload[1] = 3
    4,  // valid payload[2] = 4
    6,  // valid payload[3] = 6
    45, // validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
];
pub const NESTED_1_OPTION_U8_INODES: [Inode; 1] = nested_1_option_inode(WireKind::OptionU8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_option_u8() {
    let schema = nested_1_option_schema(option_primitive(PrimitiveDataType::U8));
    let data = nested_1_option_value([PyroValue::List(vec![
        PyroValue::U8(1),
        PyroValue::Null,
        PyroValue::U8(3),
        PyroValue::U8(4),
        PyroValue::Null,
        PyroValue::U8(6),
    ])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_OPTION_U8,
        &NESTED_1_OPTION_U8_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionI8
// ---------------------------------------------------------------------------

pub const NESTED_1_OPTION_I8: [u8; 17] = [
    12, 0, 0, 0, // payload start pointer
    4, 0, 0, 0, // valid samples
    6, 0, 0, 0,   // total samples (nulls included)
    255, // valid payload[0] = -1
    253, // valid payload[1] = -3
    4,   // valid payload[2] = 4
    6,   // valid payload[3] = 6
    45,  // validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
];
pub const NESTED_1_OPTION_I8_INODES: [Inode; 1] = nested_1_option_inode(WireKind::OptionI8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_option_i8() {
    let schema = nested_1_option_schema(option_primitive(PrimitiveDataType::I8));
    let data = nested_1_option_value([PyroValue::List(vec![
        PyroValue::I8(-1),
        PyroValue::Null,
        PyroValue::I8(-3),
        PyroValue::I8(4),
        PyroValue::Null,
        PyroValue::I8(6),
    ])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_OPTION_I8,
        &NESTED_1_OPTION_I8_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionU16
// ---------------------------------------------------------------------------

pub const NESTED_1_OPTION_U16: [u8; 21] = [
    12, 0, 0, 0, // payload start pointer
    4, 0, 0, 0, // valid samples
    6, 0, 0, 0, // total samples (nulls included)
    1, 0, // valid payload[0] = 1
    3, 0, // valid payload[1] = 3
    4, 0, // valid payload[2] = 4
    6, 0,  // valid payload[3] = 6
    45, // validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
];
pub const NESTED_1_OPTION_U16_INODES: [Inode; 1] = nested_1_option_inode(WireKind::OptionU16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_option_u16() {
    let schema = nested_1_option_schema(option_primitive(PrimitiveDataType::U16));
    let data = nested_1_option_value([PyroValue::List(vec![
        PyroValue::U16(1),
        PyroValue::Null,
        PyroValue::U16(3),
        PyroValue::U16(4),
        PyroValue::Null,
        PyroValue::U16(6),
    ])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_OPTION_U16,
        &NESTED_1_OPTION_U16_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionI16
// ---------------------------------------------------------------------------

pub const NESTED_1_OPTION_I16: [u8; 21] = [
    12, 0, 0, 0, // payload start pointer
    4, 0, 0, 0, // valid samples
    6, 0, 0, 0, // total samples (nulls included)
    255, 255, // valid payload[0] = -1
    253, 255, // valid payload[1] = -3
    4, 0, // valid payload[2] = 4
    6, 0,  // valid payload[3] = 6
    45, // validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
];
pub const NESTED_1_OPTION_I16_INODES: [Inode; 1] = nested_1_option_inode(WireKind::OptionI16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_option_i16() {
    let schema = nested_1_option_schema(option_primitive(PrimitiveDataType::I16));
    let data = nested_1_option_value([PyroValue::List(vec![
        PyroValue::I16(-1),
        PyroValue::Null,
        PyroValue::I16(-3),
        PyroValue::I16(4),
        PyroValue::Null,
        PyroValue::I16(6),
    ])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_OPTION_I16,
        &NESTED_1_OPTION_I16_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionU32
// ---------------------------------------------------------------------------

pub const NESTED_1_OPTION_U32: [u8; 29] = [
    12, 0, 0, 0, // payload start pointer
    4, 0, 0, 0, // valid samples
    6, 0, 0, 0, // total samples (nulls included)
    1, 0, 0, 0, // valid payload[0] = 1
    3, 0, 0, 0, // valid payload[1] = 3
    4, 0, 0, 0, // valid payload[2] = 4
    6, 0, 0, 0,  // valid payload[3] = 6
    45, // validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
];
pub const NESTED_1_OPTION_U32_INODES: [Inode; 1] = nested_1_option_inode(WireKind::OptionU32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_option_u32() {
    let schema = nested_1_option_schema(option_primitive(PrimitiveDataType::U32));
    let data = nested_1_option_value([PyroValue::List(vec![
        PyroValue::U32(1),
        PyroValue::Null,
        PyroValue::U32(3),
        PyroValue::U32(4),
        PyroValue::Null,
        PyroValue::U32(6),
    ])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_OPTION_U32,
        &NESTED_1_OPTION_U32_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionI32
// ---------------------------------------------------------------------------

pub const NESTED_1_OPTION_I32: [u8; 29] = [
    12, 0, 0, 0, // payload start pointer
    4, 0, 0, 0, // valid samples
    6, 0, 0, 0, // total samples (nulls included)
    255, 255, 255, 255, // valid payload[0] = -1
    253, 255, 255, 255, // valid payload[1] = -3
    4, 0, 0, 0, // valid payload[2] = 4
    6, 0, 0, 0,  // valid payload[3] = 6
    45, // validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
];
pub const NESTED_1_OPTION_I32_INODES: [Inode; 1] = nested_1_option_inode(WireKind::OptionI32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_option_i32() {
    let schema = nested_1_option_schema(option_primitive(PrimitiveDataType::I32));
    let data = nested_1_option_value([PyroValue::List(vec![
        PyroValue::I32(-1),
        PyroValue::Null,
        PyroValue::I32(-3),
        PyroValue::I32(4),
        PyroValue::Null,
        PyroValue::I32(6),
    ])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_OPTION_I32,
        &NESTED_1_OPTION_I32_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionU64
// ---------------------------------------------------------------------------

pub const NESTED_1_OPTION_U64: [u8; 49] = [
    16, 0, 0, 0, // payload start pointer
    4, 0, 0, 0, // valid samples
    6, 0, 0, 0, // total samples (nulls included)
    0, 0, 0, 0, // alignment padding to 8 bytes before payload
    1, 0, 0, 0, 0, 0, 0, 0, // valid payload[0] = 1
    3, 0, 0, 0, 0, 0, 0, 0, // valid payload[1] = 3
    4, 0, 0, 0, 0, 0, 0, 0, // valid payload[2] = 4
    6, 0, 0, 0, 0, 0, 0, 0,  // valid payload[3] = 6
    45, // validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
];
pub const NESTED_1_OPTION_U64_INODES: [Inode; 1] = nested_1_option_inode(WireKind::OptionU64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_option_u64() {
    let schema = nested_1_option_schema(option_primitive(PrimitiveDataType::U64));
    let data = nested_1_option_value([PyroValue::List(vec![
        PyroValue::U64(1),
        PyroValue::Null,
        PyroValue::U64(3),
        PyroValue::U64(4),
        PyroValue::Null,
        PyroValue::U64(6),
    ])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_OPTION_U64,
        &NESTED_1_OPTION_U64_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionI64
// ---------------------------------------------------------------------------

pub const NESTED_1_OPTION_I64: [u8; 49] = [
    16, 0, 0, 0, // payload start pointer
    4, 0, 0, 0, // valid samples
    6, 0, 0, 0, // total samples (nulls included)
    0, 0, 0, 0, // alignment padding to 8 bytes before payload
    255, 255, 255, 255, 255, 255, 255, 255, // valid payload[0] = -1
    253, 255, 255, 255, 255, 255, 255, 255, // valid payload[1] = -3
    4, 0, 0, 0, 0, 0, 0, 0, // valid payload[2] = 4
    6, 0, 0, 0, 0, 0, 0, 0,  // valid payload[3] = 6
    45, // validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
];
pub const NESTED_1_OPTION_I64_INODES: [Inode; 1] = nested_1_option_inode(WireKind::OptionI64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_option_i64() {
    let schema = nested_1_option_schema(option_primitive(PrimitiveDataType::I64));
    let data = nested_1_option_value([PyroValue::List(vec![
        PyroValue::I64(-1),
        PyroValue::Null,
        PyroValue::I64(-3),
        PyroValue::I64(4),
        PyroValue::Null,
        PyroValue::I64(6),
    ])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_OPTION_I64,
        &NESTED_1_OPTION_I64_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionF16
// ---------------------------------------------------------------------------

pub const NESTED_1_OPTION_F16: [u8; 21] = [
    12, 0, 0, 0, // payload start pointer
    4, 0, 0, 0, // valid samples
    6, 0, 0, 0, // total samples (nulls included)
    0, 62, // valid payload[0] = 1.5
    128, 66, // valid payload[1] = 3.25
    0, 68, // valid payload[2] = 4.0
    0, 70, // valid payload[3] = 6.0
    45, // validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
];
pub const NESTED_1_OPTION_F16_INODES: [Inode; 1] = nested_1_option_inode(WireKind::OptionF16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_option_f16() {
    let schema = nested_1_option_schema(option_primitive(PrimitiveDataType::F16));
    let data = nested_1_option_value([PyroValue::List(vec![
        PyroValue::F16(f16::from_f32(1.5)),
        PyroValue::Null,
        PyroValue::F16(f16::from_f32(3.25)),
        PyroValue::F16(f16::from_f32(4.0)),
        PyroValue::Null,
        PyroValue::F16(f16::from_f32(6.0)),
    ])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_OPTION_F16,
        &NESTED_1_OPTION_F16_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionF32
// ---------------------------------------------------------------------------

pub const NESTED_1_OPTION_F32: [u8; 29] = [
    12, 0, 0, 0, // payload start pointer
    4, 0, 0, 0, // valid samples
    6, 0, 0, 0, // total samples (nulls included)
    0, 0, 192, 63, // valid payload[0] = 1.5
    0, 0, 80, 64, // valid payload[1] = 3.25
    0, 0, 128, 64, // valid payload[2] = 4.0
    0, 0, 192, 64, // valid payload[3] = 6.0
    45, // validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
];
pub const NESTED_1_OPTION_F32_INODES: [Inode; 1] = nested_1_option_inode(WireKind::OptionF32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_option_f32() {
    let schema = nested_1_option_schema(option_primitive(PrimitiveDataType::F32));
    let data = nested_1_option_value([PyroValue::List(vec![
        PyroValue::F32(1.5),
        PyroValue::Null,
        PyroValue::F32(3.25),
        PyroValue::F32(4.0),
        PyroValue::Null,
        PyroValue::F32(6.0),
    ])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_OPTION_F32,
        &NESTED_1_OPTION_F32_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionF64
// ---------------------------------------------------------------------------

pub const NESTED_1_OPTION_F64: [u8; 49] = [
    16, 0, 0, 0, // payload start pointer
    4, 0, 0, 0, // valid samples
    6, 0, 0, 0, // total samples (nulls included)
    0, 0, 0, 0, // alignment padding to 8 bytes before payload
    0, 0, 0, 0, 0, 0, 248, 63, // valid payload[0] = 1.5
    0, 0, 0, 0, 0, 0, 10, 64, // valid payload[1] = 3.25
    0, 0, 0, 0, 0, 0, 16, 64, // valid payload[2] = 4.0
    0, 0, 0, 0, 0, 0, 24, 64, // valid payload[3] = 6.0
    45, // validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
];
pub const NESTED_1_OPTION_F64_INODES: [Inode; 1] = nested_1_option_inode(WireKind::OptionF64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_option_f64() {
    let schema = nested_1_option_schema(option_primitive(PrimitiveDataType::F64));
    let data = nested_1_option_value([PyroValue::List(vec![
        PyroValue::F64(1.5),
        PyroValue::Null,
        PyroValue::F64(3.25),
        PyroValue::F64(4.0),
        PyroValue::Null,
        PyroValue::F64(6.0),
    ])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_OPTION_F64,
        &NESTED_1_OPTION_F64_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionBool
// ---------------------------------------------------------------------------

pub const NESTED_1_OPTION_BOOL: [u8; 14] = [
    12, 0, 0, 0, // payload start pointer
    4, 0, 0, 0, // valid samples
    6, 0, 0, 0,  // total samples (nulls included)
    7,  // valid payload bits 0..4 (LSB first) = 1, 1, 1, 0
    45, // validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
];
pub const NESTED_1_OPTION_BOOL_INODES: [Inode; 1] = nested_1_option_inode(WireKind::OptionBool);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_option_bool() {
    let schema = nested_1_option_schema(option_primitive(PrimitiveDataType::Bool));
    let data = nested_1_option_value([PyroValue::List(vec![
        PyroValue::Bool(true),
        PyroValue::Null,
        PyroValue::Bool(true),
        PyroValue::Bool(true),
        PyroValue::Null,
        PyroValue::Bool(false),
    ])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_OPTION_BOOL,
        &NESTED_1_OPTION_BOOL_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionTimestamp
// ---------------------------------------------------------------------------

pub const NESTED_1_OPTION_TIMESTAMP: [u8; 81] = [
    16, 0, 0, 0, // payload start pointer
    4, 0, 0, 0, // valid samples
    6, 0, 0, 0, // total samples (nulls included)
    0, 0, 0, 0, // alignment padding to 16 bytes before payload
    232, 205, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, // valid payload[0] = Time(1000001000)
    184, 206, 157, 208, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, // valid payload[1] = Time(3500003000)
    160, 55, 107, 238, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, // valid payload[2] = Time(4000004000)
    112, 23, 42, 54, 254, 156, 151, 23, 0, 0, 0, 0, 0, 0, 0,
    0,  // valid payload[3] = Time(1700000000000006000)
    45, // validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
];
pub const NESTED_1_OPTION_TIMESTAMP_INODES: [Inode; 1] =
    nested_1_option_inode(WireKind::OptionTimestamp);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_option_timestamp() {
    let schema = nested_1_option_schema(PyroType::OptionPrimitiveScalar {
        primitive: PrimitiveDataType::Timestamp,
    });
    let data = nested_1_option_value([PyroValue::List(vec![
        PyroValue::from(Time(1_000_001_000)),
        PyroValue::Null,
        PyroValue::from(Time(3_500_003_000)),
        PyroValue::from(Time(4_000_004_000)),
        PyroValue::Null,
        PyroValue::from(Time(1_700_000_000_000_006_000)),
    ])]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_OPTION_TIMESTAMP,
        &NESTED_1_OPTION_TIMESTAMP_INODES,
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
// OptionU8: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_1_OPTION_U8_EMPTY_0: [u8; 12] = [
    12, 0, 0, 0, // [0..4] payload start pointer
    0, 0, 0, 0, // [4..8] valid samples
    0, 0, 0, 0, // [8..12] total samples (nulls included)
];
pub const NESTED_1_OPTION_U8_EMPTY_0_INODES: [Inode; 1] = nested_1_option_inode(WireKind::OptionU8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_option_u8_empty_0() {
    let schema = nested_1_option_schema(option_primitive(PrimitiveDataType::U8));
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_OPTION_U8_EMPTY_0,
        &NESTED_1_OPTION_U8_EMPTY_0_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionI8: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_1_OPTION_I8_EMPTY_0: [u8; 12] = [
    12, 0, 0, 0, // [0..4] payload start pointer
    0, 0, 0, 0, // [4..8] valid samples
    0, 0, 0, 0, // [8..12] total samples (nulls included)
];
pub const NESTED_1_OPTION_I8_EMPTY_0_INODES: [Inode; 1] = nested_1_option_inode(WireKind::OptionI8);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_option_i8_empty_0() {
    let schema = nested_1_option_schema(option_primitive(PrimitiveDataType::I8));
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_OPTION_I8_EMPTY_0,
        &NESTED_1_OPTION_I8_EMPTY_0_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionU16: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_1_OPTION_U16_EMPTY_0: [u8; 12] = [
    12, 0, 0, 0, // [0..4] payload start pointer
    0, 0, 0, 0, // [4..8] valid samples
    0, 0, 0, 0, // [8..12] total samples (nulls included)
];
pub const NESTED_1_OPTION_U16_EMPTY_0_INODES: [Inode; 1] =
    nested_1_option_inode(WireKind::OptionU16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_option_u16_empty_0() {
    let schema = nested_1_option_schema(option_primitive(PrimitiveDataType::U16));
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_OPTION_U16_EMPTY_0,
        &NESTED_1_OPTION_U16_EMPTY_0_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionI16: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_1_OPTION_I16_EMPTY_0: [u8; 12] = [
    12, 0, 0, 0, // [0..4] payload start pointer
    0, 0, 0, 0, // [4..8] valid samples
    0, 0, 0, 0, // [8..12] total samples (nulls included)
];
pub const NESTED_1_OPTION_I16_EMPTY_0_INODES: [Inode; 1] =
    nested_1_option_inode(WireKind::OptionI16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_option_i16_empty_0() {
    let schema = nested_1_option_schema(option_primitive(PrimitiveDataType::I16));
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_OPTION_I16_EMPTY_0,
        &NESTED_1_OPTION_I16_EMPTY_0_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionU32: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_1_OPTION_U32_EMPTY_0: [u8; 12] = [
    12, 0, 0, 0, // [0..4] payload start pointer
    0, 0, 0, 0, // [4..8] valid samples
    0, 0, 0, 0, // [8..12] total samples (nulls included)
];
pub const NESTED_1_OPTION_U32_EMPTY_0_INODES: [Inode; 1] =
    nested_1_option_inode(WireKind::OptionU32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_option_u32_empty_0() {
    let schema = nested_1_option_schema(option_primitive(PrimitiveDataType::U32));
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_OPTION_U32_EMPTY_0,
        &NESTED_1_OPTION_U32_EMPTY_0_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionI32: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_1_OPTION_I32_EMPTY_0: [u8; 12] = [
    12, 0, 0, 0, // [0..4] payload start pointer
    0, 0, 0, 0, // [4..8] valid samples
    0, 0, 0, 0, // [8..12] total samples (nulls included)
];
pub const NESTED_1_OPTION_I32_EMPTY_0_INODES: [Inode; 1] =
    nested_1_option_inode(WireKind::OptionI32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_option_i32_empty_0() {
    let schema = nested_1_option_schema(option_primitive(PrimitiveDataType::I32));
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_OPTION_I32_EMPTY_0,
        &NESTED_1_OPTION_I32_EMPTY_0_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionU64: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_1_OPTION_U64_EMPTY_0: [u8; 16] = [
    16, 0, 0, 0, // [0..4] payload start pointer
    0, 0, 0, 0, // [4..8] valid samples
    0, 0, 0, 0, // [8..12] total samples (nulls included)
    0, 0, 0, 0, // [12..16] alignment padding to 8 bytes before payload
];
pub const NESTED_1_OPTION_U64_EMPTY_0_INODES: [Inode; 1] =
    nested_1_option_inode(WireKind::OptionU64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_option_u64_empty_0() {
    let schema = nested_1_option_schema(option_primitive(PrimitiveDataType::U64));
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_OPTION_U64_EMPTY_0,
        &NESTED_1_OPTION_U64_EMPTY_0_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionI64: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_1_OPTION_I64_EMPTY_0: [u8; 16] = [
    16, 0, 0, 0, // [0..4] payload start pointer
    0, 0, 0, 0, // [4..8] valid samples
    0, 0, 0, 0, // [8..12] total samples (nulls included)
    0, 0, 0, 0, // [12..16] alignment padding to 8 bytes before payload
];
pub const NESTED_1_OPTION_I64_EMPTY_0_INODES: [Inode; 1] =
    nested_1_option_inode(WireKind::OptionI64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_option_i64_empty_0() {
    let schema = nested_1_option_schema(option_primitive(PrimitiveDataType::I64));
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_OPTION_I64_EMPTY_0,
        &NESTED_1_OPTION_I64_EMPTY_0_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionF16: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_1_OPTION_F16_EMPTY_0: [u8; 12] = [
    12, 0, 0, 0, // [0..4] payload start pointer
    0, 0, 0, 0, // [4..8] valid samples
    0, 0, 0, 0, // [8..12] total samples (nulls included)
];
pub const NESTED_1_OPTION_F16_EMPTY_0_INODES: [Inode; 1] =
    nested_1_option_inode(WireKind::OptionF16);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_option_f16_empty_0() {
    let schema = nested_1_option_schema(option_primitive(PrimitiveDataType::F16));
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_OPTION_F16_EMPTY_0,
        &NESTED_1_OPTION_F16_EMPTY_0_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionF32: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_1_OPTION_F32_EMPTY_0: [u8; 12] = [
    12, 0, 0, 0, // [0..4] payload start pointer
    0, 0, 0, 0, // [4..8] valid samples
    0, 0, 0, 0, // [8..12] total samples (nulls included)
];
pub const NESTED_1_OPTION_F32_EMPTY_0_INODES: [Inode; 1] =
    nested_1_option_inode(WireKind::OptionF32);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_option_f32_empty_0() {
    let schema = nested_1_option_schema(option_primitive(PrimitiveDataType::F32));
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_OPTION_F32_EMPTY_0,
        &NESTED_1_OPTION_F32_EMPTY_0_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionF64: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_1_OPTION_F64_EMPTY_0: [u8; 16] = [
    16, 0, 0, 0, // [0..4] payload start pointer
    0, 0, 0, 0, // [4..8] valid samples
    0, 0, 0, 0, // [8..12] total samples (nulls included)
    0, 0, 0, 0, // [12..16] alignment padding to 8 bytes before payload
];
pub const NESTED_1_OPTION_F64_EMPTY_0_INODES: [Inode; 1] =
    nested_1_option_inode(WireKind::OptionF64);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_option_f64_empty_0() {
    let schema = nested_1_option_schema(option_primitive(PrimitiveDataType::F64));
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_OPTION_F64_EMPTY_0,
        &NESTED_1_OPTION_F64_EMPTY_0_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionBool: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_1_OPTION_BOOL_EMPTY_0: [u8; 12] = [
    12, 0, 0, 0, // [0..4] payload start pointer
    0, 0, 0, 0, // [4..8] valid samples
    0, 0, 0, 0, // [8..12] total samples (nulls included)
];
pub const NESTED_1_OPTION_BOOL_EMPTY_0_INODES: [Inode; 1] =
    nested_1_option_inode(WireKind::OptionBool);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_option_bool_empty_0() {
    let schema = nested_1_option_schema(option_primitive(PrimitiveDataType::Bool));
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_OPTION_BOOL_EMPTY_0,
        &NESTED_1_OPTION_BOOL_EMPTY_0_INODES,
    );
}

// ---------------------------------------------------------------------------
// OptionTimestamp: empty lists
// ---------------------------------------------------------------------------

/// `[]`
pub const NESTED_1_OPTION_TIMESTAMP_EMPTY_0: [u8; 16] = [
    16, 0, 0, 0, // [0..4] payload start pointer
    0, 0, 0, 0, // [4..8] valid samples
    0, 0, 0, 0, // [8..12] total samples (nulls included)
    0, 0, 0, 0, // [12..16] alignment padding to 16 bytes before payload
];
pub const NESTED_1_OPTION_TIMESTAMP_EMPTY_0_INODES: [Inode; 1] =
    nested_1_option_inode(WireKind::OptionTimestamp);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_option_timestamp_empty_0() {
    let schema = nested_1_option_schema(PyroType::OptionPrimitiveScalar {
        primitive: PrimitiveDataType::Timestamp,
    });
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_OPTION_TIMESTAMP_EMPTY_0,
        &NESTED_1_OPTION_TIMESTAMP_EMPTY_0_INODES,
    );
}

// ===========================================================================
// A run of one present sample: its value byte and its mask byte, each
// starting a fresh byte.
// ===========================================================================

#[rustfmt::skip]
pub const NESTED_1_OPTION_BOOL_SINGLE_SOME: [u8; 14] = [
    12, 0, 0, 0, // payload start pointer
    1, 0, 0, 0, // valid samples
    1, 0, 0, 0, // total samples (nulls included)
    1, // valid payload bits 0..1 (LSB first) = 1
    1, // validity mask bits 0..1 (LSB first) = 1
];
pub const NESTED_1_OPTION_BOOL_SINGLE_SOME_INODES: [Inode; 1] =
    nested_1_option_inode(WireKind::OptionBool);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_nested_1_option_bool_single_some() {
    let schema = nested_1_option_schema(option_primitive(PrimitiveDataType::Bool));
    let data = PyroValue::List(vec![PyroValue::Bool(true)]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &NESTED_1_OPTION_BOOL_SINGLE_SOME,
        &NESTED_1_OPTION_BOOL_SINGLE_SOME_INODES,
    );
}
