// One case per `benches/roundtrip.rs` group, each the bench's own struct
// shape expressed as a hand-written schema. Sweep parameters are cut down to
// the smallest size that keeps the shape (two list elements, two map
// entries, a 2x2x2 cube, 4-byte payloads) so the goldens stay legible;
// fixed-size types (`simple`, `wide`, `fixed_arrays`, ...) are the bench's
// exact values. `tests/value_match/bench_u16.rs` derives the same structs and
// checks the derive writers against these goldens.
//
// Captured from the value writer's output: fixed region labelled by the
// inode offsets, variable region by offset.

#[cfg(test)]
use pyro_schema::{
    PrimitiveDataType,
    human::{PyroField, PyroSchema, PyroType},
};

#[cfg(test)]
use crate::{
    PyroValue,
    value::{PrimitiveValueList, Time},
    wire::tests::support::run_list_wrapped_roundtrip_u16,
};
#[cfg(test)]
use half::f16;

use pyro_schema::{Inode, KindMeta, WireKind};

#[cfg(test)]
fn field(name: &str, data_type: PyroType) -> PyroField {
    PyroField {
        name: name.to_string(),
        doc: String::new(),
        data_type,
    }
}

#[cfg(test)]
fn scalar(primitive: PrimitiveDataType) -> PyroType {
    PyroType::PrimitiveScalar { primitive }
}

#[cfg(test)]
fn option(primitive: PrimitiveDataType) -> PyroType {
    PyroType::OptionPrimitiveScalar { primitive }
}

#[cfg(test)]
fn prim_list(element: PrimitiveDataType) -> PyroType {
    PyroType::List {
        element: Box::new(PyroType::PrimitiveScalar { primitive: element }),
    }
}

#[cfg(test)]
fn list(element: PyroType) -> PyroType {
    PyroType::List {
        element: Box::new(element),
    }
}

#[cfg(test)]
fn group(fields: Vec<PyroField>) -> PyroType {
    PyroType::Group { fields }
}

#[cfg(test)]
fn schema(fields: Vec<PyroField>) -> pyro_schema::human::PyroSchema {
    PyroSchema {
        doc: String::new(),
        fields,
    }
}

#[cfg(test)]
fn row<const N: usize>(fields: [(&'static str, PyroValue<'static>); N]) -> PyroValue<'static> {
    PyroValue::Group(fields.into())
}

#[cfg(test)]
fn str_(s: &str) -> PyroValue<'static> {
    PyroValue::Str(s.to_string().into())
}

// ---------------------------------------------------------------------------
// simple
// ---------------------------------------------------------------------------

#[cfg(test)]
fn simple_schema() -> pyro_schema::human::PyroSchema {
    use PrimitiveDataType::*;
    schema(vec![
        field("id", scalar(U32)),
        field("count", scalar(I32)),
        field("active", scalar(Bool)),
        field("score", scalar(F64)),
    ])
}

#[cfg(test)]
fn simple_value() -> PyroValue<'static> {
    row([
        ("id", PyroValue::U32(42)),
        ("count", PyroValue::I32(-7)),
        ("active", PyroValue::Bool(true)),
        ("score", PyroValue::F64(3.14)),
    ])
}

// ---------------------------------------------------------------------------
// strings
// ---------------------------------------------------------------------------

#[cfg(test)]
fn strings_schema() -> pyro_schema::human::PyroSchema {
    schema(vec![
        field("name", PyroType::Str),
        field("tag", PyroType::Str),
    ])
}

#[cfg(test)]
fn strings_value() -> PyroValue<'static> {
    row([("name", str_("hello world")), ("tag", str_("greeting"))])
}

// ---------------------------------------------------------------------------
// vecs (`WithVecs` and its `WithBitVec` sibling share one schema: both
// `Vec<bool>` and `BitVec` are a packed bool list on the wire)
// ---------------------------------------------------------------------------

#[cfg(test)]
fn vecs_schema() -> pyro_schema::human::PyroSchema {
    use PrimitiveDataType::*;
    schema(vec![
        field("data_u32", prim_list(U32)),
        field("data_i64", prim_list(I64)),
        field("flags", prim_list(Bool)),
    ])
}

#[cfg(test)]
fn vecs_value() -> PyroValue<'static> {
    row([
        ("data_u32", PyroValue::from(vec![0u32, 1, 2])),
        ("data_i64", PyroValue::from(vec![0i64, 1, 2])),
        ("flags", PyroValue::from(vec![true, false, true])),
    ])
}

// ---------------------------------------------------------------------------
// nested
// ---------------------------------------------------------------------------

#[cfg(test)]
fn address_type() -> PyroType {
    group(vec![
        field("street", PyroType::Str),
        field("city", PyroType::Str),
        field("zip", scalar(PrimitiveDataType::U32)),
    ])
}

#[cfg(test)]
fn address_value(street: &str, city: &str, zip: u32) -> PyroValue<'static> {
    row([
        ("street", str_(street)),
        ("city", str_(city)),
        ("zip", PyroValue::U32(zip)),
    ])
}

#[cfg(test)]
fn nested_schema() -> pyro_schema::human::PyroSchema {
    schema(vec![
        field("inner", address_type()),
        field("count", scalar(PrimitiveDataType::U32)),
    ])
}

#[cfg(test)]
fn nested_value() -> PyroValue<'static> {
    row([
        ("inner", address_value("Main St", "Springfield", 12345)),
        ("count", PyroValue::U32(7)),
    ])
}

// ---------------------------------------------------------------------------
// triple_nested_vec
// ---------------------------------------------------------------------------

#[cfg(test)]
fn triple_nested_vec_schema() -> pyro_schema::human::PyroSchema {
    use PrimitiveDataType::*;
    schema(vec![
        field("id", scalar(U32)),
        field("cube", list(list(prim_list(U32)))),
    ])
}

#[cfg(test)]
fn triple_nested_vec_value() -> PyroValue<'static> {
    // `make_cube(2)`.
    let cube = (0..2u32)
        .map(|i| {
            PyroValue::List(
                (0..2u32)
                    .map(|j| PyroValue::from(vec![i * 4 + j * 2, i * 4 + j * 2 + 1]))
                    .collect(),
            )
        })
        .collect();
    row([("id", PyroValue::U32(1)), ("cube", PyroValue::List(cube))])
}

// ---------------------------------------------------------------------------
// nested_vec_struct
// ---------------------------------------------------------------------------

#[cfg(test)]
fn person_fields() -> Vec<PyroField> {
    vec![
        field("name", PyroType::Str),
        field("age", scalar(PrimitiveDataType::I32)),
        field("address", list(address_type())),
    ]
}

/// `make_person(id, num_addresses)`.
#[cfg(test)]
fn person_value(id: u32, num_addresses: u32) -> PyroValue<'static> {
    row([
        ("name", str_(&format!("Person {id}"))),
        ("age", PyroValue::I32(20 + (id % 50) as i32)),
        (
            "address",
            PyroValue::List(
                (0..num_addresses)
                    .map(|i| address_value(&format!("{i} Main St"), "Springfield", 10000 + i))
                    .collect(),
            ),
        ),
    ])
}

#[cfg(test)]
fn nested_vec_struct_schema() -> pyro_schema::human::PyroSchema {
    schema(person_fields())
}

#[cfg(test)]
fn nested_vec_struct_value() -> PyroValue<'static> {
    person_value(0, 2)
}

// ---------------------------------------------------------------------------
// mixed_nested
// ---------------------------------------------------------------------------

#[cfg(test)]
fn mixed_nested_schema() -> pyro_schema::human::PyroSchema {
    use PrimitiveDataType::*;
    schema(vec![
        field("timestamp", scalar(U64)),
        field("value", scalar(F64)),
        field(
            "sensor",
            list(group(vec![
                field("id", scalar(U64)),
                field("label", PyroType::Str),
            ])),
        ),
        field("flags", prim_list(Bool)),
        field("tags", list(PyroType::Str)),
        field("quality", scalar(I32)),
    ])
}

#[cfg(test)]
fn mixed_nested_value() -> PyroValue<'static> {
    row([
        ("timestamp", PyroValue::U64(1_700_000_000)),
        ("value", PyroValue::F64(98.6)),
        (
            "sensor",
            PyroValue::List(
                (0..2u64)
                    .map(|i| {
                        row([
                            ("id", PyroValue::U64(i)),
                            ("label", str_(&format!("sensor-{i}"))),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("flags", PyroValue::from(vec![true, false])),
        ("tags", PyroValue::List(vec![str_("tag-0"), str_("tag-1")])),
        ("quality", PyroValue::I32(-3)),
    ])
}

// ---------------------------------------------------------------------------
// deep_nested_vec_struct
// ---------------------------------------------------------------------------

#[cfg(test)]
fn deep_nested_vec_struct_schema() -> pyro_schema::human::PyroSchema {
    use PrimitiveDataType::*;
    schema(vec![
        field("name", PyroType::Str),
        field("founded_year", scalar(U32)),
        field("employees", list(group(person_fields()))),
        field("revenue", scalar(F64)),
    ])
}

#[cfg(test)]
fn deep_nested_vec_struct_value() -> PyroValue<'static> {
    row([
        ("name", str_("Acme Corp")),
        ("founded_year", PyroValue::U32(1990)),
        (
            "employees",
            PyroValue::List((0..2).map(|i| person_value(i, 2)).collect()),
        ),
        ("revenue", PyroValue::F64(1_234_567.89)),
    ])
}

// ---------------------------------------------------------------------------
// pcap
// ---------------------------------------------------------------------------

#[cfg(test)]
fn pcap_record_fields() -> Vec<PyroField> {
    use PrimitiveDataType::*;
    vec![
        field("ts_secs", scalar(U64)),
        field("ts_micros", scalar(U32)),
        field("src_ip", scalar(U32)),
        field("dst_ip", scalar(U32)),
        field("src_port", scalar(U16)),
        field("dst_port", scalar(U16)),
        field("protocol", scalar(U8)),
        field("payload", PyroType::Bytes),
    ]
}

/// `make_pcap_record(seq, payload_len)`.
#[cfg(test)]
fn pcap_record_value(seq: u64, payload_len: usize) -> PyroValue<'static> {
    row([
        ("ts_secs", PyroValue::U64(1_700_000_000 + seq)),
        ("ts_micros", PyroValue::U32((seq % 1_000_000) as u32)),
        ("src_ip", PyroValue::U32(0xC0A8_0001)),
        ("dst_ip", PyroValue::U32(0x0808_0808)),
        ("src_port", PyroValue::U16(443)),
        ("dst_port", PyroValue::U16((1024 + seq % 60_000) as u16)),
        ("protocol", PyroValue::U8(6)),
        (
            "payload",
            PyroValue::Bytes(
                (0..payload_len)
                    .map(|i| ((seq as usize + i) % 256) as u8)
                    .collect::<Vec<u8>>()
                    .into(),
            ),
        ),
    ])
}

#[cfg(test)]
fn pcap_schema() -> pyro_schema::human::PyroSchema {
    schema(pcap_record_fields())
}

#[cfg(test)]
fn pcap_value() -> PyroValue<'static> {
    pcap_record_value(0, 4)
}

// ---------------------------------------------------------------------------
// multi_pcap
// ---------------------------------------------------------------------------

#[cfg(test)]
fn multi_pcap_schema() -> pyro_schema::human::PyroSchema {
    schema(vec![
        field("capture_id", scalar(PrimitiveDataType::U64)),
        field("records", list(group(pcap_record_fields()))),
    ])
}

#[cfg(test)]
fn multi_pcap_value() -> PyroValue<'static> {
    row([
        ("capture_id", PyroValue::U64(1)),
        (
            "records",
            PyroValue::List((0..2).map(|seq| pcap_record_value(seq, 4)).collect()),
        ),
    ])
}

// ---------------------------------------------------------------------------
// nullable
// ---------------------------------------------------------------------------

#[cfg(test)]
fn nullable_schema() -> pyro_schema::human::PyroSchema {
    use PrimitiveDataType::*;
    schema(vec![
        field("id", scalar(U32)),
        field("a", option(U32)),
        field("b", option(I64)),
        field("c", option(F64)),
        field("d", option(Bool)),
        field("e", option(U16)),
        field("f", option(I32)),
        field("g", option(I8)),
        field("h", option(U64)),
    ])
}

/// `make_nullable_row(1, null_ratio)`.
#[cfg(test)]
fn nullable_value(null_ratio: f64) -> PyroValue<'static> {
    let keep = |i: usize, v: PyroValue<'static>| {
        if (i as f64 / 8.0) >= null_ratio {
            v
        } else {
            PyroValue::Null
        }
    };
    row([
        ("id", PyroValue::U32(1)),
        ("a", keep(0, PyroValue::U32(1))),
        ("b", keep(1, PyroValue::I64(-2))),
        ("c", keep(2, PyroValue::F64(3.5))),
        ("d", keep(3, PyroValue::Bool(true))),
        ("e", keep(4, PyroValue::U16(7))),
        ("f", keep(5, PyroValue::I32(-9))),
        ("g", keep(6, PyroValue::I8(-1))),
        ("h", keep(7, PyroValue::U64(42))),
    ])
}

// ---------------------------------------------------------------------------
// fixed_arrays
// ---------------------------------------------------------------------------

#[cfg(test)]
fn fixed_arrays_schema() -> pyro_schema::human::PyroSchema {
    use PrimitiveDataType::*;
    schema(vec![
        field("id", scalar(U32)),
        field(
            "small",
            PyroType::PrimitiveFixedList {
                element: U32,
                size: 4,
            },
        ),
        field(
            "large",
            PyroType::PrimitiveFixedList {
                element: F64,
                size: 64,
            },
        ),
        field(
            "maybe",
            PyroType::OptionPrimitiveFixedList {
                element: I64,
                size: 8,
            },
        ),
    ])
}

#[cfg(test)]
fn fixed_arrays_value() -> PyroValue<'static> {
    row([
        ("id", PyroValue::U32(1)),
        ("small", PyroValue::from([1u32, 2, 3, 4])),
        (
            "large",
            PyroValue::from(std::array::from_fn::<f64, 64, _>(|i| i as f64 * 1.5)),
        ),
        (
            "maybe",
            PyroValue::from(std::array::from_fn::<i64, 8, _>(|i| i as i64 - 4)),
        ),
    ])
}

// ---------------------------------------------------------------------------
// raw_bytes
// ---------------------------------------------------------------------------

#[cfg(test)]
fn raw_bytes_schema() -> pyro_schema::human::PyroSchema {
    use PrimitiveDataType::*;
    schema(vec![
        field("id", scalar(U32)),
        field("blob", PyroType::Bytes),
        field("samples", prim_list(U8)),
    ])
}

#[cfg(test)]
fn raw_bytes_value() -> PyroValue<'static> {
    // `make_raw_bytes(4)`.
    row([
        ("id", PyroValue::U32(1)),
        ("blob", PyroValue::Bytes(vec![0u8, 1, 2, 3].into())),
        (
            "samples",
            PyroValue::PrimitiveList(PrimitiveValueList::U8(vec![0u8, 1, 2, 3].into())),
        ),
    ])
}

// ---------------------------------------------------------------------------
// wide
// ---------------------------------------------------------------------------

#[cfg(test)]
fn wide_schema() -> pyro_schema::human::PyroSchema {
    use PrimitiveDataType::*;
    let mut fields: Vec<PyroField> = [
        U32, I32, U64, I64, F32, F64, Bool, U16, I16, U8, I8, U32, I32, U64, I64, F32, F64, Bool,
        U16, I16, U32, I32, U64, I64, F32, F64, Bool, U16, I16, U8, I8, U32,
    ]
    .into_iter()
    .enumerate()
    .map(|(i, p)| {
        // `f16` is a Rust type name, so the struct field is `f16_`.
        let name = if i == 16 {
            "f16_".to_string()
        } else {
            format!("f{i}")
        };
        field(&name, scalar(p))
    })
    .collect();
    fields.push(field("reading", scalar(F16)));
    fields.push(field(
        "ts",
        PyroType::PrimitiveScalar {
            primitive: PrimitiveDataType::Timestamp,
        },
    ));
    schema(fields)
}

#[cfg(test)]
fn wide_value() -> PyroValue<'static> {
    row([
        ("f0", PyroValue::U32(1)),
        ("f1", PyroValue::I32(-1)),
        ("f2", PyroValue::U64(2)),
        ("f3", PyroValue::I64(-2)),
        ("f4", PyroValue::F32(1.5)),
        ("f5", PyroValue::F64(-1.5)),
        ("f6", PyroValue::Bool(true)),
        ("f7", PyroValue::U16(3)),
        ("f8", PyroValue::I16(-3)),
        ("f9", PyroValue::U8(4)),
        ("f10", PyroValue::I8(-4)),
        ("f11", PyroValue::U32(5)),
        ("f12", PyroValue::I32(-5)),
        ("f13", PyroValue::U64(6)),
        ("f14", PyroValue::I64(-6)),
        ("f15", PyroValue::F32(2.5)),
        ("f16_", PyroValue::F64(-2.5)),
        ("f17", PyroValue::Bool(false)),
        ("f18", PyroValue::U16(7)),
        ("f19", PyroValue::I16(-7)),
        ("f20", PyroValue::U32(8)),
        ("f21", PyroValue::I32(-8)),
        ("f22", PyroValue::U64(9)),
        ("f23", PyroValue::I64(-9)),
        ("f24", PyroValue::F32(3.5)),
        ("f25", PyroValue::F64(-3.5)),
        ("f26", PyroValue::Bool(true)),
        ("f27", PyroValue::U16(10)),
        ("f28", PyroValue::I16(-10)),
        ("f29", PyroValue::U8(11)),
        ("f30", PyroValue::I8(-11)),
        ("f31", PyroValue::U32(12)),
        ("reading", PyroValue::F16(f16::from_f32(98.6))),
        ("ts", PyroValue::Timestamp(Time(1_700_000_000_000_000_000))),
    ])
}

// ---------------------------------------------------------------------------
// map
// ---------------------------------------------------------------------------

#[cfg(test)]
fn map_schema() -> pyro_schema::human::PyroSchema {
    schema(vec![
        field("id", scalar(PrimitiveDataType::U32)),
        field(
            "tags",
            PyroType::Map {
                key: Box::new(PyroType::Str),
                value: Box::new(PyroType::Str),
            },
        ),
    ])
}

#[cfg(test)]
fn map_value() -> PyroValue<'static> {
    // `make_with_map(2)`.
    row([
        ("id", PyroValue::U32(1)),
        (
            "tags",
            PyroValue::Map(
                vec![
                    (str_("key-0"), str_("value-0")),
                    (str_("key-1"), str_("value-1")),
                ]
                .into(),
            ),
        ),
    ])
}

// ===========================================================================
// Goldens
// ===========================================================================

#[rustfmt::skip]
pub const BENCH_SIMPLE: [u8; 17] = [
    31, 133, 235, 81, 184, 30, 9, 64, // 0 score
    42, 0, 0, 0, // 8 id
    249, 255, 255, 255, // 12 count
    1, // 16 active
];
pub const BENCH_SIMPLE_INODES: [Inode; 4] = [
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 8,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
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
            group: false,
            group_nest: false,
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
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::F64,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 0,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_bench_simple() {
    run_list_wrapped_roundtrip_u16(
        simple_schema(),
        simple_value(),
        &BENCH_SIMPLE,
        &BENCH_SIMPLE_INODES,
    );
}

#[rustfmt::skip]
pub const BENCH_STRINGS: [u8; 27] = [
    8, 0, 11, 0, // 0 name
    19, 0, 8, 0, // 4 tag
    104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100, 103, 114, 101, 101, 116, // 8 variable region: "hello worldgreet"
    105, 110, 103, // 24 "ing"
];
pub const BENCH_STRINGS_INODES: [Inode; 2] = [
    Inode {
        kind: WireKind::Str,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 0,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Str,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 4,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_bench_strings() {
    run_list_wrapped_roundtrip_u16(
        strings_schema(),
        strings_value(),
        &BENCH_STRINGS,
        &BENCH_STRINGS_INODES,
    );
}

#[rustfmt::skip]
pub const BENCH_VECS: [u8; 53] = [
    40, 0, 3, 0, // 0 data_u32
    16, 0, 3, 0, // 4 data_i64
    52, 0, 3, 0, // 8 flags
    0, 0, 0, 0, // 12 variable region
    0, 0, 0, 0, // 16
    0, 0, 0, 0, // 20 alignment padding (4 bytes)
    1, 0, 0, 0, // 24
    0, 0, 0, 0, // 28
    2, 0, 0, 0, // 32
    0, 0, 0, 0, // 36
    0, 0, 0, 0, // 40
    1, 0, 0, 0, // 44
    2, 0, 0, 0, // 48
    5, // 52
];
pub const BENCH_VECS_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 0,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I64,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 4,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Bool,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 8,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_bench_vecs() {
    run_list_wrapped_roundtrip_u16(vecs_schema(), vecs_value(), &BENCH_VECS, &BENCH_VECS_INODES);
}

#[rustfmt::skip]
pub const BENCH_NESTED: [u8; 34] = [
    16, 0, 7, 0, // 0 inner.street
    23, 0, 11, 0, // 4 inner.city
    57, 48, 0, 0, // 8 inner.zip
    7, 0, // 12 count
    0, 0, // 14 alignment padding (2 bytes)
    77, 97, 105, 110, 32, 83, 116, 83, 112, 114, 105, 110, 103, 102, 105, 101, // 16 variable region: "Main StSpringfie"
    108, 100, // 32 "ld"
];
pub const BENCH_NESTED_INODES: [Inode; 5] = [
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 0,
            count: 3,
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
            data_offset: 0,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Str,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 4,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
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
            group_nest: false,
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
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_bench_nested() {
    run_list_wrapped_roundtrip_u16(
        nested_schema(),
        nested_value(),
        &BENCH_NESTED,
        &BENCH_NESTED_INODES,
    );
}

#[rustfmt::skip]
pub const BENCH_TRIPLE_NESTED_VEC: [u8; 60] = [
    12, 0, 8, 0, // 0 cube topology
    28, 0, 8, 0, // 4 cube
    1, 0, 0, 0, // 8 id
    255, 255, // 12 variable region
    2, 0, // 14
    4, 0, // 16
    255, 255, // 18
    2, 0, // 20
    4, 0, // 22
    6, 0, // 24
    8, 0, // 26
    0, 0, 0, 0, // 28
    1, 0, 0, 0, // 32
    2, 0, 0, 0, // 36
    3, 0, 0, 0, // 40
    4, 0, 0, 0, // 44
    5, 0, 0, 0, // 48
    6, 0, 0, 0, // 52
    7, 0, 0, 0, // 56
];
pub const BENCH_TRIPLE_NESTED_VEC_INODES: [Inode; 2] = [
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 8,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 3,
            data_offset: 4,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_bench_triple_nested_vec() {
    run_list_wrapped_roundtrip_u16(
        triple_nested_vec_schema(),
        triple_nested_vec_value(),
        &BENCH_TRIPLE_NESTED_VEC,
        &BENCH_TRIPLE_NESTED_VEC_INODES,
    );
}

#[rustfmt::skip]
pub const BENCH_NESTED_VEC_STRUCT: [u8; 100] = [
    52, 0, 8, 0, // 0 name
    2, 0, // 4 address topology
    32, 0, 3, 0, // 6 address.street topology
    60, 0, 18, 0, // 10 address.street
    38, 0, 3, 0, // 14 address.city topology
    78, 0, 22, 0, // 18 address.city
    44, 0, 2, 0, // 22 address.zip
    0, 0, // 26 alignment padding (2 bytes)
    20, 0, 0, 0, // 28 age
    255, 255, // 32 variable region
    9, 0, // 34
    18, 0, // 36
    255, 255, // 38
    11, 0, // 40
    22, 0, // 42
    16, 39, 0, 0, // 44
    17, 39, 0, 0, // 48
    80, 101, 114, 115, 111, 110, 32, 48, 48, 32, 77, 97, 105, 110, 32, 83, // 52 "Person 00 Main S"
    116, 49, 32, 77, 97, 105, 110, 32, 83, 116, 83, 112, 114, 105, 110, 103, // 68 "t1 Main StSpring"
    102, 105, 101, 108, 100, 83, 112, 114, 105, 110, 103, 102, 105, 101, 108, 100, // 84 "fieldSpringfield"
];
pub const BENCH_NESTED_VEC_STRUCT_INODES: [Inode; 6] = [
    Inode {
        kind: WireKind::Str,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 0,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 28,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 1,
            count: 3,
            topology_offset: 4,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Str,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 10,
            topology_offset: 6,
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
        kind: WireKind::U32,
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
fn test_bench_nested_vec_struct() {
    run_list_wrapped_roundtrip_u16(
        nested_vec_struct_schema(),
        nested_vec_struct_value(),
        &BENCH_NESTED_VEC_STRUCT,
        &BENCH_NESTED_VEC_STRUCT_INODES,
    );
}

#[rustfmt::skip]
pub const BENCH_MIXED_NESTED: [u8; 107] = [
    2, 0, // 0 sensor topology
    64, 0, 2, 0, // 2 sensor.id
    52, 0, 3, 0, // 6 sensor.label topology
    80, 0, 16, 0, // 10 sensor.label
    96, 0, 2, 0, // 14 flags
    58, 0, 3, 0, // 18 tags topology
    97, 0, 10, 0, // 22 tags
    0, 0, 0, 0, 0, 0, // 26 padding
    0, 241, 83, 101, 0, 0, 0, 0, // 32 timestamp
    102, 102, 102, 102, 102, 166, 88, 64, // 40 value
    253, 255, 255, 255, // 48 quality
    255, 255, // 52 variable region
    8, 0, // 54
    16, 0, // 56
    255, 255, // 58
    5, 0, // 60
    10, 0, // 62
    0, 0, 0, 0, // 64
    0, 0, // 68
    0, 0, // 70
    1, 0, 0, 0, // 72
    0, 0, 0, 0, // 76
    115, 101, 110, 115, 111, 114, 45, 48, 115, 101, 110, 115, 111, 114, 45, 49, // 80 "sensor-0sensor-1"
    1, 116, 97, 103, // 96
    45, 48, 116, 97, 103, 45, 49, // 100 "-0tag-1"
];
pub const BENCH_MIXED_NESTED_INODES: [Inode; 8] = [
    Inode {
        kind: WireKind::U64,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 32,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::F64,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 40,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
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
        kind: WireKind::U64,
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
        kind: WireKind::Str,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 10,
            topology_offset: 6,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::Bool,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 14,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Str,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 22,
            topology_offset: 18,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 48,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_bench_mixed_nested() {
    run_list_wrapped_roundtrip_u16(
        mixed_nested_schema(),
        mixed_nested_value(),
        &BENCH_MIXED_NESTED,
        &BENCH_MIXED_NESTED_INODES,
    );
}

#[rustfmt::skip]
pub const BENCH_DEEP_NESTED_VEC_STRUCT: [u8; 221] = [
    116, 0, 9, 0, // 0 name
    2, 0, // 4 employees topology
    60, 0, 3, 0, // 6 employees.name topology
    125, 0, 16, 0, // 10 employees.name
    92, 0, 2, 0, // 14 employees.age
    66, 0, // 18 employees.address topology
    3, 0, // 20 padding
    72, 0, 5, 0, // 22 employees.address.street topology
    141, 0, 36, 0, // 26 employees.address.street
    82, 0, 5, 0, // 30 employees.address.city topology
    177, 0, 44, 0, // 34 employees.address.city
    100, 0, 4, 0, // 38 employees.address.zip
    0, 0, 0, 0, 0, 0, // 42 padding
    61, 10, 215, 227, 135, 214, 50, 65, // 48 revenue
    198, 7, 0, 0, // 56 founded_year
    255, 255, // 60 variable region
    8, 0, // 62
    16, 0, // 64
    255, 255, // 66
    2, 0, // 68
    4, 0, // 70
    255, 255, // 72
    9, 0, // 74
    18, 0, // 76
    27, 0, // 78
    36, 0, // 80
    255, 255, // 82
    11, 0, // 84
    22, 0, // 86
    33, 0, // 88
    44, 0, // 90
    20, 0, 0, 0, // 92
    21, 0, 0, 0, // 96
    16, 39, 0, 0, // 100
    17, 39, 0, 0, // 104
    16, 39, 0, 0, // 108
    17, 39, 0, 0, // 112
    65, 99, 109, 101, 32, 67, 111, 114, 112, 80, 101, 114, 115, 111, 110, 32, // 116 "Acme CorpPerson "
    48, 80, 101, 114, 115, 111, 110, 32, 49, 48, 32, 77, 97, 105, 110, 32, // 132 "0Person 10 Main "
    83, 116, 49, 32, 77, 97, 105, 110, 32, 83, 116, 48, 32, 77, 97, 105, // 148 "St1 Main St0 Mai"
    110, 32, 83, 116, 49, 32, 77, 97, 105, 110, 32, 83, 116, 83, 112, 114, // 164 "n St1 Main StSpr"
    105, 110, 103, 102, 105, 101, 108, 100, 83, 112, 114, 105, 110, 103, 102, 105, // 180 "ingfieldSpringfi"
    101, 108, 100, 83, 112, 114, 105, 110, 103, 102, 105, 101, 108, 100, 83, 112, // 196 "eldSpringfieldSp"
    114, 105, 110, 103, 102, 105, 101, 108, 100, // 212 "ringfield"
];
pub const BENCH_DEEP_NESTED_VEC_STRUCT_INODES: [Inode; 10] = [
    Inode {
        kind: WireKind::Str,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 0,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 56,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 1,
            count: 3,
            topology_offset: 4,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Str,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 10,
            topology_offset: 6,
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
            data_offset: 14,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 1,
            count: 3,
            topology_offset: 18,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
    Inode {
        kind: WireKind::Str,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 26,
            topology_offset: 22,
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
            data_offset: 34,
            topology_offset: 30,
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
            data_offset: 38,
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
            data_offset: 48,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_bench_deep_nested_vec_struct() {
    run_list_wrapped_roundtrip_u16(
        deep_nested_vec_struct_schema(),
        deep_nested_vec_struct_value(),
        &BENCH_DEEP_NESTED_VEC_STRUCT,
        &BENCH_DEEP_NESTED_VEC_STRUCT_INODES,
    );
}

#[rustfmt::skip]
pub const BENCH_PCAP: [u8; 37] = [
    33, 0, 4, 0, // 0 payload
    0, 0, 0, 0, // 4 alignment padding (4 bytes)
    0, 241, 83, 101, 0, 0, 0, 0, // 8 ts_secs
    0, 0, 0, 0, // 16 ts_micros
    1, 0, 168, 192, // 20 src_ip
    8, 8, 8, 8, // 24 dst_ip
    187, 1, // 28 src_port
    0, 4, // 30 dst_port
    6, // 32 protocol
    0, 1, 2, 3, // 33 variable region
];
pub const BENCH_PCAP_INODES: [Inode; 8] = [
    Inode {
        kind: WireKind::U64,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 8,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 16,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 20,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 24,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 28,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 30,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 32,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Bytes,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 0,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_bench_pcap() {
    run_list_wrapped_roundtrip_u16(pcap_schema(), pcap_value(), &BENCH_PCAP, &BENCH_PCAP_INODES);
}

#[rustfmt::skip]
pub const BENCH_MULTI_PCAP: [u8; 114] = [
    2, 0, // 0 records topology
    56, 0, 2, 0, // 2 records.ts_secs
    72, 0, 2, 0, // 6 records.ts_micros
    80, 0, 2, 0, // 10 records.src_ip
    88, 0, 2, 0, // 14 records.dst_ip
    96, 0, 2, 0, // 18 records.src_port
    100, 0, 2, 0, // 22 records.dst_port
    104, 0, 2, 0, // 26 records.protocol
    48, 0, 3, 0, // 30 records.payload topology
    106, 0, 8, 0, // 34 records.payload
    0, 0, // 38 padding
    1, 0, 0, 0, 0, 0, 0, 0, // 40 capture_id
    255, 255, // 48 variable region
    4, 0, // 50
    8, 0, // 52
    0, 0, // 54
    0, 241, 83, 101, // 56
    0, 0, 0, 0, // 60
    1, 241, 83, 101, // 64
    0, 0, 0, 0, // 68
    0, 0, 0, 0, // 72
    1, 0, 0, 0, // 76
    1, 0, 168, 192, // 80
    1, 0, 168, 192, // 84
    8, 8, 8, 8, // 88
    8, 8, 8, 8, // 92
    187, 1, 187, 1, // 96
    0, 4, 1, 4, // 100
    6, 6, 0, 1, // 104
    2, 3, 1, 2, // 108
    3, 4, // 112
];
pub const BENCH_MULTI_PCAP_INODES: [Inode; 10] = [
    Inode {
        kind: WireKind::U64,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 40,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Group,
        kind_meta: KindMeta::Group {
            nesting: 1,
            count: 8,
            topology_offset: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U64,
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
        kind: WireKind::U32,
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
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 14,
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
            data_offset: 18,
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
            data_offset: 22,
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
            data_offset: 26,
            topology_offset: 0,
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
            data_offset: 34,
            topology_offset: 30,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_bench_multi_pcap() {
    run_list_wrapped_roundtrip_u16(
        multi_pcap_schema(),
        multi_pcap_value(),
        &BENCH_MULTI_PCAP,
        &BENCH_MULTI_PCAP_INODES,
    );
}

#[rustfmt::skip]
pub const BENCH_NULLABLE_0: [u8; 41] = [
    254, 255, 255, 255, 255, 255, 255, 255, // 0 b
    0, 0, 0, 0, 0, 0, 12, 64, // 8 c
    42, 0, 0, 0, 0, 0, 0, 0, // 16 h
    1, 0, 0, 0, // 24 id
    1, 0, 0, 0, // 28 a
    247, 255, 255, 255, // 32 f
    7, 0, // 36 e
    1, // 38 d
    255, // 39 g
    255, // 40 validity
];
pub const BENCH_NULLABLE_0_INODES: [Inode; 9] = [
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 24,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionU32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 28,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionI64,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 0,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 1,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionF64,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 8,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 2,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionBool,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 38,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 3,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionU16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 36,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 4,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionI32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 32,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 5,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionI8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 39,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 6,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionU64,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 16,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 7,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_bench_nullable_0() {
    run_list_wrapped_roundtrip_u16(
        nullable_schema(),
        nullable_value(0.0),
        &BENCH_NULLABLE_0,
        &BENCH_NULLABLE_0_INODES,
    );
}

#[rustfmt::skip]
pub const BENCH_NULLABLE_50: [u8; 41] = [
    0, 0, 0, 0, 0, 0, 0, 0, // 0 b
    0, 0, 0, 0, 0, 0, 0, 0, // 8 c
    42, 0, 0, 0, 0, 0, 0, 0, // 16 h
    1, 0, 0, 0, // 24 id
    0, 0, 0, 0, // 28 a
    247, 255, 255, 255, // 32 f
    7, 0, // 36 e
    0, // 38 d
    255, // 39 g
    240, // 40 validity
];
pub const BENCH_NULLABLE_50_INODES: [Inode; 9] = [
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 24,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionU32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 28,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionI64,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 0,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 1,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionF64,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 8,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 2,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionBool,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 38,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 3,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionU16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 36,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 4,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionI32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 32,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 5,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionI8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 39,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 6,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionU64,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 16,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 7,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_bench_nullable_50() {
    run_list_wrapped_roundtrip_u16(
        nullable_schema(),
        nullable_value(0.5),
        &BENCH_NULLABLE_50,
        &BENCH_NULLABLE_50_INODES,
    );
}

#[rustfmt::skip]
pub const BENCH_NULLABLE_100: [u8; 41] = [
    0, 0, 0, 0, 0, 0, 0, 0, // 0 b
    0, 0, 0, 0, 0, 0, 0, 0, // 8 c
    0, 0, 0, 0, 0, 0, 0, 0, // 16 h
    1, 0, 0, 0, // 24 id
    0, 0, 0, 0, // 28 a
    0, 0, 0, 0, // 32 f
    0, 0, // 36 e
    0, // 38 d
    0, // 39 g
    0, // 40 validity
];
pub const BENCH_NULLABLE_100_INODES: [Inode; 9] = [
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 24,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionU32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 28,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionI64,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 0,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 1,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionF64,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 8,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 2,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionBool,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 38,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 3,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionU16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 36,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 4,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionI32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 32,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 5,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionI8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 39,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 6,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionU64,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 16,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 7,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_bench_nullable_100() {
    run_list_wrapped_roundtrip_u16(
        nullable_schema(),
        nullable_value(1.0),
        &BENCH_NULLABLE_100,
        &BENCH_NULLABLE_100_INODES,
    );
}

#[rustfmt::skip]
pub const BENCH_FIXED_ARRAYS: [u8; 597] = [
    0, 0, 0, 0, 0, 0, 0, 0, // 0 large[0]
    0, 0, 0, 0, 0, 0, 248, 63, // 8 large[1]
    0, 0, 0, 0, 0, 0, 8, 64, // 16 large[2]
    0, 0, 0, 0, 0, 0, 18, 64, // 24 large[3]
    0, 0, 0, 0, 0, 0, 24, 64, // 32 large[4]
    0, 0, 0, 0, 0, 0, 30, 64, // 40 large[5]
    0, 0, 0, 0, 0, 0, 34, 64, // 48 large[6]
    0, 0, 0, 0, 0, 0, 37, 64, // 56 large[7]
    0, 0, 0, 0, 0, 0, 40, 64, // 64 large[8]
    0, 0, 0, 0, 0, 0, 43, 64, // 72 large[9]
    0, 0, 0, 0, 0, 0, 46, 64, // 80 large[10]
    0, 0, 0, 0, 0, 128, 48, 64, // 88 large[11]
    0, 0, 0, 0, 0, 0, 50, 64, // 96 large[12]
    0, 0, 0, 0, 0, 128, 51, 64, // 104 large[13]
    0, 0, 0, 0, 0, 0, 53, 64, // 112 large[14]
    0, 0, 0, 0, 0, 128, 54, 64, // 120 large[15]
    0, 0, 0, 0, 0, 0, 56, 64, // 128 large[16]
    0, 0, 0, 0, 0, 128, 57, 64, // 136 large[17]
    0, 0, 0, 0, 0, 0, 59, 64, // 144 large[18]
    0, 0, 0, 0, 0, 128, 60, 64, // 152 large[19]
    0, 0, 0, 0, 0, 0, 62, 64, // 160 large[20]
    0, 0, 0, 0, 0, 128, 63, 64, // 168 large[21]
    0, 0, 0, 0, 0, 128, 64, 64, // 176 large[22]
    0, 0, 0, 0, 0, 64, 65, 64, // 184 large[23]
    0, 0, 0, 0, 0, 0, 66, 64, // 192 large[24]
    0, 0, 0, 0, 0, 192, 66, 64, // 200 large[25]
    0, 0, 0, 0, 0, 128, 67, 64, // 208 large[26]
    0, 0, 0, 0, 0, 64, 68, 64, // 216 large[27]
    0, 0, 0, 0, 0, 0, 69, 64, // 224 large[28]
    0, 0, 0, 0, 0, 192, 69, 64, // 232 large[29]
    0, 0, 0, 0, 0, 128, 70, 64, // 240 large[30]
    0, 0, 0, 0, 0, 64, 71, 64, // 248 large[31]
    0, 0, 0, 0, 0, 0, 72, 64, // 256 large[32]
    0, 0, 0, 0, 0, 192, 72, 64, // 264 large[33]
    0, 0, 0, 0, 0, 128, 73, 64, // 272 large[34]
    0, 0, 0, 0, 0, 64, 74, 64, // 280 large[35]
    0, 0, 0, 0, 0, 0, 75, 64, // 288 large[36]
    0, 0, 0, 0, 0, 192, 75, 64, // 296 large[37]
    0, 0, 0, 0, 0, 128, 76, 64, // 304 large[38]
    0, 0, 0, 0, 0, 64, 77, 64, // 312 large[39]
    0, 0, 0, 0, 0, 0, 78, 64, // 320 large[40]
    0, 0, 0, 0, 0, 192, 78, 64, // 328 large[41]
    0, 0, 0, 0, 0, 128, 79, 64, // 336 large[42]
    0, 0, 0, 0, 0, 32, 80, 64, // 344 large[43]
    0, 0, 0, 0, 0, 128, 80, 64, // 352 large[44]
    0, 0, 0, 0, 0, 224, 80, 64, // 360 large[45]
    0, 0, 0, 0, 0, 64, 81, 64, // 368 large[46]
    0, 0, 0, 0, 0, 160, 81, 64, // 376 large[47]
    0, 0, 0, 0, 0, 0, 82, 64, // 384 large[48]
    0, 0, 0, 0, 0, 96, 82, 64, // 392 large[49]
    0, 0, 0, 0, 0, 192, 82, 64, // 400 large[50]
    0, 0, 0, 0, 0, 32, 83, 64, // 408 large[51]
    0, 0, 0, 0, 0, 128, 83, 64, // 416 large[52]
    0, 0, 0, 0, 0, 224, 83, 64, // 424 large[53]
    0, 0, 0, 0, 0, 64, 84, 64, // 432 large[54]
    0, 0, 0, 0, 0, 160, 84, 64, // 440 large[55]
    0, 0, 0, 0, 0, 0, 85, 64, // 448 large[56]
    0, 0, 0, 0, 0, 96, 85, 64, // 456 large[57]
    0, 0, 0, 0, 0, 192, 85, 64, // 464 large[58]
    0, 0, 0, 0, 0, 32, 86, 64, // 472 large[59]
    0, 0, 0, 0, 0, 128, 86, 64, // 480 large[60]
    0, 0, 0, 0, 0, 224, 86, 64, // 488 large[61]
    0, 0, 0, 0, 0, 64, 87, 64, // 496 large[62]
    0, 0, 0, 0, 0, 160, 87, 64, // 504 large[63]
    252, 255, 255, 255, 255, 255, 255, 255, // 512 maybe[0]
    253, 255, 255, 255, 255, 255, 255, 255, // 520 maybe[1]
    254, 255, 255, 255, 255, 255, 255, 255, // 528 maybe[2]
    255, 255, 255, 255, 255, 255, 255, 255, // 536 maybe[3]
    0, 0, 0, 0, 0, 0, 0, 0, // 544 maybe[4]
    1, 0, 0, 0, 0, 0, 0, 0, // 552 maybe[5]
    2, 0, 0, 0, 0, 0, 0, 0, // 560 maybe[6]
    3, 0, 0, 0, 0, 0, 0, 0, // 568 maybe[7]
    1, 0, 0, 0, // 576 id
    1, 0, 0, 0, // 580 small[0]
    2, 0, 0, 0, // 584 small[1]
    3, 0, 0, 0, // 588 small[2]
    4, 0, 0, 0, // 592 small[3]
    1, // 596 validity
];
pub const BENCH_FIXED_ARRAYS_INODES: [Inode; 4] = [
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 576,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 580,
            topology_offset: 0,
            fixed_size: 4,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::F64,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 0,
            topology_offset: 0,
            fixed_size: 64,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::OptionI64,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 512,
            topology_offset: 0,
            fixed_size: 8,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_bench_fixed_arrays() {
    run_list_wrapped_roundtrip_u16(
        fixed_arrays_schema(),
        fixed_arrays_value(),
        &BENCH_FIXED_ARRAYS,
        &BENCH_FIXED_ARRAYS_INODES,
    );
}

#[rustfmt::skip]
pub const BENCH_RAW_BYTES: [u8; 20] = [
    12, 0, 4, 0, // 0 blob
    16, 0, 4, 0, // 4 samples
    1, 0, 0, 0, // 8 id
    0, 1, 2, 3, // 12 variable region
    0, 1, 2, 3, // 16
];
pub const BENCH_RAW_BYTES_INODES: [Inode; 3] = [
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 8,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Bytes,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 0,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 1,
            data_offset: 4,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_bench_raw_bytes() {
    run_list_wrapped_roundtrip_u16(
        raw_bytes_schema(),
        raw_bytes_value(),
        &BENCH_RAW_BYTES,
        &BENCH_RAW_BYTES_INODES,
    );
}

#[rustfmt::skip]
pub const BENCH_WIDE: [u8; 149] = [
    0, 0, 42, 54, 254, 156, 151, 23, 0, 0, 0, 0, 0, 0, 0, 0, // 0 ts
    2, 0, 0, 0, 0, 0, 0, 0, // 16 f2
    254, 255, 255, 255, 255, 255, 255, 255, // 24 f3
    0, 0, 0, 0, 0, 0, 248, 191, // 32 f5
    6, 0, 0, 0, 0, 0, 0, 0, // 40 f13
    250, 255, 255, 255, 255, 255, 255, 255, // 48 f14
    0, 0, 0, 0, 0, 0, 4, 192, // 56 f16_
    9, 0, 0, 0, 0, 0, 0, 0, // 64 f22
    247, 255, 255, 255, 255, 255, 255, 255, // 72 f23
    0, 0, 0, 0, 0, 0, 12, 192, // 80 f25
    1, 0, 0, 0, // 88 f0
    255, 255, 255, 255, // 92 f1
    0, 0, 192, 63, // 96 f4
    5, 0, 0, 0, // 100 f11
    251, 255, 255, 255, // 104 f12
    0, 0, 32, 64, // 108 f15
    8, 0, 0, 0, // 112 f20
    248, 255, 255, 255, // 116 f21
    0, 0, 96, 64, // 120 f24
    12, 0, 0, 0, // 124 f31
    3, 0, // 128 f7
    253, 255, // 130 f8
    7, 0, // 132 f18
    249, 255, // 134 f19
    10, 0, // 136 f27
    246, 255, // 138 f28
    42, 86, // 140 reading
    1, // 142 f6
    4, // 143 f9
    252, // 144 f10
    0, // 145 f17
    1, // 146 f26
    11, // 147 f29
    245, // 148 f30
];
pub const BENCH_WIDE_INODES: [Inode; 34] = [
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 88,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 92,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U64,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 16,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I64,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 24,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::F32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 96,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::F64,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 32,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Bool,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 142,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 128,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 130,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 143,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 144,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 100,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 104,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
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
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I64,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 48,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::F32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 108,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::F64,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 56,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Bool,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 145,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 132,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 134,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 112,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 116,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U64,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 64,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I64,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 72,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::F32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 120,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::F64,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 80,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Bool,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 146,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 136,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 138,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 147,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::I8,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 148,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 124,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::F16,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 140,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Timestamp,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 0,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_bench_wide() {
    run_list_wrapped_roundtrip_u16(wide_schema(), wide_value(), &BENCH_WIDE, &BENCH_WIDE_INODES);
}

#[rustfmt::skip]
pub const BENCH_MAP: [u8; 60] = [
    2, 0, // 0 tags count
    24, 0, 3, 0, // 2 tags.key topology
    36, 0, 10, 0, // 6 tags.key
    30, 0, 3, 0, // 10 tags.value topology
    46, 0, 14, 0, // 14 tags.value
    0, 0, // 18 alignment padding (2 bytes)
    1, 0, 0, 0, // 20 id
    255, 255, // 24 variable region
    5, 0, // 26
    10, 0, // 28
    255, 255, // 30
    7, 0, // 32
    14, 0, // 34
    107, 101, 121, 45, 48, 107, 101, 121, 45, 49, 118, 97, 108, 117, 101, 45, // 36 "key-0key-1value-"
    48, 118, 97, 108, 117, 101, 45, 49, // 52 "0value-1"
];
pub const BENCH_MAP_INODES: [Inode; 4] = [
    Inode {
        kind: WireKind::U32,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 20,
            topology_offset: 0,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: false,
            group_nest: false,
        },
    },
    Inode {
        kind: WireKind::Map,
        kind_meta: KindMeta::Map {
            nesting: 0,
            topology_offset: 0,
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
        kind: WireKind::Str,
        kind_meta: KindMeta::Primitive {
            nesting: 0,
            data_offset: 14,
            topology_offset: 10,
            fixed_size: 0,
            nullable_bit_index: 0,
            group: true,
            group_nest: true,
        },
    },
];

#[cfg_attr(test, tracing_test::traced_test)]
#[test]
fn test_bench_map() {
    run_list_wrapped_roundtrip_u16(map_schema(), map_value(), &BENCH_MAP, &BENCH_MAP_INODES);
}
