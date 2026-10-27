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
use crate::{PyroValue, wire::tests::support::run_list_wrapped_roundtrip_u32};

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

pub const GROUP_1_PRIMITIVE: [u8; 71] = [
    6, 0, 0, 0, // group count
    65, 0, 0, 0, // "a" payload start pointer
    6, 0, 0, 0, // "a" sample count
    32, 0, 0, 0, // "b" payload start pointer
    6, 0, 0, 0, // "b" sample count
    56, 0, 0, 0, // "c" payload start pointer
    4, 0, 0, 0, // "c" valid sample count
    6, 0, 0, 0, // "c" total sample count (nulls included)
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
            data_offset: 4,
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
            data_offset: 12,
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
            data_offset: 20,
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
    run_list_wrapped_roundtrip_u32(
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

pub const GROUP_1_BLOB: [u8; 80] = [
    6, 0, 0, 0, // group count
    28, 0, 0, 0, // "s" byte-boundary start pointer
    7, 0, 0, 0, // "s" byte-boundary word count
    68, 0, 0, 0, // "s" payload start pointer
    12, 0, 0, 0, // "s" payload byte count
    56, 0, 0, 0, // "n" payload start pointer
    6, 0, 0, 0, // "n" sample count
    255, 255, 255, 255, // "s" byte boundaries: sentinel
    2, 0, 0, 0, // "s" byte boundary at 2 ("ab" ends)
    2, 0, 0, 0, // "s" byte boundary at 2 ("" ends)
    5, 0, 0, 0, // "s" byte boundary at 5 ("cde" ends)
    6, 0, 0, 0, // "s" byte boundary at 6 ("f" ends)
    10, 0, 0, 0, // "s" byte boundary at 10 ("ghij" ends)
    12, 0, 0, 0, // "s" byte boundary at 12 ("é" ends)
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
            data_offset: 12,
            topology_offset: 4,
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
            data_offset: 20,
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
    run_list_wrapped_roundtrip_u32(
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

pub const GROUP_1_MIXED: [u8; 203] = [
    6, 0, 0, 0, // [0..4] group count
    171, 0, 0, 0, // [4..8] "a" payload start pointer
    6, 0, 0, 0, // [8..12] "a" sample count
    144, 0, 0, 0, // [12..16] "b" payload start pointer
    4, 0, 0, 0, // [16..20] "b" valid sample count
    6, 0, 0, 0, // [20..24] "b" total sample count (nulls included)
    68, 0, 0, 0, // [24..28] "c" byte-boundary start pointer
    7, 0, 0, 0, // [28..32] "c" byte-boundary word count
    172, 0, 0, 0, // [32..36] "c" payload start pointer
    13, 0, 0, 0, // [36..40] "c" payload byte count
    96, 0, 0, 0, // [40..44] "d" payload start pointer
    6, 0, 0, 0, // [44..48] "d" sample count
    162, 0, 0, 0, // [48..52] "e" payload start pointer
    4, 0, 0, 0, // [52..56] "e" valid sample count
    6, 0, 0, 0, // [56..60] "e" total sample count (nulls included)
    185, 0, 0, 0, // [60..64] "f" payload start pointer
    6, 0, 0, 0, // [64..68] "f" array count
    255, 255, 255, 255, // [68..72] "c" byte-boundary level 0: sentinel
    2, 0, 0, 0, // [72..76] "c" byte-boundary level 0: boundary at 2
    2, 0, 0, 0, // [76..80] "c" byte-boundary level 0: boundary at 2
    5, 0, 0, 0, // [80..84] "c" byte-boundary level 0: boundary at 5
    7, 0, 0, 0, // [84..88] "c" byte-boundary level 0: boundary at 7
    12, 0, 0, 0, // [88..92] "c" byte-boundary level 0: boundary at 12
    13, 0, 0, 0, // [92..96] "c" byte-boundary level 0: boundary at 13
    1, 0, 0, 0, 0, 0, 0, 0, // [96..104] "d"[0] = 1
    3, 0, 0, 0, 0, 1, 0, 0, // [104..112] "d"[1] = 1099511627779
    3, 0, 0, 0, 0, 0, 0, 0, // [112..120] "d"[2] = 3
    4, 0, 0, 0, 0, 0, 0, 0, // [120..128] "d"[3] = 4
    255, 255, 255, 255, 255, 255, 255, 255, // [128..136] "d"[4] = 18446744073709551615
    6, 0, 0, 0, 0, 0, 0, 0, // [136..144] "d"[5] = 6
    0, 0, 192, 63, // [144..148] "b" valid[0] = 1.5
    0, 0, 16, 192, // [148..152] "b" valid[1] = -2.25
    0, 0, 0, 65, // [152..156] "b" valid[2] = 8.0
    0, 0, 0, 63, // [156..160] "b" valid[3] = 0.5
    45, // [160..161] "b" validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
    0,  // [161..162] alignment padding (1 byte)
    254, 255, // [162..164] "e" valid[0] = -2
    3, 0, // [164..166] "e" valid[1] = 3
    251, 255, // [166..168] "e" valid[2] = -5
    6, 0,  // [168..170] "e" valid[3] = 6
    54, // [170..171] "e" validity mask bits 0..6 (LSB first) = 0, 1, 1, 0, 1, 1
    25, // [171..172] "a" payload bits 0..6 (LSB first) = 1, 0, 0, 1, 1, 0
    97, 98, // [172..174] "c" payload "ab"
    99, 100, 101, // [174..177] "c" payload "cde"
    195, 169, // [177..179] "c" payload "é"
    102, 103, 104, 105, 106, // [179..184] "c" payload "fghij"
    107, // [184..185] "c" payload "k"
    1, 2, 3, // [185..188] "f"[0] = [1, 2, 3]
    4, 5, 6, // [188..191] "f"[1] = [4, 5, 6]
    7, 8, 9, // [191..194] "f"[2] = [7, 8, 9]
    10, 11, 12, // [194..197] "f"[3] = [10, 11, 12]
    13, 14, 15, // [197..200] "f"[4] = [13, 14, 15]
    16, 17, 18, // [200..203] "f"[5] = [16, 17, 18]
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
            data_offset: 12,
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
            data_offset: 32,
            topology_offset: 24,
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
            data_offset: 40,
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
            data_offset: 48,
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
            data_offset: 60,
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
    run_list_wrapped_roundtrip_u32(
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

pub const GROUP_1_NESTED: [u8; 254] = [
    6, 0, 0, 0, // [0..4] group count
    248, 0, 0, 0, // [4..8] "a" payload start pointer
    6, 0, 0, 0, // [8..12] "a" sample count
    56, 0, 0, 0, // [12..16] "b" topology start pointer
    16, 0, 0, 0, // [16..20] "b" topology word count
    152, 0, 0, 0, // [20..24] "b" payload start pointer
    8, 0, 0, 0, // [24..28] "b" sample count
    120, 0, 0, 0, // [28..32] "c" topology start pointer
    7, 0, 0, 0, // [32..36] "c" topology word count
    234, 0, 0, 0, // [36..40] "c" payload start pointer
    7, 0, 0, 0, // [40..44] "c" sample count
    216, 0, 0, 0, // [44..48] "d" payload start pointer
    4, 0, 0, 0, // [48..52] "d" valid sample count
    6, 0, 0, 0, // [52..56] "d" total sample count (nulls included)
    255, 255, 255, 255, // [56..60] "b" topology level 0: sentinel
    2, 0, 0, 0, // [60..64] "b" topology level 0: boundary at 2
    2, 0, 0, 0, // [64..68] "b" topology level 0: boundary at 2
    3, 0, 0, 0, // [68..72] "b" topology level 0: boundary at 3
    4, 0, 0, 0, // [72..76] "b" topology level 0: boundary at 4
    7, 0, 0, 0, // [76..80] "b" topology level 0: boundary at 7
    8, 0, 0, 0, // [80..84] "b" topology level 0: boundary at 8
    255, 255, 255, 255, // [84..88] "b" topology level 1: sentinel
    1, 0, 0, 0, // [88..92] "b" topology level 1: boundary at 1
    3, 0, 0, 0, // [92..96] "b" topology level 1: boundary at 3
    3, 0, 0, 0, // [96..100] "b" topology level 1: boundary at 3
    4, 0, 0, 0, // [100..104] "b" topology level 1: boundary at 4
    6, 0, 0, 0, // [104..108] "b" topology level 1: boundary at 6
    6, 0, 0, 0, // [108..112] "b" topology level 1: boundary at 6
    7, 0, 0, 0, // [112..116] "b" topology level 1: boundary at 7
    8, 0, 0, 0, // [116..120] "b" topology level 1: boundary at 8
    255, 255, 255, 255, // [120..124] "c" topology level 0: sentinel
    3, 0, 0, 0, // [124..128] "c" topology level 0: boundary at 3
    3, 0, 0, 0, // [128..132] "c" topology level 0: boundary at 3
    4, 0, 0, 0, // [132..136] "c" topology level 0: boundary at 4
    6, 0, 0, 0, // [136..140] "c" topology level 0: boundary at 6
    6, 0, 0, 0, // [140..144] "c" topology level 0: boundary at 6
    7, 0, 0, 0, // [144..148] "c" topology level 0: boundary at 7
    0, 0, 0, 0, // [148..152] alignment padding (4 bytes)
    1, 0, 0, 0, 0, 0, 0, 0, // [152..160] "b"[0] = 1
    2, 0, 0, 0, 0, 0, 0, 0, // [160..168] "b"[1] = 2
    3, 0, 0, 0, 0, 0, 0, 0, // [168..176] "b"[2] = 3
    4, 0, 0, 0, 0, 0, 0, 0, // [176..184] "b"[3] = 4
    5, 0, 0, 0, 0, 0, 0, 0, // [184..192] "b"[4] = 5
    6, 0, 0, 0, 0, 0, 0, 0, // [192..200] "b"[5] = 6
    7, 0, 0, 0, 0, 0, 0, 0, // [200..208] "b"[6] = 7
    8, 0, 0, 0, 0, 0, 0, 0, // [208..216] "b"[7] = 8
    255, 255, 255, 255, // [216..220] "d" valid[0] = -1
    3, 0, 0, 0, // [220..224] "d" valid[1] = 3
    252, 255, 255, 255, // [224..228] "d" valid[2] = -4
    6, 0, 0, 0,  // [228..232] "d" valid[3] = 6
    45, // [232..233] "d" validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
    0,  // [233..234] alignment padding (1 byte)
    1, 0, // [234..236] "c"[0] = 1
    2, 0, // [236..238] "c"[1] = 2
    3, 0, // [238..240] "c"[2] = 3
    4, 0, // [240..242] "c"[3] = 4
    5, 0, // [242..244] "c"[4] = 5
    6, 0, // [244..246] "c"[5] = 6
    7, 0, // [246..248] "c"[6] = 7
    1, // [248..249] "a"[0] = 1
    2, // [249..250] "a"[1] = 2
    3, // [250..251] "a"[2] = 3
    4, // [251..252] "a"[3] = 4
    5, // [252..253] "a"[4] = 5
    6, // [253..254] "a"[5] = 6
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
            data_offset: 20,
            topology_offset: 12,
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
            data_offset: 36,
            topology_offset: 28,
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
            data_offset: 44,
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
    run_list_wrapped_roundtrip_u32(
        schema,
        group_1_value(group_1_nested_entry),
        &GROUP_1_NESTED,
        &GROUP_1_NESTED_INODES,
    );
}
