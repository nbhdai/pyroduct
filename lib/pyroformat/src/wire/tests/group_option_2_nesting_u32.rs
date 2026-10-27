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
use crate::{PyroValue, wire::tests::support::run_list_wrapped_roundtrip_u32};

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

pub const GROUP_OPTION_2_PRIMITIVE: [u8; 108] = [
    44, 0, 0, 0, // group slot: topology start pointer
    4, 0, 0, 0, // group slot: topology word count (1 level, sentinels included)
    96, 0, 0, 0, // "a" payload start pointer
    4, 0, 0, 0, // "a" sample count
    104, 0, 0, 0, // "b" payload start pointer
    4, 0, 0, 0, // "b" sample count
    64, 0, 0, 0, // "c" payload start pointer
    4, 0, 0, 0, // "c" sample count
    105, 0, 0, 0, // "d" payload start pointer
    2, 0, 0, 0, // "d" valid sample count
    4, 0, 0, 0, // "d" total sample count (nulls included)
    255, 255, 255, 255, // group level 0: sentinel
    2, 0, 0, 0, // group level 0: boundary at 2 groups (absent included)
    3, 0, 0, 0, // group level 0: boundary at 3 groups (absent included)
    6, 0, 0, 0, // group level 0: boundary at 6 groups (absent included)
    27, 0, 0, 0, // group validity mask (u32 words, LSB first) = 1, 1, 0, 1, 1, 0
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
            data_offset: 8,
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
            data_offset: 16,
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
            data_offset: 24,
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
    run_list_wrapped_roundtrip_u32(
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

pub const GROUP_OPTION_2_BLOB: [u8; 71] = [
    24, 0, 0, 0, // group slot: topology start pointer
    4, 0, 0, 0, // group slot: topology word count (1 level, sentinels included)
    44, 0, 0, 0, // "b" byte-boundary start pointer
    5, 0, 0, 0, // "b" byte-boundary word count
    64, 0, 0, 0, // "b" payload start pointer
    7, 0, 0, 0, // "b" payload byte count
    255, 255, 255, 255, // group level 0: sentinel
    2, 0, 0, 0, // group level 0: boundary at 2 groups (absent included)
    3, 0, 0, 0, // group level 0: boundary at 3 groups (absent included)
    6, 0, 0, 0, // group level 0: boundary at 6 groups (absent included)
    27, 0, 0, 0, // group validity mask (u32 words, LSB first) = 1, 1, 0, 1, 1, 0
    255, 255, 255, 255, // "b" byte boundaries: sentinel
    2, 0, 0, 0, // "b" byte boundary at 2 ([1, 2] ends)
    2, 0, 0, 0, // "b" byte boundary at 2 ([] ends)
    3, 0, 0, 0, // "b" byte boundary at 3 ([255] ends)
    7, 0, 0, 0, // "b" byte boundary at 7 ([0, 6, 7, 8] ends)
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
            data_offset: 16,
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
fn test_group_option_2_blob() {
    let schema = group_option_2_schema(vec![field("b", PyroType::Bytes)]);
    run_list_wrapped_roundtrip_u32(
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

pub const GROUP_OPTION_2_MIXED: [u8; 181] = [
    72, 0, 0, 0, // [0..4] group topology start pointer
    4, 0, 0, 0, // [4..8] group topology word count
    159, 0, 0, 0, // [8..12] "a" payload start pointer
    4, 0, 0, 0, // [12..16] "a" sample count
    144, 0, 0, 0, // [16..20] "b" payload start pointer
    2, 0, 0, 0, // [20..24] "b" valid sample count
    4, 0, 0, 0, // [24..28] "b" total sample count (nulls included)
    92, 0, 0, 0, // [28..32] "c" byte-boundary start pointer
    5, 0, 0, 0, // [32..36] "c" byte-boundary word count
    160, 0, 0, 0, // [36..40] "c" payload start pointer
    9, 0, 0, 0, // [40..44] "c" payload byte count
    112, 0, 0, 0, // [44..48] "d" payload start pointer
    4, 0, 0, 0, // [48..52] "d" sample count
    154, 0, 0, 0, // [52..56] "e" payload start pointer
    2, 0, 0, 0, // [56..60] "e" valid sample count
    4, 0, 0, 0, // [60..64] "e" total sample count (nulls included)
    169, 0, 0, 0, // [64..68] "f" payload start pointer
    4, 0, 0, 0, // [68..72] "f" array count
    255, 255, 255, 255, // [72..76] group level 0: sentinel
    2, 0, 0, 0, // [76..80] group level 0: boundary at 2
    3, 0, 0, 0, // [80..84] group level 0: boundary at 3
    6, 0, 0, 0, // [84..88] group level 0: boundary at 6
    27, 0, 0, 0, // [88..92] group validity mask (u32 word 0, LSB first) = 1, 1, 0, 1, 1, 0
    255, 255, 255, 255, // [92..96] "c" byte-boundary level 0: sentinel
    2, 0, 0, 0, // [96..100] "c" byte-boundary level 0: boundary at 2
    2, 0, 0, 0, // [100..104] "c" byte-boundary level 0: boundary at 2
    4, 0, 0, 0, // [104..108] "c" byte-boundary level 0: boundary at 4
    9, 0, 0, 0, // [108..112] "c" byte-boundary level 0: boundary at 9
    1, 0, 0, 0, 0, 0, 0, 0, // [112..120] "d"[0] = 1
    3, 0, 0, 0, 0, 1, 0, 0, // [120..128] "d"[1] = 1099511627779
    4, 0, 0, 0, 0, 0, 0, 0, // [128..136] "d"[2] = 4
    255, 255, 255, 255, 255, 255, 255, 255, // [136..144] "d"[3] = 18446744073709551615
    0, 0, 192, 63, // [144..148] "b" valid[0] = 1.5
    0, 0, 0, 65, // [148..152] "b" valid[1] = 8.0
    5,  // [152..153] "b" validity mask bits 0..4 (LSB first) = 1, 0, 1, 0
    0,  // [153..154] alignment padding (1 byte)
    254, 255, // [154..156] "e" valid[0] = -2
    251, 255, // [156..158] "e" valid[1] = -5
    10,  // [158..159] "e" validity mask bits 0..4 (LSB first) = 0, 1, 0, 1
    13,  // [159..160] "a" payload bits 0..4 (LSB first) = 1, 0, 1, 1
    97, 98, // [160..162] "c" payload "ab"
    195, 169, // [162..164] "c" payload "é"
    102, 103, 104, 105, 106, // [164..169] "c" payload "fghij"
    1, 2, 3, // [169..172] "f"[0] = [1, 2, 3]
    4, 5, 6, // [172..175] "f"[1] = [4, 5, 6]
    10, 11, 12, // [175..178] "f"[2] = [10, 11, 12]
    13, 14, 15, // [178..181] "f"[3] = [13, 14, 15]
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
    run_list_wrapped_roundtrip_u32(
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

pub const GROUP_OPTION_2_NESTED: [u8; 232] = [
    60, 0, 0, 0, // [0..4] group topology start pointer
    4, 0, 0, 0, // [4..8] group topology word count
    228, 0, 0, 0, // [8..12] "a" payload start pointer
    4, 0, 0, 0, // [12..16] "a" sample count
    80, 0, 0, 0, // [16..20] "b" topology start pointer
    12, 0, 0, 0, // [20..24] "b" topology word count
    152, 0, 0, 0, // [24..28] "b" payload start pointer
    7, 0, 0, 0, // [28..32] "b" sample count
    128, 0, 0, 0, // [32..36] "c" topology start pointer
    5, 0, 0, 0, // [36..40] "c" topology word count
    218, 0, 0, 0, // [40..44] "c" payload start pointer
    5, 0, 0, 0, // [44..48] "c" sample count
    208, 0, 0, 0, // [48..52] "d" payload start pointer
    2, 0, 0, 0, // [52..56] "d" valid sample count
    4, 0, 0, 0, // [56..60] "d" total sample count (nulls included)
    255, 255, 255, 255, // [60..64] group level 0: sentinel
    2, 0, 0, 0, // [64..68] group level 0: boundary at 2
    3, 0, 0, 0, // [68..72] group level 0: boundary at 3
    6, 0, 0, 0, // [72..76] group level 0: boundary at 6
    27, 0, 0, 0, // [76..80] group validity mask (u32 word 0, LSB first) = 1, 1, 0, 1, 1, 0
    255, 255, 255, 255, // [80..84] "b" topology level 0: sentinel
    2, 0, 0, 0, // [84..88] "b" topology level 0: boundary at 2
    2, 0, 0, 0, // [88..92] "b" topology level 0: boundary at 2
    3, 0, 0, 0, // [92..96] "b" topology level 0: boundary at 3
    6, 0, 0, 0, // [96..100] "b" topology level 0: boundary at 6
    255, 255, 255, 255, // [100..104] "b" topology level 1: sentinel
    1, 0, 0, 0, // [104..108] "b" topology level 1: boundary at 1
    3, 0, 0, 0, // [108..112] "b" topology level 1: boundary at 3
    4, 0, 0, 0, // [112..116] "b" topology level 1: boundary at 4
    6, 0, 0, 0, // [116..120] "b" topology level 1: boundary at 6
    6, 0, 0, 0, // [120..124] "b" topology level 1: boundary at 6
    7, 0, 0, 0, // [124..128] "b" topology level 1: boundary at 7
    255, 255, 255, 255, // [128..132] "c" topology level 0: sentinel
    3, 0, 0, 0, // [132..136] "c" topology level 0: boundary at 3
    3, 0, 0, 0, // [136..140] "c" topology level 0: boundary at 3
    5, 0, 0, 0, // [140..144] "c" topology level 0: boundary at 5
    5, 0, 0, 0, // [144..148] "c" topology level 0: boundary at 5
    0, 0, 0, 0, // [148..152] alignment padding (4 bytes)
    1, 0, 0, 0, 0, 0, 0, 0, // [152..160] "b"[0] = 1
    2, 0, 0, 0, 0, 0, 0, 0, // [160..168] "b"[1] = 2
    3, 0, 0, 0, 0, 0, 0, 0, // [168..176] "b"[2] = 3
    4, 0, 0, 0, 0, 0, 0, 0, // [176..184] "b"[3] = 4
    5, 0, 0, 0, 0, 0, 0, 0, // [184..192] "b"[4] = 5
    6, 0, 0, 0, 0, 0, 0, 0, // [192..200] "b"[5] = 6
    7, 0, 0, 0, 0, 0, 0, 0, // [200..208] "b"[6] = 7
    255, 255, 255, 255, // [208..212] "d" valid[0] = -1
    252, 255, 255, 255, // [212..216] "d" valid[1] = -4
    5,   // [216..217] "d" validity mask bits 0..4 (LSB first) = 1, 0, 1, 0
    0,   // [217..218] alignment padding (1 byte)
    1, 0, // [218..220] "c"[0] = 1
    2, 0, // [220..222] "c"[1] = 2
    3, 0, // [222..224] "c"[2] = 3
    5, 0, // [224..226] "c"[3] = 5
    6, 0, // [226..228] "c"[4] = 6
    1, // [228..229] "a"[0] = 1
    2, // [229..230] "a"[1] = 2
    4, // [230..231] "a"[2] = 4
    5, // [231..232] "a"[3] = 5
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
    run_list_wrapped_roundtrip_u32(
        schema,
        group_option_2_value(group_option_2_nested_entry),
        &GROUP_OPTION_2_NESTED,
        &GROUP_OPTION_2_NESTED_INODES,
    );
}
