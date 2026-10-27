//! Every derived struct shape from `tests/value_match/primitive_1_option_nesting_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use half::f16;
use pyroformat::value::Time;
use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionU8W32 {
    a: Vec<Option<u8>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionU8W16 {
    a: Vec<Option<u8>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionI8W32 {
    a: Vec<Option<i8>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionI8W16 {
    a: Vec<Option<i8>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionU16W32 {
    a: Vec<Option<u16>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionU16W16 {
    a: Vec<Option<u16>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionI16W32 {
    a: Vec<Option<i16>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionI16W16 {
    a: Vec<Option<i16>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionU32W32 {
    a: Vec<Option<u32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionU32W16 {
    a: Vec<Option<u32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionI32W32 {
    a: Vec<Option<i32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionI32W16 {
    a: Vec<Option<i32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionU64W32 {
    a: Vec<Option<u64>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionU64W16 {
    a: Vec<Option<u64>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionI64W32 {
    a: Vec<Option<i64>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionI64W16 {
    a: Vec<Option<i64>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionF16W32 {
    a: Vec<Option<f16>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionF16W16 {
    a: Vec<Option<f16>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionF32W32 {
    a: Vec<Option<f32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionF32W16 {
    a: Vec<Option<f32>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionF64W32 {
    a: Vec<Option<f64>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionF64W16 {
    a: Vec<Option<f64>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionBool32 {
    a: Vec<Option<bool>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionBool16 {
    a: Vec<Option<bool>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionTimestamp32 {
    a: Vec<Option<Time>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionTimestamp16 {
    a: Vec<Option<Time>>,
}

fn main() {
    assert_pyro_traits::<PNested1OptionU8W32>();
    assert_pyro_traits::<PNested1OptionU8W16>();
    assert_pyro_traits::<PNested1OptionI8W32>();
    assert_pyro_traits::<PNested1OptionI8W16>();
    assert_pyro_traits::<PNested1OptionU16W32>();
    assert_pyro_traits::<PNested1OptionU16W16>();
    assert_pyro_traits::<PNested1OptionI16W32>();
    assert_pyro_traits::<PNested1OptionI16W16>();
    assert_pyro_traits::<PNested1OptionU32W32>();
    assert_pyro_traits::<PNested1OptionU32W16>();
    assert_pyro_traits::<PNested1OptionI32W32>();
    assert_pyro_traits::<PNested1OptionI32W16>();
    assert_pyro_traits::<PNested1OptionU64W32>();
    assert_pyro_traits::<PNested1OptionU64W16>();
    assert_pyro_traits::<PNested1OptionI64W32>();
    assert_pyro_traits::<PNested1OptionI64W16>();
    assert_pyro_traits::<PNested1OptionF16W32>();
    assert_pyro_traits::<PNested1OptionF16W16>();
    assert_pyro_traits::<PNested1OptionF32W32>();
    assert_pyro_traits::<PNested1OptionF32W16>();
    assert_pyro_traits::<PNested1OptionF64W32>();
    assert_pyro_traits::<PNested1OptionF64W16>();
    assert_pyro_traits::<PNested1OptionBool32>();
    assert_pyro_traits::<PNested1OptionBool16>();
    assert_pyro_traits::<PNested1OptionTimestamp32>();
    assert_pyro_traits::<PNested1OptionTimestamp16>();
}
