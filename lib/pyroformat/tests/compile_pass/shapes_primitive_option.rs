//! Every derived struct shape from `tests/value_match/primitive_option_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use half::f16;
use pyroformat::value::Time;
use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct POptBool32 { a: Option<bool> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct POptBool16 { a: Option<bool> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct POptI8W32 { a: Option<i8> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct POptI8W16 { a: Option<i8> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct POptI16W32 { a: Option<i16> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct POptI16W16 { a: Option<i16> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct POptI32W32 { a: Option<i32> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct POptI32W16 { a: Option<i32> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct POptI64W32 { a: Option<i64> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct POptI64W16 { a: Option<i64> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct POptU8W32 { a: Option<u8> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct POptU8W16 { a: Option<u8> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct POptU16W32 { a: Option<u16> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct POptU16W16 { a: Option<u16> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct POptU32W32 { a: Option<u32> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct POptU32W16 { a: Option<u32> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct POptU64W32 { a: Option<u64> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct POptU64W16 { a: Option<u64> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct POptF16W32 { a: Option<f16> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct POptF16W16 { a: Option<f16> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct POptF32W32 { a: Option<f32> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct POptF32W16 { a: Option<f32> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct POptF64W32 { a: Option<f64> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct POptF64W16 { a: Option<f64> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct POptTimestamp32 { a: Option<Time> }

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct POptTimestamp16 { a: Option<Time> }

fn main() {
    assert_pyro_traits::<POptBool32>();
    assert_pyro_traits::<POptBool16>();
    assert_pyro_traits::<POptI8W32>();
    assert_pyro_traits::<POptI8W16>();
    assert_pyro_traits::<POptI16W32>();
    assert_pyro_traits::<POptI16W16>();
    assert_pyro_traits::<POptI32W32>();
    assert_pyro_traits::<POptI32W16>();
    assert_pyro_traits::<POptI64W32>();
    assert_pyro_traits::<POptI64W16>();
    assert_pyro_traits::<POptU8W32>();
    assert_pyro_traits::<POptU8W16>();
    assert_pyro_traits::<POptU16W32>();
    assert_pyro_traits::<POptU16W16>();
    assert_pyro_traits::<POptU32W32>();
    assert_pyro_traits::<POptU32W16>();
    assert_pyro_traits::<POptU64W32>();
    assert_pyro_traits::<POptU64W16>();
    assert_pyro_traits::<POptF16W32>();
    assert_pyro_traits::<POptF16W16>();
    assert_pyro_traits::<POptF32W32>();
    assert_pyro_traits::<POptF32W16>();
    assert_pyro_traits::<POptF64W32>();
    assert_pyro_traits::<POptF64W16>();
    assert_pyro_traits::<POptTimestamp32>();
    assert_pyro_traits::<POptTimestamp16>();
}
