#[cfg(test)]
use half::f16;
#[cfg(test)]
use pyro_schema::{
    PrimitiveDataType,
    human::{PyroField, PyroSchema, PyroType},
};

#[cfg(test)]
use crate::{PyroValue, value::Time, wire::tests::support::run_list_wrapped_roundtrip_u32};

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

    run_list_wrapped_roundtrip_u32(
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

    run_list_wrapped_roundtrip_u32(
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

    run_list_wrapped_roundtrip_u32(
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

    run_list_wrapped_roundtrip_u32(
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

    run_list_wrapped_roundtrip_u32(
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

    run_list_wrapped_roundtrip_u32(
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

    run_list_wrapped_roundtrip_u32(
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

    run_list_wrapped_roundtrip_u32(
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

    run_list_wrapped_roundtrip_u32(
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

    run_list_wrapped_roundtrip_u32(
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

    run_list_wrapped_roundtrip_u32(
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

pub const GROUP_OPTION_GAPS_MASKS: [u8; 137] = [
    104, 0, 0, 0, // [0..4] "a" payload start pointer
    2, 0, 0, 0, // [4..8] "a" valid sample count
    3, 0, 0, 0, // [8..12] "a" total sample count (nulls included)
    124, 0, 0, 0, // [12..16] "b" payload start pointer
    0, 0, 0, 0, // [16..20] "b" valid sample count
    1, 0, 0, 0, // [20..24] "b" total sample count (nulls included)
    126, 0, 0, 0, // [24..28] "c" payload start pointer
    3, 0, 0, 0, // [28..32] "c" valid sample count
    3, 0, 0, 0, // [32..36] "c" total sample count (nulls included)
    133, 0, 0, 0, // [36..40] "d" payload start pointer
    3, 0, 0, 0, // [40..44] "d" valid sample count
    5, 0, 0, 0, // [44..48] "d" total sample count (nulls included)
    80, 0, 0, 0, // [48..52] "e" payload start pointer
    1, 0, 0, 0, // [52..56] "e" valid sample count
    2, 0, 0, 0, // [56..60] "e" total sample count (nulls included)
    0, 0, 0, 0, // [60..64] alignment padding (4 bytes)
    0, 0, 0, 0, 0, 0, 0, 0, // [64..72] "g" = None (slot left zeroed)
    4, // [72..73] "f" = 4
    1, // [73..74] validity bitvec (LSB first): bit 0 "f" present = 1, bit 1 "g" present = 0
    0, 0, 0, 0, 0, 0, // [74..80] alignment padding (6 bytes)
    99, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // [80..96] "e" valid[0] = Time(99)
    2, // [96..97] "e" validity mask bits 0..2 (LSB first) = 0, 1
    0, 0, 0, 0, 0, 0, 0, // [97..104] alignment padding (7 bytes)
    1, 0, 0, 0, 0, 0, 0, 0, // [104..112] "a" valid[0] = 1
    3, 0, 0, 0, 0, 0, 0, 0, // [112..120] "a" valid[1] = 3
    5, // [120..121] "a" validity mask bits 0..3 (LSB first) = 1, 0, 1
    0, 0, 0, // [121..124] alignment padding (3 bytes)
    0, // [124..125] "b" validity mask bits 0..1 (LSB first) = 0
    0, // [125..126] alignment padding (1 byte)
    1, 0, // [126..128] "c" valid[0] = 1
    2, 0, // [128..130] "c" valid[1] = 2
    3, 0,  // [130..132] "c" valid[2] = 3
    7,  // [132..133] "c" validity mask bits 0..3 (LSB first) = 1, 1, 1
    5,  // [133..134] "d" valid[0] = 5
    7,  // [134..135] "d" valid[1] = 7
    8,  // [135..136] "d" valid[2] = 8
    26, // [136..137] "d" validity mask bits 0..5 (LSB first) = 0, 1, 0, 1, 1
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
            data_offset: 12,
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
            nesting: 1,
            data_offset: 36,
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
            data_offset: 48,
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
            data_offset: 72,
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
            data_offset: 64,
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

    run_list_wrapped_roundtrip_u32(
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

pub const GROUP_OPTION_GAPS_MASKS_ALL_NONE: [u8; 97] = [
    88, 0, 0, 0, // [0..4] "a" payload start pointer
    0, 0, 0, 0, // [4..8] "a" valid sample count
    2, 0, 0, 0, // [8..12] "a" total sample count (nulls included)
    92, 0, 0, 0, // [12..16] "b" payload start pointer
    0, 0, 0, 0, // [16..20] "b" valid sample count
    1, 0, 0, 0, // [20..24] "b" total sample count (nulls included)
    94, 0, 0, 0, // [24..28] "c" payload start pointer
    0, 0, 0, 0, // [28..32] "c" valid sample count
    3, 0, 0, 0, // [32..36] "c" total sample count (nulls included)
    95, 0, 0, 0, // [36..40] "d" payload start pointer
    0, 0, 0, 0, // [40..44] "d" valid sample count
    9, 0, 0, 0, // [44..48] "d" total sample count (nulls included)
    80, 0, 0, 0, // [48..52] "e" payload start pointer
    0, 0, 0, 0, // [52..56] "e" valid sample count
    1, 0, 0, 0, // [56..60] "e" total sample count (nulls included)
    0, 0, 0, 0, // [60..64] alignment padding (4 bytes)
    0, 0, 0, 0, 0, 0, 4, 64, // [64..72] "g" = 2.5
    0,  // [72..73] "f" = None (slot left zeroed)
    2,  // [73..74] validity bitvec (LSB first): bit 0 "f" present = 0, bit 1 "g" present = 1
    0, 0, 0, 0, 0, 0, // [74..80] alignment padding (6 bytes)
    0, // [80..81] "e" validity mask bits 0..1 (LSB first) = 0
    0, 0, 0, 0, 0, 0, 0, // [81..88] alignment padding (7 bytes)
    0, // [88..89] "a" validity mask bits 0..2 (LSB first) = 0, 0
    0, 0, 0, // [89..92] alignment padding (3 bytes)
    0, // [92..93] "b" validity mask bits 0..1 (LSB first) = 0
    0, // [93..94] alignment padding (1 byte)
    0, // [94..95] "c" validity mask bits 0..3 (LSB first) = 0, 0, 0
    0, // [95..96] "d" validity mask bits 0..8 (LSB first) = 0, 0, 0, 0, 0, 0, 0, 0
    0, // [96..97] "d" validity mask bits 8..9 (LSB first) = 0
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
            data_offset: 12,
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
            nesting: 1,
            data_offset: 36,
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
            data_offset: 48,
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
            data_offset: 72,
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
            data_offset: 64,
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

    run_list_wrapped_roundtrip_u32(
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

pub const GROUP_OPTION_GAPS_OPTION_GROUP_PRESENT: [u8; 115] = [
    110, 0, 0, 0, // [0..4] "a.x" payload start pointer
    1, 0, 0, 0, // [4..8] "a.x" valid sample count
    2, 0, 0, 0, // [8..12] "a.x" total sample count (nulls included)
    68, 0, 0, 0, // [12..16] group "b" slot: validity mask pointer
    3, 0, 0, 0, // [16..20] group "b" slot: group count (absent included)
    88, 0, 0, 0, // [20..24] "b.p" payload start pointer
    2, 0, 0, 0, // [24..28] "b.p" sample count
    113, 0, 0, 0, // [28..32] "b.q" payload start pointer
    1, 0, 0, 0, // [32..36] "b.q" valid sample count
    2, 0, 0, 0, // [36..40] "b.q" total sample count (nulls included)
    72, 0, 0, 0, // [40..44] "c" topology start pointer
    4, 0, 0, 0, // [44..48] "c" topology word count
    104, 0, 0, 0, // [48..52] "c" payload start pointer
    1, 0, 0, 0, // [52..56] "c" valid sample count
    3, 0, 0, 0, // [56..60] "c" total sample count (nulls included)
    5, 0, 0, 0, // [60..64] "a.y" = 5
    1, // [64..65] "d" = 1
    3, // [65..66] validity bitvec (LSB first): bit 0 "a" present = 1, bit 1 "a.y" present = 1
    0, 0, // [66..68] alignment padding (2 bytes)
    5, 0, 0, 0, // [68..72] group "b" validity mask (u32 word 0, LSB first) = 1, 0, 1
    255, 255, 255, 255, // [72..76] "c" topology level 0: sentinel
    2, 0, 0, 0, // [76..80] "c" topology level 0: boundary at 2
    2, 0, 0, 0, // [80..84] "c" topology level 0: boundary at 2
    3, 0, 0, 0, // [84..88] "c" topology level 0: boundary at 3
    1, 0, 0, 0, 0, 0, 0, 0, // [88..96] "b.p"[0] = 1
    3, 0, 0, 0, 0, 0, 0, 0, // [96..104] "b.p"[1] = 3
    0, 0, 192, 63, // [104..108] "c" valid[0] = 1.5
    1,  // [108..109] "c" validity mask bits 0..3 (LSB first) = 1, 0, 0
    0,  // [109..110] alignment padding (1 byte)
    254, 255, // [110..112] "a.x" valid[0] = -2
    2,   // [112..113] "a.x" validity mask bits 0..2 (LSB first) = 0, 1
    4,   // [113..114] "b.q" valid[0] = 4
    2,   // [114..115] "b.q" validity mask bits 0..2 (LSB first) = 0, 1
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
            data_offset: 60,
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
            topology_offset: 12,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U64,
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
            data_offset: 28,
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
            data_offset: 48,
            topology_offset: 40,
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
            data_offset: 64,
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

    run_list_wrapped_roundtrip_u32(
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

pub const GROUP_OPTION_GAPS_OPTION_GROUP_ABSENT: [u8; 112] = [
    110, 0, 0, 0, // [0..4] "a.x" payload start pointer
    0, 0, 0, 0, // [4..8] "a.x" valid sample count
    0, 0, 0, 0, // [8..12] "a.x" total sample count (nulls included)
    68, 0, 0, 0, // [12..16] group "b" slot: validity mask pointer
    3, 0, 0, 0, // [16..20] group "b" slot: group count (absent included)
    88, 0, 0, 0, // [20..24] "b.p" payload start pointer
    2, 0, 0, 0, // [24..28] "b.p" sample count
    110, 0, 0, 0, // [28..32] "b.q" payload start pointer
    1, 0, 0, 0, // [32..36] "b.q" valid sample count
    2, 0, 0, 0, // [36..40] "b.q" total sample count (nulls included)
    72, 0, 0, 0, // [40..44] "c" topology start pointer
    4, 0, 0, 0, // [44..48] "c" topology word count
    104, 0, 0, 0, // [48..52] "c" payload start pointer
    1, 0, 0, 0, // [52..56] "c" valid sample count
    3, 0, 0, 0, // [56..60] "c" total sample count (nulls included)
    0, 0, 0, 0, // [60..64] "a.y" (parent group absent, slot left zeroed)
    1, // [64..65] "d" = 1
    0, // [65..66] validity bitvec (LSB first): bit 0 "a" present = 0, bit 1 "a.y" present = 0
    0, 0, // [66..68] alignment padding (2 bytes)
    5, 0, 0, 0, // [68..72] group "b" validity mask (u32 word 0, LSB first) = 1, 0, 1
    255, 255, 255, 255, // [72..76] "c" topology level 0: sentinel
    2, 0, 0, 0, // [76..80] "c" topology level 0: boundary at 2
    2, 0, 0, 0, // [80..84] "c" topology level 0: boundary at 2
    3, 0, 0, 0, // [84..88] "c" topology level 0: boundary at 3
    1, 0, 0, 0, 0, 0, 0, 0, // [88..96] "b.p"[0] = 1
    3, 0, 0, 0, 0, 0, 0, 0, // [96..104] "b.p"[1] = 3
    0, 0, 192, 63, // [104..108] "c" valid[0] = 1.5
    1,  // [108..109] "c" validity mask bits 0..3 (LSB first) = 1, 0, 0
    0,  // [109..110] alignment padding (1 byte)
    4,  // [110..111] "b.q" valid[0] = 4
    2,  // [111..112] "b.q" validity mask bits 0..2 (LSB first) = 0, 1
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
            data_offset: 60,
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
            topology_offset: 12,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U64,
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
            data_offset: 28,
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
            data_offset: 48,
            topology_offset: 40,
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
            data_offset: 64,
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

    run_list_wrapped_roundtrip_u32(
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

pub const GROUP_OPTION_GAPS_FIXED: [u8; 81] = [
    66, 0, 0, 0, // [0..4] "a" payload start pointer
    2, 0, 0, 0, // [4..8] "a" valid array count
    3, 0, 0, 0, // [8..12] "a" total array count (nulls included)
    48, 0, 0, 0, // [12..16] "c" payload start pointer
    1, 0, 0, 0, // [16..20] "c" valid array count
    2, 0, 0, 0, // [20..24] "c" total array count (nulls included)
    79, 0, 0, 0, // [24..28] "d" payload start pointer
    1, 0, 0, 0, // [28..32] "d" valid array count
    2, 0, 0, 0, // [32..36] "d" total array count (nulls included)
    1, 2, 3, 4, 5,   // [36..41] "b" = [1, 2, 3, 4, 5]
    255, // [41..42] "e" = -1
    3,   // [42..43] validity bitvec (LSB first): bit 0 "b" present = 1, bit 1 "e" present = 1
    0, 0, 0, 0, 0, // [43..48] alignment padding (5 bytes)
    0, 0, 0, 0, 0, 0, 224, 63, 0, 0, 0, 0, 0, 0, 224,
    191, // [48..64] "c" valid[0] = [0.5, -0.5]
    2,   // [64..65] "c" validity mask bits 0..2 (LSB first) = 0, 1
    0,   // [65..66] alignment padding (1 byte)
    1, 0, 2, 0, 3, 0, // [66..72] "a" valid[0] = [1, 2, 3]
    4, 0, 5, 0, 6, 0, // [72..78] "a" valid[1] = [4, 5, 6]
    5, // [78..79] "a" validity mask bits 0..3 (LSB first) = 1, 0, 1
    3, // [79..80] "d" valid payload bits 0..3 (LSB first) = 1, 1, 0
    1, // [80..81] "d" validity mask bits 0..2 (LSB first) = 1, 0
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
            data_offset: 36,
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
            data_offset: 12,
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
            data_offset: 24,
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
            data_offset: 41,
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

    run_list_wrapped_roundtrip_u32(
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

pub const GROUP_OPTION_GROUP_ABSENT_STR: [u8; 22] = [
    19, 0, 0,
    0, // [0..4] "a.s" payload start pointer (group absent: empty, at its lane start)
    0, 0, 0, 0, // [4..8] "a.s" byte length
    19, 0, 0, 0, // [8..12] "b" payload start pointer
    3, 0, 0, 0, // [12..16] "b" byte length
    0, // [16..17] "a.n" (parent group absent, slot left zeroed)
    7, // [17..18] "c" = 7
    0, // [18..19] validity bitvec (LSB first): bit 0 "a" present = 0
    120, 121, 122, // [19..22] "b" = "xyz"
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
            data_offset: 16,
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
            data_offset: 8,
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
            data_offset: 17,
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

    run_list_wrapped_roundtrip_u32(
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

pub const GROUP_OPTION_GROUP_ABSENT_WIDE_LIST: [u8; 36] = [
    24, 0, 0, 0, // [0..4] "a.v" payload start pointer (group absent: empty, 8-aligned)
    0, 0, 0, 0, // [4..8] "a.v" sample count
    24, 0, 0, 0, // [8..12] "b" payload start pointer
    3, 0, 0, 0, // [12..16] "b" sample count
    9, // [16..17] "d" = 9
    0, // [17..18] validity bitvec (LSB first): bit 0 "a" present = 0
    0, 0, 0, 0, 0, 0, // [18..24] alignment padding (6 bytes): "a.v" aligns to 8
    1, 0, 0, 0, // [24..28] "b"[0] = 1
    2, 0, 0, 0, // [28..32] "b"[1] = 2
    3, 0, 0, 0, // [32..36] "b"[2] = 3
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
            data_offset: 8,
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
            data_offset: 16,
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

    run_list_wrapped_roundtrip_u32(
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

pub const GROUP_OPTION_GROUP_ABSENT_NESTED_LIST: [u8; 35] = [
    32, 0, 0, 0, // [0..4] "a.m" topology start pointer (group absent: empty, word-aligned)
    0, 0, 0, 0, // [4..8] "a.m" topology word count
    32, 0, 0, 0, // [8..12] "a.m" payload start pointer
    0, 0, 0, 0, // [12..16] "a.m" sample count
    32, 0, 0, 0, // [16..20] "b" payload start pointer
    3, 0, 0, 0, // [20..24] "b" sample count
    0, 0, 0, 0, // [24..28] "a.k" (parent group absent, slot left zeroed)
    0, // [28..29] validity bitvec (LSB first): bit 0 "a" present = 0
    0, 0, 0, // [29..32] alignment padding (3 bytes): "a.m" topology aligns to the word
    5, 6, 7, // [32..35] "b" = 5, 6, 7
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
            data_offset: 8,
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
            data_offset: 24,
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
            data_offset: 16,
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

    run_list_wrapped_roundtrip_u32(
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

pub const GROUP_OPTION_GROUP_ABSENT_INSIDE_PRESENT: [u8; 29] = [
    28, 0, 0, 0, // [0..4] "a.inner.v" payload start pointer (inner group absent: empty)
    0, 0, 0, 0, // [4..8] "a.inner.v" sample count
    28, 0, 0, 0, // [8..12] "a.inner.w" payload start pointer (inner group absent: empty)
    0, 0, 0, 0, // [12..16] "a.inner.w" byte length
    28, 0, 0, 0, // [16..20] "b" payload start pointer
    1, 0, 0, 0, // [20..24] "b" sample count
    1, 2, // [24..26] "c" = 513
    3, // [26..27] "a.x" = 3
    1, // [27..28] validity bitvec (LSB first): bit 0 "a" present = 1, bit 1 "a.inner" present = 0
    1, // [28..29] "b"[0] = 1
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
            data_offset: 26,
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
            data_offset: 8,
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
            data_offset: 16,
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
            data_offset: 24,
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

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_OPTION_GROUP_ABSENT_INSIDE_PRESENT,
        &GROUP_OPTION_GROUP_ABSENT_INSIDE_PRESENT_INODES,
    );
}
