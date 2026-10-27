//! Every derived struct shape from `tests/value_match/primitive_3_option_nesting_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use half::f16;
use pyroformat::value::Time;
use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3OptionU8W32 {
    a: Vec<Vec<Vec<Option<u8>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested3OptionU8W16 {
    a: Vec<Vec<Vec<Option<u8>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3OptionI8W32 {
    a: Vec<Vec<Vec<Option<i8>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested3OptionI8W16 {
    a: Vec<Vec<Vec<Option<i8>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3OptionU16W32 {
    a: Vec<Vec<Vec<Option<u16>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested3OptionU16W16 {
    a: Vec<Vec<Vec<Option<u16>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3OptionI16W32 {
    a: Vec<Vec<Vec<Option<i16>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested3OptionI16W16 {
    a: Vec<Vec<Vec<Option<i16>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3OptionU32W32 {
    a: Vec<Vec<Vec<Option<u32>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested3OptionU32W16 {
    a: Vec<Vec<Vec<Option<u32>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3OptionI32W32 {
    a: Vec<Vec<Vec<Option<i32>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested3OptionI32W16 {
    a: Vec<Vec<Vec<Option<i32>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3OptionU64W32 {
    a: Vec<Vec<Vec<Option<u64>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested3OptionU64W16 {
    a: Vec<Vec<Vec<Option<u64>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3OptionI64W32 {
    a: Vec<Vec<Vec<Option<i64>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested3OptionI64W16 {
    a: Vec<Vec<Vec<Option<i64>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3OptionF16W32 {
    a: Vec<Vec<Vec<Option<f16>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested3OptionF16W16 {
    a: Vec<Vec<Vec<Option<f16>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3OptionF32W32 {
    a: Vec<Vec<Vec<Option<f32>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested3OptionF32W16 {
    a: Vec<Vec<Vec<Option<f32>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3OptionF64W32 {
    a: Vec<Vec<Vec<Option<f64>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested3OptionF64W16 {
    a: Vec<Vec<Vec<Option<f64>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3OptionBool32 {
    a: Vec<Vec<Vec<Option<bool>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested3OptionBool16 {
    a: Vec<Vec<Vec<Option<bool>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested3OptionTimestamp32 {
    a: Vec<Vec<Vec<Option<Time>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested3OptionTimestamp16 {
    a: Vec<Vec<Vec<Option<Time>>>>,
}

fn main() {
    assert_pyro_traits::<PNested3OptionU8W32>();
    assert_pyro_traits::<PNested3OptionU8W16>();
    assert_pyro_traits::<PNested3OptionI8W32>();
    assert_pyro_traits::<PNested3OptionI8W16>();
    assert_pyro_traits::<PNested3OptionU16W32>();
    assert_pyro_traits::<PNested3OptionU16W16>();
    assert_pyro_traits::<PNested3OptionI16W32>();
    assert_pyro_traits::<PNested3OptionI16W16>();
    assert_pyro_traits::<PNested3OptionU32W32>();
    assert_pyro_traits::<PNested3OptionU32W16>();
    assert_pyro_traits::<PNested3OptionI32W32>();
    assert_pyro_traits::<PNested3OptionI32W16>();
    assert_pyro_traits::<PNested3OptionU64W32>();
    assert_pyro_traits::<PNested3OptionU64W16>();
    assert_pyro_traits::<PNested3OptionI64W32>();
    assert_pyro_traits::<PNested3OptionI64W16>();
    assert_pyro_traits::<PNested3OptionF16W32>();
    assert_pyro_traits::<PNested3OptionF16W16>();
    assert_pyro_traits::<PNested3OptionF32W32>();
    assert_pyro_traits::<PNested3OptionF32W16>();
    assert_pyro_traits::<PNested3OptionF64W32>();
    assert_pyro_traits::<PNested3OptionF64W16>();
    assert_pyro_traits::<PNested3OptionBool32>();
    assert_pyro_traits::<PNested3OptionBool16>();
    assert_pyro_traits::<PNested3OptionTimestamp32>();
    assert_pyro_traits::<PNested3OptionTimestamp16>();
}
