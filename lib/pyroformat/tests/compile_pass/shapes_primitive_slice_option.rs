//! Every derived struct shape from `tests/value_match/primitive_slice_option_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use half::f16;
use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecOptBool32 { a: Vec<Option<bool>> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecOptBool16 { a: Vec<Option<bool>> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecOptI8W32 { a: Vec<Option<i8>> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecOptI8W16 { a: Vec<Option<i8>> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecOptI16W32 { a: Vec<Option<i16>> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecOptI16W16 { a: Vec<Option<i16>> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecOptI32W32 { a: Vec<Option<i32>> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecOptI32W16 { a: Vec<Option<i32>> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecOptI64W32 { a: Vec<Option<i64>> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecOptI64W16 { a: Vec<Option<i64>> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecOptU8W32 { a: Vec<Option<u8>> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecOptU8W16 { a: Vec<Option<u8>> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecOptU16W32 { a: Vec<Option<u16>> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecOptU16W16 { a: Vec<Option<u16>> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecOptU32W32 { a: Vec<Option<u32>> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecOptU32W16 { a: Vec<Option<u32>> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecOptU64W32 { a: Vec<Option<u64>> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecOptU64W16 { a: Vec<Option<u64>> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecOptF16W32 { a: Vec<Option<f16>> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecOptF16W16 { a: Vec<Option<f16>> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecOptF32W32 { a: Vec<Option<f32>> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecOptF32W16 { a: Vec<Option<f32>> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PVecOptF64W32 { a: Vec<Option<f64>> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PVecOptF64W16 { a: Vec<Option<f64>> }

fn main() {
    assert_pyro_traits::<PVecOptBool32>();
    assert_pyro_traits::<PVecOptBool16>();
    assert_pyro_traits::<PVecOptI8W32>();
    assert_pyro_traits::<PVecOptI8W16>();
    assert_pyro_traits::<PVecOptI16W32>();
    assert_pyro_traits::<PVecOptI16W16>();
    assert_pyro_traits::<PVecOptI32W32>();
    assert_pyro_traits::<PVecOptI32W16>();
    assert_pyro_traits::<PVecOptI64W32>();
    assert_pyro_traits::<PVecOptI64W16>();
    assert_pyro_traits::<PVecOptU8W32>();
    assert_pyro_traits::<PVecOptU8W16>();
    assert_pyro_traits::<PVecOptU16W32>();
    assert_pyro_traits::<PVecOptU16W16>();
    assert_pyro_traits::<PVecOptU32W32>();
    assert_pyro_traits::<PVecOptU32W16>();
    assert_pyro_traits::<PVecOptU64W32>();
    assert_pyro_traits::<PVecOptU64W16>();
    assert_pyro_traits::<PVecOptF16W32>();
    assert_pyro_traits::<PVecOptF16W16>();
    assert_pyro_traits::<PVecOptF32W32>();
    assert_pyro_traits::<PVecOptF32W16>();
    assert_pyro_traits::<PVecOptF64W32>();
    assert_pyro_traits::<PVecOptF64W16>();
}
