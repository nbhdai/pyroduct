//! Groups with no fields, in every position a group can take: bare,
//! nullable, beside a sibling, listed, nested lists, a nullable list, inside
//! a listed group, and as a map's value. An empty group has no data of its
//! own, so each table shows only what its position makes it own: a count, a
//! nullable bit, a validity mask, or a topology.

#[cfg(test)]
use pyro_schema::{
    PrimitiveDataType,
    human::{PyroField, PyroSchema, PyroType},
};

#[cfg(test)]
use crate::{PyroValue, value::PyroRow, wire::tests::support::run_list_wrapped_roundtrip_u32};

use pyro_schema::{Inode, KindMeta, WireKind};

#[cfg(test)]
fn field(name: &str, data_type: PyroType) -> PyroField {
    PyroField {
        name: name.to_string(),
        doc: String::new(),
        data_type,
    }
}

#[cfg(test)]
fn empty() -> PyroType {
    PyroType::Group { fields: vec![] }
}

#[cfg(test)]
fn list(element: PyroType) -> PyroType {
    PyroType::List {
        element: Box::new(element),
    }
}

#[cfg(test)]
fn u8_ty() -> PyroType {
    PyroType::PrimitiveScalar {
        primitive: PrimitiveDataType::U8,
    }
}

#[cfg(test)]
fn empty_row() -> PyroValue<'static> {
    PyroValue::Group(PyroRow::empty())
}

// `a: Group {}`: a bare empty group. No slot, no bytes.
#[rustfmt::skip]
pub const EMPTY_GROUP: [u8; 0] = [];
pub const EMPTY_GROUP_INODES: [Inode; 1] = [Inode {
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
fn test_empty_group() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![field("a", empty())],
    };

    let data = empty_row();

    run_list_wrapped_roundtrip_u32(schema, data, &EMPTY_GROUP, &EMPTY_GROUP_INODES);
}

// `a: Option<Group {}>`, present: no slot, just its bit in the
// nullable bitmap.
#[rustfmt::skip]
pub const EMPTY_OPTION_GROUP_SOME: [u8; 1] = [
    1, // 0 nullable bitmap: a present
];
pub const EMPTY_OPTION_GROUP_SOME_INODES: [Inode; 1] = [Inode {
    kind: WireKind::OptionGroup,
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
fn test_empty_option_group_some() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![field("a", PyroType::OptionGroup { fields: vec![] })],
    };

    let data = empty_row();

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &EMPTY_OPTION_GROUP_SOME,
        &EMPTY_OPTION_GROUP_SOME_INODES,
    );
}

// `a: Option<Group {}>`, absent.
#[rustfmt::skip]
pub const EMPTY_OPTION_GROUP_NONE: [u8; 1] = [
    0, // 0 nullable bitmap: a absent
];
pub const EMPTY_OPTION_GROUP_NONE_INODES: [Inode; 1] = [Inode {
    kind: WireKind::OptionGroup,
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
fn test_empty_option_group_none() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![field("a", PyroType::OptionGroup { fields: vec![] })],
    };

    let data = PyroValue::Null;

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &EMPTY_OPTION_GROUP_NONE,
        &EMPTY_OPTION_GROUP_NONE_INODES,
    );
}

// `{ x: U8, e: Group {} }`: the empty group takes nothing beside its
// sibling.
#[rustfmt::skip]
pub const EMPTY_GROUP_BESIDE_U8: [u8; 1] = [
    7, // 0 x
];
pub const EMPTY_GROUP_BESIDE_U8_INODES: [Inode; 2] = [
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
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 0,
            topology_offset: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_empty_group_beside_u8() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![field("x", u8_ty()), field("e", empty())],
    };

    let data = PyroValue::Group([("x", PyroValue::U8(7)), ("e", empty_row())].into());

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &EMPTY_GROUP_BESIDE_U8,
        &EMPTY_GROUP_BESIDE_U8_INODES,
    );
}

// `a: List<Group {}>` with two rows: the row count is the whole
// message.
#[rustfmt::skip]
pub const LIST_OF_EMPTY_GROUP: [u8; 4] = [
    2, 0, 0, 0, // 0 row count
];
pub const LIST_OF_EMPTY_GROUP_INODES: [Inode; 1] = [Inode {
    kind: WireKind::Group,
    kind_meta: KindMeta::Group {
        nesting: 1,
        count: 0,
        topology_offset: 0,
        nullable_bit_index: 0,
        group: false,
        group_nest: false,
    },
}];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_list_of_empty_group() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![field("a", list(empty()))],
    };

    let data = PyroValue::List(vec![empty_row(), empty_row()]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &LIST_OF_EMPTY_GROUP,
        &LIST_OF_EMPTY_GROUP_INODES,
    );
}

// `a: List<Group {}>` with no rows.
#[rustfmt::skip]
pub const LIST_OF_EMPTY_GROUP_NO_ROWS: [u8; 4] = [
    0, 0, 0, 0, // 0 row count
];
pub const LIST_OF_EMPTY_GROUP_NO_ROWS_INODES: [Inode; 1] = [Inode {
    kind: WireKind::Group,
    kind_meta: KindMeta::Group {
        nesting: 1,
        count: 0,
        topology_offset: 0,
        nullable_bit_index: 0,
        group: false,
        group_nest: false,
    },
}];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_list_of_empty_group_no_rows() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![field("a", list(empty()))],
    };

    let data = PyroValue::List(vec![]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &LIST_OF_EMPTY_GROUP_NO_ROWS,
        &LIST_OF_EMPTY_GROUP_NO_ROWS_INODES,
    );
}

// `a: List<List<Group {}>>` holding `[[{}], [], [{}, {}]]`: one level
// of list boundaries, and nothing below it.
#[rustfmt::skip]
pub const LIST_LIST_OF_EMPTY_GROUP: [u8; 24] = [
    8, 0, 0, 0, 4, 0, 0, 0, // 0 topology pointer, 4 words
    255, 255, 255, 255, // 8 level 0: sentinel
    1, 0, 0, 0, // 12 level 0: boundary at 1 group
    1, 0, 0, 0, // 16 level 0: boundary at 1 group (the empty list)
    3, 0, 0, 0, // 20 level 0: boundary at 3 groups
];
pub const LIST_LIST_OF_EMPTY_GROUP_INODES: [Inode; 1] = [Inode {
    kind: WireKind::Group,
    kind_meta: KindMeta::Group {
        nesting: 2,
        count: 0,
        topology_offset: 0,
        nullable_bit_index: 0,
        group: false,
        group_nest: false,
    },
}];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_list_list_of_empty_group() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![field("a", list(list(empty())))],
    };

    let data = PyroValue::List(vec![
        PyroValue::List(vec![empty_row()]),
        PyroValue::List(vec![]),
        PyroValue::List(vec![empty_row(), empty_row()]),
    ]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &LIST_LIST_OF_EMPTY_GROUP,
        &LIST_LIST_OF_EMPTY_GROUP_INODES,
    );
}

// `a: List<Option<Group {}>>` holding `[{}, null, {}]`: the validity
// mask is all there is.
#[rustfmt::skip]
pub const LIST_OF_EMPTY_OPTION_GROUP: [u8; 9] = [
    8, 0, 0, 0, 3, 0, 0, 0, // 0 mask pointer, 3 slots
    5, // 8 validity mask bits 0..3 (LSB first) = 1, 0, 1
];
pub const LIST_OF_EMPTY_OPTION_GROUP_INODES: [Inode; 1] = [Inode {
    kind: WireKind::OptionGroup,
    kind_meta: KindMeta::Group {
        nesting: 1,
        count: 0,
        topology_offset: 0,
        nullable_bit_index: 0,
        group: false,
        group_nest: false,
    },
}];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_list_of_empty_option_group() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![field("a", list(PyroType::OptionGroup { fields: vec![] }))],
    };

    let data = PyroValue::List(vec![empty_row(), PyroValue::Null, empty_row()]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &LIST_OF_EMPTY_OPTION_GROUP,
        &LIST_OF_EMPTY_OPTION_GROUP_INODES,
    );
}

// `a: List<Group { x: U8, e: Group {} }>` with two rows: the empty
// group is repeated by the list, so it owns a count of its own.
#[rustfmt::skip]
pub const EMPTY_GROUP_IN_LISTED_GROUP: [u8; 18] = [
    2, 0, 0, 0, // 0 row count
    16, 0, 0, 0, 2, 0, 0, 0, // 4 x (u8) pointer
    2, 0, 0, 0, // 12 e count
    1, 2, // 16 x
];
pub const EMPTY_GROUP_IN_LISTED_GROUP_INODES: [Inode; 3] = [
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
            count: 0,
            topology_offset: 12,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_empty_group_in_listed_group() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![field(
            "a",
            list(PyroType::Group {
                fields: vec![field("x", u8_ty()), field("e", empty())],
            }),
        )],
    };

    let data = PyroValue::List(vec![
        PyroValue::Group([("x", PyroValue::U8(1)), ("e", empty_row())].into()),
        PyroValue::Group([("x", PyroValue::U8(2)), ("e", empty_row())].into()),
    ]);

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &EMPTY_GROUP_IN_LISTED_GROUP,
        &EMPTY_GROUP_IN_LISTED_GROUP_INODES,
    );
}

// `a: Map<U8, Group {}>` with two entries: the value group owns a count
// beside the map's own.
#[rustfmt::skip]
pub const MAP_U8_KEY_EMPTY_GROUP_VALUE: [u8; 18] = [
    2, 0, 0, 0, // 0 Map count
    16, 0, 0, 0, 2, 0, 0, 0, // 4 u8 key pointer
    2, 0, 0, 0, // 12 value group count
    1, 2, // 16 keys
];
pub const MAP_U8_KEY_EMPTY_GROUP_VALUE_INODES: [Inode; 3] = [
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
            count: 0,
            topology_offset: 12,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_map_u8_key_empty_group_value() {
    let schema = PyroSchema {
        doc: String::new(),
        fields: vec![field(
            "a",
            PyroType::Map {
                key: Box::new(u8_ty()),
                value: Box::new(empty()),
            },
        )],
    };

    let data = PyroValue::Map(
        vec![
            (PyroValue::U8(1), empty_row()),
            (PyroValue::U8(2), empty_row()),
        ]
        .into(),
    );

    run_list_wrapped_roundtrip_u32(
        schema,
        data,
        &MAP_U8_KEY_EMPTY_GROUP_VALUE,
        &MAP_U8_KEY_EMPTY_GROUP_VALUE_INODES,
    );
}
