//! `Vec<Vec<Vec<Vec<Vec<[T; N]>>>>>` for every primitive `T` and
//! `N` in [1, 3, 16]: as a row, as a nested group, as `Option<Group>`,
//! as `Vec<Group>`, and as a map value.

use std::collections::BTreeMap;

use half::f16;
use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FixedDepth5W32 {
    u8_1: Vec<Vec<Vec<Vec<Vec<[u8; 1]>>>>>,
    u8_3: Vec<Vec<Vec<Vec<Vec<[u8; 3]>>>>>,
    u8_16: Vec<Vec<Vec<Vec<Vec<[u8; 16]>>>>>,
    i8_1: Vec<Vec<Vec<Vec<Vec<[i8; 1]>>>>>,
    i8_3: Vec<Vec<Vec<Vec<Vec<[i8; 3]>>>>>,
    i8_16: Vec<Vec<Vec<Vec<Vec<[i8; 16]>>>>>,
    u16_1: Vec<Vec<Vec<Vec<Vec<[u16; 1]>>>>>,
    u16_3: Vec<Vec<Vec<Vec<Vec<[u16; 3]>>>>>,
    u16_16: Vec<Vec<Vec<Vec<Vec<[u16; 16]>>>>>,
    i16_1: Vec<Vec<Vec<Vec<Vec<[i16; 1]>>>>>,
    i16_3: Vec<Vec<Vec<Vec<Vec<[i16; 3]>>>>>,
    i16_16: Vec<Vec<Vec<Vec<Vec<[i16; 16]>>>>>,
    u32_1: Vec<Vec<Vec<Vec<Vec<[u32; 1]>>>>>,
    u32_3: Vec<Vec<Vec<Vec<Vec<[u32; 3]>>>>>,
    u32_16: Vec<Vec<Vec<Vec<Vec<[u32; 16]>>>>>,
    i32_1: Vec<Vec<Vec<Vec<Vec<[i32; 1]>>>>>,
    i32_3: Vec<Vec<Vec<Vec<Vec<[i32; 3]>>>>>,
    i32_16: Vec<Vec<Vec<Vec<Vec<[i32; 16]>>>>>,
    u64_1: Vec<Vec<Vec<Vec<Vec<[u64; 1]>>>>>,
    u64_3: Vec<Vec<Vec<Vec<Vec<[u64; 3]>>>>>,
    u64_16: Vec<Vec<Vec<Vec<Vec<[u64; 16]>>>>>,
    i64_1: Vec<Vec<Vec<Vec<Vec<[i64; 1]>>>>>,
    i64_3: Vec<Vec<Vec<Vec<Vec<[i64; 3]>>>>>,
    i64_16: Vec<Vec<Vec<Vec<Vec<[i64; 16]>>>>>,
    f16_1: Vec<Vec<Vec<Vec<Vec<[f16; 1]>>>>>,
    f16_3: Vec<Vec<Vec<Vec<Vec<[f16; 3]>>>>>,
    f16_16: Vec<Vec<Vec<Vec<Vec<[f16; 16]>>>>>,
    f32_1: Vec<Vec<Vec<Vec<Vec<[f32; 1]>>>>>,
    f32_3: Vec<Vec<Vec<Vec<Vec<[f32; 3]>>>>>,
    f32_16: Vec<Vec<Vec<Vec<Vec<[f32; 16]>>>>>,
    f64_1: Vec<Vec<Vec<Vec<Vec<[f64; 1]>>>>>,
    f64_3: Vec<Vec<Vec<Vec<Vec<[f64; 3]>>>>>,
    f64_16: Vec<Vec<Vec<Vec<Vec<[f64; 16]>>>>>,
    bool_1: Vec<Vec<Vec<Vec<Vec<[bool; 1]>>>>>,
    bool_3: Vec<Vec<Vec<Vec<Vec<[bool; 3]>>>>>,
    bool_16: Vec<Vec<Vec<Vec<Vec<[bool; 16]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct FixedDepth5W16 {
    u8_1: Vec<Vec<Vec<Vec<Vec<[u8; 1]>>>>>,
    u8_3: Vec<Vec<Vec<Vec<Vec<[u8; 3]>>>>>,
    u8_16: Vec<Vec<Vec<Vec<Vec<[u8; 16]>>>>>,
    i8_1: Vec<Vec<Vec<Vec<Vec<[i8; 1]>>>>>,
    i8_3: Vec<Vec<Vec<Vec<Vec<[i8; 3]>>>>>,
    i8_16: Vec<Vec<Vec<Vec<Vec<[i8; 16]>>>>>,
    u16_1: Vec<Vec<Vec<Vec<Vec<[u16; 1]>>>>>,
    u16_3: Vec<Vec<Vec<Vec<Vec<[u16; 3]>>>>>,
    u16_16: Vec<Vec<Vec<Vec<Vec<[u16; 16]>>>>>,
    i16_1: Vec<Vec<Vec<Vec<Vec<[i16; 1]>>>>>,
    i16_3: Vec<Vec<Vec<Vec<Vec<[i16; 3]>>>>>,
    i16_16: Vec<Vec<Vec<Vec<Vec<[i16; 16]>>>>>,
    u32_1: Vec<Vec<Vec<Vec<Vec<[u32; 1]>>>>>,
    u32_3: Vec<Vec<Vec<Vec<Vec<[u32; 3]>>>>>,
    u32_16: Vec<Vec<Vec<Vec<Vec<[u32; 16]>>>>>,
    i32_1: Vec<Vec<Vec<Vec<Vec<[i32; 1]>>>>>,
    i32_3: Vec<Vec<Vec<Vec<Vec<[i32; 3]>>>>>,
    i32_16: Vec<Vec<Vec<Vec<Vec<[i32; 16]>>>>>,
    u64_1: Vec<Vec<Vec<Vec<Vec<[u64; 1]>>>>>,
    u64_3: Vec<Vec<Vec<Vec<Vec<[u64; 3]>>>>>,
    u64_16: Vec<Vec<Vec<Vec<Vec<[u64; 16]>>>>>,
    i64_1: Vec<Vec<Vec<Vec<Vec<[i64; 1]>>>>>,
    i64_3: Vec<Vec<Vec<Vec<Vec<[i64; 3]>>>>>,
    i64_16: Vec<Vec<Vec<Vec<Vec<[i64; 16]>>>>>,
    f16_1: Vec<Vec<Vec<Vec<Vec<[f16; 1]>>>>>,
    f16_3: Vec<Vec<Vec<Vec<Vec<[f16; 3]>>>>>,
    f16_16: Vec<Vec<Vec<Vec<Vec<[f16; 16]>>>>>,
    f32_1: Vec<Vec<Vec<Vec<Vec<[f32; 1]>>>>>,
    f32_3: Vec<Vec<Vec<Vec<Vec<[f32; 3]>>>>>,
    f32_16: Vec<Vec<Vec<Vec<Vec<[f32; 16]>>>>>,
    f64_1: Vec<Vec<Vec<Vec<Vec<[f64; 1]>>>>>,
    f64_3: Vec<Vec<Vec<Vec<Vec<[f64; 3]>>>>>,
    f64_16: Vec<Vec<Vec<Vec<Vec<[f64; 16]>>>>>,
    bool_1: Vec<Vec<Vec<Vec<Vec<[bool; 1]>>>>>,
    bool_3: Vec<Vec<Vec<Vec<Vec<[bool; 3]>>>>>,
    bool_16: Vec<Vec<Vec<Vec<Vec<[bool; 16]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FixedDepth5Single32 {
    a: Vec<Vec<Vec<Vec<Vec<[u32; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct FixedDepth5Single16 {
    a: Vec<Vec<Vec<Vec<Vec<[u32; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FixedDepth5Mixed32 {
    id: u64,
    name: String,
    a: Vec<Vec<Vec<Vec<Vec<[i16; 4]>>>>>,
    tag: Option<u8>,
    b: Vec<Vec<Vec<Vec<Vec<[f64; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct FixedDepth5Mixed16 {
    id: u64,
    name: String,
    a: Vec<Vec<Vec<Vec<Vec<[i16; 4]>>>>>,
    tag: Option<u8>,
    b: Vec<Vec<Vec<Vec<Vec<[f64; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FixedDepth5InGroup32 {
    a: FixedDepth5Single32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct FixedDepth5InGroup16 {
    a: FixedDepth5Single16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FixedDepth5InOptionGroup32 {
    a: Option<FixedDepth5Single32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct FixedDepth5InOptionGroup16 {
    a: Option<FixedDepth5Single16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FixedDepth5InVecGroup32 {
    a: Vec<FixedDepth5Mixed32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct FixedDepth5InVecGroup16 {
    a: Vec<FixedDepth5Mixed16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct FixedDepth5InMapValue32 {
    a: BTreeMap<String, FixedDepth5Single32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct FixedDepth5InMapValue16 {
    a: BTreeMap<String, FixedDepth5Single16>,
}

fn main() {
    assert_pyro_traits::<FixedDepth5W32>();
    assert_pyro_traits::<FixedDepth5W16>();
    assert_pyro_traits::<FixedDepth5Single32>();
    assert_pyro_traits::<FixedDepth5Single16>();
    assert_pyro_traits::<FixedDepth5Mixed32>();
    assert_pyro_traits::<FixedDepth5Mixed16>();
    assert_pyro_traits::<FixedDepth5InGroup32>();
    assert_pyro_traits::<FixedDepth5InGroup16>();
    assert_pyro_traits::<FixedDepth5InOptionGroup32>();
    assert_pyro_traits::<FixedDepth5InOptionGroup16>();
    assert_pyro_traits::<FixedDepth5InVecGroup32>();
    assert_pyro_traits::<FixedDepth5InVecGroup16>();
    assert_pyro_traits::<FixedDepth5InMapValue32>();
    assert_pyro_traits::<FixedDepth5InMapValue16>();
}
