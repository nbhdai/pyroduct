//! Every derived struct shape from `tests/value_match/primitive_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use half::f16;
use pyroformat::value::Time;
use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PBool32 {
    a: bool,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PBool16 {
    a: bool,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PI8W32 {
    a: i8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PI8W16 {
    a: i8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PI16W32 {
    a: i16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PI16W16 {
    a: i16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PI32W32 {
    a: i32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PI32W16 {
    a: i32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PI64W32 {
    a: i64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PI64W16 {
    a: i64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PU8W32 {
    a: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PU8W16 {
    a: u8,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PU16W32 {
    a: u16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PU16W16 {
    a: u16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PU32W32 {
    a: u32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PU32W16 {
    a: u32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PU64W32 {
    a: u64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PU64W16 {
    a: u64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PF16W32 {
    a: f16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PF16W16 {
    a: f16,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PF32W32 {
    a: f32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PF32W16 {
    a: f32,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PF64W32 {
    a: f64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PF64W16 {
    a: f64,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PTimestamp32 {
    a: Time,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PTimestamp16 {
    a: Time,
}

fn main() {
    assert_pyro_traits::<PBool32>();
    assert_pyro_traits::<PBool16>();
    assert_pyro_traits::<PI8W32>();
    assert_pyro_traits::<PI8W16>();
    assert_pyro_traits::<PI16W32>();
    assert_pyro_traits::<PI16W16>();
    assert_pyro_traits::<PI32W32>();
    assert_pyro_traits::<PI32W16>();
    assert_pyro_traits::<PI64W32>();
    assert_pyro_traits::<PI64W16>();
    assert_pyro_traits::<PU8W32>();
    assert_pyro_traits::<PU8W16>();
    assert_pyro_traits::<PU16W32>();
    assert_pyro_traits::<PU16W16>();
    assert_pyro_traits::<PU32W32>();
    assert_pyro_traits::<PU32W16>();
    assert_pyro_traits::<PU64W32>();
    assert_pyro_traits::<PU64W16>();
    assert_pyro_traits::<PF16W32>();
    assert_pyro_traits::<PF16W16>();
    assert_pyro_traits::<PF32W32>();
    assert_pyro_traits::<PF32W16>();
    assert_pyro_traits::<PF64W32>();
    assert_pyro_traits::<PF64W16>();
    assert_pyro_traits::<PTimestamp32>();
    assert_pyro_traits::<PTimestamp16>();
}
