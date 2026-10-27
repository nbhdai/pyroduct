//! `List<List<Option<Group<..>>>>` with two different groups:
//! * `primitive`: `Option<Group{a: i16, b: bool, c: f64, d: Option<u8>}>`
//! * `blob`: `Option<Group{b: Vec<u8>}>`
//!
//! Shared value shape: `[[a, b], [c], [d, e, f]]`, each letter one group `0..6`;
//! scalar fields take the primitive files' sample lists, `Option<T>` fields
//! are `None` in groups 1 and 4, and blob fields take the `basic_*` blobs.
//! Groups 2 and 5 are absent: they take a topology slot and a clear
//! validity bit, and feed no field at all.
//!
//! Layout: the group slot, then each field's slot in declaration order
//! (`T`: `(start, count)`; `Option<T>`: `(start, valid, total)`;
//! `Str`/`Bytes`: a byte-boundary pointer, then `(start, byte count)`).
//! The dynamic region holds the group topology and validity mask, each blob's byte
//! boundaries, then the payloads in descending alignment (declaration order
//! within one), each padded to its own alignment. `Option<T>` payloads are
//! the valid values followed by their validity mask.

#[cfg(test)]
use pyro_schema::PrimitiveDataType;
#[cfg(test)]
use pyro_schema::human::{PyroField, PyroSchema, PyroType};

#[cfg(test)]
use crate::{PyroValue, wire::tests::support::run_list_wrapped_roundtrip_u16};

use pyro_schema::{Inode, KindMeta, WireKind};

/// The field list wrapped in `List` `nesting` times, as field `a`.
#[cfg(test)]
fn group_option_2_schema(fields: Vec<PyroField>) -> PyroSchema {
    let mut ty = PyroType::OptionGroup { fields };
    for _ in 0..2 {
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

#[cfg(test)]
fn field(name: &str, data_type: PyroType) -> PyroField {
    PyroField {
        name: name.to_string(),
        doc: String::new(),
        data_type,
    }
}

/// `[[a, b], [c], [d, e, f]]` over groups `0..6`, each built by `entry`.
#[cfg(test)]
fn group_option_2_value(entry: fn(usize) -> PyroValue<'static>) -> PyroValue<'static> {
    PyroValue::List(vec![
        PyroValue::List(vec![entry(0), entry(1)]),
        PyroValue::List(vec![entry(2)]),
        PyroValue::List(vec![entry(3), entry(4), entry(5)]),
    ])
}

// ---------------------------------------------------------------------------
// primitive: Option<Group{a: i16, b: bool, c: f64, d: Option<u8>}>
// ---------------------------------------------------------------------------

/// Group `i` of `primitive` (absent for 2 and 5).
#[cfg(test)]
fn group_option_2_primitive_entry(i: usize) -> PyroValue<'static> {
    if [2, 5].contains(&i) {
        return PyroValue::Null;
    }
    let a = [
        PyroValue::I16(-1),
        PyroValue::I16(2),
        PyroValue::I16(-3),
        PyroValue::I16(4),
        PyroValue::I16(-5),
        PyroValue::I16(6),
    ];
    let b = [
        PyroValue::Bool(true),
        PyroValue::Bool(false),
        PyroValue::Bool(true),
        PyroValue::Bool(true),
        PyroValue::Bool(true),
        PyroValue::Bool(false),
    ];
    let c = [
        PyroValue::F64(1.5),
        PyroValue::F64(-2.0),
        PyroValue::F64(3.25),
        PyroValue::F64(4.0),
        PyroValue::F64(-5.5),
        PyroValue::F64(6.0),
    ];
    let d = [
        PyroValue::U8(1),
        PyroValue::Null,
        PyroValue::U8(3),
        PyroValue::U8(4),
        PyroValue::Null,
        PyroValue::U8(6),
    ];
    PyroValue::Group(
        [
            ("a", a[i].clone()),
            ("b", b[i].clone()),
            ("c", c[i].clone()),
            ("d", d[i].clone()),
        ]
        .into(),
    )
}

pub const GROUP_OPTION_2_PRIMITIVE: [u8; 76] = [
    22, 0, // group slot: topology start pointer
    4, 0, // group slot: topology word count (1 level, sentinels included)
    64, 0, // "a" payload start pointer
    4, 0, // "a" sample count
    72, 0, // "b" payload start pointer
    4, 0, // "b" sample count
    32, 0, // "c" payload start pointer
    4, 0, // "c" sample count
    73, 0, // "d" payload start pointer
    2, 0, // "d" valid sample count
    4, 0, // "d" total sample count (nulls included)
    255, 255, // group level 0: sentinel
    2, 0, // group level 0: boundary at 2 groups (absent included)
    3, 0, // group level 0: boundary at 3 groups (absent included)
    6, 0, // group level 0: boundary at 6 groups (absent included)
    27, 0, // group validity mask (u32 words, LSB first) = 1, 1, 0, 1, 1, 0
    0, 0, 0, 0, 0, 0, 248, 63, // "c"[group 0] = 1.5
    0, 0, 0, 0, 0, 0, 0, 192, // "c"[group 1] = -2.0
    0, 0, 0, 0, 0, 0, 16, 64, // "c"[group 3] = 4.0
    0, 0, 0, 0, 0, 0, 22, 192, // "c"[group 4] = -5.5
    255, 255, // "a"[group 0] = -1
    2, 0, // "a"[group 1] = 2
    4, 0, // "a"[group 3] = 4
    251, 255, // "a"[group 4] = -5
    13,  // "b" payload bits (LSB first) = 1, 0, 1, 1
    1,   // "d"[group 0] = Some(1)
    4,   // "d"[group 3] = Some(4)
    5,   // "d" validity mask bits (LSB first) = 1, 0, 1, 0
];
pub const GROUP_OPTION_2_PRIMITIVE_INODES: [Inode; 5] = [
    Inode {
        kind: WireKind::OptionGroup,
        kind_meta: KindMeta::Group {
            nesting: 2,
            count: 4,
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
            data_offset: 8,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::F64,
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
        kind: WireKind::OptionU8,
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
fn test_group_option_2_primitive() {
    let schema = group_option_2_schema(vec![
        field(
            "a",
            PyroType::PrimitiveScalar {
                primitive: PrimitiveDataType::I16,
            },
        ),
        field(
            "b",
            PyroType::PrimitiveScalar {
                primitive: PrimitiveDataType::Bool,
            },
        ),
        field(
            "c",
            PyroType::PrimitiveScalar {
                primitive: PrimitiveDataType::F64,
            },
        ),
        field(
            "d",
            PyroType::OptionPrimitiveScalar {
                primitive: PrimitiveDataType::U8,
            },
        ),
    ]);
    run_list_wrapped_roundtrip_u16(
        schema,
        group_option_2_value(group_option_2_primitive_entry),
        &GROUP_OPTION_2_PRIMITIVE,
        &GROUP_OPTION_2_PRIMITIVE_INODES,
    );
}

// ---------------------------------------------------------------------------
// blob: Option<Group{b: Vec<u8>}>
// ---------------------------------------------------------------------------

/// Group `i` of `blob` (absent for 2 and 5).
#[cfg(test)]
fn group_option_2_blob_entry(i: usize) -> PyroValue<'static> {
    if [2, 5].contains(&i) {
        return PyroValue::Null;
    }
    let b = [
        PyroValue::Bytes(vec![1, 2].into()),
        PyroValue::Bytes(vec![].into()),
        PyroValue::Bytes(vec![3, 4, 5].into()),
        PyroValue::Bytes(vec![255].into()),
        PyroValue::Bytes(vec![0, 6, 7, 8].into()),
        PyroValue::Bytes(vec![9].into()),
    ];
    PyroValue::Group([("b", b[i].clone())].into())
}

pub const GROUP_OPTION_2_BLOB: [u8; 39] = [
    12, 0, // group slot: topology start pointer
    4, 0, // group slot: topology word count (1 level, sentinels included)
    22, 0, // "b" byte-boundary start pointer
    5, 0, // "b" byte-boundary word count
    32, 0, // "b" payload start pointer
    7, 0, // "b" payload byte count
    255, 255, // group level 0: sentinel
    2, 0, // group level 0: boundary at 2 groups (absent included)
    3, 0, // group level 0: boundary at 3 groups (absent included)
    6, 0, // group level 0: boundary at 6 groups (absent included)
    27, 0, // group validity mask (u32 words, LSB first) = 1, 1, 0, 1, 1, 0
    255, 255, // "b" byte boundaries: sentinel
    2, 0, // "b" byte boundary at 2 ([1, 2] ends)
    2, 0, // "b" byte boundary at 2 ([] ends)
    3, 0, // "b" byte boundary at 3 ([255] ends)
    7, 0, // "b" byte boundary at 7 ([0, 6, 7, 8] ends)
    1, 2,   // "b" payload [1, 2]
    255, // "b" payload [255]
    0, 6, 7, 8, // "b" payload [0, 6, 7, 8]
];
pub const GROUP_OPTION_2_BLOB_INODES: [Inode; 2] = [
    Inode {
        kind: WireKind::OptionGroup,
        kind_meta: KindMeta::Group {
            nesting: 2,
            count: 1,
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
            data_offset: 8,
            topology_offset: 4,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_option_2_blob() {
    let schema = group_option_2_schema(vec![field("b", PyroType::Bytes)]);
    run_list_wrapped_roundtrip_u16(
        schema,
        group_option_2_value(group_option_2_blob_entry),
        &GROUP_OPTION_2_BLOB,
        &GROUP_OPTION_2_BLOB_INODES,
    );
}

#[cfg(test)]
use crate::value::PrimitiveValueList;

// ---------------------------------------------------------------------------
// mixed: Option<Group{a: bool, b: Option<f32>, c: Str, d: u64, e: Option<i16>,
// f: [u8; 3]}>
// ---------------------------------------------------------------------------
//
// every slot shape a list-nested group field can take (bit-packed, `Option`,
// byte-boundary blob, 8-byte scalar, fixed array) side by side. Groups 2 and 5
// are absent. Byte ranges are `[start..end)` offsets into the buffer; captured
// from the value writer's output.

/// Group `i` of `mixed` (absent for 2 and 5).
#[cfg(test)]
fn group_option_2_mixed_entry(i: usize) -> PyroValue<'static> {
    if [2, 5].contains(&i) {
        return PyroValue::Null;
    }
    let a = [
        PyroValue::Bool(true),
        PyroValue::Bool(false),
        PyroValue::Bool(false),
        PyroValue::Bool(true),
        PyroValue::Bool(true),
        PyroValue::Bool(false),
    ];
    let b = [
        PyroValue::F32(1.5),
        PyroValue::Null,
        PyroValue::F32(-2.25),
        PyroValue::F32(8.0),
        PyroValue::Null,
        PyroValue::F32(0.5),
    ];
    let c = [
        PyroValue::Str("ab".into()),
        PyroValue::Str("".into()),
        PyroValue::Str("cde".into()),
        PyroValue::Str("é".into()),
        PyroValue::Str("fghij".into()),
        PyroValue::Str("k".into()),
    ];
    let d = [
        PyroValue::U64(1),
        PyroValue::U64(1099511627779),
        PyroValue::U64(3),
        PyroValue::U64(4),
        PyroValue::U64(18446744073709551615),
        PyroValue::U64(6),
    ];
    let e = [
        PyroValue::Null,
        PyroValue::I16(-2),
        PyroValue::I16(3),
        PyroValue::Null,
        PyroValue::I16(-5),
        PyroValue::I16(6),
    ];
    let f = [
        PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![1, 2, 3].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![4, 5, 6].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![7, 8, 9].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![10, 11, 12].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![13, 14, 15].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![16, 17, 18].into())),
    ];
    PyroValue::Group(
        [
            ("a", a[i].clone()),
            ("b", b[i].clone()),
            ("c", c[i].clone()),
            ("d", d[i].clone()),
            ("e", e[i].clone()),
            ("f", f[i].clone()),
        ]
        .into(),
    )
}

pub const GROUP_OPTION_2_MIXED: [u8; 125] = [
    36, 0, // [0..2] group topology start pointer
    4, 0, // [2..4] group topology word count
    103, 0, // [4..6] "a" payload start pointer
    4, 0, // [6..8] "a" sample count
    88, 0, // [8..10] "b" payload start pointer
    2, 0, // [10..12] "b" valid sample count
    4, 0, // [12..14] "b" total sample count (nulls included)
    46, 0, // [14..16] "c" byte-boundary start pointer
    5, 0, // [16..18] "c" byte-boundary word count
    104, 0, // [18..20] "c" payload start pointer
    9, 0, // [20..22] "c" payload byte count
    56, 0, // [22..24] "d" payload start pointer
    4, 0, // [24..26] "d" sample count
    98, 0, // [26..28] "e" payload start pointer
    2, 0, // [28..30] "e" valid sample count
    4, 0, // [30..32] "e" total sample count (nulls included)
    113, 0, // [32..34] "f" payload start pointer
    4, 0, // [34..36] "f" array count
    255, 255, // [36..38] group level 0: sentinel
    2, 0, // [38..40] group level 0: boundary at 2
    3, 0, // [40..42] group level 0: boundary at 3
    6, 0, // [42..44] group level 0: boundary at 6
    27, 0, // [44..46] group validity mask (u32 word 0, LSB first) = 1, 1, 0, 1, 1, 0
    255, 255, // [46..48] "c" byte-boundary level 0: sentinel
    2, 0, // [48..50] "c" byte-boundary level 0: boundary at 2
    2, 0, // [50..52] "c" byte-boundary level 0: boundary at 2
    4, 0, // [52..54] "c" byte-boundary level 0: boundary at 4
    9, 0, // [54..56] "c" byte-boundary level 0: boundary at 9
    1, 0, 0, 0, 0, 0, 0, 0, // [56..64] "d"[0] = 1
    3, 0, 0, 0, 0, 1, 0, 0, // [64..72] "d"[1] = 1099511627779
    4, 0, 0, 0, 0, 0, 0, 0, // [72..80] "d"[2] = 4
    255, 255, 255, 255, 255, 255, 255, 255, // [80..88] "d"[3] = 18446744073709551615
    0, 0, 192, 63, // [88..92] "b" valid[0] = 1.5
    0, 0, 0, 65, // [92..96] "b" valid[1] = 8.0
    5,  // [96..97] "b" validity mask bits 0..4 (LSB first) = 1, 0, 1, 0
    0,  // [97..98] alignment padding (1 byte)
    254, 255, // [98..100] "e" valid[0] = -2
    251, 255, // [100..102] "e" valid[1] = -5
    10,  // [102..103] "e" validity mask bits 0..4 (LSB first) = 0, 1, 0, 1
    13,  // [103..104] "a" payload bits 0..4 (LSB first) = 1, 0, 1, 1
    97, 98, // [104..106] "c" payload "ab"
    195, 169, // [106..108] "c" payload "é"
    102, 103, 104, 105, 106, // [108..113] "c" payload "fghij"
    1, 2, 3, // [113..116] "f"[0] = [1, 2, 3]
    4, 5, 6, // [116..119] "f"[1] = [4, 5, 6]
    10, 11, 12, // [119..122] "f"[2] = [10, 11, 12]
    13, 14, 15, // [122..125] "f"[3] = [13, 14, 15]
];
pub const GROUP_OPTION_2_MIXED_INODES: [Inode; 7] = [
    Inode {
        kind: WireKind::OptionGroup,
        kind_meta: KindMeta::Group {
            nesting: 2,
            count: 6,
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
            data_offset: 8,
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
            data_offset: 18,
            topology_offset: 14,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::U64,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 22,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::OptionI16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 26,
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
            data_offset: 32,
            topology_offset: 0,
            fixed_size: 3,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_option_2_mixed() {
    let schema = group_option_2_schema(vec![
        field(
            "a",
            PyroType::PrimitiveScalar {
                primitive: PrimitiveDataType::Bool,
            },
        ),
        field(
            "b",
            PyroType::OptionPrimitiveScalar {
                primitive: PrimitiveDataType::F32,
            },
        ),
        field("c", PyroType::Str),
        field(
            "d",
            PyroType::PrimitiveScalar {
                primitive: PrimitiveDataType::U64,
            },
        ),
        field(
            "e",
            PyroType::OptionPrimitiveScalar {
                primitive: PrimitiveDataType::I16,
            },
        ),
        field(
            "f",
            PyroType::PrimitiveFixedList {
                element: PrimitiveDataType::U8,
                size: 3,
            },
        ),
    ]);
    run_list_wrapped_roundtrip_u16(
        schema,
        group_option_2_value(group_option_2_mixed_entry),
        &GROUP_OPTION_2_MIXED,
        &GROUP_OPTION_2_MIXED_INODES,
    );
}

// ---------------------------------------------------------------------------
// nested: Option<Group{a: u8, b: List<List<u64>>, c: List<u16>, d:
// Option<i32>}>
// ---------------------------------------------------------------------------
//
// a double-nested field and a single-nested field (with empty inner lists)
// next to plain and `Option` scalars, so each field's own topology sits
// between the group topology and payloads of four different alignments. Groups
// 2 and 5 are absent. Byte ranges are `[start..end)` offsets into the buffer;
// captured from the value writer's output.

/// Group `i` of `nested` (absent for 2 and 5).
#[cfg(test)]
fn group_option_2_nested_entry(i: usize) -> PyroValue<'static> {
    if [2, 5].contains(&i) {
        return PyroValue::Null;
    }
    let a = [
        PyroValue::U8(1),
        PyroValue::U8(2),
        PyroValue::U8(3),
        PyroValue::U8(4),
        PyroValue::U8(5),
        PyroValue::U8(6),
    ];
    let b = [
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::U64(vec![1].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::U64(vec![2, 3].into())),
        ]),
        PyroValue::List(vec![]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::U64(
            vec![].into(),
        ))]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::U64(
            vec![4].into(),
        ))]),
        PyroValue::List(vec![
            PyroValue::PrimitiveList(PrimitiveValueList::U64(vec![5, 6].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::U64(vec![].into())),
            PyroValue::PrimitiveList(PrimitiveValueList::U64(vec![7].into())),
        ]),
        PyroValue::List(vec![PyroValue::PrimitiveList(PrimitiveValueList::U64(
            vec![8].into(),
        ))]),
    ];
    let c = [
        PyroValue::PrimitiveList(PrimitiveValueList::U16(vec![1, 2, 3].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::U16(vec![].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::U16(vec![4].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::U16(vec![5, 6].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::U16(vec![].into())),
        PyroValue::PrimitiveList(PrimitiveValueList::U16(vec![7].into())),
    ];
    let d = [
        PyroValue::I32(-1),
        PyroValue::Null,
        PyroValue::I32(3),
        PyroValue::I32(-4),
        PyroValue::Null,
        PyroValue::I32(6),
    ];
    PyroValue::Group(
        [
            ("a", a[i].clone()),
            ("b", b[i].clone()),
            ("c", c[i].clone()),
            ("d", d[i].clone()),
        ]
        .into(),
    )
}

pub const GROUP_OPTION_2_NESTED: [u8; 160] = [
    30, 0, // [0..2] group topology start pointer
    4, 0, // [2..4] group topology word count
    156, 0, // [4..6] "a" payload start pointer
    4, 0, // [6..8] "a" sample count
    40, 0, // [8..10] "b" topology start pointer
    12, 0, // [10..12] "b" topology word count
    80, 0, // [12..14] "b" payload start pointer
    7, 0, // [14..16] "b" sample count
    64, 0, // [16..18] "c" topology start pointer
    5, 0, // [18..20] "c" topology word count
    146, 0, // [20..22] "c" payload start pointer
    5, 0, // [22..24] "c" sample count
    136, 0, // [24..26] "d" payload start pointer
    2, 0, // [26..28] "d" valid sample count
    4, 0, // [28..30] "d" total sample count (nulls included)
    255, 255, // [30..32] group level 0: sentinel
    2, 0, // [32..34] group level 0: boundary at 2
    3, 0, // [34..36] group level 0: boundary at 3
    6, 0, // [36..38] group level 0: boundary at 6
    27, 0, // [38..40] group validity mask (u32 word 0, LSB first) = 1, 1, 0, 1, 1, 0
    255, 255, // [40..42] "b" topology level 0: sentinel
    2, 0, // [42..44] "b" topology level 0: boundary at 2
    2, 0, // [44..46] "b" topology level 0: boundary at 2
    3, 0, // [46..48] "b" topology level 0: boundary at 3
    6, 0, // [48..50] "b" topology level 0: boundary at 6
    255, 255, // [50..52] "b" topology level 1: sentinel
    1, 0, // [52..54] "b" topology level 1: boundary at 1
    3, 0, // [54..56] "b" topology level 1: boundary at 3
    4, 0, // [56..58] "b" topology level 1: boundary at 4
    6, 0, // [58..60] "b" topology level 1: boundary at 6
    6, 0, // [60..62] "b" topology level 1: boundary at 6
    7, 0, // [62..64] "b" topology level 1: boundary at 7
    255, 255, // [64..66] "c" topology level 0: sentinel
    3, 0, // [66..68] "c" topology level 0: boundary at 3
    3, 0, // [68..70] "c" topology level 0: boundary at 3
    5, 0, // [70..72] "c" topology level 0: boundary at 5
    5, 0, // [72..74] "c" topology level 0: boundary at 5
    0, 0, 0, 0, 0, 0, // [74..80] alignment padding (6 bytes)
    1, 0, 0, 0, 0, 0, 0, 0, // [80..88] "b"[0] = 1
    2, 0, 0, 0, 0, 0, 0, 0, // [88..96] "b"[1] = 2
    3, 0, 0, 0, 0, 0, 0, 0, // [96..104] "b"[2] = 3
    4, 0, 0, 0, 0, 0, 0, 0, // [104..112] "b"[3] = 4
    5, 0, 0, 0, 0, 0, 0, 0, // [112..120] "b"[4] = 5
    6, 0, 0, 0, 0, 0, 0, 0, // [120..128] "b"[5] = 6
    7, 0, 0, 0, 0, 0, 0, 0, // [128..136] "b"[6] = 7
    255, 255, 255, 255, // [136..140] "d" valid[0] = -1
    252, 255, 255, 255, // [140..144] "d" valid[1] = -4
    5,   // [144..145] "d" validity mask bits 0..4 (LSB first) = 1, 0, 1, 0
    0,   // [145..146] alignment padding (1 byte)
    1, 0, // [146..148] "c"[0] = 1
    2, 0, // [148..150] "c"[1] = 2
    3, 0, // [150..152] "c"[2] = 3
    5, 0, // [152..154] "c"[3] = 5
    6, 0, // [154..156] "c"[4] = 6
    1, // [156..157] "a"[0] = 1
    2, // [157..158] "a"[1] = 2
    4, // [158..159] "a"[2] = 4
    5, // [159..160] "a"[3] = 5
];
pub const GROUP_OPTION_2_NESTED_INODES: [Inode; 5] = [
    Inode {
        kind: WireKind::OptionGroup,
        kind_meta: KindMeta::Group {
            nesting: 2,
            count: 4,
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
        kind: WireKind::U64,
        kind_meta: KindMeta::Primitive {
            nesting: 2,
            data_offset: 12,
            topology_offset: 8,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::U16,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 20,
            topology_offset: 16,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::OptionI32,
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
fn test_group_option_2_nested() {
    let schema = group_option_2_schema(vec![
        field(
            "a",
            PyroType::PrimitiveScalar {
                primitive: PrimitiveDataType::U8,
            },
        ),
        field(
            "b",
            PyroType::List {
                element: Box::new(PyroType::List {
                    element: Box::new(PyroType::PrimitiveScalar {
                        primitive: PrimitiveDataType::U64,
                    }),
                }),
            },
        ),
        field(
            "c",
            PyroType::List {
                element: Box::new(PyroType::PrimitiveScalar {
                    primitive: PrimitiveDataType::U16,
                }),
            },
        ),
        field(
            "d",
            PyroType::OptionPrimitiveScalar {
                primitive: PrimitiveDataType::I32,
            },
        ),
    ]);
    run_list_wrapped_roundtrip_u16(
        schema,
        group_option_2_value(group_option_2_nested_entry),
        &GROUP_OPTION_2_NESTED,
        &GROUP_OPTION_2_NESTED_INODES,
    );
}
