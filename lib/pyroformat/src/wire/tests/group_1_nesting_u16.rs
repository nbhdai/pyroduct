//! `List<Group<..>>` with two different groups:
//! * `primitive`: `Group{a: u8, b: u32, c: Option<u16>}`
//! * `blob`: `Group{s: String, n: u16}`
//!
//! Shared value shape: `[a, b, c, d, e, f]`, each letter one group `0..6`;
//! scalar fields take the primitive files' sample lists, `Option<T>` fields
//! are `None` in groups 1 and 4, and blob fields take the `basic_*` blobs.
//!
//! Layout: the group slot, then each field's slot in declaration order
//! (`T`: `(start, count)`; `Option<T>`: `(start, valid, total)`;
//! `Str`/`Bytes`: a byte-boundary pointer, then `(start, byte count)`).
//! The dynamic region holds the group topology, each blob's byte
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
fn group_1_schema(fields: Vec<PyroField>) -> PyroSchema {
    let mut ty = PyroType::Group { fields };
    for _ in 0..1 {
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

/// `[a, b, c, d, e, f]` over groups `0..6`, each built by `entry`.
#[cfg(test)]
fn group_1_value(entry: fn(usize) -> PyroValue<'static>) -> PyroValue<'static> {
    PyroValue::List(vec![
        entry(0),
        entry(1),
        entry(2),
        entry(3),
        entry(4),
        entry(5),
    ])
}

// ---------------------------------------------------------------------------
// primitive: Group{a: u8, b: u32, c: Option<u16>}
// ---------------------------------------------------------------------------

/// Group `i` of `primitive`.
#[cfg(test)]
fn group_1_primitive_entry(i: usize) -> PyroValue<'static> {
    let a = [
        PyroValue::U8(1),
        PyroValue::U8(2),
        PyroValue::U8(3),
        PyroValue::U8(4),
        PyroValue::U8(5),
        PyroValue::U8(6),
    ];
    let b = [
        PyroValue::U32(1),
        PyroValue::U32(2),
        PyroValue::U32(3),
        PyroValue::U32(4),
        PyroValue::U32(5),
        PyroValue::U32(6),
    ];
    let c = [
        PyroValue::U16(1),
        PyroValue::Null,
        PyroValue::U16(3),
        PyroValue::U16(4),
        PyroValue::Null,
        PyroValue::U16(6),
    ];
    PyroValue::Group(
        [
            ("a", a[i].clone()),
            ("b", b[i].clone()),
            ("c", c[i].clone()),
        ]
        .into(),
    )
}

pub const GROUP_1_PRIMITIVE: [u8; 55] = [
    6, 0, // group count
    49, 0, // "a" payload start pointer
    6, 0, // "a" sample count
    16, 0, // "b" payload start pointer
    6, 0, // "b" sample count
    40, 0, // "c" payload start pointer
    4, 0, // "c" valid sample count
    6, 0, // "c" total sample count (nulls included)
    1, 0, 0, 0, // "b"[group 0] = 1
    2, 0, 0, 0, // "b"[group 1] = 2
    3, 0, 0, 0, // "b"[group 2] = 3
    4, 0, 0, 0, // "b"[group 3] = 4
    5, 0, 0, 0, // "b"[group 4] = 5
    6, 0, 0, 0, // "b"[group 5] = 6
    1, 0, // "c"[group 0] = Some(1)
    3, 0, // "c"[group 2] = Some(3)
    4, 0, // "c"[group 3] = Some(4)
    6, 0,  // "c"[group 5] = Some(6)
    45, // "c" validity mask bits (LSB first) = 1, 0, 1, 1, 0, 1  // "c" validity mask padding to 2 bytes
    1,  // "a"[group 0] = 1
    2,  // "a"[group 1] = 2
    3,  // "a"[group 2] = 3
    4,  // "a"[group 3] = 4
    5,  // "a"[group 4] = 5
    6,  // "a"[group 5] = 6
];
pub const GROUP_1_PRIMITIVE_INODES: [Inode; 4] = [
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 1,
            count: 3,
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
fn test_group_1_primitive() {
    let schema = group_1_schema(vec![
        field(
            "a",
            PyroType::PrimitiveScalar {
                primitive: PrimitiveDataType::U8,
            },
        ),
        field(
            "b",
            PyroType::PrimitiveScalar {
                primitive: PrimitiveDataType::U32,
            },
        ),
        field(
            "c",
            PyroType::OptionPrimitiveScalar {
                primitive: PrimitiveDataType::U16,
            },
        ),
    ]);
    run_list_wrapped_roundtrip_u16(
        schema,
        group_1_value(group_1_primitive_entry),
        &GROUP_1_PRIMITIVE,
        &GROUP_1_PRIMITIVE_INODES,
    );
}

// ---------------------------------------------------------------------------
// blob: Group{s: String, n: u16}
// ---------------------------------------------------------------------------

/// Group `i` of `blob`.
#[cfg(test)]
fn group_1_blob_entry(i: usize) -> PyroValue<'static> {
    let s = [
        PyroValue::Str("ab".into()),
        PyroValue::Str("".into()),
        PyroValue::Str("cde".into()),
        PyroValue::Str("f".into()),
        PyroValue::Str("ghij".into()),
        PyroValue::Str("é".into()),
    ];
    let n = [
        PyroValue::U16(1),
        PyroValue::U16(2),
        PyroValue::U16(3),
        PyroValue::U16(4),
        PyroValue::U16(5),
        PyroValue::U16(6),
    ];
    PyroValue::Group([("s", s[i].clone()), ("n", n[i].clone())].into())
}

pub const GROUP_1_BLOB: [u8; 52] = [
    6, 0, // group count
    14, 0, // "s" byte-boundary start pointer
    7, 0, // "s" byte-boundary word count
    40, 0, // "s" payload start pointer
    12, 0, // "s" payload byte count
    28, 0, // "n" payload start pointer
    6, 0, // "n" sample count
    255, 255, // "s" byte boundaries: sentinel
    2, 0, // "s" byte boundary at 2 ("ab" ends)
    2, 0, // "s" byte boundary at 2 ("" ends)
    5, 0, // "s" byte boundary at 5 ("cde" ends)
    6, 0, // "s" byte boundary at 6 ("f" ends)
    10, 0, // "s" byte boundary at 10 ("ghij" ends)
    12, 0, // "s" byte boundary at 12 ("é" ends)
    1, 0, // "n"[group 0] = 1
    2, 0, // "n"[group 1] = 2
    3, 0, // "n"[group 2] = 3
    4, 0, // "n"[group 3] = 4
    5, 0, // "n"[group 4] = 5
    6, 0, // "n"[group 5] = 6
    97, 98, // "s" payload "ab"
    99, 100, 101, // "s" payload "cde"
    102, // "s" payload "f"
    103, 104, 105, 106, // "s" payload "ghij"
    195, 169, // "s" payload "é"
];
pub const GROUP_1_BLOB_INODES: [Inode; 3] = [
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
        kind: WireKind::U16,
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
fn test_group_1_blob() {
    let schema = group_1_schema(vec![
        field("s", PyroType::Str),
        field(
            "n",
            PyroType::PrimitiveScalar {
                primitive: PrimitiveDataType::U16,
            },
        ),
    ]);
    run_list_wrapped_roundtrip_u16(
        schema,
        group_1_value(group_1_blob_entry),
        &GROUP_1_BLOB,
        &GROUP_1_BLOB_INODES,
    );
}

#[cfg(test)]
use crate::value::PrimitiveValueList;

// ---------------------------------------------------------------------------
// mixed: Group{a: bool, b: Option<f32>, c: Str, d: u64, e: Option<i16>, f:
// [u8; 3]}
// ---------------------------------------------------------------------------
//
// every slot shape a list-nested group field can take (bit-packed, `Option`,
// byte-boundary blob, 8-byte scalar, fixed array) side by side. Byte ranges
// are `[start..end)` offsets into the buffer; captured from the value writer's
// output.

/// Group `i` of `mixed`.
#[cfg(test)]
fn group_1_mixed_entry(i: usize) -> PyroValue<'static> {
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

pub const GROUP_1_MIXED: [u8; 155] = [
    6, 0, // [0..2] group count
    123, 0, // [2..4] "a" payload start pointer
    6, 0, // [4..6] "a" sample count
    96, 0, // [6..8] "b" payload start pointer
    4, 0, // [8..10] "b" valid sample count
    6, 0, // [10..12] "b" total sample count (nulls included)
    34, 0, // [12..14] "c" byte-boundary start pointer
    7, 0, // [14..16] "c" byte-boundary word count
    124, 0, // [16..18] "c" payload start pointer
    13, 0, // [18..20] "c" payload byte count
    48, 0, // [20..22] "d" payload start pointer
    6, 0, // [22..24] "d" sample count
    114, 0, // [24..26] "e" payload start pointer
    4, 0, // [26..28] "e" valid sample count
    6, 0, // [28..30] "e" total sample count (nulls included)
    137, 0, // [30..32] "f" payload start pointer
    6, 0, // [32..34] "f" array count
    255, 255, // [34..36] "c" byte-boundary level 0: sentinel
    2, 0, // [36..38] "c" byte-boundary level 0: boundary at 2
    2, 0, // [38..40] "c" byte-boundary level 0: boundary at 2
    5, 0, // [40..42] "c" byte-boundary level 0: boundary at 5
    7, 0, // [42..44] "c" byte-boundary level 0: boundary at 7
    12, 0, // [44..46] "c" byte-boundary level 0: boundary at 12
    13, 0, // [46..48] "c" byte-boundary level 0: boundary at 13
    1, 0, 0, 0, 0, 0, 0, 0, // [48..56] "d"[0] = 1
    3, 0, 0, 0, 0, 1, 0, 0, // [56..64] "d"[1] = 1099511627779
    3, 0, 0, 0, 0, 0, 0, 0, // [64..72] "d"[2] = 3
    4, 0, 0, 0, 0, 0, 0, 0, // [72..80] "d"[3] = 4
    255, 255, 255, 255, 255, 255, 255, 255, // [80..88] "d"[4] = 18446744073709551615
    6, 0, 0, 0, 0, 0, 0, 0, // [88..96] "d"[5] = 6
    0, 0, 192, 63, // [96..100] "b" valid[0] = 1.5
    0, 0, 16, 192, // [100..104] "b" valid[1] = -2.25
    0, 0, 0, 65, // [104..108] "b" valid[2] = 8.0
    0, 0, 0, 63, // [108..112] "b" valid[3] = 0.5
    45, // [112..113] "b" validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
    0,  // [113..114] alignment padding (1 byte)
    254, 255, // [114..116] "e" valid[0] = -2
    3, 0, // [116..118] "e" valid[1] = 3
    251, 255, // [118..120] "e" valid[2] = -5
    6, 0,  // [120..122] "e" valid[3] = 6
    54, // [122..123] "e" validity mask bits 0..6 (LSB first) = 0, 1, 1, 0, 1, 1
    25, // [123..124] "a" payload bits 0..6 (LSB first) = 1, 0, 0, 1, 1, 0
    97, 98, // [124..126] "c" payload "ab"
    99, 100, 101, // [126..129] "c" payload "cde"
    195, 169, // [129..131] "c" payload "é"
    102, 103, 104, 105, 106, // [131..136] "c" payload "fghij"
    107, // [136..137] "c" payload "k"
    1, 2, 3, // [137..140] "f"[0] = [1, 2, 3]
    4, 5, 6, // [140..143] "f"[1] = [4, 5, 6]
    7, 8, 9, // [143..146] "f"[2] = [7, 8, 9]
    10, 11, 12, // [146..149] "f"[3] = [10, 11, 12]
    13, 14, 15, // [149..152] "f"[4] = [13, 14, 15]
    16, 17, 18, // [152..155] "f"[5] = [16, 17, 18]
];
pub const GROUP_1_MIXED_INODES: [Inode; 7] = [
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 1,
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
            data_offset: 2,
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
            data_offset: 16,
            topology_offset: 12,
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
            data_offset: 20,
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
            data_offset: 24,
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
            data_offset: 30,
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
fn test_group_1_mixed() {
    let schema = group_1_schema(vec![
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
        group_1_value(group_1_mixed_entry),
        &GROUP_1_MIXED,
        &GROUP_1_MIXED_INODES,
    );
}

// ---------------------------------------------------------------------------
// nested: Group{a: u8, b: List<List<u64>>, c: List<u16>, d: Option<i32>}
// ---------------------------------------------------------------------------
//
// a double-nested field and a single-nested field (with empty inner lists)
// next to plain and `Option` scalars, so each field's own topology sits
// between the group topology and payloads of four different alignments. Byte
// ranges are `[start..end)` offsets into the buffer; captured from the value
// writer's output.

/// Group `i` of `nested`.
#[cfg(test)]
fn group_1_nested_entry(i: usize) -> PyroValue<'static> {
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

pub const GROUP_1_NESTED: [u8; 182] = [
    6, 0, // [0..2] group count
    176, 0, // [2..4] "a" payload start pointer
    6, 0, // [4..6] "a" sample count
    28, 0, // [6..8] "b" topology start pointer
    16, 0, // [8..10] "b" topology word count
    80, 0, // [10..12] "b" payload start pointer
    8, 0, // [12..14] "b" sample count
    60, 0, // [14..16] "c" topology start pointer
    7, 0, // [16..18] "c" topology word count
    162, 0, // [18..20] "c" payload start pointer
    7, 0, // [20..22] "c" sample count
    144, 0, // [22..24] "d" payload start pointer
    4, 0, // [24..26] "d" valid sample count
    6, 0, // [26..28] "d" total sample count (nulls included)
    255, 255, // [28..30] "b" topology level 0: sentinel
    2, 0, // [30..32] "b" topology level 0: boundary at 2
    2, 0, // [32..34] "b" topology level 0: boundary at 2
    3, 0, // [34..36] "b" topology level 0: boundary at 3
    4, 0, // [36..38] "b" topology level 0: boundary at 4
    7, 0, // [38..40] "b" topology level 0: boundary at 7
    8, 0, // [40..42] "b" topology level 0: boundary at 8
    255, 255, // [42..44] "b" topology level 1: sentinel
    1, 0, // [44..46] "b" topology level 1: boundary at 1
    3, 0, // [46..48] "b" topology level 1: boundary at 3
    3, 0, // [48..50] "b" topology level 1: boundary at 3
    4, 0, // [50..52] "b" topology level 1: boundary at 4
    6, 0, // [52..54] "b" topology level 1: boundary at 6
    6, 0, // [54..56] "b" topology level 1: boundary at 6
    7, 0, // [56..58] "b" topology level 1: boundary at 7
    8, 0, // [58..60] "b" topology level 1: boundary at 8
    255, 255, // [60..62] "c" topology level 0: sentinel
    3, 0, // [62..64] "c" topology level 0: boundary at 3
    3, 0, // [64..66] "c" topology level 0: boundary at 3
    4, 0, // [66..68] "c" topology level 0: boundary at 4
    6, 0, // [68..70] "c" topology level 0: boundary at 6
    6, 0, // [70..72] "c" topology level 0: boundary at 6
    7, 0, // [72..74] "c" topology level 0: boundary at 7
    0, 0, 0, 0, 0, 0, // [74..80] alignment padding (6 bytes)
    1, 0, 0, 0, 0, 0, 0, 0, // [80..88] "b"[0] = 1
    2, 0, 0, 0, 0, 0, 0, 0, // [88..96] "b"[1] = 2
    3, 0, 0, 0, 0, 0, 0, 0, // [96..104] "b"[2] = 3
    4, 0, 0, 0, 0, 0, 0, 0, // [104..112] "b"[3] = 4
    5, 0, 0, 0, 0, 0, 0, 0, // [112..120] "b"[4] = 5
    6, 0, 0, 0, 0, 0, 0, 0, // [120..128] "b"[5] = 6
    7, 0, 0, 0, 0, 0, 0, 0, // [128..136] "b"[6] = 7
    8, 0, 0, 0, 0, 0, 0, 0, // [136..144] "b"[7] = 8
    255, 255, 255, 255, // [144..148] "d" valid[0] = -1
    3, 0, 0, 0, // [148..152] "d" valid[1] = 3
    252, 255, 255, 255, // [152..156] "d" valid[2] = -4
    6, 0, 0, 0,  // [156..160] "d" valid[3] = 6
    45, // [160..161] "d" validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
    0,  // [161..162] alignment padding (1 byte)
    1, 0, // [162..164] "c"[0] = 1
    2, 0, // [164..166] "c"[1] = 2
    3, 0, // [166..168] "c"[2] = 3
    4, 0, // [168..170] "c"[3] = 4
    5, 0, // [170..172] "c"[4] = 5
    6, 0, // [172..174] "c"[5] = 6
    7, 0, // [174..176] "c"[6] = 7
    1, // [176..177] "a"[0] = 1
    2, // [177..178] "a"[1] = 2
    3, // [178..179] "a"[2] = 3
    4, // [179..180] "a"[3] = 4
    5, // [180..181] "a"[4] = 5
    6, // [181..182] "a"[5] = 6
];
pub const GROUP_1_NESTED_INODES: [Inode; 5] = [
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 1,
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
            data_offset: 2,
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
            data_offset: 10,
            topology_offset: 6,
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
            data_offset: 18,
            topology_offset: 14,
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
            data_offset: 22,
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
fn test_group_1_nested() {
    let schema = group_1_schema(vec![
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
        group_1_value(group_1_nested_entry),
        &GROUP_1_NESTED,
        &GROUP_1_NESTED_INODES,
    );
}
