#[cfg(test)]
use pyro_schema::{
    PrimitiveDataType,
    human::{PyroField, PyroSchema, PyroType},
};

#[cfg(test)]
use crate::{PyroValue, wire::tests::support::run_list_wrapped_roundtrip_u32};

use pyro_schema::{Inode, KindMeta, WireKind};

#[rustfmt::skip]
pub const MAP_STR_U32_SINGLE_ENTRY: [u8; 43] = [
    1, 0, 0, 0, // 0 map count
    28, 0, 0, 0, 2, 0, 0, 0, // 4 string topology pointer
    40, 0, 0, 0, 3, 0, 0, 0, // 12 string data
    36, 0, 0, 0, 1, 0, 0, 0, // 20 u32 data
    255, 255, 255, 255, 3, 0, 0, 0, // 28 string topology
    42, 0, 0, 0, // 36 u32 data
    107, 101, 121, // 40 string data
];
pub const MAP_STR_U32_SINGLE_ENTRY_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::Map,
        kind_meta: KindMeta::Map {
            nesting: 0,
            topology_offset: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Str,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 12,
            topology_offset: 4,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 20,
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
fn test_map_str_u32_single_entry() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::Map {
                key: Box::new(PyroType::Str),
                value: Box::new(PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U32,
                }),
            },
        }],
    };

    let data = PyroValue::Map(vec![(PyroValue::Str("key".into()), PyroValue::U32(42))].into());

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &MAP_STR_U32_SINGLE_ENTRY,
        &MAP_STR_U32_SINGLE_ENTRY_INODES,
    );
}

// Captured from the real writer output.
#[rustfmt::skip]
pub const MAP_STR_U32_THREE_ENTRIES: [u8; 59] = [
    3, 0, 0, 0, // 0 Map count
    28, 0, 0, 0, 4, 0, 0, 0, // 4 String topology pointer
    56, 0, 0, 0, 3, 0, 0, 0, // 12 String data offset
    44, 0, 0, 0, 3, 0, 0, 0, // 20 U32 topology pointer
    255, 255, 255, 255, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, // 28 String data
    1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, // 44 U32 data
    97, 98, 99, // 56 String data
];
pub const MAP_STR_U32_THREE_ENTRIES_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::Map,
        kind_meta: KindMeta::Map {
            nesting: 0,
            topology_offset: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Str,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 12,
            topology_offset: 4,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 20,
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
fn test_map_str_u32_three_entries() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::Map {
                key: Box::new(PyroType::Str),
                value: Box::new(PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U32,
                }),
            },
        }],
    };

    let data = PyroValue::Map(
        vec![
            (PyroValue::Str("a".into()), PyroValue::U32(1)),
            (PyroValue::Str("b".into()), PyroValue::U32(2)),
            (PyroValue::Str("c".into()), PyroValue::U32(3)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &MAP_STR_U32_THREE_ENTRIES,
        &MAP_STR_U32_THREE_ENTRIES_INODES,
    );
}

// Captured from the real writer output.
#[rustfmt::skip]
pub const MAP_STR_U32_THREE_ENTRIES_XYZ: [u8; 59] = [
    3, 0, 0, 0, // 0 Map count
    28, 0, 0, 0, 4, 0, 0, 0, // 4 String topology pointer
    56, 0, 0, 0, 3, 0, 0, 0, // 12 String data offset
    44, 0, 0, 0, 3, 0, 0, 0, // 20 U32 topology pointer
    255, 255, 255, 255, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, // 28 String topology
    10, 0, 0, 0, 20, 0, 0, 0, 30, 0, 0, 0, // 44 U32 data
    120, 121, 122, // 56
];
pub const MAP_STR_U32_THREE_ENTRIES_XYZ_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::Map,
        kind_meta: KindMeta::Map {
            nesting: 0,
            topology_offset: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Str,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 12,
            topology_offset: 4,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 20,
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
fn test_map_str_u32_three_entries_xyz() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::Map {
                key: Box::new(PyroType::Str),
                value: Box::new(PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U32,
                }),
            },
        }],
    };

    let data = PyroValue::Map(
        vec![
            (PyroValue::Str("x".into()), PyroValue::U32(10)),
            (PyroValue::Str("y".into()), PyroValue::U32(20)),
            (PyroValue::Str("z".into()), PyroValue::U32(30)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &MAP_STR_U32_THREE_ENTRIES_XYZ,
        &MAP_STR_U32_THREE_ENTRIES_XYZ_INODES,
    );
}

// Captured from the real writer output — same shape as
// `test_map_str_u32_single_entry`, just a 1-byte key instead of 3.
#[rustfmt::skip]
pub const MAP_STR_U32_SINGLE_ENTRY_A: [u8; 41] = [
    1, 0, 0, 0, // 0 Map count
    28, 0, 0, 0, 2, 0, 0, 0, // 4 String topology pointer
    40, 0, 0, 0, 1, 0, 0, 0, // 12 string data pointer
    36, 0, 0, 0, 1, 0, 0, 0, // 20 U32 data pointer
    255, 255, 255, 255, 1, 0, 0, 0, // 28 String topology
    1, 0, 0, 0, // 36 U32 data
    97, // 40 String data
];
pub const MAP_STR_U32_SINGLE_ENTRY_A_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::Map,
        kind_meta: KindMeta::Map {
            nesting: 0,
            topology_offset: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Str,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 12,
            topology_offset: 4,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 20,
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
fn test_map_str_u32_single_entry_a() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::Map {
                key: Box::new(PyroType::Str),
                value: Box::new(PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U32,
                }),
            },
        }],
    };

    let data = PyroValue::Map(vec![(PyroValue::Str("a".into()), PyroValue::U32(1))].into());

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &MAP_STR_U32_SINGLE_ENTRY_A,
        &MAP_STR_U32_SINGLE_ENTRY_A_INODES,
    );
}

// Captured from the real writer output. Unlike the `Str`-keyed cases
// above, `U8` is fixed-size, so the key list needs no boundary/stride
// machinery at all — just a plain `[ptr, count]` pair, same shape as
// the value list.
#[rustfmt::skip]
pub const MAP_U8_KEY_U32_VALUE: [u8; 30] = [
    2, 0, 0, 0, // Map count
    28, 0, 0, 0, 2, 0, 0, 0, // u8 data pointer
    20, 0, 0, 0, 2, 0, 0, 0, // u32 data pointer
    100, 0, 0, 0, 200, 0, 0, 0, // u32 data
    1, 2, // u8 data
];
pub const MAP_U8_KEY_U32_VALUE_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::Map,
        kind_meta: KindMeta::Map {
            nesting: 0,
            topology_offset: 0,
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
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 12,
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
fn test_map_u8_key_u32_value() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::Map {
                key: Box::new(PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                }),
                value: Box::new(PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U32,
                }),
            },
        }],
    };

    let data = PyroValue::Map(
        vec![
            (PyroValue::U8(1), PyroValue::U32(100)),
            (PyroValue::U8(2), PyroValue::U32(200)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &MAP_U8_KEY_U32_VALUE,
        &MAP_U8_KEY_U32_VALUE_INODES,
    );
}

// Captured from the real writer output. Zero entries still emits a
// key-list `NestedListTopology` and a value-list pointer+count pair,
// both with count 0 — no payload bytes at all.
#[rustfmt::skip]
pub const MAP_STR_U32_EMPTY: [u8; 28] = [
    0, 0, 0, 0, // 0 Map count
    28, 0, 0, 0, 0, 0, 0, 0, // 4 string topology
    28, 0, 0, 0, 0, 0, 0, 0, // 12 string data
    28, 0, 0, 0, 0, 0, 0, 0, // 24 u32 data
];
pub const MAP_STR_U32_EMPTY_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::Map,
        kind_meta: KindMeta::Map {
            nesting: 0,
            topology_offset: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Str,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 12,
            topology_offset: 4,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 20,
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
fn test_map_str_u32_empty() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::Map {
                key: Box::new(PyroType::Str),
                value: Box::new(PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U32,
                }),
            },
        }],
    };

    let data = PyroValue::Map(vec![].into());

    run_list_wrapped_roundtrip_u32(schema, data, &MAP_STR_U32_EMPTY, &MAP_STR_U32_EMPTY_INODES);
}

// Captured from the real writer output.
#[rustfmt::skip]
pub const MAP_STR_OPTION_U32_ONE_NULL: [u8; 60] = [
    3, 0, 0, 0, // Map count
    32, 0, 0, 0, 4, 0, 0, 0, // String topology pointer
    57, 0, 0, 0, 3, 0, 0, 0, // String data pointer
    48, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, // 24 Option u32 pointer
    255, 255, 255, 255, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, // 40 string topology
    1, 0, 0, 0, 3, 0, 0, 0, // Option u32 data
    5, // Option u32 mask
    97, 98, 99, // 57 String data
];
pub const MAP_STR_OPTION_U32_ONE_NULL_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::Map,
        kind_meta: KindMeta::Map {
            nesting: 0,
            topology_offset: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Str,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 12,
            topology_offset: 4,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::OptionU32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 20,
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
fn test_map_str_option_u32_one_null() {
    // `Map<Str, OptionU32>` — the map's *value* child is itself nullable,
    // as opposed to every other case in this file, where the whole map can
    // be null but every present entry's value is a plain scalar.
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::Map {
                key: Box::new(PyroType::Str),
                value: Box::new(PyroType::OptionPrimitiveScalar {
                    primitive: PrimitiveDataType::U32,
                }),
            },
        }],
    };

    let data = PyroValue::Map(
        vec![
            (PyroValue::Str("a".into()), PyroValue::U32(1)),
            (PyroValue::Str("b".into()), PyroValue::Null),
            (PyroValue::Str("c".into()), PyroValue::U32(3)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &MAP_STR_OPTION_U32_ONE_NULL,
        &MAP_STR_OPTION_U32_ONE_NULL_INODES,
    );
}

// Captured from the real writer output for `List<Map<Str, U32>>` (one
// outer list element containing a single-entry map). Unlike a bare Map
// wrapped by nothing, the key/value children's own `nesting` stays 0 —
// only the Map root's `nesting` tracks the outer List wrapping (the same
// rule as `List<Group>`, where only the Group root's nesting is bumped).
#[rustfmt::skip]
pub const LIST_MAP_STR_U32: [u8; 53] = [
    32, 0, 0, 0, 2, 0, 0, 0, // 0 Map topology pointer
    40, 0, 0, 0, 2, 0, 0, 0, // 8 str topology pointer
    52, 0, 0, 0, 1, 0, 0, 0, // 16 str pointer
    48, 0, 0, 0, 1, 0, 0, 0, // 24 u32 pointer
    255, 255, 255, 255, 1, 0, 0, 0, // 32 
    255, 255, 255, 255, 1, 0, 0, 0, // 40 
    // value (U32) data: 10
    10, 0, 0, 0, // 48
    // key string bytes: "x"
    120, // 52
];
pub const LIST_MAP_STR_U32_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::Map,
        kind_meta: KindMeta::Map {
            nesting: 1,
            topology_offset: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Str,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 16,
            topology_offset: 8,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 24,
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
fn test_list_of_map_str_u32() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::Map {
                    key: Box::new(PyroType::Str),
                    value: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::U32,
                    }),
                }),
            },
        }],
    };

    let data = PyroValue::List(vec![PyroValue::Map(
        vec![(PyroValue::Str("x".into()), PyroValue::U32(10))].into(),
    )]);

    run_list_wrapped_roundtrip_u32(schema, data, &LIST_MAP_STR_U32, &LIST_MAP_STR_U32_INODES);
}

// Captured writer output for `Map<U8, Group<{ x: U32, y: U32 }>>`.
#[rustfmt::skip]
pub const MAP_U8_KEY_GROUP_VALUE: [u8; 41] = [
    1, 0, 0, 0, // 0 Map count
    40, 0, 0, 0, 1, 0, 0, 0, // 4 u8 key pointer
    1, 0, 0, 0, // 12 value group count
    32, 0, 0, 0, 1, 0, 0, 0, // 16 x (u32) pointer
    36, 0, 0, 0, 1, 0, 0, 0, // 24 y (u32) pointer
    10, 0, 0, 0, 20, 0, 0, 0, // 32 u32 data
    1, // 40 u8 data
];
pub const MAP_U8_KEY_GROUP_VALUE_INODES: [Inode; 5] = [
    Inode {
        kind: WireKind::Map,
        kind_meta: KindMeta::Map {
            nesting: 0,
            topology_offset: 0,
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
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 2,
            topology_offset: 12,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 16,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 24,
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
fn test_map_u8_key_group_value() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::Map {
                key: Box::new(PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                }),
                value: Box::new(PyroType::Group {
                    fields: vec![
                        PyroField {
                            name: "x".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U32,
                            },
                        },
                        PyroField {
                            name: "y".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U32,
                            },
                        },
                    ],
                }),
            },
        }],
    };

    let data = PyroValue::Map(
        vec![(
            PyroValue::U8(1),
            PyroValue::Group([("x", PyroValue::U32(10)), ("y", PyroValue::U32(20))].into()),
        )]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &MAP_U8_KEY_GROUP_VALUE,
        &MAP_U8_KEY_GROUP_VALUE_INODES,
    );
}

// Captured writer output for `Map<Group<{ x: U32, y: U32 }>, U8>`.
#[rustfmt::skip]
pub const MAP_GROUP_KEY_U8_VALUE: [u8; 41] = [
    1, 0, 0, 0, // 0 Map count
    1, 0, 0, 0, // 4 key group count
    32, 0, 0, 0, 1, 0, 0, 0, // 8 x (u32) pointer
    36, 0, 0, 0, 1, 0, 0, 0, // 16 y (u32) pointer
    40, 0, 0, 0, 1, 0, 0, 0, // 24 u8 value pointer
    10, 0, 0, 0, 20, 0, 0, 0, // 32 u32 data
    1, // 40 u8 data
];
pub const MAP_GROUP_KEY_U8_VALUE_INODES: [Inode; 5] = [
    Inode {
        kind: WireKind::Map,
        kind_meta: KindMeta::Map {
            nesting: 0,
            topology_offset: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 2,
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
            data_offset: 8,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 16,
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
            data_offset: 24,
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
fn test_map_group_key_u8_value() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::Map {
                key: Box::new(PyroType::Group {
                    fields: vec![
                        PyroField {
                            name: "x".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U32,
                            },
                        },
                        PyroField {
                            name: "y".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U32,
                            },
                        },
                    ],
                }),
                value: Box::new(PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                }),
            },
        }],
    };

    let data = PyroValue::Map(
        vec![(
            PyroValue::Group([("x", PyroValue::U32(10)), ("y", PyroValue::U32(20))].into()),
            PyroValue::U8(1),
        )]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &MAP_GROUP_KEY_U8_VALUE,
        &MAP_GROUP_KEY_U8_VALUE_INODES,
    );
}

// Captured writer output for `Map<U8, List<Group<{ x: U32, y: U32 }>>>`.
#[rustfmt::skip]
pub const MAP_U8_KEY_LIST_GROUP_VALUE: [u8; 53] = [
    1, 0, 0, 0, // Map count
    52, 0, 0, 0, 1, 0, 0, 0, // u8 data pointer
    36, 0, 0, 0, 2, 0, 0, 0, // group topology pointer
    44, 0, 0, 0, 1, 0, 0, 0, //
    48, 0, 0, 0, 1, 0, 0, 0, //
    255, 255, 255, 255, 1, 0, 0, 0, //
    10, 0, 0, 0, 20, 0, 0, 0, //
    1, // 48
];

pub const MAP_U8_KEY_LIST_GROUP_VALUE_INODES: [Inode; 5] = [
    Inode {
        kind: WireKind::Map,
        kind_meta: KindMeta::Map {
            nesting: 0,
            topology_offset: 0,
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
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 1,
            count: 2,
            topology_offset: 12,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 20,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 28,
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
fn test_map_u8_key_list_group_value() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::Map {
                key: Box::new(PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                }),
                value: Box::new(PyroType::List {
                    element: Box::new(PyroType::Group {
                        fields: vec![
                            PyroField {
                                name: "x".to_string(),
                                doc: String::new(),
                                data_type: PyroType::PrimitiveScalar {
                                    primitive: PrimitiveDataType::U32,
                                },
                            },
                            PyroField {
                                name: "y".to_string(),
                                doc: String::new(),
                                data_type: PyroType::PrimitiveScalar {
                                    primitive: PrimitiveDataType::U32,
                                },
                            },
                        ],
                    }),
                }),
            },
        }],
    };

    let data = PyroValue::Map(
        vec![(
            PyroValue::U8(1),
            PyroValue::List(vec![PyroValue::Group(
                [("x", PyroValue::U32(10)), ("y", PyroValue::U32(20))].into(),
            )]),
        )]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &MAP_U8_KEY_LIST_GROUP_VALUE,
        &MAP_U8_KEY_LIST_GROUP_VALUE_INODES,
    );
}

// Captured writer output for `Map<List<Group<{ x: U32, y: U32 }>>, U8>`.
#[rustfmt::skip]
pub const MAP_LIST_GROUP_KEY_U8_VALUE: [u8; 53] = [
    1, 0, 0, 0, // 0 Map count
    36, 0, 0, 0, 2, 0, 0, 0, // 4 Group topology pointer
    44, 0, 0, 0, 1, 0, 0, 0, // 12 u32 data pointer
    48, 0, 0, 0, 1, 0, 0, 0, // 20 u32 data pointer
    52, 0, 0, 0, 1, 0, 0, 0, // 28 u8 data pointer
    255, 255, 255, 255, 1, 0, 0, 0, // 36 group topology
    10, 0, 0, 0, 20, 0, 0, 0, // 44 u32 data
    1, // 52 u8 data
];
pub const MAP_LIST_GROUP_KEY_U8_VALUE_INODES: [Inode; 5] = [
    Inode {
        kind: WireKind::Map,
        kind_meta: KindMeta::Map {
            nesting: 0,
            topology_offset: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 1,
            count: 2,
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
            data_offset: 12,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 20,
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
            data_offset: 28,
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
fn test_map_list_group_key_u8_value() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::Map {
                key: Box::new(PyroType::List {
                    element: Box::new(PyroType::Group {
                        fields: vec![
                            PyroField {
                                name: "x".to_string(),
                                doc: String::new(),
                                data_type: PyroType::PrimitiveScalar {
                                    primitive: PrimitiveDataType::U32,
                                },
                            },
                            PyroField {
                                name: "y".to_string(),
                                doc: String::new(),
                                data_type: PyroType::PrimitiveScalar {
                                    primitive: PrimitiveDataType::U32,
                                },
                            },
                        ],
                    }),
                }),
                value: Box::new(PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                }),
            },
        }],
    };

    let data = PyroValue::Map(
        vec![(
            PyroValue::List(vec![PyroValue::Group(
                [("x", PyroValue::U32(10)), ("y", PyroValue::U32(20))].into(),
            )]),
            PyroValue::U8(1),
        )]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &MAP_LIST_GROUP_KEY_U8_VALUE,
        &MAP_LIST_GROUP_KEY_U8_VALUE_INODES,
    );
}

#[rustfmt::skip]
pub const MAP_U8_KEY_LIST_BYTES_VALUE: [u8; 47] = [
    1, 0, 0, 0, // 0 Map count
    44, 0, 0, 0, 1, 0, 0, 0, // 4 u8 key data pointer
    28, 0, 0, 0, 4, 0, 0, 0, // 12 Bytes value topology (outer array span)
    45, 0, 0, 0, 2, 0, 0, 0, // 20 Bytes value payload pointer
    255, 255, 255, 255, 1, 0, 0, 0, // 28 outer per-entry boundary array (addresses)
    255, 255, 255, 255, 2, 0, 0, 0, // 36 inner per-blob byte boundary array
    1, // 44 u8 key data
    10, 20, // 45 payload bytes
];
pub const MAP_U8_KEY_LIST_BYTES_VALUE_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::Map,
        kind_meta: KindMeta::Map {
            nesting: 0,
            topology_offset: 0,
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
        kind: WireKind::Bytes,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 20,
            topology_offset: 12,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_map_u8_key_list_bytes_value() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::Map {
                key: Box::new(PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                }),
                value: Box::new(PyroType::List {
                    element: Box::new(PyroType::Bytes),
                }),
            },
        }],
    };

    let data = PyroValue::Map(
        vec![(
            PyroValue::U8(1),
            PyroValue::List(vec![PyroValue::Bytes(vec![10, 20].into())]),
        )]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &MAP_U8_KEY_LIST_BYTES_VALUE,
        &MAP_U8_KEY_LIST_BYTES_VALUE_INODES,
    );
}

// `Map<List<Bytes>, U8>` — the mirror of `MAP_U8_KEY_LIST_BYTES_VALUE`,
// nested `Vec<Vec<u8>>` on the key side instead. Same HAND-DERIVED caveat
// as above applies — not captured from the real writer, verify by hand.
//
// One map entry, key = `[[10, 20]]`, value = `1u8`. Same byte layout as
// `MAP_U8_KEY_LIST_BYTES_VALUE` numerically (header total size is
// unchanged by which side is Bytes vs u8), just the header slot order
// swaps (key's Bytes topology/payload come first, then the u8 value's
// pointer last) to match this file's Group-key mirror pair
// (`MAP_U8_KEY_LIST_GROUP_VALUE` / `MAP_LIST_GROUP_KEY_U8_VALUE`).
#[rustfmt::skip]
pub const MAP_LIST_BYTES_KEY_U8_VALUE: [u8; 47] = [
    1, 0, 0, 0, // 0 Map count
    28, 0, 0, 0, 4, 0, 0, 0, // 4 Bytes key topology (outer array span)
    44, 0, 0, 0, 2, 0, 0, 0, // 12 Bytes key payload pointer
    46, 0, 0, 0, 1, 0, 0, 0, // 20 u8 value data pointer
    255, 255, 255, 255, 1, 0, 0, 0, // 28 outer per-entry boundary array (addresses)
    255, 255, 255, 255, 2, 0, 0, 0, // 36 inner per-blob byte boundary array
    10, 20, // 44 payload bytes
    1, // 46 u8 value data
];
pub const MAP_LIST_BYTES_KEY_U8_VALUE_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::Map,
        kind_meta: KindMeta::Map {
            nesting: 0,
            topology_offset: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Bytes,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 12,
            topology_offset: 4,
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
            data_offset: 20,
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
fn test_map_list_bytes_key_u8_value() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::Map {
                key: Box::new(PyroType::List {
                    element: Box::new(PyroType::Bytes),
                }),
                value: Box::new(PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                }),
            },
        }],
    };

    let data = PyroValue::Map(
        vec![(
            PyroValue::List(vec![PyroValue::Bytes(vec![10, 20].into())]),
            PyroValue::U8(1),
        )]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &MAP_LIST_BYTES_KEY_U8_VALUE,
        &MAP_LIST_BYTES_KEY_U8_VALUE_INODES,
    );
}

#[rustfmt::skip]
pub const MAP_LIST_BYTES_KEY_LIST_BYTES_VALUE: [u8; 76] = [
    1, 0, 0, 0, // 0 Map count
    36, 0, 0, 0, 4, 0, 0, 0, // 4 Bytes key topology (outer array span)
    72, 0, 0, 0, 1, 0, 0, 0, // 12 Bytes key payload pointer
    52, 0, 0, 0, 5, 0, 0, 0, // 20 Bytes value topology (outer array span)
    73, 0, 0, 0, 3, 0, 0, 0, // 28 Bytes value payload pointer
    255, 255, 255, 255, 1, 0, 0, 0, // 36 key outer per-entry boundary array (addresses)
    255, 255, 255, 255, 1, 0, 0, 0, // 44 key inner per-blob byte boundary array
    255, 255, 255, 255, 2, 0, 0, 0, // 52 value outer per-entry boundary array (addresses)
    255, 255, 255, 255, 1, 0, 0, 0, 3, 0, 0, 0, // 60 value inner per-blob byte boundary array
    10, // 72 key payload byte
    1, 2, 3, // 73 value payload bytes
];
pub const MAP_LIST_BYTES_KEY_LIST_BYTES_VALUE_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::Map,
        kind_meta: KindMeta::Map {
            nesting: 0,
            topology_offset: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Bytes,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 12,
            topology_offset: 4,
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
            data_offset: 28,
            topology_offset: 20,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_map_list_bytes_key_list_bytes_value() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::Map {
                key: Box::new(PyroType::List {
                    element: Box::new(PyroType::Bytes),
                }),
                value: Box::new(PyroType::List {
                    element: Box::new(PyroType::Bytes),
                }),
            },
        }],
    };

    let data = PyroValue::Map(
        vec![(
            PyroValue::List(vec![PyroValue::Bytes(vec![10].into())]),
            PyroValue::List(vec![
                PyroValue::Bytes(vec![1].into()),
                PyroValue::Bytes(vec![2, 3].into()),
            ]),
        )]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &MAP_LIST_BYTES_KEY_LIST_BYTES_VALUE,
        &MAP_LIST_BYTES_KEY_LIST_BYTES_VALUE_INODES,
    );
}

// `{ m: Map<Str, Option<u64>>, z: Option<u8> }`: the map's value column is
// a `(start, valid, total)` slot, not an inline `u64`, so `z` is the last
// fixed field and the validity bitmap follows it.
#[rustfmt::skip]
pub const MAP_STR_OPTION_U64_BESIDE_OPTION_EMPTY: [u8; 40] = [
    0, 0, 0, 0, // 0 map count
    36, 0, 0, 0, 0, 0, 0, 0, // 4 string topology pointer
    40, 0, 0, 0, 0, 0, 0, 0, // 12 string data
    40, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 20 Option u64 pointer (start, valid, total)
    0, // 32 z (null)
    0, // 33 validity bitmap: z absent
    0, 0, // 34 alignment padding (2 bytes)
    0, 0, 0, 0, // 36 alignment padding (4 bytes)
];
#[rustfmt::skip]
pub const MAP_STR_OPTION_U64_BESIDE_OPTION: [u8; 59] = [
    2, 0, 0, 0, // 0 map count
    36, 0, 0, 0, 3, 0, 0, 0, // 4 string topology pointer
    57, 0, 0, 0, 2, 0, 0, 0, // 12 string data
    48, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, // 20 Option u64 pointer (start, valid, total)
    5, // 32 z = 5
    1, // 33 validity bitmap: z present
    0, 0, // 34 alignment padding (2 bytes)
    255, 255, 255, 255, 1, 0, 0, 0, 2, 0, 0, 0, // 36 string topology
    7, 0, 0, 0, 0, 0, 0, 0, // 48 Option u64 data: a = 7
    1, // 56 Option u64 mask: a present, b null
    97, 98, // 57 string data
];
pub const MAP_STR_OPTION_U64_BESIDE_OPTION_INODES: [Inode; 4] = [
    Inode {
        kind: WireKind::Map,
        kind_meta: KindMeta::Map {
            nesting: 0,
            topology_offset: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Str,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 12,
            topology_offset: 4,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::OptionU64,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 20,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::OptionU8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 32,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg(test)]
fn map_str_option_u64_beside_option_schema() -> pyro_schema::human::PyroSchema {
    PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "m".to_string(),
                doc: String::new(),
                data_type: PyroType::Map {
                    key: Box::new(PyroType::Str),
                    value: Box::new(PyroType::OptionPrimitiveScalar {
                        primitive: PrimitiveDataType::U64,
                    }),
                },
            },
            PyroField {
                name: "z".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionPrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                },
            },
        ],
    }
}

#[cfg(test)]
fn map_str_option_u64_beside_option_value(
    m: Vec<(PyroValue<'static>, PyroValue<'static>)>,
    z: PyroValue<'static>,
) -> PyroValue<'static> {
    PyroValue::Group(
        vec![
            ("m".to_string(), PyroValue::Map(m.into())),
            ("z".to_string(), z),
        ]
        .into_iter()
        .collect(),
    )
}

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_map_str_option_u64_beside_option_empty() {
    run_list_wrapped_roundtrip_u32(
        map_str_option_u64_beside_option_schema(),
        map_str_option_u64_beside_option_value(vec![], PyroValue::Null),
        &MAP_STR_OPTION_U64_BESIDE_OPTION_EMPTY,
        &MAP_STR_OPTION_U64_BESIDE_OPTION_INODES,
    );
}

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_map_str_option_u64_beside_option() {
    run_list_wrapped_roundtrip_u32(
        map_str_option_u64_beside_option_schema(),
        map_str_option_u64_beside_option_value(
            vec![
                (PyroValue::Str("a".into()), PyroValue::U64(7)),
                (PyroValue::Str("b".into()), PyroValue::Null),
            ],
            PyroValue::U8(5),
        ),
        &MAP_STR_OPTION_U64_BESIDE_OPTION,
        &MAP_STR_OPTION_U64_BESIDE_OPTION_INODES,
    );
}

// `Map<U8, Option<bool>>` with one present entry: the value column's run is
// one value byte and one mask byte.
#[rustfmt::skip]
pub const MAP_U8_OPTION_BOOL_SINGLE_SOME: [u8; 27] = [
    1, 0, 0, 0, // 0 Map count
    24, 0, 0, 0, 1, 0, 0, 0, // 4 u8 key pointer
    25, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, // 12 Option bool pointer, valid samples, total samples
    1, // 24 keys
    1, // 25 Option bool payload bits = 1
    1, // 26 Option bool mask = 1
];
pub const MAP_U8_OPTION_BOOL_SINGLE_SOME_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::Map,
        kind_meta: KindMeta::Map {
            nesting: 0,
            topology_offset: 0,
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
        kind: WireKind::OptionBool,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 12,
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
fn test_map_u8_option_bool_single_some() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::Map {
                key: Box::new(PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                }),
                value: Box::new(PyroType::OptionPrimitiveScalar {
                    primitive: PrimitiveDataType::Bool,
                }),
            },
        }],
    };

    let data = PyroValue::Map(vec![(PyroValue::U8(1), PyroValue::Bool(true))].into());

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &MAP_U8_OPTION_BOOL_SINGLE_SOME,
        &MAP_U8_OPTION_BOOL_SINGLE_SOME_INODES,
    );
}

// `Map<U8, Map<U8, U8>>`: a map as a map's value.
#[rustfmt::skip]
pub const MAP_U8_KEY_MAP_VALUE: [u8; 47] = [
    1, 0, 0, 0, // 0 outer map count
    44, 0, 0, 0, 1, 0, 0, 0, // 4 outer u8 key pointer
    // The inner map is iterated once per outer entry, so its slot points
    // at a tier of per-map entry counts rather than holding one count.
    36, 0, 0, 0, 2, 0, 0, 0, // 12 inner map topology pointer
    45, 0, 0, 0, 1, 0, 0, 0, // 20 inner u8 key pointer
    46, 0, 0, 0, 1, 0, 0, 0, // 28 inner u8 value pointer
    255, 255, 255, 255, 1, 0, 0, 0, // 36 inner entry counts: one entry
    1, // 44 outer keys
    2, // 45 inner keys
    3, // 46 inner values
];
pub const MAP_U8_KEY_MAP_VALUE_INODES: [Inode; 5] = [
    Inode {
        kind: WireKind::Map,
        kind_meta: KindMeta::Map {
            nesting: 0,
            topology_offset: 0,
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
        kind: WireKind::Map,
        kind_meta: KindMeta::Map {
            nesting: 0,
            topology_offset: 12,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 20,
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
            data_offset: 28,
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
fn test_map_u8_key_map_value() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::Map {
                key: Box::new(PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                }),
                value: Box::new(PyroType::Map {
                    key: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::U8,
                    }),
                    value: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::U8,
                    }),
                }),
            },
        }],
    };

    let data = PyroValue::Map(
        vec![(
            PyroValue::U8(1),
            PyroValue::Map(vec![(PyroValue::U8(2), PyroValue::U8(3))].into()),
        )]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &MAP_U8_KEY_MAP_VALUE,
        &MAP_U8_KEY_MAP_VALUE_INODES,
    );
}
