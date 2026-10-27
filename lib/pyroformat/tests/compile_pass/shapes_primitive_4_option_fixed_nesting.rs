//! Every derived struct shape from `tests/value_match/primitive_4_option_fixed_nesting_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use half::f16;
use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionFixedU8W32 {
    a: Vec<Vec<Vec<Vec<Option<[u8; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4OptionFixedU8W16 {
    a: Vec<Vec<Vec<Vec<Option<[u8; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionFixedI8W32 {
    a: Vec<Vec<Vec<Vec<Option<[i8; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4OptionFixedI8W16 {
    a: Vec<Vec<Vec<Vec<Option<[i8; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionFixedU16W32 {
    a: Vec<Vec<Vec<Vec<Option<[u16; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4OptionFixedU16W16 {
    a: Vec<Vec<Vec<Vec<Option<[u16; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionFixedI16W32 {
    a: Vec<Vec<Vec<Vec<Option<[i16; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4OptionFixedI16W16 {
    a: Vec<Vec<Vec<Vec<Option<[i16; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionFixedU32W32 {
    a: Vec<Vec<Vec<Vec<Option<[u32; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4OptionFixedU32W16 {
    a: Vec<Vec<Vec<Vec<Option<[u32; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionFixedI32W32 {
    a: Vec<Vec<Vec<Vec<Option<[i32; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4OptionFixedI32W16 {
    a: Vec<Vec<Vec<Vec<Option<[i32; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionFixedU64W32 {
    a: Vec<Vec<Vec<Vec<Option<[u64; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4OptionFixedU64W16 {
    a: Vec<Vec<Vec<Vec<Option<[u64; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionFixedI64W32 {
    a: Vec<Vec<Vec<Vec<Option<[i64; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4OptionFixedI64W16 {
    a: Vec<Vec<Vec<Vec<Option<[i64; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionFixedF16W32 {
    a: Vec<Vec<Vec<Vec<Option<[f16; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4OptionFixedF16W16 {
    a: Vec<Vec<Vec<Vec<Option<[f16; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionFixedF32W32 {
    a: Vec<Vec<Vec<Vec<Option<[f32; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4OptionFixedF32W16 {
    a: Vec<Vec<Vec<Vec<Option<[f32; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionFixedF64W32 {
    a: Vec<Vec<Vec<Vec<Option<[f64; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4OptionFixedF64W16 {
    a: Vec<Vec<Vec<Vec<Option<[f64; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4OptionFixedBool32 {
    a: Vec<Vec<Vec<Vec<Option<[bool; 2]>>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4OptionFixedBool16 {
    a: Vec<Vec<Vec<Vec<Option<[bool; 2]>>>>>,
}

fn main() {
    assert_pyro_traits::<PNested4OptionFixedU8W32>();
    assert_pyro_traits::<PNested4OptionFixedU8W16>();
    assert_pyro_traits::<PNested4OptionFixedI8W32>();
    assert_pyro_traits::<PNested4OptionFixedI8W16>();
    assert_pyro_traits::<PNested4OptionFixedU16W32>();
    assert_pyro_traits::<PNested4OptionFixedU16W16>();
    assert_pyro_traits::<PNested4OptionFixedI16W32>();
    assert_pyro_traits::<PNested4OptionFixedI16W16>();
    assert_pyro_traits::<PNested4OptionFixedU32W32>();
    assert_pyro_traits::<PNested4OptionFixedU32W16>();
    assert_pyro_traits::<PNested4OptionFixedI32W32>();
    assert_pyro_traits::<PNested4OptionFixedI32W16>();
    assert_pyro_traits::<PNested4OptionFixedU64W32>();
    assert_pyro_traits::<PNested4OptionFixedU64W16>();
    assert_pyro_traits::<PNested4OptionFixedI64W32>();
    assert_pyro_traits::<PNested4OptionFixedI64W16>();
    assert_pyro_traits::<PNested4OptionFixedF16W32>();
    assert_pyro_traits::<PNested4OptionFixedF16W16>();
    assert_pyro_traits::<PNested4OptionFixedF32W32>();
    assert_pyro_traits::<PNested4OptionFixedF32W16>();
    assert_pyro_traits::<PNested4OptionFixedF64W32>();
    assert_pyro_traits::<PNested4OptionFixedF64W16>();
    assert_pyro_traits::<PNested4OptionFixedBool32>();
    assert_pyro_traits::<PNested4OptionFixedBool16>();
}
