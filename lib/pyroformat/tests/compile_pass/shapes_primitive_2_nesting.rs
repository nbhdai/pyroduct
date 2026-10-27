//! Every derived struct shape from `tests/value_match/primitive_2_nesting_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use half::f16;
use pyroformat::value::Time;
use pyroformat::{BitVec, Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2U8W32 {
    a: Vec<Vec<u8>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2U8W16 {
    a: Vec<Vec<u8>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2I8W32 {
    a: Vec<Vec<i8>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2I8W16 {
    a: Vec<Vec<i8>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2U16W32 {
    a: Vec<Vec<u16>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2U16W16 {
    a: Vec<Vec<u16>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2I16W32 {
    a: Vec<Vec<i16>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2I16W16 {
    a: Vec<Vec<i16>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2U32W32 {
    a: Vec<Vec<u32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2U32W16 {
    a: Vec<Vec<u32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2I32W32 {
    a: Vec<Vec<i32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2I32W16 {
    a: Vec<Vec<i32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2U64W32 {
    a: Vec<Vec<u64>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2U64W16 {
    a: Vec<Vec<u64>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2I64W32 {
    a: Vec<Vec<i64>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2I64W16 {
    a: Vec<Vec<i64>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2F16W32 {
    a: Vec<Vec<f16>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2F16W16 {
    a: Vec<Vec<f16>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2F32W32 {
    a: Vec<Vec<f32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2F32W16 {
    a: Vec<Vec<f32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2F64W32 {
    a: Vec<Vec<f64>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2F64W16 {
    a: Vec<Vec<f64>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2Bool32 {
    #[pyro(allow(vec_bool))]
    a: Vec<Vec<bool>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2Bool16 {
    #[pyro(allow(vec_bool))]
    a: Vec<Vec<bool>>,
}

// `PNested2Bool32` with the innermost `Vec<bool>` a packed `BitVec<u8, Lsb0>`.
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2BitVec32 {
    a: Vec<BitVec>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2BitVec16 {
    a: Vec<BitVec>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2Timestamp32 {
    a: Vec<Vec<Time>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2Timestamp16 {
    a: Vec<Vec<Time>>,
}

fn main() {
    assert_pyro_traits::<PNested2U8W32>();
    assert_pyro_traits::<PNested2U8W16>();
    assert_pyro_traits::<PNested2I8W32>();
    assert_pyro_traits::<PNested2I8W16>();
    assert_pyro_traits::<PNested2U16W32>();
    assert_pyro_traits::<PNested2U16W16>();
    assert_pyro_traits::<PNested2I16W32>();
    assert_pyro_traits::<PNested2I16W16>();
    assert_pyro_traits::<PNested2U32W32>();
    assert_pyro_traits::<PNested2U32W16>();
    assert_pyro_traits::<PNested2I32W32>();
    assert_pyro_traits::<PNested2I32W16>();
    assert_pyro_traits::<PNested2U64W32>();
    assert_pyro_traits::<PNested2U64W16>();
    assert_pyro_traits::<PNested2I64W32>();
    assert_pyro_traits::<PNested2I64W16>();
    assert_pyro_traits::<PNested2F16W32>();
    assert_pyro_traits::<PNested2F16W16>();
    assert_pyro_traits::<PNested2F32W32>();
    assert_pyro_traits::<PNested2F32W16>();
    assert_pyro_traits::<PNested2F64W32>();
    assert_pyro_traits::<PNested2F64W16>();
    assert_pyro_traits::<PNested2Bool32>();
    assert_pyro_traits::<PNested2Bool16>();
    assert_pyro_traits::<PNested2BitVec32>();
    assert_pyro_traits::<PNested2BitVec16>();
    assert_pyro_traits::<PNested2Timestamp32>();
    assert_pyro_traits::<PNested2Timestamp16>();
}
