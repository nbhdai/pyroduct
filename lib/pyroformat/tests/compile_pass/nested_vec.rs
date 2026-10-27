//! Test #[derive(Layout, Writer, Reader)] compiles for Vec nesting, struct nesting, and
//! mixed field types — a compile-only smoke test.

use pyroformat::{Reader, Layout, Writer};

// ── Nested struct in Vec ────────────────────────────────────────────────────

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u32)]
struct Address32 {
    street: String,
    city: String,
    zip: u32,
}

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u16)]
struct Address16 {
    street: String,
    city: String,
    zip: u32,
}

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u32)]
struct Person32 {
    name: String,
    age: i32,
    address: Vec<Address32>,
}

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u16)]
struct Person16 {
    name: String,
    age: i32,
    address: Vec<Address16>,
}

// ── Deep nesting: Vec<struct> in Vec<struct> ────────────────────────────────

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u32)]
struct Company32 {
    name: String,
    founded_year: u32,
    employees: Vec<Person32>,
    revenue: f64,
}

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u16)]
struct Company16 {
    name: String,
    founded_year: u32,
    employees: Vec<Person16>,
    revenue: f64,
}

// ── Every Vec<primitive> type ───────────────────────────────────────────────

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u32)]
struct MixedVecs32 {
    u8_data: Vec<u8>,
    u16_data: Vec<u16>,
    u32_data: Vec<u32>,
    u64_data: Vec<u64>,
    i8_data: Vec<i8>,
    i16_data: Vec<i16>,
    i32_data: Vec<i32>,
    i64_data: Vec<i64>,
    f32_data: Vec<f32>,
    f64_data: Vec<f64>,
    #[pyro(allow(vec_bool))]
    bool_data: Vec<bool>,
    string_data: Vec<String>,
}

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u16)]
struct MixedVecs16 {
    u8_data: Vec<u8>,
    u16_data: Vec<u16>,
    u32_data: Vec<u32>,
    u64_data: Vec<u64>,
    i8_data: Vec<i8>,
    i16_data: Vec<i16>,
    i32_data: Vec<i32>,
    i64_data: Vec<i64>,
    f32_data: Vec<f32>,
    f64_data: Vec<f64>,
    #[pyro(allow(vec_bool))]
    bool_data: Vec<bool>,
    string_data: Vec<String>,
}

// ── Mixed scalars + Vec<struct> + Vec<bool> + Vec<String> ───────────────────

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u32)]
struct Sensor32 {
    id: u64,
    name: String,
}

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u16)]
struct Sensor16 {
    id: u64,
    name: String,
}

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u32)]
struct SensorReading32 {
    timestamp: u64,
    value: f64,
    sensor: Vec<Sensor32>,
    #[pyro(allow(vec_bool))]
    flags: Vec<bool>,
    tags: Vec<String>,
    quality: i32,
}

#[derive(Layout, Writer, Reader)]
#[pyro(Word = u16)]
struct SensorReading16 {
    timestamp: u64,
    value: f64,
    sensor: Vec<Sensor16>,
    #[pyro(allow(vec_bool))]
    flags: Vec<bool>,
    tags: Vec<String>,
    quality: i32,
}

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

fn main() {
    assert_pyro_traits::<Address32>();
    assert_pyro_traits::<Address16>();
    assert_pyro_traits::<Person32>();
    assert_pyro_traits::<Person16>();
    assert_pyro_traits::<Company32>();
    assert_pyro_traits::<Company16>();
    assert_pyro_traits::<MixedVecs32>();
    assert_pyro_traits::<MixedVecs16>();
    assert_pyro_traits::<Sensor32>();
    assert_pyro_traits::<Sensor16>();
    assert_pyro_traits::<SensorReading32>();
    assert_pyro_traits::<SensorReading16>();
}
