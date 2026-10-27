//! Every derived struct shape from `tests/value_match/primitive_4_nesting_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use half::f16;
use pyroformat::value::Time;
use pyroformat::{BitVec, Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4U8W32 {
    a: Vec<Vec<Vec<Vec<u8>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4U8W16 {
    a: Vec<Vec<Vec<Vec<u8>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4I8W32 {
    a: Vec<Vec<Vec<Vec<i8>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4I8W16 {
    a: Vec<Vec<Vec<Vec<i8>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4U16W32 {
    a: Vec<Vec<Vec<Vec<u16>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4U16W16 {
    a: Vec<Vec<Vec<Vec<u16>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4I16W32 {
    a: Vec<Vec<Vec<Vec<i16>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4I16W16 {
    a: Vec<Vec<Vec<Vec<i16>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4U32W32 {
    a: Vec<Vec<Vec<Vec<u32>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4U32W16 {
    a: Vec<Vec<Vec<Vec<u32>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4I32W32 {
    a: Vec<Vec<Vec<Vec<i32>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4I32W16 {
    a: Vec<Vec<Vec<Vec<i32>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4U64W32 {
    a: Vec<Vec<Vec<Vec<u64>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4U64W16 {
    a: Vec<Vec<Vec<Vec<u64>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4I64W32 {
    a: Vec<Vec<Vec<Vec<i64>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4I64W16 {
    a: Vec<Vec<Vec<Vec<i64>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4F16W32 {
    a: Vec<Vec<Vec<Vec<f16>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4F16W16 {
    a: Vec<Vec<Vec<Vec<f16>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4F32W32 {
    a: Vec<Vec<Vec<Vec<f32>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4F32W16 {
    a: Vec<Vec<Vec<Vec<f32>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4F64W32 {
    a: Vec<Vec<Vec<Vec<f64>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4F64W16 {
    a: Vec<Vec<Vec<Vec<f64>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4Bool32 {
    #[pyro(allow(vec_bool))]
    a: Vec<Vec<Vec<Vec<bool>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4Bool16 {
    #[pyro(allow(vec_bool))]
    a: Vec<Vec<Vec<Vec<bool>>>>,
}

// `PNested4Bool32` with the innermost `Vec<bool>` a packed `BitVec<u8, Lsb0>`.
#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4BitVec32 {
    a: Vec<Vec<Vec<BitVec>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4BitVec16 {
    a: Vec<Vec<Vec<BitVec>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4Timestamp32 {
    a: Vec<Vec<Vec<Vec<Time>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4Timestamp16 {
    a: Vec<Vec<Vec<Vec<Time>>>>,
}

fn main() {
    assert_pyro_traits::<PNested4U8W32>();
    assert_pyro_traits::<PNested4U8W16>();
    assert_pyro_traits::<PNested4I8W32>();
    assert_pyro_traits::<PNested4I8W16>();
    assert_pyro_traits::<PNested4U16W32>();
    assert_pyro_traits::<PNested4U16W16>();
    assert_pyro_traits::<PNested4I16W32>();
    assert_pyro_traits::<PNested4I16W16>();
    assert_pyro_traits::<PNested4U32W32>();
    assert_pyro_traits::<PNested4U32W16>();
    assert_pyro_traits::<PNested4I32W32>();
    assert_pyro_traits::<PNested4I32W16>();
    assert_pyro_traits::<PNested4U64W32>();
    assert_pyro_traits::<PNested4U64W16>();
    assert_pyro_traits::<PNested4I64W32>();
    assert_pyro_traits::<PNested4I64W16>();
    assert_pyro_traits::<PNested4F16W32>();
    assert_pyro_traits::<PNested4F16W16>();
    assert_pyro_traits::<PNested4F32W32>();
    assert_pyro_traits::<PNested4F32W16>();
    assert_pyro_traits::<PNested4F64W32>();
    assert_pyro_traits::<PNested4F64W16>();
    assert_pyro_traits::<PNested4Bool32>();
    assert_pyro_traits::<PNested4Bool16>();
    assert_pyro_traits::<PNested4BitVec32>();
    assert_pyro_traits::<PNested4BitVec16>();
    assert_pyro_traits::<PNested4Timestamp32>();
    assert_pyro_traits::<PNested4Timestamp16>();
}
