//! Every derived struct shape from `tests/value_match/primitive_slice_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use half::f16;
use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecBool32 {
    #[pyro(allow(vec_bool))]
    a: Vec<bool>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecBool16 {
    #[pyro(allow(vec_bool))]
    a: Vec<bool>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecI8W32 { a: Vec<i8> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecI8W16 { a: Vec<i8> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecI16W32 { a: Vec<i16> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecI16W16 { a: Vec<i16> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecI32W32 { a: Vec<i32> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecI32W16 { a: Vec<i32> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecI64W32 { a: Vec<i64> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecI64W16 { a: Vec<i64> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecU8W32 { #[pyro(numeric)] a: Vec<u8> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecU8W16 { #[pyro(numeric)] a: Vec<u8> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecU16W32 { a: Vec<u16> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecU16W16 { a: Vec<u16> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecU32W32 { a: Vec<u32> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecU32W16 { a: Vec<u32> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecU64W32 { a: Vec<u64> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecU64W16 { a: Vec<u64> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecF16W32 { a: Vec<f16> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecF16W16 { a: Vec<f16> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecF32W32 { a: Vec<f32> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecF32W16 { a: Vec<f32> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecF64W32 { a: Vec<f64> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecF64W16 { a: Vec<f64> }

fn main() {
    assert_pyro_traits::<PVecBool32>();
    assert_pyro_traits::<PVecBool16>();
    assert_pyro_traits::<PVecI8W32>();
    assert_pyro_traits::<PVecI8W16>();
    assert_pyro_traits::<PVecI16W32>();
    assert_pyro_traits::<PVecI16W16>();
    assert_pyro_traits::<PVecI32W32>();
    assert_pyro_traits::<PVecI32W16>();
    assert_pyro_traits::<PVecI64W32>();
    assert_pyro_traits::<PVecI64W16>();
    assert_pyro_traits::<PVecU8W32>();
    assert_pyro_traits::<PVecU8W16>();
    assert_pyro_traits::<PVecU16W32>();
    assert_pyro_traits::<PVecU16W16>();
    assert_pyro_traits::<PVecU32W32>();
    assert_pyro_traits::<PVecU32W16>();
    assert_pyro_traits::<PVecU64W32>();
    assert_pyro_traits::<PVecU64W16>();
    assert_pyro_traits::<PVecF16W32>();
    assert_pyro_traits::<PVecF16W16>();
    assert_pyro_traits::<PVecF32W32>();
    assert_pyro_traits::<PVecF32W16>();
    assert_pyro_traits::<PVecF64W32>();
    assert_pyro_traits::<PVecF64W16>();
}
