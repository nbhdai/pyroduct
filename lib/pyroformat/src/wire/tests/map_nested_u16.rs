#[cfg(test)]
use pyro_schema::PrimitiveDataType;
#[cfg(test)]
use pyro_schema::human::{PyroField, PyroSchema, PyroType};

#[cfg(test)]
use crate::{PyroValue, wire::tests::support::run_list_wrapped_roundtrip_u16};

use pyro_schema::{Inode, KindMeta, WireKind};

pub const LIST_MAP_U8_U8: [u8; 24] = [
    12, 0, 3, 0, // 0
    18, 0, 3, 0, // 4
    21, 0, 3, 0, // 8
    255, 255, 2, 0, 3, 0, // 12
    1, 2, 3, // 18
    10, 20, 30, // 21
];
pub const LIST_MAP_U8_U8_INODES: [Inode; 3] = [
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
fn test_list_of_map_u8_u8() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::Map {
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

    let data = PyroValue::List(vec![
        PyroValue::Map(
            vec![
                (PyroValue::U8(1), PyroValue::U8(10)),
                (PyroValue::U8(2), PyroValue::U8(20)),
            ]
            .into(),
        ),
        PyroValue::Map(vec![(PyroValue::U8(3), PyroValue::U8(30))].into()),
    ]);

    run_list_wrapped_roundtrip_u16(schema, data, &LIST_MAP_U8_U8, &LIST_MAP_U8_U8_INODES);
}

pub const LIST_LIST_MAP_U8_U8: [u8; 26] = [
    12, 0, 5, 0, // 0 Map topology pointer
    22, 0, 2, 0, // 4 u8 key pointer
    24, 0, 2, 0, // 8 u8 value pointer
    // A map is a `List<Group<key, value>>`, so a map two lists deep needs
    // both levels of list structure *and* its own per-map entry counts.
    255, 255, 2, 0, // 12 list level: the one inner list holds both maps
    255, 255, 1, 0, 2, 0, // 16 entry counts: one entry per map
    1, 2, // 22 keys
    10, 20, // 24 values
];
pub const LIST_LIST_MAP_U8_U8_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::Map,
        kind_meta: KindMeta::Map {
            nesting: 2,
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
fn test_list_of_list_of_map_u8_u8() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::List {
                    element: Box::new(PyroType::Map {
                        key: Box::new(PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::U8,
                        }),
                        value: Box::new(PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::U8,
                        }),
                    }),
                }),
            },
        }],
    };

    let data = PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::Map(vec![(PyroValue::U8(1), PyroValue::U8(10))].into()),
        PyroValue::Map(vec![(PyroValue::U8(2), PyroValue::U8(20))].into()),
    ])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &LIST_LIST_MAP_U8_U8,
        &LIST_LIST_MAP_U8_U8_INODES,
    );
}

pub const LIST_LIST_LIST_MAP_U8_U8: [u8; 46] = [
    12, 0, 12, 0, // 0 Map topology pointer
    36, 0, 5, 0, // 4 u8 key pointer
    41, 0, 5, 0, // 8 u8 value pointer
    // Layer 1: the two outer elements hold two middle lists, then one
    255, 255, 2, 0, 3, 0, // 12
    // Layer 2: those three middle lists hold two maps, then one, then one
    255, 255, 2, 0, 3, 0, 4, 0, // 18
    // Layer 3: the four maps hold 2, 0, 1 and 2 entries
    255, 255, 2, 0, 2, 0, 3, 0, 5, 0, // 26
    1, 2, 3, 4, 5, // 36 keys
    10, 20, 30, 40, 50, // 41 values
];
pub const LIST_LIST_LIST_MAP_U8_U8_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::Map,
        kind_meta: KindMeta::Map {
            nesting: 3,
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
fn test_list_of_list_of_list_of_map_u8_u8() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::List {
                    element: Box::new(PyroType::List {
                        element: Box::new(PyroType::Map {
                            key: Box::new(PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U8,
                            }),
                            value: Box::new(PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U8,
                            }),
                        }),
                    }),
                }),
            },
        }],
    };

    let data = PyroValue::List(vec![
        PyroValue::List(vec![
            PyroValue::List(vec![
                PyroValue::Map(
                    vec![
                        (PyroValue::U8(1), PyroValue::U8(10)),
                        (PyroValue::U8(2), PyroValue::U8(20)),
                    ]
                    .into(),
                ),
                PyroValue::Map(vec![].into()),
            ]),
            PyroValue::List(vec![PyroValue::Map(
                vec![(PyroValue::U8(3), PyroValue::U8(30))].into(),
            )]),
        ]),
        PyroValue::List(vec![PyroValue::List(vec![PyroValue::Map(
            vec![
                (PyroValue::U8(4), PyroValue::U8(40)),
                (PyroValue::U8(5), PyroValue::U8(50)),
            ]
            .into(),
        )])]),
    ]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &LIST_LIST_LIST_MAP_U8_U8,
        &LIST_LIST_LIST_MAP_U8_U8_INODES,
    );
}

// `List<Group<{ m: Map<U8, U8> }>>`: a map as a field of a listed group.
#[rustfmt::skip]
pub const LIST_GROUP_MAP_U8_U8: [u8; 20] = [
    1, 0, // 0 group row count
    // The map is a field of a listed group, so its slot points at a tier
    // of per-map entry counts.
    14, 0, 2, 0, // 2 map topology pointer
    18, 0, 1, 0, // 6 u8 key pointer
    19, 0, 1, 0, // 10 u8 value pointer
    255, 255, 1, 0, // 14 entry counts: one entry
    1, // 18 keys
    2, // 19 values
];
pub const LIST_GROUP_MAP_U8_U8_INODES: [Inode; 4] = [
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
        kind: WireKind::Map,
        kind_meta: KindMeta::Map {
            nesting: 0,
            topology_offset: 2,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::U8,
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
    Inode {
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 10,
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
fn test_list_of_group_map_u8_u8() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::Group {
                    fields: vec![PyroField {
                        name: "m".to_string(),
                        doc: String::new(),
                        data_type: PyroType::Map {
                            key: Box::new(PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U8,
                            }),
                            value: Box::new(PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U8,
                            }),
                        },
                    }],
                }),
            },
        }],
    };

    let data = PyroValue::List(vec![PyroValue::Group(
        [(
            "m",
            PyroValue::Map(vec![(PyroValue::U8(1), PyroValue::U8(2))].into()),
        )]
        .into(),
    )]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &LIST_GROUP_MAP_U8_U8,
        &LIST_GROUP_MAP_U8_U8_INODES,
    );
}

// `List<Group<{ m: Map<U8, U8> }>>` with no rows.
#[rustfmt::skip]
pub const LIST_GROUP_MAP_U8_U8_EMPTY: [u8; 14] = [
    0, 0, // 0 group row count
    // No maps: the tier is empty, but its pointer is still laid out.
    14, 0, 0, 0, // 2 map topology pointer
    14, 0, 0, 0, // 6 u8 key pointer
    14, 0, 0, 0, // 10 u8 value pointer
];
pub const LIST_GROUP_MAP_U8_U8_EMPTY_INODES: [Inode; 4] = LIST_GROUP_MAP_U8_U8_INODES;

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_list_of_group_map_u8_u8_empty() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::Group {
                    fields: vec![PyroField {
                        name: "m".to_string(),
                        doc: String::new(),
                        data_type: PyroType::Map {
                            key: Box::new(PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U8,
                            }),
                            value: Box::new(PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U8,
                            }),
                        },
                    }],
                }),
            },
        }],
    };

    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &LIST_GROUP_MAP_U8_U8_EMPTY,
        &LIST_GROUP_MAP_U8_U8_EMPTY_INODES,
    );
}

// `List<Map<Group<{ a: U8, s: Str }>, U8>>`: struct keys one list level
// down. The map's slot is a topology pointer, not a count, so the key group
// owns a one-word slot of its own: its instance count, the map's entry total.
#[rustfmt::skip]
pub const LIST_MAP_GROUP_KEY_U8_VALUE: [u8; 33] = [
    22, 0, 2, 0, // 0 Map topology pointer
    1, 0, // 4 key group count
    30, 0, 1, 0, // 6 key.a u8 pointer
    26, 0, 2, 0, // 10 key.s str topology pointer
    31, 0, 1, 0, // 14 key.s str bytes pointer
    32, 0, 1, 0, // 18 u8 value pointer
    255, 255, 1, 0, // 22 entry counts: one entry in the one map
    255, 255, 1, 0, // 26 str ends
    1, // 30 key.a
    97, // 31 key.s: "a"
    1, // 32 values
];
pub const LIST_MAP_GROUP_KEY_U8_VALUE_INODES: [Inode; 5] = [
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
        kind: WireKind::U8,
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
    Inode {
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 18,
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
fn test_list_of_map_group_key_u8_value() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::Map {
                    key: Box::new(PyroType::Group {
                        fields: vec![
                            PyroField {
                                name: "a".to_string(),
                                doc: String::new(),
                                data_type: PyroType::PrimitiveScalar {
                                    primitive: PrimitiveDataType::U8,
                                },
                            },
                            PyroField {
                                name: "s".to_string(),
                                doc: String::new(),
                                data_type: PyroType::Str,
                            },
                        ],
                    }),
                    value: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::U8,
                    }),
                }),
            },
        }],
    };

    let data = PyroValue::List(vec![PyroValue::Map(
        vec![(
            PyroValue::Group([("a", PyroValue::U8(1)), ("s", PyroValue::Str("a".into()))].into()),
            PyroValue::U8(1),
        )]
        .into(),
    )]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &LIST_MAP_GROUP_KEY_U8_VALUE,
        &LIST_MAP_GROUP_KEY_U8_VALUE_INODES,
    );
}

// `List<List<Map<U8, U8>>>` holding `[[]]`.
#[rustfmt::skip]
pub const LIST_LIST_MAP_U8_U8_EMPTY_INNER: [u8; 16] = [
    12, 0, 2, 0, // 0 Map topology pointer
    16, 0, 0, 0, // 4 u8 key pointer
    16, 0, 0, 0, // 8 u8 value pointer
    // The one inner list holds no maps, so the entry-count level below it
    // has no nodes and isn't written: the inode's `nesting: 2`, not the
    // levels present, says how deep the value is.
    255, 255, 0, 0, // 12 list level: the inner list is empty
];
pub const LIST_LIST_MAP_U8_U8_EMPTY_INNER_INODES: [Inode; 3] = LIST_LIST_MAP_U8_U8_INODES;

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_list_of_list_of_map_u8_u8_empty_inner() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::List {
                    element: Box::new(PyroType::Map {
                        key: Box::new(PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::U8,
                        }),
                        value: Box::new(PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::U8,
                        }),
                    }),
                }),
            },
        }],
    };

    let data = PyroValue::List(vec![PyroValue::List(vec![])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &LIST_LIST_MAP_U8_U8_EMPTY_INNER,
        &LIST_LIST_MAP_U8_U8_EMPTY_INNER_INODES,
    );
}
