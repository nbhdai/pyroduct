//! Criterion benchmarks for the derive-generated writer/reader roundtrip.
//!
//! Requires the `fixtures` feature (for `encode_pre` / `encode_reg`). Run
//! with:
//!   cargo bench --features fixtures
//! or a single group:
//!   cargo bench --features fixtures --bench roundtrip simple
//!
//! Every pyroformat struct here is `#[derive(Layout, Writer, Reader)]`, and
//! each group measures the same three things against an rkyv twin:
//!
//!   - `buf_len`: `fixed_len() + variable_length()` alone — the sizing
//!     pass `write` pays before it writes anything.
//!   - `write_pre`, `write_reg`, `write`: one full `encode_pre`,
//!     `encode_reg` and `encode` respectively — the pre-writer path, the
//!     regular writer path, and the optimized path callers get by default.
//!     Each preallocates from the type's running `max_variable()` high-water
//!     mark, so there is no separate sizing pass.
//!   - `read`: `pyro_read` and `realize()` into a fully owned value — the
//!     apples-to-apples comparison against `read_rkyv`.
//!   - `read_ref`: `pyro_read` alone — the borrowed view, the one to hold
//!     against `read_rkyv_access`.
//!
//! Groups whose interesting part is one list-of-structs field read just that
//! field (`read_*_field`) instead of the whole struct.

use criterion::measurement::WallTime;
use criterion::{BenchmarkGroup, BenchmarkId, Criterion, criterion_group, criterion_main};
use half::f16;
use pprof::criterion::{Output, PProfProfiler};
use pyroformat::buf::PyroBuf;
use pyroformat::value::Time;
use pyroformat::{BitVec, Layout, Reader, Realize, Writer};
use rkyv::api::high::{HighDeserializer, HighSerializer, HighValidator};
use rkyv::rancor::Error as RkError;
use rkyv::ser::allocator::ArenaHandle;
use rkyv::util::AlignedVec;
use rkyv::{
    Archive as RkArchive, Deserialize as RkDeserialize, Portable as RkPortable,
    Serialize as RkSerialize,
};
use std::collections::BTreeMap;
use std::fmt::Display;
use std::hint::black_box;

// ── pyroformat helpers ─────────────────────────────────────────────────────

/// The full owned-value read.
fn decode<T: Reader>(buf: &[u8]) -> T {
    T::pyro_read(buf).unwrap().realize()
}

/// A benchmark name, with the sweep parameter when the group has one.
fn id(name: &str, param: Option<&dyn Display>) -> BenchmarkId {
    match param {
        Some(param) => BenchmarkId::new(name, param),
        None => BenchmarkId::from_parameter(name),
    }
}

/// The final buffer length `fixed_len() + variable_length()`.
fn buf_len<T: Writer>(data: &T) -> usize {
    data.fixed_len() + data.variable_length()
}

/// `buf_len{suffix}`: [`buf_len`] of `data` alone, then
/// `write_pre{suffix}`, `write_reg{suffix}` and `write{suffix}`:
/// [`Writer::encode_pre`], [`Writer::encode_reg`] and [`Writer::encode`] of
/// `data`. Returns the `encode` buffer for the read benches to use.
fn bench_write<T: Writer>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    suffix: &str,
    param: Option<&dyn Display>,
    data: &T,
) -> PyroBuf {
    group.bench_function(id(&format!("buf_len{suffix}"), param), |b| {
        b.iter(|| buf_len(black_box(data)))
    });
    group.bench_function(id(&format!("write{suffix}"), param), |b| {
        b.iter(|| black_box(data).encode().unwrap())
    });
    data.encode().unwrap()
}

/// `read_ref{suffix}`: the borrowed view alone.
fn bench_read_ref<T: Reader>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    suffix: &str,
    param: Option<&dyn Display>,
    buf: &[u8],
) {
    group.bench_function(id(&format!("read_ref{suffix}"), param), |b| {
        b.iter(|| black_box(T::pyro_read(black_box(buf)).unwrap()))
    });
}

/// The standard pyroformat set: `write`, `read` and `read_ref`.
fn bench_pyro<T: Writer + Reader>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    suffix: &str,
    param: Option<&dyn Display>,
    data: &T,
) {
    let buf = bench_write(group, suffix, param, data);
    group.bench_function(id(&format!("read{suffix}"), param), |b| {
        b.iter(|| black_box(decode::<T>(black_box(&buf))))
    });
    bench_read_ref::<T>(group, suffix, param, &buf);
}

// ── rkyv comparison helpers ─────────────────────────────────────────────────
//
//   - `rkyv_encode` is a brand new serialize every call — the analog of
//     `Writer::encode`. rkyv's own `to_bytes` reuses a thread-local arena
//     internally (see `rkyv::util::with_arena`), so it is already partly
//     warm.
//   - `rkyv_decode` is the full owned-value deserialize — the fair
//     comparison against pyroformat's `read`, which also returns fully
//     owned Rust values.
//   - `rkyv_access` is validated, zero-copy access to the archived bytes
//     with no deserialization at all — the comparison against `read_ref`.
fn rkyv_encode<T>(data: &T) -> AlignedVec
where
    T: for<'a> RkSerialize<HighSerializer<AlignedVec, ArenaHandle<'a>, RkError>>,
{
    rkyv::to_bytes::<RkError>(data).unwrap()
}

fn rkyv_decode<T>(bytes: &[u8]) -> T
where
    T: RkArchive,
    T::Archived: for<'a> rkyv::bytecheck::CheckBytes<HighValidator<'a, RkError>>
        + RkDeserialize<T, HighDeserializer<RkError>>,
{
    rkyv::from_bytes::<T, RkError>(bytes).unwrap()
}

fn rkyv_access<T>(bytes: &[u8]) -> &T::Archived
where
    T: RkArchive,
    T::Archived: RkPortable + for<'a> rkyv::bytecheck::CheckBytes<HighValidator<'a, RkError>>,
{
    rkyv::access::<T::Archived, RkError>(bytes).unwrap()
}

/// `write_rkyv`: [`rkyv_encode`] of `data`. Returns the archived bytes.
fn bench_rkyv_write<T>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    param: Option<&dyn Display>,
    data: &T,
) -> AlignedVec
where
    T: for<'a> RkSerialize<HighSerializer<AlignedVec, ArenaHandle<'a>, RkError>>,
{
    group.bench_function(id("write_rkyv", param), |b| {
        b.iter(|| rkyv_encode(black_box(data)))
    });
    rkyv_encode(data)
}

/// The standard rkyv set: `write_rkyv`, `read_rkyv` and `read_rkyv_access`.
fn bench_rkyv<T>(group: &mut BenchmarkGroup<'_, WallTime>, param: Option<&dyn Display>, data: &T)
where
    T: RkArchive + for<'a> RkSerialize<HighSerializer<AlignedVec, ArenaHandle<'a>, RkError>>,
    T::Archived: RkPortable
        + for<'a> rkyv::bytecheck::CheckBytes<HighValidator<'a, RkError>>
        + RkDeserialize<T, HighDeserializer<RkError>>,
{
    let rk_buf = bench_rkyv_write(group, param, data);
    group.bench_function(id("read_rkyv", param), |b| {
        b.iter(|| black_box(rkyv_decode::<T>(black_box(&rk_buf))))
    });
    group.bench_function(id("read_rkyv_access", param), |b| {
        b.iter(|| black_box(rkyv_access::<T>(black_box(&rk_buf))))
    });
}

// ── The structs ─────────────────────────────────────────────────────────────

#[derive(Layout, Writer, Reader, Debug, PartialEq, Clone)]
#[pyro(Word = u32)]
struct Simple {
    id: u32,
    count: i32,
    active: bool,
    score: f64,
}

#[derive(RkArchive, RkSerialize, RkDeserialize, Debug, PartialEq, Clone)]
struct RkSimple {
    id: u32,
    count: i32,
    active: bool,
    score: f64,
}

fn to_rk_simple(v: &Simple) -> RkSimple {
    RkSimple {
        id: v.id,
        count: v.count,
        active: v.active,
        score: v.score,
    }
}

#[derive(Layout, Writer, Reader, Debug, PartialEq, Clone)]
#[pyro(Word = u32)]
struct WithStrings {
    name: String,
    tag: String,
}

#[derive(RkArchive, RkSerialize, RkDeserialize, Debug, PartialEq, Clone)]
struct RkWithStrings {
    name: String,
    tag: String,
}

fn to_rk_with_strings(v: &WithStrings) -> RkWithStrings {
    RkWithStrings {
        name: v.name.clone(),
        tag: v.tag.clone(),
    }
}

#[derive(Layout, Writer, Reader, Debug, PartialEq, Clone)]
#[pyro(Word = u32)]
struct WithVecs {
    data_u32: Vec<u32>,
    data_i64: Vec<i64>,
    // Deliberately `Vec<bool>`: `WithBitVec` below is the packed sibling.
    #[pyro(allow(vec_bool))]
    flags: Vec<bool>,
}

#[derive(RkArchive, RkSerialize, RkDeserialize, Debug, PartialEq, Clone)]
struct RkWithVecs {
    data_u32: Vec<u32>,
    data_i64: Vec<i64>,
    flags: Vec<bool>,
}

fn to_rk_with_vecs(v: &WithVecs) -> RkWithVecs {
    RkWithVecs {
        data_u32: v.data_u32.clone(),
        data_i64: v.data_i64.clone(),
        flags: v.flags.clone(),
    }
}

// `WithVecs`'s `flags: Vec<bool>` sibling, but with `pyroformat::BitVec`
// instead — same fields otherwise, so `bench_vecs`'s `write`/`write_bitvec`
// comparison isolates exactly the one difference: a `Vec<bool>` is packed
// bit by bit on write, where a `BitVec` is already packed and writes with one
// copy.
#[derive(Layout, Writer, Reader, Debug, PartialEq, Clone)]
#[pyro(Word = u32)]
struct WithBitVec {
    data_u32: Vec<u32>,
    data_i64: Vec<i64>,
    flags: BitVec,
}

fn bits_from_bools(bools: &[bool]) -> BitVec {
    let mut bv = BitVec::with_capacity(bools.len());
    for b in bools {
        bv.push(*b);
    }
    bv
}

// `Address`, `Outer`, `Person` and `Company` nest into each other:
// `Company.employees: Vec<Person>`, `Person.address: Vec<Address>`,
// `Outer.inner: Address`. A nested struct is just another field type with
// its own `ForeignWriter`/`ForeignReader`, so each is an ordinary derive.
#[derive(Layout, Writer, Reader, Debug, PartialEq, Clone)]
#[pyro(Word = u32)]
struct Address {
    street: String,
    city: String,
    zip: u32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq, Clone)]
#[pyro(Word = u32)]
struct Outer {
    inner: Address,
    count: u32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq, Clone)]
#[pyro(Word = u32)]
struct Person {
    name: String,
    age: i32,
    address: Vec<Address>,
}

// Two levels of `Vec<Struct>` nesting: `Company.employees: Vec<Person>`
// where `Person` itself has `address: Vec<Address>`.
#[derive(Layout, Writer, Reader, Debug, PartialEq, Clone)]
#[pyro(Word = u32)]
struct Company {
    name: String,
    founded_year: u32,
    employees: Vec<Person>,
    revenue: f64,
}

#[derive(RkArchive, RkSerialize, RkDeserialize, Debug, PartialEq, Clone)]
struct RkAddress {
    street: String,
    city: String,
    zip: u32,
}

fn to_rk_address(v: &Address) -> RkAddress {
    RkAddress {
        street: v.street.clone(),
        city: v.city.clone(),
        zip: v.zip,
    }
}

#[derive(RkArchive, RkSerialize, RkDeserialize, Debug, PartialEq, Clone)]
struct RkOuter {
    inner: RkAddress,
    count: u32,
}

fn to_rk_outer(v: &Outer) -> RkOuter {
    RkOuter {
        inner: to_rk_address(&v.inner),
        count: v.count,
    }
}

// Deep *primitive* nesting (three `Vec`s around a `u32`), not a
// `Vec<Struct>`.
#[derive(Layout, Writer, Reader, Debug, PartialEq, Clone)]
#[pyro(Word = u32)]
struct TripleNested {
    id: u32,
    cube: Vec<Vec<Vec<u32>>>,
}

#[derive(RkArchive, RkSerialize, RkDeserialize, Debug, PartialEq, Clone)]
struct RkTripleNested {
    id: u32,
    cube: Vec<Vec<Vec<u32>>>,
}

fn to_rk_triple_nested(v: &TripleNested) -> RkTripleNested {
    RkTripleNested {
        id: v.id,
        cube: v.cube.clone(),
    }
}

fn make_cube(dim: usize) -> Vec<Vec<Vec<u32>>> {
    (0..dim)
        .map(|i| {
            (0..dim)
                .map(|j| {
                    (0..dim)
                        .map(|k| (i * dim * dim + j * dim + k) as u32)
                        .collect()
                })
                .collect()
        })
        .collect()
}

#[derive(RkArchive, RkSerialize, RkDeserialize, Debug, PartialEq, Clone)]
struct RkPerson {
    name: String,
    age: i32,
    address: Vec<RkAddress>,
}

fn to_rk_person(v: &Person) -> RkPerson {
    RkPerson {
        name: v.name.clone(),
        age: v.age,
        address: v.address.iter().map(to_rk_address).collect(),
    }
}

#[derive(Layout, Writer, Reader, Debug, PartialEq, Clone)]
#[pyro(Word = u32)]
struct Sensor {
    id: u64,
    label: String,
}

// Mixes a `Vec<Struct>` field with sibling `BitVec`/`Vec<String>` fields —
// stresses the writer's field interleaving beyond a single vec-of-struct.
#[derive(Layout, Writer, Reader, Debug, PartialEq, Clone)]
#[pyro(Word = u32)]
struct SensorReading {
    timestamp: u64,
    value: f64,
    sensor: Vec<Sensor>,
    flags: BitVec,
    tags: Vec<String>,
    quality: i32,
}

#[derive(RkArchive, RkSerialize, RkDeserialize, Debug, PartialEq, Clone)]
struct RkSensor {
    id: u64,
    label: String,
}

fn to_rk_sensor(v: &Sensor) -> RkSensor {
    RkSensor {
        id: v.id,
        label: v.label.clone(),
    }
}

#[derive(RkArchive, RkSerialize, RkDeserialize, Debug, PartialEq, Clone)]
struct RkSensorReading {
    timestamp: u64,
    value: f64,
    sensor: Vec<RkSensor>,
    flags: Vec<bool>,
    tags: Vec<String>,
    quality: i32,
}

fn to_rk_sensor_reading(v: &SensorReading) -> RkSensorReading {
    RkSensorReading {
        timestamp: v.timestamp,
        value: v.value,
        sensor: v.sensor.iter().map(to_rk_sensor).collect(),
        // rkyv has no packed-bitmap type, so its twin stays `Vec<bool>` and
        // the comparison unpacks here — the same asymmetry `RkWideRow` has
        // for `f16`. The unpacking cost is outside the measured region.
        flags: v.flags.iter().map(|b| *b).collect(),
        tags: v.tags.clone(),
        quality: v.quality,
    }
}

#[derive(RkArchive, RkSerialize, RkDeserialize, Debug, PartialEq, Clone)]
struct RkCompany {
    name: String,
    founded_year: u32,
    employees: Vec<RkPerson>,
    revenue: f64,
}

fn to_rk_company(v: &Company) -> RkCompany {
    RkCompany {
        name: v.name.clone(),
        founded_year: v.founded_year,
        employees: v.employees.iter().map(to_rk_person).collect(),
        revenue: v.revenue,
    }
}

fn make_addresses(n: usize) -> Vec<Address> {
    (0..n)
        .map(|i| Address {
            street: format!("{i} Main St"),
            city: "Springfield".to_string(),
            zip: 10000 + i as u32,
        })
        .collect()
}

fn make_person(id: usize, num_addresses: usize) -> Person {
    Person {
        name: format!("Person {id}"),
        age: 20 + (id % 50) as i32,
        address: make_addresses(num_addresses),
    }
}

// A single packet-capture record: fixed-width header fields (timestamp,
// src/dst IP, ports, protocol) plus a variable-length raw payload. Seven
// fixed-width scalars of four different widths, so bucket packing puts them
// nowhere near declaration order.
#[derive(Layout, Writer, Reader, Debug, PartialEq, Clone)]
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

// A capture file: a batch of `PcapRecord`s.
#[derive(Layout, Writer, Reader, Debug, PartialEq, Clone)]
#[pyro(Word = u32)]
struct PcapFile {
    capture_id: u64,
    records: Vec<PcapRecord>,
}

#[derive(RkArchive, RkSerialize, RkDeserialize, Debug, PartialEq, Clone)]
struct RkPcapRecord {
    ts_secs: u64,
    ts_micros: u32,
    src_ip: u32,
    dst_ip: u32,
    src_port: u16,
    dst_port: u16,
    protocol: u8,
    payload: Vec<u8>,
}

fn to_rk_pcap_record(v: &PcapRecord) -> RkPcapRecord {
    RkPcapRecord {
        ts_secs: v.ts_secs,
        ts_micros: v.ts_micros,
        src_ip: v.src_ip,
        dst_ip: v.dst_ip,
        src_port: v.src_port,
        dst_port: v.dst_port,
        protocol: v.protocol,
        payload: v.payload.clone(),
    }
}

fn make_pcap_record(seq: u64, payload_len: usize) -> PcapRecord {
    PcapRecord {
        ts_secs: 1_700_000_000 + seq,
        ts_micros: (seq % 1_000_000) as u32,
        src_ip: 0xC0A8_0001,
        dst_ip: 0x0808_0808,
        src_port: 443,
        dst_port: (1024 + seq % 60_000) as u16,
        protocol: 6, // TCP
        payload: (0..payload_len)
            .map(|i| ((seq as usize + i) % 256) as u8)
            .collect(),
    }
}

#[derive(RkArchive, RkSerialize, RkDeserialize, Debug, PartialEq, Clone)]
struct RkPcapFile {
    capture_id: u64,
    records: Vec<RkPcapRecord>,
}

fn to_rk_pcap_file(v: &PcapFile) -> RkPcapFile {
    RkPcapFile {
        capture_id: v.capture_id,
        records: v.records.iter().map(to_rk_pcap_record).collect(),
    }
}

// Eight nullable scalar fields, exercising the validity-bitmap read/write
// path — untouched by every other group, which are all fields that are
// always present.
#[derive(Layout, Writer, Reader, Debug, PartialEq, Clone)]
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

#[derive(RkArchive, RkSerialize, RkDeserialize, Debug, PartialEq, Clone)]
struct RkNullableRow {
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

fn to_rk_nullable_row(v: &NullableRow) -> RkNullableRow {
    RkNullableRow {
        id: v.id,
        a: v.a,
        b: v.b,
        c: v.c,
        d: v.d,
        e: v.e,
        f: v.f,
        g: v.g,
        h: v.h,
    }
}

fn make_nullable_row(id: u32, null_ratio: f64) -> NullableRow {
    // Deterministic per-field pattern: field index `i` (0-based, over the 8
    // Option fields) is `None` whenever `i as f64 / 8.0 < null_ratio`.
    let keep = |i: usize| (i as f64 / 8.0) >= null_ratio;
    NullableRow {
        id,
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

// Fixed-size array fields (`[T; N]`, not `Vec<T>`), including a nullable one.
#[derive(Layout, Writer, Reader, Debug, PartialEq, Clone)]
#[pyro(Word = u32)]
struct FixedArrays {
    id: u32,
    small: [u32; 4],
    large: [f64; 64],
    maybe: Option<[i64; 8]>,
}

#[derive(RkArchive, RkSerialize, RkDeserialize, Debug, PartialEq, Clone)]
struct RkFixedArrays {
    id: u32,
    small: [u32; 4],
    large: [f64; 64],
    maybe: Option<[i64; 8]>,
}

fn to_rk_fixed_arrays(v: &FixedArrays) -> RkFixedArrays {
    RkFixedArrays {
        id: v.id,
        small: v.small,
        large: v.large,
        maybe: v.maybe,
    }
}

// A raw `Vec<u8>` field two ways: `blob` is the default, `Bytes`;
// `samples` is `#[pyro(numeric)]`-tagged, so it is a list of `u8` like any
// other `Vec<T>` primitive — two different schemas for the same Rust type.
#[derive(Layout, Writer, Reader, Debug, PartialEq, Clone)]
#[pyro(Word = u32)]
struct RawBytes {
    id: u32,
    blob: Vec<u8>,
    #[pyro(numeric)]
    samples: Vec<u8>,
}

// rkyv makes no distinction between "opaque blob" and "numeric slice" —
// `Vec<u8>` is `Vec<u8>` either way.
#[derive(RkArchive, RkSerialize, RkDeserialize, Debug, PartialEq, Clone)]
struct RkRawBytes {
    id: u32,
    blob: Vec<u8>,
    samples: Vec<u8>,
}

fn to_rk_raw_bytes(v: &RawBytes) -> RkRawBytes {
    RkRawBytes {
        id: v.id,
        blob: v.blob.clone(),
        samples: v.samples.clone(),
    }
}

fn make_raw_bytes(len: usize) -> RawBytes {
    let data: Vec<u8> = (0..len).map(|i| (i % 256) as u8).collect();
    RawBytes {
        id: 1,
        blob: data.clone(),
        samples: data,
    }
}

// A "wide" struct: many scalar fields, no nesting and no collections at
// all. Every other group scales *element count* (vec/list length); this
// scales *field count* (struct arity) instead, isolating per-field overhead
// from any per-element work. Also the only place a scalar `half::f16` and a
// scalar `pyroformat::value::Time` are exercised.
#[derive(Layout, Writer, Reader, Debug, PartialEq, Clone)]
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

// rkyv has no built-in support for the external `half` crate, so `reading`
// is represented by its raw bit pattern (`u16`) instead — same byte width,
// same wire cost, just without a semantic wrapper type on the rkyv side.
// `Time` (pyroformat's own type) is skipped entirely rather than
// substituted — it's a pyroformat-specific concept with no useful rkyv
// analog to compare against.
#[derive(RkArchive, RkSerialize, RkDeserialize, Debug, PartialEq, Clone)]
struct RkWideRow {
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
    reading_bits: u16,
}

fn to_rk_wide_row(v: &WideRow) -> RkWideRow {
    RkWideRow {
        f0: v.f0,
        f1: v.f1,
        f2: v.f2,
        f3: v.f3,
        f4: v.f4,
        f5: v.f5,
        f6: v.f6,
        f7: v.f7,
        f8: v.f8,
        f9: v.f9,
        f10: v.f10,
        f11: v.f11,
        f12: v.f12,
        f13: v.f13,
        f14: v.f14,
        f15: v.f15,
        f16_: v.f16_,
        f17: v.f17,
        f18: v.f18,
        f19: v.f19,
        f20: v.f20,
        f21: v.f21,
        f22: v.f22,
        f23: v.f23,
        f24: v.f24,
        f25: v.f25,
        f26: v.f26,
        f27: v.f27,
        f28: v.f28,
        f29: v.f29,
        f30: v.f30,
        f31: v.f31,
        reading_bits: v.reading.to_bits(),
    }
}

fn make_wide_row() -> WideRow {
    WideRow {
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
    }
}

// A `BTreeMap<String, String>` field — the only collection type in this file
// that isn't a `Vec<T>`. Scales entry count the same way `bench_vecs` scales
// element count.
#[derive(Layout, Writer, Reader, Debug, PartialEq, Clone)]
#[pyro(Word = u32)]
struct WithMap {
    id: u32,
    tags: BTreeMap<String, String>,
}

#[derive(RkArchive, RkSerialize, RkDeserialize, Debug, PartialEq, Clone)]
struct RkWithMap {
    id: u32,
    tags: BTreeMap<String, String>,
}

fn to_rk_with_map(v: &WithMap) -> RkWithMap {
    RkWithMap {
        id: v.id,
        tags: v.tags.clone(),
    }
}

fn make_with_map(len: usize) -> WithMap {
    WithMap {
        id: 1,
        tags: (0..len)
            .map(|i| (format!("key-{i}"), format!("value-{i}")))
            .collect(),
    }
}

// ── The groups ──────────────────────────────────────────────────────────────

fn bench_simple(c: &mut Criterion) {
    let data = Simple {
        id: 42,
        count: -7,
        active: true,
        score: 3.14,
    };

    let mut group = c.benchmark_group("simple");
    bench_pyro(&mut group, "", None, &data);
    bench_rkyv(&mut group, None, &to_rk_simple(&data));
    group.finish();
}

fn bench_strings(c: &mut Criterion) {
    let data = WithStrings {
        name: "hello world".to_string(),
        tag: "greeting".to_string(),
    };

    let mut group = c.benchmark_group("strings");
    bench_pyro(&mut group, "", None, &data);

    let rk_data = to_rk_with_strings(&data);
    bench_rkyv(&mut group, None, &rk_data);
    // The like-for-like comparison against `read_ref`.
    //
    // `read_rkyv_access` validates the archive and hands back a
    // `&ArchivedRkWithStrings` — it never decodes a field. `read_ref`
    // validates *and* produces two native `&str`s. Comparing those two
    // directly flatters rkyv by the cost of everything it hasn't done yet,
    // so this variant does the field materialization as well.
    let rk_buf = rkyv_encode(&rk_data);
    group.bench_function("read_rkyv_access_fields", |b| {
        b.iter(|| {
            let a = rkyv_access::<RkWithStrings>(black_box(&rk_buf));
            black_box((a.name.as_str(), a.tag.as_str()))
        })
    });
    group.finish();
}

fn bench_vecs(c: &mut Criterion) {
    let mut group = c.benchmark_group("vecs");
    for &len in &[8usize, 64, 512] {
        let data = WithVecs {
            data_u32: (0..len as u32).collect(),
            data_i64: (0..len as i64).collect(),
            flags: (0..len).map(|i| i % 2 == 0).collect(),
        };
        bench_pyro(&mut group, "", Some(&len), &data);
        bench_rkyv(&mut group, Some(&len), &to_rk_with_vecs(&data));

        // `pyroformat::BitVec` sibling — same `data_u32`/`data_i64`, but
        // `flags` is pre-packed instead of `Vec<bool>`.
        let bitvec_data = WithBitVec {
            data_u32: data.data_u32.clone(),
            data_i64: data.data_i64.clone(),
            flags: bits_from_bools(&data.flags),
        };
        bench_pyro(&mut group, "_bitvec", Some(&len), &bitvec_data);
    }
    group.finish();
}

fn bench_nested(c: &mut Criterion) {
    let data = Outer {
        inner: Address {
            street: "Main St".to_string(),
            city: "Springfield".to_string(),
            zip: 12345,
        },
        count: 7,
    };

    let mut group = c.benchmark_group("nested");
    bench_pyro(&mut group, "", None, &data);
    bench_rkyv(&mut group, None, &to_rk_outer(&data));
    group.finish();
}

fn bench_triple_nested_vec(c: &mut Criterion) {
    let mut group = c.benchmark_group("triple_nested_vec");
    // Uniform cube of `dim^3` u32 leaves — id'd by `dim` since that's what
    // varies, not the (much larger) element count.
    for &dim in &[4usize, 8, 16] {
        let data = TripleNested {
            id: 1,
            cube: make_cube(dim),
        };
        bench_pyro(&mut group, "", Some(&dim), &data);
        bench_rkyv(&mut group, Some(&dim), &to_rk_triple_nested(&data));
    }
    group.finish();
}

fn bench_nested_vec_struct(c: &mut Criterion) {
    let mut group = c.benchmark_group("nested_vec_struct");
    for &len in &[8usize, 64, 256] {
        let data = make_person(0, len);
        let buf = bench_write(&mut group, "", Some(&len), &data);
        group.bench_function(id("read", Some(&len)), |b| {
            b.iter(|| {
                let person = Person::pyro_read(black_box(&buf)).unwrap();
                black_box(Realize::<Vec<Address>>::realize(&person.address))
            })
        });
        bench_read_ref::<Person>(&mut group, "", Some(&len), &buf);

        // rkyv has no notion of "read just this field" — the closest analog
        // is decoding/accessing the whole struct and pulling the field out.
        let rk_buf = bench_rkyv_write(&mut group, Some(&len), &to_rk_person(&data));
        group.bench_function(id("read_rkyv", Some(&len)), |b| {
            b.iter(|| black_box(rkyv_decode::<RkPerson>(black_box(&rk_buf)).address))
        });
        group.bench_function(id("read_rkyv_access", Some(&len)), |b| {
            b.iter(|| black_box(&rkyv_access::<RkPerson>(black_box(&rk_buf)).address))
        });
    }
    group.finish();
}

fn bench_mixed_nested(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_nested");
    for &len in &[8usize, 64, 256] {
        let data = SensorReading {
            timestamp: 1_700_000_000,
            value: 98.6,
            sensor: (0..len)
                .map(|i| Sensor {
                    id: i as u64,
                    label: format!("sensor-{i}"),
                })
                .collect(),
            flags: (0..len).map(|i| i % 2 == 0).collect(),
            tags: (0..len).map(|i| format!("tag-{i}")).collect(),
            quality: -3,
        };
        let buf = bench_write(&mut group, "", Some(&len), &data);
        group.bench_function(id("read_sensor_field", Some(&len)), |b| {
            b.iter(|| {
                let reading = SensorReading::pyro_read(black_box(&buf)).unwrap();
                black_box(Realize::<Vec<Sensor>>::realize(&reading.sensor))
            })
        });
        bench_read_ref::<SensorReading>(&mut group, "", Some(&len), &buf);

        let rk_buf = bench_rkyv_write(&mut group, Some(&len), &to_rk_sensor_reading(&data));
        group.bench_function(id("read_sensor_field_rkyv", Some(&len)), |b| {
            b.iter(|| black_box(rkyv_decode::<RkSensorReading>(black_box(&rk_buf)).sensor))
        });
        group.bench_function(id("read_sensor_field_rkyv_access", Some(&len)), |b| {
            b.iter(|| black_box(&rkyv_access::<RkSensorReading>(black_box(&rk_buf)).sensor))
        });
    }
    group.finish();
}

fn bench_deep_nested_vec_struct(c: &mut Criterion) {
    let mut group = c.benchmark_group("deep_nested_vec_struct");
    // Fixed 3 addresses per employee; vary the employee count so the
    // group-of-groups depth stays constant while the outer list grows.
    for &num_employees in &[8usize, 64, 256] {
        let data = Company {
            name: "Acme Corp".to_string(),
            founded_year: 1990,
            employees: (0..num_employees).map(|i| make_person(i, 3)).collect(),
            revenue: 1_234_567.89,
        };
        let param: Option<&dyn Display> = Some(&num_employees);
        let buf = bench_write(&mut group, "", param, &data);
        group.bench_function(id("read_employees_field", param), |b| {
            b.iter(|| {
                let company = Company::pyro_read(black_box(&buf)).unwrap();
                black_box(Realize::<Vec<Person>>::realize(&company.employees))
            })
        });
        bench_read_ref::<Company>(&mut group, "", param, &buf);

        let rk_buf = bench_rkyv_write(&mut group, param, &to_rk_company(&data));
        group.bench_function(id("read_employees_field_rkyv", param), |b| {
            b.iter(|| black_box(rkyv_decode::<RkCompany>(black_box(&rk_buf)).employees))
        });
        group.bench_function(id("read_employees_field_rkyv_access", param), |b| {
            b.iter(|| black_box(&rkyv_access::<RkCompany>(black_box(&rk_buf)).employees))
        });
    }
    group.finish();
}

fn bench_pcap(c: &mut Criterion) {
    let mut group = c.benchmark_group("pcap");
    // Small ACK-sized, typical MTU, and jumbo-frame-ish payloads.
    for &payload_len in &[64usize, 1500, 9000] {
        let data = make_pcap_record(0, payload_len);
        bench_pyro(&mut group, "", Some(&payload_len), &data);
        bench_rkyv(&mut group, Some(&payload_len), &to_rk_pcap_record(&data));
    }
    group.finish();
}

fn bench_multi_pcap(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_pcap");
    // Fixed 256-byte payload per record; vary the batch size — i.e. packets
    // per capture file, not packet size.
    for &num_records in &[16usize, 128, 1024] {
        let data = PcapFile {
            capture_id: 1,
            records: (0..num_records as u64)
                .map(|seq| make_pcap_record(seq, 256))
                .collect(),
        };
        let param: Option<&dyn Display> = Some(&num_records);
        let buf = bench_write(&mut group, "", param, &data);
        group.bench_function(id("read_records_field", param), |b| {
            b.iter(|| {
                let file = PcapFile::pyro_read(black_box(&buf)).unwrap();
                black_box(Realize::<Vec<PcapRecord>>::realize(&file.records))
            })
        });
        bench_read_ref::<PcapFile>(&mut group, "", param, &buf);

        let rk_buf = bench_rkyv_write(&mut group, param, &to_rk_pcap_file(&data));
        group.bench_function(id("read_records_field_rkyv", param), |b| {
            b.iter(|| black_box(rkyv_decode::<RkPcapFile>(black_box(&rk_buf)).records))
        });
        group.bench_function(id("read_records_field_rkyv_access", param), |b| {
            b.iter(|| black_box(&rkyv_access::<RkPcapFile>(black_box(&rk_buf)).records))
        });
    }
    group.finish();
}

fn bench_nullable(c: &mut Criterion) {
    let mut group = c.benchmark_group("nullable");
    for &null_ratio in &[0.0f64, 0.5, 1.0] {
        // Label by percent-null since `0.5` reads oddly in a benchmark name.
        let pct = (null_ratio * 100.0) as u32;
        let data = make_nullable_row(1, null_ratio);
        bench_pyro(&mut group, "", Some(&pct), &data);
        bench_rkyv(&mut group, Some(&pct), &to_rk_nullable_row(&data));
    }
    group.finish();
}

fn bench_fixed_arrays(c: &mut Criterion) {
    let data = FixedArrays {
        id: 1,
        small: [1, 2, 3, 4],
        large: std::array::from_fn(|i| i as f64 * 1.5),
        maybe: Some(std::array::from_fn(|i| i as i64 - 4)),
    };

    let mut group = c.benchmark_group("fixed_arrays");
    bench_pyro(&mut group, "", None, &data);
    bench_rkyv(&mut group, None, &to_rk_fixed_arrays(&data));
    group.finish();
}

fn bench_raw_bytes(c: &mut Criterion) {
    let mut group = c.benchmark_group("raw_bytes");
    for &len in &[64usize, 1500, 9000] {
        let data = make_raw_bytes(len);
        bench_pyro(&mut group, "", Some(&len), &data);
        bench_rkyv(&mut group, Some(&len), &to_rk_raw_bytes(&data));
    }
    group.finish();
}

fn bench_wide(c: &mut Criterion) {
    let data = make_wide_row();

    let mut group = c.benchmark_group("wide");
    bench_pyro(&mut group, "", None, &data);
    bench_rkyv(&mut group, None, &to_rk_wide_row(&data));
    group.finish();
}

fn bench_map(c: &mut Criterion) {
    let mut group = c.benchmark_group("map");
    for &len in &[8usize, 64, 512] {
        let data = make_with_map(len);
        bench_pyro(&mut group, "", Some(&len), &data);
        bench_rkyv(&mut group, Some(&len), &to_rk_with_map(&data));
    }
    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(1000, Output::Flamegraph(None)));
    targets = bench_simple, bench_strings, bench_vecs, bench_nested,
        bench_triple_nested_vec, bench_nested_vec_struct, bench_mixed_nested,
        bench_deep_nested_vec_struct, bench_pcap, bench_multi_pcap,
        bench_nullable, bench_fixed_arrays, bench_raw_bytes, bench_wide,
        bench_map
);
criterion_main!(benches);
