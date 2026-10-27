//! `List<List<List<List<Group<..>>>>>` with two different groups:
//! * `primitive`: `Group{a: f32, b: i64, c: u16, d: Option<f64>}`
//! * `blob`: `Group{b: Vec<u8>, f: f64}`
//!
//! Shared value shape: `[[[[a, b], [c]], [[d]]], [[[e, f]]]]`, each letter one group `0..6`;
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
fn group_4_schema(fields: Vec<PyroField>) -> PyroSchema {
    let mut ty = PyroType::Group { fields };
    for _ in 0..4 {
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

/// `[[[[a, b], [c]], [[d]]], [[[e, f]]]]` over groups `0..6`, each built by `entry`.
#[cfg(test)]
fn group_4_value(entry: fn(usize) -> PyroValue<'static>) -> PyroValue<'static> {
    PyroValue::List(vec![
        PyroValue::List(vec![
            PyroValue::List(vec![
                PyroValue::List(vec![entry(0), entry(1)]),
                PyroValue::List(vec![entry(2)]),
            ]),
            PyroValue::List(vec![PyroValue::List(vec![entry(3)])]),
        ]),
        PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
            entry(4),
            entry(5),
        ])])]),
    ])
}

// ---------------------------------------------------------------------------
// primitive: Group{a: f32, b: i64, c: u16, d: Option<f64>}
// ---------------------------------------------------------------------------

/// Group `i` of `primitive`.
#[cfg(test)]
fn group_4_primitive_entry(i: usize) -> PyroValue<'static> {
    let a = [
        PyroValue::F32(1.5),
        PyroValue::F32(-2.0),
        PyroValue::F32(3.25),
        PyroValue::F32(4.0),
        PyroValue::F32(-5.5),
        PyroValue::F32(6.0),
    ];
    let b = [
        PyroValue::I64(-1),
        PyroValue::I64(2),
        PyroValue::I64(-3),
        PyroValue::I64(4),
        PyroValue::I64(-5),
        PyroValue::I64(6),
    ];
    let c = [
        PyroValue::U16(1),
        PyroValue::U16(2),
        PyroValue::U16(3),
        PyroValue::U16(4),
        PyroValue::U16(5),
        PyroValue::U16(6),
    ];
    let d = [
        PyroValue::F64(1.5),
        PyroValue::Null,
        PyroValue::F64(3.25),
        PyroValue::F64(4.0),
        PyroValue::Null,
        PyroValue::F64(6.0),
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

pub const GROUP_4_PRIMITIVE: [u8; 168] = [
    22, 0, // group topology start pointer
    12, 0, // group topology word count (3 levels, sentinels included)
    132, 0, // "a" payload start pointer
    6, 0, // "a" sample count
    48, 0, // "b" payload start pointer
    6, 0, // "b" sample count
    156, 0, // "c" payload start pointer
    6, 0, // "c" sample count
    96, 0, // "d" payload start pointer
    4, 0, // "d" valid sample count
    6, 0, // "d" total sample count (nulls included)
    255, 255, // group level 0: sentinel
    2, 0, // group level 0: boundary at 2 lists
    3, 0, // group level 0: boundary at 3 lists
    255, 255, // group level 1: sentinel
    2, 0, // group level 1: boundary at 2 lists
    3, 0, // group level 1: boundary at 3 lists
    4, 0, // group level 1: boundary at 4 lists
    255, 255, // group level 2: sentinel
    2, 0, // group level 2: boundary at 2 groups
    3, 0, // group level 2: boundary at 3 groups
    4, 0, // group level 2: boundary at 4 groups
    6, 0, // group level 2: boundary at 6 groups
    0, 0, // alignment padding to 8 bytes before "b"
    255, 255, 255, 255, 255, 255, 255, 255, // "b"[group 0] = -1
    2, 0, 0, 0, 0, 0, 0, 0, // "b"[group 1] = 2
    253, 255, 255, 255, 255, 255, 255, 255, // "b"[group 2] = -3
    4, 0, 0, 0, 0, 0, 0, 0, // "b"[group 3] = 4
    251, 255, 255, 255, 255, 255, 255, 255, // "b"[group 4] = -5
    6, 0, 0, 0, 0, 0, 0, 0, // "b"[group 5] = 6
    0, 0, 0, 0, 0, 0, 248, 63, // "d"[group 0] = Some(1.5)
    0, 0, 0, 0, 0, 0, 10, 64, // "d"[group 2] = Some(3.25)
    0, 0, 0, 0, 0, 0, 16, 64, // "d"[group 3] = Some(4.0)
    0, 0, 0, 0, 0, 0, 24, 64, // "d"[group 5] = Some(6.0)
    45, 0, 0, 0, // "d" validity mask bits (LSB first) = 1, 0, 1, 1, 0, 1
    0, 0, 192, 63, // "a"[group 0] = 1.5
    0, 0, 0, 192, // "a"[group 1] = -2.0
    0, 0, 80, 64, // "a"[group 2] = 3.25
    0, 0, 128, 64, // "a"[group 3] = 4.0
    0, 0, 176, 192, // "a"[group 4] = -5.5
    0, 0, 192, 64, // "a"[group 5] = 6.0
    1, 0, // "c"[group 0] = 1
    2, 0, // "c"[group 1] = 2
    3, 0, // "c"[group 2] = 3
    4, 0, // "c"[group 3] = 4
    5, 0, // "c"[group 4] = 5
    6, 0, // "c"[group 5] = 6
];
pub const GROUP_4_PRIMITIVE_INODES: [Inode; 5] = [
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 4,
            count: 4,
            topology_offset: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::F32,
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
        kind: WireKind::I64,
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
        kind: WireKind::U16,
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
        kind: WireKind::OptionF64,
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
fn test_group_4_primitive() {
    let schema = group_4_schema(vec![
        field(
            "a",
            PyroType::PrimitiveScalar {
                primitive: PrimitiveDataType::F32,
            },
        ),
        field(
            "b",
            PyroType::PrimitiveScalar {
                primitive: PrimitiveDataType::I64,
            },
        ),
        field(
            "c",
            PyroType::PrimitiveScalar {
                primitive: PrimitiveDataType::U16,
            },
        ),
        field(
            "d",
            PyroType::OptionPrimitiveScalar {
                primitive: PrimitiveDataType::F64,
            },
        ),
    ]);
    run_list_wrapped_roundtrip_u16(
        schema,
        group_4_value(group_4_primitive_entry),
        &GROUP_4_PRIMITIVE,
        &GROUP_4_PRIMITIVE_INODES,
    );
}

// ---------------------------------------------------------------------------
// blob: Group{b: Vec<u8>, f: f64}
// ---------------------------------------------------------------------------

/// Group `i` of `blob`.
#[cfg(test)]
fn group_4_blob_entry(i: usize) -> PyroValue<'static> {
    let b = [
        PyroValue::Bytes(vec![1, 2].into()),
        PyroValue::Bytes(vec![].into()),
        PyroValue::Bytes(vec![3, 4, 5].into()),
        PyroValue::Bytes(vec![255].into()),
        PyroValue::Bytes(vec![0, 6, 7, 8].into()),
        PyroValue::Bytes(vec![9].into()),
    ];
    let f = [
        PyroValue::F64(1.5),
        PyroValue::F64(-2.0),
        PyroValue::F64(3.25),
        PyroValue::F64(4.0),
        PyroValue::F64(-5.5),
        PyroValue::F64(6.0),
    ];
    PyroValue::Group([("b", b[i].clone()), ("f", f[i].clone())].into())
}

pub const GROUP_4_BLOB: [u8; 115] = [
    16, 0, // group topology start pointer
    12, 0, // group topology word count (3 levels, sentinels included)
    40, 0, // "b" byte-boundary start pointer
    7, 0, // "b" byte-boundary word count
    104, 0, // "b" payload start pointer
    11, 0, // "b" payload byte count
    56, 0, // "f" payload start pointer
    6, 0, // "f" sample count
    255, 255, // group level 0: sentinel
    2, 0, // group level 0: boundary at 2 lists
    3, 0, // group level 0: boundary at 3 lists
    255, 255, // group level 1: sentinel
    2, 0, // group level 1: boundary at 2 lists
    3, 0, // group level 1: boundary at 3 lists
    4, 0, // group level 1: boundary at 4 lists
    255, 255, // group level 2: sentinel
    2, 0, // group level 2: boundary at 2 groups
    3, 0, // group level 2: boundary at 3 groups
    4, 0, // group level 2: boundary at 4 groups
    6, 0, // group level 2: boundary at 6 groups
    255, 255, // "b" byte boundaries: sentinel
    2, 0, // "b" byte boundary at 2 ([1, 2] ends)
    2, 0, // "b" byte boundary at 2 ([] ends)
    5, 0, // "b" byte boundary at 5 ([3, 4, 5] ends)
    6, 0, // "b" byte boundary at 6 ([255] ends)
    10, 0, // "b" byte boundary at 10 ([0, 6, 7, 8] ends)
    11, 0, // "b" byte boundary at 11 ([9] ends)
    0, 0, // alignment padding to 8 bytes before "f"
    0, 0, 0, 0, 0, 0, 248, 63, // "f"[group 0] = 1.5
    0, 0, 0, 0, 0, 0, 0, 192, // "f"[group 1] = -2.0
    0, 0, 0, 0, 0, 0, 10, 64, // "f"[group 2] = 3.25
    0, 0, 0, 0, 0, 0, 16, 64, // "f"[group 3] = 4.0
    0, 0, 0, 0, 0, 0, 22, 192, // "f"[group 4] = -5.5
    0, 0, 0, 0, 0, 0, 24, 64, // "f"[group 5] = 6.0
    1, 2, // "b" payload [1, 2]
    3, 4, 5,   // "b" payload [3, 4, 5]
    255, // "b" payload [255]
    0, 6, 7, 8, // "b" payload [0, 6, 7, 8]
    9, // "b" payload [9]
];
pub const GROUP_4_BLOB_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 4,
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
            data_offset: 8,
            topology_offset: 4,
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
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_4_blob() {
    let schema = group_4_schema(vec![
        field("b", PyroType::Bytes),
        field(
            "f",
            PyroType::PrimitiveScalar {
                primitive: PrimitiveDataType::F64,
            },
        ),
    ]);
    run_list_wrapped_roundtrip_u16(
        schema,
        group_4_value(group_4_blob_entry),
        &GROUP_4_BLOB,
        &GROUP_4_BLOB_INODES,
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
fn group_4_mixed_entry(i: usize) -> PyroValue<'static> {
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

pub const GROUP_4_MIXED: [u8; 187] = [
    36, 0, // [0..2] group topology start pointer
    12, 0, // [2..4] group topology word count
    155, 0, // [4..6] "a" payload start pointer
    6, 0, // [6..8] "a" sample count
    128, 0, // [8..10] "b" payload start pointer
    4, 0, // [10..12] "b" valid sample count
    6, 0, // [12..14] "b" total sample count (nulls included)
    60, 0, // [14..16] "c" byte-boundary start pointer
    7, 0, // [16..18] "c" byte-boundary word count
    156, 0, // [18..20] "c" payload start pointer
    13, 0, // [20..22] "c" payload byte count
    80, 0, // [22..24] "d" payload start pointer
    6, 0, // [24..26] "d" sample count
    146, 0, // [26..28] "e" payload start pointer
    4, 0, // [28..30] "e" valid sample count
    6, 0, // [30..32] "e" total sample count (nulls included)
    169, 0, // [32..34] "f" payload start pointer
    6, 0, // [34..36] "f" array count
    255, 255, // [36..38] group level 0: sentinel
    2, 0, // [38..40] group level 0: boundary at 2
    3, 0, // [40..42] group level 0: boundary at 3
    255, 255, // [42..44] group level 1: sentinel
    2, 0, // [44..46] group level 1: boundary at 2
    3, 0, // [46..48] group level 1: boundary at 3
    4, 0, // [48..50] group level 1: boundary at 4
    255, 255, // [50..52] group level 2: sentinel
    2, 0, // [52..54] group level 2: boundary at 2
    3, 0, // [54..56] group level 2: boundary at 3
    4, 0, // [56..58] group level 2: boundary at 4
    6, 0, // [58..60] group level 2: boundary at 6
    255, 255, // [60..62] "c" byte-boundary level 0: sentinel
    2, 0, // [62..64] "c" byte-boundary level 0: boundary at 2
    2, 0, // [64..66] "c" byte-boundary level 0: boundary at 2
    5, 0, // [66..68] "c" byte-boundary level 0: boundary at 5
    7, 0, // [68..70] "c" byte-boundary level 0: boundary at 7
    12, 0, // [70..72] "c" byte-boundary level 0: boundary at 12
    13, 0, // [72..74] "c" byte-boundary level 0: boundary at 13
    0, 0, 0, 0, 0, 0, // [74..80] alignment padding (6 bytes)
    1, 0, 0, 0, 0, 0, 0, 0, // [80..88] "d"[0] = 1
    3, 0, 0, 0, 0, 1, 0, 0, // [88..96] "d"[1] = 1099511627779
    3, 0, 0, 0, 0, 0, 0, 0, // [96..104] "d"[2] = 3
    4, 0, 0, 0, 0, 0, 0, 0, // [104..112] "d"[3] = 4
    255, 255, 255, 255, 255, 255, 255, 255, // [112..120] "d"[4] = 18446744073709551615
    6, 0, 0, 0, 0, 0, 0, 0, // [120..128] "d"[5] = 6
    0, 0, 192, 63, // [128..132] "b" valid[0] = 1.5
    0, 0, 16, 192, // [132..136] "b" valid[1] = -2.25
    0, 0, 0, 65, // [136..140] "b" valid[2] = 8.0
    0, 0, 0, 63, // [140..144] "b" valid[3] = 0.5
    45, // [144..145] "b" validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
    0,  // [145..146] alignment padding (1 byte)
    254, 255, // [146..148] "e" valid[0] = -2
    3, 0, // [148..150] "e" valid[1] = 3
    251, 255, // [150..152] "e" valid[2] = -5
    6, 0,  // [152..154] "e" valid[3] = 6
    54, // [154..155] "e" validity mask bits 0..6 (LSB first) = 0, 1, 1, 0, 1, 1
    25, // [155..156] "a" payload bits 0..6 (LSB first) = 1, 0, 0, 1, 1, 0
    97, 98, // [156..158] "c" payload "ab"
    99, 100, 101, // [158..161] "c" payload "cde"
    195, 169, // [161..163] "c" payload "é"
    102, 103, 104, 105, 106, // [163..168] "c" payload "fghij"
    107, // [168..169] "c" payload "k"
    1, 2, 3, // [169..172] "f"[0] = [1, 2, 3]
    4, 5, 6, // [172..175] "f"[1] = [4, 5, 6]
    7, 8, 9, // [175..178] "f"[2] = [7, 8, 9]
    10, 11, 12, // [178..181] "f"[3] = [10, 11, 12]
    13, 14, 15, // [181..184] "f"[4] = [13, 14, 15]
    16, 17, 18, // [184..187] "f"[5] = [16, 17, 18]
];
pub const GROUP_4_MIXED_INODES: [Inode; 7] = [
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 4,
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
fn test_group_4_mixed() {
    let schema = group_4_schema(vec![
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
        group_4_value(group_4_mixed_entry),
        &GROUP_4_MIXED,
        &GROUP_4_MIXED_INODES,
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
fn group_4_nested_entry(i: usize) -> PyroValue<'static> {
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

pub const GROUP_4_NESTED: [u8; 206] = [
    30, 0, // [0..2] group topology start pointer
    12, 0, // [2..4] group topology word count
    200, 0, // [4..6] "a" payload start pointer
    6, 0, // [6..8] "a" sample count
    54, 0, // [8..10] "b" topology start pointer
    16, 0, // [10..12] "b" topology word count
    104, 0, // [12..14] "b" payload start pointer
    8, 0, // [14..16] "b" sample count
    86, 0, // [16..18] "c" topology start pointer
    7, 0, // [18..20] "c" topology word count
    186, 0, // [20..22] "c" payload start pointer
    7, 0, // [22..24] "c" sample count
    168, 0, // [24..26] "d" payload start pointer
    4, 0, // [26..28] "d" valid sample count
    6, 0, // [28..30] "d" total sample count (nulls included)
    255, 255, // [30..32] group level 0: sentinel
    2, 0, // [32..34] group level 0: boundary at 2
    3, 0, // [34..36] group level 0: boundary at 3
    255, 255, // [36..38] group level 1: sentinel
    2, 0, // [38..40] group level 1: boundary at 2
    3, 0, // [40..42] group level 1: boundary at 3
    4, 0, // [42..44] group level 1: boundary at 4
    255, 255, // [44..46] group level 2: sentinel
    2, 0, // [46..48] group level 2: boundary at 2
    3, 0, // [48..50] group level 2: boundary at 3
    4, 0, // [50..52] group level 2: boundary at 4
    6, 0, // [52..54] group level 2: boundary at 6
    255, 255, // [54..56] "b" topology level 0: sentinel
    2, 0, // [56..58] "b" topology level 0: boundary at 2
    2, 0, // [58..60] "b" topology level 0: boundary at 2
    3, 0, // [60..62] "b" topology level 0: boundary at 3
    4, 0, // [62..64] "b" topology level 0: boundary at 4
    7, 0, // [64..66] "b" topology level 0: boundary at 7
    8, 0, // [66..68] "b" topology level 0: boundary at 8
    255, 255, // [68..70] "b" topology level 1: sentinel
    1, 0, // [70..72] "b" topology level 1: boundary at 1
    3, 0, // [72..74] "b" topology level 1: boundary at 3
    3, 0, // [74..76] "b" topology level 1: boundary at 3
    4, 0, // [76..78] "b" topology level 1: boundary at 4
    6, 0, // [78..80] "b" topology level 1: boundary at 6
    6, 0, // [80..82] "b" topology level 1: boundary at 6
    7, 0, // [82..84] "b" topology level 1: boundary at 7
    8, 0, // [84..86] "b" topology level 1: boundary at 8
    255, 255, // [86..88] "c" topology level 0: sentinel
    3, 0, // [88..90] "c" topology level 0: boundary at 3
    3, 0, // [90..92] "c" topology level 0: boundary at 3
    4, 0, // [92..94] "c" topology level 0: boundary at 4
    6, 0, // [94..96] "c" topology level 0: boundary at 6
    6, 0, // [96..98] "c" topology level 0: boundary at 6
    7, 0, // [98..100] "c" topology level 0: boundary at 7
    0, 0, 0, 0, // [100..104] alignment padding (4 bytes)
    1, 0, 0, 0, 0, 0, 0, 0, // [104..112] "b"[0] = 1
    2, 0, 0, 0, 0, 0, 0, 0, // [112..120] "b"[1] = 2
    3, 0, 0, 0, 0, 0, 0, 0, // [120..128] "b"[2] = 3
    4, 0, 0, 0, 0, 0, 0, 0, // [128..136] "b"[3] = 4
    5, 0, 0, 0, 0, 0, 0, 0, // [136..144] "b"[4] = 5
    6, 0, 0, 0, 0, 0, 0, 0, // [144..152] "b"[5] = 6
    7, 0, 0, 0, 0, 0, 0, 0, // [152..160] "b"[6] = 7
    8, 0, 0, 0, 0, 0, 0, 0, // [160..168] "b"[7] = 8
    255, 255, 255, 255, // [168..172] "d" valid[0] = -1
    3, 0, 0, 0, // [172..176] "d" valid[1] = 3
    252, 255, 255, 255, // [176..180] "d" valid[2] = -4
    6, 0, 0, 0,  // [180..184] "d" valid[3] = 6
    45, // [184..185] "d" validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
    0,  // [185..186] alignment padding (1 byte)
    1, 0, // [186..188] "c"[0] = 1
    2, 0, // [188..190] "c"[1] = 2
    3, 0, // [190..192] "c"[2] = 3
    4, 0, // [192..194] "c"[3] = 4
    5, 0, // [194..196] "c"[4] = 5
    6, 0, // [196..198] "c"[5] = 6
    7, 0, // [198..200] "c"[6] = 7
    1, // [200..201] "a"[0] = 1
    2, // [201..202] "a"[1] = 2
    3, // [202..203] "a"[2] = 3
    4, // [203..204] "a"[3] = 4
    5, // [204..205] "a"[4] = 5
    6, // [205..206] "a"[5] = 6
];
pub const GROUP_4_NESTED_INODES: [Inode; 5] = [
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 4,
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
fn test_group_4_nested() {
    let schema = group_4_schema(vec![
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
        group_4_value(group_4_nested_entry),
        &GROUP_4_NESTED,
        &GROUP_4_NESTED_INODES,
    );
}
