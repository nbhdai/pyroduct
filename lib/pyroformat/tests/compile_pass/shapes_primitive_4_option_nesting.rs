//! Every derived struct shape from `tests/value_match/primitive_4_option_nesting_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use half::f16;
use pyroformat::value::Time;
use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionU8W32 {
    a: Vec<Vec<Vec<Vec<Option<u8>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4OptionU8W16 {
    a: Vec<Vec<Vec<Vec<Option<u8>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionI8W32 {
    a: Vec<Vec<Vec<Vec<Option<i8>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4OptionI8W16 {
    a: Vec<Vec<Vec<Vec<Option<i8>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionU16W32 {
    a: Vec<Vec<Vec<Vec<Option<u16>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4OptionU16W16 {
    a: Vec<Vec<Vec<Vec<Option<u16>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionI16W32 {
    a: Vec<Vec<Vec<Vec<Option<i16>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4OptionI16W16 {
    a: Vec<Vec<Vec<Vec<Option<i16>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionU32W32 {
    a: Vec<Vec<Vec<Vec<Option<u32>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4OptionU32W16 {
    a: Vec<Vec<Vec<Vec<Option<u32>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionI32W32 {
    a: Vec<Vec<Vec<Vec<Option<i32>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4OptionI32W16 {
    a: Vec<Vec<Vec<Vec<Option<i32>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionU64W32 {
    a: Vec<Vec<Vec<Vec<Option<u64>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4OptionU64W16 {
    a: Vec<Vec<Vec<Vec<Option<u64>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionI64W32 {
    a: Vec<Vec<Vec<Vec<Option<i64>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4OptionI64W16 {
    a: Vec<Vec<Vec<Vec<Option<i64>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionF16W32 {
    a: Vec<Vec<Vec<Vec<Option<f16>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4OptionF16W16 {
    a: Vec<Vec<Vec<Vec<Option<f16>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionF32W32 {
    a: Vec<Vec<Vec<Vec<Option<f32>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4OptionF32W16 {
    a: Vec<Vec<Vec<Vec<Option<f32>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionF64W32 {
    a: Vec<Vec<Vec<Vec<Option<f64>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4OptionF64W16 {
    a: Vec<Vec<Vec<Vec<Option<f64>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionBool32 {
    a: Vec<Vec<Vec<Vec<Option<bool>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4OptionBool16 {
    a: Vec<Vec<Vec<Vec<Option<bool>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionTimestamp32 {
    a: Vec<Vec<Vec<Vec<Option<Time>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4OptionTimestamp16 {
    a: Vec<Vec<Vec<Vec<Option<Time>>>>>,
}

fn main() {
    assert_pyro_traits::<PNested4OptionU8W32>();
    assert_pyro_traits::<PNested4OptionU8W16>();
    assert_pyro_traits::<PNested4OptionI8W32>();
    assert_pyro_traits::<PNested4OptionI8W16>();
    assert_pyro_traits::<PNested4OptionU16W32>();
    assert_pyro_traits::<PNested4OptionU16W16>();
    assert_pyro_traits::<PNested4OptionI16W32>();
    assert_pyro_traits::<PNested4OptionI16W16>();
    assert_pyro_traits::<PNested4OptionU32W32>();
    assert_pyro_traits::<PNested4OptionU32W16>();
    assert_pyro_traits::<PNested4OptionI32W32>();
    assert_pyro_traits::<PNested4OptionI32W16>();
    assert_pyro_traits::<PNested4OptionU64W32>();
    assert_pyro_traits::<PNested4OptionU64W16>();
    assert_pyro_traits::<PNested4OptionI64W32>();
    assert_pyro_traits::<PNested4OptionI64W16>();
    assert_pyro_traits::<PNested4OptionF16W32>();
    assert_pyro_traits::<PNested4OptionF16W16>();
    assert_pyro_traits::<PNested4OptionF32W32>();
    assert_pyro_traits::<PNested4OptionF32W16>();
    assert_pyro_traits::<PNested4OptionF64W32>();
    assert_pyro_traits::<PNested4OptionF64W16>();
    assert_pyro_traits::<PNested4OptionBool32>();
    assert_pyro_traits::<PNested4OptionBool16>();
    assert_pyro_traits::<PNested4OptionTimestamp32>();
    assert_pyro_traits::<PNested4OptionTimestamp16>();
}
