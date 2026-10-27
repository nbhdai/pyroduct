//! `List<List<List<List<List<Option<Group<..>>>>>>>` with two different groups:
//! * `primitive`: `Option<Group{a: bool, b: bool, c: i32, d: Option<bool>}>`
//! * `blob`: `Option<Group{s: String, t: Vec<u8>}>`
//!
//! Shared value shape: `[[[[[a, b]], [[c]]], [[[d]]]], [[[[e, f]]]]]`, each letter one group `0..6`;
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
fn group_option_5_schema(fields: Vec<PyroField>) -> PyroSchema {
    let mut ty = PyroType::OptionGroup { fields };
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
fn group_option_5_value(entry: fn(usize) -> PyroValue<'static>) -> PyroValue<'static> {
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
// primitive: Option<Group{a: bool, b: bool, c: i32, d: Option<bool>}>
// ---------------------------------------------------------------------------

/// Group `i` of `primitive` (absent for 2 and 5).
#[cfg(test)]
fn group_option_5_primitive_entry(i: usize) -> PyroValue<'static> {
    if [2, 5].contains(&i) {
        return PyroValue::Null;
    }
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

pub const GROUP_OPTION_5_PRIMITIVE: [u8; 136] = [
    44, 0, 0, 0, // group slot: topology start pointer
    17, 0, 0, 0, // group slot: topology word count (4 levels, sentinels included)
    132, 0, 0, 0, // "a" payload start pointer
    4, 0, 0, 0, // "a" sample count
    133, 0, 0, 0, // "b" payload start pointer
    4, 0, 0, 0, // "b" sample count
    116, 0, 0, 0, // "c" payload start pointer
    4, 0, 0, 0, // "c" sample count
    134, 0, 0, 0, // "d" payload start pointer
    2, 0, 0, 0, // "d" valid sample count
    4, 0, 0, 0, // "d" total sample count (nulls included)
    255, 255, 255, 255, // group level 0: sentinel
    2, 0, 0, 0, // group level 0: boundary at 2 lists
    3, 0, 0, 0, // group level 0: boundary at 3 lists
    255, 255, 255, 255, // group level 1: sentinel
    2, 0, 0, 0, // group level 1: boundary at 2 lists
    3, 0, 0, 0, // group level 1: boundary at 3 lists
    4, 0, 0, 0, // group level 1: boundary at 4 lists
    255, 255, 255, 255, // group level 2: sentinel
    1, 0, 0, 0, // group level 2: boundary at 1 lists
    2, 0, 0, 0, // group level 2: boundary at 2 lists
    3, 0, 0, 0, // group level 2: boundary at 3 lists
    4, 0, 0, 0, // group level 2: boundary at 4 lists
    255, 255, 255, 255, // group level 3: sentinel
    2, 0, 0, 0, // group level 3: boundary at 2 groups (absent included)
    3, 0, 0, 0, // group level 3: boundary at 3 groups (absent included)
    4, 0, 0, 0, // group level 3: boundary at 4 groups (absent included)
    6, 0, 0, 0, // group level 3: boundary at 6 groups (absent included)
    27, 0, 0, 0, // group validity mask (u32 words, LSB first) = 1, 1, 0, 1, 1, 0
    255, 255, 255, 255, // "c"[group 0] = -1
    2, 0, 0, 0, // "c"[group 1] = 2
    4, 0, 0, 0, // "c"[group 3] = 4
    251, 255, 255, 255, // "c"[group 4] = -5
    13,  // "a" payload bits (LSB first) = 1, 0, 1, 1
    13,  // "b" payload bits (LSB first) = 1, 0, 1, 1
    3,   // "d" valid payload bits (LSB first) = 1, 1
    5,   // "d" validity mask bits (LSB first) = 1, 0, 1, 0
];
pub const GROUP_OPTION_5_PRIMITIVE_INODES: [Inode; 5] = [
    Inode {
        kind: WireKind::OptionGroup,
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
        kind: WireKind::I32,
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
        kind: WireKind::OptionBool,
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
fn test_group_option_5_primitive() {
    let schema = group_option_5_schema(vec![
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
    run_list_wrapped_roundtrip_u32(
        schema,
        group_option_5_value(group_option_5_primitive_entry),
        &GROUP_OPTION_5_PRIMITIVE,
        &GROUP_OPTION_5_PRIMITIVE_INODES,
    );
}

// ---------------------------------------------------------------------------
// blob: Option<Group{s: String, t: Vec<u8>}>
// ---------------------------------------------------------------------------

/// Group `i` of `blob` (absent for 2 and 5).
#[cfg(test)]
fn group_option_5_blob_entry(i: usize) -> PyroValue<'static> {
    if [2, 5].contains(&i) {
        return PyroValue::Null;
    }
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

pub const GROUP_OPTION_5_BLOB: [u8; 166] = [
    40, 0, 0, 0, // group slot: topology start pointer
    17, 0, 0, 0, // group slot: topology word count (4 levels, sentinels included)
    112, 0, 0, 0, // "s" byte-boundary start pointer
    5, 0, 0, 0, // "s" byte-boundary word count
    152, 0, 0, 0, // "s" payload start pointer
    7, 0, 0, 0, // "s" payload byte count
    132, 0, 0, 0, // "t" byte-boundary start pointer
    5, 0, 0, 0, // "t" byte-boundary word count
    159, 0, 0, 0, // "t" payload start pointer
    7, 0, 0, 0, // "t" payload byte count
    255, 255, 255, 255, // group level 0: sentinel
    2, 0, 0, 0, // group level 0: boundary at 2 lists
    3, 0, 0, 0, // group level 0: boundary at 3 lists
    255, 255, 255, 255, // group level 1: sentinel
    2, 0, 0, 0, // group level 1: boundary at 2 lists
    3, 0, 0, 0, // group level 1: boundary at 3 lists
    4, 0, 0, 0, // group level 1: boundary at 4 lists
    255, 255, 255, 255, // group level 2: sentinel
    1, 0, 0, 0, // group level 2: boundary at 1 lists
    2, 0, 0, 0, // group level 2: boundary at 2 lists
    3, 0, 0, 0, // group level 2: boundary at 3 lists
    4, 0, 0, 0, // group level 2: boundary at 4 lists
    255, 255, 255, 255, // group level 3: sentinel
    2, 0, 0, 0, // group level 3: boundary at 2 groups (absent included)
    3, 0, 0, 0, // group level 3: boundary at 3 groups (absent included)
    4, 0, 0, 0, // group level 3: boundary at 4 groups (absent included)
    6, 0, 0, 0, // group level 3: boundary at 6 groups (absent included)
    27, 0, 0, 0, // group validity mask (u32 words, LSB first) = 1, 1, 0, 1, 1, 0
    255, 255, 255, 255, // "s" byte boundaries: sentinel
    2, 0, 0, 0, // "s" byte boundary at 2 ("ab" ends)
    2, 0, 0, 0, // "s" byte boundary at 2 ("" ends)
    3, 0, 0, 0, // "s" byte boundary at 3 ("f" ends)
    7, 0, 0, 0, // "s" byte boundary at 7 ("ghij" ends)
    255, 255, 255, 255, // "t" byte boundaries: sentinel
    2, 0, 0, 0, // "t" byte boundary at 2 ([1, 2] ends)
    2, 0, 0, 0, // "t" byte boundary at 2 ([] ends)
    3, 0, 0, 0, // "t" byte boundary at 3 ([255] ends)
    7, 0, 0, 0, // "t" byte boundary at 7 ([0, 6, 7, 8] ends)
    97, 98,  // "s" payload "ab"
    102, // "s" payload "f"
    103, 104, 105, 106, // "s" payload "ghij"
    1, 2,   // "t" payload [1, 2]
    255, // "t" payload [255]
    0, 6, 7, 8, // "t" payload [0, 6, 7, 8]
];
pub const GROUP_OPTION_5_BLOB_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::OptionGroup,
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
            data_offset: 16,
            topology_offset: 8,
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
            data_offset: 32,
            topology_offset: 24,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_group_option_5_blob() {
    let schema =
        group_option_5_schema(vec![field("s", PyroType::Str), field("t", PyroType::Bytes)]);
    run_list_wrapped_roundtrip_u32(
        schema,
        group_option_5_value(group_option_5_blob_entry),
        &GROUP_OPTION_5_BLOB,
        &GROUP_OPTION_5_BLOB_INODES,
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
fn group_option_5_mixed_entry(i: usize) -> PyroValue<'static> {
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

pub const GROUP_OPTION_5_MIXED: [u8; 237] = [
    72, 0, 0, 0, // [0..4] group topology start pointer
    17, 0, 0, 0, // [4..8] group topology word count
    215, 0, 0, 0, // [8..12] "a" payload start pointer
    4, 0, 0, 0, // [12..16] "a" sample count
    200, 0, 0, 0, // [16..20] "b" payload start pointer
    2, 0, 0, 0, // [20..24] "b" valid sample count
    4, 0, 0, 0, // [24..28] "b" total sample count (nulls included)
    144, 0, 0, 0, // [28..32] "c" byte-boundary start pointer
    5, 0, 0, 0, // [32..36] "c" byte-boundary word count
    216, 0, 0, 0, // [36..40] "c" payload start pointer
    9, 0, 0, 0, // [40..44] "c" payload byte count
    168, 0, 0, 0, // [44..48] "d" payload start pointer
    4, 0, 0, 0, // [48..52] "d" sample count
    210, 0, 0, 0, // [52..56] "e" payload start pointer
    2, 0, 0, 0, // [56..60] "e" valid sample count
    4, 0, 0, 0, // [60..64] "e" total sample count (nulls included)
    225, 0, 0, 0, // [64..68] "f" payload start pointer
    4, 0, 0, 0, // [68..72] "f" array count
    255, 255, 255, 255, // [72..76] group level 0: sentinel
    2, 0, 0, 0, // [76..80] group level 0: boundary at 2
    3, 0, 0, 0, // [80..84] group level 0: boundary at 3
    255, 255, 255, 255, // [84..88] group level 1: sentinel
    2, 0, 0, 0, // [88..92] group level 1: boundary at 2
    3, 0, 0, 0, // [92..96] group level 1: boundary at 3
    4, 0, 0, 0, // [96..100] group level 1: boundary at 4
    255, 255, 255, 255, // [100..104] group level 2: sentinel
    1, 0, 0, 0, // [104..108] group level 2: boundary at 1
    2, 0, 0, 0, // [108..112] group level 2: boundary at 2
    3, 0, 0, 0, // [112..116] group level 2: boundary at 3
    4, 0, 0, 0, // [116..120] group level 2: boundary at 4
    255, 255, 255, 255, // [120..124] group level 3: sentinel
    2, 0, 0, 0, // [124..128] group level 3: boundary at 2
    3, 0, 0, 0, // [128..132] group level 3: boundary at 3
    4, 0, 0, 0, // [132..136] group level 3: boundary at 4
    6, 0, 0, 0, // [136..140] group level 3: boundary at 6
    27, 0, 0, 0, // [140..144] group validity mask (u32 word 0, LSB first) = 1, 1, 0, 1, 1, 0
    255, 255, 255, 255, // [144..148] "c" byte-boundary level 0: sentinel
    2, 0, 0, 0, // [148..152] "c" byte-boundary level 0: boundary at 2
    2, 0, 0, 0, // [152..156] "c" byte-boundary level 0: boundary at 2
    4, 0, 0, 0, // [156..160] "c" byte-boundary level 0: boundary at 4
    9, 0, 0, 0, // [160..164] "c" byte-boundary level 0: boundary at 9
    0, 0, 0, 0, // [164..168] alignment padding (4 bytes)
    1, 0, 0, 0, 0, 0, 0, 0, // [168..176] "d"[0] = 1
    3, 0, 0, 0, 0, 1, 0, 0, // [176..184] "d"[1] = 1099511627779
    4, 0, 0, 0, 0, 0, 0, 0, // [184..192] "d"[2] = 4
    255, 255, 255, 255, 255, 255, 255, 255, // [192..200] "d"[3] = 18446744073709551615
    0, 0, 192, 63, // [200..204] "b" valid[0] = 1.5
    0, 0, 0, 65, // [204..208] "b" valid[1] = 8.0
    5,  // [208..209] "b" validity mask bits 0..4 (LSB first) = 1, 0, 1, 0
    0,  // [209..210] alignment padding (1 byte)
    254, 255, // [210..212] "e" valid[0] = -2
    251, 255, // [212..214] "e" valid[1] = -5
    10,  // [214..215] "e" validity mask bits 0..4 (LSB first) = 0, 1, 0, 1
    13,  // [215..216] "a" payload bits 0..4 (LSB first) = 1, 0, 1, 1
    97, 98, // [216..218] "c" payload "ab"
    195, 169, // [218..220] "c" payload "é"
    102, 103, 104, 105, 106, // [220..225] "c" payload "fghij"
    1, 2, 3, // [225..228] "f"[0] = [1, 2, 3]
    4, 5, 6, // [228..231] "f"[1] = [4, 5, 6]
    10, 11, 12, // [231..234] "f"[2] = [10, 11, 12]
    13, 14, 15, // [234..237] "f"[3] = [13, 14, 15]
];
pub const GROUP_OPTION_5_MIXED_INODES: [Inode; 7] = [
    Inode {
        kind: WireKind::OptionGroup,
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
fn test_group_option_5_mixed() {
    let schema = group_option_5_schema(vec![
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
        group_option_5_value(group_option_5_mixed_entry),
        &GROUP_OPTION_5_MIXED,
        &GROUP_OPTION_5_MIXED_INODES,
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
fn group_option_5_nested_entry(i: usize) -> PyroValue<'static> {
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

pub const GROUP_OPTION_5_NESTED: [u8; 280] = [
    60, 0, 0, 0, // [0..4] group topology start pointer
    17, 0, 0, 0, // [4..8] group topology word count
    20, 1, 0, 0, // [8..12] "a" payload start pointer
    4, 0, 0, 0, // [12..16] "a" sample count
    132, 0, 0, 0, // [16..20] "b" topology start pointer
    12, 0, 0, 0, // [20..24] "b" topology word count
    200, 0, 0, 0, // [24..28] "b" payload start pointer
    7, 0, 0, 0, // [28..32] "b" sample count
    180, 0, 0, 0, // [32..36] "c" topology start pointer
    5, 0, 0, 0, // [36..40] "c" topology word count
    10, 1, 0, 0, // [40..44] "c" payload start pointer
    5, 0, 0, 0, // [44..48] "c" sample count
    0, 1, 0, 0, // [48..52] "d" payload start pointer
    2, 0, 0, 0, // [52..56] "d" valid sample count
    4, 0, 0, 0, // [56..60] "d" total sample count (nulls included)
    255, 255, 255, 255, // [60..64] group level 0: sentinel
    2, 0, 0, 0, // [64..68] group level 0: boundary at 2
    3, 0, 0, 0, // [68..72] group level 0: boundary at 3
    255, 255, 255, 255, // [72..76] group level 1: sentinel
    2, 0, 0, 0, // [76..80] group level 1: boundary at 2
    3, 0, 0, 0, // [80..84] group level 1: boundary at 3
    4, 0, 0, 0, // [84..88] group level 1: boundary at 4
    255, 255, 255, 255, // [88..92] group level 2: sentinel
    1, 0, 0, 0, // [92..96] group level 2: boundary at 1
    2, 0, 0, 0, // [96..100] group level 2: boundary at 2
    3, 0, 0, 0, // [100..104] group level 2: boundary at 3
    4, 0, 0, 0, // [104..108] group level 2: boundary at 4
    255, 255, 255, 255, // [108..112] group level 3: sentinel
    2, 0, 0, 0, // [112..116] group level 3: boundary at 2
    3, 0, 0, 0, // [116..120] group level 3: boundary at 3
    4, 0, 0, 0, // [120..124] group level 3: boundary at 4
    6, 0, 0, 0, // [124..128] group level 3: boundary at 6
    27, 0, 0, 0, // [128..132] group validity mask (u32 word 0, LSB first) = 1, 1, 0, 1, 1, 0
    255, 255, 255, 255, // [132..136] "b" topology level 0: sentinel
    2, 0, 0, 0, // [136..140] "b" topology level 0: boundary at 2
    2, 0, 0, 0, // [140..144] "b" topology level 0: boundary at 2
    3, 0, 0, 0, // [144..148] "b" topology level 0: boundary at 3
    6, 0, 0, 0, // [148..152] "b" topology level 0: boundary at 6
    255, 255, 255, 255, // [152..156] "b" topology level 1: sentinel
    1, 0, 0, 0, // [156..160] "b" topology level 1: boundary at 1
    3, 0, 0, 0, // [160..164] "b" topology level 1: boundary at 3
    4, 0, 0, 0, // [164..168] "b" topology level 1: boundary at 4
    6, 0, 0, 0, // [168..172] "b" topology level 1: boundary at 6
    6, 0, 0, 0, // [172..176] "b" topology level 1: boundary at 6
    7, 0, 0, 0, // [176..180] "b" topology level 1: boundary at 7
    255, 255, 255, 255, // [180..184] "c" topology level 0: sentinel
    3, 0, 0, 0, // [184..188] "c" topology level 0: boundary at 3
    3, 0, 0, 0, // [188..192] "c" topology level 0: boundary at 3
    5, 0, 0, 0, // [192..196] "c" topology level 0: boundary at 5
    5, 0, 0, 0, // [196..200] "c" topology level 0: boundary at 5
    1, 0, 0, 0, 0, 0, 0, 0, // [200..208] "b"[0] = 1
    2, 0, 0, 0, 0, 0, 0, 0, // [208..216] "b"[1] = 2
    3, 0, 0, 0, 0, 0, 0, 0, // [216..224] "b"[2] = 3
    4, 0, 0, 0, 0, 0, 0, 0, // [224..232] "b"[3] = 4
    5, 0, 0, 0, 0, 0, 0, 0, // [232..240] "b"[4] = 5
    6, 0, 0, 0, 0, 0, 0, 0, // [240..248] "b"[5] = 6
    7, 0, 0, 0, 0, 0, 0, 0, // [248..256] "b"[6] = 7
    255, 255, 255, 255, // [256..260] "d" valid[0] = -1
    252, 255, 255, 255, // [260..264] "d" valid[1] = -4
    5,   // [264..265] "d" validity mask bits 0..4 (LSB first) = 1, 0, 1, 0
    0,   // [265..266] alignment padding (1 byte)
    1, 0, // [266..268] "c"[0] = 1
    2, 0, // [268..270] "c"[1] = 2
    3, 0, // [270..272] "c"[2] = 3
    5, 0, // [272..274] "c"[3] = 5
    6, 0, // [274..276] "c"[4] = 6
    1, // [276..277] "a"[0] = 1
    2, // [277..278] "a"[1] = 2
    4, // [278..279] "a"[2] = 4
    5, // [279..280] "a"[3] = 5
];
pub const GROUP_OPTION_5_NESTED_INODES: [Inode; 5] = [
    Inode {
        kind: WireKind::OptionGroup,
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
fn test_group_option_5_nested() {
    let schema = group_option_5_schema(vec![
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
        group_option_5_value(group_option_5_nested_entry),
        &GROUP_OPTION_5_NESTED,
        &GROUP_OPTION_5_NESTED_INODES,
    );
}
