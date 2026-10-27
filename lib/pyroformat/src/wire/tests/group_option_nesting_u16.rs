#[cfg(test)]
use pyro_schema::{
    PrimitiveDataType,
    human::{PyroField, PyroSchema, PyroType},
};

#[cfg(test)]
use crate::{PyroValue, wire::tests::support::run_list_wrapped_roundtrip_u16};

use pyro_schema::{Inode, KindMeta, WireKind};

pub const LIST_OPT_GROUP: [u8; 11] = [
    8, 0, 3, 0, // 0 Group topology pointer
    9, 0, 2, 0, // 4
    5, // 8 Validity map
    100, 42, // u8 data
];
pub const LIST_OPT_GROUP_INODE: [Inode; 2] = [
    Inode {
        kind: WireKind::OptionGroup,
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
            data_offset: 4,
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
fn test_list_of_opt_group() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::OptionGroup {
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
        PyroValue::Null,
        PyroValue::Group([("y", PyroValue::from(42u8))].into()),
    ]);

    run_list_wrapped_roundtrip_u16(schema, data, &LIST_OPT_GROUP, &LIST_OPT_GROUP_INODE);
}

// 40 elements is the first size where the validity mask needs a second
// `u32` word (bit 32 onwards), so this is the fixture that pins down how the
// mask is sized and where the payload lands behind it: the mask is 5 bytes
// at 8..13, and `y`'s payload starts right behind it at 13.
//
// Every fifth element is absent (indices 4, 9, … 39), so 32 of the 40 slots
// carry a `y` — the nulls take a bit and a topology slot but no payload byte.
// Word 0 is `0xdef7bdef` (bits 4, 9, 14, 19, 24, 29 clear), word 1 is `0x7b`
// (bits 34 and 39 clear, i.e. bits 2 and 7 of the word).
pub const LIST_OPT_GROUP_TWO_VALIDITY_WORDS: [u8; 45] = [
    8, 0, 40, 0, // 0 Group topology pointer
    13, 0, 32, 0, // 4 "y" payload pointer
    239, 189, 247, 222, // 8 Validity map, word 0 (bits 0..32)
    123, // 12 Validity map, word 1 (bits 32..40)
    0, 1, 2, 3, 5, 6, 7, 8, // 24
    10, 11, 12, 13, 15, 16, 17, 18, // 32
    20, 21, 22, 23, 25, 26, 27, 28, // 40
    30, 31, 32, 33, 35, 36, 37, 38, // 48
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_list_of_opt_group_two_validity_words() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::OptionGroup {
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

    let data = PyroValue::List(
        (0..40u8)
            .map(|i| {
                if i % 5 == 4 {
                    PyroValue::Null
                } else {
                    PyroValue::Group([("y", PyroValue::from(i))].into())
                }
            })
            .collect(),
    );

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &LIST_OPT_GROUP_TWO_VALIDITY_WORDS,
        &LIST_OPT_GROUP_INODE,
    );
}

pub const LIST_GROUP_OPT_PRIM_ALL_SOME: [u8; 23] = [
    2, 0, // 0 Group element count
    20, 0, 2, 0, 2, 0, // 2 Option I8 pointer, count, validity count
    12, 0, 2, 0, // 8 F32 pointer
    195, 245, 72, 64, // 12 3.14
    0, 0, 192, 63, // 16 1.5
    156, 42, // 20 Some(-100), Some(42)
    3,  // 22 both present
];
pub const LIST_GROUP_OPT_PRIM_ALL_SOME_INODES: [Inode; 3] = [
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
        kind: WireKind::OptionI8,
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
        kind: WireKind::F32,
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
fn test_list_of_group_option_primitive_all_some() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::Group {
                    fields: vec![
                        PyroField {
                            name: "x".to_string(),
                            doc: String::new(),
                            data_type: PyroType::OptionPrimitiveScalar {
                                primitive: PrimitiveDataType::I8,
                            },
                        },
                        PyroField {
                            name: "y".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::F32,
                            },
                        },
                    ],
                }),
            },
        }],
    };

    let data = PyroValue::List(vec![
        PyroValue::Group(
            [
                ("x", PyroValue::from(Some(-100i8))),
                ("y", PyroValue::from(3.14f32)),
            ]
            .into(),
        ),
        PyroValue::Group(
            [
                ("x", PyroValue::from(Some(42i8))),
                ("y", PyroValue::from(1.5f32)),
            ]
            .into(),
        ),
    ]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &LIST_GROUP_OPT_PRIM_ALL_SOME,
        &LIST_GROUP_OPT_PRIM_ALL_SOME_INODES,
    );
}

pub const LIST_GROUP_OPT_PRIM_MIXED_NONE: [u8; 22] = [
    2, 0, // 0 Group element count
    20, 0, 1, 0, 2, 0, // 2 Option I8 pointer, count, validity count
    12, 0, 2, 0, // 8 F32 pointer
    195, 245, 72, 64, // 12 3.14
    0, 0, 192, 63,  // 16 1.5
    156, // 20 Some(-100); the second element is absent, so it stores nothing
    1,   // 21 first present, second absent
];
pub const LIST_GROUP_OPT_PRIM_MIXED_NONE_INODES: [Inode; 3] = [
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
        kind: WireKind::OptionI8,
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
        kind: WireKind::F32,
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
fn test_list_of_group_option_primitive_mixed_none() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::Group {
                    fields: vec![
                        PyroField {
                            name: "x".to_string(),
                            doc: String::new(),
                            data_type: PyroType::OptionPrimitiveScalar {
                                primitive: PrimitiveDataType::I8,
                            },
                        },
                        PyroField {
                            name: "y".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::F32,
                            },
                        },
                    ],
                }),
            },
        }],
    };

    let data = PyroValue::List(vec![
        PyroValue::Group(
            [
                ("x", PyroValue::from(Some(-100i8))),
                ("y", PyroValue::from(3.14f32)),
            ]
            .into(),
        ),
        PyroValue::Group(
            [
                ("x", PyroValue::from(None::<i8>)),
                ("y", PyroValue::from(1.5f32)),
            ]
            .into(),
        ),
    ]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &LIST_GROUP_OPT_PRIM_MIXED_NONE,
        &LIST_GROUP_OPT_PRIM_MIXED_NONE_INODES,
    );
}

pub const LIST_GROUP_OPTION_GROUP_PRESENT: [u8; 31] = [
    1, 0, // 0 Group element count
    28, 0, 1, 0, // 2 u8 pointer
    // 12 Option group slot: a pointer to its validity bitmap, and how many
    // elements that bitmap covers
    20, 0, 1, 0, // 6
    29, 0, 1, 0, 1, 0, // 10 Option<i8> pointer
    24, 0, 1, 0, // 16 Float pointer
    1, 0, 0, 0, // 20 Option group validity: present
    195, 245, 72, 64,  // 24 f32 3.14
    200, // 28 u8 200
    156, 1, // 29 i8 -100 with validity 1
];
pub const LIST_GROUP_OPTION_GROUP_PRESENT_INODES: [Inode; 5] = [
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
    Inode {
        kind: WireKind::OptionGroup,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 2,
            topology_offset: 6,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::OptionI8,
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
    Inode {
        kind: WireKind::F32,
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
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_list_of_group_option_group_present() {
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
                                primitive: PrimitiveDataType::U8,
                            },
                        },
                        PyroField {
                            name: "c".to_string(),
                            doc: String::new(),
                            data_type: PyroType::OptionGroup {
                                fields: vec![
                                    PyroField {
                                        name: "x".to_string(),
                                        doc: String::new(),
                                        data_type: PyroType::OptionPrimitiveScalar {
                                            primitive: PrimitiveDataType::I8,
                                        },
                                    },
                                    PyroField {
                                        name: "y".to_string(),
                                        doc: String::new(),
                                        data_type: PyroType::PrimitiveScalar {
                                            primitive: PrimitiveDataType::F32,
                                        },
                                    },
                                ],
                            },
                        },
                    ],
                }),
            },
        }],
    };

    let data = PyroValue::List(vec![PyroValue::Group(
        [
            ("a", PyroValue::from(200u8)),
            (
                "c",
                PyroValue::from(Some(PyroValue::Group(
                    [
                        ("x", PyroValue::from(Some(-100i8))),
                        ("y", PyroValue::from(3.14f32)),
                    ]
                    .into(),
                ))),
            ),
        ]
        .into(),
    )]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &LIST_GROUP_OPTION_GROUP_PRESENT,
        &LIST_GROUP_OPTION_GROUP_PRESENT_INODES,
    );
}

pub const LIST_GROUP_OPTION_GROUP_ABSENT: [u8; 25] = [
    1, 0, // 0 Group element count
    24, 0, 1, 0, // 2 u8 pointer
    20, 0, 1, 0, // 6 Option group: validity pointer, element count
    25, 0, 0, 0, 0, 0, // 10 Option<i8> pointer — nothing stored
    24, 0, 0, 0, // 16 Float pointer — nothing stored
    0, 0, 0, 0,   // 20 Option group validity: absent
    200, // 24 u8 200
];
pub const LIST_GROUP_OPTION_GROUP_ABSENT_INODES: [Inode; 5] = [
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
    Inode {
        kind: WireKind::OptionGroup,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 2,
            topology_offset: 6,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::OptionI8,
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
    Inode {
        kind: WireKind::F32,
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
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_list_of_group_option_group_absent() {
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
                                primitive: PrimitiveDataType::U8,
                            },
                        },
                        PyroField {
                            name: "c".to_string(),
                            doc: String::new(),
                            data_type: PyroType::OptionGroup {
                                fields: vec![
                                    PyroField {
                                        name: "x".to_string(),
                                        doc: String::new(),
                                        data_type: PyroType::OptionPrimitiveScalar {
                                            primitive: PrimitiveDataType::I8,
                                        },
                                    },
                                    PyroField {
                                        name: "y".to_string(),
                                        doc: String::new(),
                                        data_type: PyroType::PrimitiveScalar {
                                            primitive: PrimitiveDataType::F32,
                                        },
                                    },
                                ],
                            },
                        },
                    ],
                }),
            },
        }],
    };

    let data = PyroValue::List(vec![PyroValue::Group(
        [("a", PyroValue::from(200u8)), ("c", PyroValue::Null)].into(),
    )]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &LIST_GROUP_OPTION_GROUP_ABSENT,
        &LIST_GROUP_OPTION_GROUP_ABSENT_INODES,
    );
}

pub const LIST_GROUP_OPTION_GROUP_MIXED: [u8; 31] = [
    2, 0, // 0 Group element count
    28, 0, 2, 0, // 2 u8 pointer
    20, 0, 2, 0, // 6 Option group: validity pointer, element count
    30, 0, 0, 0, 1, 0, // 10 Option<i8> pointer
    24, 0, 1, 0, // 16 Float pointer
    1, 0, 0, 0, // 20 Option group validity: present, then absent
    195, 245, 72, 64, // 24 f32 3.14
    200, 1, // 28 the two u8s
    0, // 30 the present group's `x` is itself None
];
pub const LIST_GROUP_OPTION_GROUP_MIXED_INODES: [Inode; 5] = [
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
    Inode {
        kind: WireKind::OptionGroup,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 2,
            topology_offset: 6,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::OptionI8,
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
    Inode {
        kind: WireKind::F32,
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
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_list_of_group_option_group_mixed_present_absent() {
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
                                primitive: PrimitiveDataType::U8,
                            },
                        },
                        PyroField {
                            name: "c".to_string(),
                            doc: String::new(),
                            data_type: PyroType::OptionGroup {
                                fields: vec![
                                    PyroField {
                                        name: "x".to_string(),
                                        doc: String::new(),
                                        data_type: PyroType::OptionPrimitiveScalar {
                                            primitive: PrimitiveDataType::I8,
                                        },
                                    },
                                    PyroField {
                                        name: "y".to_string(),
                                        doc: String::new(),
                                        data_type: PyroType::PrimitiveScalar {
                                            primitive: PrimitiveDataType::F32,
                                        },
                                    },
                                ],
                            },
                        },
                    ],
                }),
            },
        }],
    };

    let data = PyroValue::List(vec![
        PyroValue::Group(
            [
                ("a", PyroValue::from(200u8)),
                (
                    "c",
                    PyroValue::from(Some(PyroValue::Group(
                        [
                            ("x", PyroValue::from(None::<i8>)),
                            ("y", PyroValue::from(3.14f32)),
                        ]
                        .into(),
                    ))),
                ),
            ]
            .into(),
        ),
        PyroValue::Group([("a", PyroValue::from(1u8)), ("c", PyroValue::Null)].into()),
    ]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &LIST_GROUP_OPTION_GROUP_MIXED,
        &LIST_GROUP_OPTION_GROUP_MIXED_INODES,
    );
}

pub const LIST_GROUP_DOUBLE_NESTED_OPTION_GROUP: [u8; 42] = [
    2, 0, // 0 Group element count
    36, 0, 2, 0, // 2 "a" (u8) pointer
    30, 0, 2, 0, // 6 "b" option group: validity pointer, element count
    38, 0, 1, 0, 1, 0, // 10 "x" Option<i8> pointer
    32, 0, 1, 0, // 16 "y" option group: validity pointer, element count
    34, 0, 1, 0, // 20 "p" (u16) pointer
    40, 0, 1, 0, 1, 0, // 24 "q" Option<bool> pointer
    1, 0, // 30 "b" validity: present, then absent
    1, 0, // 32 "y" validity: present
    44, 1, // 34 u16 300
    10, 30,  // 36 the two "a" values
    251, // 38 i8 -5
    1,   // 39 "x" validity
    1,   // 40 "q" = true
    1,   // 41 "q" validity
];
pub const LIST_GROUP_DOUBLE_NESTED_OPTION_GROUP_INODES: [Inode; 7] = [
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
    Inode {
        kind: WireKind::OptionGroup,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 2,
            topology_offset: 6,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::OptionI8,
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
    Inode {
        kind: WireKind::OptionGroup,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 2,
            topology_offset: 16,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::U16,
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
        kind: WireKind::OptionBool,
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
fn test_list_of_group_double_nested_option_group() {
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
                                primitive: PrimitiveDataType::U8,
                            },
                        },
                        PyroField {
                            name: "b".to_string(),
                            doc: String::new(),
                            data_type: PyroType::OptionGroup {
                                fields: vec![
                                    PyroField {
                                        name: "x".to_string(),
                                        doc: String::new(),
                                        data_type: PyroType::OptionPrimitiveScalar {
                                            primitive: PrimitiveDataType::I8,
                                        },
                                    },
                                    PyroField {
                                        name: "y".to_string(),
                                        doc: String::new(),
                                        data_type: PyroType::OptionGroup {
                                            fields: vec![
                                                PyroField {
                                                    name: "p".to_string(),
                                                    doc: String::new(),
                                                    data_type: PyroType::PrimitiveScalar {
                                                        primitive: PrimitiveDataType::U16,
                                                    },
                                                },
                                                PyroField {
                                                    name: "q".to_string(),
                                                    doc: String::new(),
                                                    data_type: PyroType::OptionPrimitiveScalar {
                                                        primitive: PrimitiveDataType::Bool,
                                                    },
                                                },
                                            ],
                                        },
                                    },
                                ],
                            },
                        },
                    ],
                }),
            },
        }],
    };

    let data = PyroValue::List(vec![
        PyroValue::Group(
            [
                ("a", PyroValue::from(10u8)),
                (
                    "b",
                    PyroValue::from(Some(PyroValue::Group(
                        [
                            ("x", PyroValue::from(Some(-5i8))),
                            (
                                "y",
                                PyroValue::from(Some(PyroValue::Group(
                                    [
                                        ("p", PyroValue::from(300u16)),
                                        ("q", PyroValue::from(Some(true))),
                                    ]
                                    .into(),
                                ))),
                            ),
                        ]
                        .into(),
                    ))),
                ),
            ]
            .into(),
        ),
        PyroValue::Group([("a", PyroValue::from(30u8)), ("b", PyroValue::Null)].into()),
    ]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &LIST_GROUP_DOUBLE_NESTED_OPTION_GROUP,
        &LIST_GROUP_DOUBLE_NESTED_OPTION_GROUP_INODES,
    );
}

// NOTE: `OptionTimestamp` (and plain `Timestamp`) inside a list-nested
// `Group` is not currently supported by the writer: `create_list_writer`
// in `src/wire/writers.rs` has no match arm for `WireKind::Timestamp` /
// `WireKind::OptionTimestamp` (only the `Bool`/integer/float kinds), so
// `ValueWriter::new` returns `Err(WireError::InvalidInodes)` for that
// schema shape — confirmed by actually running it. `OptionF64` (the
// widest-aligned kind that *is* supported in this position) stands in for
// it here instead.
pub const LIST_GROUP_OPTION_F64_BOOL: [u8; 26] = [
    14, 0, 3, 0, // 0 validity pointer, element count
    16, 0, 1, 0, 2, 0, // 4 option_f64 pointer
    25, 0, 2, 0, // 10 bool pointer
    5, 0, // 14 validity: present, absent, present
    155, 145, 4, 139, 10, 191, 5, 64, // 16 2.718281828
    1,  // 24 validity of the f64
    2,  // 48 the two bools: false, true
];
pub const LIST_GROUP_OPTION_F64_BOOL_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::OptionGroup,
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
        kind: WireKind::OptionF64,
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
        kind: WireKind::Bool,
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
// (`OptionF64` here takes 6 bytes as a group-list topology item — not the
// flat 4-byte pointer pair every other `Option*` case in this file uses —
// so `Bool` lands at `data_offset: 10`; captured from the real writer's
// own output, not independently re-derived.)

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_list_of_group_option_f64_bool_some_none() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::OptionGroup {
                    fields: vec![
                        PyroField {
                            name: "t".to_string(),
                            doc: String::new(),
                            data_type: PyroType::OptionPrimitiveScalar {
                                primitive: PrimitiveDataType::F64,
                            },
                        },
                        PyroField {
                            name: "u".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::Bool,
                            },
                        },
                    ],
                }),
            },
        }],
    };

    let data = PyroValue::List(vec![
        PyroValue::Group(
            [
                ("t", PyroValue::from(Some(2.718281828f64))),
                ("u", PyroValue::from(false)),
            ]
            .into(),
        ),
        PyroValue::Null,
        PyroValue::Group(
            [
                ("t", PyroValue::from(None::<f64>)),
                ("u", PyroValue::from(true)),
            ]
            .into(),
        ),
    ]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &LIST_GROUP_OPTION_F64_BOOL,
        &LIST_GROUP_OPTION_F64_BOOL_INODES,
    );
}

pub const EMPTY_LIST_GROUP_OPTION: [u8; 10] = [
    // An empty `List<Option<Group<..>>>`: a count of zero, and a validity
    // bitmap that still gets its (empty) word so the read side needs no
    // special case.
    10, 0, 0, 0, // 0 validity pointer, element count
    10, 0, 0, 0, 0, 0, // 4 "y" pointer, count, validity count
];
pub const EMPTY_LIST_GROUP_OPTION_INODES: [Inode; 2] = [
    Inode {
        kind: WireKind::OptionGroup,
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
        kind: WireKind::OptionU8,
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
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_empty_list_of_group_option() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::OptionGroup {
                    fields: vec![PyroField {
                        name: "x".to_string(),
                        doc: String::new(),
                        data_type: PyroType::OptionPrimitiveScalar {
                            primitive: PrimitiveDataType::U8,
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
        &EMPTY_LIST_GROUP_OPTION,
        &EMPTY_LIST_GROUP_OPTION_INODES,
    );
}

pub const DOUBLE_LIST_GROUP_OPTION: [u8; 35] = [
    16, 0, 2, 0, // 0 topology pointer — two levels, so a (start, end) pair
    33, 0, 1, 0, 2, 0, // 4 Option<i8> pointer
    24, 0, 2, 0, 2, 0, // 10 f32 pointer
    255, 255, 3, 0, // 16 topology: one inner list covering three groups
    5, 0, 0, 0, // 20 group validity: present, absent, present
    195, 245, 72, 64, // 24 3.14
    0, 0, 192, 63, // 28 1.5
    3,  // 32 validity of the i8s
    156, 1, // 56 Some(-100), then the second group's own value
];
pub const DOUBLE_LIST_GROUP_OPTION_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::OptionGroup,
        kind_meta: KindMeta::Group {
            nesting: 2,
            count: 2,
            topology_offset: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionI8,
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
        kind: WireKind::OptionF32,
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
fn test_double_list_of_group_option() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::List {
                    element: Box::new(PyroType::OptionGroup {
                        fields: vec![
                            PyroField {
                                name: "x".to_string(),
                                doc: String::new(),
                                data_type: PyroType::OptionPrimitiveScalar {
                                    primitive: PrimitiveDataType::I8,
                                },
                            },
                            PyroField {
                                name: "y".to_string(),
                                doc: String::new(),
                                data_type: PyroType::OptionPrimitiveScalar {
                                    primitive: PrimitiveDataType::F32,
                                },
                            },
                        ],
                    }),
                }),
            },
        }],
    };

    let data = PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::Group(
            [
                ("x", PyroValue::from(Some(-100i8))),
                ("y", PyroValue::from(Some(3.14f32))),
            ]
            .into(),
        ),
        PyroValue::Null,
        PyroValue::Group(
            [
                ("x", PyroValue::from(None::<i8>)),
                ("y", PyroValue::from(Some(1.5f32))),
            ]
            .into(),
        ),
    ])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &DOUBLE_LIST_GROUP_OPTION,
        &DOUBLE_LIST_GROUP_OPTION_INODES,
    );
}

pub const TRIPLE_LIST_GROUP_OPTION: [u8; 31] = [
    16, 0, 4, 0, // 0 topology pointer — three levels, so a (start, end) pair
    29, 0, 1, 0, 2, 0, // 4 Option<u8> pointer
    26, 0, 1, 0, 2, 0, // 10 Option<u16> pointer
    255, 255, 1, 0, // 16 topology level 1
    255, 255, 3, 0, // 20 topology level 2: three groups in the one inner list
    5, 0, // 24 group validity: present, absent, present
    44, 1,  // 26 u16 300
    2,  // 28 its validity, then padding
    10, // 29 u8 10
    2,  // 57 its validity
];
pub const TRIPLE_LIST_GROUP_OPTION_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::OptionGroup,
        kind_meta: KindMeta::Group {
            nesting: 3,
            count: 2,
            topology_offset: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionU8,
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
        kind: WireKind::OptionU16,
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
fn test_triple_list_of_group_option() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::List {
                element: Box::new(PyroType::List {
                    element: Box::new(PyroType::List {
                        element: Box::new(PyroType::OptionGroup {
                            fields: vec![
                                PyroField {
                                    name: "a".to_string(),
                                    doc: String::new(),
                                    data_type: PyroType::OptionPrimitiveScalar {
                                        primitive: PrimitiveDataType::U8,
                                    },
                                },
                                PyroField {
                                    name: "b".to_string(),
                                    doc: String::new(),
                                    data_type: PyroType::OptionPrimitiveScalar {
                                        primitive: PrimitiveDataType::U16,
                                    },
                                },
                            ],
                        }),
                    }),
                }),
            },
        }],
    };

    let data = PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
        PyroValue::Group(
            [
                ("a", PyroValue::from(None::<u8>)),
                ("b", PyroValue::from(None::<u16>)),
            ]
            .into(),
        ),
        PyroValue::Null,
        PyroValue::Group(
            [
                ("a", PyroValue::from(Some(10u8))),
                ("b", PyroValue::from(Some(300u16))),
            ]
            .into(),
        ),
    ])])]);

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &TRIPLE_LIST_GROUP_OPTION,
        &TRIPLE_LIST_GROUP_OPTION_INODES,
    );
}

// `Option<Group { a: Vec<Option<Group { x: u8 }>> }>`: the outer group's
// presence is a bit in the fixed-region validity bitmap, and everything in
// front of that bitmap is a word-sized slot — `a`'s `(mask start, slots)`
// and `x`'s `(start, count)` — with no fixed value after them. So the bitmap
// starts at the end of the topology section, 8, not at the end of `x`'s
// own element width.
pub const OPT_GROUP_LIST_OPT_GROUP: [u8; 13] = [
    10, 0, 3, 0, // 0 "a" validity pointer, element count
    11, 0, 2, 0, // 4 "x" pointer, count
    1, // 8 validity bitmap: "o" present
    0, // 9 padding to the word
    5, // 10 "a" validity mask: slots 0 and 2 present
    100, 42, // 11 "x" data
];
pub const OPT_GROUP_LIST_OPT_GROUP_ABSENT: [u8; 10] = [
    10, 0, 0, 0, // 0 "a" validity pointer, element count
    10, 0, 0, 0, // 4 "x" pointer, count
    0, // 8 validity bitmap: "o" absent
    0, // 9 padding to the word
];
pub const OPT_GROUP_LIST_OPT_GROUP_INODES: [Inode; 3] = [
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
        kind: WireKind::OptionGroup,
        kind_meta: KindMeta::Group {
            nesting: 1,
            count: 1,
            topology_offset: 0,
            nullable_bit_index: 0,
            group: true,
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
];

#[cfg(test)]
fn opt_group_list_opt_group_schema() -> pyro_schema::human::PyroSchema {
    PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "o".to_string(),
            doc: String::new(),
            data_type: PyroType::OptionGroup {
                fields: vec![PyroField {
                    name: "a".to_string(),
                    doc: String::new(),
                    data_type: PyroType::List {
                        element: Box::new(PyroType::OptionGroup {
                            fields: vec![PyroField {
                                name: "x".to_string(),
                                doc: String::new(),
                                data_type: PyroType::PrimitiveScalar {
                                    primitive: PrimitiveDataType::U8,
                                },
                            }],
                        }),
                    },
                }],
            },
        }],
    }
}

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_opt_group_of_list_of_opt_group() {
    let data = PyroValue::Group(
        [(
            "a",
            PyroValue::List(vec![
                PyroValue::Group([("x", PyroValue::from(100u8))].into()),
                PyroValue::Null,
                PyroValue::Group([("x", PyroValue::from(42u8))].into()),
            ]),
        )]
        .into(),
    );

    run_list_wrapped_roundtrip_u16(
        opt_group_list_opt_group_schema(),
        data,
        &OPT_GROUP_LIST_OPT_GROUP,
        &OPT_GROUP_LIST_OPT_GROUP_INODES,
    );
}

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_opt_group_of_list_of_opt_group_absent() {
    run_list_wrapped_roundtrip_u16(
        opt_group_list_opt_group_schema(),
        PyroValue::Null,
        &OPT_GROUP_LIST_OPT_GROUP_ABSENT,
        &OPT_GROUP_LIST_OPT_GROUP_INODES,
    );
}
