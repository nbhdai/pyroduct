//! Every derived struct shape from `tests/value_match/primitive_1_nesting_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use half::f16;
use pyroformat::value::Time;
use pyroformat::{BitVec, Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1U8W32 {
    a: Vec<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1U8W16 {
    a: Vec<u8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1I8W32 {
    a: Vec<i8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1I8W16 {
    a: Vec<i8>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1U16W32 {
    a: Vec<u16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1U16W16 {
    a: Vec<u16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1I16W32 {
    a: Vec<i16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1I16W16 {
    a: Vec<i16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1U32W32 {
    a: Vec<u32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1U32W16 {
    a: Vec<u32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1I32W32 {
    a: Vec<i32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1I32W16 {
    a: Vec<i32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1U64W32 {
    a: Vec<u64>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1U64W16 {
    a: Vec<u64>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1I64W32 {
    a: Vec<i64>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1I64W16 {
    a: Vec<i64>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1F16W32 {
    a: Vec<f16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1F16W16 {
    a: Vec<f16>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1F32W32 {
    a: Vec<f32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1F32W16 {
    a: Vec<f32>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1F64W32 {
    a: Vec<f64>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1F64W16 {
    a: Vec<f64>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1Bool32 {
    #[pyro(allow(vec_bool))]
    a: Vec<bool>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1Bool16 {
    #[pyro(allow(vec_bool))]
    a: Vec<bool>,
}

// `PNested1Bool32` with the innermost `Vec<bool>` a packed `BitVec<u8, Lsb0>`.
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1BitVec32 {
    a: BitVec,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1BitVec16 {
    a: BitVec,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1Timestamp32 {
    a: Vec<Time>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1Timestamp16 {
    a: Vec<Time>,
}

fn main() {
    assert_pyro_traits::<PNested1U8W32>();
    assert_pyro_traits::<PNested1U8W16>();
    assert_pyro_traits::<PNested1I8W32>();
    assert_pyro_traits::<PNested1I8W16>();
    assert_pyro_traits::<PNested1U16W32>();
    assert_pyro_traits::<PNested1U16W16>();
    assert_pyro_traits::<PNested1I16W32>();
    assert_pyro_traits::<PNested1I16W16>();
    assert_pyro_traits::<PNested1U32W32>();
    assert_pyro_traits::<PNested1U32W16>();
    assert_pyro_traits::<PNested1I32W32>();
    assert_pyro_traits::<PNested1I32W16>();
    assert_pyro_traits::<PNested1U64W32>();
    assert_pyro_traits::<PNested1U64W16>();
    assert_pyro_traits::<PNested1I64W32>();
    assert_pyro_traits::<PNested1I64W16>();
    assert_pyro_traits::<PNested1F16W32>();
    assert_pyro_traits::<PNested1F16W16>();
    assert_pyro_traits::<PNested1F32W32>();
    assert_pyro_traits::<PNested1F32W16>();
    assert_pyro_traits::<PNested1F64W32>();
    assert_pyro_traits::<PNested1F64W16>();
    assert_pyro_traits::<PNested1Bool32>();
    assert_pyro_traits::<PNested1Bool16>();
    assert_pyro_traits::<PNested1BitVec32>();
    assert_pyro_traits::<PNested1BitVec16>();
    assert_pyro_traits::<PNested1Timestamp32>();
    assert_pyro_traits::<PNested1Timestamp16>();
}
