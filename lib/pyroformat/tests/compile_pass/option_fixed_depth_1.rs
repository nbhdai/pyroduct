//! `Vec<Option<[T; N]>>` for every primitive `T` and
//! `N` in [1, 3, 16]: as a row, as a nested group, as `Option<Group>`,
//! as `Vec<Group>`, and as a map value.

use std::collections::BTreeMap;

use half::f16;
use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OptionFixedDepth1W32 {
    u8_1: Vec<Option<[u8; 1]>>,
    u8_3: Vec<Option<[u8; 3]>>,
    u8_16: Vec<Option<[u8; 16]>>,
    i8_1: Vec<Option<[i8; 1]>>,
    i8_3: Vec<Option<[i8; 3]>>,
    i8_16: Vec<Option<[i8; 16]>>,
    u16_1: Vec<Option<[u16; 1]>>,
    u16_3: Vec<Option<[u16; 3]>>,
    u16_16: Vec<Option<[u16; 16]>>,
    i16_1: Vec<Option<[i16; 1]>>,
    i16_3: Vec<Option<[i16; 3]>>,
    i16_16: Vec<Option<[i16; 16]>>,
    u32_1: Vec<Option<[u32; 1]>>,
    u32_3: Vec<Option<[u32; 3]>>,
    u32_16: Vec<Option<[u32; 16]>>,
    i32_1: Vec<Option<[i32; 1]>>,
    i32_3: Vec<Option<[i32; 3]>>,
    i32_16: Vec<Option<[i32; 16]>>,
    u64_1: Vec<Option<[u64; 1]>>,
    u64_3: Vec<Option<[u64; 3]>>,
    u64_16: Vec<Option<[u64; 16]>>,
    i64_1: Vec<Option<[i64; 1]>>,
    i64_3: Vec<Option<[i64; 3]>>,
    i64_16: Vec<Option<[i64; 16]>>,
    f16_1: Vec<Option<[f16; 1]>>,
    f16_3: Vec<Option<[f16; 3]>>,
    f16_16: Vec<Option<[f16; 16]>>,
    f32_1: Vec<Option<[f32; 1]>>,
    f32_3: Vec<Option<[f32; 3]>>,
    f32_16: Vec<Option<[f32; 16]>>,
    f64_1: Vec<Option<[f64; 1]>>,
    f64_3: Vec<Option<[f64; 3]>>,
    f64_16: Vec<Option<[f64; 16]>>,
    bool_1: Vec<Option<[bool; 1]>>,
    bool_3: Vec<Option<[bool; 3]>>,
    bool_16: Vec<Option<[bool; 16]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct OptionFixedDepth1W16 {
    u8_1: Vec<Option<[u8; 1]>>,
    u8_3: Vec<Option<[u8; 3]>>,
    u8_16: Vec<Option<[u8; 16]>>,
    i8_1: Vec<Option<[i8; 1]>>,
    i8_3: Vec<Option<[i8; 3]>>,
    i8_16: Vec<Option<[i8; 16]>>,
    u16_1: Vec<Option<[u16; 1]>>,
    u16_3: Vec<Option<[u16; 3]>>,
    u16_16: Vec<Option<[u16; 16]>>,
    i16_1: Vec<Option<[i16; 1]>>,
    i16_3: Vec<Option<[i16; 3]>>,
    i16_16: Vec<Option<[i16; 16]>>,
    u32_1: Vec<Option<[u32; 1]>>,
    u32_3: Vec<Option<[u32; 3]>>,
    u32_16: Vec<Option<[u32; 16]>>,
    i32_1: Vec<Option<[i32; 1]>>,
    i32_3: Vec<Option<[i32; 3]>>,
    i32_16: Vec<Option<[i32; 16]>>,
    u64_1: Vec<Option<[u64; 1]>>,
    u64_3: Vec<Option<[u64; 3]>>,
    u64_16: Vec<Option<[u64; 16]>>,
    i64_1: Vec<Option<[i64; 1]>>,
    i64_3: Vec<Option<[i64; 3]>>,
    i64_16: Vec<Option<[i64; 16]>>,
    f16_1: Vec<Option<[f16; 1]>>,
    f16_3: Vec<Option<[f16; 3]>>,
    f16_16: Vec<Option<[f16; 16]>>,
    f32_1: Vec<Option<[f32; 1]>>,
    f32_3: Vec<Option<[f32; 3]>>,
    f32_16: Vec<Option<[f32; 16]>>,
    f64_1: Vec<Option<[f64; 1]>>,
    f64_3: Vec<Option<[f64; 3]>>,
    f64_16: Vec<Option<[f64; 16]>>,
    bool_1: Vec<Option<[bool; 1]>>,
    bool_3: Vec<Option<[bool; 3]>>,
    bool_16: Vec<Option<[bool; 16]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OptionFixedDepth1Single32 {
    a: Vec<Option<[u32; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct OptionFixedDepth1Single16 {
    a: Vec<Option<[u32; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OptionFixedDepth1Mixed32 {
    id: u64,
    name: String,
    a: Vec<Option<[i16; 4]>>,
    tag: Option<u8>,
    b: Vec<Option<[f64; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct OptionFixedDepth1Mixed16 {
    id: u64,
    name: String,
    a: Vec<Option<[i16; 4]>>,
    tag: Option<u8>,
    b: Vec<Option<[f64; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OptionFixedDepth1InGroup32 {
    a: OptionFixedDepth1Single32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct OptionFixedDepth1InGroup16 {
    a: OptionFixedDepth1Single16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OptionFixedDepth1InOptionGroup32 {
    a: Option<OptionFixedDepth1Single32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct OptionFixedDepth1InOptionGroup16 {
    a: Option<OptionFixedDepth1Single16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OptionFixedDepth1InVecGroup32 {
    a: Vec<OptionFixedDepth1Mixed32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct OptionFixedDepth1InVecGroup16 {
    a: Vec<OptionFixedDepth1Mixed16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OptionFixedDepth1InMapValue32 {
    a: BTreeMap<String, OptionFixedDepth1Single32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct OptionFixedDepth1InMapValue16 {
    a: BTreeMap<String, OptionFixedDepth1Single16>,
}

fn main() {
    assert_pyro_traits::<OptionFixedDepth1W32>();
    assert_pyro_traits::<OptionFixedDepth1W16>();
    assert_pyro_traits::<OptionFixedDepth1Single32>();
    assert_pyro_traits::<OptionFixedDepth1Single16>();
    assert_pyro_traits::<OptionFixedDepth1Mixed32>();
    assert_pyro_traits::<OptionFixedDepth1Mixed16>();
    assert_pyro_traits::<OptionFixedDepth1InGroup32>();
    assert_pyro_traits::<OptionFixedDepth1InGroup16>();
    assert_pyro_traits::<OptionFixedDepth1InOptionGroup32>();
    assert_pyro_traits::<OptionFixedDepth1InOptionGroup16>();
    assert_pyro_traits::<OptionFixedDepth1InVecGroup32>();
    assert_pyro_traits::<OptionFixedDepth1InVecGroup16>();
    assert_pyro_traits::<OptionFixedDepth1InMapValue32>();
    assert_pyro_traits::<OptionFixedDepth1InMapValue16>();
}
