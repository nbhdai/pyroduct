//! `List<List<List<Group<..>>>>` with two different groups:
//! * `primitive`: `Group{a: Time, b: u8, c: Option<i32>}`
//! * `blob`: `Group{id: u32, s: String}`
//!
//! Shared value shape: `[[[a, b], [c]], [[d, e, f]]]`, each letter one group `0..6`;
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
use crate::{PyroValue, value::Time, wire::tests::support::run_list_wrapped_roundtrip_u16};

use pyro_schema::{Inode, KindMeta, WireKind};

/// The field list wrapped in `List` `nesting` times, as field `a`.
#[cfg(test)]
fn group_3_schema(fields: Vec<PyroField>) -> PyroSchema {
    let mut ty = PyroType::Group { fields };
    for _ in 0..3 {
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

/// `[[[a, b], [c]], [[d, e, f]]]` over groups `0..6`, each built by `entry`.
#[cfg(test)]
fn group_3_value(entry: fn(usize) -> PyroValue<'static>) -> PyroValue<'static> {
    PyroValue::List(vec![
        PyroValue::List(vec![
            PyroValue::List(vec![entry(0), entry(1)]),
            PyroValue::List(vec![entry(2)]),
        ]),
        PyroValue::List(vec![PyroValue::List(vec![entry(3), entry(4), entry(5)])]),
    ])
}

// ---------------------------------------------------------------------------
// primitive: Group{a: Time, b: u8, c: Option<i32>}
// ---------------------------------------------------------------------------

/// Group `i` of `primitive`.
#[cfg(test)]
fn group_3_primitive_entry(i: usize) -> PyroValue<'static> {
    let a = [
        PyroValue::from(Time(1_000_001_000)),
        PyroValue::from(Time(-2_000_002_000)),
        PyroValue::from(Time(3_500_003_000)),
        PyroValue::from(Time(4_000_004_000)),
        PyroValue::from(Time(-5_000_005_000)),
        PyroValue::from(Time(1_700_000_000_000_006_000)),
    ];
    let b = [
        PyroValue::U8(1),
        PyroValue::U8(2),
        PyroValue::U8(3),
        PyroValue::U8(4),
        PyroValue::U8(5),
        PyroValue::U8(6),
    ];
    let c = [
        PyroValue::I32(-1),
        PyroValue::Null,
        PyroValue::I32(-3),
        PyroValue::I32(4),
        PyroValue::Null,
        PyroValue::I32(6),
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

pub const GROUP_3_PRIMITIVE: [u8; 151] = [
    18, 0, // group topology start pointer
    7, 0, // group topology word count (2 levels, sentinels included)
    32, 0, // "a" payload start pointer
    6, 0, // "a" sample count
    145, 0, // "b" payload start pointer
    6, 0, // "b" sample count
    128, 0, // "c" payload start pointer
    4, 0, // "c" valid sample count
    6, 0, // "c" total sample count (nulls included)
    255, 255, // group level 0: sentinel
    2, 0, // group level 0: boundary at 2 lists
    3, 0, // group level 0: boundary at 3 lists
    255, 255, // group level 1: sentinel
    2, 0, // group level 1: boundary at 2 groups
    3, 0, // group level 1: boundary at 3 groups
    6, 0, // group level 1: boundary at 6 groups
    232, 205, 154, 59, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // "a"[group 0] = Time(1000001000)
    48, 100, 202, 136, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, // "a"[group 1] = Time(-2000002000)
    184, 206, 157, 208, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // "a"[group 2] = Time(3500003000)
    160, 55, 107, 238, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // "a"[group 3] = Time(4000004000)
    120, 250, 249, 213, 254, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, // "a"[group 4] = Time(-5000005000)
    112, 23, 42, 54, 254, 156, 151, 23, 0, 0, 0, 0, 0, 0, 0,
    0, // "a"[group 5] = Time(1700000000000006000)
    255, 255, 255, 255, // "c"[group 0] = Some(-1)
    253, 255, 255, 255, // "c"[group 2] = Some(-3)
    4, 0, 0, 0, // "c"[group 3] = Some(4)
    6, 0, 0, 0,  // "c"[group 5] = Some(6)
    45, // "c" validity mask bits (LSB first) = 1, 0, 1, 1, 0, 1
    1,  // "b"[group 0] = 1
    2,  // "b"[group 1] = 2
    3,  // "b"[group 2] = 3
    4,  // "b"[group 3] = 4
    5,  // "b"[group 4] = 5
    6,  // "b"[group 5] = 6
];
pub const GROUP_3_PRIMITIVE_INODES: [Inode; 4] = [
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 3,
            count: 3,
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
    Inode {
        kind: WireKind::OptionI32,
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
fn test_group_3_primitive() {
    let schema = group_3_schema(vec![
        field(
            "a",
            PyroType::PrimitiveScalar {
                primitive: PrimitiveDataType::Timestamp,
            },
        ),
        field(
            "b",
            PyroType::PrimitiveScalar {
                primitive: PrimitiveDataType::U8,
            },
        ),
        field(
            "c",
            PyroType::OptionPrimitiveScalar {
                primitive: PrimitiveDataType::I32,
            },
        ),
    ]);
    run_list_wrapped_roundtrip_u16(
        schema,
        group_3_value(group_3_primitive_entry),
        &GROUP_3_PRIMITIVE,
        &GROUP_3_PRIMITIVE_INODES,
    );
}

// ---------------------------------------------------------------------------
// blob: Group{id: u32, s: String}
// ---------------------------------------------------------------------------

/// Group `i` of `blob`.
#[cfg(test)]
fn group_3_blob_entry(i: usize) -> PyroValue<'static> {
    let id = [
        PyroValue::U32(1),
        PyroValue::U32(2),
        PyroValue::U32(3),
        PyroValue::U32(4),
        PyroValue::U32(5),
        PyroValue::U32(6),
    ];
    let s = [
        PyroValue::Str("ab".into()),
        PyroValue::Str("".into()),
        PyroValue::Str("cde".into()),
        PyroValue::Str("f".into()),
        PyroValue::Str("ghij".into()),
        PyroValue::Str("é".into()),
    ];
    PyroValue::Group([("id", id[i].clone()), ("s", s[i].clone())].into())
}

pub const GROUP_3_BLOB: [u8; 80] = [
    16, 0, // group topology start pointer
    7, 0, // group topology word count (2 levels, sentinels included)
    44, 0, // "id" payload start pointer
    6, 0, // "id" sample count
    30, 0, // "s" byte-boundary start pointer
    7, 0, // "s" byte-boundary word count
    68, 0, // "s" payload start pointer
    12, 0, // "s" payload byte count
    255, 255, // group level 0: sentinel
    2, 0, // group level 0: boundary at 2 lists
    3, 0, // group level 0: boundary at 3 lists
    255, 255, // group level 1: sentinel
    2, 0, // group level 1: boundary at 2 groups
    3, 0, // group level 1: boundary at 3 groups
    6, 0, // group level 1: boundary at 6 groups
    255, 255, // "s" byte boundaries: sentinel
    2, 0, // "s" byte boundary at 2 ("ab" ends)
    2, 0, // "s" byte boundary at 2 ("" ends)
    5, 0, // "s" byte boundary at 5 ("cde" ends)
    6, 0, // "s" byte boundary at 6 ("f" ends)
    10, 0, // "s" byte boundary at 10 ("ghij" ends)
    12, 0, // "s" byte boundary at 12 ("é" ends)
    1, 0, 0, 0, // "id"[group 0] = 1
    2, 0, 0, 0, // "id"[group 1] = 2
    3, 0, 0, 0, // "id"[group 2] = 3
    4, 0, 0, 0, // "id"[group 3] = 4
    5, 0, 0, 0, // "id"[group 4] = 5
    6, 0, 0, 0, // "id"[group 5] = 6
    97, 98, // "s" payload "ab"
    99, 100, 101, // "s" payload "cde"
    102, // "s" payload "f"
    103, 104, 105, 106, // "s" payload "ghij"
    195, 169, // "s" payload "é"
];
pub const GROUP_3_BLOB_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::Group,
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
        kind: WireKind::U32,
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
        kind: WireKind::Str,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 12,
            topology_offset: 8,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_3_blob() {
    let schema = group_3_schema(vec![
        field(
            "id",
            PyroType::PrimitiveScalar {
                primitive: PrimitiveDataType::U32,
            },
        ),
        field("s", PyroType::Str),
    ]);
    run_list_wrapped_roundtrip_u16(
        schema,
        group_3_value(group_3_blob_entry),
        &GROUP_3_BLOB,
        &GROUP_3_BLOB_INODES,
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
fn group_3_mixed_entry(i: usize) -> PyroValue<'static> {
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

pub const GROUP_3_MIXED: [u8; 171] = [
    36, 0, // [0..2] group topology start pointer
    7, 0, // [2..4] group topology word count
    139, 0, // [4..6] "a" payload start pointer
    6, 0, // [6..8] "a" sample count
    112, 0, // [8..10] "b" payload start pointer
    4, 0, // [10..12] "b" valid sample count
    6, 0, // [12..14] "b" total sample count (nulls included)
    50, 0, // [14..16] "c" byte-boundary start pointer
    7, 0, // [16..18] "c" byte-boundary word count
    140, 0, // [18..20] "c" payload start pointer
    13, 0, // [20..22] "c" payload byte count
    64, 0, // [22..24] "d" payload start pointer
    6, 0, // [24..26] "d" sample count
    130, 0, // [26..28] "e" payload start pointer
    4, 0, // [28..30] "e" valid sample count
    6, 0, // [30..32] "e" total sample count (nulls included)
    153, 0, // [32..34] "f" payload start pointer
    6, 0, // [34..36] "f" array count
    255, 255, // [36..38] group level 0: sentinel
    2, 0, // [38..40] group level 0: boundary at 2
    3, 0, // [40..42] group level 0: boundary at 3
    255, 255, // [42..44] group level 1: sentinel
    2, 0, // [44..46] group level 1: boundary at 2
    3, 0, // [46..48] group level 1: boundary at 3
    6, 0, // [48..50] group level 1: boundary at 6
    255, 255, // [50..52] "c" byte-boundary level 0: sentinel
    2, 0, // [52..54] "c" byte-boundary level 0: boundary at 2
    2, 0, // [54..56] "c" byte-boundary level 0: boundary at 2
    5, 0, // [56..58] "c" byte-boundary level 0: boundary at 5
    7, 0, // [58..60] "c" byte-boundary level 0: boundary at 7
    12, 0, // [60..62] "c" byte-boundary level 0: boundary at 12
    13, 0, // [62..64] "c" byte-boundary level 0: boundary at 13
    1, 0, 0, 0, 0, 0, 0, 0, // [64..72] "d"[0] = 1
    3, 0, 0, 0, 0, 1, 0, 0, // [72..80] "d"[1] = 1099511627779
    3, 0, 0, 0, 0, 0, 0, 0, // [80..88] "d"[2] = 3
    4, 0, 0, 0, 0, 0, 0, 0, // [88..96] "d"[3] = 4
    255, 255, 255, 255, 255, 255, 255, 255, // [96..104] "d"[4] = 18446744073709551615
    6, 0, 0, 0, 0, 0, 0, 0, // [104..112] "d"[5] = 6
    0, 0, 192, 63, // [112..116] "b" valid[0] = 1.5
    0, 0, 16, 192, // [116..120] "b" valid[1] = -2.25
    0, 0, 0, 65, // [120..124] "b" valid[2] = 8.0
    0, 0, 0, 63, // [124..128] "b" valid[3] = 0.5
    45, // [128..129] "b" validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
    0,  // [129..130] alignment padding (1 byte)
    254, 255, // [130..132] "e" valid[0] = -2
    3, 0, // [132..134] "e" valid[1] = 3
    251, 255, // [134..136] "e" valid[2] = -5
    6, 0,  // [136..138] "e" valid[3] = 6
    54, // [138..139] "e" validity mask bits 0..6 (LSB first) = 0, 1, 1, 0, 1, 1
    25, // [139..140] "a" payload bits 0..6 (LSB first) = 1, 0, 0, 1, 1, 0
    97, 98, // [140..142] "c" payload "ab"
    99, 100, 101, // [142..145] "c" payload "cde"
    195, 169, // [145..147] "c" payload "é"
    102, 103, 104, 105, 106, // [147..152] "c" payload "fghij"
    107, // [152..153] "c" payload "k"
    1, 2, 3, // [153..156] "f"[0] = [1, 2, 3]
    4, 5, 6, // [156..159] "f"[1] = [4, 5, 6]
    7, 8, 9, // [159..162] "f"[2] = [7, 8, 9]
    10, 11, 12, // [162..165] "f"[3] = [10, 11, 12]
    13, 14, 15, // [165..168] "f"[4] = [13, 14, 15]
    16, 17, 18, // [168..171] "f"[5] = [16, 17, 18]
];
pub const GROUP_3_MIXED_INODES: [Inode; 7] = [
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 3,
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
fn test_group_3_mixed() {
    let schema = group_3_schema(vec![
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
        group_3_value(group_3_mixed_entry),
        &GROUP_3_MIXED,
        &GROUP_3_MIXED_INODES,
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
fn group_3_nested_entry(i: usize) -> PyroValue<'static> {
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

pub const GROUP_3_NESTED: [u8; 198] = [
    30, 0, // [0..2] group topology start pointer
    7, 0, // [2..4] group topology word count
    192, 0, // [4..6] "a" payload start pointer
    6, 0, // [6..8] "a" sample count
    44, 0, // [8..10] "b" topology start pointer
    16, 0, // [10..12] "b" topology word count
    96, 0, // [12..14] "b" payload start pointer
    8, 0, // [14..16] "b" sample count
    76, 0, // [16..18] "c" topology start pointer
    7, 0, // [18..20] "c" topology word count
    178, 0, // [20..22] "c" payload start pointer
    7, 0, // [22..24] "c" sample count
    160, 0, // [24..26] "d" payload start pointer
    4, 0, // [26..28] "d" valid sample count
    6, 0, // [28..30] "d" total sample count (nulls included)
    255, 255, // [30..32] group level 0: sentinel
    2, 0, // [32..34] group level 0: boundary at 2
    3, 0, // [34..36] group level 0: boundary at 3
    255, 255, // [36..38] group level 1: sentinel
    2, 0, // [38..40] group level 1: boundary at 2
    3, 0, // [40..42] group level 1: boundary at 3
    6, 0, // [42..44] group level 1: boundary at 6
    255, 255, // [44..46] "b" topology level 0: sentinel
    2, 0, // [46..48] "b" topology level 0: boundary at 2
    2, 0, // [48..50] "b" topology level 0: boundary at 2
    3, 0, // [50..52] "b" topology level 0: boundary at 3
    4, 0, // [52..54] "b" topology level 0: boundary at 4
    7, 0, // [54..56] "b" topology level 0: boundary at 7
    8, 0, // [56..58] "b" topology level 0: boundary at 8
    255, 255, // [58..60] "b" topology level 1: sentinel
    1, 0, // [60..62] "b" topology level 1: boundary at 1
    3, 0, // [62..64] "b" topology level 1: boundary at 3
    3, 0, // [64..66] "b" topology level 1: boundary at 3
    4, 0, // [66..68] "b" topology level 1: boundary at 4
    6, 0, // [68..70] "b" topology level 1: boundary at 6
    6, 0, // [70..72] "b" topology level 1: boundary at 6
    7, 0, // [72..74] "b" topology level 1: boundary at 7
    8, 0, // [74..76] "b" topology level 1: boundary at 8
    255, 255, // [76..78] "c" topology level 0: sentinel
    3, 0, // [78..80] "c" topology level 0: boundary at 3
    3, 0, // [80..82] "c" topology level 0: boundary at 3
    4, 0, // [82..84] "c" topology level 0: boundary at 4
    6, 0, // [84..86] "c" topology level 0: boundary at 6
    6, 0, // [86..88] "c" topology level 0: boundary at 6
    7, 0, // [88..90] "c" topology level 0: boundary at 7
    0, 0, 0, 0, 0, 0, // [90..96] alignment padding (6 bytes)
    1, 0, 0, 0, 0, 0, 0, 0, // [96..104] "b"[0] = 1
    2, 0, 0, 0, 0, 0, 0, 0, // [104..112] "b"[1] = 2
    3, 0, 0, 0, 0, 0, 0, 0, // [112..120] "b"[2] = 3
    4, 0, 0, 0, 0, 0, 0, 0, // [120..128] "b"[3] = 4
    5, 0, 0, 0, 0, 0, 0, 0, // [128..136] "b"[4] = 5
    6, 0, 0, 0, 0, 0, 0, 0, // [136..144] "b"[5] = 6
    7, 0, 0, 0, 0, 0, 0, 0, // [144..152] "b"[6] = 7
    8, 0, 0, 0, 0, 0, 0, 0, // [152..160] "b"[7] = 8
    255, 255, 255, 255, // [160..164] "d" valid[0] = -1
    3, 0, 0, 0, // [164..168] "d" valid[1] = 3
    252, 255, 255, 255, // [168..172] "d" valid[2] = -4
    6, 0, 0, 0,  // [172..176] "d" valid[3] = 6
    45, // [176..177] "d" validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
    0,  // [177..178] alignment padding (1 byte)
    1, 0, // [178..180] "c"[0] = 1
    2, 0, // [180..182] "c"[1] = 2
    3, 0, // [182..184] "c"[2] = 3
    4, 0, // [184..186] "c"[3] = 4
    5, 0, // [186..188] "c"[4] = 5
    6, 0, // [188..190] "c"[5] = 6
    7, 0, // [190..192] "c"[6] = 7
    1, // [192..193] "a"[0] = 1
    2, // [193..194] "a"[1] = 2
    3, // [194..195] "a"[2] = 3
    4, // [195..196] "a"[3] = 4
    5, // [196..197] "a"[4] = 5
    6, // [197..198] "a"[5] = 6
];
pub const GROUP_3_NESTED_INODES: [Inode; 5] = [
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 3,
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
fn test_group_3_nested() {
    let schema = group_3_schema(vec![
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
        group_3_value(group_3_nested_entry),
        &GROUP_3_NESTED,
        &GROUP_3_NESTED_INODES,
    );
}
