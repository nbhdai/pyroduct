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
use crate::{PyroValue, wire::tests::support::run_list_wrapped_roundtrip_u32};

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

pub const GROUP_4_PRIMITIVE: [u8; 216] = [
    44, 0, 0, 0, // group topology start pointer
    12, 0, 0, 0, // group topology word count (3 levels, sentinels included)
    180, 0, 0, 0, // "a" payload start pointer
    6, 0, 0, 0, // "a" sample count
    96, 0, 0, 0, // "b" payload start pointer
    6, 0, 0, 0, // "b" sample count
    204, 0, 0, 0, // "c" payload start pointer
    6, 0, 0, 0, // "c" sample count
    144, 0, 0, 0, // "d" payload start pointer
    4, 0, 0, 0, // "d" valid sample count
    6, 0, 0, 0, // "d" total sample count (nulls included)
    255, 255, 255, 255, // group level 0: sentinel
    2, 0, 0, 0, // group level 0: boundary at 2 lists
    3, 0, 0, 0, // group level 0: boundary at 3 lists
    255, 255, 255, 255, // group level 1: sentinel
    2, 0, 0, 0, // group level 1: boundary at 2 lists
    3, 0, 0, 0, // group level 1: boundary at 3 lists
    4, 0, 0, 0, // group level 1: boundary at 4 lists
    255, 255, 255, 255, // group level 2: sentinel
    2, 0, 0, 0, // group level 2: boundary at 2 groups
    3, 0, 0, 0, // group level 2: boundary at 3 groups
    4, 0, 0, 0, // group level 2: boundary at 4 groups
    6, 0, 0, 0, // group level 2: boundary at 6 groups
    0, 0, 0, 0, // alignment padding to 8 bytes before "b"
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
            data_offset: 8,
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
            data_offset: 16,
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
            data_offset: 24,
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
            data_offset: 32,
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
    run_list_wrapped_roundtrip_u32(
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

pub const GROUP_4_BLOB: [u8; 171] = [
    32, 0, 0, 0, // group topology start pointer
    12, 0, 0, 0, // group topology word count (3 levels, sentinels included)
    80, 0, 0, 0, // "b" byte-boundary start pointer
    7, 0, 0, 0, // "b" byte-boundary word count
    160, 0, 0, 0, // "b" payload start pointer
    11, 0, 0, 0, // "b" payload byte count
    112, 0, 0, 0, // "f" payload start pointer
    6, 0, 0, 0, // "f" sample count
    255, 255, 255, 255, // group level 0: sentinel
    2, 0, 0, 0, // group level 0: boundary at 2 lists
    3, 0, 0, 0, // group level 0: boundary at 3 lists
    255, 255, 255, 255, // group level 1: sentinel
    2, 0, 0, 0, // group level 1: boundary at 2 lists
    3, 0, 0, 0, // group level 1: boundary at 3 lists
    4, 0, 0, 0, // group level 1: boundary at 4 lists
    255, 255, 255, 255, // group level 2: sentinel
    2, 0, 0, 0, // group level 2: boundary at 2 groups
    3, 0, 0, 0, // group level 2: boundary at 3 groups
    4, 0, 0, 0, // group level 2: boundary at 4 groups
    6, 0, 0, 0, // group level 2: boundary at 6 groups
    255, 255, 255, 255, // "b" byte boundaries: sentinel
    2, 0, 0, 0, // "b" byte boundary at 2 ([1, 2] ends)
    2, 0, 0, 0, // "b" byte boundary at 2 ([] ends)
    5, 0, 0, 0, // "b" byte boundary at 5 ([3, 4, 5] ends)
    6, 0, 0, 0, // "b" byte boundary at 6 ([255] ends)
    10, 0, 0, 0, // "b" byte boundary at 10 ([0, 6, 7, 8] ends)
    11, 0, 0, 0, // "b" byte boundary at 11 ([9] ends)
    0, 0, 0, 0, // alignment padding to 8 bytes before "f"
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
            data_offset: 16,
            topology_offset: 8,
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
    run_list_wrapped_roundtrip_u32(
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

pub const GROUP_4_MIXED: [u8; 259] = [
    72, 0, 0, 0, // [0..4] group topology start pointer
    12, 0, 0, 0, // [4..8] group topology word count
    227, 0, 0, 0, // [8..12] "a" payload start pointer
    6, 0, 0, 0, // [12..16] "a" sample count
    200, 0, 0, 0, // [16..20] "b" payload start pointer
    4, 0, 0, 0, // [20..24] "b" valid sample count
    6, 0, 0, 0, // [24..28] "b" total sample count (nulls included)
    120, 0, 0, 0, // [28..32] "c" byte-boundary start pointer
    7, 0, 0, 0, // [32..36] "c" byte-boundary word count
    228, 0, 0, 0, // [36..40] "c" payload start pointer
    13, 0, 0, 0, // [40..44] "c" payload byte count
    152, 0, 0, 0, // [44..48] "d" payload start pointer
    6, 0, 0, 0, // [48..52] "d" sample count
    218, 0, 0, 0, // [52..56] "e" payload start pointer
    4, 0, 0, 0, // [56..60] "e" valid sample count
    6, 0, 0, 0, // [60..64] "e" total sample count (nulls included)
    241, 0, 0, 0, // [64..68] "f" payload start pointer
    6, 0, 0, 0, // [68..72] "f" array count
    255, 255, 255, 255, // [72..76] group level 0: sentinel
    2, 0, 0, 0, // [76..80] group level 0: boundary at 2
    3, 0, 0, 0, // [80..84] group level 0: boundary at 3
    255, 255, 255, 255, // [84..88] group level 1: sentinel
    2, 0, 0, 0, // [88..92] group level 1: boundary at 2
    3, 0, 0, 0, // [92..96] group level 1: boundary at 3
    4, 0, 0, 0, // [96..100] group level 1: boundary at 4
    255, 255, 255, 255, // [100..104] group level 2: sentinel
    2, 0, 0, 0, // [104..108] group level 2: boundary at 2
    3, 0, 0, 0, // [108..112] group level 2: boundary at 3
    4, 0, 0, 0, // [112..116] group level 2: boundary at 4
    6, 0, 0, 0, // [116..120] group level 2: boundary at 6
    255, 255, 255, 255, // [120..124] "c" byte-boundary level 0: sentinel
    2, 0, 0, 0, // [124..128] "c" byte-boundary level 0: boundary at 2
    2, 0, 0, 0, // [128..132] "c" byte-boundary level 0: boundary at 2
    5, 0, 0, 0, // [132..136] "c" byte-boundary level 0: boundary at 5
    7, 0, 0, 0, // [136..140] "c" byte-boundary level 0: boundary at 7
    12, 0, 0, 0, // [140..144] "c" byte-boundary level 0: boundary at 12
    13, 0, 0, 0, // [144..148] "c" byte-boundary level 0: boundary at 13
    0, 0, 0, 0, // [148..152] alignment padding (4 bytes)
    1, 0, 0, 0, 0, 0, 0, 0, // [152..160] "d"[0] = 1
    3, 0, 0, 0, 0, 1, 0, 0, // [160..168] "d"[1] = 1099511627779
    3, 0, 0, 0, 0, 0, 0, 0, // [168..176] "d"[2] = 3
    4, 0, 0, 0, 0, 0, 0, 0, // [176..184] "d"[3] = 4
    255, 255, 255, 255, 255, 255, 255, 255, // [184..192] "d"[4] = 18446744073709551615
    6, 0, 0, 0, 0, 0, 0, 0, // [192..200] "d"[5] = 6
    0, 0, 192, 63, // [200..204] "b" valid[0] = 1.5
    0, 0, 16, 192, // [204..208] "b" valid[1] = -2.25
    0, 0, 0, 65, // [208..212] "b" valid[2] = 8.0
    0, 0, 0, 63, // [212..216] "b" valid[3] = 0.5
    45, // [216..217] "b" validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
    0,  // [217..218] alignment padding (1 byte)
    254, 255, // [218..220] "e" valid[0] = -2
    3, 0, // [220..222] "e" valid[1] = 3
    251, 255, // [222..224] "e" valid[2] = -5
    6, 0,  // [224..226] "e" valid[3] = 6
    54, // [226..227] "e" validity mask bits 0..6 (LSB first) = 0, 1, 1, 0, 1, 1
    25, // [227..228] "a" payload bits 0..6 (LSB first) = 1, 0, 0, 1, 1, 0
    97, 98, // [228..230] "c" payload "ab"
    99, 100, 101, // [230..233] "c" payload "cde"
    195, 169, // [233..235] "c" payload "é"
    102, 103, 104, 105, 106, // [235..240] "c" payload "fghij"
    107, // [240..241] "c" payload "k"
    1, 2, 3, // [241..244] "f"[0] = [1, 2, 3]
    4, 5, 6, // [244..247] "f"[1] = [4, 5, 6]
    7, 8, 9, // [247..250] "f"[2] = [7, 8, 9]
    10, 11, 12, // [250..253] "f"[3] = [10, 11, 12]
    13, 14, 15, // [253..256] "f"[4] = [13, 14, 15]
    16, 17, 18, // [256..259] "f"[5] = [16, 17, 18]
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
    run_list_wrapped_roundtrip_u32(
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

pub const GROUP_4_NESTED: [u8; 302] = [
    60, 0, 0, 0, // [0..4] group topology start pointer
    12, 0, 0, 0, // [4..8] group topology word count
    40, 1, 0, 0, // [8..12] "a" payload start pointer
    6, 0, 0, 0, // [12..16] "a" sample count
    108, 0, 0, 0, // [16..20] "b" topology start pointer
    16, 0, 0, 0, // [20..24] "b" topology word count
    200, 0, 0, 0, // [24..28] "b" payload start pointer
    8, 0, 0, 0, // [28..32] "b" sample count
    172, 0, 0, 0, // [32..36] "c" topology start pointer
    7, 0, 0, 0, // [36..40] "c" topology word count
    26, 1, 0, 0, // [40..44] "c" payload start pointer
    7, 0, 0, 0, // [44..48] "c" sample count
    8, 1, 0, 0, // [48..52] "d" payload start pointer
    4, 0, 0, 0, // [52..56] "d" valid sample count
    6, 0, 0, 0, // [56..60] "d" total sample count (nulls included)
    255, 255, 255, 255, // [60..64] group level 0: sentinel
    2, 0, 0, 0, // [64..68] group level 0: boundary at 2
    3, 0, 0, 0, // [68..72] group level 0: boundary at 3
    255, 255, 255, 255, // [72..76] group level 1: sentinel
    2, 0, 0, 0, // [76..80] group level 1: boundary at 2
    3, 0, 0, 0, // [80..84] group level 1: boundary at 3
    4, 0, 0, 0, // [84..88] group level 1: boundary at 4
    255, 255, 255, 255, // [88..92] group level 2: sentinel
    2, 0, 0, 0, // [92..96] group level 2: boundary at 2
    3, 0, 0, 0, // [96..100] group level 2: boundary at 3
    4, 0, 0, 0, // [100..104] group level 2: boundary at 4
    6, 0, 0, 0, // [104..108] group level 2: boundary at 6
    255, 255, 255, 255, // [108..112] "b" topology level 0: sentinel
    2, 0, 0, 0, // [112..116] "b" topology level 0: boundary at 2
    2, 0, 0, 0, // [116..120] "b" topology level 0: boundary at 2
    3, 0, 0, 0, // [120..124] "b" topology level 0: boundary at 3
    4, 0, 0, 0, // [124..128] "b" topology level 0: boundary at 4
    7, 0, 0, 0, // [128..132] "b" topology level 0: boundary at 7
    8, 0, 0, 0, // [132..136] "b" topology level 0: boundary at 8
    255, 255, 255, 255, // [136..140] "b" topology level 1: sentinel
    1, 0, 0, 0, // [140..144] "b" topology level 1: boundary at 1
    3, 0, 0, 0, // [144..148] "b" topology level 1: boundary at 3
    3, 0, 0, 0, // [148..152] "b" topology level 1: boundary at 3
    4, 0, 0, 0, // [152..156] "b" topology level 1: boundary at 4
    6, 0, 0, 0, // [156..160] "b" topology level 1: boundary at 6
    6, 0, 0, 0, // [160..164] "b" topology level 1: boundary at 6
    7, 0, 0, 0, // [164..168] "b" topology level 1: boundary at 7
    8, 0, 0, 0, // [168..172] "b" topology level 1: boundary at 8
    255, 255, 255, 255, // [172..176] "c" topology level 0: sentinel
    3, 0, 0, 0, // [176..180] "c" topology level 0: boundary at 3
    3, 0, 0, 0, // [180..184] "c" topology level 0: boundary at 3
    4, 0, 0, 0, // [184..188] "c" topology level 0: boundary at 4
    6, 0, 0, 0, // [188..192] "c" topology level 0: boundary at 6
    6, 0, 0, 0, // [192..196] "c" topology level 0: boundary at 6
    7, 0, 0, 0, // [196..200] "c" topology level 0: boundary at 7
    1, 0, 0, 0, 0, 0, 0, 0, // [200..208] "b"[0] = 1
    2, 0, 0, 0, 0, 0, 0, 0, // [208..216] "b"[1] = 2
    3, 0, 0, 0, 0, 0, 0, 0, // [216..224] "b"[2] = 3
    4, 0, 0, 0, 0, 0, 0, 0, // [224..232] "b"[3] = 4
    5, 0, 0, 0, 0, 0, 0, 0, // [232..240] "b"[4] = 5
    6, 0, 0, 0, 0, 0, 0, 0, // [240..248] "b"[5] = 6
    7, 0, 0, 0, 0, 0, 0, 0, // [248..256] "b"[6] = 7
    8, 0, 0, 0, 0, 0, 0, 0, // [256..264] "b"[7] = 8
    255, 255, 255, 255, // [264..268] "d" valid[0] = -1
    3, 0, 0, 0, // [268..272] "d" valid[1] = 3
    252, 255, 255, 255, // [272..276] "d" valid[2] = -4
    6, 0, 0, 0,  // [276..280] "d" valid[3] = 6
    45, // [280..281] "d" validity mask bits 0..6 (LSB first) = 1, 0, 1, 1, 0, 1
    0,  // [281..282] alignment padding (1 byte)
    1, 0, // [282..284] "c"[0] = 1
    2, 0, // [284..286] "c"[1] = 2
    3, 0, // [286..288] "c"[2] = 3
    4, 0, // [288..290] "c"[3] = 4
    5, 0, // [290..292] "c"[4] = 5
    6, 0, // [292..294] "c"[5] = 6
    7, 0, // [294..296] "c"[6] = 7
    1, // [296..297] "a"[0] = 1
    2, // [297..298] "a"[1] = 2
    3, // [298..299] "a"[2] = 3
    4, // [299..300] "a"[3] = 4
    5, // [300..301] "a"[4] = 5
    6, // [301..302] "a"[5] = 6
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
    run_list_wrapped_roundtrip_u32(
        schema,
        group_4_value(group_4_nested_entry),
        &GROUP_4_NESTED,
        &GROUP_4_NESTED_INODES,
    );
}
