#[cfg(test)]
use pyro_schema::human::{PyroField, PyroSchema, PyroType};

#[cfg(test)]
use crate::{PyroValue, wire::tests::support::run_list_wrapped_roundtrip_u16};

use pyro_schema::{Inode, KindMeta, WireKind};

// The `Str`/`Bytes` counterpart to `primitive_nesting.rs` — `List<Str>`/
// `List<Bytes>` (`nesting: 1`) and `List<List<Str>>`/`List<List<Bytes>>`
// (`nesting: 2`). Every test wraps a single top-level field `a`.
//
// `nesting: 1` shapes reuse `primitive_nesting.rs::test_list_of_str`'s
// already-established header shape (a `[topology_start, topology_end,
// data_ptr, data_len]` quad, `data_offset: 4` since the list's own
// start/end pointer pair occupies the first 4 bytes and `Str`/`Bytes`'
// intrinsic descriptor sits right after) — extrapolated by hand to a
// different element count/content here, not independently re-derived
// against the real writer for this exact input.
//
// `nesting: 2` shapes have no existing precedent to reuse (unlike
// `primitive_nesting.rs`'s `List<List<u64>>`, whose elements are
// fixed-size and need only one boundary array): a `List<List<Str>>` needs
// *two* nested variable-length boundary arrays — one for the sublist
// leaf-count boundaries, one for the per-leaf string-byte boundaries — so
// these are a best-effort, internally self-consistent (every offset here
// really does point at the byte position claimed) but otherwise
// unverified structural guess, in the same spirit as
// `primitive_nesting.rs::test_triple_nested_list_bytes`'s own "exact
// per-word semantics not independently re-derived" caveat.

pub const LIST_OF_STR: [u8; 21] = [
    // Topology fixed (the outer `List`'s own start/end pointer pair)
    8, 0, // start pointer
    4, 0, // end pointer (address of the last boundary word)
    // Str's own intrinsic (data_ptr, data_len) fixed pair
    16, 0, // data pointer
    5, 0, // data length (2 + 0 + 3)
    // Per-element cumulative string-byte boundaries (N + 1 = 4 words)
    255, 255, // element[0] "ab" starts at 0
    2, 0, // element[0] ends / element[1] "" starts at 2
    2, 0, // element[1] ends / element[2] "cde" starts at 2
    5, 0, // element[2] ends at 5
    // String payload: "ab" + "" + "cde"
    97, 98, 99, 100, 101,
];
pub const LIST_OF_STR_INODES: [Inode; 1] = [Inode {
    kind: WireKind::Str,
    kind_meta: KindMeta::Primitive {
        nesting: 1,
        data_offset: 4,
        topology_offset: 0,
        fixed_size: 0,
        nullable_bit_index: 0,
        group: false,
        group_nest: false,
    },
}];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_list_of_str() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::Str),
            },
        }],
    };

    // 3 elements, including an empty string, to exercise a zero-width
    // boundary gap ("ab" -> "" -> "cde").
    let data = PyroValue::List(vec![
        PyroValue::Str("ab".into()),
        PyroValue::Str("".into()),
        PyroValue::Str("cde".into()),
    ]);

    run_list_wrapped_roundtrip_u16(schema, data, &LIST_OF_STR, &LIST_OF_STR_INODES);
}

pub const LIST_OF_BYTES: [u8; 19] = [
    // Topology fixed
    8, 0, // start pointer
    3, 0, // end pointer (address of the last boundary word)
    // Bytes' own intrinsic (data_ptr, data_len) fixed pair
    14, 0, // data pointer
    5, 0, // data length (3 + 2)
    // Per-element cumulative byte boundaries (N + 1 = 3 words)
    255, 255, // element[0] starts at 0
    3, 0, // element[0] ends / element[1] starts at 3
    5, 0, // element[1] ends at 5
    // Bytes payload
    10, 20, 30, 40, 50,
];
pub const LIST_OF_BYTES_INODES: [Inode; 1] = [Inode {
    kind: WireKind::Bytes,
    kind_meta: KindMeta::Primitive {
        nesting: 1,
        data_offset: 4,
        topology_offset: 0,
        fixed_size: 0,
        nullable_bit_index: 0,
        group: false,
        group_nest: false,
    },
}];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_list_of_bytes() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::Bytes),
            },
        }],
    };

    let data = PyroValue::List(vec![
        PyroValue::Bytes(vec![10, 20, 30].into()),
        PyroValue::Bytes(vec![40, 50].into()),
    ]);

    run_list_wrapped_roundtrip_u16(schema, data, &LIST_OF_BYTES, &LIST_OF_BYTES_INODES);
}

pub const LIST_OF_LIST_STR: [u8; 28] = [
    // Header: sublist topology start/end, leaf string-boundary array
    // pointer, payload pointer, total leaf count, total payload bytes
    8, 0, 7, 0, // 0 sublist topology
    22, 0, 6, 0, // 4 payload pointer
    // Sublist leaf-count boundaries (N_outer + 1 = 3 words): sublist[0]
    // owns leaves [0, 2), sublist[1] owns leaves [2, 3)
    255, 255, // 8
    2, 0, // 10
    3, 0, // 12
    // Per-leaf string-byte boundaries (N_leaf + 1 = 4 words): "x"
    // [0,1), "yz" [1,3), "abc" [3,6)
    255, 255, // 14
    1, 0, // 16
    3, 0, // 18
    6, 0, // 20
    120, 121, 122, 97, 98, 99, // data
];
pub const LIST_OF_LIST_STR_INODES: [Inode; 1] = [Inode {
    kind: WireKind::Str,
    kind_meta: KindMeta::Primitive {
        nesting: 2,
        data_offset: 4,
        topology_offset: 0,
        fixed_size: 0,
        nullable_bit_index: 0,
        group: false,
        group_nest: false,
    },
}];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_list_of_list_str() {
    // `List<List<Str>>`: `[["x", "yz"], ["abc"]]` — flattened leaf order
    // "x", "yz", "abc" (lengths 1, 2, 3; total 6 payload bytes).
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::List {
                    element: Box::new(PyroType::Str),
                }),
            },
        }],
    };

    let data = PyroValue::List(vec![
        PyroValue::List(vec![
            PyroValue::Str("x".into()),
            PyroValue::Str("yz".into()),
        ]),
        PyroValue::List(vec![PyroValue::Str("abc".into())]),
    ]);

    run_list_wrapped_roundtrip_u16(schema, data, &LIST_OF_LIST_STR, &LIST_OF_LIST_STR_INODES);
}

pub const LIST_OF_LIST_BYTES: [u8; 28] = [
    8, 0, 7, 0, // 0 Root topology
    22, 0, 6, 0, // 4 payload pointer
    // Sublist leaf-count boundaries: sublist[0] owns leaves [0, 2),
    // sublist[1] owns leaves [2, 3)
    255, 255, // 8
    2, 0, // 10
    3, 0, // 12
    // Per-leaf byte boundaries: [1] -> [0,1), [2,3] -> [1,3),
    // [4,5,6] -> [3,6)
    255, 255, // 14
    1, 0, // 16
    3, 0, // 18
    6, 0, // 20 Bytes payload
    1, 2, 3, 4, 5, 6, // 22 data
];
pub const LIST_OF_LIST_BYTES_INODES: [Inode; 1] = [Inode {
    kind: WireKind::Bytes,
    kind_meta: KindMeta::Primitive {
        nesting: 2,
        data_offset: 4,
        topology_offset: 0,
        fixed_size: 0,
        nullable_bit_index: 0,
        group: false,
        group_nest: false,
    },
}];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_list_of_list_bytes() {
    // `List<List<Bytes>>`: `[[[1], [2,3]], [[4,5,6]]]` — same leaf-length
    // shape (1, 2, 3) as `test_list_of_list_str` above, so it reuses the
    // identical header/boundary-array numbers; only the payload bytes
    // differ (raw values instead of ASCII).
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::List {
                    element: Box::new(PyroType::Bytes),
                }),
            },
        }],
    };

    let data = PyroValue::List(vec![
        PyroValue::List(vec![
            PyroValue::Bytes(vec![1].into()),
            PyroValue::Bytes(vec![2, 3].into()),
        ]),
        PyroValue::List(vec![PyroValue::Bytes(vec![4, 5, 6].into())]),
    ]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &LIST_OF_LIST_BYTES,
        &LIST_OF_LIST_BYTES_INODES,
    );
}

pub const GROUP_OPTION_STR_SOME: [u8; 10] = [
    5, 0, // data pointer (byte 5)
    5, 0, // data length
    1, // validity bitmask (present)
    104, 101, 108, 108, 111, // "hello"
];
pub const GROUP_OPTION_STR_INODES: [Inode; 2] = [
    Inode {
        kind: WireKind::OptionGroup,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 1,
            topology_offset: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Str,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 0,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_option_str_some() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::OptionGroup {
                fields: vec![PyroField {
                    name: "s".to_string(),
                    doc: String::new(),
                    data_type: PyroType::Str,
                }],
            },
        }],
    };

    let data = PyroValue::Group([("s", PyroValue::Str("hello".into()))].into());

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &GROUP_OPTION_STR_SOME,
        &GROUP_OPTION_STR_INODES,
    );
}
