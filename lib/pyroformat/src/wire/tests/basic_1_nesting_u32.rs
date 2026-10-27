//! `List<Str>` / `List<Bytes>`: the `Str`/`Bytes`
//! counterpart to `primitive_1_nesting.rs`. A blob is a list of bytes, so
//! nesting 1 carries 1 topology level: the list levels, then one
//! more of per-blob cumulative byte boundaries.
//!
//! Shared value shape: `[a, b, c, d, e, f]`, six blobs:
//! `Str` = `"ab", "", "cde", "f", "ghij", "é"` (the last is two UTF-8
//! bytes); `Bytes` = `[1, 2], [], [3, 4, 5], [255], [0, 6, 7, 8], [9]`.
//!
//! Layout: `[topology start, topology word count, payload start, payload
//! byte count]`, then the sentinel-prefixed cumulative boundaries, then the
//! raw payload bytes (no alignment).

#[cfg(test)]
use pyro_schema::human::{PyroField, PyroSchema, PyroType};

#[cfg(test)]
use crate::{PyroValue, wire::tests::support::run_list_wrapped_roundtrip_u32};

use pyro_schema::{Inode, KindMeta, WireKind};

#[cfg(test)]
fn basic_1_schema(leaf: PyroType) -> PyroSchema {
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

/// `[a, b, c, d, e, f]` from 1 run of blobs.
#[cfg(test)]
fn basic_1_value(runs: [Vec<PyroValue<'static>>; 1]) -> PyroValue<'static> {
    let [r0] = runs;
    PyroValue::List(r0)
}

/// `List<Str|Bytes>` at the root: one `Primitive` inode with
/// `nesting: 1`, its topology slot at 0 and its data slot at 8.
const fn basic_1_inode(kind: WireKind) -> [Inode; 1] {
    [Inode {
        kind,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 8,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    }]
}

// ---------------------------------------------------------------------------
// Str
// ---------------------------------------------------------------------------

pub const BASIC_1_STR: [u8; 56] = [
    16, 0, 0, 0, // topology start pointer
    7, 0, 0, 0, // topology word count (1 level, sentinels included)
    44, 0, 0, 0, // payload start pointer
    12, 0, 0, 0, // payload byte count
    255, 255, 255, 255, // level 0: sentinel
    2, 0, 0, 0, // level 0: boundary at 2 bytes ("ab" ends)
    2, 0, 0, 0, // level 0: boundary at 2 bytes ("" ends)
    5, 0, 0, 0, // level 0: boundary at 5 bytes ("cde" ends)
    6, 0, 0, 0, // level 0: boundary at 6 bytes ("f" ends)
    10, 0, 0, 0, // level 0: boundary at 10 bytes ("ghij" ends)
    12, 0, 0, 0, // level 0: boundary at 12 bytes ("é" ends)
    97, 98, // payload "ab"
    99, 100, 101, // payload "cde"
    102, // payload "f"
    103, 104, 105, 106, // payload "ghij"
    195, 169, // payload "é"
];
pub const BASIC_1_STR_INODES: [Inode; 1] = basic_1_inode(WireKind::Str);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_basic_1_str() {
    let schema = basic_1_schema(PyroType::Str);
    let data = basic_1_value([vec![
        PyroValue::Str("ab".into()),
        PyroValue::Str("".into()),
        PyroValue::Str("cde".into()),
        PyroValue::Str("f".into()),
        PyroValue::Str("ghij".into()),
        PyroValue::Str("é".into()),
    ]]);

    run_list_wrapped_roundtrip_u32(schema, data, &BASIC_1_STR, &BASIC_1_STR_INODES);
}

// ---------------------------------------------------------------------------
// Bytes
// ---------------------------------------------------------------------------

pub const BASIC_1_BYTES: [u8; 55] = [
    16, 0, 0, 0, // topology start pointer
    7, 0, 0, 0, // topology word count (1 level, sentinels included)
    44, 0, 0, 0, // payload start pointer
    11, 0, 0, 0, // payload byte count
    255, 255, 255, 255, // level 0: sentinel
    2, 0, 0, 0, // level 0: boundary at 2 bytes ([1, 2] ends)
    2, 0, 0, 0, // level 0: boundary at 2 bytes ([] ends)
    5, 0, 0, 0, // level 0: boundary at 5 bytes ([3, 4, 5] ends)
    6, 0, 0, 0, // level 0: boundary at 6 bytes ([255] ends)
    10, 0, 0, 0, // level 0: boundary at 10 bytes ([0, 6, 7, 8] ends)
    11, 0, 0, 0, // level 0: boundary at 11 bytes ([9] ends)
    1, 2, // payload [1, 2]
    3, 4, 5,   // payload [3, 4, 5]
    255, // payload [255]
    0, 6, 7, 8, // payload [0, 6, 7, 8]
    9, // payload [9]
];
pub const BASIC_1_BYTES_INODES: [Inode; 1] = basic_1_inode(WireKind::Bytes);

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_basic_1_bytes() {
    let schema = basic_1_schema(PyroType::Bytes);
    let data = basic_1_value([vec![
        PyroValue::Bytes(vec![1, 2].into()),
        PyroValue::Bytes(vec![].into()),
        PyroValue::Bytes(vec![3, 4, 5].into()),
        PyroValue::Bytes(vec![255].into()),
        PyroValue::Bytes(vec![0, 6, 7, 8].into()),
        PyroValue::Bytes(vec![9].into()),
    ]]);

    run_list_wrapped_roundtrip_u32(schema, data, &BASIC_1_BYTES, &BASIC_1_BYTES_INODES);
}
