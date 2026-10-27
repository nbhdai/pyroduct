//! Every derived struct shape from `tests/value_match/primitive_2_option_nesting_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use half::f16;
use pyroformat::value::Time;
use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2OptionU8W32 {
    a: Vec<Vec<Option<u8>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2OptionU8W16 {
    a: Vec<Vec<Option<u8>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2OptionI8W32 {
    a: Vec<Vec<Option<i8>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2OptionI8W16 {
    a: Vec<Vec<Option<i8>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2OptionU16W32 {
    a: Vec<Vec<Option<u16>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2OptionU16W16 {
    a: Vec<Vec<Option<u16>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2OptionI16W32 {
    a: Vec<Vec<Option<i16>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2OptionI16W16 {
    a: Vec<Vec<Option<i16>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2OptionU32W32 {
    a: Vec<Vec<Option<u32>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2OptionU32W16 {
    a: Vec<Vec<Option<u32>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2OptionI32W32 {
    a: Vec<Vec<Option<i32>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2OptionI32W16 {
    a: Vec<Vec<Option<i32>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2OptionU64W32 {
    a: Vec<Vec<Option<u64>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2OptionU64W16 {
    a: Vec<Vec<Option<u64>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2OptionI64W32 {
    a: Vec<Vec<Option<i64>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2OptionI64W16 {
    a: Vec<Vec<Option<i64>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2OptionF16W32 {
    a: Vec<Vec<Option<f16>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2OptionF16W16 {
    a: Vec<Vec<Option<f16>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2OptionF32W32 {
    a: Vec<Vec<Option<f32>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2OptionF32W16 {
    a: Vec<Vec<Option<f32>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2OptionF64W32 {
    a: Vec<Vec<Option<f64>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2OptionF64W16 {
    a: Vec<Vec<Option<f64>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2OptionBool32 {
    a: Vec<Vec<Option<bool>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2OptionBool16 {
    a: Vec<Vec<Option<bool>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested2OptionTimestamp32 {
    a: Vec<Vec<Option<Time>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested2OptionTimestamp16 {
    a: Vec<Vec<Option<Time>>>,
}

fn main() {
    assert_pyro_traits::<PNested2OptionU8W32>();
    assert_pyro_traits::<PNested2OptionU8W16>();
    assert_pyro_traits::<PNested2OptionI8W32>();
    assert_pyro_traits::<PNested2OptionI8W16>();
    assert_pyro_traits::<PNested2OptionU16W32>();
    assert_pyro_traits::<PNested2OptionU16W16>();
    assert_pyro_traits::<PNested2OptionI16W32>();
    assert_pyro_traits::<PNested2OptionI16W16>();
    assert_pyro_traits::<PNested2OptionU32W32>();
    assert_pyro_traits::<PNested2OptionU32W16>();
    assert_pyro_traits::<PNested2OptionI32W32>();
    assert_pyro_traits::<PNested2OptionI32W16>();
    assert_pyro_traits::<PNested2OptionU64W32>();
    assert_pyro_traits::<PNested2OptionU64W16>();
    assert_pyro_traits::<PNested2OptionI64W32>();
    assert_pyro_traits::<PNested2OptionI64W16>();
    assert_pyro_traits::<PNested2OptionF16W32>();
    assert_pyro_traits::<PNested2OptionF16W16>();
    assert_pyro_traits::<PNested2OptionF32W32>();
    assert_pyro_traits::<PNested2OptionF32W16>();
    assert_pyro_traits::<PNested2OptionF64W32>();
    assert_pyro_traits::<PNested2OptionF64W16>();
    assert_pyro_traits::<PNested2OptionBool32>();
    assert_pyro_traits::<PNested2OptionBool16>();
    assert_pyro_traits::<PNested2OptionTimestamp32>();
    assert_pyro_traits::<PNested2OptionTimestamp16>();
}
