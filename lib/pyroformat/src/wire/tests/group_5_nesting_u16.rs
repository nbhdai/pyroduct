//! `List<List<List<List<List<Group<..>>>>>>` with two different groups:
//! * `primitive`: `Group{a: bool, b: bool, c: i32, d: Option<bool>}`
//! * `blob`: `Group{s: String, t: Vec<u8>}`
//!
//! Shared value shape: `[[[[[a, b]], [[c]]], [[[d]]]], [[[[e, f]]]]]`, each letter one group `0..6`;
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
fn group_5_schema(fields: Vec<PyroField>) -> PyroSchema {
    let mut ty = PyroType::Group { fields };
    for _ in 0..5 {
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

/// `[[[[[a, b]], [[c]]], [[[d]]]], [[[[e, f]]]]]` over groups `0..6`, each built by `entry`.
#[cfg(test)]
fn group_5_value(entry: fn(usize) -> PyroValue<'static>) -> PyroValue<'static> {
    PyroValue::List(vec![
        PyroValue::List(vec![
            PyroValue::List(vec![
                PyroValue::List(vec![PyroValue::List(vec![entry(0), entry(1)])]),
                PyroValue::List(vec![PyroValue::List(vec![entry(2)])]),
            ]),
            PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![entry(3)])])]),
        ]),
        PyroValue::List(vec![PyroValue::List(vec![PyroValue::List(vec![
            PyroValue::List(vec![entry(4), entry(5)]),
        ])])]),
    ])
}

// ---------------------------------------------------------------------------
// primitive: Group{a: bool, b: bool, c: i32, d: Option<bool>}
// ---------------------------------------------------------------------------

/// Group `i` of `primitive`.
#[cfg(test)]
fn group_5_primitive_entry(i: usize) -> PyroValue<'static> {
    let a = [
        PyroValue::Bool(true),
        PyroValue::Bool(false),
        PyroValue::Bool(true),
        PyroValue::Bool(true),
        PyroValue::Bool(true),
        PyroValue::Bool(false),
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
        PyroValue::I32(-1),
        PyroValue::I32(2),
        PyroValue::I32(-3),
        PyroValue::I32(4),
        PyroValue::I32(-5),
        PyroValue::I32(6),
    ];
    let d = [
        PyroValue::Bool(true),
        PyroValue::Null,
        PyroValue::Bool(true),
        PyroValue::Bool(true),
        PyroValue::Null,
        PyroValue::Bool(false),
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

pub const GROUP_5_PRIMITIVE: [u8; 84] = [
    22, 0, // group topology start pointer
    17, 0, // group topology word count (4 levels, sentinels included)
    80, 0, // "a" payload start pointer
    6, 0, // "a" sample count
    81, 0, // "b" payload start pointer
    6, 0, // "b" sample count
    56, 0, // "c" payload start pointer
    6, 0, // "c" sample count
    82, 0, // "d" payload start pointer
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
    1, 0, // group level 2: boundary at 1 lists
    2, 0, // group level 2: boundary at 2 lists
    3, 0, // group level 2: boundary at 3 lists
    4, 0, // group level 2: boundary at 4 lists
    255, 255, // group level 3: sentinel
    2, 0, // group level 3: boundary at 2 groups
    3, 0, // group level 3: boundary at 3 groups
    4, 0, // group level 3: boundary at 4 groups
    6, 0, // group level 3: boundary at 6 groups
    255, 255, 255, 255, // "c"[group 0] = -1
    2, 0, 0, 0, // "c"[group 1] = 2
    253, 255, 255, 255, // "c"[group 2] = -3
    4, 0, 0, 0, // "c"[group 3] = 4
    251, 255, 255, 255, // "c"[group 4] = -5
    6, 0, 0, 0,  // "c"[group 5] = 6
    29, // "a" payload bits (LSB first) = 1, 0, 1, 1, 1, 0
    29, // "b" payload bits (LSB first) = 1, 0, 1, 1, 1, 0
    7,  // "d" valid payload bits (LSB first) = 1, 1, 1, 0
    45, // "d" validity mask bits (LSB first) = 1, 0, 1, 1, 0, 1
];
pub const GROUP_5_PRIMITIVE_INODES: [Inode; 5] = [
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 5,
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
        kind: WireKind::I32,
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
        kind: WireKind::OptionBool,
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
fn test_group_5_primitive() {
    let schema = group_5_schema(vec![
        field(
            "a",
            PyroType::PrimitiveScalar {
                primitive: PrimitiveDataType::Bool,
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
                primitive: PrimitiveDataType::I32,
            },
        ),
        field(
            "d",
            PyroType::OptionPrimitiveScalar {
                primitive: PrimitiveDataType::Bool,
            },
        ),
    ]);
    run_list_wrapped_roundtrip_u16(
        schema,
        group_5_value(group_5_primitive_entry),
        &GROUP_5_PRIMITIVE,
        &GROUP_5_PRIMITIVE_INODES,
    );
}

// ---------------------------------------------------------------------------
// blob: Group{s: String, t: Vec<u8>}
// ---------------------------------------------------------------------------

/// Group `i` of `blob`.
#[cfg(test)]
fn group_5_blob_entry(i: usize) -> PyroValue<'static> {
    let s = [
        PyroValue::Str("ab".into()),
        PyroValue::Str("".into()),
        PyroValue::Str("cde".into()),
        PyroValue::Str("f".into()),
        PyroValue::Str("ghij".into()),
        PyroValue::Str("é".into()),
    ];
    let t = [
        PyroValue::Bytes(vec![1, 2].into()),
        PyroValue::Bytes(vec![].into()),
        PyroValue::Bytes(vec![3, 4, 5].into()),
        PyroValue::Bytes(vec![255].into()),
        PyroValue::Bytes(vec![0, 6, 7, 8].into()),
        PyroValue::Bytes(vec![9].into()),
    ];
    PyroValue::Group([("s", s[i].clone()), ("t", t[i].clone())].into())
}

pub const GROUP_5_BLOB: [u8; 105] = [
    20, 0, // group topology start pointer
    17, 0, // group topology word count (4 levels, sentinels included)
    54, 0, // "s" byte-boundary start pointer
    7, 0, // "s" byte-boundary word count
    82, 0, // "s" payload start pointer
    12, 0, // "s" payload byte count
    68, 0, // "t" byte-boundary start pointer
    7, 0, // "t" byte-boundary word count
    94, 0, // "t" payload start pointer
    11, 0, // "t" payload byte count
    255, 255, // group level 0: sentinel
    2, 0, // group level 0: boundary at 2 lists
    3, 0, // group level 0: boundary at 3 lists
    255, 255, // group level 1: sentinel
    2, 0, // group level 1: boundary at 2 lists
    3, 0, // group level 1: boundary at 3 lists
    4, 0, // group level 1: boundary at 4 lists
    255, 255, // group level 2: sentinel
    1, 0, // group level 2: boundary at 1 lists
    2, 0, // group level 2: boundary at 2 lists
    3, 0, // group level 2: boundary at 3 lists
    4, 0, // group level 2: boundary at 4 lists
    255, 255, // group level 3: sentinel
    2, 0, // group level 3: boundary at 2 groups
    3, 0, // group level 3: boundary at 3 groups
    4, 0, // group level 3: boundary at 4 groups
    6, 0, // group level 3: boundary at 6 groups
    255, 255, // "s" byte boundaries: sentinel
    2, 0, // "s" byte boundary at 2 ("ab" ends)
    2, 0, // "s" byte boundary at 2 ("" ends)
    5, 0, // "s" byte boundary at 5 ("cde" ends)
    6, 0, // "s" byte boundary at 6 ("f" ends)
    10, 0, // "s" byte boundary at 10 ("ghij" ends)
    12, 0, // "s" byte boundary at 12 ("é" ends)
    255, 255, // "t" byte boundaries: sentinel
    2, 0, // "t" byte boundary at 2 ([1, 2] ends)
    2, 0, // "t" byte boundary at 2 ([] ends)
    5, 0, // "t" byte boundary at 5 ([3, 4, 5] ends)
    6, 0, // "t" byte boundary at 6 ([255] ends)
    10, 0, // "t" byte boundary at 10 ([0, 6, 7, 8] ends)
    11, 0, // "t" byte boundary at 11 ([9] ends)
    97, 98, // "s" payload "ab"
    99, 100, 101, // "s" payload "cde"
    102, // "s" payload "f"
    103, 104, 105, 106, // "s" payload "ghij"
    195, 169, // "s" payload "é"
    1, 2, // "t" payload [1, 2]
    3, 4, 5,   // "t" payload [3, 4, 5]
    255, // "t" payload [255]
    0, 6, 7, 8, // "t" payload [0, 6, 7, 8]
    9, // "t" payload [9]
];
pub const GROUP_5_BLOB_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 5,
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
            data_offset: 8,
            topology_offset: 4,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::Bytes,
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
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_5_blob() {
    let schema = group_5_schema(vec![field("s", PyroType::Str), field("t", PyroType::Bytes)]);
    run_list_wrapped_roundtrip_u16(
        schema,
        group_5_value(group_5_blob_entry),
        &GROUP_5_BLOB,
        &GROUP_5_BLOB_INODES,
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
fn group_5_mixed_entry(i: usize) -> PyroValue<'static> {
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

pub const GROUP_5_MIXED: [u8; 195] = [
    36, 0, // [0..2] group topology start pointer
    17, 0, // [2..4] group topology word count
    163, 0, // [4..6] "a" payload start pointer
    6, 0, // [6..8] "a" sample count
    136, 0, // [8..10] "b" payload start pointer
    4, 0, // [10..12] "b" valid sample count
    6, 0, // [12..14] "b" total sample count (nulls included)
    70, 0, // [14..16] "c" byte-boundary start pointer
    7, 0, // [16..18] "c" byte-boundary word count
    164, 0, // [18..20] "c" payload start pointer
    13, 0, // [20..22] "c" payload byte count
    88, 0, // [22..24] "d" payload start pointer
    6, 0, // [24..26] "d" sample count
    154, 0, // [26..28] "e" payload start pointer
    4, 0, // [28..30] "e" valid sample count
    6, 0, // [30..32] "e" total sample count (nulls included)
    177, 0, // [32..34] "f" payload start pointer
    6, 0, // [34..36] "f" array count
    255, 255, // [36..38] group level 0: sentinel
    2, 0, // [38..40] group level 0: boundary at 2
    3, 0, // [40..42] group level 0: boundary at 3
    255, 255, // [42..44] group level 1: sentinel
    2, 0, // [44..46] group level 1: boundary at 2
    3, 0, // [46..48] group level 1: boundary at 3
    4, 0, // [48..50] group level 1: boundary at 4
    255, 255, // [50..52] group level 2: sentinel
    1, 0, // [52..54] group level 2: boundary at 1
    2, 0, // [54..56] group level 2: boundary at 2
    3, 0, // [56..58] group level 2: boundary at 3
    4, 0, // [58..60] group level 2: boundary at 4
    255, 255, // [60..62] group level 3: sentinel
    2, 0, // [62..64] group level 3: boundary at 2
    3, 0, // [64..66] group level 3: boundary at 3
    4, 0, // [66..68] group level 3: boundary at 4
    6, 0, // [68..70] group level 3: boundary at 6
    255, 255, // [70..72] "c" byte-boundary level 0: sentinel
    2, 0, // [72..74] "c" byte-boundary level 0: boundary at 2
    2, 0, // [74..76] "c" byte-boundary level 0: boundary at 2
    5, 0, // [76..78] "c" byte-boundary level 0: boundary at 5
    7, 0, // [78..80] "c" byte-boundary level 0: boundary at 7
    12, 0, // [80..82] "c" byte-boundary level 0: boundary at 12
    13, 0, // [82..84] "c" byte-boundary level 0: boundary at 13
    0, 0, 0, 0, // [84..88] alignment padding (4 bytes)
    1, 0, 0, 0, 0, 0, 0, 0, // [88..96] "d"[0] = 1
    3, 0, 0, 0, 0, 1, 0, 0, // [96..104] "d"[1] = 1099511627779
    3, 0, 0, 0, 0, 0, 0, 0, // [104..112] "d"[2] = 3
    4, 0, 0, 0, 0, 0, 0, 0, // [112..120] "d"[3] = 4
    255, 255, 255, 255, 255, 255, 255, 255, // [120..128] "d"[4] = 18446744073709551615
    6, 0, 0, 0, 0, 0, 0, 0, // [128..136] "d"[5] = 6
    0, 0, 192, 63, // [136..140] "b" valid[0] = 1.5
    0, 0, 16, 192, // [140..144] "b" valid[1] = -2.25
    0, 0, 0, 65, // [144..148] "b" valid[2] = 8.0
    0, 0, 0, 63, // [148..152] "b" valid[3] = 0.5
    45, // [152..153] "b" validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
    0,  // [153..154] alignment padding (1 byte)
    254, 255, // [154..156] "e" valid[0] = -2
    3, 0, // [156..158] "e" valid[1] = 3
    251, 255, // [158..160] "e" valid[2] = -5
    6, 0,  // [160..162] "e" valid[3] = 6
    54, // [162..163] "e" validity mask bits 0..6 (LSB first) = 0, 1, 1, 0, 1, 1
    25, // [163..164] "a" payload bits 0..6 (LSB first) = 1, 0, 0, 1, 1, 0
    97, 98, // [164..166] "c" payload "ab"
    99, 100, 101, // [166..169] "c" payload "cde"
    195, 169, // [169..171] "c" payload "é"
    102, 103, 104, 105, 106, // [171..176] "c" payload "fghij"
    107, // [176..177] "c" payload "k"
    1, 2, 3, // [177..180] "f"[0] = [1, 2, 3]
    4, 5, 6, // [180..183] "f"[1] = [4, 5, 6]
    7, 8, 9, // [183..186] "f"[2] = [7, 8, 9]
    10, 11, 12, // [186..189] "f"[3] = [10, 11, 12]
    13, 14, 15, // [189..192] "f"[4] = [13, 14, 15]
    16, 17, 18, // [192..195] "f"[5] = [16, 17, 18]
];
pub const GROUP_5_MIXED_INODES: [Inode; 7] = [
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 5,
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
fn test_group_5_mixed() {
    let schema = group_5_schema(vec![
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
        group_5_value(group_5_mixed_entry),
        &GROUP_5_MIXED,
        &GROUP_5_MIXED_INODES,
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
fn group_5_nested_entry(i: usize) -> PyroValue<'static> {
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

pub const GROUP_5_NESTED: [u8; 214] = [
    30, 0, // [0..2] group topology start pointer
    17, 0, // [2..4] group topology word count
    208, 0, // [4..6] "a" payload start pointer
    6, 0, // [6..8] "a" sample count
    64, 0, // [8..10] "b" topology start pointer
    16, 0, // [10..12] "b" topology word count
    112, 0, // [12..14] "b" payload start pointer
    8, 0, // [14..16] "b" sample count
    96, 0, // [16..18] "c" topology start pointer
    7, 0, // [18..20] "c" topology word count
    194, 0, // [20..22] "c" payload start pointer
    7, 0, // [22..24] "c" sample count
    176, 0, // [24..26] "d" payload start pointer
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
    1, 0, // [46..48] group level 2: boundary at 1
    2, 0, // [48..50] group level 2: boundary at 2
    3, 0, // [50..52] group level 2: boundary at 3
    4, 0, // [52..54] group level 2: boundary at 4
    255, 255, // [54..56] group level 3: sentinel
    2, 0, // [56..58] group level 3: boundary at 2
    3, 0, // [58..60] group level 3: boundary at 3
    4, 0, // [60..62] group level 3: boundary at 4
    6, 0, // [62..64] group level 3: boundary at 6
    255, 255, // [64..66] "b" topology level 0: sentinel
    2, 0, // [66..68] "b" topology level 0: boundary at 2
    2, 0, // [68..70] "b" topology level 0: boundary at 2
    3, 0, // [70..72] "b" topology level 0: boundary at 3
    4, 0, // [72..74] "b" topology level 0: boundary at 4
    7, 0, // [74..76] "b" topology level 0: boundary at 7
    8, 0, // [76..78] "b" topology level 0: boundary at 8
    255, 255, // [78..80] "b" topology level 1: sentinel
    1, 0, // [80..82] "b" topology level 1: boundary at 1
    3, 0, // [82..84] "b" topology level 1: boundary at 3
    3, 0, // [84..86] "b" topology level 1: boundary at 3
    4, 0, // [86..88] "b" topology level 1: boundary at 4
    6, 0, // [88..90] "b" topology level 1: boundary at 6
    6, 0, // [90..92] "b" topology level 1: boundary at 6
    7, 0, // [92..94] "b" topology level 1: boundary at 7
    8, 0, // [94..96] "b" topology level 1: boundary at 8
    255, 255, // [96..98] "c" topology level 0: sentinel
    3, 0, // [98..100] "c" topology level 0: boundary at 3
    3, 0, // [100..102] "c" topology level 0: boundary at 3
    4, 0, // [102..104] "c" topology level 0: boundary at 4
    6, 0, // [104..106] "c" topology level 0: boundary at 6
    6, 0, // [106..108] "c" topology level 0: boundary at 6
    7, 0, // [108..110] "c" topology level 0: boundary at 7
    0, 0, // [110..112] alignment padding (2 bytes)
    1, 0, 0, 0, 0, 0, 0, 0, // [112..120] "b"[0] = 1
    2, 0, 0, 0, 0, 0, 0, 0, // [120..128] "b"[1] = 2
    3, 0, 0, 0, 0, 0, 0, 0, // [128..136] "b"[2] = 3
    4, 0, 0, 0, 0, 0, 0, 0, // [136..144] "b"[3] = 4
    5, 0, 0, 0, 0, 0, 0, 0, // [144..152] "b"[4] = 5
    6, 0, 0, 0, 0, 0, 0, 0, // [152..160] "b"[5] = 6
    7, 0, 0, 0, 0, 0, 0, 0, // [160..168] "b"[6] = 7
    8, 0, 0, 0, 0, 0, 0, 0, // [168..176] "b"[7] = 8
    255, 255, 255, 255, // [176..180] "d" valid[0] = -1
    3, 0, 0, 0, // [180..184] "d" valid[1] = 3
    252, 255, 255, 255, // [184..188] "d" valid[2] = -4
    6, 0, 0, 0,  // [188..192] "d" valid[3] = 6
    45, // [192..193] "d" validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
    0,  // [193..194] alignment padding (1 byte)
    1, 0, // [194..196] "c"[0] = 1
    2, 0, // [196..198] "c"[1] = 2
    3, 0, // [198..200] "c"[2] = 3
    4, 0, // [200..202] "c"[3] = 4
    5, 0, // [202..204] "c"[4] = 5
    6, 0, // [204..206] "c"[5] = 6
    7, 0, // [206..208] "c"[6] = 7
    1, // [208..209] "a"[0] = 1
    2, // [209..210] "a"[1] = 2
    3, // [210..211] "a"[2] = 3
    4, // [211..212] "a"[3] = 4
    5, // [212..213] "a"[4] = 5
    6, // [213..214] "a"[5] = 6
];
pub const GROUP_5_NESTED_INODES: [Inode; 5] = [
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 5,
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
fn test_group_5_nested() {
    let schema = group_5_schema(vec![
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
        group_5_value(group_5_nested_entry),
        &GROUP_5_NESTED,
        &GROUP_5_NESTED_INODES,
    );
}
