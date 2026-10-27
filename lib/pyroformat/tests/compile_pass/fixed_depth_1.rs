//! `Vec<[T; N]>` for every primitive `T` and
//! `N` in [1, 3, 16]: as a row, as a nested group, as `Option<Group>`,
//! as `Vec<Group>`, and as a map value.

use std::collections::BTreeMap;

use half::f16;
use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FixedDepth1W32 {
    u8_1: Vec<[u8; 1]>,
    u8_3: Vec<[u8; 3]>,
    u8_16: Vec<[u8; 16]>,
    i8_1: Vec<[i8; 1]>,
    i8_3: Vec<[i8; 3]>,
    i8_16: Vec<[i8; 16]>,
    u16_1: Vec<[u16; 1]>,
    u16_3: Vec<[u16; 3]>,
    u16_16: Vec<[u16; 16]>,
    i16_1: Vec<[i16; 1]>,
    i16_3: Vec<[i16; 3]>,
    i16_16: Vec<[i16; 16]>,
    u32_1: Vec<[u32; 1]>,
    u32_3: Vec<[u32; 3]>,
    u32_16: Vec<[u32; 16]>,
    i32_1: Vec<[i32; 1]>,
    i32_3: Vec<[i32; 3]>,
    i32_16: Vec<[i32; 16]>,
    u64_1: Vec<[u64; 1]>,
    u64_3: Vec<[u64; 3]>,
    u64_16: Vec<[u64; 16]>,
    i64_1: Vec<[i64; 1]>,
    i64_3: Vec<[i64; 3]>,
    i64_16: Vec<[i64; 16]>,
    f16_1: Vec<[f16; 1]>,
    f16_3: Vec<[f16; 3]>,
    f16_16: Vec<[f16; 16]>,
    f32_1: Vec<[f32; 1]>,
    f32_3: Vec<[f32; 3]>,
    f32_16: Vec<[f32; 16]>,
    f64_1: Vec<[f64; 1]>,
    f64_3: Vec<[f64; 3]>,
    f64_16: Vec<[f64; 16]>,
    bool_1: Vec<[bool; 1]>,
    bool_3: Vec<[bool; 3]>,
    bool_16: Vec<[bool; 16]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct FixedDepth1W16 {
    u8_1: Vec<[u8; 1]>,
    u8_3: Vec<[u8; 3]>,
    u8_16: Vec<[u8; 16]>,
    i8_1: Vec<[i8; 1]>,
    i8_3: Vec<[i8; 3]>,
    i8_16: Vec<[i8; 16]>,
    u16_1: Vec<[u16; 1]>,
    u16_3: Vec<[u16; 3]>,
    u16_16: Vec<[u16; 16]>,
    i16_1: Vec<[i16; 1]>,
    i16_3: Vec<[i16; 3]>,
    i16_16: Vec<[i16; 16]>,
    u32_1: Vec<[u32; 1]>,
    u32_3: Vec<[u32; 3]>,
    u32_16: Vec<[u32; 16]>,
    i32_1: Vec<[i32; 1]>,
    i32_3: Vec<[i32; 3]>,
    i32_16: Vec<[i32; 16]>,
    u64_1: Vec<[u64; 1]>,
    u64_3: Vec<[u64; 3]>,
    u64_16: Vec<[u64; 16]>,
    i64_1: Vec<[i64; 1]>,
    i64_3: Vec<[i64; 3]>,
    i64_16: Vec<[i64; 16]>,
    f16_1: Vec<[f16; 1]>,
    f16_3: Vec<[f16; 3]>,
    f16_16: Vec<[f16; 16]>,
    f32_1: Vec<[f32; 1]>,
    f32_3: Vec<[f32; 3]>,
    f32_16: Vec<[f32; 16]>,
    f64_1: Vec<[f64; 1]>,
    f64_3: Vec<[f64; 3]>,
    f64_16: Vec<[f64; 16]>,
    bool_1: Vec<[bool; 1]>,
    bool_3: Vec<[bool; 3]>,
    bool_16: Vec<[bool; 16]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FixedDepth1Single32 {
    a: Vec<[u32; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct FixedDepth1Single16 {
    a: Vec<[u32; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FixedDepth1Mixed32 {
    id: u64,
    name: String,
    a: Vec<[i16; 4]>,
    tag: Option<u8>,
    b: Vec<[f64; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct FixedDepth1Mixed16 {
    id: u64,
    name: String,
    a: Vec<[i16; 4]>,
    tag: Option<u8>,
    b: Vec<[f64; 2]>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FixedDepth1InGroup32 {
    a: FixedDepth1Single32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct FixedDepth1InGroup16 {
    a: FixedDepth1Single16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FixedDepth1InOptionGroup32 {
    a: Option<FixedDepth1Single32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct FixedDepth1InOptionGroup16 {
    a: Option<FixedDepth1Single16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FixedDepth1InVecGroup32 {
    a: Vec<FixedDepth1Mixed32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct FixedDepth1InVecGroup16 {
    a: Vec<FixedDepth1Mixed16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FixedDepth1InMapValue32 {
    a: BTreeMap<String, FixedDepth1Single32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct FixedDepth1InMapValue16 {
    a: BTreeMap<String, FixedDepth1Single16>,
}

fn main() {
    assert_pyro_traits::<FixedDepth1W32>();
    assert_pyro_traits::<FixedDepth1W16>();
    assert_pyro_traits::<FixedDepth1Single32>();
    assert_pyro_traits::<FixedDepth1Single16>();
    assert_pyro_traits::<FixedDepth1Mixed32>();
    assert_pyro_traits::<FixedDepth1Mixed16>();
    assert_pyro_traits::<FixedDepth1InGroup32>();
    assert_pyro_traits::<FixedDepth1InGroup16>();
    assert_pyro_traits::<FixedDepth1InOptionGroup32>();
    assert_pyro_traits::<FixedDepth1InOptionGroup16>();
    assert_pyro_traits::<FixedDepth1InVecGroup32>();
    assert_pyro_traits::<FixedDepth1InVecGroup16>();
    assert_pyro_traits::<FixedDepth1InMapValue32>();
    assert_pyro_traits::<FixedDepth1InMapValue16>();
}
