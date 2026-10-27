#[cfg(test)]
use crate::{PyroValue, value::Time, wire::tests::support::run_list_wrapped_roundtrip_u32};
#[cfg(test)]
use half::f16;
#[cfg(test)]
use pyro_schema::{
    PrimitiveDataType,
    human::{PyroField, PyroSchema, PyroType},
};

use pyro_schema::{Inode, KindMeta, WireKind};

// Every test below lays primitives out as the schema's own top-level
// fields, most with one or more `Group` layers among them — this file is
// entirely about verifying that wrapping primitives in `Group` layers does
// not change their fixed-region placement, per wiki/04-layout-and-alignment
// 4.5 and wiki/03-inodes 3.4.9/3.4.10: a `Group` with `nesting == 0`
// contributes no alignment/size of its own, so its descendants bucket-sort
// by descending alignment (16 -> 1) exactly as if they were declared flat,
// regardless of how many `Group` layers wrap them or what order they're
// declared in. `expected_buf`/`expected_inodes` below are computed
// independently from the production code (a standalone Python
// implementation of the same clause), not copied from its output.

pub const GROUP_PAIR_BOOL_BOOL: [u8; 2] = [1, 1];
pub const GROUP_PAIR_BOOL_BOOL_INODES: [Inode; 2] = [
    Inode {
        kind: WireKind::Bool,
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
        kind: WireKind::Bool,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 1,
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
fn test_group_pair_bool_bool() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "x".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::Bool,
                },
            },
            PyroField {
                name: "y".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::Bool,
                },
            },
        ],
    };

    let data =
        PyroValue::Group([("x", PyroValue::from(true)), ("y", PyroValue::from(true))].into());

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_PAIR_BOOL_BOOL,
        &GROUP_PAIR_BOOL_BOOL_INODES,
    );
}

pub const GROUP_PAIR_BOOL_U16: [u8; 3] = [49, 212, 1];
pub const GROUP_PAIR_BOOL_U16_INODES: [Inode; 2] = [
    Inode {
        kind: WireKind::U16,
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
        kind: WireKind::Bool,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 2,
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
fn test_group_pair_bool_u16() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "y".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U16,
                },
            },
            PyroField {
                name: "x".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::Bool,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("y", PyroValue::from(54321u16)),
            ("x", PyroValue::from(true)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_PAIR_BOOL_U16,
        &GROUP_PAIR_BOOL_U16_INODES,
    );
}

pub const GROUP_PAIR_BOOL_U32: [u8; 5] = [64, 226, 1, 0, 1];
pub const GROUP_PAIR_BOOL_U32_INODES: [Inode; 2] = [
    Inode {
        kind: WireKind::Bool,
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
        kind: WireKind::U32,
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
];
#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_pair_bool_u32() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "x".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::Bool,
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
    };

    let data = PyroValue::Group(
        [
            ("x", PyroValue::from(true)),
            ("y", PyroValue::from(123456u32)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_PAIR_BOOL_U32,
        &GROUP_PAIR_BOOL_U32_INODES,
    );
}

pub const GROUP_PAIR_BOOL_U64: [u8; 9] = [210, 10, 31, 235, 140, 169, 84, 171, 1];
pub const GROUP_PAIR_BOOL_U64_INODES: [Inode; 2] = [
    Inode {
        kind: WireKind::U64,
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
        kind: WireKind::Bool,
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
fn test_group_pair_bool_u64() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "y".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U64,
                },
            },
            PyroField {
                name: "x".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::Bool,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("y", PyroValue::from(12345678901234567890u64)),
            ("x", PyroValue::from(true)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_PAIR_BOOL_U64,
        &GROUP_PAIR_BOOL_U64_INODES,
    );
}

pub const GROUP_PAIR_BOOL_TIMESTAMP: [u8; 17] =
    [255, 201, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
pub const GROUP_PAIR_BOOL_TIMESTAMP_INODES: [Inode; 2] = [
    Inode {
        kind: WireKind::Bool,
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
    Inode {
        kind: WireKind::Timestamp,
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
];
#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_pair_bool_timestamp() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "x".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::Bool,
                },
            },
            PyroField {
                name: "y".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::Timestamp,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("x", PyroValue::from(true)),
            ("y", PyroValue::from(Time(999_999_999))),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_PAIR_BOOL_TIMESTAMP,
        &GROUP_PAIR_BOOL_TIMESTAMP_INODES,
    );
}

pub const GROUP_PAIR_U16_U16: [u8; 4] = [49, 212, 49, 212];
pub const GROUP_PAIR_U16_U16_INODES: [Inode; 2] = [
    Inode {
        kind: WireKind::U16,
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
        kind: WireKind::U16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 2,
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
fn test_group_pair_u16_u16() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "y".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U16,
                },
            },
            PyroField {
                name: "x".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U16,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("y", PyroValue::from(54321u16)),
            ("x", PyroValue::from(54321u16)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_PAIR_U16_U16,
        &GROUP_PAIR_U16_U16_INODES,
    );
}

pub const GROUP_PAIR_U16_U32: [u8; 6] = [64, 226, 1, 0, 49, 212];
pub const GROUP_PAIR_U16_U32_INODES: [Inode; 2] = [
    Inode {
        kind: WireKind::U16,
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
        kind: WireKind::U32,
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
];
#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_pair_u16_u32() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "x".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U16,
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
    };

    let data = PyroValue::Group(
        [
            ("x", PyroValue::from(54321u16)),
            ("y", PyroValue::from(123456u32)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_PAIR_U16_U32,
        &GROUP_PAIR_U16_U32_INODES,
    );
}

pub const GROUP_PAIR_U16_U64: [u8; 10] = [210, 10, 31, 235, 140, 169, 84, 171, 49, 212];
pub const GROUP_PAIR_U16_U64_INODES: [Inode; 2] = [
    Inode {
        kind: WireKind::U64,
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
        kind: WireKind::U16,
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
fn test_group_pair_u16_u64() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "y".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U64,
                },
            },
            PyroField {
                name: "x".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U16,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("y", PyroValue::from(12345678901234567890u64)),
            ("x", PyroValue::from(54321u16)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_PAIR_U16_U64,
        &GROUP_PAIR_U16_U64_INODES,
    );
}

pub const GROUP_PAIR_U16_TIMESTAMP: [u8; 18] = [
    255, 201, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 49, 212,
];
pub const GROUP_PAIR_U16_TIMESTAMP_INODES: [Inode; 2] = [
    Inode {
        kind: WireKind::U16,
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
    Inode {
        kind: WireKind::Timestamp,
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
];
#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_pair_u16_timestamp() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "x".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U16,
                },
            },
            PyroField {
                name: "y".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::Timestamp,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("x", PyroValue::from(54321u16)),
            ("y", PyroValue::from(Time(999_999_999))),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_PAIR_U16_TIMESTAMP,
        &GROUP_PAIR_U16_TIMESTAMP_INODES,
    );
}

pub const GROUP_PAIR_U32_U32: [u8; 8] = [64, 226, 1, 0, 64, 226, 1, 0];
pub const GROUP_PAIR_U32_U32_INODES: [Inode; 2] = [
    Inode {
        kind: WireKind::U32,
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
        kind: WireKind::U32,
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
];
#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_pair_u32_u32() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "y".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U32,
                },
            },
            PyroField {
                name: "x".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U32,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("y", PyroValue::from(123456u32)),
            ("x", PyroValue::from(123456u32)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_PAIR_U32_U32,
        &GROUP_PAIR_U32_U32_INODES,
    );
}

pub const GROUP_PAIR_U32_U64: [u8; 12] = [210, 10, 31, 235, 140, 169, 84, 171, 64, 226, 1, 0];
pub const GROUP_PAIR_U32_U64_INODES: [Inode; 2] = [
    Inode {
        kind: WireKind::U32,
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
        kind: WireKind::U64,
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
];
#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_pair_u32_u64() {
    let schema = PyroSchema {
        doc: String::new(),
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
                    primitive: PrimitiveDataType::U64,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("x", PyroValue::from(123456u32)),
            ("y", PyroValue::from(12345678901234567890u64)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_PAIR_U32_U64,
        &GROUP_PAIR_U32_U64_INODES,
    );
}

pub const GROUP_PAIR_U32_TIMESTAMP: [u8; 20] = [
    255, 201, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 64, 226, 1, 0,
];
pub const GROUP_PAIR_U32_TIMESTAMP_INODES: [Inode; 2] = [
    Inode {
        kind: WireKind::Timestamp,
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
        kind: WireKind::U32,
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
fn test_group_pair_u32_timestamp() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "y".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::Timestamp,
                },
            },
            PyroField {
                name: "x".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U32,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("y", PyroValue::from(Time(999_999_999))),
            ("x", PyroValue::from(123456u32)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_PAIR_U32_TIMESTAMP,
        &GROUP_PAIR_U32_TIMESTAMP_INODES,
    );
}

pub const GROUP_PAIR_U64_U64: [u8; 16] = [
    210, 10, 31, 235, 140, 169, 84, 171, 210, 10, 31, 235, 140, 169, 84, 171,
];
pub const GROUP_PAIR_U64_U64_INODES: [Inode; 2] = [
    Inode {
        kind: WireKind::U64,
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
        kind: WireKind::U64,
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
fn test_group_pair_u64_u64() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "x".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U64,
                },
            },
            PyroField {
                name: "y".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U64,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("x", PyroValue::from(12345678901234567890u64)),
            ("y", PyroValue::from(12345678901234567890u64)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_PAIR_U64_U64,
        &GROUP_PAIR_U64_U64_INODES,
    );
}

pub const GROUP_PAIR_U64_TIMESTAMP: [u8; 24] = [
    255, 201, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 210, 10, 31, 235, 140, 169, 84, 171,
];
pub const GROUP_PAIR_U64_TIMESTAMP_INODES: [Inode; 2] = [
    Inode {
        kind: WireKind::Timestamp,
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
        kind: WireKind::U64,
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
fn test_group_pair_u64_timestamp() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "y".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::Timestamp,
                },
            },
            PyroField {
                name: "x".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U64,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("y", PyroValue::from(Time(999_999_999))),
            ("x", PyroValue::from(12345678901234567890u64)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_PAIR_U64_TIMESTAMP,
        &GROUP_PAIR_U64_TIMESTAMP_INODES,
    );
}

pub const GROUP_PAIR_TIMESTAMP_TIMESTAMP: [u8; 32] = [
    255, 201, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 201, 154, 59, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0,
];
pub const GROUP_PAIR_TIMESTAMP_TIMESTAMP_INODES: [Inode; 2] = [
    Inode {
        kind: WireKind::Timestamp,
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
        kind: WireKind::Timestamp,
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
fn test_group_pair_timestamp_timestamp() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "x".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::Timestamp,
                },
            },
            PyroField {
                name: "y".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::Timestamp,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("x", PyroValue::from(Time(999_999_999))),
            ("y", PyroValue::from(Time(999_999_999))),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_PAIR_TIMESTAMP_TIMESTAMP,
        &GROUP_PAIR_TIMESTAMP_TIMESTAMP_INODES,
    );
}

pub const GROUP_FIVE_MIXED_ORDER_A: [u8; 31] = [
    255, 201, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 210, 10, 31, 235, 140, 169, 84, 171, 64,
    226, 1, 0, 49, 212, 1,
];
pub const GROUP_FIVE_MIXED_ORDER_A_INODES: [Inode; 5] = [
    Inode {
        kind: WireKind::U64,
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
    Inode {
        kind: WireKind::Bool,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 30,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Timestamp,
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
        kind: WireKind::U16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 28,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
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
            group: false,
            group_nest: false,
        },
    },
];
#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_five_mixed_order_a() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U64,
                },
            },
            PyroField {
                name: "b".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::Bool,
                },
            },
            PyroField {
                name: "c".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::Timestamp,
                },
            },
            PyroField {
                name: "d".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U16,
                },
            },
            PyroField {
                name: "e".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U32,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("a", PyroValue::from(12345678901234567890u64)),
            ("b", PyroValue::from(true)),
            ("c", PyroValue::from(Time(999_999_999))),
            ("d", PyroValue::from(54321u16)),
            ("e", PyroValue::from(123456u32)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_FIVE_MIXED_ORDER_A,
        &GROUP_FIVE_MIXED_ORDER_A_INODES,
    );
}

pub const GROUP_FIVE_MIXED_ORDER_B: [u8; 31] = [
    255, 201, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 210, 10, 31, 235, 140, 169, 84, 171, 64,
    226, 1, 0, 49, 212, 1,
];
pub const GROUP_FIVE_MIXED_ORDER_B_INODES: [Inode; 5] = [
    Inode {
        kind: WireKind::Timestamp,
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
        kind: WireKind::U32,
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
    Inode {
        kind: WireKind::Bool,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 30,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U64,
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
    Inode {
        kind: WireKind::U16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 28,
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
fn test_group_five_mixed_order_b() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::Timestamp,
                },
            },
            PyroField {
                name: "b".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U32,
                },
            },
            PyroField {
                name: "c".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::Bool,
                },
            },
            PyroField {
                name: "d".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U64,
                },
            },
            PyroField {
                name: "e".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U16,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("a", PyroValue::from(Time(999_999_999))),
            ("b", PyroValue::from(123456u32)),
            ("c", PyroValue::from(true)),
            ("d", PyroValue::from(12345678901234567890u64)),
            ("e", PyroValue::from(54321u16)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_FIVE_MIXED_ORDER_B,
        &GROUP_FIVE_MIXED_ORDER_B_INODES,
    );
}

pub const GROUP_SIGNED_INTS: [u8; 15] = [
    53, 251, 4, 142, 224, 254, 255, 255, 192, 29, 254, 255, 199, 207, 156,
];
pub const GROUP_SIGNED_INTS_INODES: [Inode; 4] = [
    Inode {
        kind: WireKind::I8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 14,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I16,
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
    Inode {
        kind: WireKind::I32,
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
        kind: WireKind::I64,
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
];
#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_signed_ints() {
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
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::I16,
                },
            },
            PyroField {
                name: "c".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::I32,
                },
            },
            PyroField {
                name: "d".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::I64,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("a", PyroValue::from(-100i8)),
            ("b", PyroValue::from(-12345i16)),
            ("c", PyroValue::from(-123456i32)),
            ("d", PyroValue::from(-1234567890123i64)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(schema, data, &GROUP_SIGNED_INTS, &GROUP_SIGNED_INTS_INODES);
}

pub const GROUP_U8_FLOATS: [u8; 15] = [
    155, 145, 4, 139, 10, 191, 5, 64, 195, 245, 72, 64, 0, 62, 200,
];
pub const GROUP_U8_FLOATS_INODES: [Inode; 4] = [
    Inode {
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 14,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::F16,
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
    Inode {
        kind: WireKind::F32,
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
        kind: WireKind::F64,
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
];
#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_u8_floats() {
    let schema = PyroSchema {
        doc: String::new(),
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
                    primitive: PrimitiveDataType::F16,
                },
            },
            PyroField {
                name: "c".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::F32,
                },
            },
            PyroField {
                name: "d".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::F64,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("a", PyroValue::from(200u8)),
            ("b", PyroValue::from(f16::from_f32(1.5))),
            ("c", PyroValue::from(3.14f32)),
            ("d", PyroValue::from(2.718281828f64)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(schema, data, &GROUP_U8_FLOATS, &GROUP_U8_FLOATS_INODES);
}

pub const GROUP_THREE_BYTE_ALIGNED: [u8; 3] = [1, 156, 200];
pub const GROUP_THREE_BYTE_ALIGNED_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::Bool,
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
        kind: WireKind::I8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 1,
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
            data_offset: 2,
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
fn test_group_three_byte_aligned() {
    let schema = PyroSchema {
        doc: String::new(),
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
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::I8,
                },
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
            ("a", PyroValue::from(true)),
            ("b", PyroValue::from(-100i8)),
            ("c", PyroValue::from(200u8)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_THREE_BYTE_ALIGNED,
        &GROUP_THREE_BYTE_ALIGNED_INODES,
    );
}

pub const GROUP_ALL_TWELVE_NATURAL_ORDER: [u8; 45] = [
    53, 251, 4, 142, 224, 254, 255, 255, 210, 10, 31, 235, 140, 169, 84, 171, 155, 145, 4, 139, 10,
    191, 5, 64, 192, 29, 254, 255, 64, 226, 1, 0, 195, 245, 72, 64, 199, 207, 49, 212, 0, 62, 1,
    156, 200,
];
pub const GROUP_ALL_TWELVE_NATURAL_ORDER_INODES: [Inode; 12] = [
    Inode {
        kind: WireKind::Bool,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 42,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 43,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I16,
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
    Inode {
        kind: WireKind::I32,
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
    Inode {
        kind: WireKind::I64,
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
            data_offset: 44,
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
            data_offset: 38,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
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
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U64,
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
        kind: WireKind::F16,
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
        kind: WireKind::F32,
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
    Inode {
        kind: WireKind::F64,
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
fn test_group_all_twelve_natural_order() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "f_bool".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::Bool,
                },
            },
            PyroField {
                name: "f_i8".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::I8,
                },
            },
            PyroField {
                name: "f_i16".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::I16,
                },
            },
            PyroField {
                name: "f_i32".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::I32,
                },
            },
            PyroField {
                name: "f_i64".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::I64,
                },
            },
            PyroField {
                name: "f_u8".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                },
            },
            PyroField {
                name: "f_u16".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U16,
                },
            },
            PyroField {
                name: "f_u32".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U32,
                },
            },
            PyroField {
                name: "f_u64".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U64,
                },
            },
            PyroField {
                name: "f_f16".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::F16,
                },
            },
            PyroField {
                name: "f_f32".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::F32,
                },
            },
            PyroField {
                name: "f_f64".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::F64,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("f_bool", PyroValue::from(true)),
            ("f_i8", PyroValue::from(-100i8)),
            ("f_i16", PyroValue::from(-12345i16)),
            ("f_i32", PyroValue::from(-123456i32)),
            ("f_i64", PyroValue::from(-1234567890123i64)),
            ("f_u8", PyroValue::from(200u8)),
            ("f_u16", PyroValue::from(54321u16)),
            ("f_u32", PyroValue::from(123456u32)),
            ("f_u64", PyroValue::from(12345678901234567890u64)),
            ("f_f16", PyroValue::from(f16::from_f32(1.5))),
            ("f_f32", PyroValue::from(3.14f32)),
            ("f_f64", PyroValue::from(2.718281828f64)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_ALL_TWELVE_NATURAL_ORDER,
        &GROUP_ALL_TWELVE_NATURAL_ORDER_INODES,
    );
}

pub const GROUP_ALL_TWELVE_REVERSE_ORDER: [u8; 45] = [
    155, 145, 4, 139, 10, 191, 5, 64, 210, 10, 31, 235, 140, 169, 84, 171, 53, 251, 4, 142, 224,
    254, 255, 255, 195, 245, 72, 64, 64, 226, 1, 0, 192, 29, 254, 255, 0, 62, 49, 212, 199, 207,
    200, 156, 1,
];
pub const GROUP_ALL_TWELVE_REVERSE_ORDER_INODES: [Inode; 12] = [
    Inode {
        kind: WireKind::F64,
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
        kind: WireKind::F32,
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
    Inode {
        kind: WireKind::F16,
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
    Inode {
        kind: WireKind::U64,
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
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 28,
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
            data_offset: 38,
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
            data_offset: 42,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I64,
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
    Inode {
        kind: WireKind::I32,
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
    Inode {
        kind: WireKind::I16,
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
        kind: WireKind::I8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 43,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Bool,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 44,
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
fn test_group_all_twelve_reverse_order() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "f_f64".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::F64,
                },
            },
            PyroField {
                name: "f_f32".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::F32,
                },
            },
            PyroField {
                name: "f_f16".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::F16,
                },
            },
            PyroField {
                name: "f_u64".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U64,
                },
            },
            PyroField {
                name: "f_u32".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U32,
                },
            },
            PyroField {
                name: "f_u16".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U16,
                },
            },
            PyroField {
                name: "f_u8".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                },
            },
            PyroField {
                name: "f_i64".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::I64,
                },
            },
            PyroField {
                name: "f_i32".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::I32,
                },
            },
            PyroField {
                name: "f_i16".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::I16,
                },
            },
            PyroField {
                name: "f_i8".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::I8,
                },
            },
            PyroField {
                name: "f_bool".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::Bool,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("f_f64", PyroValue::from(2.718281828f64)),
            ("f_f32", PyroValue::from(3.14f32)),
            ("f_f16", PyroValue::from(f16::from_f32(1.5))),
            ("f_u64", PyroValue::from(12345678901234567890u64)),
            ("f_u32", PyroValue::from(123456u32)),
            ("f_u16", PyroValue::from(54321u16)),
            ("f_u8", PyroValue::from(200u8)),
            ("f_i64", PyroValue::from(-1234567890123i64)),
            ("f_i32", PyroValue::from(-123456i32)),
            ("f_i16", PyroValue::from(-12345i16)),
            ("f_i8", PyroValue::from(-100i8)),
            ("f_bool", PyroValue::from(true)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_ALL_TWELVE_REVERSE_ORDER,
        &GROUP_ALL_TWELVE_REVERSE_ORDER_INODES,
    );
}

pub const GROUP_ALL_TWELVE_SHUFFLED_ORDER: [u8; 45] = [
    210, 10, 31, 235, 140, 169, 84, 171, 155, 145, 4, 139, 10, 191, 5, 64, 53, 251, 4, 142, 224,
    254, 255, 255, 195, 245, 72, 64, 192, 29, 254, 255, 64, 226, 1, 0, 49, 212, 0, 62, 199, 207, 1,
    156, 200,
];
pub const GROUP_ALL_TWELVE_SHUFFLED_ORDER_INODES: [Inode; 12] = [
    Inode {
        kind: WireKind::U64,
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
        kind: WireKind::Bool,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 42,
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
            data_offset: 24,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 43,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::F64,
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
        kind: WireKind::U16,
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
    Inode {
        kind: WireKind::I32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 28,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::F16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 38,
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
            data_offset: 44,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I64,
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
    Inode {
        kind: WireKind::I16,
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
        kind: WireKind::U32,
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
#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_all_twelve_shuffled_order() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "f_u64".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U64,
                },
            },
            PyroField {
                name: "f_bool".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::Bool,
                },
            },
            PyroField {
                name: "f_f32".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::F32,
                },
            },
            PyroField {
                name: "f_i8".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::I8,
                },
            },
            PyroField {
                name: "f_f64".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::F64,
                },
            },
            PyroField {
                name: "f_u16".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U16,
                },
            },
            PyroField {
                name: "f_i32".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::I32,
                },
            },
            PyroField {
                name: "f_f16".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::F16,
                },
            },
            PyroField {
                name: "f_u8".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                },
            },
            PyroField {
                name: "f_i64".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::I64,
                },
            },
            PyroField {
                name: "f_i16".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::I16,
                },
            },
            PyroField {
                name: "f_u32".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U32,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("f_u64", PyroValue::from(12345678901234567890u64)),
            ("f_bool", PyroValue::from(true)),
            ("f_f32", PyroValue::from(3.14f32)),
            ("f_i8", PyroValue::from(-100i8)),
            ("f_f64", PyroValue::from(2.718281828f64)),
            ("f_u16", PyroValue::from(54321u16)),
            ("f_i32", PyroValue::from(-123456i32)),
            ("f_f16", PyroValue::from(f16::from_f32(1.5))),
            ("f_u8", PyroValue::from(200u8)),
            ("f_i64", PyroValue::from(-1234567890123i64)),
            ("f_i16", PyroValue::from(-12345i16)),
            ("f_u32", PyroValue::from(123456u32)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_ALL_TWELVE_SHUFFLED_ORDER,
        &GROUP_ALL_TWELVE_SHUFFLED_ORDER_INODES,
    );
}

pub const GROUP_U8_THEN_NESTED_U64: [u8; 9] = [210, 10, 31, 235, 140, 169, 84, 171, 200];
pub const GROUP_U8_THEN_NESTED_U64_INODES: [Inode; 3] = [
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
    Inode {
        kind: WireKind::Group,
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
fn test_group_u8_then_nested_u64() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                },
            },
            PyroField {
                name: "inner".to_string(),
                doc: String::new(),
                data_type: PyroType::Group {
                    fields: vec![PyroField {
                        name: "b".to_string(),
                        doc: String::new(),
                        data_type: PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::U64,
                        },
                    }],
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("a", PyroValue::from(200u8)),
            (
                "inner",
                PyroValue::Group([("b", PyroValue::from(12345678901234567890u64))].into()),
            ),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_U8_THEN_NESTED_U64,
        &GROUP_U8_THEN_NESTED_U64_INODES,
    );
}

pub const GROUP_NESTED_U64_THEN_U8: [u8; 9] = [210, 10, 31, 235, 140, 169, 84, 171, 200];
pub const GROUP_NESTED_U64_THEN_U8_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::Group,
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
            group: false,
            group_nest: false,
        },
    },
];
#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_nested_u64_then_u8() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "inner".to_string(),
                doc: String::new(),
                data_type: PyroType::Group {
                    fields: vec![PyroField {
                        name: "b".to_string(),
                        doc: String::new(),
                        data_type: PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::U64,
                        },
                    }],
                },
            },
            PyroField {
                name: "a".to_string(),
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
                "inner",
                PyroValue::Group([("b", PyroValue::from(12345678901234567890u64))].into()),
            ),
            ("a", PyroValue::from(200u8)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_NESTED_U64_THEN_U8,
        &GROUP_NESTED_U64_THEN_U8_INODES,
    );
}

pub const GROUP_TIMESTAMP_THEN_NESTED_BOOL: [u8; 17] =
    [255, 201, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
pub const GROUP_TIMESTAMP_THEN_NESTED_BOOL_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::Timestamp,
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
        kind: WireKind::Group,
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
        kind: WireKind::Bool,
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
];
#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_timestamp_then_nested_bool() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::Timestamp,
                },
            },
            PyroField {
                name: "inner".to_string(),
                doc: String::new(),
                data_type: PyroType::Group {
                    fields: vec![PyroField {
                        name: "b".to_string(),
                        doc: String::new(),
                        data_type: PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::Bool,
                        },
                    }],
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("a", PyroValue::from(Time(999_999_999))),
            (
                "inner",
                PyroValue::Group([("b", PyroValue::from(true))].into()),
            ),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_TIMESTAMP_THEN_NESTED_BOOL,
        &GROUP_TIMESTAMP_THEN_NESTED_BOOL_INODES,
    );
}

pub const GROUP_BOOL_THEN_NESTED_TIMESTAMP: [u8; 17] =
    [255, 201, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
pub const GROUP_BOOL_THEN_NESTED_TIMESTAMP_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::Bool,
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
    Inode {
        kind: WireKind::Group,
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
        kind: WireKind::Timestamp,
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
fn test_group_bool_then_nested_timestamp() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::Bool,
                },
            },
            PyroField {
                name: "inner".to_string(),
                doc: String::new(),
                data_type: PyroType::Group {
                    fields: vec![PyroField {
                        name: "b".to_string(),
                        doc: String::new(),
                        data_type: PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::Timestamp,
                        },
                    }],
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("a", PyroValue::from(true)),
            (
                "inner",
                PyroValue::Group([("b", PyroValue::from(Time(999_999_999)))].into()),
            ),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_BOOL_THEN_NESTED_TIMESTAMP,
        &GROUP_BOOL_THEN_NESTED_TIMESTAMP_INODES,
    );
}

pub const GROUP_NESTED_TWO_THEN_U16: [u8; 7] = [64, 226, 1, 0, 49, 212, 200];
pub const GROUP_NESTED_TWO_THEN_U16_INODES: [Inode; 4] = [
    Inode {
        kind: WireKind::Group,
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
        kind: WireKind::U32,
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
            data_offset: 6,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U16,
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
];
#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_nested_two_then_u16() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "inner".to_string(),
                doc: String::new(),
                data_type: PyroType::Group {
                    fields: vec![
                        PyroField {
                            name: "b".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U32,
                            },
                        },
                        PyroField {
                            name: "c".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U8,
                            },
                        },
                    ],
                },
            },
            PyroField {
                name: "a".to_string(),
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
                "inner",
                PyroValue::Group(
                    [
                        ("b", PyroValue::from(123456u32)),
                        ("c", PyroValue::from(200u8)),
                    ]
                    .into(),
                ),
            ),
            ("a", PyroValue::from(54321u16)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_NESTED_TWO_THEN_U16,
        &GROUP_NESTED_TWO_THEN_U16_INODES,
    );
}

pub const GROUP_U16_THEN_NESTED_TWO: [u8; 7] = [64, 226, 1, 0, 49, 212, 200];
pub const GROUP_U16_THEN_NESTED_TWO_INODES: [Inode; 4] = [
    Inode {
        kind: WireKind::U16,
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
        kind: WireKind::Group,
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
            data_offset: 6,
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
fn test_group_u16_then_nested_two() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U16,
                },
            },
            PyroField {
                name: "inner".to_string(),
                doc: String::new(),
                data_type: PyroType::Group {
                    fields: vec![
                        PyroField {
                            name: "b".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U8,
                            },
                        },
                        PyroField {
                            name: "c".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U32,
                            },
                        },
                    ],
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("a", PyroValue::from(54321u16)),
            (
                "inner",
                PyroValue::Group(
                    [
                        ("b", PyroValue::from(200u8)),
                        ("c", PyroValue::from(123456u32)),
                    ]
                    .into(),
                ),
            ),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_U16_THEN_NESTED_TWO,
        &GROUP_U16_THEN_NESTED_TWO_INODES,
    );
}

pub const GROUP_TWO_SIBLING_NESTED_GROUPS_BIG_SMALL: [u8; 9] =
    [210, 10, 31, 235, 140, 169, 84, 171, 200];
pub const GROUP_TWO_SIBLING_NESTED_GROUPS_BIG_SMALL_INODES: [Inode; 4] = [
    Inode {
        kind: WireKind::Group,
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
        kind: WireKind::Group,
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
];
#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_two_sibling_nested_groups_big_small() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "inner_a".to_string(),
                doc: String::new(),
                data_type: PyroType::Group {
                    fields: vec![PyroField {
                        name: "x".to_string(),
                        doc: String::new(),
                        data_type: PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::U64,
                        },
                    }],
                },
            },
            PyroField {
                name: "inner_b".to_string(),
                doc: String::new(),
                data_type: PyroType::Group {
                    fields: vec![PyroField {
                        name: "y".to_string(),
                        doc: String::new(),
                        data_type: PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::U8,
                        },
                    }],
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            (
                "inner_a",
                PyroValue::Group([("x", PyroValue::from(12345678901234567890u64))].into()),
            ),
            (
                "inner_b",
                PyroValue::Group([("y", PyroValue::from(200u8))].into()),
            ),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_TWO_SIBLING_NESTED_GROUPS_BIG_SMALL,
        &GROUP_TWO_SIBLING_NESTED_GROUPS_BIG_SMALL_INODES,
    );
}

pub const GROUP_TWO_SIBLING_NESTED_GROUPS_SMALL_BIG: [u8; 9] =
    [210, 10, 31, 235, 140, 169, 84, 171, 200];
pub const GROUP_TWO_SIBLING_NESTED_GROUPS_SMALL_BIG_INODES: [Inode; 4] = [
    Inode {
        kind: WireKind::Group,
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
        kind: WireKind::Group,
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
fn test_group_two_sibling_nested_groups_small_big() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "inner_a".to_string(),
                doc: String::new(),
                data_type: PyroType::Group {
                    fields: vec![PyroField {
                        name: "x".to_string(),
                        doc: String::new(),
                        data_type: PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::U8,
                        },
                    }],
                },
            },
            PyroField {
                name: "inner_b".to_string(),
                doc: String::new(),
                data_type: PyroType::Group {
                    fields: vec![PyroField {
                        name: "y".to_string(),
                        doc: String::new(),
                        data_type: PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::U64,
                        },
                    }],
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            (
                "inner_a",
                PyroValue::Group([("x", PyroValue::from(200u8))].into()),
            ),
            (
                "inner_b",
                PyroValue::Group([("y", PyroValue::from(12345678901234567890u64))].into()),
            ),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_TWO_SIBLING_NESTED_GROUPS_SMALL_BIG,
        &GROUP_TWO_SIBLING_NESTED_GROUPS_SMALL_BIG_INODES,
    );
}

pub const GROUP_SCALAR_PLUS_TWO_SIBLING_GROUPS: [u8; 13] =
    [155, 145, 4, 139, 10, 191, 5, 64, 195, 245, 72, 64, 1];
pub const GROUP_SCALAR_PLUS_TWO_SIBLING_GROUPS_INODES: [Inode; 5] = [
    Inode {
        kind: WireKind::F32,
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
        kind: WireKind::Group,
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
        kind: WireKind::F64,
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
        kind: WireKind::Group,
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
        kind: WireKind::Bool,
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
];
#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_scalar_plus_two_sibling_groups() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::F32,
                },
            },
            PyroField {
                name: "inner_a".to_string(),
                doc: String::new(),
                data_type: PyroType::Group {
                    fields: vec![PyroField {
                        name: "x".to_string(),
                        doc: String::new(),
                        data_type: PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::F64,
                        },
                    }],
                },
            },
            PyroField {
                name: "inner_b".to_string(),
                doc: String::new(),
                data_type: PyroType::Group {
                    fields: vec![PyroField {
                        name: "y".to_string(),
                        doc: String::new(),
                        data_type: PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::Bool,
                        },
                    }],
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("a", PyroValue::from(3.14f32)),
            (
                "inner_a",
                PyroValue::Group([("x", PyroValue::from(2.718281828f64))].into()),
            ),
            (
                "inner_b",
                PyroValue::Group([("y", PyroValue::from(true))].into()),
            ),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_SCALAR_PLUS_TWO_SIBLING_GROUPS,
        &GROUP_SCALAR_PLUS_TWO_SIBLING_GROUPS_INODES,
    );
}

pub const GROUP_THREE_WAY_MIXED_NESTING: [u8; 12] =
    [210, 10, 31, 235, 140, 169, 84, 171, 49, 212, 1, 200];
pub const GROUP_THREE_WAY_MIXED_NESTING_INODES: [Inode; 6] = [
    Inode {
        kind: WireKind::Group,
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
        kind: WireKind::Bool,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 10,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U16,
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
            nesting: 0,
            data_offset: 11,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Group,
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
fn test_group_three_way_mixed_nesting() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "inner_a".to_string(),
                doc: String::new(),
                data_type: PyroType::Group {
                    fields: vec![
                        PyroField {
                            name: "x".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::Bool,
                            },
                        },
                        PyroField {
                            name: "y".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U16,
                            },
                        },
                    ],
                },
            },
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                },
            },
            PyroField {
                name: "inner_b".to_string(),
                doc: String::new(),
                data_type: PyroType::Group {
                    fields: vec![PyroField {
                        name: "z".to_string(),
                        doc: String::new(),
                        data_type: PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::U64,
                        },
                    }],
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            (
                "inner_a",
                PyroValue::Group(
                    [
                        ("x", PyroValue::from(true)),
                        ("y", PyroValue::from(54321u16)),
                    ]
                    .into(),
                ),
            ),
            ("a", PyroValue::from(200u8)),
            (
                "inner_b",
                PyroValue::Group([("z", PyroValue::from(12345678901234567890u64))].into()),
            ),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_THREE_WAY_MIXED_NESTING,
        &GROUP_THREE_WAY_MIXED_NESTING_INODES,
    );
}

pub const GROUP_TRIPLE_NESTED_SINGLE_LEAF: [u8; 8] = [210, 10, 31, 235, 140, 169, 84, 171];
pub const GROUP_TRIPLE_NESTED_SINGLE_LEAF_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::Group,
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
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 1,
            topology_offset: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U64,
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
fn test_group_triple_nested_single_leaf() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "level1".to_string(),
            doc: String::new(),
            data_type: PyroType::Group {
                fields: vec![PyroField {
                    name: "level2".to_string(),
                    doc: String::new(),
                    data_type: PyroType::Group {
                        fields: vec![PyroField {
                            name: "c".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U64,
                            },
                        }],
                    },
                }],
            },
        }],
    };

    let data = PyroValue::Group(
        [(
            "level2",
            PyroValue::Group([("c", PyroValue::from(12345678901234567890u64))].into()),
        )]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_TRIPLE_NESTED_SINGLE_LEAF,
        &GROUP_TRIPLE_NESTED_SINGLE_LEAF_INODES,
    );
}

pub const GROUP_MIXED_THREE_LEVEL: [u8; 15] = [
    210, 10, 31, 235, 140, 169, 84, 171, 64, 226, 1, 0, 49, 212, 200,
];
pub const GROUP_MIXED_THREE_LEVEL_INODES: [Inode; 6] = [
    Inode {
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 14,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Group,
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
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 2,
            topology_offset: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
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
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U64,
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
fn test_group_mixed_three_level() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                },
            },
            PyroField {
                name: "mid".to_string(),
                doc: String::new(),
                data_type: PyroType::Group {
                    fields: vec![
                        PyroField {
                            name: "b".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U16,
                            },
                        },
                        PyroField {
                            name: "deep".to_string(),
                            doc: String::new(),
                            data_type: PyroType::Group {
                                fields: vec![
                                    PyroField {
                                        name: "c".to_string(),
                                        doc: String::new(),
                                        data_type: PyroType::PrimitiveScalar {
                                            primitive: PrimitiveDataType::U32,
                                        },
                                    },
                                    PyroField {
                                        name: "d".to_string(),
                                        doc: String::new(),
                                        data_type: PyroType::PrimitiveScalar {
                                            primitive: PrimitiveDataType::U64,
                                        },
                                    },
                                ],
                            },
                        },
                    ],
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("a", PyroValue::from(200u8)),
            (
                "mid",
                PyroValue::Group(
                    [
                        ("b", PyroValue::from(54321u16)),
                        (
                            "deep",
                            PyroValue::Group(
                                [
                                    ("c", PyroValue::from(123456u32)),
                                    ("d", PyroValue::from(12345678901234567890u64)),
                                ]
                                .into(),
                            ),
                        ),
                    ]
                    .into(),
                ),
            ),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_MIXED_THREE_LEVEL,
        &GROUP_MIXED_THREE_LEVEL_INODES,
    );
}

pub const GROUP_FOUR_LEVEL_SINGLE_LEAF: [u8; 16] =
    [255, 201, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
pub const GROUP_FOUR_LEVEL_SINGLE_LEAF_INODES: [Inode; 4] = [
    Inode {
        kind: WireKind::Group,
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
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 1,
            topology_offset: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 1,
            topology_offset: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Timestamp,
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
fn test_group_four_level_single_leaf() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "l1".to_string(),
            doc: String::new(),
            data_type: PyroType::Group {
                fields: vec![PyroField {
                    name: "l2".to_string(),
                    doc: String::new(),
                    data_type: PyroType::Group {
                        fields: vec![PyroField {
                            name: "l3".to_string(),
                            doc: String::new(),
                            data_type: PyroType::Group {
                                fields: vec![PyroField {
                                    name: "x".to_string(),
                                    doc: String::new(),
                                    data_type: PyroType::PrimitiveScalar {
                                        primitive: PrimitiveDataType::Timestamp,
                                    },
                                }],
                            },
                        }],
                    },
                }],
            },
        }],
    };

    let data = PyroValue::Group(
        [(
            "l2",
            PyroValue::Group(
                [(
                    "l3",
                    PyroValue::Group([("x", PyroValue::from(Time(999_999_999)))].into()),
                )]
                .into(),
            ),
        )]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_FOUR_LEVEL_SINGLE_LEAF,
        &GROUP_FOUR_LEVEL_SINGLE_LEAF_INODES,
    );
}

pub const GROUP_MIXED_FOUR_LEVEL: [u8; 4] = [49, 212, 1, 200];
pub const GROUP_MIXED_FOUR_LEVEL_INODES: [Inode; 6] = [
    Inode {
        kind: WireKind::Group,
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
        kind: WireKind::Bool,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 2,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 2,
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
            data_offset: 3,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 1,
            topology_offset: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U16,
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
fn test_group_mixed_four_level() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::Group {
                fields: vec![
                    PyroField {
                        name: "x".to_string(),
                        doc: String::new(),
                        data_type: PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::Bool,
                        },
                    },
                    PyroField {
                        name: "y".to_string(),
                        doc: String::new(),
                        data_type: PyroType::Group {
                            fields: vec![
                                PyroField {
                                    name: "z".to_string(),
                                    doc: String::new(),
                                    data_type: PyroType::PrimitiveScalar {
                                        primitive: PrimitiveDataType::U8,
                                    },
                                },
                                PyroField {
                                    name: "l3".to_string(),
                                    doc: String::new(),
                                    data_type: PyroType::Group {
                                        fields: vec![PyroField {
                                            name: "v".to_string(),
                                            doc: String::new(),
                                            data_type: PyroType::PrimitiveScalar {
                                                primitive: PrimitiveDataType::U16,
                                            },
                                        }],
                                    },
                                },
                            ],
                        },
                    },
                ],
            },
        }],
    };

    let data = PyroValue::Group(
        [
            ("x", PyroValue::from(true)),
            (
                "y",
                PyroValue::Group(
                    [
                        ("z", PyroValue::from(200u8)),
                        (
                            "l3",
                            PyroValue::Group([("v", PyroValue::from(54321u16))].into()),
                        ),
                    ]
                    .into(),
                ),
            ),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_MIXED_FOUR_LEVEL,
        &GROUP_MIXED_FOUR_LEVEL_INODES,
    );
}

pub const GROUP_TWO_PARALLEL_DEEP_CHAINS: [u8; 9] = [210, 10, 31, 235, 140, 169, 84, 171, 200];
pub const GROUP_TWO_PARALLEL_DEEP_CHAINS_INODES: [Inode; 6] = [
    Inode {
        kind: WireKind::Group,
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
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 0,
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
            data_offset: 8,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Group,
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
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 1,
            topology_offset: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U64,
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
fn test_group_two_parallel_deep_chains() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "l1a".to_string(),
                doc: String::new(),
                data_type: PyroType::Group {
                    fields: vec![PyroField {
                        name: "l2a".to_string(),
                        doc: String::new(),
                        data_type: PyroType::Group {
                            fields: vec![PyroField {
                                name: "l3a".to_string(),
                                doc: String::new(),
                                data_type: PyroType::PrimitiveScalar {
                                    primitive: PrimitiveDataType::U8,
                                },
                            }],
                        },
                    }],
                },
            },
            PyroField {
                name: "l1b".to_string(),
                doc: String::new(),
                data_type: PyroType::Group {
                    fields: vec![PyroField {
                        name: "l2b".to_string(),
                        doc: String::new(),
                        data_type: PyroType::Group {
                            fields: vec![PyroField {
                                name: "l3b".to_string(),
                                doc: String::new(),
                                data_type: PyroType::PrimitiveScalar {
                                    primitive: PrimitiveDataType::U64,
                                },
                            }],
                        },
                    }],
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            (
                "l1a",
                PyroValue::Group(
                    [(
                        "l2a",
                        PyroValue::Group([("l3a", PyroValue::from(200u8))].into()),
                    )]
                    .into(),
                ),
            ),
            (
                "l1b",
                PyroValue::Group(
                    [(
                        "l2b",
                        PyroValue::Group(
                            [("l3b", PyroValue::from(12345678901234567890u64))].into(),
                        ),
                    )]
                    .into(),
                ),
            ),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_TWO_PARALLEL_DEEP_CHAINS,
        &GROUP_TWO_PARALLEL_DEEP_CHAINS_INODES,
    );
}

pub const GROUP_EIGHT_FLAT_FIELDS: [u8; 23] = [
    53, 251, 4, 142, 224, 254, 255, 255, 192, 29, 254, 255, 64, 226, 1, 0, 199, 207, 49, 212, 1,
    156, 200,
];
pub const GROUP_EIGHT_FLAT_FIELDS_INODES: [Inode; 8] = [
    Inode {
        kind: WireKind::Bool,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 20,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 21,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I16,
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
    Inode {
        kind: WireKind::I32,
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
        kind: WireKind::I64,
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
            data_offset: 22,
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
            data_offset: 18,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
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
            group: false,
            group_nest: false,
        },
    },
];
#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_eight_flat_fields() {
    let schema = PyroSchema {
        doc: String::new(),
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
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::I8,
                },
            },
            PyroField {
                name: "c".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::I16,
                },
            },
            PyroField {
                name: "d".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::I32,
                },
            },
            PyroField {
                name: "e".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::I64,
                },
            },
            PyroField {
                name: "f".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
                },
            },
            PyroField {
                name: "g".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U16,
                },
            },
            PyroField {
                name: "h".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U32,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("a", PyroValue::from(true)),
            ("b", PyroValue::from(-100i8)),
            ("c", PyroValue::from(-12345i16)),
            ("d", PyroValue::from(-123456i32)),
            ("e", PyroValue::from(-1234567890123i64)),
            ("f", PyroValue::from(200u8)),
            ("g", PyroValue::from(54321u16)),
            ("h", PyroValue::from(123456u32)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_EIGHT_FLAT_FIELDS,
        &GROUP_EIGHT_FLAT_FIELDS_INODES,
    );
}

pub const GROUP_FIVE_FLAT_PLUS_NESTED_PAIR: [u8; 40] = [
    255, 201, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 210, 10, 31, 235, 140, 169, 84, 171,
    155, 145, 4, 139, 10, 191, 5, 64, 195, 245, 72, 64, 0, 62, 1, 200,
];
pub const GROUP_FIVE_FLAT_PLUS_NESTED_PAIR_INODES: [Inode; 8] = [
    Inode {
        kind: WireKind::U64,
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
    Inode {
        kind: WireKind::F16,
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
    Inode {
        kind: WireKind::F32,
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
    Inode {
        kind: WireKind::F64,
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
    Inode {
        kind: WireKind::Timestamp,
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
        kind: WireKind::Group,
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
        kind: WireKind::Bool,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 38,
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
            data_offset: 39,
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
fn test_group_five_flat_plus_nested_pair() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U64,
                },
            },
            PyroField {
                name: "b".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::F16,
                },
            },
            PyroField {
                name: "c".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::F32,
                },
            },
            PyroField {
                name: "d".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::F64,
                },
            },
            PyroField {
                name: "e".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::Timestamp,
                },
            },
            PyroField {
                name: "inner".to_string(),
                doc: String::new(),
                data_type: PyroType::Group {
                    fields: vec![
                        PyroField {
                            name: "x".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::Bool,
                            },
                        },
                        PyroField {
                            name: "y".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
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
            ("a", PyroValue::from(12345678901234567890u64)),
            ("b", PyroValue::from(f16::from_f32(1.5))),
            ("c", PyroValue::from(3.14f32)),
            ("d", PyroValue::from(2.718281828f64)),
            ("e", PyroValue::from(Time(999_999_999))),
            (
                "inner",
                PyroValue::Group(
                    [("x", PyroValue::from(true)), ("y", PyroValue::from(200u8))].into(),
                ),
            ),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_FIVE_FLAT_PLUS_NESTED_PAIR,
        &GROUP_FIVE_FLAT_PLUS_NESTED_PAIR_INODES,
    );
}

pub const GROUP_TWO_NESTED_PAIRS_PLUS_TWO_SCALARS: [u8; 32] = [
    255, 201, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 210, 10, 31, 235, 140, 169, 84, 171, 64,
    226, 1, 0, 49, 212, 200, 1,
];
pub const GROUP_TWO_NESTED_PAIRS_PLUS_TWO_SCALARS_INODES: [Inode; 8] = [
    Inode {
        kind: WireKind::Group,
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
            data_offset: 30,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 28,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Group,
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
        kind: WireKind::U64,
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
        kind: WireKind::Bool,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 31,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Timestamp,
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
];
#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_two_nested_pairs_plus_two_scalars() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "inner1".to_string(),
                doc: String::new(),
                data_type: PyroType::Group {
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
                                primitive: PrimitiveDataType::U16,
                            },
                        },
                    ],
                },
            },
            PyroField {
                name: "inner2".to_string(),
                doc: String::new(),
                data_type: PyroType::Group {
                    fields: vec![
                        PyroField {
                            name: "c".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U32,
                            },
                        },
                        PyroField {
                            name: "d".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U64,
                            },
                        },
                    ],
                },
            },
            PyroField {
                name: "e".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::Bool,
                },
            },
            PyroField {
                name: "f".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::Timestamp,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            (
                "inner1",
                PyroValue::Group(
                    [
                        ("a", PyroValue::from(200u8)),
                        ("b", PyroValue::from(54321u16)),
                    ]
                    .into(),
                ),
            ),
            (
                "inner2",
                PyroValue::Group(
                    [
                        ("c", PyroValue::from(123456u32)),
                        ("d", PyroValue::from(12345678901234567890u64)),
                    ]
                    .into(),
                ),
            ),
            ("e", PyroValue::from(true)),
            ("f", PyroValue::from(Time(999_999_999))),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_TWO_NESTED_PAIRS_PLUS_TWO_SCALARS,
        &GROUP_TWO_NESTED_PAIRS_PLUS_TWO_SCALARS_INODES,
    );
}

pub const GROUP_ONE_SCALAR_PLUS_BIG_NESTED: [u8; 30] = [
    53, 251, 4, 142, 224, 254, 255, 255, 210, 10, 31, 235, 140, 169, 84, 171, 192, 29, 254, 255,
    64, 226, 1, 0, 199, 207, 49, 212, 156, 200,
];
pub const GROUP_ONE_SCALAR_PLUS_BIG_NESTED_INODES: [Inode; 9] = [
    Inode {
        kind: WireKind::I8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 28,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 7,
            topology_offset: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I16,
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
        kind: WireKind::I32,
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
        kind: WireKind::I64,
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
            data_offset: 29,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U16,
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
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 20,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U64,
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
];
#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_one_scalar_plus_big_nested() {
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
                name: "inner".to_string(),
                doc: String::new(),
                data_type: PyroType::Group {
                    fields: vec![
                        PyroField {
                            name: "b".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::I16,
                            },
                        },
                        PyroField {
                            name: "c".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::I32,
                            },
                        },
                        PyroField {
                            name: "d".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::I64,
                            },
                        },
                        PyroField {
                            name: "e".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U8,
                            },
                        },
                        PyroField {
                            name: "f".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U16,
                            },
                        },
                        PyroField {
                            name: "g".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U32,
                            },
                        },
                        PyroField {
                            name: "h".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U64,
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
            (
                "inner",
                PyroValue::Group(
                    [
                        ("b", PyroValue::from(-12345i16)),
                        ("c", PyroValue::from(-123456i32)),
                        ("d", PyroValue::from(-1234567890123i64)),
                        ("e", PyroValue::from(200u8)),
                        ("f", PyroValue::from(54321u16)),
                        ("g", PyroValue::from(123456u32)),
                        ("h", PyroValue::from(12345678901234567890u64)),
                    ]
                    .into(),
                ),
            ),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_ONE_SCALAR_PLUS_BIG_NESTED,
        &GROUP_ONE_SCALAR_PLUS_BIG_NESTED_INODES,
    );
}

pub const GROUP_ALL_TWELVE_WRAPPED_IN_NESTED_GROUP: [u8; 45] = [
    53, 251, 4, 142, 224, 254, 255, 255, 210, 10, 31, 235, 140, 169, 84, 171, 155, 145, 4, 139, 10,
    191, 5, 64, 192, 29, 254, 255, 64, 226, 1, 0, 195, 245, 72, 64, 199, 207, 49, 212, 0, 62, 1,
    156, 200,
];
pub const GROUP_ALL_TWELVE_WRAPPED_IN_NESTED_GROUP_INODES: [Inode; 13] = [
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 12,
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
            data_offset: 42,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 43,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 36,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I32,
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
        kind: WireKind::I64,
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
            data_offset: 44,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 38,
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
            data_offset: 28,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U64,
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
        kind: WireKind::F16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 40,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::F32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 32,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::F64,
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
];
#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_all_twelve_wrapped_in_nested_group() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "inner".to_string(),
            doc: String::new(),
            data_type: PyroType::Group {
                fields: vec![
                    PyroField {
                        name: "f_bool".to_string(),
                        doc: String::new(),
                        data_type: PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::Bool,
                        },
                    },
                    PyroField {
                        name: "f_i8".to_string(),
                        doc: String::new(),
                        data_type: PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::I8,
                        },
                    },
                    PyroField {
                        name: "f_i16".to_string(),
                        doc: String::new(),
                        data_type: PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::I16,
                        },
                    },
                    PyroField {
                        name: "f_i32".to_string(),
                        doc: String::new(),
                        data_type: PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::I32,
                        },
                    },
                    PyroField {
                        name: "f_i64".to_string(),
                        doc: String::new(),
                        data_type: PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::I64,
                        },
                    },
                    PyroField {
                        name: "f_u8".to_string(),
                        doc: String::new(),
                        data_type: PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::U8,
                        },
                    },
                    PyroField {
                        name: "f_u16".to_string(),
                        doc: String::new(),
                        data_type: PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::U16,
                        },
                    },
                    PyroField {
                        name: "f_u32".to_string(),
                        doc: String::new(),
                        data_type: PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::U32,
                        },
                    },
                    PyroField {
                        name: "f_u64".to_string(),
                        doc: String::new(),
                        data_type: PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::U64,
                        },
                    },
                    PyroField {
                        name: "f_f16".to_string(),
                        doc: String::new(),
                        data_type: PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::F16,
                        },
                    },
                    PyroField {
                        name: "f_f32".to_string(),
                        doc: String::new(),
                        data_type: PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::F32,
                        },
                    },
                    PyroField {
                        name: "f_f64".to_string(),
                        doc: String::new(),
                        data_type: PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::F64,
                        },
                    },
                ],
            },
        }],
    };

    let data = PyroValue::Group(
        [
            ("f_bool", PyroValue::from(true)),
            ("f_i8", PyroValue::from(-100i8)),
            ("f_i16", PyroValue::from(-12345i16)),
            ("f_i32", PyroValue::from(-123456i32)),
            ("f_i64", PyroValue::from(-1234567890123i64)),
            ("f_u8", PyroValue::from(200u8)),
            ("f_u16", PyroValue::from(54321u16)),
            ("f_u32", PyroValue::from(123456u32)),
            ("f_u64", PyroValue::from(12345678901234567890u64)),
            ("f_f16", PyroValue::from(f16::from_f32(1.5))),
            ("f_f32", PyroValue::from(3.14f32)),
            ("f_f64", PyroValue::from(2.718281828f64)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_ALL_TWELVE_WRAPPED_IN_NESTED_GROUP,
        &GROUP_ALL_TWELVE_WRAPPED_IN_NESTED_GROUP_INODES,
    );
}

pub const GROUP_SINGLE_FIELD_TIMESTAMP: [u8; 16] =
    [255, 201, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
pub const GROUP_SINGLE_FIELD_TIMESTAMP_INODES: [Inode; 1] = [Inode {
    kind: WireKind::Timestamp,
    kind_meta: KindMeta::Primitive {
        nesting: 0,
        data_offset: 0,
        topology_offset: 0,
        fixed_size: 0,
        nullable_bit_index: 0,
        group: false,
        group_nest: false,
    },
}];
#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_single_field_timestamp() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::PrimitiveScalar {
                primitive: PrimitiveDataType::Timestamp,
            },
        }],
    };

    let data = PyroValue::from(Time(999_999_999));

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_SINGLE_FIELD_TIMESTAMP,
        &GROUP_SINGLE_FIELD_TIMESTAMP_INODES,
    );
}

pub const GROUP_SINGLE_FIELD_BOOL: [u8; 1] = [1];
pub const GROUP_SINGLE_FIELD_BOOL_INODES: [Inode; 1] = [Inode {
    kind: WireKind::Bool,
    kind_meta: KindMeta::Primitive {
        nesting: 0,
        data_offset: 0,
        topology_offset: 0,
        fixed_size: 0,
        nullable_bit_index: 0,
        group: false,
        group_nest: false,
    },
}];
#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_single_field_bool() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::PrimitiveScalar {
                primitive: PrimitiveDataType::Bool,
            },
        }],
    };

    let data = PyroValue::from(true);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_SINGLE_FIELD_BOOL,
        &GROUP_SINGLE_FIELD_BOOL_INODES,
    );
}

pub const GROUP_NESTED_SINGLE_FIELD_BOOL: [u8; 1] = [1];
pub const GROUP_NESTED_SINGLE_FIELD_BOOL_INODES: [Inode; 2] = [
    Inode {
        kind: WireKind::Group,
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
        kind: WireKind::Bool,
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
fn test_group_nested_single_field_bool() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "inner".to_string(),
            doc: String::new(),
            data_type: PyroType::Group {
                fields: vec![PyroField {
                    name: "a".to_string(),
                    doc: String::new(),
                    data_type: PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::Bool,
                    },
                }],
            },
        }],
    };

    let data = PyroValue::Group([("a", PyroValue::from(true))].into());

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_NESTED_SINGLE_FIELD_BOOL,
        &GROUP_NESTED_SINGLE_FIELD_BOOL_INODES,
    );
}

pub const GROUP_FOUR_IDENTICAL_U8: [u8; 4] = [200, 200, 200, 200];
pub const GROUP_FOUR_IDENTICAL_U8_INODES: [Inode; 4] = [
    Inode {
        kind: WireKind::U8,
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
            data_offset: 1,
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
            data_offset: 2,
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
            data_offset: 3,
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
fn test_group_four_identical_u8() {
    let schema = PyroSchema {
        doc: String::new(),
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
            PyroField {
                name: "c".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U8,
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
            ("a", PyroValue::from(200u8)),
            ("b", PyroValue::from(200u8)),
            ("c", PyroValue::from(200u8)),
            ("d", PyroValue::from(200u8)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_FOUR_IDENTICAL_U8,
        &GROUP_FOUR_IDENTICAL_U8_INODES,
    );
}

pub const GROUP_THREE_IDENTICAL_U64: [u8; 24] = [
    210, 10, 31, 235, 140, 169, 84, 171, 210, 10, 31, 235, 140, 169, 84, 171, 210, 10, 31, 235,
    140, 169, 84, 171,
];
pub const GROUP_THREE_IDENTICAL_U64_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::U64,
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
        kind: WireKind::U64,
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
        kind: WireKind::U64,
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
fn test_group_three_identical_u64() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U64,
                },
            },
            PyroField {
                name: "b".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U64,
                },
            },
            PyroField {
                name: "c".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U64,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("a", PyroValue::from(12345678901234567890u64)),
            ("b", PyroValue::from(12345678901234567890u64)),
            ("c", PyroValue::from(12345678901234567890u64)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_THREE_IDENTICAL_U64,
        &GROUP_THREE_IDENTICAL_U64_INODES,
    );
}

pub const GROUP_TWO_IDENTICAL_TIMESTAMP: [u8; 32] = [
    255, 201, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 201, 154, 59, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0,
];
pub const GROUP_TWO_IDENTICAL_TIMESTAMP_INODES: [Inode; 2] = [
    Inode {
        kind: WireKind::Timestamp,
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
        kind: WireKind::Timestamp,
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
fn test_group_two_identical_timestamp() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::Timestamp,
                },
            },
            PyroField {
                name: "b".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::Timestamp,
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            ("a", PyroValue::from(Time(999_999_999))),
            ("b", PyroValue::from(Time(999_999_999))),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_TWO_IDENTICAL_TIMESTAMP,
        &GROUP_TWO_IDENTICAL_TIMESTAMP_INODES,
    );
}

pub const GROUP_NESTED_FOUR_IDENTICAL_U32: [u8; 16] =
    [64, 226, 1, 0, 64, 226, 1, 0, 64, 226, 1, 0, 64, 226, 1, 0];
pub const GROUP_NESTED_FOUR_IDENTICAL_U32_INODES: [Inode; 5] = [
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 4,
            topology_offset: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U32,
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
        kind: WireKind::U32,
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
        kind: WireKind::U32,
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
];
#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_nested_four_identical_u32() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "inner".to_string(),
            doc: String::new(),
            data_type: PyroType::Group {
                fields: vec![
                    PyroField {
                        name: "a".to_string(),
                        doc: String::new(),
                        data_type: PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::U32,
                        },
                    },
                    PyroField {
                        name: "b".to_string(),
                        doc: String::new(),
                        data_type: PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::U32,
                        },
                    },
                    PyroField {
                        name: "c".to_string(),
                        doc: String::new(),
                        data_type: PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::U32,
                        },
                    },
                    PyroField {
                        name: "d".to_string(),
                        doc: String::new(),
                        data_type: PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::U32,
                        },
                    },
                ],
            },
        }],
    };

    let data = PyroValue::Group(
        [
            ("a", PyroValue::from(123456u32)),
            ("b", PyroValue::from(123456u32)),
            ("c", PyroValue::from(123456u32)),
            ("d", PyroValue::from(123456u32)),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_NESTED_FOUR_IDENTICAL_U32,
        &GROUP_NESTED_FOUR_IDENTICAL_U32_INODES,
    );
}

/// The same 12 leaves as `test_group_all_twelve_natural_order`, in the
/// same declared order, but partitioned into three 4-field sibling
/// `Group`s (`inner_1`/`inner_2`/`inner_3`) instead of one flat group.
/// Where a leaf's descendant path crosses a `Group` boundary must not
/// affect its placement — only its own alignment and its position in
/// the flattened leaf sequence do (wiki/03-inodes 3.4.9/3.4.10,
/// wiki/04-layout-and-alignment 4.5). Asserts the resulting bytes are
/// byte-for-byte identical to the single-group version.
pub const GROUP_ALL_TWELVE_SPLIT_ACROSS_THREE_SIBLINGS_MATCHES_SINGLE_GROUP: [u8; 45] = [
    53, 251, 4, 142, 224, 254, 255, 255, 210, 10, 31, 235, 140, 169, 84, 171, 155, 145, 4, 139, 10,
    191, 5, 64, 192, 29, 254, 255, 64, 226, 1, 0, 195, 245, 72, 64, 199, 207, 49, 212, 0, 62, 1,
    156, 200,
];
pub const GROUP_ALL_TWELVE_SPLIT_ACROSS_THREE_SIBLINGS_MATCHES_SINGLE_GROUP_INODES: [Inode; 15] = [
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 4,
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
            data_offset: 42,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 43,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 36,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I32,
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
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 4,
            topology_offset: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I64,
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
            data_offset: 44,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 38,
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
            data_offset: 28,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 4,
            topology_offset: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U64,
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
        kind: WireKind::F16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 40,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::F32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 32,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::F64,
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
];
#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_all_twelve_split_across_three_siblings_matches_single_group() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "inner_1".to_string(),
                doc: String::new(),
                data_type: PyroType::Group {
                    fields: vec![
                        PyroField {
                            name: "f_bool".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::Bool,
                            },
                        },
                        PyroField {
                            name: "f_i8".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::I8,
                            },
                        },
                        PyroField {
                            name: "f_i16".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::I16,
                            },
                        },
                        PyroField {
                            name: "f_i32".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::I32,
                            },
                        },
                    ],
                },
            },
            PyroField {
                name: "inner_2".to_string(),
                doc: String::new(),
                data_type: PyroType::Group {
                    fields: vec![
                        PyroField {
                            name: "f_i64".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::I64,
                            },
                        },
                        PyroField {
                            name: "f_u8".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U8,
                            },
                        },
                        PyroField {
                            name: "f_u16".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U16,
                            },
                        },
                        PyroField {
                            name: "f_u32".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U32,
                            },
                        },
                    ],
                },
            },
            PyroField {
                name: "inner_3".to_string(),
                doc: String::new(),
                data_type: PyroType::Group {
                    fields: vec![
                        PyroField {
                            name: "f_u64".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U64,
                            },
                        },
                        PyroField {
                            name: "f_f16".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::F16,
                            },
                        },
                        PyroField {
                            name: "f_f32".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::F32,
                            },
                        },
                        PyroField {
                            name: "f_f64".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::F64,
                            },
                        },
                    ],
                },
            },
        ],
    };

    let data = PyroValue::Group(
        [
            (
                "inner_1",
                PyroValue::Group(
                    [
                        ("f_bool", PyroValue::from(true)),
                        ("f_i8", PyroValue::from(-100i8)),
                        ("f_i16", PyroValue::from(-12345i16)),
                        ("f_i32", PyroValue::from(-123456i32)),
                    ]
                    .into(),
                ),
            ),
            (
                "inner_2",
                PyroValue::Group(
                    [
                        ("f_i64", PyroValue::from(-1234567890123i64)),
                        ("f_u8", PyroValue::from(200u8)),
                        ("f_u16", PyroValue::from(54321u16)),
                        ("f_u32", PyroValue::from(123456u32)),
                    ]
                    .into(),
                ),
            ),
            (
                "inner_3",
                PyroValue::Group(
                    [
                        ("f_u64", PyroValue::from(12345678901234567890u64)),
                        ("f_f16", PyroValue::from(f16::from_f32(1.5))),
                        ("f_f32", PyroValue::from(3.14f32)),
                        ("f_f64", PyroValue::from(2.718281828f64)),
                    ]
                    .into(),
                ),
            ),
        ]
        .into(),
    );

    // Splitting the same 12 fields across however many sibling/nested
    // `Group`s must not change one single byte of the fixed-region
    // output — group boundaries carry no alignment or size of their
    // own (wiki/03-inodes 3.4.9/3.4.10), so only each leaf's own
    // alignment and its position in the flattened leaf sequence
    // determine its offset.
    assert_eq!(
        GROUP_ALL_TWELVE_SPLIT_ACROSS_THREE_SIBLINGS_MATCHES_SINGLE_GROUP,
        [
            53, 251, 4, 142, 224, 254, 255, 255, 210, 10, 31, 235, 140, 169, 84, 171, 155, 145, 4,
            139, 10, 191, 5, 64, 192, 29, 254, 255, 64, 226, 1, 0, 195, 245, 72, 64, 199, 207, 49,
            212, 0, 62, 1, 156, 200
        ]
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_ALL_TWELVE_SPLIT_ACROSS_THREE_SIBLINGS_MATCHES_SINGLE_GROUP,
        &GROUP_ALL_TWELVE_SPLIT_ACROSS_THREE_SIBLINGS_MATCHES_SINGLE_GROUP_INODES,
    );
}

// The empty group's own inode is what this pins, so it stays wrapped in
// `a`: a schema with no fields at all has no inodes.
pub const GROUP_EMPTY_BUF: [u8; 0] = [];
pub const GROUP_EMPTY_INODES: [Inode; 1] = [Inode {
    kind: WireKind::Group,
    kind_meta: KindMeta::Group {
        nesting: 0,
        count: 0,
        topology_offset: 0,
        nullable_bit_index: 0,
        group: false,
        group_nest: false,
    },
}];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_empty() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![PyroField {
            name: "a".to_string(),
            doc: String::new(),
            data_type: PyroType::Group { fields: vec![] },
        }],
    };

    let data = PyroValue::Group(crate::value::PyroRow::empty());

    run_list_wrapped_roundtrip_u32(schema, data, &GROUP_EMPTY_BUF, &GROUP_EMPTY_INODES);
}

#[cfg(test)]
use crate::value::PrimitiveValueList;

// ---------------------------------------------------------------------------
// gaps_ladder_ascending: Group{a: u8, b: List<u8>, c: List<u16>, d: List<u32>,
// e: List<u64>, f: List<Time>, g: Str}
// ---------------------------------------------------------------------------
//
// one list per alignment class, declared smallest alignment first. Byte ranges
// are `[start..end)` offsets into the buffer; captured from the value writer's
// output.

pub const GROUP_GAPS_LADDER_ASCENDING: [u8; 100] = [
    94, 0, 0, 0, // [0..4] "b" payload start pointer
    3, 0, 0, 0, // [4..8] "b" sample count
    92, 0, 0, 0, // [8..12] "c" payload start pointer
    1, 0, 0, 0, // [12..16] "c" sample count
    88, 0, 0, 0, // [16..20] "d" payload start pointer
    1, 0, 0, 0, // [20..24] "d" sample count
    80, 0, 0, 0, // [24..28] "e" payload start pointer
    1, 0, 0, 0, // [28..32] "e" sample count
    64, 0, 0, 0, // [32..36] "f" payload start pointer
    1, 0, 0, 0, // [36..40] "f" sample count
    97, 0, 0, 0, // [40..44] "g" payload start pointer
    3, 0, 0, 0, // [44..48] "g" payload byte count
    7, // [48..49] "a" = 7
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // [49..64] alignment padding (15 bytes)
    7, 202, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // [64..80] "f"[0] = Time(1000000007)
    6, 0, 0, 0, 0, 0, 0, 0, // [80..88] "e"[0] = 6
    5, 0, 0, 0, // [88..92] "d"[0] = 5
    4, 0, // [92..94] "c"[0] = 4
    1, // [94..95] "b"[0] = 1
    2, // [95..96] "b"[1] = 2
    3, // [96..97] "b"[2] = 3
    120, 121, 122, // [97..100] "g" payload "xyz"
];
pub const GROUP_GAPS_LADDER_ASCENDING_INODES: [Inode; 7] = [
    Inode {
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 48,
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
        kind: WireKind::U16,
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
        kind: WireKind::U32,
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
        kind: WireKind::U64,
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
        kind: WireKind::Timestamp,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 32,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Str,
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
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_gaps_ladder_ascending() {
    let schema = PyroSchema {
        doc: String::new(),
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
                data_type: PyroType::List {
                    element: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::U8,
                    }),
                },
            },
            PyroField {
                name: "c".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::U16,
                    }),
                },
            },
            PyroField {
                name: "d".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::U32,
                    }),
                },
            },
            PyroField {
                name: "e".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::U64,
                    }),
                },
            },
            PyroField {
                name: "f".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::Timestamp,
                    }),
                },
            },
            PyroField {
                name: "g".to_string(),
                doc: String::new(),
                data_type: PyroType::Str,
            },
        ],
    };
    let data = PyroValue::Group(
        [
            ("a", PyroValue::U8(7)),
            (
                "b",
                PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![1, 2, 3].into())),
            ),
            (
                "c",
                PyroValue::PrimitiveList(PrimitiveValueList::U16(vec![4].into())),
            ),
            (
                "d",
                PyroValue::PrimitiveList(PrimitiveValueList::U32(vec![5].into())),
            ),
            (
                "e",
                PyroValue::PrimitiveList(PrimitiveValueList::U64(vec![6].into())),
            ),
            ("f", PyroValue::from(vec![Time(1000000007)])),
            ("g", PyroValue::Str("xyz".into())),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_GAPS_LADDER_ASCENDING,
        &GROUP_GAPS_LADDER_ASCENDING_INODES,
    );
}

// ---------------------------------------------------------------------------
// gaps_ladder_descending: Group{a: List<Time>, b: List<u64>, c: List<u32>, d:
// List<u16>, e: List<u8>, f: bool, g: Bytes}
// ---------------------------------------------------------------------------
//
// one list per alignment class, declared largest alignment first, odd lengths.
// Byte ranges are `[start..end)` offsets into the buffer; captured from the
// value writer's output.

pub const GROUP_GAPS_LADDER_DESCENDING: [u8; 125] = [
    64, 0, 0, 0, // [0..4] "a" payload start pointer
    2, 0, 0, 0, // [4..8] "a" sample count
    96, 0, 0, 0, // [8..12] "b" payload start pointer
    1, 0, 0, 0, // [12..16] "b" sample count
    104, 0, 0, 0, // [16..20] "c" payload start pointer
    3, 0, 0, 0, // [20..24] "c" sample count
    116, 0, 0, 0, // [24..28] "d" payload start pointer
    1, 0, 0, 0, // [28..32] "d" sample count
    118, 0, 0, 0, // [32..36] "e" payload start pointer
    5, 0, 0, 0, // [36..40] "e" sample count
    123, 0, 0, 0, // [40..44] "g" payload start pointer
    2, 0, 0, 0, // [44..48] "g" payload byte count
    1, // [48..49] "f" = true
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // [49..64] alignment padding (15 bytes)
    7, 202, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // [64..80] "a"[0] = Time(1000000007)
    249, 53, 101, 196, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, // [80..96] "a"[1] = Time(-1000000007)
    1, 0, 0, 0, 0, 0, 0, 0, // [96..104] "b"[0] = 1
    2, 0, 0, 0, // [104..108] "c"[0] = 2
    3, 0, 0, 0, // [108..112] "c"[1] = 3
    4, 0, 0, 0, // [112..116] "c"[2] = 4
    5, 0,  // [116..118] "d"[0] = 5
    6,  // [118..119] "e"[0] = 6
    7,  // [119..120] "e"[1] = 7
    8,  // [120..121] "e"[2] = 8
    9,  // [121..122] "e"[3] = 9
    10, // [122..123] "e"[4] = 10
    9, 8, // [123..125] "g" payload [9, 8]
];
pub const GROUP_GAPS_LADDER_DESCENDING_INODES: [Inode; 7] = [
    Inode {
        kind: WireKind::Timestamp,
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
        kind: WireKind::U64,
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
        kind: WireKind::U32,
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
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 32,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Bool,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 48,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Bytes,
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
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_gaps_ladder_descending() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::Timestamp,
                    }),
                },
            },
            PyroField {
                name: "b".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::U64,
                    }),
                },
            },
            PyroField {
                name: "c".to_string(),
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
                data_type: PyroType::List {
                    element: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::U16,
                    }),
                },
            },
            PyroField {
                name: "e".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::U8,
                    }),
                },
            },
            PyroField {
                name: "f".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::Bool,
                },
            },
            PyroField {
                name: "g".to_string(),
                doc: String::new(),
                data_type: PyroType::Bytes,
            },
        ],
    };
    let data = PyroValue::Group(
        [
            (
                "a",
                PyroValue::from(vec![Time(1000000007), Time(-1000000007)]),
            ),
            (
                "b",
                PyroValue::PrimitiveList(PrimitiveValueList::U64(vec![1].into())),
            ),
            (
                "c",
                PyroValue::PrimitiveList(PrimitiveValueList::U32(vec![2, 3, 4].into())),
            ),
            (
                "d",
                PyroValue::PrimitiveList(PrimitiveValueList::U16(vec![5].into())),
            ),
            (
                "e",
                PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![6, 7, 8, 9, 10].into())),
            ),
            ("f", PyroValue::Bool(true)),
            ("g", PyroValue::Bytes(vec![9, 8].into())),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_GAPS_LADDER_DESCENDING,
        &GROUP_GAPS_LADDER_DESCENDING_INODES,
    );
}

// ---------------------------------------------------------------------------
// gaps_odd_lengths: Group{a: List<bool>, b: List<u8>, c: List<List<u64>>, d:
// List<[u8; 3]>, e: List<[u32; 3]>, f: Bytes, g: List<List<List<u16>>>, h:
// List<i64>}
// ---------------------------------------------------------------------------
//
// bit-packed, odd-length, fixed-array and multi-level lists whose payloads end
// off their neighbours' alignment. Byte ranges are `[start..end)` offsets into
// the buffer; captured from the value writer's output.

pub const GROUP_GAPS_ODD_LENGTHS: [u8; 185] = [
    172, 0, 0, 0, // [0..4] "a" payload start pointer
    5, 0, 0, 0, // [4..8] "a" sample count
    173, 0, 0, 0, // [8..12] "b" payload start pointer
    1, 0, 0, 0, // [12..16] "b" sample count
    80, 0, 0, 0, // [16..20] "c" topology start pointer
    3, 0, 0, 0, // [20..24] "c" topology word count
    120, 0, 0, 0, // [24..28] "c" payload start pointer
    3, 0, 0, 0, // [28..32] "c" sample count
    174, 0, 0, 0, // [32..36] "d" payload start pointer
    2, 0, 0, 0, // [36..40] "d" array count
    152, 0, 0, 0, // [40..44] "e" payload start pointer
    1, 0, 0, 0, // [44..48] "e" array count
    180, 0, 0, 0, // [48..52] "f" payload start pointer
    5, 0, 0, 0, // [52..56] "f" payload byte count
    92, 0, 0, 0, // [56..60] "g" topology start pointer
    7, 0, 0, 0, // [60..64] "g" topology word count
    164, 0, 0, 0, // [64..68] "g" payload start pointer
    4, 0, 0, 0, // [68..72] "g" sample count
    144, 0, 0, 0, // [72..76] "h" payload start pointer
    1, 0, 0, 0, // [76..80] "h" sample count
    255, 255, 255, 255, // [80..84] "c" topology level 0: sentinel
    1, 0, 0, 0, // [84..88] "c" topology level 0: boundary at 1
    3, 0, 0, 0, // [88..92] "c" topology level 0: boundary at 3
    255, 255, 255, 255, // [92..96] "g" topology level 0: sentinel
    2, 0, 0, 0, // [96..100] "g" topology level 0: boundary at 2
    3, 0, 0, 0, // [100..104] "g" topology level 0: boundary at 3
    255, 255, 255, 255, // [104..108] "g" topology level 1: sentinel
    1, 0, 0, 0, // [108..112] "g" topology level 1: boundary at 1
    3, 0, 0, 0, // [112..116] "g" topology level 1: boundary at 3
    4, 0, 0, 0, // [116..120] "g" topology level 1: boundary at 4
    1, 0, 0, 0, 0, 0, 0, 0, // [120..128] "c"[0] = 1
    2, 0, 0, 0, 0, 0, 0, 0, // [128..136] "c"[1] = 2
    3, 0, 0, 0, 0, 0, 0, 0, // [136..144] "c"[2] = 3
    255, 255, 255, 255, 255, 255, 255, 255, // [144..152] "h"[0] = -1
    7, 0, 0, 0, 8, 0, 0, 0, 9, 0, 0, 0, // [152..164] "e"[0] = [7, 8, 9]
    1, 0, // [164..166] "g"[0] = 1
    2, 0, // [166..168] "g"[1] = 2
    3, 0, // [168..170] "g"[2] = 3
    4, 0,  // [170..172] "g"[3] = 4
    13, // [172..173] "a" payload bits 0..5 (LSB first) = 1, 0, 1, 1, 0
    1,  // [173..174] "b"[0] = 1
    1, 2, 3, // [174..177] "d"[0] = [1, 2, 3]
    4, 5, 6, // [177..180] "d"[1] = [4, 5, 6]
    1, 2, 3, 4, 5, // [180..185] "f" payload [1, 2, 3, 4, 5]
];
pub const GROUP_GAPS_ODD_LENGTHS_INODES: [Inode; 8] = [
    Inode {
        kind: WireKind::Bool,
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
        kind: WireKind::U64,
        kind_meta: KindMeta::Primitive {
            nesting: 2,
            data_offset: 24,
            topology_offset: 16,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 32,
            topology_offset: 0,
            fixed_size: 3,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 40,
            topology_offset: 0,
            fixed_size: 3,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Bytes,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 48,
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
            nesting: 3,
            data_offset: 64,
            topology_offset: 56,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I64,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 72,
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
fn test_group_gaps_odd_lengths() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::Bool,
                    }),
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
                data_type: PyroType::List {
                    element: Box::new(PyroType::List {
                        element: Box::new(PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::U64,
                        }),
                    }),
                },
            },
            PyroField {
                name: "d".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::PrimitiveFixedList {
                        element: PrimitiveDataType::U8,
                        size: 3,
                    }),
                },
            },
            PyroField {
                name: "e".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::PrimitiveFixedList {
                        element: PrimitiveDataType::U32,
                        size: 3,
                    }),
                },
            },
            PyroField {
                name: "f".to_string(),
                doc: String::new(),
                data_type: PyroType::Bytes,
            },
            PyroField {
                name: "g".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::List {
                        element: Box::new(PyroType::List {
                            element: Box::new(PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::U16,
                            }),
                        }),
                    }),
                },
            },
            PyroField {
                name: "h".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::I64,
                    }),
                },
            },
        ],
    };
    let data = PyroValue::Group(
        [
            ("a", PyroValue::from(vec![true, false, true, true, false])),
            (
                "b",
                PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![1].into())),
            ),
            (
                "c",
                PyroValue::List(vec![
                    PyroValue::PrimitiveList(PrimitiveValueList::U64(vec![1].into())),
                    PyroValue::PrimitiveList(PrimitiveValueList::U64(vec![2, 3].into())),
                ]),
            ),
            (
                "d",
                PyroValue::List(vec![
                    PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![1, 2, 3].into())),
                    PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![4, 5, 6].into())),
                ]),
            ),
            (
                "e",
                PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::U32(
                    vec![7, 8, 9].into(),
                ))]),
            ),
            ("f", PyroValue::Bytes(vec![1, 2, 3, 4, 5].into())),
            (
                "g",
                PyroValue::List(vec![
                    PyroValue::List(vec![
                        PyroValue::PrimitiveList(PrimitiveValueList::U16(vec![1].into())),
                        PyroValue::PrimitiveList(PrimitiveValueList::U16(vec![2, 3].into())),
                    ]),
                    PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::U16(
                        vec![4].into(),
                    ))]),
                ]),
            ),
            (
                "h",
                PyroValue::PrimitiveList(PrimitiveValueList::I64(vec![-1].into())),
            ),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_GAPS_ODD_LENGTHS,
        &GROUP_GAPS_ODD_LENGTHS_INODES,
    );
}

// ---------------------------------------------------------------------------
// gaps_nested_groups: Group{a: Group{x: List<u16>, y: u8}, b: List<Group{p:
// u64, q: List<u8>}>, c: Group{r: List<Time>, s: bool}, d: Str}
// ---------------------------------------------------------------------------
//
// lists spread across nested groups and a list of groups. Byte ranges are
// `[start..end)` offsets into the buffer; captured from the value writer's
// output.

pub const GROUP_GAPS_NESTED_GROUPS: [u8; 124] = [
    112, 0, 0, 0, // [0..4] "a.x" payload start pointer
    3, 0, 0, 0, // [4..8] "a.x" sample count
    2, 0, 0, 0, // [8..12] group "b" count
    96, 0, 0, 0, // [12..16] "b.p" payload start pointer
    2, 0, 0, 0, // [16..20] "b.p" sample count
    56, 0, 0, 0, // [20..24] "b.q" topology start pointer
    3, 0, 0, 0, // [24..28] "b.q" topology word count
    118, 0, 0, 0, // [28..32] "b.q" payload start pointer
    3, 0, 0, 0, // [32..36] "b.q" sample count
    80, 0, 0, 0, // [36..40] "c.r" payload start pointer
    1, 0, 0, 0, // [40..44] "c.r" sample count
    121, 0, 0, 0, // [44..48] "d" payload start pointer
    3, 0, 0, 0, // [48..52] "d" payload byte count
    9, // [52..53] "a.y" = 9
    1, // [53..54] "c.s" = true
    0, 0, // [54..56] alignment padding (2 bytes)
    255, 255, 255, 255, // [56..60] "b.q" topology level 0: sentinel
    1, 0, 0, 0, // [60..64] "b.q" topology level 0: boundary at 1
    3, 0, 0, 0, // [64..68] "b.q" topology level 0: boundary at 3
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // [68..80] alignment padding (12 bytes)
    5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // [80..96] "c.r"[0] = Time(5)
    1, 0, 0, 0, 0, 0, 0, 0, // [96..104] "b.p"[0] = 1
    2, 0, 0, 0, 0, 0, 0, 0, // [104..112] "b.p"[1] = 2
    1, 0, // [112..114] "a.x"[0] = 1
    2, 0, // [114..116] "a.x"[1] = 2
    3, 0, // [116..118] "a.x"[2] = 3
    1, // [118..119] "b.q"[0] = 1
    2, // [119..120] "b.q"[1] = 2
    3, // [120..121] "b.q"[2] = 3
    111, 100, 100, // [121..124] "d" payload "odd"
];
pub const GROUP_GAPS_NESTED_GROUPS_INODES: [Inode; 10] = [
    Inode {
        kind: WireKind::Group,
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
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 52,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 1,
            count: 2,
            topology_offset: 8,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U64,
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
        kind: WireKind::U8,
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
    Inode {
        kind: WireKind::Group,
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
        kind: WireKind::Timestamp,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 36,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Bool,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 53,
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
            data_offset: 44,
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
fn test_group_gaps_nested_groups() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::Group {
                    fields: vec![
                        PyroField {
                            name: "x".to_string(),
                            doc: String::new(),
                            data_type: PyroType::List {
                                element: Box::new(PyroType::PrimitiveScalar {
                                    primitive: PrimitiveDataType::U16,
                                }),
                            },
                        },
                        PyroField {
                            name: "y".to_string(),
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
                data_type: PyroType::List {
                    element: Box::new(PyroType::Group {
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
                                data_type: PyroType::List {
                                    element: Box::new(PyroType::PrimitiveScalar {
                                        primitive: PrimitiveDataType::U8,
                                    }),
                                },
                            },
                        ],
                    }),
                },
            },
            PyroField {
                name: "c".to_string(),
                doc: String::new(),
                data_type: PyroType::Group {
                    fields: vec![
                        PyroField {
                            name: "r".to_string(),
                            doc: String::new(),
                            data_type: PyroType::List {
                                element: Box::new(PyroType::PrimitiveScalar {
                                    primitive: PrimitiveDataType::Timestamp,
                                }),
                            },
                        },
                        PyroField {
                            name: "s".to_string(),
                            doc: String::new(),
                            data_type: PyroType::PrimitiveScalar {
                                primitive: PrimitiveDataType::Bool,
                            },
                        },
                    ],
                },
            },
            PyroField {
                name: "d".to_string(),
                doc: String::new(),
                data_type: PyroType::Str,
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
                            PyroValue::PrimitiveList(PrimitiveValueList::U16(vec![1, 2, 3].into())),
                        ),
                        ("y", PyroValue::U8(9)),
                    ]
                    .into(),
                ),
            ),
            (
                "b",
                PyroValue::List(vec![
                    PyroValue::Group(
                        [
                            ("p", PyroValue::U64(1)),
                            (
                                "q",
                                PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![1].into())),
                            ),
                        ]
                        .into(),
                    ),
                    PyroValue::Group(
                        [
                            ("p", PyroValue::U64(2)),
                            (
                                "q",
                                PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![2, 3].into())),
                            ),
                        ]
                        .into(),
                    ),
                ]),
            ),
            (
                "c",
                PyroValue::Group(
                    [
                        ("r", PyroValue::from(vec![Time(5)])),
                        ("s", PyroValue::Bool(true)),
                    ]
                    .into(),
                ),
            ),
            ("d", PyroValue::Str("odd".into())),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_GAPS_NESTED_GROUPS,
        &GROUP_GAPS_NESTED_GROUPS_INODES,
    );
}

// ---------------------------------------------------------------------------
// gaps_empty_between: Group{a: List<u64>, b: List<u8>, c: List<Time>, d:
// List<u32>, e: Str, f: List<List<f64>>, g: List<f16>}
// ---------------------------------------------------------------------------
//
// empty lists of every alignment interleaved with non-empty ones. Byte ranges
// are `[start..end)` offsets into the buffer; captured from the value writer's
// output.

pub const GROUP_GAPS_EMPTY_BETWEEN: [u8; 97] = [
    80, 0, 0, 0, // [0..4] "a" payload start pointer
    0, 0, 0, 0, // [4..8] "a" sample count
    94, 0, 0, 0, // [8..12] "b" payload start pointer
    3, 0, 0, 0, // [12..16] "b" sample count
    80, 0, 0, 0, // [16..20] "c" payload start pointer
    0, 0, 0, 0, // [20..24] "c" sample count
    88, 0, 0, 0, // [24..28] "d" payload start pointer
    1, 0, 0, 0, // [28..32] "d" sample count
    97, 0, 0, 0, // [32..36] "e" payload start pointer
    0, 0, 0, 0, // [36..40] "e" payload byte count
    64, 0, 0, 0, // [40..44] "f" topology start pointer
    3, 0, 0, 0, // [44..48] "f" topology word count
    80, 0, 0, 0, // [48..52] "f" payload start pointer
    1, 0, 0, 0, // [52..56] "f" sample count
    92, 0, 0, 0, // [56..60] "g" payload start pointer
    1, 0, 0, 0, // [60..64] "g" sample count
    255, 255, 255, 255, // [64..68] "f" topology level 0: sentinel
    0, 0, 0, 0, // [68..72] "f" topology level 0: boundary at 0
    1, 0, 0, 0, // [72..76] "f" topology level 0: boundary at 1
    0, 0, 0, 0, // [76..80] alignment padding (4 bytes)
    0, 0, 0, 0, 0, 0, 4, 64, // [80..88] "f"[0] = 2.5
    1, 0, 0, 0, // [88..92] "d"[0] = 1
    0, 62, // [92..94] "g"[0] = 1.5
    1,  // [94..95] "b"[0] = 1
    2,  // [95..96] "b"[1] = 2
    3,  // [96..97] "b"[2] = 3
];
pub const GROUP_GAPS_EMPTY_BETWEEN_INODES: [Inode; 7] = [
    Inode {
        kind: WireKind::U64,
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
        kind: WireKind::Timestamp,
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
        kind: WireKind::U32,
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
        kind: WireKind::Str,
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
    Inode {
        kind: WireKind::F64,
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
        kind: WireKind::F16,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 56,
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
fn test_group_gaps_empty_between() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::U64,
                    }),
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
                data_type: PyroType::List {
                    element: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::Timestamp,
                    }),
                },
            },
            PyroField {
                name: "d".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::U32,
                    }),
                },
            },
            PyroField {
                name: "e".to_string(),
                doc: String::new(),
                data_type: PyroType::Str,
            },
            PyroField {
                name: "f".to_string(),
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
                name: "g".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::F16,
                    }),
                },
            },
        ],
    };
    let data = PyroValue::Group(
        [
            (
                "a",
                PyroValue::PrimitiveList(PrimitiveValueList::U64(vec![].into())),
            ),
            (
                "b",
                PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![1, 2, 3].into())),
            ),
            (
                "c",
                PyroValue::PrimitiveList(PrimitiveValueList::Timestamp(vec![].into())),
            ),
            (
                "d",
                PyroValue::PrimitiveList(PrimitiveValueList::U32(vec![1].into())),
            ),
            ("e", PyroValue::Str("".into())),
            (
                "f",
                PyroValue::List(vec![
                    PyroValue::PrimitiveList(PrimitiveValueList::F64(vec![].into())),
                    PyroValue::PrimitiveList(PrimitiveValueList::F64(vec![2.5].into())),
                ]),
            ),
            (
                "g",
                PyroValue::PrimitiveList(PrimitiveValueList::F16(vec![f16::from_f32(1.5)].into())),
            ),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_GAPS_EMPTY_BETWEEN,
        &GROUP_GAPS_EMPTY_BETWEEN_INODES,
    );
}

// ---------------------------------------------------------------------------
// gaps_fixed_arrays: Group{a: [u8; 3], b: List<[u16; 3]>, c: [u64; 1], d:
// List<[bool; 3]>, e: List<[f32; 2]>, f: [i16; 5]}
// ---------------------------------------------------------------------------
//
// fixed arrays inline and in lists, including a 3-bit bool array. Byte ranges
// are `[start..end)` offsets into the buffer; captured from the value writer's
// output.

pub const GROUP_GAPS_FIXED_ARRAYS: [u8; 63] = [
    56, 0, 0, 0, // [0..4] "b" payload start pointer
    1, 0, 0, 0, // [4..8] "b" array count
    62, 0, 0, 0, // [8..12] "d" payload start pointer
    2, 0, 0, 0, // [12..16] "d" array count
    48, 0, 0, 0, // [16..20] "e" payload start pointer
    1, 0, 0, 0, // [20..24] "e" array count
    7, 0, 0, 0, 0, 0, 0, 0, // [24..32] "c" = [7]
    255, 255, 254, 255, 253, 255, 252, 255, 251, 255, // [32..42] "f" = [-1, -2, -3, -4, -5]
    1, 2, 3, // [42..45] "a" = [1, 2, 3]
    0, 0, 0, // [45..48] alignment padding (3 bytes)
    0, 0, 192, 63, 0, 0, 32, 64, // [48..56] "e"[0] = [1.5, 2.5]
    1, 0, 2, 0, 3, 0,  // [56..62] "b"[0] = [1, 2, 3]
    37, // [62..63] "d" payload bits 0..6 (LSB first) = 1, 0, 1, 0, 0, 1
];
pub const GROUP_GAPS_FIXED_ARRAYS_INODES: [Inode; 6] = [
    Inode {
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 42,
            topology_offset: 0,
            fixed_size: 3,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U16,
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
        kind: WireKind::U64,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 24,
            topology_offset: 0,
            fixed_size: 1,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Bool,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 8,
            topology_offset: 0,
            fixed_size: 3,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::F32,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 16,
            topology_offset: 0,
            fixed_size: 2,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 32,
            topology_offset: 0,
            fixed_size: 5,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_gaps_fixed_arrays() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveFixedList {
                    element: PrimitiveDataType::U8,
                    size: 3,
                },
            },
            PyroField {
                name: "b".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::PrimitiveFixedList {
                        element: PrimitiveDataType::U16,
                        size: 3,
                    }),
                },
            },
            PyroField {
                name: "c".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveFixedList {
                    element: PrimitiveDataType::U64,
                    size: 1,
                },
            },
            PyroField {
                name: "d".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::PrimitiveFixedList {
                        element: PrimitiveDataType::Bool,
                        size: 3,
                    }),
                },
            },
            PyroField {
                name: "e".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::PrimitiveFixedList {
                        element: PrimitiveDataType::F32,
                        size: 2,
                    }),
                },
            },
            PyroField {
                name: "f".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveFixedList {
                    element: PrimitiveDataType::I16,
                    size: 5,
                },
            },
        ],
    };
    let data = PyroValue::Group(
        [
            (
                "a",
                PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![1, 2, 3].into())),
            ),
            (
                "b",
                PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::U16(
                    vec![1, 2, 3].into(),
                ))]),
            ),
            (
                "c",
                PyroValue::PrimitiveList(PrimitiveValueList::U64(vec![7].into())),
            ),
            (
                "d",
                PyroValue::List(vec![
                    PyroValue::from(vec![true, false, true]),
                    PyroValue::from(vec![false, false, true]),
                ]),
            ),
            (
                "e",
                PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::F32(
                    vec![1.5, 2.5].into(),
                ))]),
            ),
            (
                "f",
                PyroValue::PrimitiveList(PrimitiveValueList::I16(vec![-1, -2, -3, -4, -5].into())),
            ),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_GAPS_FIXED_ARRAYS,
        &GROUP_GAPS_FIXED_ARRAYS_INODES,
    );
}

// ---------------------------------------------------------------------------
// gaps_timestamp_after_1_inline_bytes: Group{a: [u8; 1], b: List<Time>}
// ---------------------------------------------------------------------------
//
// a `[u8; 1]` inline array ends the fixed region 1 bytes past the list slot,
// so the 16-byte aligned Timestamp payload lands after a different amount of
// padding. Byte ranges are `[start..end)` offsets into the buffer; captured
// from the value writer's output.

pub const GROUP_GAPS_TIMESTAMP_AFTER_1_INLINE_BYTES: [u8; 32] = [
    16, 0, 0, 0, // [0..4] "b" payload start pointer
    1, 0, 0, 0, // [4..8] "b" sample count
    1, // [8..9] "a" = [1]
    0, 0, 0, 0, 0, 0, 0, // [9..16] alignment padding (7 bytes)
    7, 202, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // [16..32] "b"[0] = Time(1000000007)
];
pub const GROUP_GAPS_TIMESTAMP_AFTER_1_INLINE_BYTES_INODES: [Inode; 2] = [
    Inode {
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 8,
            topology_offset: 0,
            fixed_size: 1,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Timestamp,
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
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_gaps_timestamp_after_1_inline_bytes() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveFixedList {
                    element: PrimitiveDataType::U8,
                    size: 1,
                },
            },
            PyroField {
                name: "b".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::Timestamp,
                    }),
                },
            },
        ],
    };
    let data = PyroValue::Group(
        [
            (
                "a",
                PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![1].into())),
            ),
            ("b", PyroValue::from(vec![Time(1000000007)])),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_GAPS_TIMESTAMP_AFTER_1_INLINE_BYTES,
        &GROUP_GAPS_TIMESTAMP_AFTER_1_INLINE_BYTES_INODES,
    );
}

// ---------------------------------------------------------------------------
// gaps_timestamp_after_3_inline_bytes: Group{a: [u8; 3], b: List<Time>}
// ---------------------------------------------------------------------------
//
// a `[u8; 3]` inline array ends the fixed region 3 bytes past the list slot,
// so the 16-byte aligned Timestamp payload lands after a different amount of
// padding. Byte ranges are `[start..end)` offsets into the buffer; captured
// from the value writer's output.

pub const GROUP_GAPS_TIMESTAMP_AFTER_3_INLINE_BYTES: [u8; 32] = [
    16, 0, 0, 0, // [0..4] "b" payload start pointer
    1, 0, 0, 0, // [4..8] "b" sample count
    1, 2, 3, // [8..11] "a" = [1, 2, 3]
    0, 0, 0, 0, 0, // [11..16] alignment padding (5 bytes)
    7, 202, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // [16..32] "b"[0] = Time(1000000007)
];
pub const GROUP_GAPS_TIMESTAMP_AFTER_3_INLINE_BYTES_INODES: [Inode; 2] = [
    Inode {
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 8,
            topology_offset: 0,
            fixed_size: 3,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Timestamp,
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
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_gaps_timestamp_after_3_inline_bytes() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveFixedList {
                    element: PrimitiveDataType::U8,
                    size: 3,
                },
            },
            PyroField {
                name: "b".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::Timestamp,
                    }),
                },
            },
        ],
    };
    let data = PyroValue::Group(
        [
            (
                "a",
                PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![1, 2, 3].into())),
            ),
            ("b", PyroValue::from(vec![Time(1000000007)])),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_GAPS_TIMESTAMP_AFTER_3_INLINE_BYTES,
        &GROUP_GAPS_TIMESTAMP_AFTER_3_INLINE_BYTES_INODES,
    );
}

// ---------------------------------------------------------------------------
// gaps_timestamp_after_6_inline_bytes: Group{a: [u8; 6], b: List<Time>}
// ---------------------------------------------------------------------------
//
// a `[u8; 6]` inline array ends the fixed region 6 bytes past the list slot,
// so the 16-byte aligned Timestamp payload lands after a different amount of
// padding. Byte ranges are `[start..end)` offsets into the buffer; captured
// from the value writer's output.

pub const GROUP_GAPS_TIMESTAMP_AFTER_6_INLINE_BYTES: [u8; 32] = [
    16, 0, 0, 0, // [0..4] "b" payload start pointer
    1, 0, 0, 0, // [4..8] "b" sample count
    1, 2, 3, 4, 5, 6, // [8..14] "a" = [1, 2, 3, 4, 5, 6]
    0, 0, // [14..16] alignment padding (2 bytes)
    7, 202, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // [16..32] "b"[0] = Time(1000000007)
];
pub const GROUP_GAPS_TIMESTAMP_AFTER_6_INLINE_BYTES_INODES: [Inode; 2] = [
    Inode {
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 8,
            topology_offset: 0,
            fixed_size: 6,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Timestamp,
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
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_gaps_timestamp_after_6_inline_bytes() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveFixedList {
                    element: PrimitiveDataType::U8,
                    size: 6,
                },
            },
            PyroField {
                name: "b".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::Timestamp,
                    }),
                },
            },
        ],
    };
    let data = PyroValue::Group(
        [
            (
                "a",
                PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![1, 2, 3, 4, 5, 6].into())),
            ),
            ("b", PyroValue::from(vec![Time(1000000007)])),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_GAPS_TIMESTAMP_AFTER_6_INLINE_BYTES,
        &GROUP_GAPS_TIMESTAMP_AFTER_6_INLINE_BYTES_INODES,
    );
}

// ---------------------------------------------------------------------------
// gaps_timestamp_after_9_inline_bytes: Group{a: [u8; 9], b: List<Time>}
// ---------------------------------------------------------------------------
//
// a `[u8; 9]` inline array ends the fixed region 9 bytes past the list slot,
// so the 16-byte aligned Timestamp payload lands after a different amount of
// padding. Byte ranges are `[start..end)` offsets into the buffer; captured
// from the value writer's output.

pub const GROUP_GAPS_TIMESTAMP_AFTER_9_INLINE_BYTES: [u8; 48] = [
    32, 0, 0, 0, // [0..4] "b" payload start pointer
    1, 0, 0, 0, // [4..8] "b" sample count
    1, 2, 3, 4, 5, 6, 7, 8, 9, // [8..17] "a" = [1, 2, 3, 4, 5, 6, 7, 8, 9]
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // [17..32] alignment padding (15 bytes)
    7, 202, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // [32..48] "b"[0] = Time(1000000007)
];
pub const GROUP_GAPS_TIMESTAMP_AFTER_9_INLINE_BYTES_INODES: [Inode; 2] = [
    Inode {
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 8,
            topology_offset: 0,
            fixed_size: 9,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Timestamp,
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
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_gaps_timestamp_after_9_inline_bytes() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveFixedList {
                    element: PrimitiveDataType::U8,
                    size: 9,
                },
            },
            PyroField {
                name: "b".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::Timestamp,
                    }),
                },
            },
        ],
    };
    let data = PyroValue::Group(
        [
            (
                "a",
                PyroValue::PrimitiveList(PrimitiveValueList::U8(
                    vec![1, 2, 3, 4, 5, 6, 7, 8, 9].into(),
                )),
            ),
            ("b", PyroValue::from(vec![Time(1000000007)])),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_GAPS_TIMESTAMP_AFTER_9_INLINE_BYTES,
        &GROUP_GAPS_TIMESTAMP_AFTER_9_INLINE_BYTES_INODES,
    );
}

// ---------------------------------------------------------------------------
// gaps_timestamp_after_12_inline_bytes: Group{a: [u8; 12], b: List<Time>}
// ---------------------------------------------------------------------------
//
// a `[u8; 12]` inline array ends the fixed region 12 bytes past the list slot,
// so the 16-byte aligned Timestamp payload lands after a different amount of
// padding. Byte ranges are `[start..end)` offsets into the buffer; captured
// from the value writer's output.

pub const GROUP_GAPS_TIMESTAMP_AFTER_12_INLINE_BYTES: [u8; 48] = [
    32, 0, 0, 0, // [0..4] "b" payload start pointer
    1, 0, 0, 0, // [4..8] "b" sample count
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11,
    12, // [8..20] "a" = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // [20..32] alignment padding (12 bytes)
    7, 202, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // [32..48] "b"[0] = Time(1000000007)
];
pub const GROUP_GAPS_TIMESTAMP_AFTER_12_INLINE_BYTES_INODES: [Inode; 2] = [
    Inode {
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 8,
            topology_offset: 0,
            fixed_size: 12,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Timestamp,
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
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_gaps_timestamp_after_12_inline_bytes() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveFixedList {
                    element: PrimitiveDataType::U8,
                    size: 12,
                },
            },
            PyroField {
                name: "b".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::Timestamp,
                    }),
                },
            },
        ],
    };
    let data = PyroValue::Group(
        [
            (
                "a",
                PyroValue::PrimitiveList(PrimitiveValueList::U8(
                    vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12].into(),
                )),
            ),
            ("b", PyroValue::from(vec![Time(1000000007)])),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_GAPS_TIMESTAMP_AFTER_12_INLINE_BYTES,
        &GROUP_GAPS_TIMESTAMP_AFTER_12_INLINE_BYTES_INODES,
    );
}

// ---------------------------------------------------------------------------
// gaps_timestamp_after_15_inline_bytes: Group{a: [u8; 15], b: List<Time>}
// ---------------------------------------------------------------------------
//
// a `[u8; 15]` inline array ends the fixed region 15 bytes past the list slot,
// so the 16-byte aligned Timestamp payload lands after a different amount of
// padding. Byte ranges are `[start..end)` offsets into the buffer; captured
// from the value writer's output.

pub const GROUP_GAPS_TIMESTAMP_AFTER_15_INLINE_BYTES: [u8; 48] = [
    32, 0, 0, 0, // [0..4] "b" payload start pointer
    1, 0, 0, 0, // [4..8] "b" sample count
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14,
    15, // [8..23] "a" = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
    0, 0, 0, 0, 0, 0, 0, 0, 0, // [23..32] alignment padding (9 bytes)
    7, 202, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // [32..48] "b"[0] = Time(1000000007)
];
pub const GROUP_GAPS_TIMESTAMP_AFTER_15_INLINE_BYTES_INODES: [Inode; 2] = [
    Inode {
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 8,
            topology_offset: 0,
            fixed_size: 15,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Timestamp,
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
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_gaps_timestamp_after_15_inline_bytes() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveFixedList {
                    element: PrimitiveDataType::U8,
                    size: 15,
                },
            },
            PyroField {
                name: "b".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::Timestamp,
                    }),
                },
            },
        ],
    };
    let data = PyroValue::Group(
        [
            (
                "a",
                PyroValue::PrimitiveList(PrimitiveValueList::U8(
                    vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15].into(),
                )),
            ),
            ("b", PyroValue::from(vec![Time(1000000007)])),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_GAPS_TIMESTAMP_AFTER_15_INLINE_BYTES,
        &GROUP_GAPS_TIMESTAMP_AFTER_15_INLINE_BYTES_INODES,
    );
}

// ---------------------------------------------------------------------------
// gaps_timestamp_after_topology: Group{a: [u8; 5], b: List<List<Time>>}
// ---------------------------------------------------------------------------
//
// a two-level Timestamp list whose topology words end off the 16-byte payload
// alignment, next to a `[u8; 5]` inline array. Byte ranges are `[start..end)`
// offsets into the buffer; captured from the value writer's output.

pub const GROUP_GAPS_TIMESTAMP_AFTER_TOPOLOGY: [u8; 96] = [
    24, 0, 0, 0, // [0..4] "b" topology start pointer
    4, 0, 0, 0, // [4..8] "b" topology word count
    48, 0, 0, 0, // [8..12] "b" payload start pointer
    3, 0, 0, 0, // [12..16] "b" sample count
    1, 2, 3, 4, 5, // [16..21] "a" = [1, 2, 3, 4, 5]
    0, 0, 0, // [21..24] alignment padding (3 bytes)
    255, 255, 255, 255, // [24..28] "b" topology level 0: sentinel
    1, 0, 0, 0, // [28..32] "b" topology level 0: boundary at 1
    1, 0, 0, 0, // [32..36] "b" topology level 0: boundary at 1
    3, 0, 0, 0, // [36..40] "b" topology level 0: boundary at 3
    0, 0, 0, 0, 0, 0, 0, 0, // [40..48] alignment padding (8 bytes)
    7, 202, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // [48..64] "b"[0] = Time(1000000007)
    7, 202, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // [64..80] "b"[1] = Time(1000000007)
    249, 53, 101, 196, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, // [80..96] "b"[2] = Time(-1000000007)
];
pub const GROUP_GAPS_TIMESTAMP_AFTER_TOPOLOGY_INODES: [Inode; 2] = [
    Inode {
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 16,
            topology_offset: 0,
            fixed_size: 5,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Timestamp,
        kind_meta: KindMeta::Primitive {
            nesting: 2,
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
fn test_group_gaps_timestamp_after_topology() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![
            PyroField {
                name: "a".to_string(),
                doc: String::new(),
                data_type: PyroType::PrimitiveFixedList {
                    element: PrimitiveDataType::U8,
                    size: 5,
                },
            },
            PyroField {
                name: "b".to_string(),
                doc: String::new(),
                data_type: PyroType::List {
                    element: Box::new(PyroType::List {
                        element: Box::new(PyroType::PrimitiveScalar {
                            primitive: PrimitiveDataType::Timestamp,
                        }),
                    }),
                },
            },
        ],
    };
    let data = PyroValue::Group(
        [
            (
                "a",
                PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![1, 2, 3, 4, 5].into())),
            ),
            (
                "b",
                PyroValue::List(vec![
                    PyroValue::from(vec![Time(1000000007)]),
                    PyroValue::PrimitiveList(PrimitiveValueList::Timestamp(vec![].into())),
                    PyroValue::from(vec![Time(1000000007), Time(-1000000007)]),
                ]),
            ),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &GROUP_GAPS_TIMESTAMP_AFTER_TOPOLOGY,
        &GROUP_GAPS_TIMESTAMP_AFTER_TOPOLOGY_INODES,
    );
}
