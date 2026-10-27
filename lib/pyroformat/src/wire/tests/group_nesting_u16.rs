#[cfg(test)]
use pyro_schema::{
    PrimitiveDataType,
    human::{PyroField, PyroSchema, PyroType},
};

#[cfg(test)]
use crate::{
    PyroValue, value::PrimitiveValueList, wire::tests::support::run_list_wrapped_roundtrip_u16,
};

use pyro_schema::{Inode, KindMeta, WireKind};

// The plainest group list there is: `List<Group<..>>` stores its element
// count and nothing else, so the whole topology is the `2` at offset 0. The
// nullable twin of this test — where that count shares its slot with a
// pointer to a validity bitmap — lives in `group_option_nesting.rs`.
pub const LIST_GROUP: [u8; 8] = [
    2, 0, // 0 Group element count
    6, 0, 2, 0, // 2 "y" payload pointer
    100, 42, // 6
];
pub const LIST_GROUP_INODE: [Inode; 2] = [
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 1,
            count: 1,
            topology_offset: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 2,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_list_of_group() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::Group {
                    fields: vec![PyroField {
                        name: "y".to_string(),
                        doc: String::new(),
                        data_type: PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::U8,
                        },
                    }],
                }),
            },
        }],
    };

    let data = PyroValue::List(vec![
        PyroValue::Group([("y", PyroValue::from(100u8))].into()),
        PyroValue::Group([("y", PyroValue::from(42u8))].into()),
    ]);

    run_list_wrapped_roundtrip_u16(schema, data, &LIST_GROUP, &LIST_GROUP_INODE);
}

pub const DOUBLE_LIST_GROUP_WITH_STR_AND_INNER_LIST_GROUP: [u8; 50] = [
    // Group topology fixed
    24, 0, // 0
    2, 0, // 2
    // "a" topology fixed
    28, 0, // 4
    2, 0, // 6
    // "a" payload
    40, 0, // 8
    5, 0, // 10
    // "b" topology
    32, 0, // 12
    2, 0, // 14
    // "c" topology
    36, 0, // 16
    2, 0, // 18
    // "c" payload
    45, 0, // 20
    5, 0, // 22
    // Group topology payload — the outer list level is implied by this one
    255, 255, // 24
    1, 0, // 26
    // "a" topology payload
    255, 255, // 28
    5, 0, // 30
    // "b" topology payload
    255, 255, // 32
    1, 0, // 34
    // "c" topology payload
    255, 255, // 36
    5, 0, // 38
    119, 111, 114, 108, 100, // 40 "world"
    104, 101, 108, 108, 111, // 45 "hello"
];
pub const DOUBLE_LIST_GROUP_WITH_STR_AND_INNER_LIST_GROUP_INODES: [Inode; 4] = [
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            count: 2,
            nesting: 2,
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
            data_offset: 8,
            topology_offset: 4,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            count: 1,
            nesting: 1,
            topology_offset: 12,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::Str,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 20,
            topology_offset: 16,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_double_list_group_with_str_and_inner_list_group() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::List {
                    element: Box::new(PyroType::Group {
                        fields: vec![
                            PyroField {
                                name: "a".to_string(),
                                doc: String::new(),
                                data_type: PyroType::Str,
                            },
                            PyroField {
                                name: "b".to_string(),
                                doc: String::new(),
                                data_type: PyroType::List {
                                    element: Box::new(PyroType::Group {
                                        fields: vec![PyroField {
                                            name: "c".to_string(),
                                            doc: String::new(),
                                            data_type: PyroType::Str,
                                        }],
                                    }),
                                },
                            },
                        ],
                    }),
                }),
            },
        }],
    };

    // [{ "a": "world", "b": [{"c": "hello"}] }]
    let inner_group = PyroValue::Group([("c", PyroValue::Str("hello".into()))].into());
    let outer_group = PyroValue::Group(
        [
            ("a", PyroValue::Str("world".into())),
            ("b", PyroValue::List(vec![inner_group])),
        ]
        .into(),
    );
    let data = PyroValue::List(vec![PyroValue::List(vec![outer_group])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &DOUBLE_LIST_GROUP_WITH_STR_AND_INNER_LIST_GROUP,
        &DOUBLE_LIST_GROUP_WITH_STR_AND_INNER_LIST_GROUP_INODES,
    );
}

// The outer/inner list nesting for both `a`/`b` produces a chain of
// topology pointer indirections (generic "topology" labels below, per
// this file's usual latitude for offset/count words); the trailing 16
// bytes are the `f64`/`i64` payloads for `a: [1.0]` and `b: [-1]`.
pub const LIST_GROUP_WITH_LIST_LIST_F64_AND_LIST_LIST_I64: [u8; 56] = [
    1, 0, // 0 group element count
    // "a" topology pointer
    18, 0, 4, 0, // 2
    // "a" f64 payload pointer
    40, 0, 1, 0, // 6
    // "b" topology pointer
    26, 0, 4, 0, // 10
    // "b" i64 payload pointer
    48, 0, 1, 0, // 14
    // "a" list-of-list topology, level 1 then level 2
    255, 255, 1, 0, // 18
    255, 255, 1, 0, // 22
    // "b" list-of-list topology, level 1 then level 2
    255, 255, 1, 0, // 26
    255, 255, 1, 0, // 30
    0, 0, 0, 0, 0, 0, // 34 padding up to the f64's alignment
    // "a" f64 payload: 1.0
    0, 0, 0, 0, 0, 0, 240, 63, // 40
    // "b" i64 payload: -1
    255, 255, 255, 255, 255, 255, 255, 255, // 48
];
pub const LIST_GROUP_WITH_LIST_LIST_F64_AND_LIST_LIST_I64_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            count: 2,
            nesting: 1,
            topology_offset: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::F64,
        kind_meta: KindMeta::Primitive {
            nesting: 2,
            data_offset: 6,
            topology_offset: 2,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::I64,
        kind_meta: KindMeta::Primitive {
            nesting: 2,
            data_offset: 14,
            topology_offset: 10,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_list_group_with_list_list_f64_and_list_list_i64() {
    // `List<Group{a: List<List<f64>>, b: List<List<i64>>}>`
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::Group {
                    fields: vec![
                        PyroField {
                            name: "b1".to_string(),
                            doc: String::new(),
                            data_type: PyroType::List {
                                element: Box::new(PyroType::List {
                                    element: Box::new(PyroType::PrimitiveScalar {
                                        primitive: PrimitiveDataType::F64,
                                    }),
                                }),
                            },
                        },
                        PyroField {
                            name: "b2".to_string(),
                            doc: String::new(),
                            data_type: PyroType::List {
                                element: Box::new(PyroType::List {
                                    element: Box::new(PyroType::PrimitiveScalar {
                                        primitive: PrimitiveDataType::I64,
                                    }),
                                }),
                            },
                        },
                    ],
                }),
            },
        }],
    };

    let inner_group = PyroValue::Group(
        [
            (
                "b1",
                PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::F64(
                    vec![1.0].into(),
                ))]),
            ),
            (
                "b2",
                PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::I64(
                    vec![-1].into(),
                ))]),
            ),
        ]
        .into(),
    );
    let data = PyroValue::List(vec![inner_group]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &LIST_GROUP_WITH_LIST_LIST_F64_AND_LIST_LIST_I64,
        &LIST_GROUP_WITH_LIST_LIST_F64_AND_LIST_LIST_I64_INODES,
    );
}

#[rustfmt::skip]
pub const LIST_GROUP_TRIPLE_NESTED_U32: [u8; 16] = [
    1, 0, // 0 list-of-groups row count
    1, 0, // 2 group "b" count (repeated by the list)
    1, 0, // 4 group "c" count
    12, 0, 1, 0, // 6 "d" (u32) payload pointer
    0, 0, // 10 alignment padding (2 bytes)
    99, 0, 0, 0, // 12 u32 data
];
pub const LIST_GROUP_TRIPLE_NESTED_U32_INODES: [Inode; 4] = [
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            count: 1,
            nesting: 1,
            topology_offset: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            count: 1,
            nesting: 0,
            topology_offset: 2,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            count: 1,
            nesting: 0,
            topology_offset: 4,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 6,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_list_group_triple_nested_u32() {
    // `List<Group{a: Group{b: Group{c: u32}}}>`
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::Group {
                    fields: vec![PyroField {
                        name: "b".to_string(),
                        doc: String::new(),
                        data_type: PyroType::Group {
                            fields: vec![PyroField {
                                name: "c".to_string(),
                                doc: String::new(),
                                data_type: PyroType::Group {
                                    fields: vec![PyroField {
                                        name: "d".to_string(),
                                        doc: String::new(),
                                        data_type: PyroType::PrimitiveScalar {
                                            primitive: PrimitiveDataType::U32,
                                        },
                                    }],
                                },
                            }],
                        },
                    }],
                }),
            },
        }],
    };

    let group_c = PyroValue::Group([("d", PyroValue::U32(99))].into());
    let group_b = PyroValue::Group([("c", group_c)].into());
    let group_a = PyroValue::Group([("b", group_b)].into());
    let data = PyroValue::List(vec![group_a]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &LIST_GROUP_TRIPLE_NESTED_U32,
        &LIST_GROUP_TRIPLE_NESTED_U32_INODES,
    );
}

pub const LIST_GROUP_BYTES_AND_LIST_BYTES: [u8; 33] = [
    1, 0, // 0 Group element count
    // "a" topology pointers
    18, 0, 2, 0, // 2
    // "a" payload pointer
    30, 0, 2, 0, // 6
    // "b" topology pointers
    22, 0, 4, 0, // 10
    // "b" payload pointer
    32, 0, 1, 0, // 14
    // "a" topology
    255, 255, 2, 0, // 18
    // "b" topology, level 1
    255, 255, 1, 0, // 22
    // "b" topology, level 2
    255, 255, 1, 0, // 26
    // payload
    170, 187, // 30
    204, // 32
];
pub const LIST_GROUP_BYTES_AND_LIST_BYTES_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 1,
            count: 2,
            topology_offset: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Bytes,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 6,
            topology_offset: 2,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::Bytes,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 14,
            topology_offset: 10,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_list_group_bytes_and_list_bytes() {
    // `List<Group{a: Bytes, b: List<Bytes>}>`
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::Group {
                    fields: vec![
                        PyroField {
                            name: "a".to_string(),
                            doc: String::new(),
                            data_type: PyroType::Bytes,
                        },
                        PyroField {
                            name: "b".to_string(),
                            doc: String::new(),
                            data_type: PyroType::List {
                                element: Box::new(PyroType::Bytes),
                            },
                        },
                    ],
                }),
            },
        }],
    };

    let inner_group = PyroValue::Group(
        [
            ("a", PyroValue::Bytes(vec![0xAA, 0xBB].into())),
            (
                "b",
                PyroValue::List(vec![PyroValue::Bytes(vec![0xCC].into())]),
            ),
        ]
        .into(),
    );
    let data = PyroValue::List(vec![inner_group]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &LIST_GROUP_BYTES_AND_LIST_BYTES,
        &LIST_GROUP_BYTES_AND_LIST_BYTES_INODES,
    );
}

// `a: [u8; 16]` (fixed-size) and `b: List<[u16; 8]>` both live inside
// the repeated outer group, so both get topology/payload pointer
// indirection; the trailing 32 bytes are the two fixed-size payloads
// themselves — `b`'s `[u16; 8]` (first element 1234, rest 0) followed
// by `a`'s `[u8; 16]` (first byte 42, rest 0).
pub const LIST_GROUP_FIXED_U8_ARRAY_AND_LIST_FIXED_U16_ARRAY: [u8; 50] = [
    // outer group topology pointer -> (offset 32, offset 18)
    1, 0, // 0
    // "a" ([u8; 16]) payload pointer: offset 64, count 1
    34, 0, 1, 0, // 2 [u8;16] topology pointer
    // "b" (List<[u16; 8]>) topology pointer -> (offset 40, offset 44)
    14, 0, 2, 0, // 6 [u16;8] topology pointer
    18, 0, 1, 0, // 10 [u16;8] data pointer
    255, 255, 1, 0, // 14 [u16;8] topology
    210, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 18 [u16;8]
    // "a" payload: [u8; 16] = [42, 0, 0, ..., 0]
    42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 34 [u8; 16]
];
pub const LIST_GROUP_FIXED_U8_ARRAY_AND_LIST_FIXED_U16_ARRAY_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            count: 2,
            nesting: 1,
            topology_offset: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 2,
            topology_offset: 0,
            fixed_size: 16,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::U16,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 10,
            topology_offset: 6,
            fixed_size: 8,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_list_group_fixed_u8_array_and_list_fixed_u16_array() {
    // `List<Group{a: [u8; 16], b: List<[u16; 8]>}>` — a fixed-size array
    // field ("a") and a list of fixed-size arrays field ("b") inside a
    // struct that's itself repeated.
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::Group {
                    fields: vec![
                        PyroField {
                            name: "a".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveFixedList {
                                element: PrimitiveDataType::U8,
                                size: 16,
                            },
                        },
                        PyroField {
                            name: "b".to_string(),
                            doc: String::new(),
                            data_type: PyroType::List {
                                element: Box::new(PyroType::PrimitiveFixedList {
                                    element: PrimitiveDataType::U16,
                                    size: 8,
                                }),
                            },
                        },
                    ],
                }),
            },
        }],
    };

    let mut u8_data = vec![0u8; 16];
    u8_data[0] = 42;
    let mut u16_data = vec![0u16; 8];
    u16_data[0] = 1234;
    let inner_group = PyroValue::Group(
        [
            (
                "a",
                PyroValue::PrimitiveList(PrimitiveValueList::U8(u8_data.into())),
            ),
            (
                "b",
                PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::U16(
                    u16_data.into(),
                ))]),
            ),
        ]
        .into(),
    );
    let data = PyroValue::List(vec![inner_group]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &LIST_GROUP_FIXED_U8_ARRAY_AND_LIST_FIXED_U16_ARRAY,
        &LIST_GROUP_FIXED_U8_ARRAY_AND_LIST_FIXED_U16_ARRAY_INODES,
    );
}

pub const DOUBLE_LIST_GROUP_TWO_U8_FIELDS: [u8; 20] = [
    // Group topology ptr
    12, 0, // 0
    2, 0, // 2
    // "a" payload
    16, 0, // 4
    2, 0, // 6
    // "b" payload
    18, 0, // 8
    2, 0, // 10
    // Group topology: one inner list, covering groups 0..2
    255, 255, // 12
    2, 0, // 14
    // "a" payload
    1, 1, // 16
    // "b" payload
    2, 2, // 18
];
pub const DOUBLE_LIST_GROUP_TWO_U8_FIELDS_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            count: 2,
            nesting: 2,
            topology_offset: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 4,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 8,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_double_list_group_two_u8_fields() {
    // `List<List<Group{a: u8, b: u8}>>` — the `Group` itself is wrapped by
    // two `List` layers (`nesting: 2`), rather than being a plain
    // `nesting: 1` repeated group like the rest of this file.
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::List {
                    element: Box::new(PyroType::Group {
                        fields: vec![
                            PyroField {
                                name: "a".to_string(),
                                doc: String::new(),
                                data_type: PyroType::PrimitiveScalar {
                                    primitive: PrimitiveDataType::U8,
                                },
                            },
                            PyroField {
                                name: "b".to_string(),
                                doc: String::new(),
                                data_type: PyroType::PrimitiveScalar {
                                    primitive: PrimitiveDataType::U8,
                                },
                            },
                        ],
                    }),
                }),
            },
        }],
    };

    let inner_group = PyroValue::Group([("a", PyroValue::U8(1)), ("b", PyroValue::U8(2))].into());
    let data = PyroValue::List(vec![PyroValue::List(vec![
        inner_group.clone(),
        inner_group,
    ])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &DOUBLE_LIST_GROUP_TWO_U8_FIELDS,
        &DOUBLE_LIST_GROUP_TWO_U8_FIELDS_INODES,
    );
}

/// The `List<List<Group{a: u8, b: u8}>>` schema shared by the
/// `DOUBLE_LIST_GROUP_TWO_U8_FIELDS*` fixtures below, which differ only in
/// the *shape* of the value — one inner list, several, an empty one, none at
/// all. All of them share `DOUBLE_LIST_GROUP_TWO_U8_FIELDS_INODES`.
#[cfg(test)]
fn double_list_group_two_u8_fields_schema() -> PyroSchema {
    PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::List {
                    element: Box::new(PyroType::Group {
                        fields: vec![
                            PyroField {
                                name: "a".to_string(),
                                doc: String::new(),
                                data_type: PyroType::PrimitiveScalar {
                                    primitive: PrimitiveDataType::U8,
                                },
                            },
                            PyroField {
                                name: "b".to_string(),
                                doc: String::new(),
                                data_type: PyroType::PrimitiveScalar {
                                    primitive: PrimitiveDataType::U8,
                                },
                            },
                        ],
                    }),
                }),
            },
        }],
    }
}

// Uneven inner lists — the case that shows the single remaining topology
// level is doing real work: with the root level gone, the outer list's
// length is the array's entry count minus one (2 inner lists here), and each
// entry pair is one inner list's group range (0..1, then 1..3).
pub const DOUBLE_LIST_GROUP_TWO_U8_FIELDS_UNEVEN: [u8; 24] = [
    // Group topology ptr
    12, 0, // 0
    3, 0, // 2
    // "a" payload
    18, 0, // 4
    3, 0, // 6
    // "b" payload
    21, 0, // 8
    3, 0, // 10
    // Group topology: inner lists cover groups 0..1 and 1..3
    255, 255, // 12
    1, 0, // 14
    3, 0, // 16
    // "a" payload
    1, 3, 5, // 18
    // "b" payload
    2, 4, 6, // 21
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_double_list_group_two_u8_fields_uneven() {
    let data = PyroValue::List(vec![
        PyroValue::List(vec![PyroValue::Group(
            [("a", PyroValue::U8(1)), ("b", PyroValue::U8(2))].into(),
        )]),
        PyroValue::List(vec![
            PyroValue::Group([("a", PyroValue::U8(3)), ("b", PyroValue::U8(4))].into()),
            PyroValue::Group([("a", PyroValue::U8(5)), ("b", PyroValue::U8(6))].into()),
        ]),
    ]);

    run_list_wrapped_roundtrip_u16(
        double_list_group_two_u8_fields_schema(),
        data,
        &DOUBLE_LIST_GROUP_TWO_U8_FIELDS_UNEVEN,
        &DOUBLE_LIST_GROUP_TWO_U8_FIELDS_INODES,
    );
}

// An empty inner list in the middle: it takes a slot in the topology array
// (a repeated offset — 1, 1) and contributes nothing to either payload.
pub const DOUBLE_LIST_GROUP_TWO_U8_FIELDS_EMPTY_INNER: [u8; 24] = [
    // Group topology ptr
    12, 0, // 0
    4, 0, // 2
    // "a" payload
    20, 0, // 4
    2, 0, // 6
    // "b" payload
    22, 0, // 8
    2, 0, // 10
    // Group topology: groups 0..1, then the empty 1..1, then 1..2
    255, 255, // 12
    1, 0, // 14
    1, 0, // 16
    2, 0, // 18
    // "a" payload
    1, 3, // 20
    // "b" payload
    2, 4, // 22
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_double_list_group_two_u8_fields_empty_inner() {
    let data = PyroValue::List(vec![
        PyroValue::List(vec![PyroValue::Group(
            [("a", PyroValue::U8(1)), ("b", PyroValue::U8(2))].into(),
        )]),
        PyroValue::List(vec![]),
        PyroValue::List(vec![PyroValue::Group(
            [("a", PyroValue::U8(3)), ("b", PyroValue::U8(4))].into(),
        )]),
    ]);

    run_list_wrapped_roundtrip_u16(
        double_list_group_two_u8_fields_schema(),
        data,
        &DOUBLE_LIST_GROUP_TWO_U8_FIELDS_EMPTY_INNER,
        &DOUBLE_LIST_GROUP_TWO_U8_FIELDS_INODES,
    );
}

// A single empty inner list: two topology entries (one inner list) whose
// range is empty. This is what keeps `[[]]` distinguishable from `[]` now
// that the root level is gone — `[]` writes no topology array at all.
pub const DOUBLE_LIST_GROUP_TWO_U8_FIELDS_ONE_EMPTY_INNER: [u8; 16] = [
    // Group topology ptr
    12, 0, // 0
    2, 0, // 2
    // "a" payload (empty)
    16, 0, // 4
    0, 0, // 6
    // "b" payload (empty)
    16, 0, // 8
    0, 0, // 10
    // Group topology: one inner list covering the empty range 0..0
    255, 255, // 12
    0, 0, // 14
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_double_list_group_two_u8_fields_one_empty_inner() {
    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        double_list_group_two_u8_fields_schema(),
        data,
        &DOUBLE_LIST_GROUP_TWO_U8_FIELDS_ONE_EMPTY_INNER,
        &DOUBLE_LIST_GROUP_TWO_U8_FIELDS_INODES,
    );
}

// An empty outer list: the topology slot is `(start, start)`, so the reader
// bottoms out before reading any entry and there is no dynamic region at
// all — the whole value is the 24-byte fixed region.
pub const DOUBLE_LIST_GROUP_TWO_U8_FIELDS_EMPTY_OUTER: [u8; 12] = [
    // Group topology ptr — start == end, i.e. no entries
    12, 0, // 0
    0, 0, // 2
    // "a" payload (empty)
    12, 0, // 4
    0, 0, // 6
    // "b" payload (empty)
    12, 0, // 8
    0, 0, // 10
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_double_list_group_two_u8_fields_empty_outer() {
    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        double_list_group_two_u8_fields_schema(),
        data,
        &DOUBLE_LIST_GROUP_TWO_U8_FIELDS_EMPTY_OUTER,
        &DOUBLE_LIST_GROUP_TWO_U8_FIELDS_INODES,
    );
}

// Three repeated levels of nesting (outer group, inner "a" group, and
// "b"'s double-nested u32 list) each add their own topology
// pointer/data pair, per this file's usual generic "topology" labeling
// for offset/count words.
pub const TRIPLE_NESTED_LIST_GROUP_LIST_LIST_U32: [u8; 32] = [
    1, 0, // 0 outer group's element count
    14, 0, 2, 0, // 2 inner group's topology (nesting 1 inside a nested thing)
    18, 0, 4, 0, // 6 u32's topology (nesting 2 inside a nested thing)
    28, 0, 1, 0, // 10 u32's data pointer
    // The inner group keeps a level of its own: unlike the outer one it
    // repeats per outer group, so its count is an array rather than a number.
    255, 255, 1, 0, // 14 inner group's topology
    255, 255, 1, 0, // 18 u32's topology layer 1
    255, 255, 1, 0, // 22 u32's topology layer 2
    0, 0, // 26 alignment padding (2 bytes)
    42, 0, 0, 0, // 28
];
pub const TRIPLE_NESTED_LIST_GROUP_LIST_LIST_U32_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            count: 1,
            nesting: 1,
            topology_offset: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            count: 1,
            nesting: 1,
            topology_offset: 2,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 2,
            data_offset: 10,
            topology_offset: 6,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_triple_nested_list_group_list_list_u32() {
    // `List<Group{a: List<Group{b: List<List<u32>>}>}>` — three repeated
    // levels: the outer group, the `a` group nested inside it (also
    // repeated), and `b`'s own list-of-list `u32`.
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::Group {
                    fields: vec![PyroField {
                        name: "b".to_string(),
                        doc: String::new(),
                        data_type: PyroType::List {
                            element: Box::new(PyroType::Group {
                                fields: vec![PyroField {
                                    name: "c".to_string(),
                                    doc: String::new(),
                                    data_type: PyroType::List {
                                        element: Box::new(PyroType::List {
                                            element: Box::new(PyroType::PrimitiveScalar {
                                                primitive: PrimitiveDataType::U32,
                                            }),
                                        }),
                                    },
                                }],
                            }),
                        },
                    }],
                }),
            },
        }],
    };

    let inner_group = PyroValue::Group(
        [(
            "c",
            PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::U32(
                vec![42].into(),
            ))]),
        )]
        .into(),
    );
    let outer_group = PyroValue::Group([("b", PyroValue::List(vec![inner_group]))].into());
    let data = PyroValue::List(vec![outer_group]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &TRIPLE_NESTED_LIST_GROUP_LIST_LIST_U32,
        &TRIPLE_NESTED_LIST_GROUP_LIST_LIST_U32_INODES,
    );
}

// `a` (bool) and `b` (double-nested bool list, 3 elements: true,
// false, true) both live inside the repeated outer group, each getting
// their own payload pointer; offset/count words are labeled
// generically as "topology" per this file's usual latitude.
pub const LIST_GROUP_BOOL_AND_LIST_LIST_BOOL: [u8; 24] = [
    1, 0, // 0 Main group's element count
    22, 0, 1, 0, // 2 bool 1 pointer
    14, 0, 4, 0, // 6 bool 2 topology
    23, 0, 3, 0, // 10 bool 2's pointer
    255, 255, 1, 0, // 14 bool 2's layer 1
    255, 255, 3, 0, // 18 bool 2's layer 2
    1, // 22 bool 1
    5, // 23 bool 2
];
pub const LIST_GROUP_BOOL_AND_LIST_LIST_BOOL_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            count: 2,
            nesting: 1,
            topology_offset: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Bool,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 2,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::Bool,
        kind_meta: KindMeta::Primitive {
            nesting: 2,
            data_offset: 10,
            topology_offset: 6,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_list_group_bool_and_list_list_bool() {
    // `List<Group{a: bool, b: List<List<bool>>}>`
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::Group {
                    fields: vec![
                        PyroField {
                            name: "a".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::Bool,
                            },
                        },
                        PyroField {
                            name: "b".to_string(),
                            doc: String::new(),
                            data_type: PyroType::List {
                                element: Box::new(PyroType::List {
                                    element: Box::new(PyroType::PrimitiveScalar {
                                        primitive: PrimitiveDataType::Bool,
                                    }),
                                }),
                            },
                        },
                    ],
                }),
            },
        }],
    };

    let inner_group = PyroValue::Group(
        [
            ("a", PyroValue::Bool(true)),
            (
                "b",
                PyroValue::List(vec![PyroValue::from(vec![true, false, true])]),
            ),
        ]
        .into(),
    );
    let data = PyroValue::List(vec![inner_group]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &LIST_GROUP_BOOL_AND_LIST_LIST_BOOL,
        &LIST_GROUP_BOOL_AND_LIST_LIST_BOOL_INODES,
    );
}

pub const LIST_GROUP_TWO_STR_FIELDS: [u8; 33] = [
    1, 0, // 0 Group element count
    18, 0, 2, 0, // 2 "a" topology pointer
    26, 0, 3, 0, // 6 "a" string pointer
    22, 0, 2, 0, // 10 "b" topology pointer
    29, 0, 4, 0, // 14 "b" string pointer
    // "a"'s per-element string offsets
    255, 255, 3, 0, // 18
    // "b"'s per-element string offsets
    255, 255, 4, 0, // 22
    // Payload
    116, 119, 111, // 26 "two"
    102, 111, 117, 114, // 29 "four"
];
pub const LIST_GROUP_TWO_STR_FIELDS_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            count: 2,
            nesting: 1,
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
            data_offset: 6,
            topology_offset: 2,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::Str,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 14,
            topology_offset: 10,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_list_group_two_str_fields() {
    // `List<Group{a: Str, b: Str}>`
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::Group {
                    fields: vec![
                        PyroField {
                            name: "a".to_string(),
                            doc: String::new(),
                            data_type: PyroType::Str,
                        },
                        PyroField {
                            name: "b".to_string(),
                            doc: String::new(),
                            data_type: PyroType::Str,
                        },
                    ],
                }),
            },
        }],
    };

    let inner_group = PyroValue::Group(
        [
            ("a", PyroValue::Str("two".into())),
            ("b", PyroValue::Str("four".into())),
        ]
        .into(),
    );
    let data = PyroValue::List(vec![inner_group]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &LIST_GROUP_TWO_STR_FIELDS,
        &LIST_GROUP_TWO_STR_FIELDS_INODES,
    );
}
