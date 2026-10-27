//! `Vec<Vec<Vec<Option<[T; N]>>>>` for every primitive `T` and
//! `N` in [1, 3, 16]: as a row, as a nested group, as `Option<Group>`,
//! as `Vec<Group>`, and as a map value.

use std::collections::BTreeMap;

use half::f16;
use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OptionFixedDepth3W32 {
    u8_1: Vec<Vec<Vec<Option<[u8; 1]>>>>,
    u8_3: Vec<Vec<Vec<Option<[u8; 3]>>>>,
    u8_16: Vec<Vec<Vec<Option<[u8; 16]>>>>,
    i8_1: Vec<Vec<Vec<Option<[i8; 1]>>>>,
    i8_3: Vec<Vec<Vec<Option<[i8; 3]>>>>,
    i8_16: Vec<Vec<Vec<Option<[i8; 16]>>>>,
    u16_1: Vec<Vec<Vec<Option<[u16; 1]>>>>,
    u16_3: Vec<Vec<Vec<Option<[u16; 3]>>>>,
    u16_16: Vec<Vec<Vec<Option<[u16; 16]>>>>,
    i16_1: Vec<Vec<Vec<Option<[i16; 1]>>>>,
    i16_3: Vec<Vec<Vec<Option<[i16; 3]>>>>,
    i16_16: Vec<Vec<Vec<Option<[i16; 16]>>>>,
    u32_1: Vec<Vec<Vec<Option<[u32; 1]>>>>,
    u32_3: Vec<Vec<Vec<Option<[u32; 3]>>>>,
    u32_16: Vec<Vec<Vec<Option<[u32; 16]>>>>,
    i32_1: Vec<Vec<Vec<Option<[i32; 1]>>>>,
    i32_3: Vec<Vec<Vec<Option<[i32; 3]>>>>,
    i32_16: Vec<Vec<Vec<Option<[i32; 16]>>>>,
    u64_1: Vec<Vec<Vec<Option<[u64; 1]>>>>,
    u64_3: Vec<Vec<Vec<Option<[u64; 3]>>>>,
    u64_16: Vec<Vec<Vec<Option<[u64; 16]>>>>,
    i64_1: Vec<Vec<Vec<Option<[i64; 1]>>>>,
    i64_3: Vec<Vec<Vec<Option<[i64; 3]>>>>,
    i64_16: Vec<Vec<Vec<Option<[i64; 16]>>>>,
    f16_1: Vec<Vec<Vec<Option<[f16; 1]>>>>,
    f16_3: Vec<Vec<Vec<Option<[f16; 3]>>>>,
    f16_16: Vec<Vec<Vec<Option<[f16; 16]>>>>,
    f32_1: Vec<Vec<Vec<Option<[f32; 1]>>>>,
    f32_3: Vec<Vec<Vec<Option<[f32; 3]>>>>,
    f32_16: Vec<Vec<Vec<Option<[f32; 16]>>>>,
    f64_1: Vec<Vec<Vec<Option<[f64; 1]>>>>,
    f64_3: Vec<Vec<Vec<Option<[f64; 3]>>>>,
    f64_16: Vec<Vec<Vec<Option<[f64; 16]>>>>,
    bool_1: Vec<Vec<Vec<Option<[bool; 1]>>>>,
    bool_3: Vec<Vec<Vec<Option<[bool; 3]>>>>,
    bool_16: Vec<Vec<Vec<Option<[bool; 16]>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct OptionFixedDepth3W16 {
    u8_1: Vec<Vec<Vec<Option<[u8; 1]>>>>,
    u8_3: Vec<Vec<Vec<Option<[u8; 3]>>>>,
    u8_16: Vec<Vec<Vec<Option<[u8; 16]>>>>,
    i8_1: Vec<Vec<Vec<Option<[i8; 1]>>>>,
    i8_3: Vec<Vec<Vec<Option<[i8; 3]>>>>,
    i8_16: Vec<Vec<Vec<Option<[i8; 16]>>>>,
    u16_1: Vec<Vec<Vec<Option<[u16; 1]>>>>,
    u16_3: Vec<Vec<Vec<Option<[u16; 3]>>>>,
    u16_16: Vec<Vec<Vec<Option<[u16; 16]>>>>,
    i16_1: Vec<Vec<Vec<Option<[i16; 1]>>>>,
    i16_3: Vec<Vec<Vec<Option<[i16; 3]>>>>,
    i16_16: Vec<Vec<Vec<Option<[i16; 16]>>>>,
    u32_1: Vec<Vec<Vec<Option<[u32; 1]>>>>,
    u32_3: Vec<Vec<Vec<Option<[u32; 3]>>>>,
    u32_16: Vec<Vec<Vec<Option<[u32; 16]>>>>,
    i32_1: Vec<Vec<Vec<Option<[i32; 1]>>>>,
    i32_3: Vec<Vec<Vec<Option<[i32; 3]>>>>,
    i32_16: Vec<Vec<Vec<Option<[i32; 16]>>>>,
    u64_1: Vec<Vec<Vec<Option<[u64; 1]>>>>,
    u64_3: Vec<Vec<Vec<Option<[u64; 3]>>>>,
    u64_16: Vec<Vec<Vec<Option<[u64; 16]>>>>,
    i64_1: Vec<Vec<Vec<Option<[i64; 1]>>>>,
    i64_3: Vec<Vec<Vec<Option<[i64; 3]>>>>,
    i64_16: Vec<Vec<Vec<Option<[i64; 16]>>>>,
    f16_1: Vec<Vec<Vec<Option<[f16; 1]>>>>,
    f16_3: Vec<Vec<Vec<Option<[f16; 3]>>>>,
    f16_16: Vec<Vec<Vec<Option<[f16; 16]>>>>,
    f32_1: Vec<Vec<Vec<Option<[f32; 1]>>>>,
    f32_3: Vec<Vec<Vec<Option<[f32; 3]>>>>,
    f32_16: Vec<Vec<Vec<Option<[f32; 16]>>>>,
    f64_1: Vec<Vec<Vec<Option<[f64; 1]>>>>,
    f64_3: Vec<Vec<Vec<Option<[f64; 3]>>>>,
    f64_16: Vec<Vec<Vec<Option<[f64; 16]>>>>,
    bool_1: Vec<Vec<Vec<Option<[bool; 1]>>>>,
    bool_3: Vec<Vec<Vec<Option<[bool; 3]>>>>,
    bool_16: Vec<Vec<Vec<Option<[bool; 16]>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OptionFixedDepth3Single32 {
    a: Vec<Vec<Vec<Option<[u32; 2]>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct OptionFixedDepth3Single16 {
    a: Vec<Vec<Vec<Option<[u32; 2]>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OptionFixedDepth3Mixed32 {
    id: u64,
    name: String,
    a: Vec<Vec<Vec<Option<[i16; 4]>>>>,
    tag: Option<u8>,
    b: Vec<Vec<Vec<Option<[f64; 2]>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct OptionFixedDepth3Mixed16 {
    id: u64,
    name: String,
    a: Vec<Vec<Vec<Option<[i16; 4]>>>>,
    tag: Option<u8>,
    b: Vec<Vec<Vec<Option<[f64; 2]>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OptionFixedDepth3InGroup32 {
    a: OptionFixedDepth3Single32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct OptionFixedDepth3InGroup16 {
    a: OptionFixedDepth3Single16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OptionFixedDepth3InOptionGroup32 {
    a: Option<OptionFixedDepth3Single32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct OptionFixedDepth3InOptionGroup16 {
    a: Option<OptionFixedDepth3Single16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OptionFixedDepth3InVecGroup32 {
    a: Vec<OptionFixedDepth3Mixed32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct OptionFixedDepth3InVecGroup16 {
    a: Vec<OptionFixedDepth3Mixed16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct OptionFixedDepth3InMapValue32 {
    a: BTreeMap<String, OptionFixedDepth3Single32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct OptionFixedDepth3InMapValue16 {
    a: BTreeMap<String, OptionFixedDepth3Single16>,
}

fn main() {
    assert_pyro_traits::<OptionFixedDepth3W32>();
    assert_pyro_traits::<OptionFixedDepth3W16>();
    assert_pyro_traits::<OptionFixedDepth3Single32>();
    assert_pyro_traits::<OptionFixedDepth3Single16>();
    assert_pyro_traits::<OptionFixedDepth3Mixed32>();
    assert_pyro_traits::<OptionFixedDepth3Mixed16>();
    assert_pyro_traits::<OptionFixedDepth3InGroup32>();
    assert_pyro_traits::<OptionFixedDepth3InGroup16>();
    assert_pyro_traits::<OptionFixedDepth3InOptionGroup32>();
    assert_pyro_traits::<OptionFixedDepth3InOptionGroup16>();
    assert_pyro_traits::<OptionFixedDepth3InVecGroup32>();
    assert_pyro_traits::<OptionFixedDepth3InVecGroup16>();
    assert_pyro_traits::<OptionFixedDepth3InMapValue32>();
    assert_pyro_traits::<OptionFixedDepth3InMapValue16>();
}
