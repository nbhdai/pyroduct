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
use crate::{PyroValue, value::Time, wire::tests::support::run_list_wrapped_roundtrip_u32};

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

pub const GROUP_3_PRIMITIVE: [u8; 183] = [
    36, 0, 0, 0, // group topology start pointer
    7, 0, 0, 0, // group topology word count (2 levels, sentinels included)
    64, 0, 0, 0, // "a" payload start pointer
    6, 0, 0, 0, // "a" sample count
    177, 0, 0, 0, // "b" payload start pointer
    6, 0, 0, 0, // "b" sample count
    160, 0, 0, 0, // "c" payload start pointer
    4, 0, 0, 0, // "c" valid sample count
    6, 0, 0, 0, // "c" total sample count (nulls included)
    255, 255, 255, 255, // group level 0: sentinel
    2, 0, 0, 0, // group level 0: boundary at 2 lists
    3, 0, 0, 0, // group level 0: boundary at 3 lists
    255, 255, 255, 255, // group level 1: sentinel
    2, 0, 0, 0, // group level 1: boundary at 2 groups
    3, 0, 0, 0, // group level 1: boundary at 3 groups
    6, 0, 0, 0, // group level 1: boundary at 6 groups
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
            data_offset: 8,
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
            data_offset: 16,
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
    run_list_wrapped_roundtrip_u32(
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

pub const GROUP_3_BLOB: [u8; 124] = [
    32, 0, 0, 0, // group topology start pointer
    7, 0, 0, 0, // group topology word count (2 levels, sentinels included)
    88, 0, 0, 0, // "id" payload start pointer
    6, 0, 0, 0, // "id" sample count
    60, 0, 0, 0, // "s" byte-boundary start pointer
    7, 0, 0, 0, // "s" byte-boundary word count
    112, 0, 0, 0, // "s" payload start pointer
    12, 0, 0, 0, // "s" payload byte count
    255, 255, 255, 255, // group level 0: sentinel
    2, 0, 0, 0, // group level 0: boundary at 2 lists
    3, 0, 0, 0, // group level 0: boundary at 3 lists
    255, 255, 255, 255, // group level 1: sentinel
    2, 0, 0, 0, // group level 1: boundary at 2 groups
    3, 0, 0, 0, // group level 1: boundary at 3 groups
    6, 0, 0, 0, // group level 1: boundary at 6 groups
    255, 255, 255, 255, // "s" byte boundaries: sentinel
    2, 0, 0, 0, // "s" byte boundary at 2 ("ab" ends)
    2, 0, 0, 0, // "s" byte boundary at 2 ("" ends)
    5, 0, 0, 0, // "s" byte boundary at 5 ("cde" ends)
    6, 0, 0, 0, // "s" byte boundary at 6 ("f" ends)
    10, 0, 0, 0, // "s" byte boundary at 10 ("ghij" ends)
    12, 0, 0, 0, // "s" byte boundary at 12 ("é" ends)
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
            data_offset: 24,
            topology_offset: 16,
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
    run_list_wrapped_roundtrip_u32(
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

pub const GROUP_3_MIXED: [u8; 235] = [
    72, 0, 0, 0, // [0..4] group topology start pointer
    7, 0, 0, 0, // [4..8] group topology word count
    203, 0, 0, 0, // [8..12] "a" payload start pointer
    6, 0, 0, 0, // [12..16] "a" sample count
    176, 0, 0, 0, // [16..20] "b" payload start pointer
    4, 0, 0, 0, // [20..24] "b" valid sample count
    6, 0, 0, 0, // [24..28] "b" total sample count (nulls included)
    100, 0, 0, 0, // [28..32] "c" byte-boundary start pointer
    7, 0, 0, 0, // [32..36] "c" byte-boundary word count
    204, 0, 0, 0, // [36..40] "c" payload start pointer
    13, 0, 0, 0, // [40..44] "c" payload byte count
    128, 0, 0, 0, // [44..48] "d" payload start pointer
    6, 0, 0, 0, // [48..52] "d" sample count
    194, 0, 0, 0, // [52..56] "e" payload start pointer
    4, 0, 0, 0, // [56..60] "e" valid sample count
    6, 0, 0, 0, // [60..64] "e" total sample count (nulls included)
    217, 0, 0, 0, // [64..68] "f" payload start pointer
    6, 0, 0, 0, // [68..72] "f" array count
    255, 255, 255, 255, // [72..76] group level 0: sentinel
    2, 0, 0, 0, // [76..80] group level 0: boundary at 2
    3, 0, 0, 0, // [80..84] group level 0: boundary at 3
    255, 255, 255, 255, // [84..88] group level 1: sentinel
    2, 0, 0, 0, // [88..92] group level 1: boundary at 2
    3, 0, 0, 0, // [92..96] group level 1: boundary at 3
    6, 0, 0, 0, // [96..100] group level 1: boundary at 6
    255, 255, 255, 255, // [100..104] "c" byte-boundary level 0: sentinel
    2, 0, 0, 0, // [104..108] "c" byte-boundary level 0: boundary at 2
    2, 0, 0, 0, // [108..112] "c" byte-boundary level 0: boundary at 2
    5, 0, 0, 0, // [112..116] "c" byte-boundary level 0: boundary at 5
    7, 0, 0, 0, // [116..120] "c" byte-boundary level 0: boundary at 7
    12, 0, 0, 0, // [120..124] "c" byte-boundary level 0: boundary at 12
    13, 0, 0, 0, // [124..128] "c" byte-boundary level 0: boundary at 13
    1, 0, 0, 0, 0, 0, 0, 0, // [128..136] "d"[0] = 1
    3, 0, 0, 0, 0, 1, 0, 0, // [136..144] "d"[1] = 1099511627779
    3, 0, 0, 0, 0, 0, 0, 0, // [144..152] "d"[2] = 3
    4, 0, 0, 0, 0, 0, 0, 0, // [152..160] "d"[3] = 4
    255, 255, 255, 255, 255, 255, 255, 255, // [160..168] "d"[4] = 18446744073709551615
    6, 0, 0, 0, 0, 0, 0, 0, // [168..176] "d"[5] = 6
    0, 0, 192, 63, // [176..180] "b" valid[0] = 1.5
    0, 0, 16, 192, // [180..184] "b" valid[1] = -2.25
    0, 0, 0, 65, // [184..188] "b" valid[2] = 8.0
    0, 0, 0, 63, // [188..192] "b" valid[3] = 0.5
    45, // [192..193] "b" validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
    0,  // [193..194] alignment padding (1 byte)
    254, 255, // [194..196] "e" valid[0] = -2
    3, 0, // [196..198] "e" valid[1] = 3
    251, 255, // [198..200] "e" valid[2] = -5
    6, 0,  // [200..202] "e" valid[3] = 6
    54, // [202..203] "e" validity mask bits 0..6 (LSB first) = 0, 1, 1, 0, 1, 1
    25, // [203..204] "a" payload bits 0..6 (LSB first) = 1, 0, 0, 1, 1, 0
    97, 98, // [204..206] "c" payload "ab"
    99, 100, 101, // [206..209] "c" payload "cde"
    195, 169, // [209..211] "c" payload "é"
    102, 103, 104, 105, 106, // [211..216] "c" payload "fghij"
    107, // [216..217] "c" payload "k"
    1, 2, 3, // [217..220] "f"[0] = [1, 2, 3]
    4, 5, 6, // [220..223] "f"[1] = [4, 5, 6]
    7, 8, 9, // [223..226] "f"[2] = [7, 8, 9]
    10, 11, 12, // [226..229] "f"[3] = [10, 11, 12]
    13, 14, 15, // [229..232] "f"[4] = [13, 14, 15]
    16, 17, 18, // [232..235] "f"[5] = [16, 17, 18]
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
            data_offset: 8,
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
            data_offset: 16,
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
            data_offset: 36,
            topology_offset: 28,
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
            data_offset: 44,
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
            data_offset: 52,
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
            data_offset: 64,
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
    run_list_wrapped_roundtrip_u32(
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

pub const GROUP_3_NESTED: [u8; 286] = [
    60, 0, 0, 0, // [0..4] group topology start pointer
    7, 0, 0, 0, // [4..8] group topology word count
    24, 1, 0, 0, // [8..12] "a" payload start pointer
    6, 0, 0, 0, // [12..16] "a" sample count
    88, 0, 0, 0, // [16..20] "b" topology start pointer
    16, 0, 0, 0, // [20..24] "b" topology word count
    184, 0, 0, 0, // [24..28] "b" payload start pointer
    8, 0, 0, 0, // [28..32] "b" sample count
    152, 0, 0, 0, // [32..36] "c" topology start pointer
    7, 0, 0, 0, // [36..40] "c" topology word count
    10, 1, 0, 0, // [40..44] "c" payload start pointer
    7, 0, 0, 0, // [44..48] "c" sample count
    248, 0, 0, 0, // [48..52] "d" payload start pointer
    4, 0, 0, 0, // [52..56] "d" valid sample count
    6, 0, 0, 0, // [56..60] "d" total sample count (nulls included)
    255, 255, 255, 255, // [60..64] group level 0: sentinel
    2, 0, 0, 0, // [64..68] group level 0: boundary at 2
    3, 0, 0, 0, // [68..72] group level 0: boundary at 3
    255, 255, 255, 255, // [72..76] group level 1: sentinel
    2, 0, 0, 0, // [76..80] group level 1: boundary at 2
    3, 0, 0, 0, // [80..84] group level 1: boundary at 3
    6, 0, 0, 0, // [84..88] group level 1: boundary at 6
    255, 255, 255, 255, // [88..92] "b" topology level 0: sentinel
    2, 0, 0, 0, // [92..96] "b" topology level 0: boundary at 2
    2, 0, 0, 0, // [96..100] "b" topology level 0: boundary at 2
    3, 0, 0, 0, // [100..104] "b" topology level 0: boundary at 3
    4, 0, 0, 0, // [104..108] "b" topology level 0: boundary at 4
    7, 0, 0, 0, // [108..112] "b" topology level 0: boundary at 7
    8, 0, 0, 0, // [112..116] "b" topology level 0: boundary at 8
    255, 255, 255, 255, // [116..120] "b" topology level 1: sentinel
    1, 0, 0, 0, // [120..124] "b" topology level 1: boundary at 1
    3, 0, 0, 0, // [124..128] "b" topology level 1: boundary at 3
    3, 0, 0, 0, // [128..132] "b" topology level 1: boundary at 3
    4, 0, 0, 0, // [132..136] "b" topology level 1: boundary at 4
    6, 0, 0, 0, // [136..140] "b" topology level 1: boundary at 6
    6, 0, 0, 0, // [140..144] "b" topology level 1: boundary at 6
    7, 0, 0, 0, // [144..148] "b" topology level 1: boundary at 7
    8, 0, 0, 0, // [148..152] "b" topology level 1: boundary at 8
    255, 255, 255, 255, // [152..156] "c" topology level 0: sentinel
    3, 0, 0, 0, // [156..160] "c" topology level 0: boundary at 3
    3, 0, 0, 0, // [160..164] "c" topology level 0: boundary at 3
    4, 0, 0, 0, // [164..168] "c" topology level 0: boundary at 4
    6, 0, 0, 0, // [168..172] "c" topology level 0: boundary at 6
    6, 0, 0, 0, // [172..176] "c" topology level 0: boundary at 6
    7, 0, 0, 0, // [176..180] "c" topology level 0: boundary at 7
    0, 0, 0, 0, // [180..184] alignment padding (4 bytes)
    1, 0, 0, 0, 0, 0, 0, 0, // [184..192] "b"[0] = 1
    2, 0, 0, 0, 0, 0, 0, 0, // [192..200] "b"[1] = 2
    3, 0, 0, 0, 0, 0, 0, 0, // [200..208] "b"[2] = 3
    4, 0, 0, 0, 0, 0, 0, 0, // [208..216] "b"[3] = 4
    5, 0, 0, 0, 0, 0, 0, 0, // [216..224] "b"[4] = 5
    6, 0, 0, 0, 0, 0, 0, 0, // [224..232] "b"[5] = 6
    7, 0, 0, 0, 0, 0, 0, 0, // [232..240] "b"[6] = 7
    8, 0, 0, 0, 0, 0, 0, 0, // [240..248] "b"[7] = 8
    255, 255, 255, 255, // [248..252] "d" valid[0] = -1
    3, 0, 0, 0, // [252..256] "d" valid[1] = 3
    252, 255, 255, 255, // [256..260] "d" valid[2] = -4
    6, 0, 0, 0,  // [260..264] "d" valid[3] = 6
    45, // [264..265] "d" validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
    0,  // [265..266] alignment padding (1 byte)
    1, 0, // [266..268] "c"[0] = 1
    2, 0, // [268..270] "c"[1] = 2
    3, 0, // [270..272] "c"[2] = 3
    4, 0, // [272..274] "c"[3] = 4
    5, 0, // [274..276] "c"[4] = 5
    6, 0, // [276..278] "c"[5] = 6
    7, 0, // [278..280] "c"[6] = 7
    1, // [280..281] "a"[0] = 1
    2, // [281..282] "a"[1] = 2
    3, // [282..283] "a"[2] = 3
    4, // [283..284] "a"[3] = 4
    5, // [284..285] "a"[4] = 5
    6, // [285..286] "a"[5] = 6
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
            data_offset: 8,
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
            data_offset: 24,
            topology_offset: 16,
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
            data_offset: 40,
            topology_offset: 32,
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
            data_offset: 48,
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
    run_list_wrapped_roundtrip_u32(
        schema,
        group_3_value(group_3_nested_entry),
        &GROUP_3_NESTED,
        &GROUP_3_NESTED_INODES,
    );
}
