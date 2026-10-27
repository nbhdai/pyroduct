//! Every derived struct shape from `tests/value_match/primitive_3_nesting_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use half::f16;
use pyroformat::value::Time;
use pyroformat::{BitVec, Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3U8W32 {
    a: Vec<Vec<Vec<u8>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested3U8W16 {
    a: Vec<Vec<Vec<u8>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3I8W32 {
    a: Vec<Vec<Vec<i8>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested3I8W16 {
    a: Vec<Vec<Vec<i8>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3U16W32 {
    a: Vec<Vec<Vec<u16>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested3U16W16 {
    a: Vec<Vec<Vec<u16>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3I16W32 {
    a: Vec<Vec<Vec<i16>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested3I16W16 {
    a: Vec<Vec<Vec<i16>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3U32W32 {
    a: Vec<Vec<Vec<u32>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested3U32W16 {
    a: Vec<Vec<Vec<u32>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3I32W32 {
    a: Vec<Vec<Vec<i32>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested3I32W16 {
    a: Vec<Vec<Vec<i32>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3U64W32 {
    a: Vec<Vec<Vec<u64>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested3U64W16 {
    a: Vec<Vec<Vec<u64>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3I64W32 {
    a: Vec<Vec<Vec<i64>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested3I64W16 {
    a: Vec<Vec<Vec<i64>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3F16W32 {
    a: Vec<Vec<Vec<f16>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested3F16W16 {
    a: Vec<Vec<Vec<f16>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3F32W32 {
    a: Vec<Vec<Vec<f32>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested3F32W16 {
    a: Vec<Vec<Vec<f32>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3F64W32 {
    a: Vec<Vec<Vec<f64>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested3F64W16 {
    a: Vec<Vec<Vec<f64>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3Bool32 {
    #[pyro(allow(vec_bool))]
    a: Vec<Vec<Vec<bool>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested3Bool16 {
    #[pyro(allow(vec_bool))]
    a: Vec<Vec<Vec<bool>>>,
}

// `PNested3Bool32` with the innermost `Vec<bool>` a packed `BitVec<u8, Lsb0>`.
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3BitVec32 {
    a: Vec<Vec<BitVec>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested3BitVec16 {
    a: Vec<Vec<BitVec>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3Timestamp32 {
    a: Vec<Vec<Vec<Time>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested3Timestamp16 {
    a: Vec<Vec<Vec<Time>>>,
}

fn main() {
    assert_pyro_traits::<PNested3U8W32>();
    assert_pyro_traits::<PNested3U8W16>();
    assert_pyro_traits::<PNested3I8W32>();
    assert_pyro_traits::<PNested3I8W16>();
    assert_pyro_traits::<PNested3U16W32>();
    assert_pyro_traits::<PNested3U16W16>();
    assert_pyro_traits::<PNested3I16W32>();
    assert_pyro_traits::<PNested3I16W16>();
    assert_pyro_traits::<PNested3U32W32>();
    assert_pyro_traits::<PNested3U32W16>();
    assert_pyro_traits::<PNested3I32W32>();
    assert_pyro_traits::<PNested3I32W16>();
    assert_pyro_traits::<PNested3U64W32>();
    assert_pyro_traits::<PNested3U64W16>();
    assert_pyro_traits::<PNested3I64W32>();
    assert_pyro_traits::<PNested3I64W16>();
    assert_pyro_traits::<PNested3F16W32>();
    assert_pyro_traits::<PNested3F16W16>();
    assert_pyro_traits::<PNested3F32W32>();
    assert_pyro_traits::<PNested3F32W16>();
    assert_pyro_traits::<PNested3F64W32>();
    assert_pyro_traits::<PNested3F64W16>();
    assert_pyro_traits::<PNested3Bool32>();
    assert_pyro_traits::<PNested3Bool16>();
    assert_pyro_traits::<PNested3BitVec32>();
    assert_pyro_traits::<PNested3BitVec16>();
    assert_pyro_traits::<PNested3Timestamp32>();
    assert_pyro_traits::<PNested3Timestamp16>();
}
