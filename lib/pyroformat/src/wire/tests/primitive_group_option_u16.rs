#[cfg(test)]
use half::f16;
#[cfg(test)]
use pyro_schema::{
    PrimitiveDataType,
    human::{PyroField, PyroSchema, PyroType},
};

#[cfg(test)]
use crate::{PyroValue, value::Time, wire::tests::support::run_list_wrapped_roundtrip_u16};

use pyro_schema::{Inode, KindMeta, WireKind};

// This file mixes `Group` (see primitive_group.rs) with `Option` (see
// primitive_option.rs). Every `Option` field, wherever it lives in the
// schema (top-level group or nested group), shares one global validity
// bitvec and one global `nullable_bit_index` counter (wiki/04-layout-and-
// alignment 4.6, `layout_nullable_section`) — the counter walks the flat
// inode array in declaration/pre-order-DFS order, *not* by data_offset. A
// `Group`/`OptionGroup` with `nesting == 0` still contributes no
// alignment/size of its own, so plain fields and nested-group fields
// bucket-sort together by descending alignment (16 -> 1), tying by
// ascending inode index within a class, exactly as in primitive_group.rs.
// A nested `Option<Group>` (`OptionGroup`) itself takes zero fixed-region
// bytes but claims one bit in the shared validity bitvec (wiki/07-groups-
// and-maps 7.3.4). `expected_buf`/`expected_inodes` are computed
// independently from the production code, following those clauses.

pub const GROUP_OPTION_I8_F32_U8_SOME_SOME: [u8; 7] = [195, 245, 72, 64, 156, 200, 3];
pub const GROUP_OPTION_I8_F32_U8_SOME_SOME_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::OptionI8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 4,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::F32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 0,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionU8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 5,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 1,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_option_i8_f32_u8_some_some() {
    let schema = PyroSchema {
        doc: String::new(),
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
            PyroField {
                name: "z".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionPrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("x", PyroValue::from(Some(-100i8))),
            ("y", PyroValue::from(3.14f32)),
            ("z", PyroValue::from(Some(200u8))),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &GROUP_OPTION_I8_F32_U8_SOME_SOME,
        &GROUP_OPTION_I8_F32_U8_SOME_SOME_INODES,
    );
}

pub const GROUP_OPTION_I8_F32_U8_NONE_SOME: [u8; 7] = [195, 245, 72, 64, 0, 200, 2];
pub const GROUP_OPTION_I8_F32_U8_NONE_SOME_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::OptionI8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 4,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::F32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 0,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionU8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 5,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 1,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_option_i8_f32_u8_none_some() {
    let schema = PyroSchema {
        doc: String::new(),
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
            PyroField {
                name: "z".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionPrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("x", PyroValue::from(None::<i8>)),
            ("y", PyroValue::from(3.14f32)),
            ("z", PyroValue::from(Some(200u8))),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &GROUP_OPTION_I8_F32_U8_NONE_SOME,
        &GROUP_OPTION_I8_F32_U8_NONE_SOME_INODES,
    );
}

pub const GROUP_OPTION_I8_F32_U8_SOME_NONE: [u8; 7] = [195, 245, 72, 64, 156, 0, 1];
pub const GROUP_OPTION_I8_F32_U8_SOME_NONE_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::OptionI8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 4,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::F32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 0,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionU8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 5,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 1,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_option_i8_f32_u8_some_none() {
    let schema = PyroSchema {
        doc: String::new(),
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
            PyroField {
                name: "z".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionPrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("x", PyroValue::from(Some(-100i8))),
            ("y", PyroValue::from(3.14f32)),
            ("z", PyroValue::from(None::<u8>)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &GROUP_OPTION_I8_F32_U8_SOME_NONE,
        &GROUP_OPTION_I8_F32_U8_SOME_NONE_INODES,
    );
}

pub const GROUP_OPTION_I8_F32_U8_NONE_NONE: [u8; 7] = [195, 245, 72, 64, 0, 0, 0];
pub const GROUP_OPTION_I8_F32_U8_NONE_NONE_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::OptionI8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 4,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::F32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 0,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionU8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 5,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 1,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_option_i8_f32_u8_none_none() {
    let schema = PyroSchema {
        doc: String::new(),
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
            PyroField {
                name: "z".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionPrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("x", PyroValue::from(None::<i8>)),
            ("y", PyroValue::from(3.14f32)),
            ("z", PyroValue::from(None::<u8>)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &GROUP_OPTION_I8_F32_U8_NONE_NONE,
        &GROUP_OPTION_I8_F32_U8_NONE_NONE_INODES,
    );
}

pub const GROUP_OPTION_U32_U8_U16_ALL_SOME: [u8; 8] = [64, 226, 1, 0, 49, 212, 200, 7];
pub const GROUP_OPTION_U32_U8_U16_ALL_SOME_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::OptionU32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 0,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionU8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 6,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 1,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionU16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 4,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 2,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_option_u32_u8_u16_all_some() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "p".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionPrimitiveScalar {
                    primitive: PrimitiveDataType::U32,
                },
            },
            PyroField {
                name: "q".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionPrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                },
            },
            PyroField {
                name: "r".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionPrimitiveScalar {
                    primitive: PrimitiveDataType::U16,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("p", PyroValue::from(Some(123456u32))),
            ("q", PyroValue::from(Some(200u8))),
            ("r", PyroValue::from(Some(54321u16))),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &GROUP_OPTION_U32_U8_U16_ALL_SOME,
        &GROUP_OPTION_U32_U8_U16_ALL_SOME_INODES,
    );
}

pub const GROUP_OPTION_U32_U8_U16_ALL_NONE: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];
pub const GROUP_OPTION_U32_U8_U16_ALL_NONE_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::OptionU32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 0,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionU8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 6,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 1,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionU16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 4,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 2,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_option_u32_u8_u16_all_none() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "p".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionPrimitiveScalar {
                    primitive: PrimitiveDataType::U32,
                },
            },
            PyroField {
                name: "q".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionPrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                },
            },
            PyroField {
                name: "r".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionPrimitiveScalar {
                    primitive: PrimitiveDataType::U16,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("p", PyroValue::from(None::<u32>)),
            ("q", PyroValue::from(None::<u8>)),
            ("r", PyroValue::from(None::<u16>)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &GROUP_OPTION_U32_U8_U16_ALL_NONE,
        &GROUP_OPTION_U32_U8_U16_ALL_NONE_INODES,
    );
}

pub const GROUP_OPTION_TIMESTAMP_U8_I16_SOME_SOME: [u8; 20] = [
    255, 201, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 199, 207, 200, 3,
];
pub const GROUP_OPTION_TIMESTAMP_U8_I16_SOME_SOME_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::OptionTimestamp,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 0,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
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
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionI16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 16,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 1,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_option_timestamp_u8_i16_some_some() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "t".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionPrimitiveScalar {
                    primitive: PrimitiveDataType::Timestamp,
                },
            },
            PyroField {
                name: "u".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                },
            },
            PyroField {
                name: "v".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionPrimitiveScalar {
                    primitive: PrimitiveDataType::I16,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("t", PyroValue::from(Some(Time(999_999_999)))),
            ("u", PyroValue::from(200u8)),
            ("v", PyroValue::from(Some(-12345i16))),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &GROUP_OPTION_TIMESTAMP_U8_I16_SOME_SOME,
        &GROUP_OPTION_TIMESTAMP_U8_I16_SOME_SOME_INODES,
    );
}

pub const GROUP_OPTION_TIMESTAMP_U8_I16_NONE_NONE: [u8; 20] =
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 200, 0];
pub const GROUP_OPTION_TIMESTAMP_U8_I16_NONE_NONE_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::OptionTimestamp,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 0,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
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
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionI16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 16,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 1,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_option_timestamp_u8_i16_none_none() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "t".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionPrimitiveScalar {
                    primitive: PrimitiveDataType::Timestamp,
                },
            },
            PyroField {
                name: "u".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                },
            },
            PyroField {
                name: "v".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionPrimitiveScalar {
                    primitive: PrimitiveDataType::I16,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("t", PyroValue::from(None::<Time>)),
            ("u", PyroValue::from(200u8)),
            ("v", PyroValue::from(None::<i16>)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &GROUP_OPTION_TIMESTAMP_U8_I16_NONE_NONE,
        &GROUP_OPTION_TIMESTAMP_U8_I16_NONE_NONE_INODES,
    );
}

// --- Nested `Option<Group>`: `Group { i8, Option<f16>, Option<Group {
// Option<i8>, f32, Option<u8> }> }`. The inode array is the pre-order DFS
// walk: outer Group, a, b, c (the OptionGroup), then c's own children
// (x, y, z) immediately after it and before any further outer-group
// siblings (there are none here) — see
// `structurally_nested_group_without_list_stays_in_fixed_region` in
// pyro-schema's layout tests. `c` itself claims one bit in the shared
// bitvec (wiki/07-groups-and-maps 7.3.4) but no fixed-region bytes; its
// children still bucket-sort into the very same single alignment ladder as
// `a`/`b`.

pub const GROUP_OPTION_NESTED_GROUP_ALL_PRESENT: [u8; 10] =
    [195, 245, 72, 64, 0, 62, 156, 156, 200, 15];
pub const GROUP_OPTION_NESTED_GROUP_ALL_PRESENT_INODES: [Inode; 6] = [
    Inode {
        kind: WireKind::I8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 6,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionF16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 4,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionGroup,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 3,
            topology_offset: 0,
            nullable_bit_index: 1,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionI8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 7,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 2,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::F32,
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
    Inode {
        kind: WireKind::OptionU8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 8,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 3,
            group: true,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_option_nested_group_all_present() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::I8,
                },
            },
            PyroField {
                name: "b".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionPrimitiveScalar {
                    primitive: PrimitiveDataType::F16,
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
                        PyroField {
                            name: "z".to_string(),
                            doc: String::new(),
                            data_type: PyroType::OptionPrimitiveScalar {
                                primitive: PrimitiveDataType::U8,
                            },
                        },
                    ],
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("a", PyroValue::from(-100i8)),
            ("b", PyroValue::from(Some(f16::from_f32(1.5)))),
            (
                "c",
                PyroValue::from(Some(PyroValue::Group(
                    [
                        ("x", PyroValue::from(Some(-100i8))),
                        ("y", PyroValue::from(3.14f32)),
                        ("z", PyroValue::from(Some(200u8))),
                    ]
                    .into(),
                ))),
            ),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &GROUP_OPTION_NESTED_GROUP_ALL_PRESENT,
        &GROUP_OPTION_NESTED_GROUP_ALL_PRESENT_INODES,
    );
}

pub const GROUP_OPTION_NESTED_GROUP_PRESENT_CHILDREN_NONE: [u8; 10] =
    [195, 245, 72, 64, 0, 62, 156, 0, 0, 3];
pub const GROUP_OPTION_NESTED_GROUP_PRESENT_CHILDREN_NONE_INODES: [Inode; 6] = [
    Inode {
        kind: WireKind::I8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 6,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionF16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 4,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionGroup,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 3,
            topology_offset: 0,
            nullable_bit_index: 1,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionI8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 7,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 2,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::F32,
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
    Inode {
        kind: WireKind::OptionU8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 8,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 3,
            group: true,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_option_nested_group_present_children_none() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::I8,
                },
            },
            PyroField {
                name: "b".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionPrimitiveScalar {
                    primitive: PrimitiveDataType::F16,
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
                        PyroField {
                            name: "z".to_string(),
                            doc: String::new(),
                            data_type: PyroType::OptionPrimitiveScalar {
                                primitive: PrimitiveDataType::U8,
                            },
                        },
                    ],
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("a", PyroValue::from(-100i8)),
            ("b", PyroValue::from(Some(f16::from_f32(1.5)))),
            (
                "c",
                PyroValue::from(Some(PyroValue::Group(
                    [
                        ("x", PyroValue::from(None::<i8>)),
                        ("y", PyroValue::from(3.14f32)),
                        ("z", PyroValue::from(None::<u8>)),
                    ]
                    .into(),
                ))),
            ),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &GROUP_OPTION_NESTED_GROUP_PRESENT_CHILDREN_NONE,
        &GROUP_OPTION_NESTED_GROUP_PRESENT_CHILDREN_NONE_INODES,
    );
}

pub const GROUP_OPTION_NESTED_GROUP_ABSENT: [u8; 10] = [0, 0, 0, 0, 0, 0, 156, 0, 0, 0];
pub const GROUP_OPTION_NESTED_GROUP_ABSENT_INODES: [Inode; 6] = [
    Inode {
        kind: WireKind::I8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 6,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionF16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 4,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionGroup,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 3,
            topology_offset: 0,
            nullable_bit_index: 1,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionI8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 7,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 2,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::F32,
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
    Inode {
        kind: WireKind::OptionU8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 8,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 3,
            group: true,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_option_nested_group_absent() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::I8,
                },
            },
            PyroField {
                name: "b".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionPrimitiveScalar {
                    primitive: PrimitiveDataType::F16,
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
                        PyroField {
                            name: "z".to_string(),
                            doc: String::new(),
                            data_type: PyroType::OptionPrimitiveScalar {
                                primitive: PrimitiveDataType::U8,
                            },
                        },
                    ],
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("a", PyroValue::from(-100i8)),
            ("b", PyroValue::from(None::<f16>)),
            ("c", PyroValue::Null),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &GROUP_OPTION_NESTED_GROUP_ABSENT,
        &GROUP_OPTION_NESTED_GROUP_ABSENT_INODES,
    );
}

#[cfg(test)]
use crate::value::PrimitiveValueList;

// ---------------------------------------------------------------------------
// option_gaps_masks: Group{a: List<Option<u64>>, b: List<Option<u32>>, c:
// List<Option<u16>>, d: List<Option<u8>>, e: List<Option<Time>>, f:
// Option<u8>, g: Option<f64>}
// ---------------------------------------------------------------------------
//
// `Option` lists of every alignment: each validity mask ends its payload off-
// alignment. Byte ranges are `[start..end)` offsets into the buffer; captured
// from the value writer's output.

pub const GROUP_OPTION_GAPS_MASKS: [u8; 105] = [
    72, 0, // [0..2] "a" payload start pointer
    2, 0, // [2..4] "a" valid sample count
    3, 0, // [4..6] "a" total sample count (nulls included)
    92, 0, // [6..8] "b" payload start pointer
    0, 0, // [8..10] "b" valid sample count
    1, 0, // [10..12] "b" total sample count (nulls included)
    94, 0, // [12..14] "c" payload start pointer
    3, 0, // [14..16] "c" valid sample count
    3, 0, // [16..18] "c" total sample count (nulls included)
    101, 0, // [18..20] "d" payload start pointer
    3, 0, // [20..22] "d" valid sample count
    5, 0, // [22..24] "d" total sample count (nulls included)
    48, 0, // [24..26] "e" payload start pointer
    1, 0, // [26..28] "e" valid sample count
    2, 0, // [28..30] "e" total sample count (nulls included)
    0, 0, 0, 0, 0, 0, // [30..36] alignment padding (6 bytes)
    0, 0, 0, 0, // [36..40] "g" = None (slot left zeroed)
    4, // [40..41] "f" = 4
    1, // [41..42] validity bitvec (LSB first): bit 0 "f" present = 1, bit 1 "g" present = 0
    0, 0, 0, 0, 0, 0, // [42..48] alignment padding (6 bytes)
    99, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // [48..64] "e" valid[0] = Time(99)
    2, // [64..65] "e" validity mask bits 0..2 (LSB first) = 0, 1
    0, 0, 0, 0, 0, 0, 0, // [65..72] alignment padding (7 bytes)
    1, 0, 0, 0, 0, 0, 0, 0, // [72..80] "a" valid[0] = 1
    3, 0, 0, 0, 0, 0, 0, 0, // [80..88] "a" valid[1] = 3
    5, // [88..89] "a" validity mask bits 0..3 (LSB first) = 1, 0, 1
    0, // [89..90] "b" validity mask bits 0..1 (LSB first) = 0
    0, 0, 0, 0, // [90..94] alignment padding (4 bytes)
    1, 0, // [94..96] "c" valid[0] = 1
    2, 0, // [96..98] "c" valid[1] = 2
    3, 0,  // [98..100] "c" valid[2] = 3
    7,  // [100..101] "c" validity mask bits 0..3 (LSB first) = 1, 1, 1
    5,  // [101..102] "d" valid[0] = 5
    7,  // [102..103] "d" valid[1] = 7
    8,  // [103..104] "d" valid[2] = 8
    26, // [104..105] "d" validity mask bits 0..5 (LSB first) = 0, 1, 0, 1, 1
];
pub const GROUP_OPTION_GAPS_MASKS_INODES: [Inode; 7] = [
    Inode {
        kind: WireKind::OptionU64,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 0,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionU32,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 6,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionU16,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 12,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionU8,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 18,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionTimestamp,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 24,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionU8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 40,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionF64,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 32,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 1,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_option_gaps_masks() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::OptionPrimitiveScalar {
                        primitive: PrimitiveDataType::U64,
                    }),
                },
            },
            PyroField {
                name: "b".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::OptionPrimitiveScalar {
                        primitive: PrimitiveDataType::U32,
                    }),
                },
            },
            PyroField {
                name: "c".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::OptionPrimitiveScalar {
                        primitive: PrimitiveDataType::U16,
                    }),
                },
            },
            PyroField {
                name: "d".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::OptionPrimitiveScalar {
                        primitive: PrimitiveDataType::U8,
                    }),
                },
            },
            PyroField {
                name: "e".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::OptionPrimitiveScalar {
                        primitive: PrimitiveDataType::Timestamp,
                    }),
                },
            },
            PyroField {
                name: "f".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionPrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                },
            },
            PyroField {
                name: "g".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionPrimitiveScalar {
                    primitive: PrimitiveDataType::F64,
                },
            },
        ],
    };
    let data = PyroValue::Group(
        [
            (
                "a",
                PyroValue::List(vec![PyroValue::U64(1), PyroValue::Null, PyroValue::U64(3)]),
            ),
            ("b", PyroValue::List(vec![PyroValue::Null])),
            (
                "c",
                PyroValue::List(vec![
                    PyroValue::U16(1),
                    PyroValue::U16(2),
                    PyroValue::U16(3),
                ]),
            ),
            (
                "d",
                PyroValue::List(vec![
                    PyroValue::Null,
                    PyroValue::U8(5),
                    PyroValue::Null,
                    PyroValue::U8(7),
                    PyroValue::U8(8),
                ]),
            ),
            (
                "e",
                PyroValue::List(vec![PyroValue::Null, PyroValue::from(Time(99))]),
            ),
            ("f", PyroValue::U8(4)),
            ("g", PyroValue::Null),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &GROUP_OPTION_GAPS_MASKS,
        &GROUP_OPTION_GAPS_MASKS_INODES,
    );
}

// ---------------------------------------------------------------------------
// option_gaps_masks_all_none: Group{a: List<Option<u64>>, b:
// List<Option<u32>>, c: List<Option<u16>>, d: List<Option<u8>>, e:
// List<Option<Time>>, f: Option<u8>, g: Option<f64>}
// ---------------------------------------------------------------------------
//
// the same `Option` lists holding only nulls: no valid payload, masks only.
// Byte ranges are `[start..end)` offsets into the buffer; captured from the
// value writer's output.

pub const GROUP_OPTION_GAPS_MASKS_ALL_NONE: [u8; 65] = [
    56, 0, // [0..2] "a" payload start pointer
    0, 0, // [2..4] "a" valid sample count
    2, 0, // [4..6] "a" total sample count (nulls included)
    60, 0, // [6..8] "b" payload start pointer
    0, 0, // [8..10] "b" valid sample count
    1, 0, // [10..12] "b" total sample count (nulls included)
    62, 0, // [12..14] "c" payload start pointer
    0, 0, // [14..16] "c" valid sample count
    3, 0, // [16..18] "c" total sample count (nulls included)
    63, 0, // [18..20] "d" payload start pointer
    0, 0, // [20..22] "d" valid sample count
    9, 0, // [22..24] "d" total sample count (nulls included)
    48, 0, // [24..26] "e" payload start pointer
    0, 0, // [26..28] "e" valid sample count
    1, 0, // [28..30] "e" total sample count (nulls included)
    0, 0, // [30..32] alignment padding (2 bytes)
    0, 0, 0, 0, 0, 0, 4, 64, // [32..40] "g" = 2.5
    0,  // [40..41] "f" = None (slot left zeroed)
    2,  // [41..42] validity bitvec (LSB first): bit 0 "f" present = 0, bit 1 "g" present = 1
    0,  // [42..43] "e" validity mask bits 0..1 (LSB first) = 0
    0,  // [43..44] "a" validity mask bits 0..2 (LSB first) = 0, 0
    0, 0, // [44..46] alignment padding (2 bytes)
    0, // [46..47] "b" validity mask bits 0..1 (LSB first) = 0
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // [47..62] alignment padding (15 bytes)
    0, // [62..63] "c" validity mask bits 0..3 (LSB first) = 0, 0, 0
    0, // [63..64] "d" validity mask bits 0..8 (LSB first) = 0, 0, 0, 0, 0, 0, 0, 0
    0, // [64..65] "d" validity mask bits 8..9 (LSB first) = 0
];
pub const GROUP_OPTION_GAPS_MASKS_ALL_NONE_INODES: [Inode; 7] = [
    Inode {
        kind: WireKind::OptionU64,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 0,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionU32,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 6,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionU16,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 12,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionU8,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 18,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionTimestamp,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 24,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionU8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 40,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionF64,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 32,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 1,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_option_gaps_masks_all_none() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::OptionPrimitiveScalar {
                        primitive: PrimitiveDataType::U64,
                    }),
                },
            },
            PyroField {
                name: "b".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::OptionPrimitiveScalar {
                        primitive: PrimitiveDataType::U32,
                    }),
                },
            },
            PyroField {
                name: "c".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::OptionPrimitiveScalar {
                        primitive: PrimitiveDataType::U16,
                    }),
                },
            },
            PyroField {
                name: "d".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::OptionPrimitiveScalar {
                        primitive: PrimitiveDataType::U8,
                    }),
                },
            },
            PyroField {
                name: "e".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::OptionPrimitiveScalar {
                        primitive: PrimitiveDataType::Timestamp,
                    }),
                },
            },
            PyroField {
                name: "f".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionPrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                },
            },
            PyroField {
                name: "g".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionPrimitiveScalar {
                    primitive: PrimitiveDataType::F64,
                },
            },
        ],
    };
    let data = PyroValue::Group(
        [
            ("a", PyroValue::List(vec![PyroValue::Null, PyroValue::Null])),
            ("b", PyroValue::List(vec![PyroValue::Null])),
            (
                "c",
                PyroValue::List(vec![PyroValue::Null, PyroValue::Null, PyroValue::Null]),
            ),
            (
                "d",
                PyroValue::List(vec![
                    PyroValue::Null,
                    PyroValue::Null,
                    PyroValue::Null,
                    PyroValue::Null,
                    PyroValue::Null,
                    PyroValue::Null,
                    PyroValue::Null,
                    PyroValue::Null,
                    PyroValue::Null,
                ]),
            ),
            ("e", PyroValue::List(vec![PyroValue::Null])),
            ("f", PyroValue::Null),
            ("g", PyroValue::F64(2.5)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &GROUP_OPTION_GAPS_MASKS_ALL_NONE,
        &GROUP_OPTION_GAPS_MASKS_ALL_NONE_INODES,
    );
}

// ---------------------------------------------------------------------------
// option_gaps_option_group_present: Group{a: Option<Group{x:
// List<Option<i16>>, y: Option<u32>}>, b: List<Option<Group{p: u64, q:
// Option<u8>}>>, c: List<List<Option<f32>>>, d: u8}
// ---------------------------------------------------------------------------
//
// an `Option<Group>` holding lists, a list of `Option<Group>`, and a nested
// `Option` list. Byte ranges are `[start..end)` offsets into the buffer;
// captured from the value writer's output.

pub const GROUP_OPTION_GAPS_OPTION_GROUP_PRESENT: [u8; 75] = [
    70, 0, // [0..2] "a.x" payload start pointer
    1, 0, // [2..4] "a.x" valid sample count
    2, 0, // [4..6] "a.x" total sample count (nulls included)
    38, 0, // [6..8] group "b" slot: validity mask pointer
    3, 0, // [8..10] group "b" slot: group count (absent included)
    48, 0, // [10..12] "b.p" payload start pointer
    2, 0, // [12..14] "b.p" sample count
    73, 0, // [14..16] "b.q" payload start pointer
    1, 0, // [16..18] "b.q" valid sample count
    2, 0, // [18..20] "b.q" total sample count (nulls included)
    40, 0, // [20..22] "c" topology start pointer
    4, 0, // [22..24] "c" topology word count
    64, 0, // [24..26] "c" payload start pointer
    1, 0, // [26..28] "c" valid sample count
    3, 0, // [28..30] "c" total sample count (nulls included)
    0, 0, // [30..32] alignment padding (2 bytes)
    5, 0, 0, 0, // [32..36] "a.y" = 5
    1, // [36..37] "d" = 1
    3, // [37..38] validity bitvec (LSB first): bit 0 "a" present = 1, bit 1 "a.y" present = 1
    5, 0, // [38..40] group "b" validity mask (u32 word 0, LSB first) = 1, 0, 1
    255, 255, // [40..42] "c" topology level 0: sentinel
    2, 0, // [42..44] "c" topology level 0: boundary at 2
    2, 0, // [44..46] "c" topology level 0: boundary at 2
    3, 0, // [46..48] "c" topology level 0: boundary at 3
    1, 0, 0, 0, 0, 0, 0, 0, // [48..56] "b.p"[0] = 1
    3, 0, 0, 0, 0, 0, 0, 0, // [56..64] "b.p"[1] = 3
    0, 0, 192, 63, // [64..68] "c" valid[0] = 1.5
    1,  // [68..69] "c" validity mask bits 0..3 (LSB first) = 1, 0, 0
    0,  // [69..70] alignment padding (1 byte)
    254, 255, // [70..72] "a.x" valid[0] = -2
    2,   // [72..73] "a.x" validity mask bits 0..2 (LSB first) = 0, 1
    4,   // [73..74] "b.q" valid[0] = 4
    2,   // [74..75] "b.q" validity mask bits 0..2 (LSB first) = 0, 1
];
pub const GROUP_OPTION_GAPS_OPTION_GROUP_PRESENT_INODES: [Inode; 8] = [
    Inode {
        kind: WireKind::OptionGroup,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 2,
            topology_offset: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionI16,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 0,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionU32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 32,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 1,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionGroup,
        kind_meta: KindMeta::Group {
            nesting: 1,
            count: 2,
            topology_offset: 6,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U64,
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
        kind: WireKind::OptionU8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 14,
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
            nesting: 2,
            data_offset: 24,
            topology_offset: 20,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 36,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_option_gaps_option_group_present() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionGroup {
                    fields: vec![
                        PyroField {
                            name: "x".to_string(),
                            doc: String::new(),
                            data_type: PyroType::List {
                                element: Box::new(PyroType::OptionPrimitiveScalar {
                                    primitive: PrimitiveDataType::I16,
                                }),
                            },
                        },
                        PyroField {
                            name: "y".to_string(),
                            doc: String::new(),
                            data_type: PyroType::OptionPrimitiveScalar {
                                primitive: PrimitiveDataType::U32,
                            },
                        },
                    ],
                },
            },
            PyroField {
                name: "b".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::OptionGroup {
                        fields: vec![
                            PyroField {
                                name: "p".to_string(),
                                doc: String::new(),
                                data_type: PyroType::PrimitiveScalar {
                                    primitive: PrimitiveDataType::U64,
                                },
                            },
                            PyroField {
                                name: "q".to_string(),
                                doc: String::new(),
                                data_type: PyroType::OptionPrimitiveScalar {
                                    primitive: PrimitiveDataType::U8,
                                },
                            },
                        ],
                    }),
                },
            },
            PyroField {
                name: "c".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::List {
                        element: Box::new(PyroType::OptionPrimitiveScalar {
                            primitive: PrimitiveDataType::F32,
                        }),
                    }),
                },
            },
            PyroField {
                name: "d".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                },
            },
        ],
    };
    let data = PyroValue::Group(
        [
            (
                "a",
                PyroValue::Group(
                    [
                        (
                            "x",
                            PyroValue::List(vec![PyroValue::Null, PyroValue::I16(-2)]),
                        ),
                        ("y", PyroValue::U32(5)),
                    ]
                    .into(),
                ),
            ),
            (
                "b",
                PyroValue::List(vec![
                    PyroValue::Group([("p", PyroValue::U64(1)), ("q", PyroValue::Null)].into()),
                    PyroValue::Null,
                    PyroValue::Group([("p", PyroValue::U64(3)), ("q", PyroValue::U8(4))].into()),
                ]),
            ),
            (
                "c",
                PyroValue::List(vec![
                    PyroValue::List(vec![PyroValue::F32(1.5), PyroValue::Null]),
                    PyroValue::List(vec![]),
                    PyroValue::List(vec![PyroValue::Null]),
                ]),
            ),
            ("d", PyroValue::U8(1)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &GROUP_OPTION_GAPS_OPTION_GROUP_PRESENT,
        &GROUP_OPTION_GAPS_OPTION_GROUP_PRESENT_INODES,
    );
}

// ---------------------------------------------------------------------------
// option_gaps_option_group_absent: Group{a: Option<Group{x: List<Option<i16>>,
// y: Option<u32>}>, b: List<Option<Group{p: u64, q: Option<u8>}>>, c:
// List<List<Option<f32>>>, d: u8}
// ---------------------------------------------------------------------------
//
// as `option_gaps_option_group_present`, with the `Option<Group>` absent. Byte
// ranges are `[start..end)` offsets into the buffer; captured from the value
// writer's output.

pub const GROUP_OPTION_GAPS_OPTION_GROUP_ABSENT: [u8; 72] = [
    70, 0, // [0..2] "a.x" payload start pointer
    0, 0, // [2..4] "a.x" valid sample count
    0, 0, // [4..6] "a.x" total sample count (nulls included)
    38, 0, // [6..8] group "b" slot: validity mask pointer
    3, 0, // [8..10] group "b" slot: group count (absent included)
    48, 0, // [10..12] "b.p" payload start pointer
    2, 0, // [12..14] "b.p" sample count
    70, 0, // [14..16] "b.q" payload start pointer
    1, 0, // [16..18] "b.q" valid sample count
    2, 0, // [18..20] "b.q" total sample count (nulls included)
    40, 0, // [20..22] "c" topology start pointer
    4, 0, // [22..24] "c" topology word count
    64, 0, // [24..26] "c" payload start pointer
    1, 0, // [26..28] "c" valid sample count
    3, 0, // [28..30] "c" total sample count (nulls included)
    0, 0, // [30..32] "a.y" (parent group absent, slot left zeroed)
    0, 0, 0, 0, // [32..36] alignment padding (4 bytes)
    1, // [36..37] "d" = 1
    0, // [37..38] validity bitvec (LSB first): bit 0 "a" present = 0, bit 1 "a.y" present = 0
    5, 0, // [38..40] group "b" validity mask (u32 word 0, LSB first) = 1, 0, 1
    255, 255, // [40..42] "c" topology level 0: sentinel
    2, 0, // [42..44] "c" topology level 0: boundary at 2
    2, 0, // [44..46] "c" topology level 0: boundary at 2
    3, 0, // [46..48] "c" topology level 0: boundary at 3
    1, 0, 0, 0, 0, 0, 0, 0, // [48..56] "b.p"[0] = 1
    3, 0, 0, 0, 0, 0, 0, 0, // [56..64] "b.p"[1] = 3
    0, 0, 192, 63, // [64..68] "c" valid[0] = 1.5
    1,  // [68..69] "c" validity mask bits 0..3 (LSB first) = 1, 0, 0
    0,  // [69..70] alignment padding (1 byte)
    4,  // [70..71] "b.q" valid[0] = 4
    2,  // [71..72] "b.q" validity mask bits 0..2 (LSB first) = 0, 1
];
pub const GROUP_OPTION_GAPS_OPTION_GROUP_ABSENT_INODES: [Inode; 8] = [
    Inode {
        kind: WireKind::OptionGroup,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 2,
            topology_offset: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionI16,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 0,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionU32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 32,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 1,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionGroup,
        kind_meta: KindMeta::Group {
            nesting: 1,
            count: 2,
            topology_offset: 6,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U64,
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
        kind: WireKind::OptionU8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 14,
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
            nesting: 2,
            data_offset: 24,
            topology_offset: 20,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 36,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_option_gaps_option_group_absent() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionGroup {
                    fields: vec![
                        PyroField {
                            name: "x".to_string(),
                            doc: String::new(),
                            data_type: PyroType::List {
                                element: Box::new(PyroType::OptionPrimitiveScalar {
                                    primitive: PrimitiveDataType::I16,
                                }),
                            },
                        },
                        PyroField {
                            name: "y".to_string(),
                            doc: String::new(),
                            data_type: PyroType::OptionPrimitiveScalar {
                                primitive: PrimitiveDataType::U32,
                            },
                        },
                    ],
                },
            },
            PyroField {
                name: "b".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::OptionGroup {
                        fields: vec![
                            PyroField {
                                name: "p".to_string(),
                                doc: String::new(),
                                data_type: PyroType::PrimitiveScalar {
                                    primitive: PrimitiveDataType::U64,
                                },
                            },
                            PyroField {
                                name: "q".to_string(),
                                doc: String::new(),
                                data_type: PyroType::OptionPrimitiveScalar {
                                    primitive: PrimitiveDataType::U8,
                                },
                            },
                        ],
                    }),
                },
            },
            PyroField {
                name: "c".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::List {
                        element: Box::new(PyroType::OptionPrimitiveScalar {
                            primitive: PrimitiveDataType::F32,
                        }),
                    }),
                },
            },
            PyroField {
                name: "d".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                },
            },
        ],
    };
    let data = PyroValue::Group(
        [
            ("a", PyroValue::Null),
            (
                "b",
                PyroValue::List(vec![
                    PyroValue::Group([("p", PyroValue::U64(1)), ("q", PyroValue::Null)].into()),
                    PyroValue::Null,
                    PyroValue::Group([("p", PyroValue::U64(3)), ("q", PyroValue::U8(4))].into()),
                ]),
            ),
            (
                "c",
                PyroValue::List(vec![
                    PyroValue::List(vec![PyroValue::F32(1.5), PyroValue::Null]),
                    PyroValue::List(vec![]),
                    PyroValue::List(vec![PyroValue::Null]),
                ]),
            ),
            ("d", PyroValue::U8(1)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &GROUP_OPTION_GAPS_OPTION_GROUP_ABSENT,
        &GROUP_OPTION_GAPS_OPTION_GROUP_ABSENT_INODES,
    );
}

// ---------------------------------------------------------------------------
// option_gaps_fixed: Group{a: List<Option<[u16; 3]>>, b: Option<[u8; 5]>, c:
// List<Option<[f64; 2]>>, d: List<Option<[bool; 3]>>, e: Option<i8>}
// ---------------------------------------------------------------------------
//
// `Option` fixed arrays inline and in lists. Byte ranges are `[start..end)`
// offsets into the buffer; captured from the value writer's output.

pub const GROUP_OPTION_GAPS_FIXED: [u8; 65] = [
    50, 0, // [0..2] "a" payload start pointer
    2, 0, // [2..4] "a" valid array count
    3, 0, // [4..6] "a" total array count (nulls included)
    32, 0, // [6..8] "c" payload start pointer
    1, 0, // [8..10] "c" valid array count
    2, 0, // [10..12] "c" total array count (nulls included)
    63, 0, // [12..14] "d" payload start pointer
    1, 0, // [14..16] "d" valid array count
    2, 0, // [16..18] "d" total array count (nulls included)
    1, 2, 3, 4, 5,   // [18..23] "b" = [1, 2, 3, 4, 5]
    255, // [23..24] "e" = -1
    3,   // [24..25] validity bitvec (LSB first): bit 0 "b" present = 1, bit 1 "e" present = 1
    0, 0, 0, 0, 0, 0, 0, // [25..32] alignment padding (7 bytes)
    0, 0, 0, 0, 0, 0, 224, 63, 0, 0, 0, 0, 0, 0, 224,
    191, // [32..48] "c" valid[0] = [0.5, -0.5]
    2,   // [48..49] "c" validity mask bits 0..2 (LSB first) = 0, 1
    0,   // [49..50] alignment padding (1 byte)
    1, 0, 2, 0, 3, 0, // [50..56] "a" valid[0] = [1, 2, 3]
    4, 0, 5, 0, 6, 0, // [56..62] "a" valid[1] = [4, 5, 6]
    5, // [62..63] "a" validity mask bits 0..3 (LSB first) = 1, 0, 1
    3, // [63..64] "d" valid payload bits 0..3 (LSB first) = 1, 1, 0
    1, // [64..65] "d" validity mask bits 0..2 (LSB first) = 1, 0
];
pub const GROUP_OPTION_GAPS_FIXED_INODES: [Inode; 5] = [
    Inode {
        kind: WireKind::OptionU16,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 0,
            topology_offset: 0,
            fixed_size: 3,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionU8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 18,
            topology_offset: 0,
            fixed_size: 5,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionF64,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 6,
            topology_offset: 0,
            fixed_size: 2,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionBool,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 12,
            topology_offset: 0,
            fixed_size: 3,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionI8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 23,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 1,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_option_gaps_fixed() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::OptionPrimitiveFixedList {
                        element: PrimitiveDataType::U16,
                        size: 3,
                    }),
                },
            },
            PyroField {
                name: "b".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionPrimitiveFixedList {
                    element: PrimitiveDataType::U8,
                    size: 5,
                },
            },
            PyroField {
                name: "c".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::OptionPrimitiveFixedList {
                        element: PrimitiveDataType::F64,
                        size: 2,
                    }),
                },
            },
            PyroField {
                name: "d".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::OptionPrimitiveFixedList {
                        element: PrimitiveDataType::Bool,
                        size: 3,
                    }),
                },
            },
            PyroField {
                name: "e".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionPrimitiveScalar {
                    primitive: PrimitiveDataType::I8,
                },
            },
        ],
    };
    let data = PyroValue::Group(
        [
            (
                "a",
                PyroValue::List(vec![
                    PyroValue::PrimitiveList(PrimitiveValueList::U16(vec![1, 2, 3].into())),
                    PyroValue::Null,
                    PyroValue::PrimitiveList(PrimitiveValueList::U16(vec![4, 5, 6].into())),
                ]),
            ),
            (
                "b",
                PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![1, 2, 3, 4, 5].into())),
            ),
            (
                "c",
                PyroValue::List(vec![
                    PyroValue::Null,
                    PyroValue::PrimitiveList(PrimitiveValueList::F64(vec![0.5, -0.5].into())),
                ]),
            ),
            (
                "d",
                PyroValue::List(vec![
                    PyroValue::from(vec![true, true, false]),
                    PyroValue::Null,
                ]),
            ),
            ("e", PyroValue::I8(-1)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &GROUP_OPTION_GAPS_FIXED,
        &GROUP_OPTION_GAPS_FIXED_INODES,
    );
}
// ---------------------------------------------------------------------------
// option_group_absent_str: Group{a: Option<Group{s: Str, n: u8}>, b: Str, c: u8}
// ---------------------------------------------------------------------------
//
// A `None` `Option<Group>` whose only variable lane is a `Str`: the
// absent group's string still gets its payload start pointer, at the
// offset its (empty) payload would start.
// Byte ranges are `[start..end)` offsets into the buffer; captured from the
// value writer's output.

pub const GROUP_OPTION_GROUP_ABSENT_STR: [u8; 14] = [
    11, 0, // [0..2] "a.s" payload start pointer (group absent: empty, at its lane start)
    0, 0, // [2..4] "a.s" byte length
    11, 0, // [4..6] "b" payload start pointer
    3, 0, // [6..8] "b" byte length
    0, // [8..9] "a.n" (parent group absent, slot left zeroed)
    7, // [9..10] "c" = 7
    0, // [10..11] validity bitvec (LSB first): bit 0 "a" present = 0
    120, 121, 122, // [11..14] "b" = "xyz"
];
pub const GROUP_OPTION_GROUP_ABSENT_STR_INODES: [Inode; 5] = [
    Inode {
        kind: WireKind::OptionGroup,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 2,
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
    Inode {
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 8,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Str,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 4,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 9,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_option_group_absent_str() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionGroup {
                    fields: vec![
                        PyroField {
                            name: "s".to_string(),
                            doc: String::new(),
                            data_type: PyroType::Str,
                        },
                        PyroField {
                            name: "n".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U8,
                            },
                        },
                    ],
                },
            },
            PyroField {
                name: "b".to_string(),
                doc: String::new(),
                data_type: PyroType::Str,
            },
            PyroField {
                name: "c".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                },
            },
        ],
    };
    let data = PyroValue::Group(
        [
            ("a", PyroValue::Null),
            ("b", PyroValue::Str("xyz".into())),
            ("c", PyroValue::U8(7)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &GROUP_OPTION_GROUP_ABSENT_STR,
        &GROUP_OPTION_GROUP_ABSENT_STR_INODES,
    );
}

// ---------------------------------------------------------------------------
// option_group_absent_wide_list: Group{a: Option<Group{v: List<u64>}>, b: List<u32>, d: u8}
// ---------------------------------------------------------------------------
//
// A `None` `Option<Group>` holding an 8-aligned list: its empty payload
// still aligns the variable region to 8, which moves every narrower lane
// written after it.
// Byte ranges are `[start..end)` offsets into the buffer; captured from the
// value writer's output.

pub const GROUP_OPTION_GROUP_ABSENT_WIDE_LIST: [u8; 28] = [
    16, 0, // [0..2] "a.v" payload start pointer (group absent: empty, 8-aligned)
    0, 0, // [2..4] "a.v" sample count
    16, 0, // [4..6] "b" payload start pointer
    3, 0, // [6..8] "b" sample count
    9, // [8..9] "d" = 9
    0, // [9..10] validity bitvec (LSB first): bit 0 "a" present = 0
    0, 0, 0, 0, 0, 0, // [10..16] alignment padding (6 bytes): "a.v" aligns to 8
    1, 0, 0, 0, // [16..20] "b"[0] = 1
    2, 0, 0, 0, // [20..24] "b"[1] = 2
    3, 0, 0, 0, // [24..28] "b"[2] = 3
];
pub const GROUP_OPTION_GROUP_ABSENT_WIDE_LIST_INODES: [Inode; 4] = [
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
        kind: WireKind::U64,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 0,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 4,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
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
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_option_group_absent_wide_list() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionGroup {
                    fields: vec![PyroField {
                        name: "v".to_string(),
                        doc: String::new(),
                        data_type: PyroType::List {
                            element: Box::new(PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U64,
                            }),
                        },
                    }],
                },
            },
            PyroField {
                name: "b".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::U32,
                    }),
                },
            },
            PyroField {
                name: "d".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                },
            },
        ],
    };
    let data = PyroValue::Group(
        [
            ("a", PyroValue::Null),
            (
                "b",
                PyroValue::PrimitiveList(PrimitiveValueList::U32(vec![1, 2, 3].into())),
            ),
            ("d", PyroValue::U8(9)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &GROUP_OPTION_GROUP_ABSENT_WIDE_LIST,
        &GROUP_OPTION_GROUP_ABSENT_WIDE_LIST_INODES,
    );
}

// ---------------------------------------------------------------------------
// option_group_absent_nested_list: Group{a: Option<Group{m: List<List<u16>>, k: u32}>, b: List<u8>}
// ---------------------------------------------------------------------------
//
// A `None` `Option<Group>` holding a two-level list: the absent group's
// topology is still laid out (empty), alongside its payload pointer.
// Byte ranges are `[start..end)` offsets into the buffer; captured from the
// value writer's output.

pub const GROUP_OPTION_GROUP_ABSENT_NESTED_LIST: [u8; 21] = [
    18, 0, // [0..2] "a.m" topology start pointer (group absent: empty, word-aligned)
    0, 0, // [2..4] "a.m" topology word count
    18, 0, // [4..6] "a.m" payload start pointer
    0, 0, // [6..8] "a.m" sample count
    18, 0, // [8..10] "b" payload start pointer
    3, 0, // [10..12] "b" sample count
    0, 0, 0, 0, // [12..16] "a.k" (parent group absent, slot left zeroed)
    0, // [16..17] validity bitvec (LSB first): bit 0 "a" present = 0
    0, // [17..18] alignment padding (1 byte): "a.m" topology aligns to the word
    5, 6, 7, // [18..21] "b" = 5, 6, 7
];
pub const GROUP_OPTION_GROUP_ABSENT_NESTED_LIST_INODES: [Inode; 4] = [
    Inode {
        kind: WireKind::OptionGroup,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 2,
            topology_offset: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U16,
        kind_meta: KindMeta::Primitive {
            nesting: 2,
            data_offset: 4,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
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
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 8,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_option_group_absent_nested_list() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionGroup {
                    fields: vec![
                        PyroField {
                            name: "m".to_string(),
                            doc: String::new(),
                            data_type: PyroType::List {
                                element: Box::new(PyroType::List {
                                    element: Box::new(PyroType::PrimitiveScalar {
                                        primitive: PrimitiveDataType::U16,
                                    }),
                                }),
                            },
                        },
                        PyroField {
                            name: "k".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U32,
                            },
                        },
                    ],
                },
            },
            PyroField {
                name: "b".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::U8,
                    }),
                },
            },
        ],
    };
    let data = PyroValue::Group(
        [
            ("a", PyroValue::Null),
            (
                "b",
                PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![5, 6, 7].into())),
            ),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &GROUP_OPTION_GROUP_ABSENT_NESTED_LIST,
        &GROUP_OPTION_GROUP_ABSENT_NESTED_LIST_INODES,
    );
}

// ---------------------------------------------------------------------------
// option_group_absent_inside_present: Group{a: Option<Group{x: u8, inner: Option<Group{v: List<i16>, w: Str}>}>, b: List<u8>, c: u16}
// ---------------------------------------------------------------------------
//
// The outer `Option<Group>` is present, its own `Option<Group>` is
// `None`: the inner group's lanes are laid out empty one level down.
// Byte ranges are `[start..end)` offsets into the buffer; captured from the
// value writer's output.

pub const GROUP_OPTION_GROUP_ABSENT_INSIDE_PRESENT: [u8; 17] = [
    16, 0, // [0..2] "a.inner.v" payload start pointer (inner group absent: empty)
    0, 0, // [2..4] "a.inner.v" sample count
    16, 0, // [4..6] "a.inner.w" payload start pointer (inner group absent: empty)
    0, 0, // [6..8] "a.inner.w" byte length
    16, 0, // [8..10] "b" payload start pointer
    1, 0, // [10..12] "b" sample count
    1, 2, // [12..14] "c" = 513
    3, // [14..15] "a.x" = 3
    1, // [15..16] validity bitvec (LSB first): bit 0 "a" present = 1, bit 1 "a.inner" present = 0
    1, // [16..17] "b"[0] = 1
];
pub const GROUP_OPTION_GROUP_ABSENT_INSIDE_PRESENT_INODES: [Inode; 7] = [
    Inode {
        kind: WireKind::OptionGroup,
        kind_meta: KindMeta::Group {
            nesting: 0,
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
            data_offset: 14,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionGroup,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 2,
            topology_offset: 0,
            nullable_bit_index: 1,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I16,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 0,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Str,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 4,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 8,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 12,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_option_group_absent_inside_present() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::OptionGroup {
                    fields: vec![
                        PyroField {
                            name: "x".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U8,
                            },
                        },
                        PyroField {
                            name: "inner".to_string(),
                            doc: String::new(),
                            data_type: PyroType::OptionGroup {
                                fields: vec![
                                    PyroField {
                                        name: "v".to_string(),
                                        doc: String::new(),
                                        data_type: PyroType::List {
                                            element: Box::new(PyroType::PrimitiveScalar {
                                                primitive: PrimitiveDataType::I16,
                                            }),
                                        },
                                    },
                                    PyroField {
                                        name: "w".to_string(),
                                        doc: String::new(),
                                        data_type: PyroType::Str,
                                    },
                                ],
                            },
                        },
                    ],
                },
            },
            PyroField {
                name: "b".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::U8,
                    }),
                },
            },
            PyroField {
                name: "c".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U16,
                },
            },
        ],
    };
    let data = PyroValue::Group(
        [
            (
                "a",
                PyroValue::Group([("x", PyroValue::U8(3)), ("inner", PyroValue::Null)].into()),
            ),
            (
                "b",
                PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![1].into())),
            ),
            ("c", PyroValue::U16(513)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u16(
        schema,
        data,
        &GROUP_OPTION_GROUP_ABSENT_INSIDE_PRESENT,
        &GROUP_OPTION_GROUP_ABSENT_INSIDE_PRESENT_INODES,
    );
}
