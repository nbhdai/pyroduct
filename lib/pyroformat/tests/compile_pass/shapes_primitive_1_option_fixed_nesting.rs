//! Every derived struct shape from `tests/value_match/primitive_1_option_fixed_nesting_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use half::f16;
use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionFixedU8W32 {
    a: Vec<Option<[u8; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionFixedU8W16 {
    a: Vec<Option<[u8; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionFixedI8W32 {
    a: Vec<Option<[i8; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionFixedI8W16 {
    a: Vec<Option<[i8; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionFixedU16W32 {
    a: Vec<Option<[u16; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionFixedU16W16 {
    a: Vec<Option<[u16; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionFixedI16W32 {
    a: Vec<Option<[i16; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionFixedI16W16 {
    a: Vec<Option<[i16; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionFixedU32W32 {
    a: Vec<Option<[u32; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionFixedU32W16 {
    a: Vec<Option<[u32; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionFixedI32W32 {
    a: Vec<Option<[i32; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionFixedI32W16 {
    a: Vec<Option<[i32; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionFixedU64W32 {
    a: Vec<Option<[u64; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionFixedU64W16 {
    a: Vec<Option<[u64; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionFixedI64W32 {
    a: Vec<Option<[i64; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionFixedI64W16 {
    a: Vec<Option<[i64; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionFixedF16W32 {
    a: Vec<Option<[f16; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionFixedF16W16 {
    a: Vec<Option<[f16; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionFixedF32W32 {
    a: Vec<Option<[f32; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionFixedF32W16 {
    a: Vec<Option<[f32; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionFixedF64W32 {
    a: Vec<Option<[f64; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionFixedF64W16 {
    a: Vec<Option<[f64; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested1OptionFixedBool32 {
    a: Vec<Option<[bool; 2]>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested1OptionFixedBool16 {
    a: Vec<Option<[bool; 2]>>,
}

fn main() {
    assert_pyro_traits::<PNested1OptionFixedU8W32>();
    assert_pyro_traits::<PNested1OptionFixedU8W16>();
    assert_pyro_traits::<PNested1OptionFixedI8W32>();
    assert_pyro_traits::<PNested1OptionFixedI8W16>();
    assert_pyro_traits::<PNested1OptionFixedU16W32>();
    assert_pyro_traits::<PNested1OptionFixedU16W16>();
    assert_pyro_traits::<PNested1OptionFixedI16W32>();
    assert_pyro_traits::<PNested1OptionFixedI16W16>();
    assert_pyro_traits::<PNested1OptionFixedU32W32>();
    assert_pyro_traits::<PNested1OptionFixedU32W16>();
    assert_pyro_traits::<PNested1OptionFixedI32W32>();
    assert_pyro_traits::<PNested1OptionFixedI32W16>();
    assert_pyro_traits::<PNested1OptionFixedU64W32>();
    assert_pyro_traits::<PNested1OptionFixedU64W16>();
    assert_pyro_traits::<PNested1OptionFixedI64W32>();
    assert_pyro_traits::<PNested1OptionFixedI64W16>();
    assert_pyro_traits::<PNested1OptionFixedF16W32>();
    assert_pyro_traits::<PNested1OptionFixedF16W16>();
    assert_pyro_traits::<PNested1OptionFixedF32W32>();
    assert_pyro_traits::<PNested1OptionFixedF32W16>();
    assert_pyro_traits::<PNested1OptionFixedF64W32>();
    assert_pyro_traits::<PNested1OptionFixedF64W16>();
    assert_pyro_traits::<PNested1OptionFixedBool32>();
    assert_pyro_traits::<PNested1OptionFixedBool16>();
}
