//! Every derived struct shape from `tests/value_match/primitive_4_fixed_nesting_u32.rs`, minus
//! the golden-fixture plumbing: each must derive `Layout`, `Writer`
//! and `Reader`. Generated from the value_match file — keep in sync.

use half::f16;
use pyroformat::{Layout, Reader, Writer};

fn assert_pyro_traits<T: Layout + Writer + Reader>() {}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4FixedU8W32 {
    a: Vec<Vec<Vec<Vec<[u8; 2]>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4FixedU8W16 {
    a: Vec<Vec<Vec<Vec<[u8; 2]>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4FixedI8W32 {
    a: Vec<Vec<Vec<Vec<[i8; 2]>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4FixedI8W16 {
    a: Vec<Vec<Vec<Vec<[i8; 2]>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4FixedU16W32 {
    a: Vec<Vec<Vec<Vec<[u16; 2]>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4FixedU16W16 {
    a: Vec<Vec<Vec<Vec<[u16; 2]>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4FixedI16W32 {
    a: Vec<Vec<Vec<Vec<[i16; 2]>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4FixedI16W16 {
    a: Vec<Vec<Vec<Vec<[i16; 2]>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4FixedU32W32 {
    a: Vec<Vec<Vec<Vec<[u32; 2]>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4FixedU32W16 {
    a: Vec<Vec<Vec<Vec<[u32; 2]>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4FixedI32W32 {
    a: Vec<Vec<Vec<Vec<[i32; 2]>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4FixedI32W16 {
    a: Vec<Vec<Vec<Vec<[i32; 2]>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4FixedU64W32 {
    a: Vec<Vec<Vec<Vec<[u64; 2]>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4FixedU64W16 {
    a: Vec<Vec<Vec<Vec<[u64; 2]>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4FixedI64W32 {
    a: Vec<Vec<Vec<Vec<[i64; 2]>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4FixedI64W16 {
    a: Vec<Vec<Vec<Vec<[i64; 2]>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4FixedF16W32 {
    a: Vec<Vec<Vec<Vec<[f16; 2]>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4FixedF16W16 {
    a: Vec<Vec<Vec<Vec<[f16; 2]>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4FixedF32W32 {
    a: Vec<Vec<Vec<Vec<[f32; 2]>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4FixedF32W16 {
    a: Vec<Vec<Vec<Vec<[f32; 2]>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4FixedF64W32 {
    a: Vec<Vec<Vec<Vec<[f64; 2]>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4FixedF64W16 {
    a: Vec<Vec<Vec<Vec<[f64; 2]>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u32)]
struct PNested4FixedBool32 {
    a: Vec<Vec<Vec<Vec<[bool; 2]>>>>,
}

#[derive(Layout, Writer, Reader, Debug, PartialEq)]
#[pyro(Word = u16)]
struct PNested4FixedBool16 {
    a: Vec<Vec<Vec<Vec<[bool; 2]>>>>,
}

fn main() {
    assert_pyro_traits::<PNested4FixedU8W32>();
    assert_pyro_traits::<PNested4FixedU8W16>();
    assert_pyro_traits::<PNested4FixedI8W32>();
    assert_pyro_traits::<PNested4FixedI8W16>();
    assert_pyro_traits::<PNested4FixedU16W32>();
    assert_pyro_traits::<PNested4FixedU16W16>();
    assert_pyro_traits::<PNested4FixedI16W32>();
    assert_pyro_traits::<PNested4FixedI16W16>();
    assert_pyro_traits::<PNested4FixedU32W32>();
    assert_pyro_traits::<PNested4FixedU32W16>();
    assert_pyro_traits::<PNested4FixedI32W32>();
    assert_pyro_traits::<PNested4FixedI32W16>();
    assert_pyro_traits::<PNested4FixedU64W32>();
    assert_pyro_traits::<PNested4FixedU64W16>();
    assert_pyro_traits::<PNested4FixedI64W32>();
    assert_pyro_traits::<PNested4FixedI64W16>();
    assert_pyro_traits::<PNested4FixedF16W32>();
    assert_pyro_traits::<PNested4FixedF16W16>();
    assert_pyro_traits::<PNested4FixedF32W32>();
    assert_pyro_traits::<PNested4FixedF32W16>();
    assert_pyro_traits::<PNested4FixedF64W32>();
    assert_pyro_traits::<PNested4FixedF64W16>();
    assert_pyro_traits::<PNested4FixedBool32>();
    assert_pyro_traits::<PNested4FixedBool16>();
}
