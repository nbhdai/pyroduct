// The `benches/roundtrip.rs` structs, derived exactly as the bench declares
// them, checked against `src/wire/tests/bench_u32.rs`'s value-writer goldens.
// Each value mirrors that file's `*_value()` builder.

use std::collections::BTreeMap;

use half::f16;
use pyroformat::value::Time;
use pyroformat::wire::tests::bench_u32::*;
use pyroformat::{BitVec, Layout, Reader, Writer};

use crate::support::read_back::assert_roundtrip;

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Simple {
    id: u32,
    count: i32,
    active: bool,
    score: f64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct WithStrings {
    name: String,
    tag: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct WithVecs {
    data_u32: Vec<u32>,
    data_i64: Vec<i64>,
    #[pyro(allow(vec_bool))]
    flags: Vec<bool>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct WithBitVec {
    data_u32: Vec<u32>,
    data_i64: Vec<i64>,
    flags: BitVec,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Address {
    street: String,
    city: String,
    zip: u32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Outer {
    inner: Address,
    count: u32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct TripleNested {
    id: u32,
    cube: Vec<Vec<Vec<u32>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Person {
    name: String,
    age: i32,
    address: Vec<Address>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Sensor {
    id: u64,
    label: String,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct SensorReading {
    timestamp: u64,
    value: f64,
    sensor: Vec<Sensor>,
    flags: BitVec,
    tags: Vec<String>,
    quality: i32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct Company {
    name: String,
    founded_year: u32,
    employees: Vec<Person>,
    revenue: f64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PcapRecord {
    ts_secs: u64,
    ts_micros: u32,
    src_ip: u32,
    dst_ip: u32,
    src_port: u16,
    dst_port: u16,
    protocol: u8,
    payload: Vec<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PcapFile {
    capture_id: u64,
    records: Vec<PcapRecord>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct NullableRow {
    id: u32,
    a: Option<u32>,
    b: Option<i64>,
    c: Option<f64>,
    d: Option<bool>,
    e: Option<u16>,
    f: Option<i32>,
    g: Option<i8>,
    h: Option<u64>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FixedArrays {
    id: u32,
    small: [u32; 4],
    large: [f64; 64],
    maybe: Option<[i64; 8]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct RawBytes {
    id: u32,
    blob: Vec<u8>,
    #[pyro(numeric)]
    samples: Vec<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct WideRow {
    f0: u32,
    f1: i32,
    f2: u64,
    f3: i64,
    f4: f32,
    f5: f64,
    f6: bool,
    f7: u16,
    f8: i16,
    f9: u8,
    f10: i8,
    f11: u32,
    f12: i32,
    f13: u64,
    f14: i64,
    f15: f32,
    f16_: f64,
    f17: bool,
    f18: u16,
    f19: i16,
    f20: u32,
    f21: i32,
    f22: u64,
    f23: i64,
    f24: f32,
    f25: f64,
    f26: bool,
    f27: u16,
    f28: i16,
    f29: u8,
    f30: i8,
    f31: u32,
    reading: f16,
    ts: Time,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct WithMap {
    id: u32,
    tags: BTreeMap<String, String>,
}

fn bits(bools: &[bool]) -> BitVec {
    bools.iter().copied().collect()
}

/// `make_addresses(n)`.
fn make_addresses(n: u32) -> Vec<Address> {
    (0..n)
        .map(|i| Address {
            street: format!("{i} Main St"),
            city: "Springfield".to_string(),
            zip: 10000 + i,
        })
        .collect()
}

/// `make_person(id, num_addresses)`.
fn make_person(id: u32, num_addresses: u32) -> Person {
    Person {
        name: format!("Person {id}"),
        age: 20 + (id % 50) as i32,
        address: make_addresses(num_addresses),
    }
}

/// `make_pcap_record(seq, payload_len)`.
fn make_pcap_record(seq: u64, payload_len: usize) -> PcapRecord {
    PcapRecord {
        ts_secs: 1_700_000_000 + seq,
        ts_micros: (seq % 1_000_000) as u32,
        src_ip: 0xC0A8_0001,
        dst_ip: 0x0808_0808,
        src_port: 443,
        dst_port: (1024 + seq % 60_000) as u16,
        protocol: 6,
        payload: (0..payload_len)
            .map(|i| ((seq as usize + i) % 256) as u8)
            .collect(),
    }
}

/// `make_nullable_row(1, null_ratio)`.
fn make_nullable_row(null_ratio: f64) -> NullableRow {
    let keep = |i: usize| (i as f64 / 8.0) >= null_ratio;
    NullableRow {
        id: 1,
        a: keep(0).then_some(1),
        b: keep(1).then_some(-2),
        c: keep(2).then_some(3.5),
        d: keep(3).then_some(true),
        e: keep(4).then_some(7),
        f: keep(5).then_some(-9),
        g: keep(6).then_some(-1),
        h: keep(7).then_some(42),
    }
}

#[test]
fn value_match_bench_simple() {
    let data = Simple {
        id: 42,
        count: -7,
        active: true,
        score: 3.14,
    };
    assert_roundtrip(&data, &BENCH_SIMPLE, &BENCH_SIMPLE_INODES);
}

#[test]
fn value_match_bench_strings() {
    let data = WithStrings {
        name: "hello world".to_string(),
        tag: "greeting".to_string(),
    };
    assert_roundtrip(&data, &BENCH_STRINGS, &BENCH_STRINGS_INODES);
}

#[test]
fn value_match_bench_vecs() {
    let data = WithVecs {
        data_u32: vec![0, 1, 2],
        data_i64: vec![0, 1, 2],
        flags: vec![true, false, true],
    };
    assert_roundtrip(&data, &BENCH_VECS, &BENCH_VECS_INODES);
}

#[test]
fn value_match_bench_vecs_bitvec() {
    let data = WithBitVec {
        data_u32: vec![0, 1, 2],
        data_i64: vec![0, 1, 2],
        flags: bits(&[true, false, true]),
    };
    assert_roundtrip(&data, &BENCH_VECS, &BENCH_VECS_INODES);
}

#[test]
fn value_match_bench_nested() {
    let data = Outer {
        inner: Address {
            street: "Main St".to_string(),
            city: "Springfield".to_string(),
            zip: 12345,
        },
        count: 7,
    };
    assert_roundtrip(&data, &BENCH_NESTED, &BENCH_NESTED_INODES);
}

#[test]
fn value_match_bench_triple_nested_vec() {
    // `make_cube(2)`.
    let data = TripleNested {
        id: 1,
        cube: vec![vec![vec![0, 1], vec![2, 3]], vec![vec![4, 5], vec![6, 7]]],
    };
    assert_roundtrip(
        &data,
        &BENCH_TRIPLE_NESTED_VEC,
        &BENCH_TRIPLE_NESTED_VEC_INODES,
    );
}

#[test]
fn value_match_bench_nested_vec_struct() {
    let data = make_person(0, 2);
    assert_roundtrip(
        &data,
        &BENCH_NESTED_VEC_STRUCT,
        &BENCH_NESTED_VEC_STRUCT_INODES,
    );
}

#[test]
fn value_match_bench_mixed_nested() {
    let data = SensorReading {
        timestamp: 1_700_000_000,
        value: 98.6,
        sensor: (0..2)
            .map(|i| Sensor {
                id: i,
                label: format!("sensor-{i}"),
            })
            .collect(),
        flags: bits(&[true, false]),
        tags: vec!["tag-0".to_string(), "tag-1".to_string()],
        quality: -3,
    };
    assert_roundtrip(&data, &BENCH_MIXED_NESTED, &BENCH_MIXED_NESTED_INODES);
}

#[test]
fn value_match_bench_deep_nested_vec_struct() {
    let data = Company {
        name: "Acme Corp".to_string(),
        founded_year: 1990,
        employees: (0..2).map(|i| make_person(i, 2)).collect(),
        revenue: 1_234_567.89,
    };
    assert_roundtrip(
        &data,
        &BENCH_DEEP_NESTED_VEC_STRUCT,
        &BENCH_DEEP_NESTED_VEC_STRUCT_INODES,
    );
}

#[test]
fn value_match_bench_pcap() {
    let data = make_pcap_record(0, 4);
    assert_roundtrip(&data, &BENCH_PCAP, &BENCH_PCAP_INODES);
}

#[test]
fn value_match_bench_multi_pcap() {
    let data = PcapFile {
        capture_id: 1,
        records: (0..2).map(|seq| make_pcap_record(seq, 4)).collect(),
    };
    assert_roundtrip(&data, &BENCH_MULTI_PCAP, &BENCH_MULTI_PCAP_INODES);
}

#[test]
fn value_match_bench_nullable_0() {
    let data = make_nullable_row(0.0);
    assert_roundtrip(&data, &BENCH_NULLABLE_0, &BENCH_NULLABLE_0_INODES);
}

#[test]
fn value_match_bench_nullable_50() {
    let data = make_nullable_row(0.5);
    assert_roundtrip(&data, &BENCH_NULLABLE_50, &BENCH_NULLABLE_50_INODES);
}

#[test]
fn value_match_bench_nullable_100() {
    let data = make_nullable_row(1.0);
    assert_roundtrip(&data, &BENCH_NULLABLE_100, &BENCH_NULLABLE_100_INODES);
}

#[test]
fn value_match_bench_fixed_arrays() {
    let data = FixedArrays {
        id: 1,
        small: [1, 2, 3, 4],
        large: std::array::from_fn(|i| i as f64 * 1.5),
        maybe: Some(std::array::from_fn(|i| i as i64 - 4)),
    };
    assert_roundtrip(&data, &BENCH_FIXED_ARRAYS, &BENCH_FIXED_ARRAYS_INODES);
}

#[test]
fn value_match_bench_raw_bytes() {
    // `make_raw_bytes(4)`.
    let data = RawBytes {
        id: 1,
        blob: vec![0, 1, 2, 3],
        samples: vec![0, 1, 2, 3],
    };
    assert_roundtrip(&data, &BENCH_RAW_BYTES, &BENCH_RAW_BYTES_INODES);
}

#[test]
fn value_match_bench_wide() {
    let data = WideRow {
        f0: 1,
        f1: -1,
        f2: 2,
        f3: -2,
        f4: 1.5,
        f5: -1.5,
        f6: true,
        f7: 3,
        f8: -3,
        f9: 4,
        f10: -4,
        f11: 5,
        f12: -5,
        f13: 6,
        f14: -6,
        f15: 2.5,
        f16_: -2.5,
        f17: false,
        f18: 7,
        f19: -7,
        f20: 8,
        f21: -8,
        f22: 9,
        f23: -9,
        f24: 3.5,
        f25: -3.5,
        f26: true,
        f27: 10,
        f28: -10,
        f29: 11,
        f30: -11,
        f31: 12,
        reading: f16::from_f32(98.6),
        ts: Time(1_700_000_000_000_000_000),
    };
    assert_roundtrip(&data, &BENCH_WIDE, &BENCH_WIDE_INODES);
}

#[test]
fn value_match_bench_map() {
    // `make_with_map(2)`.
    let data = WithMap {
        id: 1,
        tags: (0..2)
            .map(|i| (format!("key-{i}"), format!("value-{i}")))
            .collect(),
    };
    assert_roundtrip(&data, &BENCH_MAP, &BENCH_MAP_INODES);
}
